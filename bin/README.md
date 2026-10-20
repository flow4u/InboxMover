# Inbox Mover — Prebuilt Executables (Windows & Linux)

This folder contains standalone, ready-to-run executables for **Windows** and **Linux**. No Python or Rust installation is required to run them.

---

## 🪟 Windows (`bin/windows/`)

| Executable | Recommended For | Description |
| :--- | :--- | :--- |
| **[`inbox-mover-gui.exe`](./windows/inbox-mover-gui.exe)** | **Desktop Users (Recommended)** | Launches the **Material 3 Graphical User Interface** directly without opening a background command-prompt window. |
| **[`inbox-mover.exe`](./windows/inbox-mover.exe)** | **All-in-One / Batch / CLI** | Launches the Material 3 GUI by default, or runs headless batch processing when passed `--cli` in PowerShell / Command Prompt. |

---

## 🐧 Linux (`bin/linux/`)

| Executable | Recommended For | Description |
| :--- | :--- | :--- |
| **[`inbox-mover-gui`](./linux/inbox-mover-gui)** | **Desktop Users (GUI)** | Launches the **Material 3 Graphical User Interface** (identical to the Windows GUI). |
| **[`inbox-mover-tui`](./linux/inbox-mover-tui)** | **Terminal / SSH Users (TUI)** | Lightweight (2.7 MB) **Material 3 Terminal User Interface** with full keyboard navigation, status badges, and Smart Guidance. |
| **[`inbox-mover`](./linux/inbox-mover)** | **All-in-One Launcher** | Launches the GUI by default, TUI with `--tui`, or headless batch mode with `--cli`. |

### Running on Linux
Make sure the downloaded binary has execute permissions:
```bash
chmod +x inbox-mover-gui inbox-mover-tui inbox-mover
./inbox-mover-gui   # Launch Material 3 GUI
./inbox-mover-tui   # Launch Material 3 Terminal UI
./inbox-mover --cli # Run headless batch processing
```

---

## 📁 Where Configuration & Rules Are Stored
When you run any of these executables, Inbox Mover looks for (or automatically creates) the `permit_configs/` directory in your working directory (or alongside the executable) to store:
- `app_settings.json` (theme, UI scale, Simple/Advanced mode, search directories)
- `<PermitID>.json` and `patterns.json` (saved routing rules)
- `process_log.jsonl` (audit history)

For full usage documentation and keyboard shortcuts, see **[HELP.md](../HELP.md)**.

---

## 🛠️ For Developers / Colleagues: Rebuilding After Code Changes

Everything required to modify and rebuild both the Linux and Windows executables is included in this repository (`Cargo.toml`, `Cargo.lock`, `src/`, and `build.sh`).

### 1. One-Time Setup (Ubuntu / Debian Linux)
If building on a fresh machine, install Rust and the MinGW Windows cross-compiler:
```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-pc-windows-gnu

# Install system libraries for GUI and Windows cross-compilation
sudo apt update
sudo apt install -y build-essential pkg-config mingw-w64 \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

### 2. Build & Update `bin/`
After editing any file in `src/`, run from the repository root:
```bash
./build.sh
```
This compiles the Linux and Windows release binaries, strips debug symbols, and updates `bin/linux/` and `bin/windows/` ready to commit and push.

