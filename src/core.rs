use crate::models::*;
use crate::path_translator::PathTranslator;
use chrono::Local;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::ZipArchive;

#[derive(Clone)]
pub struct InboxMoverCore {
    pub local_config_dir: PathBuf,
    pub config_dir: PathBuf,
    pub log_file: PathBuf,
    pub app_settings: AppSettings,
    pub config_cache: HashMap<String, ConfigRule>,
    pub patterns_cache: HashMap<String, ConfigRule>,
}

impl InboxMoverCore {
    pub fn new() -> Self {
        Self::with_base_dir(Path::new("."))
    }

    pub fn with_base_dir(base_dir: &Path) -> Self {
        let local_config_dir = if base_dir == Path::new(".") {
            PathBuf::from("permit_configs")
        } else {
            base_dir.join("permit_configs")
        };
        if !local_config_dir.exists() {
            let _ = fs::create_dir_all(&local_config_dir);
        }

        let mut core = Self {
            local_config_dir: local_config_dir.clone(),
            config_dir: local_config_dir.clone(),
            log_file: local_config_dir.join("process_log.jsonl"),
            app_settings: AppSettings::default(),
            config_cache: HashMap::new(),
            patterns_cache: HashMap::new(),
        };

        core.app_settings = core.load_app_settings();
        core.set_workspace();
        core
    }

    pub fn save_settings(&mut self) {
        let s = self.app_settings.clone();
        let _ = self.save_app_settings(s);
    }

    pub fn switch_workspace(&mut self, ws: &str) {
        self.app_settings.use_global = ws.eq_ignore_ascii_case("team");
        if self.app_settings.use_global && self.app_settings.global_dir.trim().is_empty() {
            self.app_settings.global_dir = self
                .local_config_dir
                .join("team")
                .to_string_lossy()
                .to_string();
        }
        self.save_settings();
        self.set_workspace();
    }

    pub fn scan_search_directories(&self, dir1: &str, dir2: &str) -> Vec<FolderData> {
        let mut dirs = Vec::new();
        if !dir1.trim().is_empty() {
            dirs.push(dir1.trim().to_string());
        }
        if !dir2.trim().is_empty() {
            dirs.push(dir2.trim().to_string());
        }
        self.find_transfer_folders(&dirs)
    }

    pub fn get_saved_rule(&self, folder: &FolderData) -> Option<(ConfigRule, String)> {
        if folder.permit_id != "DEFAULT" && folder.permit_id != "No_ID" {
            if let Some(cfg) = self.load_config(&folder.permit_id) {
                return Some((cfg, format!("Config ID: {}", folder.permit_id)));
            }
        }

        let mut all_filenames = folder.file_list.clone();
        all_filenames.extend(folder.zip_contents.clone());
        all_filenames.push(folder.folder_name.clone());

        for (pattern, cfg) in &self.patterns_cache {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if all_filenames.iter().any(|f| {
                    let base = Path::new(f).file_name().and_then(|n| n.to_str()).unwrap_or(f);
                    glob_pattern.matches(f) || glob_pattern.matches(base)
                }) {
                    return Some((cfg.clone(), format!("Pattern: {pattern}")));
                }
            }
        }

        if let Some(d_cfg) = self.load_config("DEFAULT") {
            return Some((d_cfg, "DEFAULT".to_string()));
        }

