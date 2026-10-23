use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::epaint::{ClippedPrimitive, ImageData, Mesh, Primitive};
use egui::{Color32, Pos2, Rect, TextureId, TexturesDelta, ViewportCommand, ViewportId};
use egui_winit::winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::core::InboxMoverCore;
use crate::gui::InboxMoverGuiApp;

#[derive(Clone)]
struct SwTexture {
    width: usize,
    height: usize,
    pixels: Vec<Color32>,
}

impl SwTexture {
    #[inline]
    fn sample_bilinear(&self, u: f32, v: f32) -> Color32 {
        if self.pixels.is_empty() || self.width == 0 || self.height == 0 {
            return Color32::WHITE;
        }
        // Fast path for egui WHITE_UV = (0.0, 0.0)
        if u <= 0.0005 && v <= 0.0005 {
            return self.pixels[0];
        }

        let fx = (u.clamp(0.0, 1.0) * self.width as f32 - 0.5).max(0.0);
        let fy = (v.clamp(0.0, 1.0) * self.height as f32 - 0.5).max(0.0);

        let x0 = (fx as usize).min(self.width - 1);
        let y0 = (fy as usize).min(self.height - 1);
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let wx = ((fx - x0 as f32) * 256.0) as u32;
        let wy = ((fy - y0 as f32) * 256.0) as u32;

        if wx == 0 && wy == 0 {
            return self.pixels[y0 * self.width + x0];
        }

        let p00 = self.pixels[y0 * self.width + x0].to_array();
        let p10 = self.pixels[y0 * self.width + x1].to_array();
        let p01 = self.pixels[y1 * self.width + x0].to_array();
        let p11 = self.pixels[y1 * self.width + x1].to_array();

        let inv_wx = 256 - wx;
        let inv_wy = 256 - wy;

        let w00 = inv_wx * inv_wy;
        let w10 = wx * inv_wy;
        let w01 = inv_wx * wy;
        let w11 = wx * wy;

        let r = ((p00[0] as u32 * w00
            + p10[0] as u32 * w10
            + p01[0] as u32 * w01
            + p11[0] as u32 * w11
            + 32768)
            >> 16) as u8;
        let g = ((p00[1] as u32 * w00
            + p10[1] as u32 * w10
            + p01[1] as u32 * w01
            + p11[1] as u32 * w11
            + 32768)
            >> 16) as u8;
        let b = ((p00[2] as u32 * w00
            + p10[2] as u32 * w10
            + p01[2] as u32 * w01
            + p11[2] as u32 * w11
            + 32768)
            >> 16) as u8;
        let a = ((p00[3] as u32 * w00
            + p10[3] as u32 * w10
            + p01[3] as u32 * w01
            + p11[3] as u32 * w11
            + 32768)
            >> 16) as u8;

        Color32::from_rgba_premultiplied(r, g, b, a)
    }
}

/// Pure-CPU software rasterizer for `egui` meshes.
/// Produces a 32-bit `0x00RRGGBB` pixel buffer compatible with Win32 GDI `StretchDIBits`.
pub struct SoftwareRasterizer {
    textures: HashMap<TextureId, SwTexture>,
    pub fb: Vec<u32>,
    pub width: usize,
    pub height: usize,
}

