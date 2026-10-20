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
}

impl InboxMoverGuiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, core: InboxMoverCore) -> Self {
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
        };

        app.apply_theme(&cc.egui_ctx);
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

    fn apply_theme(&self, ctx: &egui::Context) {
        let p = self.palette();
        let mut visuals = if self.core.app_settings.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        visuals.panel_fill = p.bg_surface;
        visuals.window_fill = p.surface_container;
        visuals.faint_bg_color = p.surface_container_low;
        visuals.extreme_bg_color = p.surface_container_low;
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

        visuals.selection.bg_fill = p.primary_container;
        visuals.selection.stroke = Stroke::new(1.0_f32, p.on_primary_container);

        ctx.set_visuals(visuals);

        let scale = (self.core.app_settings.font_size as f32 / 11.0).clamp(0.75, 2.0);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        for (_text_style, font_id) in style.text_styles.iter_mut() {
            font_id.size = (font_id.size * scale).clamp(10.0, 28.0);
        }
        ctx.set_style(style);
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
}

impl eframe::App for InboxMoverGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard_shortcuts(ctx);
        let p = self.palette();
        let is_advanced = !self.core.app_settings.simple_mode;

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
                    egui::Frame::none()
                        .fill(p.primary_container)
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("📦 INBOX MOVER")
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
        egui::TopBottomPanel::bottom("m3_bottom_bar")
            .frame(
                egui::Frame::none()
                    .fill(p.surface_container)
                    .inner_margin(Margin::symmetric(16.0, 8.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let status_color = if self.status_is_error {
                        p.error
                    } else {
                        p.success
                    };
                    ui.label(RichText::new("●").color(status_color));
                    ui.label(RichText::new(&self.status_message).color(p.on_surface));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(
                                "Shortcuts: [↑/↓] Select  [Enter] Process  [Ctrl+S] Save Rule  [Del] Delete  [F5] Refresh  [F1] Help",
                            )
                            .small()
                            .color(p.on_surface_variant),
                        );
                    });
                });
            });

        // ====================================================================
        // LEFT SIDEBAR: WORKSPACE, SEARCH DIRECTORIES & TRANSFER QUEUE
        // ====================================================================
        egui::SidePanel::left("m3_left_sidebar")
            .default_width(360.0)
            .min_width(290.0)
            .max_width(520.0)
            .frame(
                egui::Frame::none()
                    .fill(p.surface_container_low)
                    .inner_margin(Margin::symmetric(12.0, 12.0))
                    .stroke(Stroke::new(1.0_f32, p.outline_variant)),
            )
            .show(ctx, |ui| {
                if is_advanced {
                    egui::Frame::none()
                        .fill(p.surface_container)
                        .rounding(Rounding::same(10.0))
                        .inner_margin(Margin::same(10.0))
                        .stroke(Stroke::new(1.0_f32, p.outline_variant))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
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

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .small_button("⇄ Sync")
                                            .on_hover_text(
                                                "Merge local Personal rules into Team workspace",
                                            )
                                            .clicked()
                                        {
                                            self.show_sync_modal = true;
                                        }
                                    },
                                );
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
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔍 Inbox Search Folders").strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .button("⟳ Refresh (F5)")
                                        .on_hover_text(
                                            "Scan search directories for transfer-* folders",
                                        )
                                        .clicked()
                                    {
                                        self.refresh_folders();
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Search 1:");
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.search_dir_1)
                                    .hint_text("Primary Inbox path...")
                                    .desired_width(ui.available_width() - 68.0),
                            );
                            if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                self.refresh_folders();
                            }
                            if ui
                                .button("Browse")
                                .on_hover_text("Pick primary Inbox directory")
                                .clicked()
                            {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    self.search_dir_1 = folder.to_string_lossy().to_string();
                                    self.refresh_folders();
                                }
                            }
                        });

                        if is_advanced || !self.search_dir_2.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("Search 2:");
                                let resp = ui.add(
                                    egui::TextEdit::singleline(&mut self.search_dir_2)
                                        .hint_text("Optional secondary path...")
                                        .desired_width(ui.available_width() - 68.0),
                                );
                                if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                    self.refresh_folders();
                                }
                                if ui
                                    .button("Browse")
                                    .on_hover_text("Pick optional secondary Inbox directory")
                                    .clicked()
                                {
                                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                        self.search_dir_2 = folder.to_string_lossy().to_string();
                                        self.refresh_folders();
                                    }
                                }
                            });
                        }
                    });

                ui.add_space(8.0);

                // 3. Queue Header & Legend
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Transfer Queue ({})", self.folders.len())).strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("✓ Ready   ✓(L) Logged   ✗ Setup")
                                .small()
                                .color(p.on_surface_variant),
                        )
                        .on_hover_text(
                            "✓ = Configured & ready\n✓(L) = Contains local Process.log from a previous run\n✗ = Needs a Target Folder",
                        );
                    });
                });

                ui.add_space(4.0);

                // 4. Scrollable Transfer Folder Queue List
                egui::ScrollArea::vertical()
                    .id_salt("transfer_queue_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.folders.is_empty() {
                            egui::Frame::none()
                                .fill(p.surface_container)
                                .rounding(Rounding::same(10.0))
                                .inner_margin(Margin::same(16.0))
                                .show(ui, |ui| {
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

                        for (idx, folder) in self.folders.iter().enumerate() {
                            let is_selected = self.selected_indices.contains(&idx);
                            let indicator = self.core.get_folder_status_indicator(folder);

                            let (badge_color, badge_bg) = match indicator {
                                "✓" => (p.success, p.success_container),
                                "✓ (L)" => (p.warning, p.warning_container),
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

                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&folder.folder_name)
                                                    .strong()
                                                    .color(text_color),
                                            );
                                            let sub = format!(
                                                "{} • ID: {} • {} file(s)",
                                                folder.created_date_str,
                                                folder.permit_id,
                                                folder.file_list.len()
                                            );
                                            ui.label(
                                                RichText::new(sub).small().color(if is_selected {
                                                    p.on_primary_container
                                                } else {
                                                    p.on_surface_variant
                                                }),
                                            );
                                        });
                                    });
                                })
                                .response
                                .interact(Sense::click());

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
                egui::ScrollArea::vertical()
                    .id_salt("central_main_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{icon} {}",
                                                guidance.headline
                                            ))
                                            .strong()
                                            .color(p.on_surface),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .small_button("✕ Hide Guide")
                                                    .on_hover_text(
                                                        "Hide this guidance banner (re-enable anytime via '💡 Guide' in the top bar or Ctrl+G)",
                                                    )
                                                    .clicked()
                                                {
                                                    self.core.app_settings.show_guidance = false;
                                                    self.core.save_settings();
                                                }
                                            },
                                        );
                                    });
                                    ui.label(
                                        RichText::new(&guidance.explanation)
                                            .small()
                                            .color(p.on_surface),
                                    );
                                    ui.label(
                                        RichText::new(&guidance.next_step_hint)
                                            .small()
                                            .strong()
                                            .color(banner_accent),
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
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new("🎯 Destinations & Processing Options")
                                            .strong(),
                                    );
                                    if self.is_rule_modified {
                                        ui.label(
                                            RichText::new("● Modified (Unsaved)")
                                                .small()
                                                .color(p.warning),
                                        )
                                        .on_hover_text(
                                            "Current settings differ from your saved workspace rule. Click 'Save Rule *' to keep them.",
                                        );
                                    }
                                });
                                ui.add_space(6.0);

                                // Target Folder Row
                                ui.horizontal(|ui| {
                                    ui.sized_label(
                                        140.0,
                                        "Target Folder *:",
                                        "Where extracted ZIP contents and loose files are placed (Required)",
                                        &p,
                                    );
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.active_rule.target_folder,
                                            )
                                            .hint_text("Required: Destination path for extracted/copied files...")
                                            .desired_width(ui.available_width() - 80.0),
                                        )
                                        .changed()
                                    {
                                        self.check_rule_modified();
                                    }
                                    if ui.button("Browse").clicked() {
                                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                            self.active_rule.target_folder =
                                                dir.to_string_lossy().to_string();
                                            self.check_rule_modified();
                                        }
                                    }
                                });

                                // Processed Folder Row
                                let need_processed =
                                    self.active_rule.post_action == PostAction::Move;
                                ui.horizontal(|ui| {
                                    ui.sized_label(
                                        140.0,
                                        if need_processed {
                                            "Processed Folder *:"
                                        } else {
                                            "Processed Folder:"
                                        },
                                        "Where the source transfer-* folder is moved after processing (Required when After Processing = Move)",
                                        &p,
                                    );
                                    if ui
                                        .add(
                                            egui::TextEdit::singleline(
                                                &mut self.active_rule.target_zip_folder,
                                            )
                                            .hint_text("Archive folder for processed transfer-* packages...")
                                            .desired_width(ui.available_width() - 80.0),
                                        )
                                        .changed()
                                    {
                                        self.check_rule_modified();
                                    }
                                    if ui.button("Browse").clicked() {
                                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                            self.active_rule.target_zip_folder =
                                                dir.to_string_lossy().to_string();
                                            self.check_rule_modified();
                                        }
                                    }
                                });

                                // Receipt Folder Row (Shown in Advanced Mode or if non-empty)
                                if is_advanced || !self.active_rule.receipt_folder.is_empty() {
                                    ui.horizontal(|ui| {
                                        ui.sized_label(
                                            140.0,
                                            "Receipt Folder:",
                                            "Optional: Separate folder to store timestamped receipt*.json files. Leave blank to place inside Target Folder.",
                                            &p,
                                        );
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(
                                                    &mut self.active_rule.receipt_folder,
                                                )
                                                .hint_text(
                                                    "Optional: Separate folder for receipt.json files...",
                                                )
                                                .desired_width(ui.available_width() - 80.0),
                                            )
                                            .changed()
                                        {
                                            self.check_rule_modified();
                                        }
                                        if ui.button("Browse").clicked() {
                                            if let Some(dir) = rfd::FileDialog::new().pick_folder()
                                            {
                                                self.active_rule.receipt_folder =
                                                    dir.to_string_lossy().to_string();
                                                self.check_rule_modified();
                                            }
                                        }
                                    });
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                // Processing Options Row
                                ui.horizontal_wrapped(|ui| {
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

                                    ui.add_space(16.0);
                                    ui.label(RichText::new("If File Exists:").strong());
                                    egui::ComboBox::from_id_salt("conflict_action_combo")
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

                                    ui.add_space(16.0);
                                    ui.label(RichText::new("After Processing:").strong());
                                    egui::ComboBox::from_id_salt("post_action_combo")
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
                            });

                        ui.add_space(12.0);

                        // 4. Primary Action Bar Card
                        egui::Frame::none()
                            .fill(p.surface_container)
                            .rounding(Rounding::same(12.0))
                            .inner_margin(Margin::symmetric(14.0, 10.0))
                            .stroke(Stroke::new(1.0_f32, p.outline_variant))
                            .show(ui, |ui| {
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
                                    .min_size(Vec2::new(165.0, 34.0));

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
                                    .min_size(Vec2::new(120.0, 34.0));

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
                                        .add_sized(
                                            [105.0, 34.0],
                                            egui::Button::new("📂 Open (Ctrl+O)"),
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
                                        .add_sized(
                                            [115.0, 34.0],
                                            egui::Button::new("📜 View Log (Ctrl+L)"),
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
                                            .add_sized(
                                                [125.0, 34.0],
                                                egui::Button::new("⚙ Manage Rules"),
                                            )
                                            .on_hover_text(
                                                "View, edit, or delete saved Config IDs and Wildcard Patterns (Ctrl+M)",
                                            )
                                            .clicked()
                                        {
                                            self.show_manage_rules_modal = true;
                                        }

                                        if ui
                                            .add_sized(
                                                [125.0, 34.0],
                                                egui::Button::new("🧾 Create Receipt"),
                                            )
                                            .on_hover_text(
                                                "Generate a standalone receipt.json file from current settings",
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
                                                        .join("receipt.json")
                                                        .to_string_lossy()
                                                        .to_string()
                                                })
                                                .unwrap_or_else(|| "receipt.json".to_string());
                                            self.show_create_receipt_modal = true;
                                        }
                                    }

                                    let del_btn = egui::Button::new(
                                        RichText::new("🗑 Delete").color(p.on_error_container),
                                    )
                                    .fill(p.error_container)
                                    .min_size(Vec2::new(90.0, 34.0));

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
                        ui.columns(2, |cols| {
                            egui::Frame::none()
                                .fill(p.surface_container)
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::same(12.0))
                                .stroke(Stroke::new(1.0_f32, p.outline_variant))
                                .show(&mut cols[0], |ui| {
                                    ui.label(
                                        RichText::new("📦 Package Files & ZIP Contents").strong(),
                                    );
                                    ui.separator();
                                    egui::ScrollArea::vertical()
                                        .id_salt("files_inspector_scroll")
                                        .max_height(260.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            if let Some(ref f) = current_folder {
                                                if f.file_list.is_empty() {
                                                    ui.label(
                                                        RichText::new("Folder is empty.")
                                                            .color(p.on_surface_variant),
                                                    );
                                                } else {
                                                    for item in &f.file_list {
                                                        let is_zip =
                                                            item.to_lowercase().ends_with(".zip");
                                                        ui.label(
                                                            RichText::new(item)
                                                                .monospace()
                                                                .small()
                                                                .color(if is_zip {
                                                                    p.primary
                                                                } else {
                                                                    p.on_surface
                                                                }),
                                                        );
                                                        if is_zip {
                                                            for zc in &f.zip_contents {
                                                                ui.label(
                                                                    RichText::new(format!(
                                                                        "  └─ {zc}"
                                                                    ))
                                                                    .monospace()
                                                                    .small()
                                                                    .color(p.on_surface_variant),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            } else {
                                                ui.label(
                                                    RichText::new(
                                                        "Select a transfer folder to inspect its contents.",
                                                    )
                                                    .color(p.on_surface_variant),
                                                );
                                            }
                                        });
                                });

                            // Right Column: Receipt JSON Inspector
                            egui::Frame::none()
                                .fill(p.surface_container)
                                .rounding(Rounding::same(12.0))
                                .inner_margin(Margin::same(12.0))
                                .stroke(Stroke::new(1.0_f32, p.outline_variant))
                                .show(&mut cols[1], |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("🧾 Receipt JSON Inspector").strong(),
                                        );
                                        if let Some(ref f) = current_folder {
                                            if f.receipt_warning.is_some() {
                                                ui.label(
                                                    RichText::new("⚠ Warning")
                                                        .small()
                                                        .strong()
                                                        .color(p.error),
                                                );
                                            }
                                        }
                                    });
                                    ui.separator();
                                    egui::ScrollArea::vertical()
                                        .id_salt("receipt_inspector_scroll")
                                        .max_height(260.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            if let Some(ref f) = current_folder {
                                                if !f.receipt_raw.trim().is_empty() {
                                                    ui.label(
                                                        RichText::new(&f.receipt_raw)
                                                            .monospace()
                                                            .small()
                                                            .color(p.on_surface),
                                                    );
                                                } else {
                                                    ui.label(
                                                        RichText::new(
                                                            "No receipt.json found in this transfer folder (Config ID: DEFAULT/No_ID).\n\nTip: You can automate transfers without receipts by saving a Wildcard Pattern rule!",
                                                        )
                                                        .small()
                                                        .color(p.on_surface_variant),
                                                    );
                                                }
                                            } else {
                                                ui.label(
                                                    RichText::new("No folder selected.")
                                                        .color(p.on_surface_variant),
                                                );
                                            }
                                        });
                                });
                        });
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

