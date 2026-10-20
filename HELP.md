# 📦 Inbox Mover — Complete User Guide & Reference Manual

**Inbox Mover** is a fast, modern Rust application for unpacking, routing, and auditing incoming `transfer-*` data packages across **Windows** and **Linux**.

It provides **three unified interfaces** powered by the exact same Rust core engine:
1. **Material 3 Graphical User Interface (`inbox-mover-gui` / `inbox-mover`)** — Identical look, feel, and features on both **Windows** and **Linux**.
2. **Material 3 Terminal User Interface (`inbox-mover-tui` / `inbox-mover --tui`)** — Full-featured terminal dashboard for **Linux** servers, SSH sessions, and keyboard enthusiasts.
3. **Headless CLI (`inbox-mover --cli`)** — Automated batch processing for scheduled scripts and cron jobs.

---

## 🧭 Table of Contents

1. [For First-Time Users: 60-Second Quick Start](#-1-for-first-time-users-60-second-quick-start)
2. [Simple Mode vs. Advanced Mode (Tailored Guidance)](#-2-simple-mode-vs-advanced-mode-tailored-guidance)
3. [Understanding the Transfer Queue & Status Badges](#-3-understanding-the-transfer-queue--status-badges)
4. [Destination Paths & Processing Options](#-4-destination-paths--processing-options)
5. [Receipts, Saved Rules & Priority Hierarchy](#-5-receipts-saved-rules--priority-hierarchy)
6. [Automating `No_ID` Transfers with Wildcard Patterns](#-6-automating-no_id-transfers-with-wildcard-patterns)
7. [Personal vs. Team Workspaces & Rule Syncing](#-7-personal-vs-team-workspaces--rule-syncing)
8. [Cross-Platform Windows ↔ Linux Path Translation](#-8-cross-platform-windows--linux-path-translation)
9. [3-Tier Audit Logging & Traceability](#-9-3-tier-audit-logging--traceability)
10. [Side-by-Side Keyboard Shortcuts (GUI & TUI)](#-10-side-by-side-keyboard-shortcuts-gui--tui)
11. [Headless CLI Reference](#-11-headless-cli-reference)
12. [Troubleshooting & FAQ](#-12-troubleshooting--faq)

---

## 🚀 1. For First-Time Users: 60-Second Quick Start

When incoming data arrives in your Inbox directory, it is packaged inside folders named `transfer-<something>` (for example, `transfer-2026-04-survey`). Inside may be `.zip` archives, loose files, and an optional `receipt.json` file.

### Processing Your First Transfer in 3 Steps:
1. **Set Your Inbox Folder (`Search 1`)**:
   - **GUI**: Click **Browse** next to `Search 1` (top-left) and choose the directory where `transfer-*` folders arrive.
   - **TUI**: Press `/` and type or paste your Inbox directory path, then press `Enter`.
2. **Select a Transfer Folder**:
   - Click a folder in the **Transfer Queue** on the left (or use `↑` / `↓` arrow keys).
   - Look at the **Smart Guidance Banner** at the top of the right panel — it tells you in plain English whether the folder is ready or what setting is missing.
3. **Configure & Process**:
   - If the folder shows **`[✗]`**, set a **Target Folder** (where extracted/copied files should go) and click **Save Rule** (`Ctrl+S` in GUI, `s` in TUI) so Inbox Mover remembers it next time.
   - Click **▶ Process Folder** (or press `Enter`).

---

## 🎛 2. Simple Mode vs. Advanced Mode (Tailored Guidance)

Inbox Mover is designed so **first-time users never feel overwhelmed** and **daily power users are never slowed down by popups or banners**.

| Feature | 🌱 Simple Mode (Guided) | ⚡ Advanced Mode (Power User) |
| :--- | :--- | :--- |
| **Target Audience** | First-time users & streamlined daily runs | Regular/advanced users & rule administrators |
| **How to Toggle** | Click **✨ Simple Mode** in top bar (or press `a` in TUI) | Click **⚙ Advanced Mode** in top bar (or press `a` in TUI) |
| **Smart Guidance Banner** | **ON** by default — explains folder state & next step | Toggle **OFF** permanently via **💡 Guide: OFF** (`Ctrl+G` / `g`) |
| **Controls Shown** | Search 1, Queue, Destinations, Process, Save Rule, Open, Logs | Adds **Personal/Team Workspace Sync**, **Overrule Receipt**, **Search 2**, **Manage Rules**, and **Create Receipt** |

> **Tip for Advanced Users**: On the initial Welcome screen, click **"Start in Power User Mode (No Banners)"** and check **"Don't show this welcome guide on startup"**. Your preference is saved in `permit_configs/settings.json` and remembered across both GUI and TUI sessions.

---

## 🏷 3. Understanding the Transfer Queue & Status Badges

Every `transfer-*` folder found in `Search 1` and `Search 2` (scanned up to 2 directory levels deep) is listed in the **Transfer Queue** with one of three status badges:

- **`[✓]` Ready (Green)**:
  - The folder contains valid files and has a complete destination configuration (either from a `receipt.json`, a saved Config ID, a matching Wildcard Pattern, or your `DEFAULT` rule).
  - You can process it immediately with a single click or `Enter`.
- **`[✓ (L)]` Previously Processed (Amber)**:
  - A local `Process.log` file was found inside this `transfer-*` folder.
  - This happens when the folder was previously processed with **After Processing** set to **Leave in Inbox**.
  - You can inspect its previous run via **View Log** (`Ctrl+L` / `l`) or process it again if needed.
- **`[✗]` Needs Setup / Attention (Red)**:
  - Either no **Target Folder** is configured yet, **After Processing** is set to *Move* without a **Processed Folder**, or the folder contains no valid processable files.

### Multi-Select Batch Processing
- **GUI**: Hold `Ctrl` to toggle individual folders or `Shift` to select a range, then click **▶ Process N Folders**.
- **TUI**: Press `Space` on folders in the queue to mark them with `[*]`, then press `Enter` or `p`.
- **Execution Order**: Batch processing always executes sequentially from **oldest to newest** (based on folder creation timestamp) so newer files naturally supersede older ones according to your conflict resolution setting.

---

## 🎯 4. Destination Paths & Processing Options

### Destination Paths
1. **Target Folder *(Required)***:
   - The destination directory where ZIP contents are extracted and loose files are copied.
2. **Processed Folder *(Required when After Processing = Move)***:
   - The archive directory where the entire source `transfer-*` folder is moved after all files have been extracted/copied.
3. **Receipt Folder *(Optional)***:
   - If specified, incoming `receipt*.json` files are routed here instead of the Target Folder. If left blank, receipts are placed inside the Target Folder.

### Processing Options
- **Auto-extract ZIP archives**:
  - **Enabled**: Extracts `.zip` archives into the Target Folder.
    - *Smart Prefix Stripping*: If a ZIP archive contains a redundant root folder named `transfer-.../`, Inbox Mover automatically strips that outer folder so your Target Folder stays clean.
    - *Encrypted ZIPs*: If a ZIP archive is password-protected, Inbox Mover prompts you for the password.
  - **Disabled**: Copies the `.zip` archive itself as a single file into the Target Folder.
- **If File Exists (Conflict Resolution)**:
  - **Overwrite**: Replaces the existing file in the Target Folder.
  - **Keep Both (Rename New)**: Preserves the existing file and renames the incoming file to `filename (1).ext`, `filename (2).ext`, etc.
  - **Rename Existing (Timestamp)**: Renames the existing file on disk to `YYMMDD-HHMMSS_filename.ext` before writing the incoming file under its original name.
- **After Processing (Post-Action)**:
  - **Move to Processed**: Moves the source `transfer-*` folder into your **Processed Folder** (appending `_1`, `_2` if a folder of the same name already exists there).
  - **Delete Source**: Permanently deletes the source `transfer-*` folder after successful extraction/copying.
  - **Leave in Inbox**: Leaves the `transfer-*` folder untouched in the Inbox and writes `Process.log` inside it (marking it `✓ (L)`).

---

## ⚖ 5. Receipts, Saved Rules & Priority Hierarchy

Incoming transfers can include a `receipt.json` (or `receipt_*.json`) either directly inside the `transfer-*` folder or packaged inside a `.zip` archive.

### How Inbox Mover Resolves Rules

#### Standard Mode (`Overrule receipt.json` is UNCHECKED)
1. **Receipt Overrides**: Any paths or options explicitly defined inside `receipt.json` (`target_folder`, `process_folder`, `receipt_folder`, `auto_extract`, `conflict_resolution`, `post_processing`) are applied on top of your base rule.
   - If the receipt's paths differ from your saved rule, Inbox Mover highlights **`● Modified`** and **`Save Rule *`** so you can see that the receipt customized the settings.
2. **Saved Config ID (`<permitId>.json`)**: Matched using `permitId` (or `id` / `configId`) from `receipt.json`.
3. **Saved Wildcard Pattern (`patterns.json`)**: Evaluated in order against the folder name, loose files, and filenames inside ZIP archives.
4. **Workspace `DEFAULT.json`**: Global fallback rule when no specific ID or pattern matches.

#### Overrule Mode (`Overrule receipt.json` is CHECKED)
- When you check **Overrule receipt.json with saved rules** (`v` in TUI):
  - Your saved **Config ID** (`<permitId>.json`) or **Wildcard Pattern** (`patterns.json`) takes **strict priority** over the paths inside `receipt.json`.
  - Receipt values are only used if you have **no** saved rule matching the folder (and the receipt paths are compatible with your current OS).

---

## ✳ 6. Automating `No_ID` Transfers with Wildcard Patterns

When a transfer folder does not contain a `receipt.json`, its Config ID is shown as **`No_ID`**.
Instead of configuring `No_ID` folders by hand every time:

1. Select the `No_ID` transfer folder and fill in your desired **Target Folder**, **Processed Folder**, and options.
2. Click **💾 Save Rule** (`Ctrl+S` in GUI, `s` in TUI).
3. Choose **Save as Wildcard Pattern Rule (`patterns.json`)**.
4. Enter a glob pattern such as:
   - `*survey_2026*` — matches if the folder name or any file inside it contains `survey_2026`.
   - `*.las` — matches any transfer containing a `.las` file (even inside a `.zip`!).
   - `transfer-clientA-*` — matches any folder starting with `transfer-clientA-`.
5. Manage or reorder pattern priority anytime in **⚙ Manage Rules** (`Ctrl+M` in GUI, `m` in TUI).

---

## 👥 7. Personal vs. Team Workspaces & Rule Syncing

All rules are stored as clean, human-readable JSON files inside `permit_configs/`:
- `permit_configs/personal/` — Your private `<permitId>.json` files, `patterns.json`, and `DEFAULT.json`.
- `permit_configs/team/` — Shared team configurations.

### Syncing Rules Between Personal & Team
In **Advanced Mode**, click **⇄ Sync** next to the Workspace selector in the left sidebar:
- Copy **all rules and patterns** from Personal → Team (or Team → Personal).
- Or copy a **single specific Config ID or Pattern** to share a newly created workflow with your colleagues.

---

## 🌐 8. Cross-Platform Windows ↔ Linux Path Translation

Teams often work across both Windows workstations and Linux servers mounted to the same network storage. Inbox Mover automatically translates network paths in both directions when loading configs, saving rules, or reading receipts:

| Windows Drive Path | Linux Mount Path |
| :--- | :--- |
| `Z:\project\inbox` | `/mnt/data/project/inbox` |
| `I:\incoming\transfers` | `/mnt/inbox/incoming/transfers` |

### Cross-OS Incompatible Path Protection
If an incoming `receipt.json` contains a machine-local path from another OS that cannot exist on your machine (for example, `C:\Users\John\Desktop` when running on Linux, or `/home/john/data` when running on Windows):
1. Inbox Mover displays a **⚠ Cross-OS Path Warning**.
2. If the folder has a matching **Wildcard Pattern** in your workspace, Inbox Mover automatically falls back to your pattern rule instead of failing on the foreign path.

---

## 📜 9. 3-Tier Audit Logging & Traceability

Every time you process a transfer folder, Inbox Mover writes a comprehensive audit record in three places:

1. **Global Audit Log (`permit_configs/process_log.jsonl`)**:
   - Appends a structured JSON-Lines entry recording the timestamp, username, status (`SUCCESS` or `ERROR`), folder name, Config ID, number of files processed, applied rule, and every individual file extract/copy/conflict action.
2. **Local Folder Log (`Process.log`)**:
   - Prepends a human-readable summary block to `Process.log` inside the transfer folder itself (newest runs appear at the top).
3. **Receipt Audit Injection (`processing_logs`)**:
   - Any `receipt*.json` extracted or copied during the run is renamed with a timestamp prefix (`YYMMDD-HHMMSS-receipt.json`) and updated in-place with a `"processing_logs": [...]` array containing the full run audit.

---

## ⌨ 10. Side-by-Side Keyboard Shortcuts (GUI & TUI)

The GUI and TUI share the same mental model and shortcut structure so you can switch between Windows/Linux desktop and Linux SSH terminals effortlessly:

| Action | GUI Shortcut (Windows & Linux) | TUI Shortcut (Linux Terminal) |
| :--- | :--- | :--- |
| **Navigate Transfer Queue** | `↑` / `↓` | `↑` / `↓` or `j` / `k` |
| **Multi-Select Folders (Batch)** | `Ctrl+Click` / `Shift+Click` | `Space` |
| **Process Selected Folder(s)** | `Enter` | `Enter` or `p` |
| **Save Current Rule** | `Ctrl+S` | `s` |
| **Refresh Transfer Queue** | `F5` or `Ctrl+R` | `r` or `F5` |
| **Toggle Smart Guidance Banner** | `Ctrl+G` | `g` |
| **Toggle Simple / Advanced Mode** | Top Bar Button | `a` |
| **Open Help Center & Shortcuts** | `F1` or `?` | `F1` or `?` |
| **View Local & Global Audit Logs** | `Ctrl+L` | `l` |
| **Manage Saved Rules & Patterns** | `Ctrl+M` | `m` |
| **Open Folder in File Manager** | `Ctrl+O` | `o` |
| **Delete Selected Folder(s)** | `Delete` | `d` or `Delete` |
| **Switch Workspace (Personal/Team)**| Sidebar Selector | `w` |
| **Toggle Overrule Receipt** | Sidebar Checkbox | `v` |
| **Edit Search 1 / Search 2 Path** | Click Input / `Browse` | `/` (Search 1), `2` (Search 2) |
| **Edit Target / Processed / Receipt**| Click Input / `Browse` | `t` (Target), `z` (Processed), `f` (Receipt) |
| **Toggle Auto-Extract ZIPs** | Checkbox | `e` |
| **Cycle Conflict / Post-Action** | Dropdown | `x` (Conflict), `u` (Post-Action) |
| **Create Custom `receipt.json`** | `Create Receipt` Button | `c` |
| **Scale UI Font Size** | `A+` / `A-` Buttons | Terminal Zoom (`Ctrl++` / `Ctrl+-`) |
| **Close Modal / Quit TUI** | `Esc` | `Esc` / `q` |

---

## 💻 11. Headless CLI Reference

Automate Inbox Mover in scripts, task schedulers, or cron jobs using `--cli`:

```bash
# 1. Process all ready (✓) transfer-* folders in a directory (oldest to newest):
inbox-mover --cli --dir /mnt/inbox

# 2. Scan two directories using Team workspace rules and overrule receipt.json:
inbox-mover --cli --dir /mnt/inbox --dir2 /mnt/data/incoming --workspace team --overrule

# 3. Process a single specific transfer folder:
inbox-mover --cli --folder /mnt/inbox/transfer-2026-04-survey
```

---

## 🛠 12. Troubleshooting & FAQ

- **Q: Why does a folder show `[✗]` in the queue?**
  - **A:** Either the **Target Folder** is blank, or **After Processing** is set to *Move to Processed* while **Processed Folder** is blank, or the folder is empty. Select the folder and read the **Smart Guidance Banner** at the top of the right panel for the exact cause.
- **Q: Why does the Save button say `Save Rule *` with a `● Modified` badge?**
  - **A:** The active settings in the right panel differ from your saved rule in `permit_configs/`. This happens either because you edited a path/option in the UI, or because an incoming `receipt.json` overrode your saved rule. Click `Save Rule *` (`Ctrl+S` / `s`) if you want to make those changes permanent.
- **Q: I'm an advanced user and don't want any help banners on screen. How do I turn them off permanently?**
  - **A:** Click **`💡 Guide: ON`** in the top bar (or press `Ctrl+G` in GUI / `g` in TUI) to switch it to **`💡 Guide: OFF`**. Your choice is saved immediately to `permit_configs/settings.json`.