        None
    }

    pub fn resolve_rule_for_folder(
        &self,
        folder: Option<&FolderData>,
    ) -> (ConfigRule, String, bool) {
        let Some(f) = folder else {
            let def = self.load_config("DEFAULT").unwrap_or_default();
            return (def, "DEFAULT".to_string(), false);
        };

        let (rule, _warn, _info, is_mod) = self.resolve_effective_config(f);
        let src = if let Some((_, saved_src)) = self.get_saved_rule(f) {
            if is_mod && !self.app_settings.overrule_receipt {
                format!("{saved_src} + Receipt Override")
            } else {
                saved_src
            }
        } else if f.receipt.is_some() {
            "Receipt JSON".to_string()
        } else {
            "Unconfigured".to_string()
        };

        (rule, src, is_mod)
    }

    pub fn get_folder_status_indicator(&self, folder: &FolderData) -> &'static str {
        if !folder.can_process {
            return "✗";
        }
        let (rule, _, _, _) = self.resolve_effective_config(folder);
        if rule.target_folder.trim().is_empty() {
            return "✗";
        }
        if rule.post_action == PostAction::Move && rule.target_zip_folder.trim().is_empty() {
            return "✗";
        }
        if folder.has_log {
            "✓ (L)"
        } else {
            "✓"
        }
    }

    pub fn read_global_logs(&self, limit: usize) -> Vec<ProcessLogEntry> {
        if !self.log_file.exists() {
            return Vec::new();
        }
        let Ok(content) = fs::read_to_string(&self.log_file) else {
            return Vec::new();
        };
        let mut entries: Vec<ProcessLogEntry> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<ProcessLogEntry>(l).ok())
            .collect();
        entries.reverse();
        entries.truncate(limit);
        entries
    }

    pub fn set_workspace(&mut self) {
        if self.app_settings.use_global && !self.app_settings.global_dir.is_empty() {
            self.config_dir = PathBuf::from(&self.app_settings.global_dir);
        } else {
            self.config_dir = self.local_config_dir.clone();
        }

        self.ensure_config_dir();
        self.log_file = self.config_dir.join("process_log.jsonl");
        self.reload_cache();
    }

    pub fn ensure_config_dir(&self) {
        if !self.config_dir.exists() {
            let _ = fs::create_dir_all(&self.config_dir);
        }
        let patterns_file = self.config_dir.join("patterns.json");
        if !patterns_file.exists() {
            if let Ok(mut f) = File::create(patterns_file) {
                let _ = f.write_all(b"{}");
            }
        }
    }

    pub fn reload_cache(&mut self) {
        self.config_cache.clear();
        self.patterns_cache.clear();

        if !self.config_dir.exists() {
            return;
        }

        // Load patterns.json
        let patterns_file = self.config_dir.join("patterns.json");
        if patterns_file.exists() {
            if let Ok(content) = fs::read_to_string(&patterns_file) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, ConfigRule>>(&content) {
                    self.patterns_cache = map;
                }
            }
        }

        // Load all <permitId>.json files
        if let Ok(entries) = fs::read_dir(&self.config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.ends_with(".json")
                            && file_name != "app_settings.json"
                            && file_name != "patterns.json"
                        {
                            let permit_id = &file_name[..file_name.len() - 5];
                            if let Ok(content) = fs::read_to_string(&path) {
                                if let Ok(rule) = serde_json::from_str::<ConfigRule>(&content) {
                                    self.config_cache.insert(permit_id.to_string(), rule);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn load_app_settings(&self) -> AppSettings {
        let settings_path = self.local_config_dir.join("app_settings.json");
        let mut settings = AppSettings::default();

        if settings_path.exists() {
            if let Ok(content) = fs::read_to_string(&settings_path) {
                if let Ok(loaded) = serde_json::from_str::<AppSettings>(&content) {
                    settings = loaded;
                }
            }
        }

        settings.search_folder_1 = PathTranslator::translate(&settings.search_folder_1);
        settings.search_folder_2 = PathTranslator::translate(&settings.search_folder_2);
        settings
    }

    pub fn save_app_settings(&mut self, settings: AppSettings) -> std::io::Result<()> {
        let settings_path = self.local_config_dir.join("app_settings.json");
        self.app_settings = settings;
        let json = serde_json::to_string_pretty(&self.app_settings)?;
        fs::write(settings_path, json)?;
        Ok(())
    }

    pub fn load_config(&self, permit_id: &str) -> Option<ConfigRule> {
        self.config_cache.get(permit_id).cloned()
    }

    pub fn get_all_configs(&self) -> HashMap<String, ConfigRule> {
        self.config_cache.clone()
    }

    pub fn save_config(&mut self, permit_id: &str, rule: ConfigRule) -> std::io::Result<()> {
        if permit_id.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot save rule with empty Config ID.",
            ));
        }
        self.ensure_config_dir();
        let path = self.config_dir.join(format!("{permit_id}.json"));
        let json = serde_json::to_string_pretty(&rule)?;
        fs::write(path, json)?;
        self.config_cache.insert(permit_id.to_string(), rule);
        Ok(())
    }

    pub fn delete_config(&mut self, permit_id: &str) -> std::io::Result<()> {
        let path = self.config_dir.join(format!("{permit_id}.json"));
        if path.exists() {
            fs::remove_file(path)?;
        }
        self.config_cache.remove(permit_id);
        Ok(())
    }

    pub fn load_patterns(&self) -> HashMap<String, ConfigRule> {
        self.patterns_cache.clone()
    }

    pub fn save_pattern(&mut self, pattern: &str, rule: ConfigRule) -> std::io::Result<()> {
        self.patterns_cache.insert(pattern.to_string(), rule);
        self.ensure_config_dir();
        let path = self.config_dir.join("patterns.json");
        let json = serde_json::to_string_pretty(&self.patterns_cache)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn delete_pattern(&mut self, pattern: &str) -> std::io::Result<()> {
        if self.patterns_cache.remove(pattern).is_some() {
            let path = self.config_dir.join("patterns.json");
            let json = serde_json::to_string_pretty(&self.patterns_cache)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn merge_local_into_team(&mut self) -> std::io::Result<usize> {
        if !self.app_settings.use_global || self.app_settings.global_dir.is_empty() {
            return Ok(0);
        }

        let mut merged_count = 0;
        let global_path = PathBuf::from(&self.app_settings.global_dir);
        if !global_path.exists() {
            fs::create_dir_all(&global_path)?;
        }

        // Copy configs
        if let Ok(entries) = fs::read_dir(&self.local_config_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(".json") && name != "app_settings.json" && name != "patterns.json" {
                            let dest = global_path.join(name);
                            if !dest.exists() {
                                if let Ok(_) = fs::copy(&path, &dest) {
                                    merged_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Merge patterns
        let local_patterns_path = self.local_config_dir.join("patterns.json");
        let global_patterns_path = global_path.join("patterns.json");
        if local_patterns_path.exists() {
            if let Ok(content) = fs::read_to_string(&local_patterns_path) {
                if let Ok(local_patterns) = serde_json::from_str::<HashMap<String, ConfigRule>>(&content) {
                    let mut team_patterns = HashMap::new();
                    if global_patterns_path.exists() {
                        if let Ok(g_content) = fs::read_to_string(&global_patterns_path) {
                            if let Ok(map) = serde_json::from_str::<HashMap<String, ConfigRule>>(&g_content) {
                                team_patterns = map;
                            }
                        }
                    }
                    for (pat, rule) in local_patterns {
                        if !team_patterns.contains_key(&pat) {
                            team_patterns.insert(pat, rule);
                            merged_count += 1;
                        }
                    }
                    let json = serde_json::to_string_pretty(&team_patterns)?;
                    fs::write(global_patterns_path, json)?;
                }
            }
        }

        self.reload_cache();
        Ok(merged_count)
    }

    /// Finds all `transfer-*` directories across search locations.
    pub fn find_transfer_folders(&self, search_folders: &[String]) -> Vec<FolderData> {
        let mut results = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for sf in search_folders {
            let sf_clean = sf.trim();
            if sf_clean.is_empty() {
                continue;
            }
            let sf_path = PathBuf::from(sf_clean);
            if !sf_path.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&sf_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                            if folder_name.to_lowercase().starts_with("transfer-") {
                                if seen.insert(path.clone()) {
                                    let data = self.inspect_transfer_folder(&path);
                                    results.push(data);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort descending by folder name (e.g. transfer-2026-10-20 before transfer-2026-10-19)
        results.sort_by(|a, b| b.folder_name.cmp(&a.folder_name));
        results
    }

    /// Inspects a single transfer folder for files, logs, loose receipts, and ZIP contents.
    pub fn inspect_transfer_folder(&self, folder_path: &Path) -> FolderData {
        let folder_name = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let (created_timestamp, created_date_str) = fs::metadata(folder_path)
            .and_then(|m| m.modified().or_else(|_| m.created()))
            .ok()
            .and_then(|st| {
                let secs = st
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let dt: chrono::DateTime<Local> = st.into();
                Some((secs, dt.format("%Y-%m-%d %H:%M").to_string()))
            })
            .unwrap_or_else(|| (0, "Unknown date".to_string()));

        let mut data = FolderData {
            folder_path: folder_path.to_path_buf(),
            folder_name,
            zip_path: None,
            permit_id: "DEFAULT".to_string(),
            receipt: None,
            receipt_raw: String::new(),
            has_valid_zip: false,
            can_process: false,
            has_log: false,
            latest_log_line: None,
            file_list: Vec::new(),
            zip_contents: Vec::new(),
            is_encrypted_zip: false,
            receipt_warning: None,
            receipt_info: None,
            created_timestamp,
            created_date_str,
            matched_pattern: None,
        };

        // Check for local process logs
        let log1 = folder_path.join("Process.log");
        let log2 = folder_path.join("Inbox Process.log");
        let active_log = if log1.exists() {
            Some(log1)
        } else if log2.exists() {
            Some(log2)
        } else {
            None
        };

        if let Some(log_path) = active_log {
            data.has_log = true;
            if let Ok(content) = fs::read_to_string(&log_path) {
                if let Some(first_line) = content.lines().next() {
                    let summary = first_line.split(" | Config: ").next().unwrap_or(first_line);
                    data.latest_log_line = Some(summary.to_string());
                }
            }
        }

        let mut valid_zip_found = false;
        let mut loose_receipt_data: Option<ReceiptData> = None;
        let mut loose_receipt_raw = String::new();
        let mut has_loose_receipt = false;

        // Traverse files
        for entry in WalkDir::new(folder_path).into_iter().flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(rel) = p.strip_prefix(folder_path) {
                    let rel_str = rel.to_string_lossy().to_string();
                    data.file_list.push(rel_str.clone());

                    let file_lower = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                    
                    // Loose receipt match: receipt*.json or reciept*.json
                    if is_receipt_filename(&file_lower) {
                        has_loose_receipt = true;
                        if let Ok(content) = fs::read_to_string(p) {
                            if loose_receipt_raw.is_empty() {
                                loose_receipt_raw = content.clone();
                            } else {
                                loose_receipt_raw.push_str(&format!("\n\n--- OTHER RECEIPT: {file_lower} ---\n{content}"));
                            }

                            if loose_receipt_data.is_none() && !content.trim().is_empty() {
                                match serde_json::from_str::<ReceiptData>(&content) {
                                    Ok(parsed) => loose_receipt_data = Some(parsed),
                                    Err(e) => {
                                        data.receipt_warning = Some(format!(
                                            "[WARNING: {file_lower} is not a valid JSON file: {e}]"
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for ZIP archives
        for entry in WalkDir::new(folder_path).into_iter().flatten() {
            let p = entry.path();
            if p.is_file() && !valid_zip_found {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.to_lowercase().ends_with(".zip") {
                        data.zip_path = Some(p.to_path_buf());
                        data.has_valid_zip = true;
                        valid_zip_found = true;

                        if has_loose_receipt {
                            data.receipt = loose_receipt_data.clone();
                            data.receipt_raw = if loose_receipt_raw.trim().is_empty() {
                                "<receipt files are empty>".to_string()
                            } else {
                                loose_receipt_raw.clone()
                            };
                            if let Some(ref r) = loose_receipt_data {
                                if let Some(ref pid) = r.permit_id {
                                    data.permit_id = pid.clone();
                                }
                            }
                        } else {
                            // Inspect inside the zip
                            let (zip_r, zip_raw, zip_files, is_enc) = self.inspect_zip(p);
                            data.zip_contents = zip_files;
                            data.is_encrypted_zip = is_enc;
                            if !zip_raw.is_empty() {
                                data.receipt_raw = zip_raw;
                                if let Some(r) = zip_r {
                                    if let Some(ref pid) = r.permit_id {
                                        data.permit_id = pid.clone();
                                    }
                                    data.receipt = Some(r);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }

        if !valid_zip_found {
            if has_loose_receipt {
                data.receipt = loose_receipt_data.clone();
                data.receipt_raw = if loose_receipt_raw.trim().is_empty() {
                    "<receipt files are empty>".to_string()
                } else {
                    loose_receipt_raw
                };
                if let Some(ref r) = loose_receipt_data {
                    if let Some(ref pid) = r.permit_id {
                        data.permit_id = pid.clone();
                    }
                }
                data.can_process = true;
            } else if data.file_list.is_empty() {
                data.can_process = false;
            } else {
                data.can_process = true;
            }
        } else {
            data.can_process = true;
        }

        data
    }

    /// Inspects a ZIP archive without full extraction to gather contents and receipt JSON.
    pub fn inspect_zip(&self, zip_path: &Path) -> (Option<ReceiptData>, String, Vec<String>, bool) {
        let mut receipt_data = None;
        let mut raw_content = String::new();
        let mut file_names = Vec::new();
        let mut is_encrypted = false;

        let file = match File::open(zip_path) {
            Ok(f) => f,
            Err(_) => return (None, String::new(), Vec::new(), false),
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(_) => return (None, String::new(), Vec::new(), false),
        };

        for i in 0..archive.len() {
            if let Ok(mut zip_file) = archive.by_index(i) {
                let name = zip_file.name().to_string();
                file_names.push(name.clone());

                if zip_file.encrypted() {
                    is_encrypted = true;
                }

                let base_name = Path::new(&name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if is_receipt_filename(&base_name) {
                    let mut buf = String::new();
                    match zip_file.read_to_string(&mut buf) {
                        Ok(_) => {
                            if raw_content.is_empty() {
                                raw_content = buf.clone();
                            } else {
                                raw_content.push_str(&format!("\n\n--- OTHER RECEIPT IN ZIP: {base_name} ---\n{buf}"));
                            }
                            if receipt_data.is_none() {
                                if let Ok(parsed) = serde_json::from_str::<ReceiptData>(&buf) {
                                    receipt_data = Some(parsed);
                                }
                            }
                        }
                        Err(_) => {
                            if zip_file.encrypted() {
                                raw_content.push_str(&format!("\n<{base_name} is password protected>"));
                            }
                        }
                    }
                }
            }
        }

        (receipt_data, raw_content, file_names, is_encrypted)
    }

    /// Resolves the effective configuration for a folder according to rule hierarchy:
    /// Permit ID -> Pattern matching -> DEFAULT -> Receipt settings (unless overruled).
    pub fn resolve_effective_config(
        &self,
        folder: &FolderData,
    ) -> (ConfigRule, Option<String>, Option<String>, bool) {
        let mut rule = ConfigRule::default();
        let mut warning = None;
        let mut info = None;
        let mut is_modified_from_saved = false;

        let default_cfg = self.load_config("DEFAULT");

        // All filenames for pattern match
        let mut all_filenames = folder.file_list.clone();
        all_filenames.extend(folder.zip_contents.clone());

        // Check pattern match
        let mut pattern_match: Option<(String, ConfigRule)> = None;
        for (pattern, cfg) in &self.patterns_cache {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if all_filenames.iter().any(|f| {
                    let base = Path::new(f).file_name().and_then(|n| n.to_str()).unwrap_or(f);
                    glob_pattern.matches(f) || glob_pattern.matches(base)
                }) {
                    pattern_match = Some((pattern.clone(), cfg.clone()));
                    break;
                }
            }
        }

        // Check permit ID config
        let permit_cfg = if folder.permit_id != "DEFAULT" {
            self.load_config(&folder.permit_id)
        } else {
            None
        };

        // Saved workspace rule hierarchy
        let (saved_rule, rule_source_desc) = if let Some(ref p_cfg) = permit_cfg {
            (Some(p_cfg.clone()), format!("Config: {}", folder.permit_id))
        } else if let Some((ref pat, ref p_cfg)) = pattern_match {
            (Some(p_cfg.clone()), format!("Pattern: {pat}"))
        } else if let Some(ref d_cfg) = default_cfg {
            (Some(d_cfg.clone()), "DEFAULT".to_string())
        } else {
            (None, String::new())
        };

        let has_receipt = folder.receipt.is_some();
        let receipt_has_settings = folder
            .receipt
            .as_ref()
            .map_or(false, |r| r.has_processing_settings());

        let is_windows = cfg!(target_os = "windows");
        let mut use_receipt = true;

        if let Some(ref r) = folder.receipt {
            let tf_incompatible = r
                .target_folder
                .as_ref()
                .map_or(false, |p| PathTranslator::is_incompatible_path(p, is_windows));
            let pf_incompatible = r
                .process_folder
                .as_ref()
                .map_or(false, |p| PathTranslator::is_incompatible_path(p, is_windows));

            if tf_incompatible || pf_incompatible {
                if pattern_match.is_some() {
                    use_receipt = false;
                    warning = Some("[WARNING: receipt*.json bypassed because it contains local destinations incompatible with this OS. Stored Pattern Matching rule is being used instead.]".to_string());
                } else {
                    warning = Some("[WARNING: receipt*.json contains local destinations incompatible with this OS. No Pattern Matching fallback found.]".to_string());
                }
            }
        }

        let overrule = self.app_settings.overrule_receipt;
        let ws_name = if self.app_settings.use_global {
            "Team"
        } else {
            "Personal"
        };

        if overrule && saved_rule.is_some() {
            // Overrule mode: saved workspace rule wins!
            if let Some(saved) = saved_rule {
                rule = saved;
                if has_receipt && receipt_has_settings {
                    info = Some(format!(
                        "[INFO: Receipt processing settings overruled in favor of {ws_name} settings ({rule_source_desc})]"
                    ));
                }
            }
        } else {
            // Apply saved defaults first as baseline
            if let Some(ref d) = default_cfg {
                rule = d.clone();
            }
            if let Some(ref p) = permit_cfg {
                rule = p.clone();
            } else if let Some((_, ref p)) = pattern_match {
                rule = p.clone();
            }

            // Receipt settings override baseline
            if use_receipt {
                if let Some(ref r) = folder.receipt {
                    if let Some(ref tf) = r.target_folder {
                        rule.target_folder = PathTranslator::translate(tf);
                        is_modified_from_saved = true;
                    }
                    if let Some(ref pf) = r.process_folder {
                        rule.target_zip_folder = PathTranslator::translate(pf);
                        is_modified_from_saved = true;
                    }
                    if let Some(ref rf) = r.receipt_folder {
                        rule.receipt_folder = PathTranslator::translate(rf);
                        is_modified_from_saved = true;
                    }
                    if let Some(ref cr) = r.conflict_resolution {
                        rule.conflict_action = ConflictAction::from_str(cr);
                        is_modified_from_saved = true;
                    }
                    if let Some(ref pp) = r.post_processing {
                        rule.post_action = PostAction::from_str(pp);
                        is_modified_from_saved = true;
                    }
                    if let Some(ae) = r.auto_extract {
                        rule.auto_extract = ae;
                        is_modified_from_saved = true;
                    }

                    if overrule && receipt_has_settings {
                        info = Some(format!(
                            "[INFO: No matching {ws_name} rule found; fell back to receipt settings]"
                        ));
                    }
                }
            } else if let Some((_, ref p)) = pattern_match {
                rule = p.clone();
            }
        }

        (rule, warning, info, is_modified_from_saved)
    }

    /// Safe file copy that handles restrictive permissions/mounts.
    pub fn safe_copy(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }

    /// Safe rename that falls back to copy + remove when crossing mount boundaries.
    pub fn safe_rename(&self, src: &Path, dst: &Path) -> std::io::Result<()> {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        if fs::rename(src, dst).is_err() {
            if src.is_dir() {
                copy_dir_all(src, dst)?;
                let _ = fs::remove_dir_all(src);
            } else {
                fs::copy(src, dst)?;
                let _ = fs::remove_file(src);
            }
        }
        Ok(())
    }

    /// Resolves conflict on destination file path according to `ConflictAction`.
    pub fn get_final_path(
        &self,
        target_path: &Path,
        conflict: ConflictAction,
        actions_log: &mut Vec<ActionLogEntry>,
    ) -> PathBuf {
        if !target_path.exists() {
            return target_path.to_path_buf();
        }

        match conflict {
            ConflictAction::Overwrite => {
                actions_log.push(ActionLogEntry {
                    action_type: "conflict_resolved".to_string(),
                    source: target_path.to_string_lossy().to_string(),
                    destination: String::new(),
                    message: "Existing file overwritten".to_string(),
                });
                target_path.to_path_buf()
            }
            ConflictAction::KeepBoth => {
                let stem = target_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file");
                let ext = target_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{e}"))
                    .unwrap_or_default();
                let parent = target_path.parent().unwrap_or_else(|| Path::new(""));

                let mut counter = 1;
                let mut candidate = parent.join(format!("{stem} ({counter}){ext}"));
                while candidate.exists() {
                    counter += 1;
                    candidate = parent.join(format!("{stem} ({counter}){ext}"));
                }

                actions_log.push(ActionLogEntry {
                    action_type: "conflict_resolved".to_string(),
                    source: target_path.to_string_lossy().to_string(),
                    destination: candidate.to_string_lossy().to_string(),
                    message: format!(
                        "Kept both. File renamed to {}",
                        candidate.file_name().and_then(|n| n.to_str()).unwrap_or("")
                    ),
                });
                candidate
            }
            ConflictAction::RenameExisting => {
                let timestamp = Local::now().format("%y%m%d-%H%M%S").to_string();
                let filename = target_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file");
                let parent = target_path.parent().unwrap_or_else(|| Path::new(""));

                let mut renamed_path = parent.join(format!("{timestamp}_{filename}"));
                let mut counter = 1;
                while renamed_path.exists() {
                    renamed_path = parent.join(format!("{timestamp}_{filename}_{counter}"));
                    counter += 1;
                }

                let _ = self.safe_rename(target_path, &renamed_path);
                actions_log.push(ActionLogEntry {
                    action_type: "conflict_resolved".to_string(),
                    source: target_path.to_string_lossy().to_string(),
                    destination: renamed_path.to_string_lossy().to_string(),
                    message: format!(
                        "Existing file renamed to {}",
                        renamed_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                    ),
                });
                target_path.to_path_buf()
            }
        }
    }

    /// Processes a single transfer folder: unzips archives, copies loose files, handles receipts,
    /// applies conflict resolution, executes post action, and writes audit logs.
    pub fn process_folder<F, P>(
        &self,
        folder: &FolderData,
        config: &ConfigRule,
        mut progress_cb: Option<F>,
        mut password_cb: Option<P>,
    ) -> Result<(), String>
    where
        F: FnMut(usize, usize, &str),
        P: FnMut(&str) -> Option<String>,
    {
        let target_folder_str = config.target_folder.trim();
        if target_folder_str.is_empty() {
            let err = "Target folder is not specified.".to_string();
            self.write_log("ERROR", folder, config, &[], &err);
            return Err(err);
        }

        let target_dir = PathBuf::from(target_folder_str);
        if let Err(e) = fs::create_dir_all(&target_dir) {
            let err = format!("Failed to create Target folder '{target_folder_str}': {e}");
            self.write_log("ERROR", folder, config, &[], &err);
            return Err(err);
        }

        let receipt_dir = if !config.receipt_folder.trim().is_empty() {
            let rd = PathBuf::from(config.receipt_folder.trim());
            if let Err(e) = fs::create_dir_all(&rd) {
                let err = format!("Failed to create Receipt folder '{}': {e}", config.receipt_folder);
                self.write_log("ERROR", folder, config, &[], &err);
                return Err(err);
            }
            Some(rd)
        } else {
            None
        };

        let target_zip_dir = if config.post_action == PostAction::Move {
            if config.target_zip_folder.trim().is_empty() {
                let err = "Processed Folder is not specified but post action is 'move'.".to_string();
                self.write_log("ERROR", folder, config, &[], &err);
                return Err(err);
            }
            let zd = PathBuf::from(config.target_zip_folder.trim());
            if let Err(e) = fs::create_dir_all(&zd) {
                let err = format!("Failed to create Processed folder '{}': {e}", config.target_zip_folder);
                self.write_log("ERROR", folder, config, &[], &err);
                return Err(err);
            }
            Some(zd)
        } else {
            None
        };

        let mut actions_log: Vec<ActionLogEntry> = Vec::new();

        // 1. Process files in the folder
        let folder_path = &folder.folder_path;
        let mut files_to_process = Vec::new();

        for entry in WalkDir::new(folder_path).into_iter().flatten() {
            let p = entry.path();
            if p.is_file() {
                let file_name_low = p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if file_name_low == "process.log" || file_name_low == "inbox process.log" {
                    continue;
                }
                files_to_process.push(p.to_path_buf());
            }
        }

        let total_items = files_to_process.len();
        for (idx, file_path) in files_to_process.iter().enumerate() {
            let file_name_low = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            if let Some(ref mut cb) = progress_cb {
                cb(idx + 1, total_items, &file_name_low);
            }

            if config.auto_extract && file_name_low.ends_with(".zip") {
                // Extract ZIP file
                if let Err(e) = self.extract_zip(
                    file_path,
                    &target_dir,
                    receipt_dir.as_deref(),
                    config.conflict_action,
                    &mut actions_log,
                    &mut progress_cb,
                    &mut password_cb,
                ) {
                    self.write_log("ERROR", folder, config, &actions_log, &e);
                    return Err(e);
                }
            } else {
                // Copy loose file
                let rel_path = file_path
                    .strip_prefix(folder_path)
                    .unwrap_or(file_path);

                let is_receipt = is_receipt_filename(&file_name_low);
                let final_dest = if is_receipt {
                    let timestamp = Local::now().format("%y%m%d-%H%M%S").to_string();
                    let new_name = format!("{timestamp}-{}", file_path.file_name().and_then(|n| n.to_str()).unwrap_or("receipt.json"));
                    if let Some(ref rd) = receipt_dir {
                        rd.join(new_name)
                    } else {
                        target_dir.join(rel_path.parent().unwrap_or(Path::new(""))).join(new_name)
                    }
                } else {
                    target_dir.join(rel_path)
                };

                let resolved_path = self.get_final_path(&final_dest, config.conflict_action, &mut actions_log);
                if let Err(e) = self.safe_copy(file_path, &resolved_path) {
                    let err = format!("Failed to copy file {:?}: {e}", file_path);
                    self.write_log("ERROR", folder, config, &actions_log, &err);
                    return Err(err);
                }

                actions_log.push(ActionLogEntry {
                    action_type: "copy".to_string(),
                    source: format!("{} -> {}", folder.folder_name, rel_path.to_string_lossy()),
                    destination: resolved_path.to_string_lossy().to_string(),
                    message: String::new(),
                });
            }
        }

        // 2. Post processing action
        match config.post_action {
            PostAction::Leave => {}
            PostAction::Delete => {
                if let Err(e) = fs::remove_dir_all(folder_path) {
                    let err = format!("Failed to delete folder {:?}: {e}", folder_path);
                    self.write_log("ERROR", folder, config, &actions_log, &err);
                    return Err(err);
                }
                actions_log.push(ActionLogEntry {
                    action_type: "post_processing".to_string(),
                    source: folder_path.to_string_lossy().to_string(),
                    destination: "DELETED".to_string(),
                    message: String::new(),
                });
            }
            PostAction::Move => {
                if let Some(zd) = target_zip_dir {
                    let mut dest = zd.join(&folder.folder_name);
                    let mut counter = 1;
                    while dest.exists() {
                        dest = zd.join(format!("{}_{counter}", folder.folder_name));
                        counter += 1;
                    }

                    if let Err(e) = self.safe_rename(folder_path, &dest) {
                        let err = format!("Failed to move folder {:?} to {:?}: {e}", folder_path, dest);
                        self.write_log("ERROR", folder, config, &actions_log, &err);
                        return Err(err);
                    }

                    actions_log.push(ActionLogEntry {
                        action_type: "post_processing".to_string(),
                        source: folder_path.to_string_lossy().to_string(),
                        destination: dest.to_string_lossy().to_string(),
                        message: String::new(),
                    });
                }
            }
        }

        self.write_log("SUCCESS", folder, config, &actions_log, "Successfully processed folder.");
        Ok(())
    }

    fn extract_zip<F, P>(
        &self,
        zip_path: &Path,
        target_dir: &Path,
        receipt_dir: Option<&Path>,
        conflict: ConflictAction,
        actions_log: &mut Vec<ActionLogEntry>,
        progress_cb: &mut Option<F>,
        password_cb: &mut Option<P>,
    ) -> Result<(), String>
    where
        F: FnMut(usize, usize, &str),
        P: FnMut(&str) -> Option<String>,
    {
        let zip_file_name = zip_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive.zip")
            .to_string();

        let file = File::open(zip_path).map_err(|e| format!("Could not open ZIP {:?}: {e}", zip_path))?;
        let mut archive = ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive {:?}: {e}", zip_path))?;

        let total_entries = archive.len();
        let mut password: Option<String> = None;

        for i in 0..total_entries {
            let need_password_prompt = if let Some(ref pw) = password {
                matches!(
                    archive.by_index_decrypt(i, pw.as_bytes()),
                    Err(zip::result::ZipError::UnsupportedArchive(zip::result::ZipError::PASSWORD_REQUIRED))
                        | Err(zip::result::ZipError::InvalidPassword)
                )
            } else {
                matches!(
                    archive.by_index(i),
                    Err(zip::result::ZipError::UnsupportedArchive(zip::result::ZipError::PASSWORD_REQUIRED))
                        | Err(zip::result::ZipError::InvalidPassword)
                )
            };

            if need_password_prompt {
                if let Some(ref mut pw_fn) = password_cb {
                    if let Some(pw) = pw_fn(&zip_file_name) {
                        password = Some(pw);
                    } else {
                        return Err(format!("Password prompt cancelled for {zip_file_name}."));
                    }
                } else {
                    return Err(format!(
                        "Password required for {zip_file_name} but no password prompt handler available."
                    ));
                }
            }

            let mut zip_entry = if let Some(ref pw) = password {
                archive
                    .by_index_decrypt(i, pw.as_bytes())
                    .map_err(|e| format!("Failed to decrypt entry with provided password: {e}"))?
            } else {
                archive
                    .by_index(i)
                    .map_err(|e| format!("Failed reading ZIP entry {i}: {e}"))?
            };

            if zip_entry.is_dir() {
                continue;
            }

            let entry_name = zip_entry.name().to_string();
            if let Some(ref mut cb) = progress_cb {
                cb(i + 1, total_entries, &entry_name);
            }

            let safe_name = entry_name.trim_start_matches(|c| c == '/' || c == '\\');
            let parts: Vec<&str> = safe_name.split(|c| c == '/' || c == '\\').collect();

            // Strip redundant transfer-* root if zipped with folder root
            let relative_name = if parts.len() > 1 && parts[0].to_lowercase().starts_with("transfer-") {
                parts[1..].join("/")
            } else {
                safe_name.to_string()
            };

            let is_absolute = entry_name.starts_with('/')
                || entry_name.starts_with('\\')
                || (entry_name.len() >= 3
                    && entry_name.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                    && entry_name.chars().nth(1) == Some(':')
                    && (entry_name.chars().nth(2) == Some('/') || entry_name.chars().nth(2) == Some('\\')));

            let base_entry_name = Path::new(&entry_name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            let target_file_path = if is_receipt_filename(&base_entry_name) {
                let timestamp = Local::now().format("%y%m%d-%H%M%S").to_string();
                let new_name = format!("{timestamp}-{base_entry_name}");
                if let Some(rd) = receipt_dir {
                    rd.join(new_name)
                } else if is_absolute {
                    Path::new(&entry_name)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(new_name)
                } else {
                    target_dir
                        .join(Path::new(&relative_name).parent().unwrap_or(Path::new("")))
                        .join(new_name)
                }
            } else if is_absolute {
                PathBuf::from(&entry_name)
            } else {
                target_dir.join(&relative_name)
            };

            let resolved_path = self.get_final_path(&target_file_path, conflict, actions_log);
            if let Some(parent) = resolved_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let mut out_file = File::create(&resolved_path)
                .map_err(|e| format!("Failed to create destination file {:?}: {e}", resolved_path))?;
            std::io::copy(&mut zip_entry, &mut out_file)
                .map_err(|e| format!("Failed to extract ZIP entry to {:?}: {e}", resolved_path))?;

            actions_log.push(ActionLogEntry {
                action_type: "extract".to_string(),
                source: format!("{zip_file_name} -> {entry_name}"),
                destination: resolved_path.to_string_lossy().to_string(),
                message: String::new(),
            });
        }

        Ok(())
    }

    /// Writes audit logs to `process_log.jsonl`, folder `Process.log`, and receipt JSON `processing_logs`.
    pub fn write_log(
        &self,
        status: &str,
        folder: &FolderData,
        config: &ConfigRule,
        actions: &[ActionLogEntry],
        message: &str,
    ) {
        let current_user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        let timestamp_iso = Local::now().to_rfc3339();

        let log_entry = ProcessLogEntry {
            timestamp: timestamp_iso.clone(),
            user: current_user.clone(),
            status: status.to_string(),
            folder_name: folder.folder_name.clone(),
            config_id: folder.permit_id.clone(),
            files_processed: actions
                .iter()
                .filter(|a| a.action_type == "extract" || a.action_type == "copy")
                .count(),
            config_applied: config.clone(),
            actions: actions.to_vec(),
            message: message.to_string(),
        };

        // 1. Append to global audit log (process_log.jsonl)
        if let Ok(json_line) = serde_json::to_string(&log_entry) {
            if let Ok(mut f) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_file)
            {
                let _ = writeln!(f, "{json_line}");
            }
        }

        // 2. Write to local folder Process.log
        let mut target_local_dir = None;
        if config.post_action == PostAction::Leave {
            target_local_dir = Some(folder.folder_path.clone());
        } else if config.post_action == PostAction::Move {
            for act in actions {
                if act.action_type == "post_processing" && !act.destination.is_empty() && act.destination != "DELETED" {
                    target_local_dir = Some(PathBuf::from(&act.destination));
                    break;
                }
            }
            if target_local_dir.is_none() {
                target_local_dir = Some(folder.folder_path.clone());
            }
        } else if config.post_action == PostAction::Delete && status != "SUCCESS" {
            target_local_dir = Some(folder.folder_path.clone());
        }

        if let Some(dir) = target_local_dir {
            if dir.is_dir() {
                let local_log_path = dir.join("Process.log");
                let ts_formatted = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

                let mut new_lines = vec![format!(
                    "[{ts_formatted}] | User: {current_user} | Config: {} | Folder: {}",
                    folder.permit_id, folder.folder_name
                )];
                if !message.is_empty() {
                    new_lines.push(format!("  Message: {message}"));
                }
                if !actions.is_empty() {
                    new_lines.push("  Actions:".to_string());
                    for act in actions {
                        let a_type = act.action_type.to_uppercase();
                        if a_type == "CONFLICT_RESOLVED" {
                            new_lines.push(format!("    - CONFLICT: {} -> {}", act.source, act.message));
                        } else {
                            new_lines.push(format!("    - {a_type}: {} -> {}", act.source, act.destination));
                        }
                    }
                }
                new_lines.push("-".repeat(80));
                let new_log_text = format!("{}\n\n", new_lines.join("\n"));

                let existing = fs::read_to_string(&local_log_path).unwrap_or_default();
                let _ = fs::write(&local_log_path, format!("{new_log_text}{existing}"));
            }
        }

        // 3. Inject into destination receipt*.json files
        for act in actions {
            let dest = &act.destination;
            if dest.ends_with(".json") {
                let base = Path::new(dest)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if is_receipt_filename(&base) {
                    let dest_path = PathBuf::from(dest);
                    if dest_path.exists() {
                        if let Ok(content) = fs::read_to_string(&dest_path) {
                            if let Ok(mut json_val) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(map) = json_val.as_object_mut() {
                                    let logs_entry = map.entry("processing_logs").or_insert_with(|| serde_json::json!([]));
                                    if let Some(arr) = logs_entry.as_array_mut() {
                                        if let Ok(val) = serde_json::to_value(&log_entry) {
                                            arr.push(val);
                                        }
                                    }
                                    let _ = fs::write(&dest_path, serde_json::to_string_pretty(&json_val).unwrap_or_default());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Creates a customized `receipt.json` file on disk.
    pub fn create_receipt_file(&self, path: &Path, config: &ConfigRule, permit_id: Option<&str>) -> std::io::Result<()> {
        let receipt = ReceiptData {
            permit_id: permit_id.map(|s| s.to_string()),
            auto_extract: Some(config.auto_extract),
            target_folder: Some(config.target_folder.clone()),
            process_folder: Some(config.target_zip_folder.clone()),
            receipt_folder: Some(config.receipt_folder.clone()),
            conflict_resolution: Some(config.conflict_action.as_str().to_string()),
            post_processing: Some(config.post_action.as_str().to_string()),
            processing_logs: None,
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&receipt)?;
        fs::write(path, json)?;
        Ok(())
    }
}

fn is_receipt_filename(name: &str) -> bool {
    let n = name.to_lowercase();
    (n.starts_with("receipt")
        || n.starts_with("reciept")
        || n.contains("-receipt")
        || n.contains("-reciept"))
        && n.ends_with(".json")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ConflictAction, PostAction};

    #[test]
    fn test_rule_resolution_priority_and_folder_inspection() {
        let tmp_dir = std::env::temp_dir().join(format!("inbox_mover_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        let mut core = InboxMoverCore::with_base_dir(&tmp_dir);

        // Save a wildcard pattern rule (matches filenames or folder name) and an exact permit rule
        let pattern_rule = ConfigRule {
            auto_extract: true,
            target_folder: "/tmp/pattern_target".to_string(),
            target_zip_folder: String::new(),
            receipt_folder: String::new(),
            conflict_action: ConflictAction::KeepBoth,
            post_action: PostAction::Leave,
        };
        core.save_pattern("*.csv", pattern_rule).unwrap();

        let exact_rule = ConfigRule {
            auto_extract: false,
            target_folder: "/tmp/exact_target".to_string(),
            target_zip_folder: String::new(),
            receipt_folder: String::new(),
            conflict_action: ConflictAction::Overwrite,
            post_action: PostAction::Delete,
        };
        core.save_config("PERMIT-2026-999", exact_rule).unwrap();

        // Create a transfer folder matching wildcard pattern (*.csv)
        let tf1 = tmp_dir.join("transfer-001");
        fs::create_dir_all(&tf1).unwrap();
        fs::write(
            tf1.join("receipt.json"),
            r#"{"permit_id": "PERMIT-2026-123"}"#,
        )
        .unwrap();
        fs::write(tf1.join("sample.csv"), "a,b,c\n1,2,3\n").unwrap();

        let fd1 = core.inspect_transfer_folder(&tf1);
        assert_eq!(fd1.permit_id, "PERMIT-2026-123");
        let (rule1, src1, _is_mod1) = core.resolve_rule_for_folder(Some(&fd1));
        assert_eq!(rule1.target_folder, "/tmp/pattern_target");
        assert!(src1.contains("Pattern"));

        // Create a transfer folder matching exact permit
        let tf2 = tmp_dir.join("transfer-002");
        fs::create_dir_all(&tf2).unwrap();
        fs::write(
            tf2.join("receipt.json"),
            r#"{"permit_id": "PERMIT-2026-999"}"#,
        )
        .unwrap();
        fs::write(tf2.join("sample.csv"), "a,b,c\n1,2,3\n").unwrap();

        let fd2 = core.inspect_transfer_folder(&tf2);
        let (rule2, src2, _is_mod2) = core.resolve_rule_for_folder(Some(&fd2));
        assert_eq!(rule2.target_folder, "/tmp/exact_target");
        assert!(src2.contains("Config ID: PERMIT-2026-999"));

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_process_folder_end_to_end() {
        let tmp_dir = std::env::temp_dir().join(format!("inbox_mover_proc_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp_dir);
        let inbox_dir = tmp_dir.join("inbox").join("transfer-abc");
        let target_dir = tmp_dir.join("target_data");
        fs::create_dir_all(&inbox_dir).unwrap();

        let core = InboxMoverCore::with_base_dir(&tmp_dir);
        let rule = ConfigRule {
            auto_extract: true,
            target_folder: target_dir.to_string_lossy().to_string(),
            target_zip_folder: String::new(),
            receipt_folder: String::new(),
            conflict_action: ConflictAction::Overwrite,
            post_action: PostAction::Leave,
        };

        core.create_receipt_file(&inbox_dir.join("receipt.json"), &rule, Some("PERMIT-E2E"))
            .unwrap();
        fs::write(inbox_dir.join("data.txt"), "hello world").unwrap();

        let fd = core.inspect_transfer_folder(&inbox_dir);
        core.process_folder(
            &fd,
            &rule,
            None::<fn(usize, usize, &str)>,
            None::<fn(&str) -> Option<String>>,
        )
        .unwrap();

        assert!(target_dir.join("data.txt").exists());
        let has_timestamped_receipt = fs::read_dir(&target_dir)
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().ends_with("-receipt.json"));
        assert!(has_timestamped_receipt);
        assert!(inbox_dir.join("Process.log").exists());

        let logs = core.read_global_logs(50);
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].config_id, "PERMIT-E2E");

        let _ = fs::remove_dir_all(&tmp_dir);
    }
}


