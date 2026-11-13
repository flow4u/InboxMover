use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Color32, Key, Margin, RichText, Rounding, Sense, Stroke, Vec2};

use crate::core::InboxMoverCore;
use crate::help::{analyze_context, GuidanceLevel, HELP_SECTIONS, QUICK_TIPS, SHORTCUTS};
use crate::models::{ConflictAction, ConfigRule, FolderData, PostAction, ProcessLogEntry};

/// Material 3 Color Palette tokens for Dark and Light themes.
#[derive(Clone, Copy)]
pub struct M3Palette {
    pub bg_surface: Color32,
    pub surface_container_low: Color32,
    pub surface_container: Color32,
    pub surface_container_high: Color32,
    pub surface_container_highest: Color32,
    pub primary: Color32,
    pub on_primary: Color32,
    pub primary_container: Color32,
    pub on_primary_container: Color32,
    pub secondary_container: Color32,
    pub on_secondary_container: Color32,
    pub error: Color32,
    pub error_container: Color32,
    pub on_error_container: Color32,
    pub success: Color32,
    pub success_container: Color32,
    pub warning: Color32,
    pub warning_container: Color32,
    pub on_surface: Color32,
    pub on_surface_variant: Color32,
    pub outline_variant: Color32,
}

impl M3Palette {
    pub fn dark() -> Self {
        Self {
            bg_surface: Color32::from_rgb(17, 19, 24),
            surface_container_low: Color32::from_rgb(25, 28, 34),
            surface_container: Color32::from_rgb(29, 32, 38),
            surface_container_high: Color32::from_rgb(39, 42, 49),
            surface_container_highest: Color32::from_rgb(50, 53, 60),
            primary: Color32::from_rgb(168, 199, 250),
            on_primary: Color32::from_rgb(10, 48, 95),
            primary_container: Color32::from_rgb(40, 71, 119),
            on_primary_container: Color32::from_rgb(214, 227, 255),
            secondary_container: Color32::from_rgb(62, 71, 89),
            on_secondary_container: Color32::from_rgb(218, 226, 249),
            error: Color32::from_rgb(255, 180, 171),
            error_container: Color32::from_rgb(147, 0, 10),
            on_error_container: Color32::from_rgb(255, 218, 214),
            success: Color32::from_rgb(129, 201, 149),
            success_container: Color32::from_rgb(20, 82, 44),
            warning: Color32::from_rgb(253, 214, 99),
            warning_container: Color32::from_rgb(92, 67, 0),
            on_surface: Color32::from_rgb(226, 226, 233),
            on_surface_variant: Color32::from_rgb(196, 198, 208),
            outline_variant: Color32::from_rgb(68, 71, 78),
        }
    }