impl Default for SoftwareRasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SoftwareRasterizer {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            fb: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn apply_textures_delta(&mut self, delta: &TexturesDelta) {
        for (id, image_delta) in &delta.set {
            let [w, h] = image_delta.image.size();
            let new_pixels: Vec<Color32> = match &image_delta.image {
                ImageData::Color(c) => c.pixels.clone(),
                ImageData::Font(f) => f.srgba_pixels(None).collect(),
            };

            if let Some([px, py]) = image_delta.pos {
                if let Some(tex) = self.textures.get_mut(id) {
                    for row in 0..h {
                        let dst_y = py + row;
                        if dst_y >= tex.height {
                            break;
                        }
                        for col in 0..w {
                            let dst_x = px + col;
                            if dst_x >= tex.width {
                                break;
                            }
                            tex.pixels[dst_y * tex.width + dst_x] = new_pixels[row * w + col];
                        }
                    }
                }
            } else {
                self.textures.insert(
                    *id,
                    SwTexture {
                        width: w,
                        height: h,
                        pixels: new_pixels,
                    },
                );
            }
        }

        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    pub fn render(
        &mut self,
        primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
        width: usize,
        height: usize,
        clear_color: Color32,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;

        let total_pixels = width * height;
        if self.fb.len() != total_pixels {
            self.fb.resize(total_pixels, 0);
        }

        let [cr, cg, cb, _] = clear_color.to_array();
        let clear_u32 = ((cr as u32) << 16) | ((cg as u32) << 8) | (cb as u32);
        self.fb.fill(clear_u32);

        for clipped in primitives {
            if let Primitive::Mesh(mesh) = &clipped.primitive {
                self.render_mesh(mesh, &clipped.clip_rect, pixels_per_point);
            }
        }
    }

