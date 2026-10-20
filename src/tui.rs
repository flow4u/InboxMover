use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};

use crate::core::InboxMoverCore;
use crate::help::{analyze_context, GuidanceLevel, HELP_SECTIONS, QUICK_TIPS, SHORTCUTS};
use crate::models::{ConflictAction, ConfigRule, FolderData, PostAction, ProcessLogEntry};

// Material 3 inspired TrueColor palette for modern terminals
const M3_BG: Color = Color::Rgb(17, 19, 24);
const M3_SURFACE: Color = Color::Rgb(29, 32, 38);
const M3_SURFACE_HIGH: Color = Color::Rgb(42, 46, 55);
const M3_PRIMARY: Color = Color::Rgb(168, 199, 250);
const M3_PRIMARY_CONTAINER: Color = Color::Rgb(40, 71, 119);
const M3_ON_SURFACE: Color = Color::Rgb(226, 226, 233);
const M3_MUTED: Color = Color::Rgb(150, 154, 168);
const M3_SUCCESS: Color = Color::Rgb(129, 201, 149);
const M3_WARNING: Color = Color::Rgb(253, 214, 99);
const M3_ERROR: Color = Color::Rgb(255, 180, 171);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ActivePane {
    Queue,
    Destinations,
    Inspector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TuiModal {
    None,
    Welcome,
    Help { tab: usize, topic: usize },
    EditInput {
        title: String,
        field: EditField,
        buffer: String,
    },
    SaveRule {
        mode: usize, // 0 = Config ID, 1 = Pattern, 2 = DEFAULT
        input: String,
    },
    ManageRules {
        tab: usize, // 0 = IDs, 1 = Patterns
        selected: usize,
    },
    Logs {
        tab: usize, // 0 = Local, 1 = Global
        scroll: u16,
    },
    DeleteConfirm,
    PasswordPrompt {
        zip_name: String,
        buffer: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditField {
    SearchDir1,
    SearchDir2,
    TargetFolder,
    ProcessedFolder,
    ReceiptFolder,
    CreateReceiptFile,
}

pub struct TuiApp {
    core: InboxMoverCore,
    folders: Vec<FolderData>,
    queue_state: ListState,
    selected_indices: BTreeSet<usize>,

    active_pane: ActivePane,
    active_rule: ConfigRule,
    active_rule_source: String,
    is_rule_modified: bool,

    search_dir_1: String,
    search_dir_2: String,

    inspector_scroll: u16,
    status_message: String,
    status_is_error: bool,

    modal: TuiModal,
    local_log_text: String,
    global_logs: Vec<ProcessLogEntry>,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(core: InboxMoverCore) -> Self {
        let search_dir_1 = core.app_settings.search_folder_1.clone();
        let search_dir_2 = core.app_settings.search_folder_2.clone();
        let show_welcome = core.app_settings.show_welcome_splash;

        let mut app = Self {
            core,
            folders: Vec::new(),
            queue_state: ListState::default(),
            selected_indices: BTreeSet::new(),
            active_pane: ActivePane::Queue,
            active_rule: ConfigRule::default(),
            active_rule_source: "Default".to_string(),
            is_rule_modified: false,
            search_dir_1,
            search_dir_2,
            inspector_scroll: 0,
            status_message: "Welcome! Press '?' or F1 for Help & Shortcuts, 'g' to toggle Smart Guide.".to_string(),
            status_is_error: false,
            modal: if show_welcome {
                TuiModal::Welcome
            } else {
                TuiModal::None
            },
            local_log_text: String::new(),
            global_logs: Vec::new(),
            should_quit: false,
        };

        app.refresh_folders();
        app
    }

    fn current_index(&self) -> Option<usize> {
        self.queue_state.selected().filter(|&i| i < self.folders.len())
    }

    fn current_folder(&self) -> Option<&FolderData> {
        self.current_index().and_then(|i| self.folders.get(i))
    }

    fn refresh_folders(&mut self) {
        self.core.app_settings.search_folder_1 = self.search_dir_1.trim().to_string();
        self.core.app_settings.search_folder_2 = self.search_dir_2.trim().to_string();
        self.core.save_settings();
        self.core.reload_cache();

        self.folders = self
            .core
            .scan_search_directories(&self.search_dir_1, &self.search_dir_2);
        self.selected_indices.clear();

        if self.folders.is_empty() {
            self.queue_state.select(None);
            let (rule, src, modified) = self.core.resolve_rule_for_folder(None);
            self.active_rule = rule;
            self.active_rule_source = src;
            self.is_rule_modified = modified;
            self.status_message =
                "No 'transfer-*' folders found. Press '/' to change Search 1 path.".to_string();
            self.status_is_error = false;
        } else {
            let idx = self
                .queue_state
                .selected()
                .map(|i| i.min(self.folders.len() - 1))
                .unwrap_or(0);
            self.queue_state.select(Some(idx));
            self.selected_indices.insert(idx);
            self.sync_active_rule();
            self.status_message = format!("Loaded {} transfer folder(s).", self.folders.len());
            self.status_is_error = false;
        }
    }

    fn sync_active_rule(&mut self) {
        let folder = self.current_folder().cloned();
        let (rule, src, modified) = self.core.resolve_rule_for_folder(folder.as_ref());
        self.active_rule = rule;
        self.active_rule_source = src;
        self.is_rule_modified = modified;
        self.inspector_scroll = 0;
    }

    fn check_rule_modified(&mut self) {
        if let Some(f) = self.current_folder() {
            if let Some((saved_rule, _)) = self.core.get_saved_rule(f) {
                self.is_rule_modified = !self.active_rule.same_settings(&saved_rule);
            } else {
                self.is_rule_modified = true;
            }
        }
    }

    fn execute_process(&mut self, password_override: Option<String>) {
        let mut indices: Vec<usize> = self.selected_indices.iter().copied().collect();
        if indices.is_empty() {
            if let Some(idx) = self.current_index() {
                indices.push(idx);
            }
        }
        if indices.is_empty() {
            self.status_message = "No folder selected to process.".to_string();
            self.status_is_error = true;
            return;
        }

        let mut selected_folders: Vec<FolderData> = indices
            .into_iter()
            .filter_map(|i| self.folders.get(i).cloned())
            .collect();
        selected_folders.sort_by_key(|f| f.created_timestamp);

        let is_batch = selected_folders.len() > 1;
        let mut ok_count = 0;
        let mut errs = Vec::new();
        let needed_pw: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        for folder in &selected_folders {
            let rule_to_use = if is_batch {
                let (r, _, _) = self.core.resolve_rule_for_folder(Some(folder));
                r
            } else {
                self.active_rule.clone()
            };

            let pw_clone = password_override.clone();
            let pw_ref = Arc::clone(&needed_pw);

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
                    if let Ok(mut g) = pw_ref.lock() {
                        *g = Some(zip_name.to_string());
                    }
                    None
                }),
            );

            match res {
                Ok(()) => ok_count += 1,
                Err(e) => {
                    if let Ok(g) = needed_pw.lock() {
                        if let Some(ref zname) = *g {
                            self.modal = TuiModal::PasswordPrompt {
                                zip_name: zname.clone(),
                                buffer: String::new(),
                            };
                            self.status_message =
                                format!("Password required for encrypted archive '{zname}'.");
                            self.status_is_error = true;
                            return;
                        }
                    }
                    errs.push(format!("{}: {}", folder.folder_name, e));
                }
            }
        }

        self.refresh_folders();
        if errs.is_empty() {
            self.status_message = format!("Successfully processed {ok_count} folder(s)!");
            self.status_is_error = false;
        } else {
            self.status_message = format!(
                "Processed {ok_count} folder(s); {} error(s): {}",
                errs.len(),
                errs.join(" | ")
            );
            self.status_is_error = true;
        }
    }

    fn open_logs_modal(&mut self) {
        if let Some(f) = self.current_folder() {
            let p1 = f.folder_path.join("Process.log");
            let p2 = f.folder_path.join("inbox process.log");
            if p1.exists() {
                self.local_log_text = std::fs::read_to_string(p1).unwrap_or_default();
            } else if p2.exists() {
                self.local_log_text = std::fs::read_to_string(p2).unwrap_or_default();
            } else {
                self.local_log_text =
                    "No local Process.log exists inside this folder yet.".to_string();
            }
        } else {
            self.local_log_text = "No folder selected.".to_string();
        }
        self.global_logs = self.core.read_global_logs(100);
        self.modal = TuiModal::Logs { tab: 0, scroll: 0 };
    }

    fn handle_key(&mut self, key: event::KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Global Ctrl+C quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        // Dispatch modal keys first
        match &mut self.modal {
            TuiModal::Welcome => {
                match key.code {
                    KeyCode::Char('1') | KeyCode::Enter => {
                        self.core.app_settings.simple_mode = true;
                        self.core.app_settings.show_guidance = true;
                        self.core.app_settings.show_welcome_splash = false;
                        self.core.save_settings();
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Char('2') => {
                        self.core.app_settings.simple_mode = false;
                        self.core.app_settings.show_guidance = false;
                        self.core.app_settings.show_welcome_splash = false;
                        self.core.save_settings();
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.modal = TuiModal::None;
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::Help { tab, topic } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::F(1) | KeyCode::Char('?') => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                        *tab = (*tab + 1) % 3;
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        *tab = if *tab == 0 { 2 } else { *tab - 1 };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *tab == 0 {
                            *topic = (*topic + 1).min(HELP_SECTIONS.len() - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *tab == 0 {
                            *topic = topic.saturating_sub(1);
                        }
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::EditInput {
                field,
                buffer,
                ..
            } => {
                let field_copy = *field;
                match key.code {
                    KeyCode::Esc => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Enter => {
                        let val = buffer.trim().to_string();
                        self.modal = TuiModal::None;
                        match field_copy {
                            EditField::SearchDir1 => {
                                self.search_dir_1 = val;
                                self.refresh_folders();
                            }
                            EditField::SearchDir2 => {
                                self.search_dir_2 = val;
                                self.refresh_folders();
                            }
                            EditField::TargetFolder => {
                                self.active_rule.target_folder = val;
                                self.check_rule_modified();
                            }
                            EditField::ProcessedFolder => {
                                self.active_rule.target_zip_folder = val;
                                self.check_rule_modified();
                            }
                            EditField::ReceiptFolder => {
                                self.active_rule.receipt_folder = val;
                                self.check_rule_modified();
                            }
                            EditField::CreateReceiptFile => {
                                let path = PathBuf::from(&val);
                                let pid = self
                                    .current_folder()
                                    .map(|f| f.permit_id.clone())
                                    .unwrap_or_else(|| "DEFAULT".to_string());
                                match self.core.create_receipt_file(
                                    &path,
                                    &self.active_rule,
                                    Some(&pid),
                                ) {
                                    Ok(()) => {
                                        self.refresh_folders();
                                        self.status_message =
                                            format!("Generated receipt at {val}");
                                        self.status_is_error = false;
                                    }
                                    Err(e) => {
                                        self.status_message =
                                            format!("Receipt error: {e}");
                                        self.status_is_error = true;
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        buffer.push(c);
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::SaveRule { mode, input } => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                        *mode = (*mode + 1) % 3;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                    }
                    KeyCode::Enter => {
                        let m = *mode;
                        let inp = input.trim().to_string();
                        let rule = self.active_rule.clone();
                        self.modal = TuiModal::None;
                        let res = match m {
                            0 => self.core.save_config(&inp, rule),
                            1 => self.core.save_pattern(&inp, rule),
                            _ => self.core.save_config("DEFAULT", rule),
                        };
                        match res {
                            Ok(()) => {
                                self.sync_active_rule();
                                self.status_message = "Rule saved to workspace!".to_string();
                                self.status_is_error = false;
                            }
                            Err(e) => {
                                self.status_message = format!("Save failed: {e}");
                                self.status_is_error = true;
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::ManageRules { tab, selected } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                        *tab = (*tab + 1) % 2;
                        *selected = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let len = if *tab == 0 {
                            self.core.config_cache.len()
                        } else {
                            self.core.patterns_cache.len()
                        };
                        if len > 0 {
                            *selected = (*selected + 1).min(len - 1);
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        *selected = selected.saturating_sub(1);
                    }
                    KeyCode::Delete | KeyCode::Char('d') => {
                        if *tab == 0 {
                            let mut ids: Vec<String> =
                                self.core.config_cache.keys().cloned().collect();
                            ids.sort();
                            if let Some(id) = ids.get(*selected) {
                                if id != "DEFAULT" {
                                    let _ = self.core.delete_config(id);
                                    self.sync_active_rule();
                                }
                            }
                        } else {
                            let mut pats: Vec<String> =
                                self.core.patterns_cache.keys().cloned().collect();
                            pats.sort();
                            if let Some(p) = pats.get(*selected) {
                                let _ = self.core.delete_pattern(p);
                                self.sync_active_rule();
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::Logs { tab, scroll } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l') => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                        *tab = (*tab + 1) % 2;
                        *scroll = 0;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        *scroll = scroll.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        *scroll = scroll.saturating_sub(1);
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::DeleteConfirm => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        let paths: Vec<PathBuf> = self
                            .selected_indices
                            .iter()
                            .filter_map(|&i| self.folders.get(i).map(|f| f.folder_path.clone()))
                            .collect();
                        let mut count = 0;
                        for p in paths {
                            if std::fs::remove_dir_all(p).is_ok() {
                                count += 1;
                            }
                        }
                        self.modal = TuiModal::None;
                        self.refresh_folders();
                        self.status_message = format!("Deleted {count} folder(s).");
                        self.status_is_error = false;
                    }
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                        self.modal = TuiModal::None;
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::PasswordPrompt { buffer, .. } => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal = TuiModal::None;
                    }
                    KeyCode::Backspace => {
                        buffer.pop();
                    }
                    KeyCode::Char(c) => {
                        buffer.push(c);
                    }
                    KeyCode::Enter => {
                        let pw = buffer.clone();
                        self.modal = TuiModal::None;
                        self.execute_process(Some(pw));
                    }
                    _ => {}
                }
                return;
            }
            TuiModal::None => {}
        }

        // Normal Main Screen Shortcuts
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::F(1) | KeyCode::Char('?') => {
                self.modal = TuiModal::Help { tab: 0, topic: 0 };
            }
            KeyCode::Char('g') => {
                self.core.app_settings.show_guidance =
                    !self.core.app_settings.show_guidance;
                self.core.save_settings();
            }
            KeyCode::Char('a') => {
                self.core.app_settings.simple_mode = !self.core.app_settings.simple_mode;
                self.core.save_settings();
            }
            KeyCode::Tab => {
                self.active_pane = match self.active_pane {
                    ActivePane::Queue => ActivePane::Destinations,
                    ActivePane::Destinations => ActivePane::Inspector,
                    ActivePane::Inspector => ActivePane::Queue,
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.active_pane == ActivePane::Inspector {
                    self.inspector_scroll = self.inspector_scroll.saturating_add(1);
                } else if !self.folders.is_empty() {
                    let next = self
                        .current_index()
                        .map(|i| (i + 1).min(self.folders.len() - 1))
                        .unwrap_or(0);
                    self.queue_state.select(Some(next));
                    if self.selected_indices.len() <= 1 {
                        self.selected_indices.clear();
                        self.selected_indices.insert(next);
                    }
                    self.sync_active_rule();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.active_pane == ActivePane::Inspector {
                    self.inspector_scroll = self.inspector_scroll.saturating_sub(1);
                } else if !self.folders.is_empty() {
                    let prev = self
                        .current_index()
                        .map(|i| i.saturating_sub(1))
                        .unwrap_or(0);
                    self.queue_state.select(Some(prev));
                    if self.selected_indices.len() <= 1 {
                        self.selected_indices.clear();
                        self.selected_indices.insert(prev);
                    }
                    self.sync_active_rule();
                }
            }
            KeyCode::Char(' ') => {
                if let Some(idx) = self.current_index() {
                    if self.selected_indices.contains(&idx) && self.selected_indices.len() > 1 {
                        self.selected_indices.remove(&idx);
                    } else {
                        self.selected_indices.insert(idx);
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('p') => {
                self.execute_process(None);
            }
            KeyCode::Char('r') | KeyCode::F(5) => {
                self.refresh_folders();
            }
            KeyCode::Char('/') => {
                self.modal = TuiModal::EditInput {
                    title: "Edit Primary Inbox Path (Search 1)".to_string(),
                    field: EditField::SearchDir1,
                    buffer: self.search_dir_1.clone(),
                };
            }
            KeyCode::Char('2') => {
                self.modal = TuiModal::EditInput {
                    title: "Edit Secondary Inbox Path (Search 2)".to_string(),
                    field: EditField::SearchDir2,
                    buffer: self.search_dir_2.clone(),
                };
            }
            KeyCode::Char('t') => {
                self.modal = TuiModal::EditInput {
                    title: "Edit Target Folder Path (Required)".to_string(),
                    field: EditField::TargetFolder,
                    buffer: self.active_rule.target_folder.clone(),
                };
            }
            KeyCode::Char('z') => {
                self.modal = TuiModal::EditInput {
                    title: "Edit Processed Archive Folder Path".to_string(),
                    field: EditField::ProcessedFolder,
                    buffer: self.active_rule.target_zip_folder.clone(),
                };
            }
            KeyCode::Char('f') => {
                self.modal = TuiModal::EditInput {
                    title: "Edit Optional Receipt Folder Path".to_string(),
                    field: EditField::ReceiptFolder,
                    buffer: self.active_rule.receipt_folder.clone(),
                };
            }
            KeyCode::Char('e') => {
                self.active_rule.auto_extract = !self.active_rule.auto_extract;
                self.check_rule_modified();
            }
            KeyCode::Char('x') => {
                self.active_rule.conflict_action = match self.active_rule.conflict_action {
                    ConflictAction::Overwrite => ConflictAction::KeepBoth,
                    ConflictAction::KeepBoth => ConflictAction::RenameExisting,
                    ConflictAction::RenameExisting => ConflictAction::Overwrite,
                };
                self.check_rule_modified();
            }
            KeyCode::Char('u') => {
                self.active_rule.post_action = match self.active_rule.post_action {
                    PostAction::Move => PostAction::Delete,
                    PostAction::Delete => PostAction::Leave,
                    PostAction::Leave => PostAction::Move,
                };
                self.check_rule_modified();
            }
            KeyCode::Char('w') => {
                let next_ws = if self.core.app_settings.use_global {
                    "personal"
                } else {
                    "team"
                };
                self.core.switch_workspace(next_ws);
                self.refresh_folders();
            }
            KeyCode::Char('v') => {
                self.core.app_settings.overrule_receipt = !self.core.app_settings.overrule_receipt;
                self.core.save_settings();
                self.sync_active_rule();
            }
            KeyCode::Char('s') => {
                let (mode, input) = if let Some(f) = self.current_folder() {
                    if f.permit_id != "No_ID" {
                        (0, f.permit_id.clone())
                    } else {
                        (
                            1,
                            f.matched_pattern.clone().unwrap_or_else(|| {
                                format!("*{}*", f.folder_name.trim_start_matches("transfer-"))
                            }),
                        )
                    }
                } else {
                    (2, "DEFAULT".to_string())
                };
                self.modal = TuiModal::SaveRule { mode, input };
            }
            KeyCode::Char('m') => {
                self.modal = TuiModal::ManageRules {
                    tab: 0,
                    selected: 0,
                };
            }
            KeyCode::Char('c') => {
                let default_path = self
                    .current_folder()
                    .map(|f| {
                        f.folder_path
                            .join("receipt.json")
                            .to_string_lossy()
                            .to_string()
                    })
                    .unwrap_or_else(|| "receipt.json".to_string());
                self.modal = TuiModal::EditInput {
                    title: "Generate receipt.json at Path".to_string(),
                    field: EditField::CreateReceiptFile,
                    buffer: default_path,
                };
            }
            KeyCode::Char('l') => {
                self.open_logs_modal();
            }
            KeyCode::Char('o') => {
                if let Some(f) = self.current_folder() {
                    let _ = open::that(&f.folder_path);
                }
            }
            KeyCode::Delete | KeyCode::Char('d') => {
                if !self.selected_indices.is_empty() {
                    self.modal = TuiModal::DeleteConfirm;
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(Block::default().style(Style::default().bg(M3_BG)), area);

        let show_guide = self.core.app_settings.show_guidance;
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top App Bar
                Constraint::Length(if show_guide { 4 } else { 0 }), // Smart Context Banner
                Constraint::Min(12),   // Body Split
                Constraint::Length(3), // Footer Status & Shortcut Bar
            ])
            .split(area);

        // 1. Top Header Bar
        let mode_badge = if !self.core.app_settings.simple_mode {
            Span::styled(
                " [⚙ ADVANCED MODE (a)] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(M3_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                " [✨ SIMPLE MODE (a)] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(M3_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        };

        let ws_badge = Span::styled(
            format!(
                " WS: {} (w) ",
                if self.core.app_settings.use_global {
                    "TEAM"
                } else {
                    "PERSONAL"
                }
            ),
            Style::default().fg(M3_ON_SURFACE).bg(M3_PRIMARY_CONTAINER),
        );

        let overrule_badge = Span::styled(
            format!(
                " Overrule Receipt: {} (v) ",
                if self.core.app_settings.overrule_receipt {
                    "ON"
                } else {
                    "OFF"
                }
            ),
            Style::default().fg(if self.core.app_settings.overrule_receipt {
                M3_WARNING
            } else {
                M3_MUTED
            }),
        );

        let guide_badge = Span::styled(
            format!(
                " 💡 Guide: {} (g) ",
                if show_guide { "ON" } else { "OFF" }
            ),
            Style::default().fg(M3_PRIMARY),
        );

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " 📦 INBOX MOVER TUI ",
                Style::default()
                    .fg(M3_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            mode_badge,
            Span::raw("  "),
            ws_badge,
            Span::raw("  "),
            overrule_badge,
            Span::raw("  "),
            guide_badge,
            Span::raw("   "),
            Span::styled(
                "[? / F1] Help Center",
                Style::default()
                    .fg(M3_WARNING)
                    .add_modifier(Modifier::BOLD),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(M3_SURFACE).fg(M3_ON_SURFACE)),
        );
        f.render_widget(header, main_chunks[0]);

        // 2. Contextual Smart Guidance Banner (when enabled)
        if show_guide {
            let folder_ref = self.current_folder();
            let guidance = analyze_context(
                folder_ref,
                &self.active_rule,
                &self.active_rule_source,
                self.is_rule_modified,
                self.core.app_settings.overrule_receipt,
            );
            let border_color = match guidance.level {
                GuidanceLevel::Ready => M3_SUCCESS,
                GuidanceLevel::Modified => M3_PRIMARY,
                GuidanceLevel::AttentionRequired => M3_ERROR,
                GuidanceLevel::PreviouslyProcessed => M3_WARNING,
                GuidanceLevel::Idle => M3_MUTED,
            };

            let guide_widget = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("💡 {} ", guidance.headline),
                        Style::default()
                            .fg(border_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("— {}", guidance.explanation),
                        Style::default().fg(M3_ON_SURFACE),
                    ),
                ]),
                Line::from(Span::styled(
                    format!("   👉 {}  (Press 'g' to hide banner)", guidance.next_step_hint),
                    Style::default().fg(M3_MUTED),
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(M3_SURFACE)),
            );
            f.render_widget(guide_widget, main_chunks[1]);
        }

        // 3. Body Split: Left Sidebar (Queue & Search) + Right Panel (Destinations & Inspector)
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(36), Constraint::Percentage(64)])
            .split(main_chunks[2]);

        // Left Sidebar: Search Paths + Queue List
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(6)])
            .split(body_chunks[0]);

        let search_info = Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Search 1 (/): ", Style::default().fg(M3_PRIMARY)),
                Span::raw(&self.search_dir_1),
            ]),
            Line::from(vec![
                Span::styled("Search 2 (2): ", Style::default().fg(M3_MUTED)),
                Span::raw(if self.search_dir_2.is_empty() {
                    "<none>"
                } else {
                    &self.search_dir_2
                }),
            ]),
        ])
        .block(
            Block::default()
                .title(" 🔍 Search Directories [r: Refresh] ")
                .borders(Borders::ALL)
                .style(Style::default().bg(M3_SURFACE).fg(M3_ON_SURFACE)),
        );
        f.render_widget(search_info, left_chunks[0]);

        let items: Vec<ListItem> = self
            .folders
            .iter()
            .enumerate()
            .map(|(i, folder)| {
                let ind = self.core.get_folder_status_indicator(folder);
                let ind_color = match ind {
                    "✓" => M3_SUCCESS,
                    "✓ (L)" => M3_WARNING,
                    _ => M3_ERROR,
                };
                let multi_mark = if self.selected_indices.contains(&i)
                    && self.selected_indices.len() > 1
                {
                    "[*] "
                } else {
                    ""
                };

                ListItem::new(vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" [{ind}] "),
                            Style::default()
                                .fg(ind_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{multi_mark}{}", folder.folder_name),
                            Style::default()
                                .fg(M3_ON_SURFACE)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(Span::styled(
                        format!(
                            "     {} | ID: {} | {} files",
                            folder.created_date_str,
                            folder.permit_id,
                            folder.file_list.len()
                        ),
                        Style::default().fg(M3_MUTED),
                    )),
                ])
            })
            .collect();

        let queue_border = if self.active_pane == ActivePane::Queue {
            M3_PRIMARY
        } else {
            M3_MUTED
        };
        let queue_list = List::new(items)
            .block(
                Block::default()
                    .title(format!(
                        " 📂 Transfer Queue ({}) [Space: Multi-Select] ",
                        self.folders.len()
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(queue_border))
                    .style(Style::default().bg(M3_SURFACE)),
            )
            .highlight_style(
                Style::default()
                    .bg(M3_PRIMARY_CONTAINER)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(queue_list, left_chunks[1], &mut self.queue_state);

        // Right Panel: Destinations Card + Split Inspector
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(9), Constraint::Min(6)])
            .split(body_chunks[1]);

        let dest_border = if self.active_pane == ActivePane::Destinations {
            M3_PRIMARY
        } else {
            M3_MUTED
        };
        let mod_tag = if self.is_rule_modified {
            " ● Modified (Press 's' to Save Rule *)"
        } else {
            ""
        };

        let dest_lines = vec![
            Line::from(vec![
                Span::styled(
                    " [t] Target Folder *:    ",
                    Style::default().fg(M3_PRIMARY).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if self.active_rule.target_folder.is_empty() {
                        "<REQUIRED — press 't' to set>"
                    } else {
                        &self.active_rule.target_folder
                    },
                    Style::default().fg(if self.active_rule.target_folder.is_empty() {
                        M3_ERROR
                    } else {
                        M3_ON_SURFACE
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled(" [z] Processed Folder:   ", Style::default().fg(M3_PRIMARY)),
                Span::raw(if self.active_rule.target_zip_folder.is_empty() {
                    "<not set>"
                } else {
                    &self.active_rule.target_zip_folder
                }),
            ]),
            Line::from(vec![
                Span::styled(" [f] Receipt Folder:     ", Style::default().fg(M3_PRIMARY)),
                Span::raw(if self.active_rule.receipt_folder.is_empty() {
                    "<defaults to Target Folder>"
                } else {
                    &self.active_rule.receipt_folder
                }),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [e] Auto-Extract ZIPs: ", Style::default().fg(M3_MUTED)),
                Span::styled(
                    if self.active_rule.auto_extract {
                        "YES "
                    } else {
                        "NO  "
                    },
                    Style::default()
                        .fg(M3_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  [x] Conflict: ", Style::default().fg(M3_MUTED)),
                Span::styled(
                    self.active_rule.conflict_action.label(),
                    Style::default().fg(M3_PRIMARY),
                ),
                Span::styled("  [u] Post-Action: ", Style::default().fg(M3_MUTED)),
                Span::styled(
                    self.active_rule.post_action.label(),
                    Style::default().fg(M3_WARNING),
                ),
            ]),
            Line::from(vec![Span::styled(
                " Actions: [Enter/p] Process  [s] Save Rule  [m] Manage Rules  [l] View Logs  [c] Create Receipt  [d] Delete",
                Style::default().fg(M3_MUTED),
            )]),
        ];

        let dest_block = Paragraph::new(dest_lines).block(
            Block::default()
                .title(format!(
                    " 🎯 Destinations & Rules (Source: {}){mod_tag} ",
                    self.active_rule_source
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(dest_border))
                .style(Style::default().bg(M3_SURFACE).fg(M3_ON_SURFACE)),
        );
        f.render_widget(dest_block, right_chunks[0]);

        // Inspector Split: Files Tree (Left) + Receipt JSON (Right)
        let insp_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right_chunks[1]);

        let insp_border = if self.active_pane == ActivePane::Inspector {
            M3_PRIMARY
        } else {
            M3_MUTED
        };

        let files_lines: Vec<Line> = if let Some(folder) = self.current_folder() {
            if folder.file_list.is_empty() && folder.zip_contents.is_empty() {
                vec![Line::from("Folder is empty.")]
            } else {
                let mut lines: Vec<Line> = folder
                    .file_list
                    .iter()
                    .map(|item| {
                        let col = if item.to_lowercase().ends_with(".zip") {
                            M3_PRIMARY
                        } else {
                            M3_ON_SURFACE
                        };
                        Line::from(Span::styled(item.clone(), Style::default().fg(col)))
                    })
                    .collect();
                for z in &folder.zip_contents {
                    lines.push(Line::from(Span::styled(
                        z.clone(),
                        Style::default().fg(M3_MUTED),
                    )));
                }
                lines
            }
        } else {
            vec![Line::from("No transfer folder selected.")]
        };

        let files_panel = Paragraph::new(files_lines)
            .scroll((self.inspector_scroll, 0))
            .block(
                Block::default()
                    .title(" 📦 Package Files & ZIP Contents ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(insp_border))
                    .style(Style::default().bg(M3_SURFACE)),
            );
        f.render_widget(files_panel, insp_chunks[0]);

        let receipt_text = if let Some(folder) = self.current_folder() {
            if folder.receipt_raw.trim().is_empty() {
                "No receipt.json found in this folder (Config ID: No_ID).\n\nTip: Press 's' to save a Wildcard Pattern rule, or 'c' to generate a receipt.json.".to_string()
            } else {
                folder.receipt_raw.clone()
            }
        } else {
            "No folder selected.".to_string()
        };

        let receipt_panel = Paragraph::new(receipt_text)
            .wrap(Wrap { trim: false })
            .scroll((self.inspector_scroll, 0))
            .block(
                Block::default()
                    .title(" 🧾 Receipt JSON Inspector ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(insp_border))
                    .style(Style::default().bg(M3_SURFACE).fg(M3_ON_SURFACE)),
            );
        f.render_widget(receipt_panel, insp_chunks[1]);

        // 4. Footer Status & Shortcut Bar
        let status_col = if self.status_is_error {
            M3_ERROR
        } else {
            M3_SUCCESS
        };
        let footer = Paragraph::new(Line::from(vec![
            Span::styled(" ● ", Style::default().fg(status_col)),
            Span::styled(&self.status_message, Style::default().fg(M3_ON_SURFACE)),
            Span::raw("   |   "),
            Span::styled(
                "[↑/↓] Select  [Enter/p] Process  [s] Save  [t/z/f] Edit Paths  [?] Help  [q] Quit",
                Style::default().fg(M3_MUTED),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(M3_SURFACE)),
        );
        f.render_widget(footer, main_chunks[3]);

        // 5. Render Modal Overlays if active
        self.draw_modal(f, area);
    }

    fn draw_modal(&self, f: &mut Frame, area: Rect) {
        match &self.modal {
            TuiModal::None => {}
            TuiModal::Welcome => {
                let popup = centered_rect(68, 55, area);
                f.render_widget(Clear, popup);
                let text = vec![
                    Line::from(Span::styled(
                        "👋 Welcome to Inbox Mover (Rust TUI & GUI)",
                        Style::default()
                            .fg(M3_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("Choose how you'd like to use Inbox Mover:"),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  [1 / Enter] 🌱 Start in Guided Mode (Recommended for First-Time Users)",
                        Style::default()
                            .fg(M3_SUCCESS)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from("              Keeps the Smart Guidance banner visible to explain each folder."),
                    Line::from(""),
                    Line::from(Span::styled(
                        "  [2]         ⚡ Start in Power-User Mode (No Help Banners)",
                        Style::default()
                            .fg(M3_WARNING)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from("              Unlocks Advanced Mode and hides guidance banners until you press 'g' or '?'."),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Press 1, 2, or Esc to continue.",
                        Style::default().fg(M3_MUTED),
                    )),
                ];
                let block = Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title(" Onboarding & Experience Selection ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(M3_PRIMARY))
                            .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                    );
                f.render_widget(block, popup);
            }
            TuiModal::Help { tab, topic } => {
                let popup = centered_rect(84, 80, area);
                f.render_widget(Clear, popup);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(10)])
                    .split(popup);

                let tabs = Tabs::new(vec![
                    " 1. Step-by-Step Guides (↑/↓) ",
                    " 2. Quick Tips & FAQ ",
                    " 3. Keyboard Shortcuts ",
                ])
                .select(*tab)
                .block(
                    Block::default()
                        .title(" ❓ Inbox Mover Help Center [Tab/←/→ Switch Tab | Esc Close] ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(M3_PRIMARY))
                        .style(Style::default().bg(M3_SURFACE_HIGH)),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(M3_PRIMARY)
                        .add_modifier(Modifier::BOLD),
                );
                f.render_widget(tabs, chunks[0]);

                let body_block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(M3_PRIMARY))
                    .style(Style::default().bg(M3_SURFACE).fg(M3_ON_SURFACE));

                if *tab == 0 {
                    let sec = &HELP_SECTIONS[*topic];
                    let mut lines = vec![
                        Line::from(Span::styled(
                            format!("Topic {}/{}: {} (Use ↑/↓ to switch topic)", topic + 1, HELP_SECTIONS.len(), sec.title),
                            Style::default().fg(M3_PRIMARY).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(sec.subtitle, Style::default().fg(M3_MUTED))),
                        Line::from(""),
                    ];
                    for l in sec.content.lines() {
                        lines.push(Line::from(l.to_string()));
                    }
                    f.render_widget(
                        Paragraph::new(lines)
                            .wrap(Wrap { trim: false })
                            .block(body_block),
                        chunks[1],
                    );
                } else if *tab == 1 {
                    let mut lines = Vec::new();
                    for (t, b) in QUICK_TIPS {
                        lines.push(Line::from(Span::styled(
                            format!("★ {t}"),
                            Style::default().fg(M3_PRIMARY).add_modifier(Modifier::BOLD),
                        )));
                        for l in b.lines() {
                            lines.push(Line::from(format!("  {l}")));
                        }
                        lines.push(Line::from(""));
                    }
                    f.render_widget(
                        Paragraph::new(lines)
                            .wrap(Wrap { trim: false })
                            .block(body_block),
                        chunks[1],
                    );
                } else {
                    let mut lines = vec![Line::from(Span::styled(
                        format!(
                            "{:<22} {:<20} {:<20} {}",
                            "Action", "TUI Key", "GUI Key", "Description"
                        ),
                        Style::default().fg(M3_PRIMARY).add_modifier(Modifier::BOLD),
                    ))];
                    lines.push(Line::from("-".repeat(90)));
                    for s in SHORTCUTS {
                        lines.push(Line::from(format!(
                            "{:<22} {:<20} {:<20} {}",
                            s.action, s.key_tui, s.key_gui, s.description
                        )));
                    }
                    f.render_widget(Paragraph::new(lines).block(body_block), chunks[1]);
                }
            }
            TuiModal::EditInput {
                title, buffer, ..
            } => {
                let popup = centered_rect(65, 22, area);
                f.render_widget(Clear, popup);
                let text = vec![
                    Line::from(Span::styled(
                        "Type or paste path below, then press Enter to confirm (Esc to cancel):",
                        Style::default().fg(M3_MUTED),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("> {buffer}_"),
                        Style::default()
                            .fg(M3_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    )),
                ];
                let block = Paragraph::new(text).block(
                    Block::default()
                        .title(format!(" ✏ {title} "))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(M3_PRIMARY))
                        .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                );
                f.render_widget(block, popup);
            }
            TuiModal::SaveRule { mode, input } => {
                let popup = centered_rect(65, 35, area);
                f.render_widget(Clear, popup);
                let modes = [
                    "Save to Config ID (<permitId>.json)",
                    "Save as Wildcard Pattern (patterns.json)",
                    "Save as Workspace DEFAULT Rule",
                ];
                let mut lines = vec![
                    Line::from("Use [Tab / ↑ / ↓] to choose rule type, edit name/pattern, and press [Enter]:"),
                    Line::from(""),
                ];
                for (i, m) in modes.iter().enumerate() {
                    let prefix = if *mode == i { " (●) " } else { " ( ) " };
                    let col = if *mode == i { M3_PRIMARY } else { M3_MUTED };
                    lines.push(Line::from(Span::styled(
                        format!("{prefix}{m}"),
                        Style::default().fg(col),
                    )));
                }
                lines.push(Line::from(""));
                if *mode < 2 {
                    lines.push(Line::from(Span::styled(
                        format!("ID / Pattern: > {input}_"),
                        Style::default()
                            .fg(M3_SUCCESS)
                            .add_modifier(Modifier::BOLD),
                    )));
                }
                let block = Paragraph::new(lines).block(
                    Block::default()
                        .title(" 💾 Save Processing Rule ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(M3_PRIMARY))
                        .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                );
                f.render_widget(block, popup);
            }
            TuiModal::ManageRules { tab, selected } => {
                let popup = centered_rect(76, 70, area);
                f.render_widget(Clear, popup);
                let mut lines = vec![
                    Line::from(Span::styled(
                        "[Tab] Switch Config IDs / Patterns   [↑/↓] Select   [d/Del] Delete Rule   [Esc] Close",
                        Style::default().fg(M3_PRIMARY),
                    )),
                    Line::from(""),
                ];
                if *tab == 0 {
                    lines.push(Line::from(Span::styled(
                        "Saved Config IDs:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    let mut ids: Vec<String> = self.core.config_cache.keys().cloned().collect();
                    ids.sort();
                    for (i, id) in ids.iter().enumerate() {
                        let r = &self.core.config_cache[id];
                        let mark = if *selected == i { "▶ " } else { "  " };
                        lines.push(Line::from(format!(
                            "{mark}{id:<18} -> Target: {}",
                            r.target_folder
                        )));
                    }
                } else {
                    lines.push(Line::from(Span::styled(
                        "Saved Wildcard Patterns:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    let mut pats: Vec<String> = self.core.patterns_cache.keys().cloned().collect();
                    pats.sort();
                    for (i, pat) in pats.iter().enumerate() {
                        let r = &self.core.patterns_cache[pat];
                        let mark = if *selected == i { "▶ " } else { "  " };
                        lines.push(Line::from(format!(
                            "{mark}{pat:<18} -> Target: {}",
                            r.target_folder
                        )));
                    }
                }
                let block = Paragraph::new(lines).block(
                    Block::default()
                        .title(" ⚙ Manage Workspace Rules ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(M3_PRIMARY))
                        .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                );
                f.render_widget(block, popup);
            }
            TuiModal::Logs { tab, scroll } => {
                let popup = centered_rect(84, 78, area);
                f.render_widget(Clear, popup);
                let text = if *tab == 0 {
                    self.local_log_text.clone()
                } else {
                    self.global_logs
                        .iter()
                        .map(|e| {
                            format!(
                                "[{}] {} | {} | ID: {} | User: {} | Files: {} | {}",
                                e.status,
                                e.timestamp,
                                e.folder_name,
                                e.config_id,
                                e.user,
                                e.files_processed,
                                e.message
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let title = if *tab == 0 {
                    " 📜 Local Folder Process.log [Tab: Switch to Global History | ↑/↓ Scroll | Esc Close] "
                } else {
                    " 🌍 Global Audit History (process_log.jsonl) [Tab: Switch to Local | ↑/↓ Scroll | Esc Close] "
                };
                let block = Paragraph::new(text)
                    .scroll((*scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title(title)
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(M3_PRIMARY))
                            .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                    );
                f.render_widget(block, popup);
            }
            TuiModal::DeleteConfirm => {
                let popup = centered_rect(55, 22, area);
                f.render_widget(Clear, popup);
                let text = vec![
                    Line::from(Span::styled(
                        format!(
                            "Permanently delete {} selected transfer folder(s) from disk?",
                            self.selected_indices.len()
                        ),
                        Style::default().fg(M3_ERROR).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from("Press [Y / Enter] to confirm deletion, or [N / Esc] to cancel."),
                ];
                let block = Paragraph::new(text)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .title(" ⚠ Confirm Deletion ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(M3_ERROR))
                            .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                    );
                f.render_widget(block, popup);
            }
            TuiModal::PasswordPrompt { zip_name, buffer } => {
                let popup = centered_rect(55, 22, area);
                f.render_widget(Clear, popup);
                let masked: String = "*".repeat(buffer.len());
                let text = vec![
                    Line::from(format!("Enter password for encrypted ZIP '{zip_name}':")),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("> {masked}_"),
                        Style::default().fg(M3_WARNING).add_modifier(Modifier::BOLD),
                    )),
                ];
                let block = Paragraph::new(text).block(
                    Block::default()
                        .title(" 🔒 Encrypted Archive Password ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(M3_WARNING))
                        .style(Style::default().bg(M3_SURFACE_HIGH).fg(M3_ON_SURFACE)),
                );
                f.render_widget(block, popup);
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Runs the interactive Terminal User Interface (TUI).
pub fn run_tui(core: InboxMoverCore) -> Result<(), String> {
    enable_raw_mode().map_err(|e| format!("Failed to enable terminal raw mode: {e}"))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| format!("Failed to enter alternate screen: {e}"))?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to initialize terminal: {e}"))?;

    let mut app = TuiApp::new(core);

    let res = (|| -> io::Result<()> {
        while !app.should_quit {
            terminal.draw(|f| app.draw(f))?;
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    app.handle_key(key);
                }
            }
        }
        Ok(())
    })();

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    res.map_err(|e| format!("TUI runtime error: {e}"))
}

