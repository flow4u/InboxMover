use crate::models::{ConfigRule, FolderData};

/// Represents a categorized help topic displayed in the in-app Help Center (GUI & TUI).
#[derive(Debug, Clone)]
pub struct HelpSection {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub badge: &'static str,
    pub content: &'static str,
}

/// Represents a keyboard shortcut entry for the Shortcut Cheat-Sheet.
#[derive(Debug, Clone)]
pub struct ShortcutEntry {
    pub key_gui: &'static str,
    pub key_tui: &'static str,
    pub action: &'static str,
    pub description: &'static str,
}

/// Generates dynamic, plain-English contextual guidance based on the currently selected folder
/// and rule state. Designed for beginners (while toggleable for power users).
#[derive(Debug, Clone)]
pub struct ContextualGuidance {
    pub level: GuidanceLevel,
    pub headline: String,
    pub explanation: String,
    pub next_step_hint: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidanceLevel {
    Ready,
    Modified,
    AttentionRequired,
    PreviouslyProcessed,
    Idle,
}

pub fn analyze_context(
    selected_folder: Option<&FolderData>,
    active_rule: &ConfigRule,
    active_rule_source: &str,
    is_modified: bool,
    overrule_receipt: bool,
) -> ContextualGuidance {
    let Some(folder) = selected_folder else {
        return ContextualGuidance {
            level: GuidanceLevel::Idle,
            headline: "No Transfer Folder Selected".to_string(),
            explanation: "Select a 'transfer-*' folder from the Queue on the left, or click 'Browse' next to Search 1 to point Inbox Mover at your Inbox directory.".to_string(),
            next_step_hint: "Tip: Use ↑/↓ arrow keys to browse folders in the queue, or press F1 / ? anytime for help.".to_string(),
        };
    };

    if folder.has_log {
        return ContextualGuidance {
            level: GuidanceLevel::PreviouslyProcessed,
            headline: format!("Folder '{}' Was Already Processed", folder.folder_name),
            explanation: "A local 'Process.log' was detected inside this folder, meaning it was previously processed with Post-Action set to 'Leave in Inbox'.".to_string(),
            next_step_hint: "Click 'View Log' (or press 'L') to inspect the previous run, or 'Process' to run it again.".to_string(),
        };
    }

    if !folder.can_process {
        return ContextualGuidance {
            level: GuidanceLevel::AttentionRequired,
            headline: "Empty or Invalid Transfer Folder".to_string(),
            explanation: "This folder does not contain any processable files or archives (only empty directories or log files).".to_string(),
            next_step_hint: "You can safely remove this empty folder using 'Delete' (Del), or inspect it in your file manager ('Open' / 'O').".to_string(),
        };
    }

    if folder.receipt_warning.is_some() && !overrule_receipt {
        return ContextualGuidance {
            level: GuidanceLevel::AttentionRequired,
            headline: "Receipt Warning / Cross-OS Path".to_string(),
            explanation: folder
                .receipt_warning
                .clone()
                .unwrap_or_else(|| "The receipt.json inside this transfer contains a warning or cross-OS incompatible path.".to_string()),
            next_step_hint: "Verify the Target Folder below, or check 'Overrule receipt' to apply your saved Personal/Team rules instead.".to_string(),
        };
    }

    if active_rule.target_folder.trim().is_empty() {
        return ContextualGuidance {
            level: GuidanceLevel::AttentionRequired,
            headline: "Target Destination Needed (✗ Unconfigured)".to_string(),
            explanation: format!(
                "Folder '{}' (Config ID: {}) does not have a Target Folder configured yet.",
                folder.folder_name, folder.permit_id
            ),
            next_step_hint: "1. Set a 'Target Folder' below -> 2. Click 'Save Rule' to remember it for next time -> 3. Click 'Process'.".to_string(),
        };
    }

    if active_rule.post_action == crate::models::PostAction::Move
        && active_rule.target_zip_folder.trim().is_empty()
    {
        return ContextualGuidance {
            level: GuidanceLevel::AttentionRequired,
            headline: "Processed Folder Required for 'Move' Action".to_string(),
            explanation: "Post-Processing is set to 'Move to Processed', but the 'Processed Folder' path is empty.".to_string(),
            next_step_hint: "Specify a 'Processed Folder' path below, or change After Processing to 'Delete' or 'Leave in Inbox'.".to_string(),
        };
    }

    if is_modified {
        return ContextualGuidance {
            level: GuidanceLevel::Modified,
            headline: format!("Ready to Process — Unsaved Rule Changes ({active_rule_source})"),
            explanation: "This folder has all required paths to process right now, but its settings differ from your saved workspace rules (either edited manually or overridden by receipt.json).".to_string(),
            next_step_hint: "Click 'Process' (Enter) to run once, or click 'Save Rule *' (Ctrl+S / S) to permanently save these settings.".to_string(),
        };
    }

    ContextualGuidance {
        level: GuidanceLevel::Ready,
        headline: format!("Ready to Process ({active_rule_source})"),
        explanation: format!(
            "All destination paths are configured. Files will be {} to '{}'.",
            if active_rule.auto_extract {
                "extracted & copied"
            } else {
                "copied without unzipping"
            },
            active_rule.target_folder
        ),
        next_step_hint: "Press 'Process' (Enter) to execute immediately, or Ctrl+Click / Space to batch-select multiple folders.".to_string(),
    }
}

pub const QUICK_TIPS: &[(&str, &str)] = &[
    (
        "What do the Queue Badges mean?",
        "• [✓] Ready: A saved rule or receipt provides a valid Target Folder.\n• [✓ (L)] Processed: Contains a local Process.log from an earlier run.\n• [✗] Needs Setup: No Target Folder is configured yet — set one and click Save Rule.",
    ),
    (
        "Simple Mode vs. Advanced Mode",
        "• Simple Mode hides advanced rule management, secondary search paths, and workspace merging so beginners can focus on 1-click processing.\n• Advanced Mode unlocks Pattern Rules (*wildcards*), Team Workspace syncing, Receipt generation, and Overrule toggles.",
    ),
    (
        "How Rule Priority Works",
        "By default, settings inside a transfer's `receipt.json` win first. Otherwise, Inbox Mover checks:\n1. Saved Config ID (`permitId` from receipt)\n2. Pattern Match (`*project*` against folder or file names)\n3. DEFAULT fallback rule.\nCheck 'Overrule receipt' if you always want your saved rules to win!",
    ),
    (
        "Batch Processing Multiple Folders",
        "• GUI: Hold Shift or Ctrl while clicking folders in the Queue, then click 'Process N Folders'.\n• TUI: Press Space on folders in the Queue to toggle selection [*], then press Enter or P to batch-process oldest to newest.",
    ),
    (
        "Seamless Windows <-> Linux Paths",
        "Inbox Mover automatically translates network drive paths between Windows and Linux:\n• `z:\\` <-> `/mnt/data/`\n• `i:\\` <-> `/mnt/inbox/`\nSo rules and receipts created on Windows work transparently on Linux!",
    ),
];

pub const SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        key_gui: "↑ / ↓",
        key_tui: "↑ / ↓ or j / k",
        action: "Navigate Queue",
        description: "Move selection up or down in the Transfer Folders queue",
    },
    ShortcutEntry {
        key_gui: "Enter",
        key_tui: "Enter or p",
        action: "Process Selected",
        description: "Process the currently selected folder (or batch of selected folders)",
    },
    ShortcutEntry {
        key_gui: "Ctrl+Click / Shift+Click",
        key_tui: "Space",
        action: "Multi-Select",
        description: "Toggle multiple folders for sequential batch processing",
    },
    ShortcutEntry {
        key_gui: "F5 / Ctrl+R",
        key_tui: "r",
        action: "Refresh Queue",
        description: "Re-scan Search 1 and Search 2 directories for transfer-* folders",
    },
    ShortcutEntry {
        key_gui: "Ctrl+S",
        key_tui: "s",
        action: "Save Rule",
        description: "Save current destinations & options to Config ID or Pattern rule",
    },
    ShortcutEntry {
        key_gui: "Delete",
        key_tui: "d / Delete",
        action: "Delete Folder(s)",
        description: "Permanently delete selected transfer folder(s) after confirmation",
    },
    ShortcutEntry {
        key_gui: "Ctrl+O",
        key_tui: "o",
        action: "Open in File Manager",
        description: "Open the selected transfer folder in Windows Explorer / Linux file manager",
    },
    ShortcutEntry {
        key_gui: "Ctrl+L",
        key_tui: "l",
        action: "View Audit Logs",
        description: "Open the Local Process.log or Global Audit Log viewer",
    },
    ShortcutEntry {
        key_gui: "Ctrl+M",
        key_tui: "m",
        action: "Manage Rules",
        description: "Open the Configuration & Pattern Rules Manager",
    },
    ShortcutEntry {
        key_gui: "Ctrl+G",
        key_tui: "g",
        action: "Toggle Smart Guide",
        description: "Show or hide the contextual Smart Guidance banner",
    },
    ShortcutEntry {
        key_gui: "F1 / ?",
        key_tui: "? / F1",
        action: "Help Center",
        description: "Open the interactive Help Center & User Guide",
    },
    ShortcutEntry {
        key_gui: "Ctrl+Plus / Ctrl+Minus",
        key_tui: "Terminal Zoom",
        action: "Scale UI Font",
        description: "Increase or decrease GUI font size (persisted automatically)",
    },
];

