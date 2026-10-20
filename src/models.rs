use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Conflict resolution strategy when an extracted or copied file already exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    #[default]
    Overwrite,
    KeepBoth,
    RenameExisting,
}

impl ConflictAction {
    pub const ALL: [ConflictAction; 3] = [
        ConflictAction::Overwrite,
        ConflictAction::KeepBoth,
        ConflictAction::RenameExisting,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            ConflictAction::Overwrite => "overwrite",
            ConflictAction::KeepBoth => "keep_both",
            ConflictAction::RenameExisting => "rename_existing",
        }
    }

    pub fn label(&self) -> &'static str {
        self.display_label()
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            ConflictAction::Overwrite => "Overwrite existing file",
            ConflictAction::KeepBoth => "Keep both (add number)",
            ConflictAction::RenameExisting => "Rename existing with timestamp",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "keep_both" | "keep both" | "keep both (add number)" => ConflictAction::KeepBoth,
            "rename_existing" | "rename existing" | "rename existing with timestamp" => {
                ConflictAction::RenameExisting
            }
            _ => ConflictAction::Overwrite,
        }
    }
}

/// Action to perform on the original transfer folder after processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PostAction {
    #[default]
    Leave,
    Delete,
    Move,
}

impl PostAction {
    pub const ALL: [PostAction; 3] = [PostAction::Move, PostAction::Delete, PostAction::Leave];

    pub fn as_str(&self) -> &'static str {
        match self {
            PostAction::Leave => "leave",
            PostAction::Delete => "delete",
            PostAction::Move => "move",
        }
    }

    pub fn label(&self) -> &'static str {
        self.display_label()
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            PostAction::Leave => "Leave files in place",
            PostAction::Delete => "Delete original folder",
            PostAction::Move => "Move to Processed folder",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "delete" | "delete original folder" => PostAction::Delete,
            "move" | "move to processed folder" => PostAction::Move,
            _ => PostAction::Leave,
        }
    }
}

/// Processing rule configuration saved in `<permitId>.json` or `patterns.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigRule {
    #[serde(default)]
    pub target_folder: String,
    #[serde(default, alias = "process_folder")]
    pub target_zip_folder: String,
    #[serde(default)]
    pub receipt_folder: String,
    #[serde(default)]
    pub conflict_action: ConflictAction,
    #[serde(default)]
    pub post_action: PostAction,
    #[serde(default = "default_true")]
    pub auto_extract: bool,
}

fn default_true() -> bool {
    true
}

impl ConfigRule {
    pub fn same_settings(&self, other: &Self) -> bool {
        self == other
    }
}

impl Default for ConfigRule {
    fn default() -> Self {
        Self {
            target_folder: String::new(),
            target_zip_folder: String::new(),
            receipt_folder: String::new(),
            conflict_action: ConflictAction::Overwrite,
            post_action: PostAction::Leave,
            auto_extract: true,
        }
    }
}

/// Deserialized content of a `receipt*.json` file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReceiptData {
    #[serde(default, rename = "permitId", alias = "permit_id")]
    pub permit_id: Option<String>,
    #[serde(default)]
    pub auto_extract: Option<bool>,
    #[serde(default)]
    pub target_folder: Option<String>,
    #[serde(default, alias = "target_zip_folder")]
    pub process_folder: Option<String>,
    #[serde(default)]
    pub receipt_folder: Option<String>,
    #[serde(default)]
    pub conflict_resolution: Option<String>,
    #[serde(default)]
    pub post_processing: Option<String>,
    #[serde(default)]
    pub processing_logs: Option<Vec<serde_json::Value>>,
}

impl ReceiptData {
    pub fn has_processing_settings(&self) -> bool {
        self.target_folder.is_some()
            || self.process_folder.is_some()
            || self.receipt_folder.is_some()
            || self.conflict_resolution.is_some()
            || self.post_processing.is_some()
            || self.auto_extract.is_some()
    }
}

/// Local application settings stored in `permit_configs/app_settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub dark_mode: bool,
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    #[serde(default = "default_geometry")]
    pub window_geometry: String,
    #[serde(default = "default_sf1")]
    pub search_folder_1: String,
    #[serde(default = "default_sf2")]
    pub search_folder_2: String,
    #[serde(default)]
    pub use_global: bool,
    #[serde(default)]
    pub global_dir: String,
    #[serde(default)]
    pub overrule_receipt: bool,
    #[serde(default = "default_true")]
    pub simple_mode: bool,
    #[serde(default = "default_true")]
    pub show_welcome_splash: bool,
    #[serde(default = "default_true")]
    pub show_guidance: bool,
}

fn default_font_size() -> u32 {
    11
}

fn default_geometry() -> String {
    "1200x850".to_string()
}

fn default_sf1() -> String {
    if cfg!(target_os = "windows") {
        "i:\\".to_string()
    } else {
        "/mnt/inbox/".to_string()
    }
}

fn default_sf2() -> String {
    if cfg!(target_os = "windows") {
        "z:\\inbox".to_string()
    } else {
        "/mnt/data/".to_string()
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            dark_mode: true,
            font_size: default_font_size(),
            window_geometry: default_geometry(),
            search_folder_1: default_sf1(),
            search_folder_2: default_sf2(),
            use_global: false,
            global_dir: String::new(),
            overrule_receipt: false,
            simple_mode: true,
            show_welcome_splash: true,
            show_guidance: true,
        }
    }
}

/// Information gathered when inspecting a `transfer-*` folder.
#[derive(Debug, Clone, Default)]
pub struct FolderData {
    pub folder_path: PathBuf,
    pub folder_name: String,
    pub zip_path: Option<PathBuf>,
    pub permit_id: String,
    pub receipt: Option<ReceiptData>,
    pub receipt_raw: String,
    pub has_valid_zip: bool,
    pub can_process: bool,
    pub has_log: bool,
    pub latest_log_line: Option<String>,
    pub file_list: Vec<String>,
    pub zip_contents: Vec<String>,
    pub is_encrypted_zip: bool,
    pub receipt_warning: Option<String>,
    pub receipt_info: Option<String>,
    pub created_timestamp: u64,
    pub created_date_str: String,
    pub matched_pattern: Option<String>,
}

/// Audit log action entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    #[serde(rename = "type")]
    pub action_type: String,
    pub source: String,
    #[serde(default)]
    pub destination: String,
    #[serde(default)]
    pub message: String,
}

/// Audit log line appended to `process_log.jsonl`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessLogEntry {
    pub timestamp: String,
    pub user: String,
    pub status: String,
    pub folder_name: String,
    pub config_id: String,
    pub files_processed: usize,
    pub config_applied: ConfigRule,
    pub actions: Vec<ActionLogEntry>,
    pub message: String,
}