    pub fn light() -> Self {
        Self {
            bg_surface: Color32::from_rgb(249, 249, 255),
            surface_container_low: Color32::from_rgb(243, 243, 250),
            surface_container: Color32::from_rgb(237, 237, 244),
            surface_container_high: Color32::from_rgb(231, 232, 238),
            surface_container_highest: Color32::from_rgb(225, 226, 233),
            primary: Color32::from_rgb(27, 94, 168),
            on_primary: Color32::from_rgb(255, 255, 255),
            primary_container: Color32::from_rgb(214, 227, 255),
            on_primary_container: Color32::from_rgb(0, 27, 62),
            secondary_container: Color32::from_rgb(218, 226, 249),
            on_secondary_container: Color32::from_rgb(19, 28, 43),
            error: Color32::from_rgb(186, 26, 26),
            error_container: Color32::from_rgb(255, 218, 214),
            on_error_container: Color32::from_rgb(65, 0, 2),
            success: Color32::from_rgb(24, 128, 56),
            success_container: Color32::from_rgb(206, 238, 214),
            warning: Color32::from_rgb(176, 96, 0),
            warning_container: Color32::from_rgb(254, 239, 195),
            on_surface: Color32::from_rgb(25, 28, 32),
            on_surface_variant: Color32::from_rgb(68, 71, 78),
            outline_variant: Color32::from_rgb(196, 198, 208),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HelpTab {
    Guides,
    QuickTips,
    Shortcuts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SaveRuleTarget {
    ConfigId,
    Pattern,
    DefaultRule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LogTab {
    LocalFolder,
    GlobalHistory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ManageRulesTab {
    ConfigIds,
    Patterns,
}

pub struct InboxMoverGuiApp {
    core: InboxMoverCore,
    folders: Vec<FolderData>,
    selected_indices: BTreeSet<usize>,
    primary_index: Option<usize>,

    // Currently edited rule in the right panel
    active_rule: ConfigRule,
    active_rule_source: String,
    is_rule_modified: bool,

    // Search paths input state
    search_dir_1: String,
    search_dir_2: String,

    // Status message bar
    status_message: String,
    status_is_error: bool,

    // Modals & Popups
    show_welcome_modal: bool,
    show_help_modal: bool,
    help_tab: HelpTab,
    selected_help_section: usize,

    show_save_modal: bool,
    save_target_type: SaveRuleTarget,
    save_config_id_input: String,
    save_pattern_input: String,

    show_manage_rules_modal: bool,
    manage_rules_tab: ManageRulesTab,
    manage_selected_id: Option<String>,
    manage_selected_pattern: Option<String>,
    manage_edit_rule: ConfigRule,
    manage_new_id_or_pattern: String,

    show_sync_modal: bool,

    show_create_receipt_modal: bool,
    receipt_permit_id: String,
    receipt_rule: ConfigRule,
    receipt_save_path: String,

    show_log_modal: bool,
    log_tab: LogTab,
    local_log_content: String,
    global_logs: Vec<ProcessLogEntry>,
    log_search_query: String,

    show_delete_confirm_modal: bool,

    show_password_modal: bool,
    password_zip_name: String,
    password_input: String,

    app_icon_tex: Option<egui::TextureHandle>,
}

impl InboxMoverGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, core: InboxMoverCore) -> Self {
        Self::new_with_ctx(&cc.egui_ctx, core)
    }

    pub fn new_with_ctx(ctx: &egui::Context, core: InboxMoverCore) -> Self {
        let search_dir_1 = core.app_settings.search_folder_1.clone();
        let search_dir_2 = core.app_settings.search_folder_2.clone();
        let show_welcome_modal = core.app_settings.show_welcome_splash;

        let mut app = Self {
            core,
            folders: Vec::new(),
            selected_indices: BTreeSet::new(),
            primary_index: None,
            active_rule: ConfigRule::default(),
            active_rule_source: "Default".to_string(),
            is_rule_modified: false,
            search_dir_1,
            search_dir_2,
            status_message: "Ready. Press F1 or '?' anytime for help.".to_string(),
            status_is_error: false,
            show_welcome_modal,
            show_help_modal: false,
            help_tab: HelpTab::Guides,
            selected_help_section: 0,
            show_save_modal: false,
            save_target_type: SaveRuleTarget::ConfigId,
            save_config_id_input: String::new(),
            save_pattern_input: String::new(),
            show_manage_rules_modal: false,
            manage_rules_tab: ManageRulesTab::ConfigIds,
            manage_selected_id: None,
            manage_selected_pattern: None,
            manage_edit_rule: ConfigRule::default(),
            manage_new_id_or_pattern: String::new(),
            show_sync_modal: false,
            show_create_receipt_modal: false,
            receipt_permit_id: String::new(),
            receipt_rule: ConfigRule::default(),
            receipt_save_path: String::new(),
            show_log_modal: false,
            log_tab: LogTab::LocalFolder,
            local_log_content: String::new(),
            global_logs: Vec::new(),
            log_search_query: String::new(),
            show_delete_confirm_modal: false,
            show_password_modal: false,
            password_zip_name: String::new(),
            password_input: String::new(),
            app_icon_tex: None,
        };

        app.apply_theme(ctx);
        app.refresh_folders();
        app
    }

    fn palette(&self) -> M3Palette {
        if self.core.app_settings.dark_mode {
            M3Palette::dark()
        } else {
            M3Palette::light()
        }
    }

    fn open_configured_folder(&mut self, label: &str, raw_path: &str) {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            self.status_message = format!("{label} path is empty.");
            self.status_is_error = true;
            return;
        }
        let resolved = crate::path_translator::PathTranslator::translate(trimmed);
        let p = std::path::Path::new(&resolved);
        if !p.is_dir() {
            self.status_message = format!("{label} does not exist on disk yet: {resolved}");
            self.status_is_error = true;
            return;
        }
        match open::that(p) {
            Ok(()) => {
                self.status_message = format!("Opened {label}: {resolved}");
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = format!("Failed to open {label} ({resolved}): {e}");
                self.status_is_error = true;
            }
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let p = self.palette();
        let theme = if self.core.app_settings.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        ctx.set_theme(theme);

        let mut visuals = if self.core.app_settings.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.panel_fill = p.bg_surface;
        visuals.window_fill = p.surface_container;
        visuals.window_stroke = Stroke::new(1.0_f32, p.outline_variant);
        visuals.faint_bg_color = p.surface_container_low;
        visuals.extreme_bg_color = p.surface_container_low;
        visuals.code_bg_color = p.surface_container_low;
        visuals.override_text_color = Some(p.on_surface);
        visuals.window_rounding = Rounding::same(12.0);
        visuals.menu_rounding = Rounding::same(8.0);

        visuals.widgets.noninteractive.bg_fill = p.surface_container;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.on_surface);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.outline_variant);
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = p.surface_container_high;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.on_surface);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.outline_variant);
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = p.surface_container_highest;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.5_f32, p.primary);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.5_f32, p.primary);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = p.primary_container;
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.on_primary_container);
        visuals.widgets.active.rounding = Rounding::same(8.0);

        visuals.widgets.open.bg_fill = p.surface_container_highest;
        visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.on_surface);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.primary);
        visuals.widgets.open.rounding = Rounding::same(8.0);

        visuals.selection.bg_fill = p.primary_container;
        visuals.selection.stroke = Stroke::new(1.0_f32, p.on_primary_container);

        let scale = (self.core.app_settings.font_size as f32 / 11.0).clamp(0.75, 2.0);
        let mut style = egui::Style {
            visuals,
            ..egui::Style::default()
        };
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        for (_text_style, font_id) in style.text_styles.iter_mut() {
            font_id.size = (font_id.size * scale).clamp(10.0, 28.0);
        }
        ctx.set_style_of(egui::Theme::Dark, style.clone());
        ctx.set_style_of(egui::Theme::Light, style);
    }

    pub fn refresh_folders(&mut self) {
        self.core.app_settings.search_folder_1 = self.search_dir_1.trim().to_string();
        self.core.app_settings.search_folder_2 = self.search_dir_2.trim().to_string();
        self.core.save_settings();
        self.core.reload_cache();

        self.folders = self
            .core
            .scan_search_directories(&self.search_dir_1, &self.search_dir_2);

        if self.folders.is_empty() {
            self.selected_indices.clear();
            self.primary_index = None;
            let (rule, src, modified) = self.core.resolve_rule_for_folder(None);
            self.active_rule = rule;
            self.active_rule_source = src;
            self.is_rule_modified = modified;
            self.status_message =
                "No 'transfer-*' folders found in search directories.".to_string();
            self.status_is_error = false;
        } else {
            let valid_idx = self
                .primary_index
                .filter(|&i| i < self.folders.len())
                .unwrap_or(0);
            self.select_single_folder(valid_idx);
            self.status_message = format!("Loaded {} transfer folder(s).", self.folders.len());
            self.status_is_error = false;
        }
    }

    fn select_single_folder(&mut self, index: usize) {
        if index >= self.folders.len() {
            return;
        }
        self.selected_indices.clear();
        self.selected_indices.insert(index);
        self.primary_index = Some(index);
        self.sync_active_rule_from_selection();
    }

    fn sync_active_rule_from_selection(&mut self) {
        let folder = self.primary_index.and_then(|i| self.folders.get(i));
        let (rule, src, modified) = self.core.resolve_rule_for_folder(folder);
        self.active_rule = rule;
        self.active_rule_source = src;
        self.is_rule_modified = modified;
    }

    fn check_rule_modified(&mut self) {
        let folder = self.primary_index.and_then(|i| self.folders.get(i));
        if let Some(f) = folder {
            if let Some((saved_rule, _)) = self.core.get_saved_rule(f) {
                self.is_rule_modified = !self.active_rule.same_settings(&saved_rule);
            } else {
                self.is_rule_modified = true;
            }
        }
    }

    fn execute_process(&mut self, password_override: Option<String>) {
        if self.selected_indices.is_empty() {
            self.status_message = "No transfer folder selected to process.".to_string();
            self.status_is_error = true;
            return;
        }

        // Collect selected folders sorted oldest to newest
        let mut selected_folders: Vec<FolderData> = self
            .selected_indices
            .iter()
            .filter_map(|&idx| self.folders.get(idx).cloned())
            .collect();
        selected_folders.sort_by_key(|f| f.created_timestamp);

        let is_batch = selected_folders.len() > 1;
        let mut processed_count = 0;
        let mut error_messages = Vec::new();
        let needed_password_zip: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        for folder in &selected_folders {
            let rule_to_use = if is_batch {
                let (r, _, _) = self.core.resolve_rule_for_folder(Some(folder));
                r
            } else {
                self.active_rule.clone()
            };

            let pw_clone = password_override.clone();
            let needed_pw_ref = Arc::clone(&needed_password_zip);

            let res = self.core.process_folder(
                folder,
                &rule_to_use,
                None::<fn(usize, usize, &str)>,
                Some(|zip_name: &str| {
                    if let Some(ref pw) = pw_clone {
                        if !pw.is_empty() {
                            return Some(pw.clone());
                        }
                    }
                    if let Ok(mut guard) = needed_pw_ref.lock() {
                        *guard = Some(zip_name.to_string());
                    }
                    None
                }),
            );

            match res {
                Ok(()) => {
                    processed_count += 1;
                }
                Err(e) => {
                    if let Ok(guard) = needed_password_zip.lock() {
                        if let Some(ref zname) = *guard {
                            self.password_zip_name = zname.clone();
                            self.password_input.clear();
                            self.show_password_modal = true;
                            self.status_message =
                                format!("Password required for encrypted archive '{zname}'.");
                            self.status_is_error = true;
                            return;
                        }
                    }
                    error_messages.push(format!("{}: {}", folder.folder_name, e));
                }
            }
        }

        self.refresh_folders();

        if error_messages.is_empty() {
            self.status_message = format!(
                "Successfully processed {} transfer folder(s).",
                processed_count
            );
            self.status_is_error = false;
        } else {
            self.status_message = format!(
                "Processed {processed_count} folder(s) with {} error(s): {}",
                error_messages.len(),
                error_messages.join(" | ")
            );
            self.status_is_error = true;
        }
    }

    fn open_save_rule_dialog(&mut self) {
        let folder = self.primary_index.and_then(|i| self.folders.get(i));
        if let Some(f) = folder {
            if f.permit_id != "DEFAULT" && f.permit_id != "No_ID" {
                self.save_target_type = SaveRuleTarget::ConfigId;
                self.save_config_id_input = f.permit_id.clone();
            } else {
                self.save_target_type = SaveRuleTarget::Pattern;
                self.save_pattern_input = f.matched_pattern.clone().unwrap_or_else(|| {
                    format!("*{}*", f.folder_name.trim_start_matches("transfer-"))
                });
            }
        } else {
            self.save_target_type = SaveRuleTarget::DefaultRule;
        }
        self.show_save_modal = true;
    }

    fn open_log_viewer(&mut self) {
        let folder = self.primary_index.and_then(|i| self.folders.get(i));
        if let Some(f) = folder {
            let local_log = f.folder_path.join("Process.log");
            let alt_log = f.folder_path.join("Inbox Process.log");
            if local_log.exists() {
                self.local_log_content = std::fs::read_to_string(&local_log).unwrap_or_default();
                self.log_tab = LogTab::LocalFolder;
            } else if alt_log.exists() {
                self.local_log_content = std::fs::read_to_string(&alt_log).unwrap_or_default();
                self.log_tab = LogTab::LocalFolder;
            } else {
                self.local_log_content =
                    "No local Process.log file exists inside this folder yet.".to_string();
                self.log_tab = LogTab::GlobalHistory;
            }
        } else {
            self.local_log_content = "No transfer folder selected.".to_string();
            self.log_tab = LogTab::GlobalHistory;
        }
        self.global_logs = self.core.read_global_logs(250);
        self.show_log_modal = true;
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let any_modal = self.show_welcome_modal
            || self.show_help_modal
            || self.show_save_modal
            || self.show_manage_rules_modal
            || self.show_sync_modal
            || self.show_create_receipt_modal
            || self.show_log_modal
            || self.show_delete_confirm_modal
            || self.show_password_modal;

        if ctx.input(|i| i.key_pressed(Key::Escape)) && any_modal {
            self.show_welcome_modal = false;
            self.show_help_modal = false;
            self.show_save_modal = false;
            self.show_manage_rules_modal = false;
            self.show_sync_modal = false;
            self.show_create_receipt_modal = false;
            self.show_log_modal = false;
            self.show_delete_confirm_modal = false;
            self.show_password_modal = false;
            return;
        }

        if any_modal || ctx.wants_keyboard_input() {
            return;
        }

        if ctx.input(|i| {
            i.key_pressed(Key::F1) || (i.modifiers.shift && i.key_pressed(Key::Questionmark))
        }) {
            self.show_help_modal = !self.show_help_modal;
        }
        if ctx.input(|i| i.key_pressed(Key::F5) || (i.modifiers.command && i.key_pressed(Key::R))) {
            self.refresh_folders();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
            self.open_save_rule_dialog();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::L)) {
            self.open_log_viewer();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::M)) {
            self.show_manage_rules_modal = true;
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::G)) {
            self.core.app_settings.show_guidance = !self.core.app_settings.show_guidance;
            self.core.save_settings();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O)) {
            if let Some(f) = self.primary_index.and_then(|idx| self.folders.get(idx)) {
                let _ = open::that(&f.folder_path);
            }
        }
        if ctx.input(|i| i.key_pressed(Key::Delete)) && !self.selected_indices.is_empty() {
            self.show_delete_confirm_modal = true;
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) && !self.selected_indices.is_empty() {
            self.execute_process(None);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !self.folders.is_empty() {
            let next = self
                .primary_index
                .map(|idx| (idx + 1).min(self.folders.len() - 1))
                .unwrap_or(0);
            self.select_single_folder(next);
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !self.folders.is_empty() {
            let prev = self
                .primary_index
                .map(|idx| idx.saturating_sub(1))
                .unwrap_or(0);
            self.select_single_folder(prev);
        }
    }

    pub fn render_ui(&mut self, ctx: &egui::Context) {
        self.apply_theme(ctx);
        self.handle_keyboard_shortcuts(ctx);
        let p = self.palette();
        let is_advanced = !self.core.app_settings.simple_mode;

        let app_icon_id = self
            .app_icon_tex
            .get_or_insert_with(|| {
                ctx.load_texture(
                    "inbox_mover_app_icon",
                    crate::icon::egui_color_image(),
                    egui::TextureOptions::LINEAR,
                )
            })
            .id();

        // ====================================================================
        // TOP HEADER BAR (Material 3 Top App Bar)
        // ====================================================================
        egui::TopBottomPanel::top("m3_top_app_bar")
            .frame(
                egui::Frame::none()
                    .fill(p.surface_container)
                    .inner_margin(Margin::symmetric(16.0, 10.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                    ui.painter().image(
                        app_icon_id,
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    egui::Frame::none()
                        .fill(p.primary_container)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("INBOX MOVER")
                                    .strong()
                                    .color(p.on_primary_container),
                            );
                        });

                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(p.on_surface_variant),
                    );

                    ui.separator();

                    // Mode Switcher: Simple Mode (Noobs/Streamlined) vs Advanced Mode (Power Users)
                    let simple_btn = egui::Button::new(
                        RichText::new("✨ Simple Mode").color(if !is_advanced {
                            p.on_primary
                        } else {
                            p.on_surface_variant
                        }),
                    )
                    .fill(if !is_advanced {
                        p.primary
                    } else {
                        p.surface_container_high
                    });

                    if ui
                        .add(simple_btn)
                        .on_hover_text("Streamlined 3-step view for quick daily use or beginners")
                        .clicked()
                    {
                        self.core.app_settings.simple_mode = true;
                        self.core.save_settings();
                    }

                    let adv_btn = egui::Button::new(
                        RichText::new("⚙ Advanced Mode").color(if is_advanced {
                            p.on_primary
                        } else {
                            p.on_surface_variant
                        }),
                    )
                    .fill(if is_advanced {
                        p.primary
                    } else {
                        p.surface_container_high
                    });

                    if ui
                        .add(adv_btn)
                        .on_hover_text(
                            "Full power-user controls: Team Workspace sync, Pattern rules, Overrule receipt, and Dual Search",
                        )
                        .clicked()
                    {
                        self.core.app_settings.simple_mode = false;
                        self.core.save_settings();
                    }

                    // Right-aligned utilities: Smart Guide toggle, Font Scaling, Theme, Help Center
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let help_btn = egui::Button::new(
                            RichText::new("❓ Help & Shortcuts (F1)")
                                .strong()
                                .color(p.on_primary_container),
                        )
                        .fill(p.primary_container);
                        if ui
                            .add(help_btn)
                            .on_hover_text(
                                "Open interactive Help Center, Quick Tips, and Keyboard Shortcuts (F1)",
                            )
                            .clicked()
                        {
                            self.show_help_modal = true;
                        }

                        // Dark / Light mode toggle
                        let theme_label = if self.core.app_settings.dark_mode {
                            "☀ Light"
                        } else {
                            "🌙 Dark"
                        };
                        if ui
                            .button(theme_label)
                            .on_hover_text("Switch between Material 3 Dark and Light themes")
                            .clicked()
                        {
                            self.core.app_settings.dark_mode = !self.core.app_settings.dark_mode;
                            self.core.save_settings();
                            self.apply_theme(ctx);
                        }

                        // Font scaling controls
                        if ui
                            .button("A+")
                            .on_hover_text("Increase UI font size")
                            .clicked()
                        {
                            self.core.app_settings.font_size =
                                (self.core.app_settings.font_size + 1).min(20);
                            self.core.save_settings();
                            self.apply_theme(ctx);
                        }
                        if ui
                            .button("A-")
                            .on_hover_text("Decrease UI font size")
                            .clicked()
                        {
                            self.core.app_settings.font_size =
                                (self.core.app_settings.font_size - 1).max(9);
                            self.core.save_settings();
                            self.apply_theme(ctx);
                        }

                        // Smart Guidance toggle (so advanced users can silence guidance completely)
                        let guide_on = self.core.app_settings.show_guidance;
                        let guide_text = if guide_on {
                            "💡 Guide: ON"
                        } else {
                            "💡 Guide: OFF"
                        };
                        let guide_btn = egui::Button::new(RichText::new(guide_text).color(
                            if guide_on {
                                p.on_secondary_container
                            } else {
                                p.on_surface_variant
                            },
                        ))
                        .fill(if guide_on {
                            p.secondary_container
                        } else {
                            p.surface_container_high
                        });
                        if ui
                            .add(guide_btn)
                            .on_hover_text(
                                "Toggle the contextual Smart Guidance banner on/off (Ctrl+G). Turn OFF if you're an experienced user!",
                            )
                            .clicked()
                        {
                            self.core.app_settings.show_guidance = !guide_on;
                            self.core.save_settings();
                        }
                    });
                });
            });

        // ====================================================================
        // BOTTOM STATUS & SHORTCUT FOOTER BAR
        // ====================================================================
        let font_scale = (self.core.app_settings.font_size as f32 / 11.0).clamp(0.75, 2.0);
        egui::TopBottomPanel::bottom("m3_bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(p.surface_container)
                    .inner_margin(Margin::symmetric(16.0, 8.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant)),
            )
            .show(ctx, |ui| {
                let status_color = if self.status_is_error {
                    p.error
                } else {
                    p.success
                };
                let shortcuts_str =
                    "Shortcuts: [Up/Down] Select  [Enter] Process  [Ctrl+S] Save Rule  [Del] Delete  [F5] Refresh  [F1] Help";

                if ui.available_width() >= 860.0 * font_scale {
                    ui.horizontal(|ui| {
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 4.5, status_color);
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.status_message).color(p.on_surface),
                            )
                            .truncate(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(shortcuts_str)
                                    .small()
                                    .color(p.on_surface_variant),
                            );
                        });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
                        ui.painter()
                            .circle_filled(dot_rect.center(), 4.5, status_color);
                        ui.label(RichText::new(&self.status_message).color(p.on_surface));
                        ui.separator();
                        ui.label(
                            RichText::new(shortcuts_str)
                                .small()
                                .color(p.on_surface_variant),
                        );
                    });
                }
            });

        // ====================================================================
        // LEFT SIDEBAR: WORKSPACE, SEARCH DIRECTORIES & TRANSFER QUEUE
        // ====================================================================
        let max_sidebar_w = (ctx.screen_rect().width() * 0.40).clamp(300.0, 460.0);
        egui::SidePanel::left("m3_left_sidebar")
            .default_width(360.0)
            .min_width(290.0)
            .max_width(max_sidebar_w)
            .frame(
                egui::Frame::none()
                    .fill(p.surface_container_low)
                    .inner_margin(Margin::symmetric(12.0, 12.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant)),
            )
            .show(ctx, |ui| {
                let sidebar_w = ui.available_width().max(240.0);
                ui.set_max_width(sidebar_w);

                if is_advanced {
                    egui::Frame::none()
                        .fill(p.surface_container)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(10.0))
                        .stroke(Stroke::new(1.0_f32, p.outline_variant))
                        .show(ui, |ui| {
                            let ws_inner_w = (sidebar_w - 22.0).max(200.0);
                            ui.set_min_width(ws_inner_w);
                            ui.set_max_width(ws_inner_w);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Workspace:").strong());
                                let is_personal = !self.core.app_settings.use_global;
                                if ui
                                    .selectable_label(is_personal, "👤 Personal")
                                    .on_hover_text("Use your private rules in permit_configs/")
                                    .clicked()
                                {
                                    self.core.switch_workspace("personal");
                                    self.refresh_folders();
                                }
                                if ui
                                    .selectable_label(!is_personal, "👥 Team")
                                    .on_hover_text("Use shared team rules")
                                    .clicked()
                                {
                                    self.core.switch_workspace("team");
                                    self.refresh_folders();
                                }

                                if ui
                                    .small_button("⟳ Sync")
                                    .on_hover_text(
                                        "Merge local Personal rules into Team workspace",
                                    )
                                    .clicked()
                                {
                                    self.show_sync_modal = true;
                                }
                            });

                            ui.add_space(4.0);
                            let mut overrule = self.core.app_settings.overrule_receipt;
                            if ui
                                .checkbox(
                                    &mut overrule,
                                    "Overrule receipt.json with saved rules",
                                )
                                .on_hover_text(
                                    "When checked, your saved Config ID or Pattern rules take priority over paths inside incoming receipt.json files",
                                )
                                .changed()
                            {
                                self.core.app_settings.overrule_receipt = overrule;
                                self.core.save_settings();
                                self.sync_active_rule_from_selection();
                            }
                        });
                    ui.add_space(8.0);
                }

                // 2. Search Directories Card
                egui::Frame::none()
                    .fill(p.surface_container)
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::same(10.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant))
                    .show(ui, |ui| {
                        let search_inner_w = (sidebar_w - 22.0).max(200.0);
                        ui.set_min_width(search_inner_w);
                        ui.set_max_width(search_inner_w);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("🔍 Inbox Search Folders").strong());
                            if ui
                                .button("⟳ Refresh (F5)")
                                .on_hover_text("Scan search directories for transfer-* folders")
                                .clicked()
                            {
                                self.refresh_folders();
                            }
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Search 1:");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button("Browse")
                                        .on_hover_text("Pick primary Inbox directory")
                                        .clicked()
                                    {
                                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                            self.search_dir_1 =
                                                folder.to_string_lossy().to_string();
                                            self.refresh_folders();
                                        }
                                    }
                                    let edit_w = ui.available_width().max(60.0);
                                    let resp = ui.add_sized(
                                        [edit_w, 24.0],
                                        egui::TextEdit::singleline(&mut self.search_dir_1)
                                            .hint_text("Primary Inbox path..."),
                                    );
                                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
                                    {
                                        self.refresh_folders();
                                    }
                                },
                            );
                        });

                        if is_advanced || !self.search_dir_2.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Search 2:");
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button("Browse")
                                            .on_hover_text(
                                                "Pick optional secondary Inbox directory",
                                            )
                                            .clicked()
                                        {
                                            if let Some(folder) =
                                                rfd::FileDialog::new().pick_folder()
                                            {
                                                self.search_dir_2 =
                                                    folder.to_string_lossy().to_string();
                                                self.refresh_folders();
                                            }
                                        }
                                        let edit_w = ui.available_width().max(60.0);
                                        let resp = ui.add_sized(
                                            [edit_w, 24.0],
                                            egui::TextEdit::singleline(&mut self.search_dir_2)
                                                .hint_text("Optional secondary path..."),
                                        );
                                        if resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(Key::Enter))
                                        {
                                            self.refresh_folders();
                                        }
                                    },
                                );
                            });
                        }
                    });

                ui.add_space(8.0);

                // 3. Queue Header & Wrapping Colored Status Legend (never cuts off!)
                ui.label(
                    RichText::new(format!("Transfer Queue ({})", self.folders.len())).strong(),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(RichText::new("✔ Ready").small().color(p.success))
                        .on_hover_text("✔ = Configured & ready to process");
                    ui.label(RichText::new("•").small().color(p.on_surface_variant));
                    ui.label(RichText::new("✔(L) Logged").small().color(p.warning))
                        .on_hover_text(
                            "✔(L) = Contains local Process.log from a previous run",
                        );
                    ui.label(RichText::new("•").small().color(p.on_surface_variant));
                    ui.label(RichText::new("✖ Setup").small().color(p.error))
                        .on_hover_text("✖ = Needs a Target Folder before processing");
                });

                ui.add_space(4.0);

                // 4. Scrollable Transfer Folder Queue List
                egui::ScrollArea::vertical()
                    .id_salt("transfer_queue_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let queue_list_w = (ui.available_width() - 8.0).max(180.0);
                        ui.set_max_width(queue_list_w);

                        if self.folders.is_empty() {
                            egui::Frame::none()
                                .fill(p.surface_container)
                                .rounding(Rounding::same(10.0))
                                .inner_margin(Margin::same(16.0))
                                .show(ui, |ui| {
                                    ui.set_max_width((queue_list_w - 32.0).max(140.0));
                                    ui.label(
                                        RichText::new("No 'transfer-*' folders found.")
                                            .strong()
                                            .color(p.on_surface_variant),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(
                                            "Click 'Browse' above to select a folder containing transfer-* packages, then click Refresh.",
                                        )
                                        .small()
                                        .color(p.on_surface_variant),
                                    );
                                });
                            return;
                        }

                        let mut clicked_idx: Option<(usize, bool, bool)> = None;
                        let card_inner_w = (queue_list_w - 22.0).max(150.0);

                        for (idx, folder) in self.folders.iter().enumerate() {
                            let is_selected = self.selected_indices.contains(&idx);
                            let indicator = self.core.get_folder_status_indicator(folder);

                            let (badge_color, badge_bg) = match indicator {
                                "✔" => (p.success, p.success_container),
                                "✔ (L)" => (p.warning, p.warning_container),
                                _ => (p.error, p.error_container),
                            };

                            let card_fill = if is_selected {
                                p.primary_container
                            } else {
                                p.surface_container
                            };
                            let text_color = if is_selected {
                                p.on_primary_container
                            } else {
                                p.on_surface
                            };

                            let frame = egui::Frame::none()
                                .fill(card_fill)
                                .rounding(Rounding::same(8.0))
                                .inner_margin(Margin::symmetric(10.0, 7.0))
                                .stroke(Stroke::new(
                                    if is_selected { 1.5_f32 } else { 1.0_f32 },
                                    if is_selected {
                                        p.primary
                                    } else {
                                        p.outline_variant
                                    },
                                ));

                            let resp = frame
                                .show(ui, |ui| {
                                    ui.set_min_width(card_inner_w);
                                    ui.set_max_width(card_inner_w);
                                    ui.horizontal(|ui| {
                                        egui::Frame::none()
                                            .fill(badge_bg)
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(indicator)
                                                        .small()
                                                        .strong()
                                                        .color(badge_color),
                                                );
                                            });

                                        let text_avail = ui.available_width().max(80.0);
                                        ui.vertical(|ui| {
                                            ui.set_max_width(text_avail);
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&folder.folder_name)
                                                        .strong()
                                                        .color(text_color),
                                                )
                                                .truncate(),
                                            );
                                            let sub = format!(
                                                "{} • ID: {} • {} file(s)",
                                                folder.created_date_str,
                                                folder.permit_id,
                                                folder.file_list.len()
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(sub).small().color(
                                                        if is_selected {
                                                            p.on_primary_container
                                                        } else {
                                                            p.on_surface_variant
                                                        },
                                                    ),
                                                )
                                                .truncate(),
                                            );
                                        });
                                    });
                                })
                                .response
                                .interact(Sense::click())
                                .on_hover_text(&folder.folder_name);

                            if resp.clicked() {
                                let modifiers = ui.input(|i| i.modifiers);
                                clicked_idx = Some((
                                    idx,
                                    modifiers.command || modifiers.ctrl,
                                    modifiers.shift,
                                ));
                            }
                            ui.add_space(4.0);
                        }

                        if let Some((idx, ctrl, shift)) = clicked_idx {
                            if shift {
                                let anchor = self.primary_index.unwrap_or(0);
                                let start = anchor.min(idx);
                                let end = anchor.max(idx);
                                self.selected_indices.clear();
                                for i in start..=end {
                                    self.selected_indices.insert(i);
                                }
                                self.primary_index = Some(idx);
                                self.sync_active_rule_from_selection();
                            } else if ctrl {
                                if self.selected_indices.contains(&idx)
                                    && self.selected_indices.len() > 1
                                {
                                    self.selected_indices.remove(&idx);
                                    self.primary_index =
                                        self.selected_indices.iter().next().copied();
                                } else {
                                    self.selected_indices.insert(idx);
                                    self.primary_index = Some(idx);
                                }
                                self.sync_active_rule_from_selection();
                            } else {
                                self.select_single_folder(idx);
                            }
                        }
                    });
            });

        // ====================================================================
        // MAIN CENTRAL PANEL: SMART GUIDANCE, DESTINATIONS, OPTIONS & INSPECTOR
        // ====================================================================
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_surface)
                    .inner_margin(Margin::symmetric(18.0, 14.0)),
            )
            .show(ctx, |ui| {
                let outer_avail_w = ui.available_width().max(320.0);
                egui::ScrollArea::vertical()
                    .id_salt("central_main_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Reserve scrollbar gutter and strictly cap content width so no child row or
                        // ComboBox can ever stretch the ScrollArea past the visible right window edge.
                        let panel_width = (outer_avail_w - 10.0).max(300.0);
                        ui.set_min_width(panel_width);
                        ui.set_max_width(panel_width);

                        let card_inner_w = (panel_width - 30.0).max(240.0);
                        let dest_inner_w = (panel_width - 34.0).max(240.0);
                        let label_col_w = (132.0 * font_scale.min(1.3)).clamp(110.0, 175.0);

                        let current_folder = self
                            .primary_index
                            .and_then(|idx| self.folders.get(idx))
                            .cloned();

                        // 1. Contextual Smart Guidance Card (Toggleable so advanced users aren't bothered)
                        if self.core.app_settings.show_guidance {
                            let guidance = analyze_context(
                                current_folder.as_ref(),
                                &self.active_rule,
                                &self.active_rule_source,
                                self.is_rule_modified,
                                self.core.app_settings.overrule_receipt,
                            );

                            let (banner_bg, banner_accent, icon) = match guidance.level {
                                GuidanceLevel::Ready => (p.success_container, p.success, "✅"),
                                GuidanceLevel::Modified => {
                                    (p.secondary_container, p.primary, "📝")
                                }
                                GuidanceLevel::AttentionRequired => {
                                    (p.error_container, p.error, "⚠")
                                }
                                GuidanceLevel::PreviouslyProcessed => {
                                    (p.warning_container, p.warning, "🕘")
                                }
                                GuidanceLevel::Idle => {
                                    (p.surface_container_high, p.on_surface_variant, "💡")
                                }
                            };

                            egui::Frame::none()
                                .fill(banner_bg)
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::symmetric(14.0, 10.0))
                                .stroke(Stroke::new(1.0_f32, banner_accent))
                                .show(ui, |ui| {
                                    ui.set_min_width(card_inner_w);
                                    ui.set_max_width(card_inner_w);
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("✖ Hide Guide")
                                                    .on_hover_text(
                                                        "Hide this guidance banner (re-enable anytime via '💡 Guide' in the top bar or Ctrl+G)",
                                                    )
                                                    .clicked()
                                                {
                                                    self.core.app_settings.show_guidance = false;
                                                    self.core.save_settings();
                                                }
                                                ui.with_layout(
                                                    egui::Layout::left_to_right(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(format!(
                                                                    "{icon} {}",
                                                                    guidance.headline
                                                                ))
                                                                .strong()
                                                                .color(p.on_surface),
                                                            )
                                                            .wrap(),
                                                        );
                                                    },
                                                );
                                            },
                                        );
                                    });
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&guidance.explanation)
                                                .small()
                                                .color(p.on_surface),
                                        )
                                        .wrap(),
                                    );
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&guidance.next_step_hint)
                                                .small()
                                                .strong()
                                                .color(banner_accent),
                                        )
                                        .wrap(),
                                    );
                                });

                            ui.add_space(12.0);
                        }

                        // 2. Selected Folder Metadata Header Card
                        egui::Frame::none()
                            .fill(p.surface_container)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .stroke(Stroke::new(1.0_f32, p.outline_variant))
                            .show(ui, |ui| {
                                ui.set_min_width(card_inner_w);
                                ui.set_max_width(card_inner_w);
                                ui.horizontal_wrapped(|ui| {
                                    if self.selected_indices.len() > 1 {
                                        ui.label(
                                            RichText::new(format!(
                                                "📚 Batch Selection: {} Folders Selected (Processes Oldest -> Newest)",
                                                self.selected_indices.len()
                                            ))
                                            .strong()
                                            .color(p.primary),
                                        );
                                    } else if let Some(ref f) = current_folder {
                                        ui.label(
                                            RichText::new(format!("📂 {}", f.folder_name))
                                                .strong()
                                                .size(16.0),
                                        );
                                        ui.separator();
                                        ui.label(
                                            RichText::new(format!("Config ID: {}", f.permit_id))
                                                .color(p.primary),
                                        );
                                        ui.separator();
                                        ui.label(
                                            RichText::new(format!(
                                                "Rule Source: {}",
                                                self.active_rule_source
                                            ))
                                            .color(p.on_surface_variant),
                                        );
                                        ui.separator();
                                        ui.label(
                                            RichText::new(format!(
                                                "Created: {}",
                                                f.created_date_str
                                            ))
                                            .color(p.on_surface_variant),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("No Transfer Folder Selected")
                                                .strong()
                                                .color(p.on_surface_variant),
                                        );
                                    }
                                });
                            });

                        ui.add_space(12.0);

                        // 3. Destinations & Processing Rules Card
                        egui::Frame::none()
                            .fill(p.surface_container)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(16.0, 14.0))
                            .stroke(Stroke::new(1.0_f32, p.outline_variant))
                            .show(ui, |ui| {
                                ui.set_min_width(dest_inner_w);
                                ui.set_max_width(dest_inner_w);
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(
                                        RichText::new("🎯 Destinations & Processing Options")
                                            .strong(),
                                    );
                                    if self.is_rule_modified {
                                        ui.label(
                                            RichText::new("• Modified (Unsaved)")
                                                .small()
                                                .color(p.warning),
                                        )
                                        .on_hover_text(
                                            "Current settings differ from your saved workspace rule. Click 'Save Rule *' to keep them.",
                                        );
                                    }
                                });
                                ui.add_space(6.0);

                                // Target Folder Row (Browse + Open anchored right so they never clip)
                                let mut open_folder_req: Option<(&'static str, String)> = None;
                                ui.horizontal(|ui| {
                                    ui.sized_label(
                                        label_col_w,
                                        "Target Folder *:",
                                        "Where extracted ZIP contents and loose files are placed (Required)",
                                        &p,
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let has_target =
                                                !self.active_rule.target_folder.trim().is_empty();
                                            if ui
                                                .add_enabled(
                                                    has_target,
                                                    egui::Button::new("📂 Open"),
                                                )
                                                .on_hover_text(
                                                    "Open Target Folder in your file manager",
                                                )
                                                .on_disabled_hover_text(
                                                    "Set a Target Folder path first",
                                                )
                                                .clicked()
                                            {
                                                open_folder_req = Some((
                                                    "Target Folder",
                                                    self.active_rule.target_folder.clone(),
                                                ));
                                            }
                                            if ui
                                                .button("Browse")
                                                .on_hover_text("Select Target Folder")
                                                .clicked()
                                            {
                                                if let Some(dir) =
                                                    rfd::FileDialog::new().pick_folder()
                                                {
                                                    self.active_rule.target_folder =
                                                        dir.to_string_lossy().to_string();
                                                    self.check_rule_modified();
                                                }
                                            }
                                            let edit_w = ui.available_width().max(80.0);
                                            if ui
                                                .add_sized(
                                                    [edit_w, 24.0],
                                                    egui::TextEdit::singleline(
                                                        &mut self.active_rule.target_folder,
                                                    )
                                                    .hint_text(
                                                        "Required: Destination path for extracted/copied files...",
                                                    ),
                                                )
                                                .changed()
                                            {
                                                self.check_rule_modified();
                                            }
                                        },
                                    );
                                });

                                // Processed Folder Row
                                let need_processed =
                                    self.active_rule.post_action == PostAction::Move;
                                ui.horizontal(|ui| {
                                    ui.sized_label(
                                        label_col_w,
                                        if need_processed {
                                            "Processed Folder *:"
                                        } else {
                                            "Processed Folder:"
                                        },
                                        "Where the source transfer-* folder is moved after processing (Required when After Processing = Move)",
                                        &p,
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let has_proc = !self
                                                .active_rule
                                                .target_zip_folder
                                                .trim()
                                                .is_empty();
                                            if ui
                                                .add_enabled(
                                                    has_proc,
                                                    egui::Button::new("📂 Open"),
                                                )
                                                .on_hover_text(
                                                    "Open Processed Folder in your file manager",
                                                )
                                                .on_disabled_hover_text(
                                                    "Set a Processed Folder path first",
                                                )
                                                .clicked()
                                            {
                                                open_folder_req = Some((
                                                    "Processed Folder",
                                                    self.active_rule.target_zip_folder.clone(),
                                                ));
                                            }
                                            if ui
                                                .button("Browse")
                                                .on_hover_text("Select Processed Folder")
                                                .clicked()
                                            {
                                                if let Some(dir) =
                                                    rfd::FileDialog::new().pick_folder()
                                                {
                                                    self.active_rule.target_zip_folder =
                                                        dir.to_string_lossy().to_string();
                                                    self.check_rule_modified();
                                                }
                                            }
                                            let edit_w = ui.available_width().max(80.0);
                                            if ui
                                                .add_sized(
                                                    [edit_w, 24.0],
                                                    egui::TextEdit::singleline(
                                                        &mut self.active_rule.target_zip_folder,
                                                    )
                                                    .hint_text(
                                                        "Archive folder for processed transfer-* packages...",
                                                    ),
                                                )
                                                .changed()
                                            {
                                                self.check_rule_modified();
                                            }
                                        },
                                    );
                                });

                                // Receipt Folder Row
                                ui.horizontal(|ui| {
                                    ui.sized_label(
                                        label_col_w,
                                        "Receipt Folder:",
                                        "Optional: Separate folder to store timestamped receipt*.json and manifest*.json files. Leave blank to place inside Target Folder.",
                                        &p,
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let has_rec =
                                                !self.active_rule.receipt_folder.trim().is_empty();
                                            if ui
                                                .add_enabled(
                                                    has_rec,
                                                    egui::Button::new("📂 Open"),
                                                )
                                                .on_hover_text(
                                                    "Open Receipt Folder in your file manager",
                                                )
                                                .on_disabled_hover_text(
                                                    "Set a Receipt Folder path first",
                                                )
                                                .clicked()
                                            {
                                                open_folder_req = Some((
                                                    "Receipt Folder",
                                                    self.active_rule.receipt_folder.clone(),
                                                ));
                                            }
                                            if ui
                                                .button("Browse")
                                                .on_hover_text("Select Receipt Folder")
                                                .clicked()
                                            {
                                                if let Some(dir) =
                                                    rfd::FileDialog::new().pick_folder()
                                                {
                                                    self.active_rule.receipt_folder =
                                                        dir.to_string_lossy().to_string();
                                                    self.check_rule_modified();
                                                }
                                            }
                                            let edit_w = ui.available_width().max(80.0);
                                            if ui
                                                .add_sized(
                                                    [edit_w, 24.0],
                                                    egui::TextEdit::singleline(
                                                        &mut self.active_rule.receipt_folder,
                                                    )
                                                    .hint_text(
                                                        "Optional: Separate folder for receipt.json / manifest.json files...",
                                                    ),
                                                )
                                                .changed()
                                            {
                                                self.check_rule_modified();
                                            }
                                        },
                                    );
                                });

                                if let Some((lbl, path_to_open)) = open_folder_req {
                                    self.open_configured_folder(lbl, &path_to_open);
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                // Processing Options (Responsive so ComboBoxes never overflow right edge!)
                                let combo_w = (172.0 * font_scale.min(1.2)).clamp(140.0, 210.0);
                                if dest_inner_w >= 840.0 * font_scale {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .checkbox(
                                                &mut self.active_rule.auto_extract,
                                                "Auto-extract ZIP archives",
                                            )
                                            .on_hover_text(
                                                "When enabled, .zip archives are extracted into Target Folder (and redundant transfer-* root directories inside the ZIP are stripped). When disabled, .zip files are copied as-is.",
                                            )
                                            .changed()
                                        {
                                            self.check_rule_modified();
                                        }

                                        ui.add_space(12.0);
                                        ui.label(RichText::new("If File Exists:").strong());
                                        egui::ComboBox::from_id_salt("conflict_action_combo")
                                            .width(combo_w)
                                            .selected_text(self.active_rule.conflict_action.label())
                                            .show_ui(ui, |ui| {
                                                for item in ConflictAction::ALL {
                                                    if ui
                                                        .selectable_value(
                                                            &mut self.active_rule.conflict_action,
                                                            item,
                                                            item.label(),
                                                        )
                                                        .changed()
                                                    {
                                                        self.check_rule_modified();
                                                    }
                                                }
                                            });

                                        ui.add_space(12.0);
                                        ui.label(RichText::new("After Processing:").strong());
                                        egui::ComboBox::from_id_salt("post_action_combo")
                                            .width(combo_w)
                                            .selected_text(self.active_rule.post_action.label())
                                            .show_ui(ui, |ui| {
                                                for item in PostAction::ALL {
                                                    if ui
                                                        .selectable_value(
                                                            &mut self.active_rule.post_action,
                                                            item,
                                                            item.label(),
                                                        )
                                                        .changed()
                                                    {
                                                        self.check_rule_modified();
                                                    }
                                                }
                                            });
                                    });
                                } else {
                                    if ui
                                        .checkbox(
                                            &mut self.active_rule.auto_extract,
                                            "Auto-extract ZIP archives",
                                        )
                                        .on_hover_text(
                                            "When enabled, .zip archives are extracted into Target Folder (and redundant transfer-* root directories inside the ZIP are stripped). When disabled, .zip files are copied as-is.",
                                        )
                                        .changed()
                                    {
                                        self.check_rule_modified();
                                    }
                                    ui.add_space(4.0);
                                    if dest_inner_w >= 600.0 * font_scale {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("If File Exists:").strong());
                                            egui::ComboBox::from_id_salt("conflict_action_combo")
                                                .width(combo_w)
                                                .selected_text(
                                                    self.active_rule.conflict_action.label(),
                                                )
                                                .show_ui(ui, |ui| {
                                                    for item in ConflictAction::ALL {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self
                                                                    .active_rule
                                                                    .conflict_action,
                                                                item,
                                                                item.label(),
                                                            )
                                                            .changed()
                                                        {
                                                            self.check_rule_modified();
                                                        }
                                                    }
                                                });

                                            ui.add_space(12.0);
                                            ui.label(RichText::new("After Processing:").strong());
                                            egui::ComboBox::from_id_salt("post_action_combo")
                                                .width(combo_w)
                                                .selected_text(self.active_rule.post_action.label())
                                                .show_ui(ui, |ui| {
                                                    for item in PostAction::ALL {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self.active_rule.post_action,
                                                                item,
                                                                item.label(),
                                                            )
                                                            .changed()
                                                        {
                                                            self.check_rule_modified();
                                                        }
                                                    }
                                                });
                                        });
                                    } else {
                                        ui.horizontal(|ui| {
                                            ui.sized_label(
                                                label_col_w,
                                                "If File Exists:",
                                                "How to resolve filename collisions in Target Folder",
                                                &p,
                                            );
                                            egui::ComboBox::from_id_salt("conflict_action_combo")
                                                .width(combo_w)
                                                .selected_text(
                                                    self.active_rule.conflict_action.label(),
                                                )
                                                .show_ui(ui, |ui| {
                                                    for item in ConflictAction::ALL {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self
                                                                    .active_rule
                                                                    .conflict_action,
                                                                item,
                                                                item.label(),
                                                            )
                                                            .changed()
                                                        {
                                                            self.check_rule_modified();
                                                        }
                                                    }
                                                });
                                        });
                                        ui.horizontal(|ui| {
                                            ui.sized_label(
                                                label_col_w,
                                                "After Processing:",
                                                "What happens to the source transfer-* folder after unpacking",
                                                &p,
                                            );
                                            egui::ComboBox::from_id_salt("post_action_combo")
                                                .width(combo_w)
                                                .selected_text(self.active_rule.post_action.label())
                                                .show_ui(ui, |ui| {
                                                    for item in PostAction::ALL {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self.active_rule.post_action,
                                                                item,
                                                                item.label(),
                                                            )
                                                            .changed()
                                                        {
                                                            self.check_rule_modified();
                                                        }
                                                    }
                                                });
                                        });
                                    }
                                }
                            });

                        ui.add_space(12.0);

                        // 4. Primary Action Bar Card (strictly bounded inner width so buttons wrap cleanly)
                        egui::Frame::none()
                            .fill(p.surface_container)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .stroke(Stroke::new(1.0_f32, p.outline_variant))
                            .show(ui, |ui| {
                                ui.set_min_width(card_inner_w);
                                ui.set_max_width(card_inner_w);
                                ui.horizontal_wrapped(|ui| {
                                    let count = self.selected_indices.len();
                                    let process_label = if count > 1 {
                                        format!("▶ Process {count} Folders (Enter)")
                                    } else {
                                        "▶ Process Folder (Enter)".to_string()
                                    };

                                    let process_btn = egui::Button::new(
                                        RichText::new(process_label)
                                            .strong()
                                            .color(p.on_primary),
                                    )
                                    .fill(p.primary)
                                    .min_size(Vec2::new(0.0, 32.0));

                                    if ui
                                        .add_enabled(count > 0, process_btn)
                                        .on_hover_text(
                                            "Unpack/copy files, update receipt logs, and execute post-action (Enter)",
                                        )
                                        .clicked()
                                    {
                                        self.execute_process(None);
                                    }

                                    let save_label = if self.is_rule_modified {
                                        "💾 Save Rule *"
                                    } else {
                                        "💾 Save Rule"
                                    };
                                    let save_btn = egui::Button::new(
                                        RichText::new(save_label).strong().color(
                                            if self.is_rule_modified {
                                                p.on_primary_container
                                            } else {
                                                p.on_surface
                                            },
                                        ),
                                    )
                                    .fill(if self.is_rule_modified {
                                        p.primary_container
                                    } else {
                                        p.surface_container_high
                                    })
                                    .min_size(Vec2::new(0.0, 32.0));

                                    if ui
                                        .add(save_btn)
                                        .on_hover_text(
                                            "Save current destination paths & options as a Config ID, Wildcard Pattern, or Default rule (Ctrl+S)",
                                        )
                                        .clicked()
                                    {
                                        self.open_save_rule_dialog();
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new("📂 Open (Ctrl+O)")
                                                .min_size(Vec2::new(0.0, 32.0)),
                                        )
                                        .on_hover_text(
                                            "Open the selected transfer folder in your system file manager",
                                        )
                                        .clicked()
                                    {
                                        if let Some(ref f) = current_folder {
                                            let _ = open::that(&f.folder_path);
                                        }
                                    }

                                    if ui
                                        .add(
                                            egui::Button::new("📜 View Log (Ctrl+L)")
                                                .min_size(Vec2::new(0.0, 32.0)),
                                        )
                                        .on_hover_text(
                                            "Inspect the local Process.log or global audit history",
                                        )
                                        .clicked()
                                    {
                                        self.open_log_viewer();
                                    }

                                    if is_advanced {
                                        if ui
                                            .add(
                                                egui::Button::new("⚙ Manage Rules")
                                                    .min_size(Vec2::new(0.0, 32.0)),
                                            )
                                            .on_hover_text(
                                                "View, edit, or delete saved Config IDs and Wildcard Patterns (Ctrl+M)",
                                            )
                                            .clicked()
                                        {
                                            self.show_manage_rules_modal = true;
                                        }

                                        if ui
                                            .add(
                                                egui::Button::new("📋 Create Manifest")
                                                    .min_size(Vec2::new(0.0, 32.0)),
                                            )
                                            .on_hover_text(
                                                "Generate a manifest.json file from current settings to include with uploads in the User Portal",
                                            )
                                            .clicked()
                                        {
                                            self.receipt_rule = self.active_rule.clone();
                                            self.receipt_permit_id = current_folder
                                                .as_ref()
                                                .map(|f| f.permit_id.clone())
                                                .unwrap_or_else(|| "DEFAULT".to_string());
                                            self.receipt_save_path = current_folder
                                                .as_ref()
                                                .map(|f| {
                                                    f.folder_path
                                                        .join("manifest.json")
                                                        .to_string_lossy()
                                                        .to_string()
                                                })
                                                .unwrap_or_else(|| "manifest.json".to_string());
                                            self.show_create_receipt_modal = true;
                                        }
                                    }

                                    let del_btn = egui::Button::new(
                                        RichText::new("🗑 Delete").color(p.on_error_container),
                                    )
                                    .fill(p.error_container)
                                    .min_size(Vec2::new(0.0, 32.0));

                                    if ui
                                        .add_enabled(count > 0, del_btn)
                                        .on_hover_text(
                                            "Permanently delete the selected transfer folder(s) from disk (Delete)",
                                        )
                                        .clicked()
                                    {
                                        self.show_delete_confirm_modal = true;
                                    }
                                });
                            });

                        ui.add_space(12.0);

                        // 5. Split Inspector: Package Files & Archive Tree (Left) + Receipt JSON Inspector (Right)
                        // Explicitly sized vertical columns & inset monospace code panes so neither card
                        // ever inherits horizontal layout or clips past the right window edge.
                        let mono_size = (11.0 * font_scale).clamp(10.0, 22.0);
                        let inspector_scroll_h = (ui.available_height() - 80.0).clamp(108.0, 460.0);

                        let render_files_card = |ui: &mut egui::Ui, inner_w: f32, scroll_h: f32| {
                            egui::Frame::none()
                                .fill(p.surface_container)
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::same(12.0))
                                .stroke(Stroke::new(1.0_f32, p.outline_variant))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.set_min_width(inner_w);
                                        ui.set_max_width(inner_w);

                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new("📦 Package Files & ZIP Contents")
                                                    .strong(),
                                            );
                                            if let Some(ref f) = current_folder {
                                                let summary = if f.zip_contents.is_empty() {
                                                    format!("{} file(s)", f.file_list.len())
                                                } else {
                                                    format!(
                                                        "{} file(s) • {} in ZIP",
                                                        f.file_list.len(),
                                                        f.zip_contents.len()
                                                    )
                                                };
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(summary)
                                                                .small()
                                                                .color(p.on_surface_variant),
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                        ui.separator();

                                        let code_box_w = (inner_w - 16.0).max(110.0);
                                        egui::Frame::none()
                                            .fill(p.surface_container_low)
                                            .rounding(Rounding::same(8.0))
                                            .inner_margin(Margin::same(8.0))
                                            .show(ui, |ui| {
                                                ui.allocate_ui_with_layout(
                                                    Vec2::new(code_box_w, scroll_h),
                                                    egui::Layout::top_down(egui::Align::Min),
                                                    |ui| {
                                                        ui.set_clip_rect(ui.max_rect());
                                                        egui::ScrollArea::both()
                                                            .id_salt("files_inspector_scroll")
                                                            .min_scrolled_height(scroll_h)
                                                            .max_height(scroll_h)
                                                            .max_width(code_box_w)
                                                            .auto_shrink([false, false])
                                                            .show(ui, |ui| {
                                                                ui.vertical(|ui| {
                                                                    let scroll_w = (code_box_w
                                                                        - 8.0)
                                                                        .max(100.0);
                                                                    ui.set_min_width(scroll_w);
                                                                    ui.spacing_mut()
                                                                        .item_spacing
                                                                        .y = 3.0;

                                                                    if let Some(ref f) =
                                                                        current_folder
                                                                    {
                                                                        if f.file_list.is_empty() {
                                                                            ui.set_max_width(
                                                                                scroll_w,
                                                                            );
                                                                            ui.add(
                                                                                egui::Label::new(
                                                                                    RichText::new(
                                                                                        "Folder is empty.",
                                                                                    )
                                                                                    .color(
                                                                                        p.on_surface_variant,
                                                                                    ),
                                                                                )
                                                                                .wrap(),
                                                                            );
                                                                        } else {
                                                                            for item in &f.file_list
                                                                            {
                                                                                let is_zip = item
                                                                                    .to_lowercase()
                                                                                    .ends_with(
                                                                                        ".zip",
                                                                                    );
                                                                                let mut item_rt =
                                                                                    RichText::new(
                                                                                        item,
                                                                                    )
                                                                                    .font(
                                                                                        egui::FontId::monospace(
                                                                                            mono_size,
                                                                                        ),
                                                                                    )
                                                                                    .color(
                                                                                        if is_zip {
                                                                                            p.primary
                                                                                        } else {
                                                                                            p.on_surface
                                                                                        },
                                                                                    );
                                                                                if is_zip {
                                                                                    item_rt =
                                                                                        item_rt
                                                                                            .strong();
                                                                                }
                                                                                ui.add(
                                                                                    egui::Label::new(
                                                                                        item_rt,
                                                                                    )
                                                                                    .extend(),
                                                                                );
                                                                                if is_zip {
                                                                                    let zlen = f
                                                                                        .zip_contents
                                                                                        .len();
                                                                                    for (zi, zc) in
                                                                                        f.zip_contents
                                                                                            .iter()
                                                                                            .enumerate()
                                                                                    {
                                                                                        let branch =
                                                                                            if zi
                                                                                                + 1
                                                                                                == zlen
                                                                                            {
                                                                                                "└─"
                                                                                            } else {
                                                                                                "├─"
                                                                                            };
                                                                                        ui.add(
                                                                                            egui::Label::new(
                                                                                                RichText::new(
                                                                                                    format!(
                                                                                                        "  {branch} {zc}"
                                                                                                    ),
                                                                                                )
                                                                                                .font(
                                                                                                    egui::FontId::monospace(
                                                                                                        mono_size,
                                                                                                    ),
                                                                                                )
                                                                                                .color(
                                                                                                    p.on_surface_variant,
                                                                                                ),
                                                                                            )
                                                                                            .extend(),
                                                                                        );
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    } else {
                                                                        ui.set_max_width(scroll_w);
                                                                        ui.add(
                                                                            egui::Label::new(
                                                                                RichText::new(
                                                                                    "Select a transfer folder to inspect its contents.",
                                                                                )
                                                                                .color(
                                                                                    p.on_surface_variant,
                                                                                ),
                                                                            )
                                                                            .wrap(),
                                                                        );
                                                                    }
                                                                });
                                                            });
                                                    },
                                                );
                                            });
                                    });
                                });
                        };

                        let render_receipt_card =
                            |ui: &mut egui::Ui, inner_w: f32, scroll_h: f32| {
                                egui::Frame::none()
                                    .fill(p.surface_container)
                                    .rounding(Rounding::same(12.0))
                                    .inner_margin(Margin::same(12.0))
                                    .stroke(Stroke::new(1.0_f32, p.outline_variant))
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_min_width(inner_w);
                                            ui.set_max_width(inner_w);

                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("📋 Receipt & Manifest Inspector")
                                                        .strong(),
                                                );
                                                if let Some(ref f) = current_folder {
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if f.receipt_warning.is_some() {
                                                                ui.label(
                                                                    RichText::new("⚠ Warning")
                                                                        .small()
                                                                        .strong()
                                                                        .color(p.error),
                                                                );
                                                            } else if !f
                                                                .receipt_raw
                                                                .trim()
                                                                .is_empty()
                                                            {
                                                                ui.label(
                                                                    RichText::new("✔ Loaded")
                                                                        .small()
                                                                        .color(p.success),
                                                                );
                                                            }
                                                        },
                                                    );
                                                }
                                            });
                                            ui.separator();

                                            let code_box_w = (inner_w - 16.0).max(110.0);
                                            egui::Frame::none()
                                                .fill(p.surface_container_low)
                                                .rounding(Rounding::same(8.0))
                                                .inner_margin(Margin::same(8.0))
                                                .show(ui, |ui| {
                                                    ui.allocate_ui_with_layout(
                                                        Vec2::new(code_box_w, scroll_h),
                                                        egui::Layout::top_down(egui::Align::Min),
                                                        |ui| {
                                                            ui.set_clip_rect(ui.max_rect());
                                                            egui::ScrollArea::both()
                                                                .id_salt("receipt_inspector_scroll")
                                                                .min_scrolled_height(scroll_h)
                                                                .max_height(scroll_h)
                                                                .max_width(code_box_w)
                                                                .auto_shrink([false, false])
                                                                .show(ui, |ui| {
                                                                    ui.vertical(|ui| {
                                                                        let scroll_w = (code_box_w
                                                                            - 8.0)
                                                                            .max(100.0);
                                                                        ui.set_min_width(scroll_w);
                                                                        ui.spacing_mut()
                                                                            .item_spacing
                                                                            .y = 2.0;

                                                                        if let Some(ref f) =
                                                                            current_folder
                                                                        {
                                                                            if !f
                                                                                .receipt_raw
                                                                                .trim()
                                                                                .is_empty()
                                                                            {
                                                                                let formatted = if f
                                                                                    .receipt_raw
                                                                                    .trim()
                                                                                    .contains('\n')
                                                                                {
                                                                                    f.receipt_raw
                                                                                        .clone()
                                                                                } else {
                                                                                    serde_json::from_str::<serde_json::Value>(&f.receipt_raw)
                                                                                        .ok()
                                                                                        .and_then(|v| serde_json::to_string_pretty(&v).ok())
                                                                                        .unwrap_or_else(|| f.receipt_raw.clone())
                                                                                };

                                                                                let mono_font =
                                                                                    egui::FontId::monospace(
                                                                                        mono_size,
                                                                                    );
                                                                                for line in
                                                                                    formatted
                                                                                        .lines()
                                                                                {
                                                                                    let mut job = egui::text::LayoutJob::default();
                                                                                    job.wrap.max_width = f32::INFINITY;
                                                                                    append_json_highlighted_line(
                                                                                        &mut job,
                                                                                        line,
                                                                                        &mono_font,
                                                                                        &p,
                                                                                    );
                                                                                    ui.add(
                                                                                        egui::Label::new(
                                                                                            job,
                                                                                        )
                                                                                        .extend(),
                                                                                    );
                                                                                }
                                                                            } else {
                                                                                ui.set_max_width(
                                                                                    scroll_w,
                                                                                );
                                                                                ui.add(
                                                                                    egui::Label::new(
                                                                                        RichText::new(
                                                                                            "No receipt.json or manifest.json found in this transfer folder (Config ID: DEFAULT/No_ID).\n\nTip: Save a Wildcard Pattern rule, or click 'Create Manifest' and include manifest.json when uploading in the User Portal!",
                                                                                        )
                                                                                        .small()
                                                                                        .color(p.on_surface_variant),
                                                                                    )
                                                                                    .wrap(),
                                                                                );
                                                                            }
                                                                        } else {
                                                                            ui.set_max_width(
                                                                                scroll_w,
                                                                            );
                                                                            ui.add(
                                                                                egui::Label::new(
                                                                                    RichText::new(
                                                                                        "No folder selected.",
                                                                                    )
                                                                                    .color(
                                                                                        p.on_surface_variant,
                                                                                    ),
                                                                                )
                                                                                .wrap(),
                                                                            );
                                                                        }
                                                                    });
                                                                });
                                                        },
                                                    );
                                                });
                                        });
                                    });
                            };

                        let col_gap = 12.0;
                        if panel_width >= 580.0 {
                            let col_outer_w = ((panel_width - col_gap) * 0.5).max(186.0);
                            let col_inner_w = (col_outer_w - 26.0).max(160.0);
                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = col_gap;
                                ui.allocate_ui_with_layout(
                                    Vec2::new(col_outer_w, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        render_files_card(ui, col_inner_w, inspector_scroll_h);
                                    },
                                );
                                ui.allocate_ui_with_layout(
                                    Vec2::new(col_outer_w, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        render_receipt_card(ui, col_inner_w, inspector_scroll_h);
                                    },
                                );
                            });
                        } else {
                            let full_insp_inner_w = (panel_width - 26.0).max(200.0);
                            render_files_card(ui, full_insp_inner_w, 200.0);
                            ui.add_space(10.0);
                            render_receipt_card(ui, full_insp_inner_w, 200.0);
                        }
                    });
            });

        self.render_modals(ctx, &p);
    }
}