pub const HELP_SECTIONS: &[HelpSection] = &[
    HelpSection {
        id: "getting-started",
        title: "1. Getting Started (Beginner's Guide)",
        subtitle: "Understand what Inbox Mover does and process your first folder in 30 seconds",
        badge: "BEGINNER",
        content: r#"### Welcome to Inbox Mover!
Inbox Mover automates unpacking, routing, and logging incoming data transfers (`transfer-*` folders).

#### The 3-Step Workflow
1. **Select Your Inbox (`Search 1`)**: Point `Search 1` at the folder where incoming `transfer-*` packages arrive. Inbox Mover scans up to 2 levels deep.
2. **Pick a Transfer Folder**: Click any folder in the **Queue** on the left.
   - **[✓] Green Check**: Ready to go! Destination paths are already known.
   - **[✓ (L)] Processed**: Already processed earlier (a `Process.log` exists inside).
   - **[✗] Red Cross**: Needs a **Target Folder** before it can be processed.
3. **Click Process (or press `Enter`)**:
   - ZIP files are automatically extracted (if *Auto-extract* is enabled).
   - Loose files are copied preserving their folder structure.
   - `receipt.json` files are timestamped (`YYMMDD-HHMMSS-receipt.json`) and updated with an audit trail.
   - The source `transfer-*` folder is either **Moved**, **Deleted**, or **Left in Inbox** based on your setting.

#### Simple Mode vs. Advanced Mode
If you are new to Inbox Mover, switch to **Simple Mode** in the top bar. It declutters the interface to show only the essentials needed for daily processing. When you need pattern matching, receipt generation, or team workspace syncing, toggle **Advanced Mode** on anytime."#,
    },
    HelpSection {
        id: "receipts-and-priority",
        title: "2. Receipts, Rules & Priority Hierarchy",
        subtitle: "How Inbox Mover decides where files go and how 'Overrule receipt' works",
        badge: "CORE CONCEPT",
        content: r#"### How Configuration Priority Works
When you select a `transfer-*` folder, Inbox Mover determines its destination paths and behavior using a smart priority system:

#### Default Behavior (`Overrule receipt` is OFF)
1. **Receipt JSON (`receipt.json`)**: If a `receipt*.json` exists inside the folder or inside a `.zip`, its settings (`target_folder`, `process_folder`, `receipt_folder`, `conflict_resolution`, `post_processing`) take top priority.
   - If these settings differ from your saved rule, the Save button shows **`Save Rule *`** so you can optionally save them.
2. **Saved Config ID (`<permitId>.json`)**: If the receipt has a `permitId` (or `id`/`configId`), Inbox Mover loads your saved rule for that ID.
3. **Saved Pattern Rule (`patterns.json`)**: If no Config ID exists (`No_ID`), Inbox Mover tests wildcard patterns (like `*project_alpha*` or `*.csv`) against the folder name and every file inside it (including inside ZIPs!).
4. **`DEFAULT` Rule**: Your global fallback configuration.

#### Power User Mode: `Overrule receipt` is ON
When you check **Overrule receipt**:
- Your saved **Config ID** or **Pattern Match** rule wins *even if* the incoming `receipt.json` asks for different folders!
- This is essential when external senders embed their own folder paths in `receipt.json` that don't match your local project structure."#,
    },
    HelpSection {
        id: "destinations-and-options",
        title: "3. Destinations, Conflicts & Post-Actions",
        subtitle: "Detailed explanation of every path field, conflict mode, and cleanup option",
        badge: "REFERENCE",
        content: r#"### Destination Paths
- **Target Folder *(Required)***: Where extracted files from ZIPs and loose files in the transfer folder are placed.
- **Processed Folder *(Required when Post-Action = Move)***: Where the entire `transfer-*` folder is archived after successful processing.
- **Receipt Folder *(Optional)***: If specified, `receipt*.json` files are routed here instead of the Target Folder.

### Processing Options
- **Auto-extract ZIP files**:
  - **Checked**: Unzips `.zip` archives into the Target Folder (automatically stripping redundant top-level `transfer-*/` folders inside the ZIP). Supports password-protected ZIPs!
  - **Unchecked**: Copies the `.zip` file itself directly into the Target Folder as a regular file.
- **If File Exists (Conflict Resolution)**:
  - **Overwrite**: Replaces the existing file in the Target Folder.
  - **Keep Both (Rename New)**: Keeps the existing file and renames the incoming file to `filename (1).ext`.
  - **Rename Existing (Timestamp)**: Renames the old file already on disk to `YYMMDD-HHMMSS_filename.ext` and writes the incoming file with its original name.
- **After Processing (Post-Action)**:
  - **Move to Processed**: Moves the source `transfer-*` folder into your *Processed Folder*.
  - **Delete Source**: Permanently removes the source `transfer-*` folder after all files are safely extracted/copied.
  - **Leave in Inbox**: Leaves the `transfer-*` folder where it is and writes a `Process.log` inside it so the queue marks it as `✓ (L)`."#,
    },
    HelpSection {
        id: "patterns-and-workspaces",
        title: "4. Wildcard Patterns & Team Workspaces",
        subtitle: "Automate 'No_ID' transfers with glob patterns and collaborate with Team configs",
        badge: "ADVANCED",
        content: r#"### Pattern Matching (`patterns.json`)
Many transfers arrive without a `receipt.json` (shown as Config: `No_ID`). Instead of configuring them manually every time:
1. Click **Save Rule** (or open **Manage Rules**).
2. Choose **Save as Pattern Rule** and enter a wildcard pattern:
   - `*soil_survey*` — matches any folder or file containing `soil_survey`
   - `transfer-2026-*` — matches transfers from 2026
   - `*report*.pdf` — matches any transfer containing a matching PDF (even inside a ZIP!)
3. Patterns are evaluated in order; you can reorder or edit them anytime in **Manage Rules** (`Ctrl+M` / `m`).

### Personal vs. Team Workspaces
Inbox Mover stores rules inside `permit_configs/`:
- **Personal (`permit_configs/personal/`)**: Your private rules and `DEFAULT.json`.
- **Team (`permit_configs/team/`)**: Shared rules for your team.
- **Syncing (`Merge / Copy`)**: Click the **Sync / Merge** button next to Workspace to copy all rules (or a single Config ID / Pattern) between Personal and Team workspaces."#,
    },
    HelpSection {
        id: "cross-platform-and-logs",
        title: "5. Cross-Platform Paths, Audit Logs & CLI",
        subtitle: "Windows/Linux path translation, 3-tier audit logging, and headless CLI automation",
        badge: "POWER USER",
        content: r#"### Automatic Windows <-> Linux Path Translation
When working in mixed Windows and Linux environments sharing network drives, Inbox Mover automatically converts paths on the fly:
- `Z:\project\data`  <-->  `/mnt/data/project/data`
- `I:\incoming`      <-->  `/mnt/inbox/incoming`

If a receipt contains a machine-local path from another OS (like `C:\Users\...` on Linux), Inbox Mover flags a **Cross-OS Warning** and automatically checks if you have a matching Pattern Rule instead.

### 3-Tier Audit Logging
Every processing run records a complete audit trail:
1. **Global Audit Log (`permit_configs/process_log.jsonl`)**: Structured JSON-Lines history of every run across all folders. Viewable via **View Log -> Global History**.
2. **Local Folder Log (`Process.log`)**: Human-readable log prepended inside the transfer folder itself (newest entries at the top).
3. **Receipt Injection (`processing_logs`)**: Appends the run metadata directly into the extracted `YYMMDD-HHMMSS-receipt.json` file.

### Headless CLI Mode
Run Inbox Mover from scripts or cron jobs without opening a window:
```bash
# Process all ready (✓) folders in a directory automatically:
inbox-mover --cli --dir /mnt/inbox --workspace personal

# Overrule receipts and process a single specific transfer folder:
inbox-mover --cli --folder /mnt/inbox/transfer-12345 --overrule
```"#,
    },
];

