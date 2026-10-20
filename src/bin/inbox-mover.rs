use std::path::PathBuf;

use clap::Parser;
use inbox_mover_core::{cli, core::InboxMoverCore, gui, tui};

#[derive(Parser, Debug)]
#[command(
    name = "inbox-mover",
    version,
    about = "Inbox Mover — Modern Transfer Folder Processor (GUI, TUI & Headless CLI)"
)]
struct Args {
    /// Launch in Terminal User Interface (TUI) mode instead of the graphical window
    #[arg(long, short = 't')]
    tui: bool,

    /// Run in headless Command-Line Interface (CLI) mode for batch scripts / cron jobs
    #[arg(long, short = 'c')]
    cli: bool,

    /// Primary search directory containing transfer-* folders
    #[arg(long, short = 'd')]
    dir: Option<String>,

    /// Optional secondary search directory containing transfer-* folders
    #[arg(long)]
    dir2: Option<String>,

    /// Process a single specific transfer-* folder path in --cli mode
    #[arg(long, short = 'f')]
    folder: Option<String>,

    /// Active workspace ('personal' or 'team')
    #[arg(long, short = 'w')]
    workspace: Option<String>,

    /// Overrule receipt.json settings with saved workspace rules
    #[arg(long, short = 'o')]
    overrule: bool,

    /// Optional base directory where permit_configs/ is located (defaults to current/exe directory)
    #[arg(long)]
    config_root: Option<PathBuf>,
}

fn resolve_base_dir(custom: Option<PathBuf>) -> PathBuf {
    if let Some(p) = custom {
        return p;
    }
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
    let args = Args::parse();
    let base_dir = resolve_base_dir(args.config_root);
    let mut core = InboxMoverCore::with_base_dir(&base_dir);

    if let Some(ref d1) = args.dir {
        core.app_settings.search_folder_1 = d1.clone();
    }
    if let Some(ref d2) = args.dir2 {
        core.app_settings.search_folder_2 = d2.clone();
    }
    if let Some(ref ws) = args.workspace {
        core.switch_workspace(ws);
    }
    if args.overrule {
        core.app_settings.overrule_receipt = true;
    }

    let result = if args.cli {
        cli::run_cli(
            core,
            args.dir.as_deref(),
            args.dir2.as_deref(),
            args.folder.as_deref(),
            args.workspace.as_deref(),
            args.overrule,
        )
    } else if args.tui {
        tui::run_tui(core)
    } else {
        gui::run_gui(core)
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