trait UiLabelHelper {
    fn sized_label(&mut self, width: f32, label: &str, tooltip: &str, p: &M3Palette);
}

impl UiLabelHelper for egui::Ui {
    fn sized_label(&mut self, width: f32, label: &str, tooltip: &str, p: &M3Palette) {
        self.allocate_ui_with_layout(
            Vec2::new(width, 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).color(p.on_surface))
                    .on_hover_text(tooltip);
            },
        );
    }
}

/// Appends a syntax-highlighted JSON line into an `egui::text::LayoutJob` using true monospace font.
fn append_json_highlighted_line(
    job: &mut egui::text::LayoutJob,
    line: &str,
    font: &egui::FontId,
    p: &M3Palette,
) {
    let fmt = |color| egui::TextFormat {
        font_id: font.clone(),
        color,
        ..Default::default()
    };

    // Split leading whitespace so indentation stays exact
    let indent_len = line.len() - line.trim_start().len();
    if indent_len > 0 {
        job.append(&line[..indent_len], 0.0, fmt(p.on_surface_variant));
    }
    let rest = &line[indent_len..];

    // Section banner headers (when both manifest.json and receipt.json are displayed)
    if rest.starts_with("//") {
        job.append(rest, 0.0, fmt(p.warning));
        return;
    }

    // Check for `"key": value` pattern
    if let Some(after_open) = rest.strip_prefix('"') {
        if let Some(close_idx) = after_open.find("\":") {
            let key_span = &rest[..close_idx + 2]; // includes opening and closing `"`
            let remainder = &rest[close_idx + 2..]; // starts at `:`
            job.append(key_span, 0.0, fmt(p.primary));
            if let Some(after_colon) = remainder.strip_prefix(':') {
                job.append(":", 0.0, fmt(p.on_surface_variant));
                let val_trimmed = after_colon.trim_end_matches(',');
                let has_comma = after_colon.ends_with(',');
                let val_color = if val_trimmed.trim().starts_with('"') {
                    p.on_surface
                } else {
                    p.success
                };
                job.append(val_trimmed, 0.0, fmt(val_color));
                if has_comma {
                    job.append(",", 0.0, fmt(p.on_surface_variant));
                }
                return;
            }
        }
    }

    // Brackets / braces vs standalone array values
    let trimmed = rest.trim();
    let color = if matches!(
        trimmed,
        "{" | "}" | "}," | "[" | "]" | "]," | "{}" | "[]"
    ) {
        p.on_surface_variant
    } else {
        p.on_surface
    };
    job.append(rest, 0.0, fmt(color));
}

