#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use inbox_mover_core::{core::InboxMoverCore, gui};

fn resolve_base_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("permit_configs").exists() {
        return cwd;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("permit_configs").exists() {
                return parent.to_path_buf();
            }
        }
    }
    cwd
}

fn main() {
    let base_dir = resolve_base_dir();
    let core = InboxMoverCore::with_base_dir(&base_dir);
    if let Err(e) = gui::run_gui(core) {
        eprintln!("GUI Error: {e}");
        std::process::exit(1);
    }
}