    fn render_mesh(&mut self, mesh: &Mesh, clip_rect: &Rect, ppp: f32) {
        if mesh.indices.len() < 3 || mesh.vertices.is_empty() {
            return;
        }

        let clip_min_x = ((clip_rect.min.x * ppp).floor() as i32).clamp(0, self.width as i32);
        let clip_min_y = ((clip_rect.min.y * ppp).floor() as i32).clamp(0, self.height as i32);
        let clip_max_x = ((clip_rect.max.x * ppp).ceil() as i32).clamp(0, self.width as i32);
        let clip_max_y = ((clip_rect.max.y * ppp).ceil() as i32).clamp(0, self.height as i32);

        if clip_min_x >= clip_max_x || clip_min_y >= clip_max_y {
            return;
        }

        let tex = self.textures.get(&mesh.texture_id);

        for tri in mesh.indices.chunks_exact(3) {
            let i0 = tri[0] as usize;
            let i1 = tri[1] as usize;
            let i2 = tri[2] as usize;
            if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
                continue;
            }

            let v0 = mesh.vertices[i0];
            let mut v1 = mesh.vertices[i1];
            let mut v2 = mesh.vertices[i2];

            // Skip completely transparent triangles
            if v0.color.a() == 0 && v1.color.a() == 0 && v2.color.a() == 0 {
                continue;
            }

            let p0 = Pos2::new(v0.pos.x * ppp, v0.pos.y * ppp);
            let mut p1 = Pos2::new(v1.pos.x * ppp, v1.pos.y * ppp);
            let mut p2 = Pos2::new(v2.pos.x * ppp, v2.pos.y * ppp);

            let mut area = (p1.x - p0.x) * (p2.y - p0.y) - (p1.y - p0.y) * (p2.x - p0.x);
            if area.abs() < 1e-4 {
                continue;
            }
            if area < 0.0 {
                std::mem::swap(&mut v1, &mut v2);
                std::mem::swap(&mut p1, &mut p2);
                area = -area;
            }
            let inv_area = 1.0 / area;

            let min_x = (p0.x.min(p1.x).min(p2.x).floor() as i32).max(clip_min_x);
            let max_x = (p0.x.max(p1.x).max(p2.x).ceil() as i32).min(clip_max_x);
            let min_y = (p0.y.min(p1.y).min(p2.y).floor() as i32).max(clip_min_y);
            let max_y = (p0.y.max(p1.y).max(p2.y).ceil() as i32).min(clip_max_y);

            if min_x >= max_x || min_y >= max_y {
                continue;
            }

            let is_white_uv = v0.uv.x <= 0.0005
                && v0.uv.y <= 0.0005
                && v1.uv.x <= 0.0005
                && v1.uv.y <= 0.0005
                && v2.uv.x <= 0.0005
                && v2.uv.y <= 0.0005;
            let is_uniform_color = v0.color == v1.color && v1.color == v2.color;

            // Edge function gradients
            let dw0_dx = p1.y - p2.y;
            let dw0_dy = p2.x - p1.x;
            let dw1_dx = p2.y - p0.y;
            let dw1_dy = p0.x - p2.x;
            let dw2_dx = p0.y - p1.y;
            let dw2_dy = p1.x - p0.x;

            let start_x = min_x as f32 + 0.5;
            let start_y = min_y as f32 + 0.5;

            let mut w0_row = (start_x - p1.x) * dw0_dx + (start_y - p1.y) * dw0_dy;
            let mut w1_row = (start_x - p2.x) * dw1_dx + (start_y - p2.y) * dw1_dy;
            let mut w2_row = (start_x - p0.x) * dw2_dx + (start_y - p0.y) * dw2_dy;

            let c0 = v0.color.to_array();
            let c1 = v1.color.to_array();
            let c2 = v2.color.to_array();

            for py in min_y..max_y {
                let mut w0 = w0_row;
                let mut w1 = w1_row;
                let mut w2 = w2_row;
                let row_offset = (py as usize) * self.width;
                let mut entered = false;

                for px in min_x..max_x {
                    if w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4 {
                        entered = true;

                        let [sr, sg, sb, sa] = if is_white_uv && is_uniform_color {
                            c0
                        } else {
                            let b0 = (w0 * inv_area).max(0.0);
                            let b1 = (w1 * inv_area).max(0.0);
                            let b2 = (1.0 - b0 - b1).max(0.0);

                            let [vr, vg, vb, va] = if is_uniform_color {
                                c0
                            } else {
                                [
                                    (c0[0] as f32 * b0 + c1[0] as f32 * b1 + c2[0] as f32 * b2 + 0.5)
                                        as u8,
                                    (c0[1] as f32 * b0 + c1[1] as f32 * b1 + c2[1] as f32 * b2 + 0.5)
                                        as u8,
                                    (c0[2] as f32 * b0 + c1[2] as f32 * b1 + c2[2] as f32 * b2 + 0.5)
                                        as u8,
                                    (c0[3] as f32 * b0 + c1[3] as f32 * b1 + c2[3] as f32 * b2 + 0.5)
                                        as u8,
                                ]
                            };

                            if is_white_uv {
                                [vr, vg, vb, va]
                            } else if let Some(t) = tex {
                                let u = v0.uv.x * b0 + v1.uv.x * b1 + v2.uv.x * b2;
                                let v = v0.uv.y * b0 + v1.uv.y * b1 + v2.uv.y * b2;
                                let [tr, tg, tb, ta] = t.sample_bilinear(u, v).to_array();
                                [
                                    ((vr as u32 * tr as u32 + 127) / 255) as u8,
                                    ((vg as u32 * tg as u32 + 127) / 255) as u8,
                                    ((vb as u32 * tb as u32 + 127) / 255) as u8,
                                    ((va as u32 * ta as u32 + 127) / 255) as u8,
                                ]
                            } else {
                                [vr, vg, vb, va]
                            }
                        };

                        if sa == 255 {
                            self.fb[row_offset + px as usize] =
                                ((sr as u32) << 16) | ((sg as u32) << 8) | (sb as u32);
                        } else if sa > 0 {
                            let dst = self.fb[row_offset + px as usize];
                            let dr = (dst >> 16) & 0xFF;
                            let dg = (dst >> 8) & 0xFF;
                            let db = dst & 0xFF;
                            let inv_a = (255 - sa) as u32;

                            let out_r = (sr as u32 + ((dr * inv_a + 127) / 255)).min(255);
                            let out_g = (sg as u32 + ((dg * inv_a + 127) / 255)).min(255);
                            let out_b = (sb as u32 + ((db * inv_a + 127) / 255)).min(255);

                            self.fb[row_offset + px as usize] =
                                (out_r << 16) | (out_g << 8) | out_b;
                        }
                    } else if entered {
                        // Convexity optimization: once a scanline leaves the triangle, no further pixels on this row can be inside
                        break;
                    }

                    w0 += dw0_dx;
                    w1 += dw1_dx;
                    w2 += dw2_dx;
                }

                w0_row += dw0_dy;
                w1_row += dw1_dy;
                w2_row += dw2_dy;
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod win32_gdi {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;

    #[repr(C)]
    struct BITMAPINFOHEADER {
        bi_size: u32,
        bi_width: i32,
        bi_height: i32,
        bi_planes: u16,
        bi_bit_count: u16,
        bi_compression: u32,
        bi_size_image: u32,
        bi_x_pels_per_meter: i32,
        bi_y_pels_per_meter: i32,
        bi_clr_used: u32,
        bi_clr_important: u32,
    }

    #[repr(C)]
    struct BITMAPINFO {
        bmi_header: BITMAPINFOHEADER,
        bmi_colors: [u32; 1],
    }

    #[link(name = "user32")]
    extern "system" {
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, hdc: isize) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn StretchDIBits(
            hdc: isize,
            x_dest: i32,
            y_dest: i32,
            dest_width: i32,
            dest_height: i32,
            x_src: i32,
            y_src: i32,
            src_width: i32,
            src_height: i32,
            lp_bits: *const c_void,
            lpbmi: *const BITMAPINFO,
            i_usage: u32,
            rop: u32,
        ) -> i32;
    }

    const BI_RGB: u32 = 0;
    const DIB_RGB_COLORS: u32 = 0;
    const SRCCOPY: u32 = 0x00CC_0020;

    pub fn present_frame(
        window: &egui_winit::winit::window::Window,
        fb: &[u32],
        width: usize,
        height: usize,
    ) {
        if width == 0 || height == 0 || fb.len() < width * height {
            return;
        }
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(win32_handle) = handle.as_raw() else {
            return;
        };
        let hwnd = win32_handle.hwnd.get() as isize;

        let bmi = BITMAPINFO {
            bmi_header: BITMAPINFOHEADER {
                bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                bi_width: width as i32,
                // Negative height specifies a top-down DIB with origin at top-left
                bi_height: -(height as i32),
                bi_planes: 1,
                bi_bit_count: 32,
                bi_compression: BI_RGB,
                bi_size_image: (width * height * 4) as u32,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [0],
        };

        unsafe {
            let hdc = GetDC(hwnd);
            if hdc != 0 {
                StretchDIBits(
                    hdc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    fb.as_ptr() as *const c_void,
                    &bmi,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
                ReleaseDC(hwnd, hdc);
            }
        }
    }
}

struct SwWinitHandler {
    egui_ctx: egui::Context,
    app: InboxMoverGuiApp,
    window: Option<Arc<Window>>,
    egui_state: Option<egui_winit::State>,
    rasterizer: SoftwareRasterizer,
    next_repaint_time: Option<Instant>,
}

impl SwWinitHandler {
    fn render_and_present(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(egui_state)) = (self.window.as_ref(), self.egui_state.as_mut())
        else {
            return;
        };

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }

        let raw_input = egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            self.app.render_ui(ctx);
        });

        egui_state.handle_platform_output(window, full_output.platform_output);

        let mut should_close = false;
        if let Some(vp_out) = full_output.viewport_output.get(&ViewportId::ROOT) {
            for cmd in &vp_out.commands {
                if matches!(cmd, ViewportCommand::Close) {
                    should_close = true;
                }
            }
            if vp_out.repaint_delay.is_zero() {
                window.request_redraw();
                self.next_repaint_time = None;
            } else if vp_out.repaint_delay < Duration::from_secs(3600) {
                self.next_repaint_time = Some(Instant::now() + vp_out.repaint_delay);
            } else {
                self.next_repaint_time = None;
            }
        }

        if should_close {
            event_loop.exit();
            return;
        }

        self.rasterizer
            .apply_textures_delta(&full_output.textures_delta);
        let primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let clear_color = self.egui_ctx.style().visuals.panel_fill;
        self.rasterizer.render(
            &primitives,
            full_output.pixels_per_point,
            size.width as usize,
            size.height as usize,
            clear_color,
        );

        #[cfg(target_os = "windows")]
        win32_gdi::present_frame(
            window,
            &self.rasterizer.fb,
            size.width as usize,
            size.height as usize,
        );
    }
}

impl ApplicationHandler<()> for SwWinitHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Inbox Mover — Transfer Folder Processor")
            .with_inner_size(LogicalSize::new(1240.0, 800.0))
            .with_min_inner_size(LogicalSize::new(920.0, 620.0));

