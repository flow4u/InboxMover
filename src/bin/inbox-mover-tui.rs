use std::path::PathBuf;

use clap::Parser;
use inbox_mover_core::{core::InboxMoverCore, tui};

#[derive(Parser, Debug)]
#[command(
    name = "inbox-mover-tui",
    version,
    about = "Inbox Mover — Modern Terminal User Interface (TUI)"
)]
struct TuiArgs {
    /// Primary search directory containing transfer-* folders
    #[arg(long, short = 'd')]
    dir: Option<String>,

    /// Optional secondary search directory
    #[arg(long)]
    dir2: Option<String>,

    /// Active workspace ('personal' or 'team')
    #[arg(long, short = 'w')]
    workspace: Option<String>,

    /// Overrule receipt.json settings with saved workspace rules
    #[arg(long, short = 'o')]
    overrule: bool,
}

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
    let args = TuiArgs::parse();
    let base_dir = resolve_base_dir();
    let mut core = InboxMoverCore::with_base_dir(&base_dir);

    if let Some(d1) = args.dir {
        core.app_settings.search_folder_1 = d1;
    }
    if let Some(d2) = args.dir2 {
        core.app_settings.search_folder_2 = d2;
    }
    if let Some(ws) = args.workspace {
        core.switch_workspace(&ws);
    }
    if args.overrule {
        core.app_settings.overrule_receipt = true;
    }

    if let Err(e) = tui::run_tui(core) {
        eprintln!("TUI Error: {e}");
        std::process::exit(1);
    }
}