impl InboxMoverGuiApp {
    fn render_modals(&mut self, ctx: &egui::Context, p: &M3Palette) {
        // 1. First-Time Welcome & Experience Selector Modal
        if self.show_welcome_modal {
            let welcome_icon_id = self
                .app_icon_tex
                .get_or_insert_with(|| {
                    ctx.load_texture(
                        "inbox_mover_app_icon",
                        crate::icon::egui_color_image(),
                        egui::TextureOptions::LINEAR,
                    )
                })
                .id();
            egui::Window::new("👋 Welcome to Inbox Mover")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(560.0, 385.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(36.0), Sense::hover());
                        ui.painter().image(
                            welcome_icon_id,
                            icon_rect,
                            egui::Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            Color32::WHITE,
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Modern Transfer Folder Unpacker & Rule Engine")
                                .strong()
                                .size(16.0)
                                .color(p.primary),
                        );
                    });
                    ui.add_space(6.0);
                    ui.label(
                        "Inbox Mover automatically extracts ZIP archives, routes files and receipts to project directories, translates Windows/Linux network paths, and maintains a 3-tier audit log.",
                    );
                    ui.add_space(10.0);

                    ui.columns(2, |cols| {
                        egui::Frame::none()
                            .fill(p.surface_container_high)
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::same(12.0))
                            .show(&mut cols[0], |ui| {
                                ui.label(
                                    RichText::new("🌱 New to Inbox Mover?")
                                        .strong()
                                        .color(p.on_surface),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "• Streamlined Simple Mode\n• Smart Context Guide explains what each folder needs\n• Interactive Help Center (F1)",
                                    )
                                    .small()
                                    .color(p.on_surface_variant),
                                );
                                ui.add_space(8.0);
                                let simple_start_btn = egui::Button::new(
                                    RichText::new("Start in Simple Mode (Guided)")
                                        .strong()
                                        .color(p.on_primary_container),
                                )
                                .fill(p.primary_container);
                                if ui.add(simple_start_btn).clicked() {
                                    self.core.app_settings.simple_mode = true;
                                    self.core.app_settings.show_guidance = true;
                                    self.core.save_settings();
                                    self.show_welcome_modal = false;
                                }
                            });