        if let Ok(win) = event_loop.create_window(attrs) {
            let win = Arc::new(win);
            let scale_factor = win.scale_factor() as f32;
            let state = egui_winit::State::new(
                self.egui_ctx.clone(),
                ViewportId::ROOT,
                &win,
                Some(scale_factor),
                None,
                Some(4096),
            );
            win.request_redraw();
            self.window = Some(win);
            self.egui_state = Some(state);
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let (Some(window), Some(egui_state)) = (self.window.as_ref(), self.egui_state.as_mut()) {
            let response = egui_state.on_window_event(window, &event);
            if response.repaint {
                window.request_redraw();
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) | WindowEvent::RedrawRequested => {
                self.render_and_present(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.next_repaint_time {
            if Instant::now() >= deadline {
                self.next_repaint_time = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

/// Runs the Material 3 GUI using the built-in pure-CPU software rasterizer and Win32 GDI presenter.
/// Requires zero GPU drivers, zero OpenGL 2.0+, and zero external DLLs on Windows VMs / RDP.
pub fn run_software_gui(core: InboxMoverCore) -> Result<(), String> {
    let egui_ctx = egui::Context::default();
    let app = InboxMoverGuiApp::new_with_ctx(&egui_ctx, core);

    let event_loop = EventLoop::<()>::with_user_event()
        .build()
        .map_err(|e| format!("Failed to create software GUI event loop: {e}"))?;

    let proxy = event_loop.create_proxy();
    egui_ctx.set_request_repaint_callback(move |_info| {
        let _ = proxy.send_event(());
    });

    let mut handler = SwWinitHandler {
        egui_ctx,
        app,
        window: None,
        egui_state: None,
        rasterizer: SoftwareRasterizer::new(),
        next_repaint_time: None,
    };

    event_loop
        .run_app(&mut handler)
        .map_err(|e| format!("Software GUI event loop error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_rasterizer_renders_gui_frame() {
        let temp_dir = std::env::temp_dir().join("inbox_mover_sw_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let core = InboxMoverCore::with_base_dir(&temp_dir);
        let ctx = egui::Context::default();
        let mut app = InboxMoverGuiApp::new_with_ctx(&ctx, core);
        let mut rasterizer = SoftwareRasterizer::new();

        for _ in 0..2 {
            let mut raw_input = egui::RawInput::default();
            // Simulate Windows reporting a Light OS desktop theme while the app is in Dark Mode
            raw_input.system_theme = Some(egui::Theme::Light);
            raw_input.screen_rect =
                Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1240.0, 800.0)));
            let full_output = ctx.run(raw_input, |ctx| {
                app.render_ui(ctx);
            });
            // Verify Dark Mode visuals remain active even if OS reports Light theme
            let style = ctx.style();
            assert!(
                style.visuals.window_fill.r() < 40,
                "Expected dark window_fill in dark mode, got {:?}",
                style.visuals.window_fill
            );
            assert!(
                style.visuals.extreme_bg_color.r() < 40,
                "Expected dark TextEdit extreme_bg_color in dark mode, got {:?}",
                style.visuals.extreme_bg_color
            );
            assert!(
                style.visuals.widgets.noninteractive.fg_stroke.color.r() > 200,
                "Expected light label text color in dark mode, got {:?}",
                style.visuals.widgets.noninteractive.fg_stroke.color
            );
            rasterizer.apply_textures_delta(&full_output.textures_delta);
            let primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
            let clear_color = ctx.style().visuals.panel_fill;
            rasterizer.render(&primitives, full_output.pixels_per_point, 1240, 800, clear_color);
        }

        assert_eq!(rasterizer.fb.len(), 1240 * 800);
        let bg_pixel = rasterizer.fb[0];
        let non_bg_pixels = rasterizer.fb.iter().filter(|&&p| p != bg_pixel).count();
        assert!(
            non_bg_pixels > 10_000,
            "Expected Material 3 GUI to render thousands of non-background pixels, got {non_bg_pixels}"
        );
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

