use std::sync::Arc;

pub const ICON_WIDTH: u32 = 64;
pub const ICON_HEIGHT: u32 = 64;
pub const ICON_64_RGBA: &[u8] = include_bytes!("../assets/icon_64.rgba");

/// Returns `egui::IconData` for `eframe` window/taskbar icons on Windows and Linux.
pub fn eframe_icon_data() -> Arc<egui::IconData> {
    Arc::new(egui::IconData {
        rgba: ICON_64_RGBA.to_vec(),
        width: ICON_WIDTH,
        height: ICON_HEIGHT,
    })
}

/// Returns a `winit::window::Icon` for the built-in CPU/GDI software renderer window.
pub fn winit_window_icon() -> Option<egui_winit::winit::window::Icon> {
    egui_winit::winit::window::Icon::from_rgba(
        ICON_64_RGBA.to_vec(),
        ICON_WIDTH,
        ICON_HEIGHT,
    )
    .ok()
}

/// Returns an `egui::ColorImage` for rendering the custom Inbox Mover logo inside the UI.
pub fn egui_color_image() -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied(
        [ICON_WIDTH as usize, ICON_HEIGHT as usize],
        ICON_64_RGBA,
    )
}