                        egui::Frame::none()
                            .fill(p.surface_container_high)
                            .rounding(Rounding::same(10.0))
                            .inner_margin(Margin::same(12.0))
                            .show(&mut cols[1], |ui| {
                                ui.label(
                                    RichText::new("⚡ Experienced / Daily User?")
                                        .strong()
                                        .color(p.on_surface),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "• Full Advanced Mode unlocked\n• Hides Smart Guidance banners so they never get in your way\n• Full keyboard navigation",
                                    )
                                    .small()
                                    .color(p.on_surface_variant),
                                );
                                ui.add_space(8.0);
                                let power_start_btn = egui::Button::new(
                                    RichText::new("Start in Power User Mode (No Banners)")
                                        .strong()
                                        .color(p.on_secondary_container),
                                )
                                .fill(p.secondary_container);
                                if ui.add(power_start_btn).clicked() {
                                    self.core.app_settings.simple_mode = false;
                                    self.core.app_settings.show_guidance = false;
                                    self.core.save_settings();
                                    self.show_welcome_modal = false;
                                }
                            });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        let mut dont_show = !self.core.app_settings.show_welcome_splash;
                        if ui
                            .checkbox(&mut dont_show, "Don't show this welcome guide on startup")
                            .changed()
                        {
                            self.core.app_settings.show_welcome_splash = !dont_show;
                            self.core.save_settings();
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.show_welcome_modal = false;
                            }
                        });
                    });
                });
        }

        // 2. Interactive Help Center & Shortcut Cheat Sheet Modal
        if self.show_help_modal {
            egui::Window::new("❓ Inbox Mover Help Center & Reference")
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_size(Vec2::new(780.0, 540.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.help_tab,
                            HelpTab::Guides,
                            "📖 Step-by-Step Guides",
                        );
                        ui.selectable_value(
                            &mut self.help_tab,
                            HelpTab::QuickTips,
                            "💡 Quick Tips & FAQ",
                        );
                        ui.selectable_value(
                            &mut self.help_tab,
                            HelpTab::Shortcuts,
                            "⌨ Keyboard Shortcuts (GUI & TUI)",
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖ Close (Esc)").clicked() {
                                self.show_help_modal = false;
                            }
                        });
                    });
                    ui.separator();

                    match self.help_tab {
                        HelpTab::Guides => {
                            ui.columns(2, |cols| {
                                cols[0].label(RichText::new("Topics").strong());
                                cols[0].add_space(4.0);
                                for (i, sec) in HELP_SECTIONS.iter().enumerate() {
                                    let is_sel = self.selected_help_section == i;
                                    let resp = cols[0].selectable_label(
                                        is_sel,
                                        format!("[{}] {}", sec.badge, sec.title),
                                    );
                                    if resp.clicked() {
                                        self.selected_help_section = i;
                                    }
                                    cols[0].add_space(2.0);
                                }

                                egui::ScrollArea::vertical()
                                    .id_salt("help_topic_detail_scroll")
                                    .show(&mut cols[1], |ui| {
                                        if let Some(sec) =
                                            HELP_SECTIONS.get(self.selected_help_section)
                                        {
                                            ui.label(
                                                RichText::new(sec.title)
                                                    .strong()
                                                    .size(16.0)
                                                    .color(p.primary),
                                            );
                                            ui.label(
                                                RichText::new(sec.subtitle)
                                                    .italics()
                                                    .color(p.on_surface_variant),
                                            );
                                            ui.separator();
                                            for line in sec.content.lines() {
                                                if let Some(h) = line.strip_prefix("### ") {
                                                    ui.add_space(6.0);
                                                    ui.label(
                                                        RichText::new(h)
                                                            .strong()
                                                            .size(15.0)
                                                            .color(p.primary),
                                                    );
                                                } else if let Some(h) = line.strip_prefix("#### ") {
                                                    ui.add_space(4.0);
                                                    ui.label(RichText::new(h).strong());
                                                } else {
                                                    ui.label(line);
                                                }
                                            }
                                        }
                                    });
                            });
                        }
                        HelpTab::QuickTips => {
                            egui::ScrollArea::vertical()
                                .id_salt("quick_tips_scroll")
                                .show(ui, |ui| {
                                    for (title, body) in QUICK_TIPS {
                                        egui::Frame::none()
                                            .fill(p.surface_container_high)
                                            .rounding(Rounding::same(8.0))
                                            .inner_margin(Margin::same(10.0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(*title)
                                                        .strong()
                                                        .color(p.primary),
                                                );
                                                ui.add_space(3.0);
                                                ui.label(*body);
                                            });
                                        ui.add_space(6.0);
                                    }
                                });
                        }
                        HelpTab::Shortcuts => {
                            egui::ScrollArea::vertical()
                                .id_salt("shortcuts_scroll")
                                .show(ui, |ui| {
                                    egui::Grid::new("shortcuts_grid")
                                        .striped(true)
                                        .spacing([16.0, 8.0])
                                        .show(ui, |ui| {
                                            ui.label(RichText::new("Action").strong());
                                            ui.label(RichText::new("GUI Shortcut").strong());
                                            ui.label(RichText::new("TUI Shortcut").strong());
                                            ui.label(RichText::new("Description").strong());
                                            ui.end_row();

                                            for s in SHORTCUTS {
                                                ui.label(RichText::new(s.action).strong());
                                                ui.label(
                                                    RichText::new(s.key_gui)
                                                        .monospace()
                                                        .color(p.primary),
                                                );
                                                ui.label(
                                                    RichText::new(s.key_tui)
                                                        .monospace()
                                                        .color(p.success),
                                                );
                                                ui.label(s.description);
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
                    }
                });
        }

        // 3. Save Rule Modal
        if self.show_save_modal {
            egui::Window::new("💾 Save Processing Rule")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(480.0, 260.0))
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Active Workspace: {}",
                        if self.core.app_settings.use_global {
                            "TEAM"
                        } else {
                            "PERSONAL"
                        }
                    ));
                    ui.separator();

                    ui.radio_value(
                        &mut self.save_target_type,
                        SaveRuleTarget::ConfigId,
                        "Save to Config ID (<permitId>.json)",
                    );
                    if self.save_target_type == SaveRuleTarget::ConfigId {
                        ui.horizontal(|ui| {
                            ui.label("Config ID:");
                            ui.text_edit_singleline(&mut self.save_config_id_input);
                        });
                    }

                    ui.radio_value(
                        &mut self.save_target_type,
                        SaveRuleTarget::Pattern,
                        "Save as Wildcard Pattern Rule (patterns.json)",
                    );
                    if self.save_target_type == SaveRuleTarget::Pattern {
                        ui.horizontal(|ui| {
                            ui.label("Glob Pattern:");
                            ui.text_edit_singleline(&mut self.save_pattern_input);
                        });
                        ui.label(
                            RichText::new(
                                "Matched against folder name, loose files, and filenames inside ZIPs (e.g. *survey*)",
                            )
                            .small()
                            .color(p.on_surface_variant),
                        );
                    }

                    ui.radio_value(
                        &mut self.save_target_type,
                        SaveRuleTarget::DefaultRule,
                        "Save as Workspace DEFAULT Rule (DEFAULT.json)",
                    );

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(RichText::new("💾 Save Now").strong().color(p.primary))
                            .clicked()
                        {
                            let res = match self.save_target_type {
                                SaveRuleTarget::ConfigId => {
                                    let id = self.save_config_id_input.trim().to_string();
                                    self.core.save_config(&id, self.active_rule.clone())
                                }
                                SaveRuleTarget::Pattern => {
                                    let pat = self.save_pattern_input.trim().to_string();
                                    self.core.save_pattern(&pat, self.active_rule.clone())
                                }
                                SaveRuleTarget::DefaultRule => {
                                    self.core.save_config("DEFAULT", self.active_rule.clone())
                                }
                            };
                            match res {
                                Ok(()) => {
                                    self.show_save_modal = false;
                                    self.sync_active_rule_from_selection();
                                    self.status_message =
                                        "Rule saved successfully to workspace.".to_string();
                                    self.status_is_error = false;
                                }
                                Err(e) => {
                                    self.status_message = format!("Failed to save rule: {e}");
                                    self.status_is_error = true;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save_modal = false;
                        }
                    });
                });
        }

        // 4. Manage Rules Modal
        if self.show_manage_rules_modal {
            egui::Window::new("⚙ Manage Saved Rules & Wildcard Patterns")
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_size(Vec2::new(720.0, 460.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.manage_rules_tab,
                            ManageRulesTab::ConfigIds,
                            format!("🆔 Config IDs ({})", self.core.config_cache.len()),
                        );
                        ui.selectable_value(
                            &mut self.manage_rules_tab,
                            ManageRulesTab::Patterns,
                            format!("✳ Wildcard Patterns ({})", self.core.patterns_cache.len()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖ Close").clicked() {
                                self.show_manage_rules_modal = false;
                            }
                        });
                    });
                    ui.separator();

                    match self.manage_rules_tab {
                        ManageRulesTab::ConfigIds => {
                            let mut ids: Vec<String> =
                                self.core.config_cache.keys().cloned().collect();
                            ids.sort();

                            ui.columns(2, |cols| {
                                egui::ScrollArea::vertical()
                                    .id_salt("manage_ids_list")
                                    .max_height(340.0)
                                    .show(&mut cols[0], |ui| {
                                        for id in &ids {
                                            let is_sel =
                                                self.manage_selected_id.as_ref() == Some(id);
                                            if ui.selectable_label(is_sel, id).clicked() {
                                                self.manage_selected_id = Some(id.clone());
                                                if let Some(r) = self.core.config_cache.get(id) {
                                                    self.manage_edit_rule = r.clone();
                                                }
                                            }
                                        }
                                    });

                                if let Some(sel_id) = self.manage_selected_id.clone() {
                                    cols[1].label(
                                        RichText::new(format!("Editing ID: {sel_id}")).strong(),
                                    );
                                    cols[1].label("Target Folder:");
                                    cols[1].text_edit_singleline(
                                        &mut self.manage_edit_rule.target_folder,
                                    );
                                    cols[1].label("Processed Folder:");
                                    cols[1].text_edit_singleline(
                                        &mut self.manage_edit_rule.target_zip_folder,
                                    );
                                    cols[1].label("Receipt Folder:");
                                    cols[1].text_edit_singleline(
                                        &mut self.manage_edit_rule.receipt_folder,
                                    );
                                    cols[1].checkbox(
                                        &mut self.manage_edit_rule.auto_extract,
                                        "Auto-extract ZIPs",
                                    );
                                    cols[1].add_space(8.0);
                                    cols[1].horizontal(|ui| {
                                        if ui.button("💾 Save Changes").clicked() {
                                            let _ = self.core.save_config(
                                                &sel_id,
                                                self.manage_edit_rule.clone(),
                                            );
                                            self.sync_active_rule_from_selection();
                                        }
                                        if sel_id != "DEFAULT" && ui.button("🗑 Delete ID").clicked()
                                        {
                                            let _ = self.core.delete_config(&sel_id);
                                            self.manage_selected_id = None;
                                            self.sync_active_rule_from_selection();
                                        }
                                    });
                                } else {
                                    cols[1].label("Select a Config ID on the left to edit.");
                                }
                            });
                        }
                        ManageRulesTab::Patterns => {
                            let mut pats: Vec<String> =
                                self.core.patterns_cache.keys().cloned().collect();
                            pats.sort();

                            ui.columns(2, |cols| {
                                egui::ScrollArea::vertical()
                                    .id_salt("manage_patterns_list")
                                    .max_height(300.0)
                                    .show(&mut cols[0], |ui| {
                                        for pat in &pats {
                                            let is_sel =
                                                self.manage_selected_pattern.as_ref() == Some(pat);
                                            if ui.selectable_label(is_sel, pat).clicked() {
                                                self.manage_selected_pattern = Some(pat.clone());
                                                if let Some(cfg) =
                                                    self.core.patterns_cache.get(pat)
                                                {
                                                    self.manage_edit_rule = cfg.clone();
                                                }
                                                self.manage_new_id_or_pattern = pat.clone();
                                            }
                                        }
                                    });

                                if let Some(sel_pat) = self.manage_selected_pattern.clone() {
                                    cols[1].label("Glob Pattern:");
                                    cols[1]
                                        .text_edit_singleline(&mut self.manage_new_id_or_pattern);
                                    cols[1].label("Target Folder:");
                                    cols[1].text_edit_singleline(
                                        &mut self.manage_edit_rule.target_folder,
                                    );
                                    cols[1].label("Processed Folder:");
                                    cols[1].text_edit_singleline(
                                        &mut self.manage_edit_rule.target_zip_folder,
                                    );
                                    cols[1].checkbox(
                                        &mut self.manage_edit_rule.auto_extract,
                                        "Auto-extract ZIPs",
                                    );
                                    cols[1].horizontal(|ui| {
                                        if ui.button("💾 Save Pattern").clicked() {
                                            let _ = self.core.save_pattern(
                                                &self.manage_new_id_or_pattern,
                                                self.manage_edit_rule.clone(),
                                            );
                                            self.sync_active_rule_from_selection();
                                        }
                                        if ui.button("🗑 Delete Pattern").clicked() {
                                            let _ = self.core.delete_pattern(&sel_pat);
                                            self.manage_selected_pattern = None;
                                            self.sync_active_rule_from_selection();
                                        }
                                    });
                                } else {
                                    cols[1].label("Select a pattern on the left to edit.");
                                }
                            });
                        }
                    }
                });
        }

        // 5. Workspace Sync / Merge Modal
        if self.show_sync_modal {
            egui::Window::new("⟳ Sync / Merge Workspaces")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(420.0, 190.0))
                .show(ctx, |ui| {
                    ui.label("Team Workspace Folder:");
                    ui.text_edit_singleline(&mut self.core.app_settings.global_dir);
                    ui.separator();
                    ui.label("Merge all local Personal Config IDs and Patterns into the Team Workspace directory:");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("⟳ Merge Local -> Team").clicked() {
                            self.core.app_settings.use_global = true;
                            self.core.save_settings();
                            match self.core.merge_local_into_team() {
                                Ok(n) => {
                                    self.status_message =
                                        format!("Merged {n} item(s) into Team workspace.");
                                    self.status_is_error = false;
                                    self.show_sync_modal = false;
                                }
                                Err(e) => {
                                    self.status_message = format!("Sync failed: {e}");
                                    self.status_is_error = true;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_sync_modal = false;
                        }
                    });
                });
        }

        // 6. Create Manifest Modal
        if self.show_create_receipt_modal {
            egui::Window::new("📋 Create Upload Manifest (manifest.json)")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(540.0, 355.0))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(p.surface_container_high)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(
                                    "💡 User Portal Tip: Add this manifest.json file along with your files when uploading in the User Portal. When the transfer arrives, Inbox Mover automatically reads manifest.json (alongside the portal's receipt.json) and uses these destination & processing settings.",
                                )
                                .small()
                                .color(p.on_surface_variant),
                            );
                        });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Permit / Config ID (optional):");
                        ui.text_edit_singleline(&mut self.receipt_permit_id);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Target Folder:");
                        ui.text_edit_singleline(&mut self.receipt_rule.target_folder);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Process Folder:");
                        ui.text_edit_singleline(&mut self.receipt_rule.target_zip_folder);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Receipt Folder:");
                        ui.text_edit_singleline(&mut self.receipt_rule.receipt_folder);
                    });
                    ui.checkbox(&mut self.receipt_rule.auto_extract, "Auto-extract ZIPs");
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Save to File:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.receipt_save_path)
                                .desired_width(340.0),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(chosen) = rfd::FileDialog::new()
                                .set_file_name("manifest.json")
                                .add_filter("JSON Manifest", &["json"])
                                .save_file()
                            {
                                self.receipt_save_path = chosen.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("💾 Generate manifest.json").clicked() {
                            let path = PathBuf::from(self.receipt_save_path.trim());
                            let pid = if self.receipt_permit_id.trim().is_empty() {
                                None
                            } else {
                                Some(self.receipt_permit_id.trim())
                            };
                            match self.core.create_manifest_file(&path, &self.receipt_rule, pid) {
                                Ok(()) => {
                                    self.show_create_receipt_modal = false;
                                    self.refresh_folders();
                                    self.status_message =
                                        format!("Created manifest at {:?}", path);
                                    self.status_is_error = false;
                                }
                                Err(e) => {
                                    self.status_message =
                                        format!("Failed creating manifest: {e}");
                                    self.status_is_error = true;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_create_receipt_modal = false;
                        }
                    });
                });
        }

        // 7. Audit Log Viewer Modal
        if self.show_log_modal {
            egui::Window::new("📜 Processing Audit Logs")
                .collapsible(false)
                .resizable(true)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .default_size(Vec2::new(760.0, 480.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.log_tab,
                            LogTab::LocalFolder,
                            "📂 Selected Folder (Process.log)",
                        );
                        ui.selectable_value(
                            &mut self.log_tab,
                            LogTab::GlobalHistory,
                            format!("🌍 Global History ({})", self.global_logs.len()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖ Close").clicked() {
                                self.show_log_modal = false;
                            }
                        });
                    });
                    ui.separator();

                    match self.log_tab {
                        LogTab::LocalFolder => {
                            let log_mono_size = (self.core.app_settings.font_size as f32).clamp(10.0, 22.0);
                            egui::ScrollArea::vertical()
                                .id_salt("local_log_scroll")
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&self.local_log_content)
                                            .font(egui::FontId::monospace(log_mono_size)),
                                    );
                                });
                        }
                        LogTab::GlobalHistory => {
                            ui.horizontal(|ui| {
                                ui.label("Filter:");
                                ui.text_edit_singleline(&mut self.log_search_query);
                            });
                            ui.add_space(4.0);
                            let q = self.log_search_query.to_lowercase();
                            egui::ScrollArea::vertical()
                                .id_salt("global_log_scroll")
                                .show(ui, |ui| {
                                    for entry in &self.global_logs {
                                        if !q.is_empty()
                                            && !entry.folder_name.to_lowercase().contains(&q)
                                            && !entry.config_id.to_lowercase().contains(&q)
                                            && !entry.user.to_lowercase().contains(&q)
                                        {
                                            continue;
                                        }
                                        egui::Frame::none()
                                            .fill(p.surface_container_high)
                                            .rounding(Rounding::same(6.0))
                                            .inner_margin(Margin::same(8.0))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let col = if entry.status == "SUCCESS" {
                                                        p.success
                                                    } else {
                                                        p.error
                                                    };
                                                    ui.label(
                                                        RichText::new(&entry.status)
                                                            .strong()
                                                            .color(col),
                                                    );
                                                    ui.label(
                                                        RichText::new(&entry.folder_name).strong(),
                                                    );
                                                    ui.label(format!(
                                                        "ID: {} | User: {} | Files: {} | {}",
                                                        entry.config_id,
                                                        entry.user,
                                                        entry.files_processed,
                                                        entry.timestamp
                                                    ));
                                                });
                                                if !entry.message.is_empty() {
                                                    ui.label(
                                                        RichText::new(&entry.message)
                                                            .small()
                                                            .color(p.on_surface_variant),
                                                    );
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }
                                });
                        }
                    }
                });
        }

        // 8. Delete Confirmation Modal
        if self.show_delete_confirm_modal {
            egui::Window::new("⚠ Confirm Folder Deletion")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(420.0, 160.0))
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Are you sure you want to permanently delete {} selected transfer folder(s) from disk?",
                        self.selected_indices.len()
                    ));
                    ui.label(
                        RichText::new("This action cannot be undone.")
                            .small()
                            .color(p.error),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let confirm_btn = egui::Button::new(
                            RichText::new("🗑 Yes, Delete Permanently")
                                .color(p.on_error_container),
                        )
                        .fill(p.error_container);
                        if ui.add(confirm_btn).clicked() {
                            let paths: Vec<PathBuf> = self
                                .selected_indices
                                .iter()
                                .filter_map(|&idx| {
                                    self.folders.get(idx).map(|f| f.folder_path.clone())
                                })
                                .collect();
                            let mut deleted = 0;
                            for path in paths {
                                if std::fs::remove_dir_all(&path).is_ok() {
                                    deleted += 1;
                                }
                            }
                            self.show_delete_confirm_modal = false;
                            self.refresh_folders();
                            self.status_message = format!("Deleted {deleted} folder(s).");
                            self.status_is_error = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_delete_confirm_modal = false;
                        }
                    });
                });
        }

        // 9. Encrypted ZIP Password Modal
        if self.show_password_modal {
            egui::Window::new("🔒 Encrypted ZIP Password Required")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(420.0, 160.0))
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Enter password to extract '{}':",
                        self.password_zip_name
                    ));
                    let resp = ui
                        .add(egui::TextEdit::singleline(&mut self.password_input).password(true));
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        let pw = self.password_input.clone();
                        self.show_password_modal = false;
                        self.execute_process(Some(pw));
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("🔓 Unlock & Process").clicked() {
                            let pw = self.password_input.clone();
                            self.show_password_modal = false;
                            self.execute_process(Some(pw));
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_password_modal = false;
                        }
                    });
                });
        }
    }
}