impl InboxMoverGuiApp {
    fn render_modals(&mut self, ctx: &egui::Context, p: &M3Palette) {
        // 1. First-Time Welcome & Experience Selector Modal
        if self.show_welcome_modal {
            egui::Window::new("👋 Welcome to Inbox Mover")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(560.0, 380.0))
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("Modern Transfer Folder Unpacker & Rule Engine")
                            .strong()
                            .size(16.0)
                            .color(p.primary),
                    );
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
                                ui.label(RichText::new("🌱 New to Inbox Mover?").strong());
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "• Streamlined Simple Mode\n• Smart Context Guide explains what each folder needs\n• Interactive Help Center (F1)",
                                    )
                                    .small(),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("Start in Simple Mode (Guided)")
                                            .strong()
                                            .color(p.primary),
                                    )
                                    .clicked()
                                {
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
                                ui.label(RichText::new("⚡ Experienced / Daily User?").strong());
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "• Full Advanced Mode unlocked\n• Hides Smart Guidance banners so they never get in your way\n• Full keyboard navigation",
                                    )
                                    .small(),
                                );
                                ui.add_space(8.0);
                                if ui
                                    .button(
                                        RichText::new("Start in Power User Mode (No Banners)")
                                            .strong(),
                                    )
                                    .clicked()
                                {
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
                            if ui.button("✕ Close (Esc)").clicked() {
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
                            if ui.button("✕ Close").clicked() {
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
            egui::Window::new("⇄ Sync / Merge Workspaces")
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
                        if ui.button("⇄ Merge Local -> Team").clicked() {
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

        // 6. Create Receipt Modal
        if self.show_create_receipt_modal {
            egui::Window::new("🧾 Create Custom receipt.json")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .fixed_size(Vec2::new(500.0, 280.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Permit / Config ID:");
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
                        ui.text_edit_singleline(&mut self.receipt_save_path);
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("💾 Generate receipt.json").clicked() {
                            let path = PathBuf::from(self.receipt_save_path.trim());
                            let pid = if self.receipt_permit_id.trim().is_empty() {
                                None
                            } else {
                                Some(self.receipt_permit_id.trim())
                            };
                            match self.core.create_receipt_file(&path, &self.receipt_rule, pid) {
                                Ok(()) => {
                                    self.show_create_receipt_modal = false;
                                    self.refresh_folders();
                                    self.status_message = format!("Created receipt at {:?}", path);
                                    self.status_is_error = false;
                                }
                                Err(e) => {
                                    self.status_message = format!("Failed creating receipt: {e}");
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
                            if ui.button("✕ Close").clicked() {
                                self.show_log_modal = false;
                            }
                        });
                    });
                    ui.separator();

                    match self.log_tab {
                        LogTab::LocalFolder => {
                            egui::ScrollArea::vertical()
                                .id_salt("local_log_scroll")
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&self.local_log_content)
                                            .monospace()
                                            .small(),
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

/// Launches the cross-platform Material 3 GUI (identical on Windows and Linux).
pub fn run_gui(core: InboxMoverCore) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Inbox Mover — Transfer Folder Processor")
            .with_inner_size([1240.0, 800.0])
            .with_min_inner_size([920.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Inbox Mover",
        options,
        Box::new(|cc| Ok(Box::new(InboxMoverGuiApp::new(cc, core)))),
    )
    .map_err(|e| format!("Failed to start GUI: {e}"))
}
