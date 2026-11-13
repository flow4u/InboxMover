# Inbox Mover — Prebuilt Executables & Edition Chooser

Inbox Mover is available in **two interchangeable editions**—**Python** and **Rust**—that share the exact same `permit_configs/` settings, rules, `receipt.json` format, and audit logs:

1. **🦀 Rust Prebuilt Executables (`bin/rust/`)** — Standalone native binaries in this folder for **Windows** and **Linux** (modern Material 3 GUI, TUI, and CLI; zero Python or driver installation required, works out-of-the-box on GPU-less Windows VMs / RDP).
2. **🐍 Python Edition (Repository Root & GitHub Releases)** — Standard-library Python scripts ([`../inbox_mover.py`](../inbox_mover.py), [`../inbox_mover_tui.py`](../inbox_mover_tui.py)) and the Python-based Windows release executable ([`inbox_mover.exe`](https://github.com/flow4u/InboxMover/releases/tag/windows)).

All Rust executables include `rust` in their filename so they never collide with Python scripts or `inbox_mover.exe` on your Desktop.

---

## 🪟 Windows — Rust Executables (`bin/rust/windows/`)

| Executable | Recommended For | Description |
| :--- | :--- | :--- |
| **[`inbox-mover-rust-gui.exe`](./rust/windows/inbox-mover-rust-gui.exe)** | **Desktop / VM Users (Recommended)** | Launches the **Material 3 Graphical User Interface** directly without a background console window. Includes built-in CPU rendering for GPU-less Windows VMs / myDRE RDP. |
| **[`inbox-mover-rust-tui.exe`](./rust/windows/inbox-mover-rust-tui.exe)** | **Terminal Users (TUI)** | Launches the **Material 3 Terminal User Interface** in PowerShell, Command Prompt, or Windows Terminal (no `windows-curses` required). |
| **[`inbox-mover-rust.exe`](./rust/windows/inbox-mover-rust.exe)** | **All-in-One / Batch / CLI** | Launches the GUI by default, TUI with `--tui`, or headless batch mode with `--cli`. |

*(Prefer the Python-based Windows executable or script? See [`inbox_mover.exe` on GitHub Releases](https://github.com/flow4u/InboxMover/releases/tag/windows) or [`../inbox_mover.py`](../inbox_mover.py).)*

---

## 🐧 Linux — Rust Executables (`bin/rust/linux/`)

| Executable | Recommended For | Description |
| :--- | :--- | :--- |
| **[`inbox-mover-rust-gui`](./rust/linux/inbox-mover-rust-gui)** | **Desktop Users (GUI)** | Launches the **Material 3 Graphical User Interface** (identical to the Windows Rust GUI; no `python3-tk` required). |
| **[`inbox-mover-rust-tui`](./rust/linux/inbox-mover-rust-tui)** | **Terminal / SSH Users (TUI)** | Lightweight (~2 MB) **Material 3 Terminal User Interface** with full keyboard navigation, status badges, and Smart Guidance. |
| **[`inbox-mover-rust`](./rust/linux/inbox-mover-rust)** | **All-in-One Launcher / CLI** | Launches the GUI by default, TUI with `--tui`, or headless batch mode with `--cli`. |

*(Prefer the Python scripts on Linux? Run `python3 ../inbox_mover.py` for GUI or `python3 ../inbox_mover_tui.py` for TUI.)*

### Running Rust Binaries on Linux
Ensure the downloaded binary has execute permissions:
```bash
chmod +x inbox-mover-rust-gui inbox-mover-rust-tui inbox-mover-rust
./inbox-mover-rust-gui     # Launch Material 3 GUI
./inbox-mover-rust-tui     # Launch Material 3 Terminal UI
./inbox-mover-rust --cli   # Run headless batch processing
```

---

## 📁 Shared Configuration & Full Compatibility

Both the Python and Rust editions read and write the `permit_configs/` directory (in your working directory or alongside the script/executable):
- `app_settings.json` (theme, font size, Simple/Advanced mode, search directories)
- `<PermitID>.json` and `patterns.json` (saved routing rules)
- `process_log.jsonl` (global audit history)

For full Rust Edition documentation and shortcuts, see **[`../rust/HELP.md`](../rust/HELP.md)**.
To rebuild these Rust executables from source after editing [`../rust/src/`](../rust/src/), run `./rust/build.sh` (see **[`../rust/README.md`](../rust/README.md)**).