impl eframe::App for InboxMoverGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_ui(ctx);
    }
}

/// Launches the cross-platform Material 3 GUI (identical on Windows and Linux).
/// Automatically falls back to the built-in CPU/GDI software renderer on GPU-less Windows VMs / RDP
/// when neither WGPU nor OpenGL 2.0+ is available — requiring zero external installations.
pub fn run_gui(core: InboxMoverCore) -> Result<(), String> {
    let env_renderer = std::env::var("INBOX_MOVER_RENDERER")
        .unwrap_or_default()
        .to_lowercase();

    if env_renderer == "software" || env_renderer == "gdi" || env_renderer == "cpu" {
        return crate::sw_renderer::run_software_gui(core);
    }

    // Pre-flight check on Windows: if no WGPU adapter is present (typical on GPU-less VMs / myDRE / RDP
    // where Windows only provides OpenGL 1.1 GDI Generic), go straight to the built-in CPU/GDI software
    // renderer before eframe creates a winit EventLoop.
    #[cfg(target_os = "windows")]
    if env_renderer.is_empty() {
        let instance = eframe::wgpu::Instance::default();
        let adapters = instance.enumerate_adapters(eframe::wgpu::Backends::all());
        if adapters.is_empty() {
            return crate::sw_renderer::run_software_gui(core);
        }
    }

    let make_options = |renderer: eframe::Renderer| eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "Inbox Mover v{} — Transfer Folder Processor",
                env!("CARGO_PKG_VERSION")
            ))
            .with_icon(crate::icon::eframe_icon_data())
            .with_app_id("inbox-mover")
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([920.0, 620.0]),
        renderer,
        ..Default::default()
    };

    let renderers = if env_renderer == "glow" || env_renderer == "opengl" {
        vec![eframe::Renderer::Glow, eframe::Renderer::Wgpu]
    } else if env_renderer == "wgpu" || env_renderer == "dx12" || env_renderer == "dx11" {
        vec![eframe::Renderer::Wgpu, eframe::Renderer::Glow]
    } else if cfg!(target_os = "windows") {
        vec![eframe::Renderer::Wgpu, eframe::Renderer::Glow]
    } else {
        vec![eframe::Renderer::Glow, eframe::Renderer::Wgpu]
    };

    let mut errors = Vec::new();
    for renderer in renderers {
        let core_clone = core.clone();
        let res = eframe::run_native(
            "Inbox Mover",
            make_options(renderer),
            Box::new(move |cc| Ok(Box::new(InboxMoverGuiApp::new(cc, core_clone)))),
        );
        match res {
            Ok(()) => return Ok(()),
            Err(e) => {
                errors.push(format!("{renderer:?}: {e}"));
            }
        }
    }

    // If hardware renderers failed after creating an EventLoop, re-invoke self with INBOX_MOVER_RENDERER=software
    if cfg!(target_os = "windows") && env_renderer.is_empty() {
        if let Ok(exe) = std::env::current_exe() {
            let args: Vec<String> = std::env::args().skip(1).collect();
            if let Ok(mut child) = std::process::Command::new(exe)
                .args(&args)
                .env("INBOX_MOVER_RENDERER", "software")
                .spawn()
            {
                let _ = child.wait();
                return Ok(());
            }
        }
    }

    let err_msg = format!(
        "Failed to initialize graphics window ({}).",
        errors.join(" | ")
    );
    let log_path = core.local_config_dir.join("gui_startup_error.log");
    let _ = std::fs::write(&log_path, &err_msg);

    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Inbox Mover — GUI Startup Error")
        .set_description(&format!(
            "{err_msg}\n\nDetails have been written to:\n{}",
            log_path.display()
        ))
        .show();

    Err(err_msg)
}
