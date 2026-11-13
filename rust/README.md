# Inbox Mover — Rust Edition (`rust/`)

This folder contains the complete source code and cross-platform build tooling for the **Rust Edition** of **Inbox Mover**.

The repository maintains **both** implementations side-by-side so users and developers can choose what fits their workflow best:
- **Python Edition (Repository Root)**: [`../inbox_mover.py`](../inbox_mover.py), [`../inbox_mover_tui.py`](../inbox_mover_tui.py), and developer plugins ([`../inbox_mover_plugin.py`](../inbox_mover_plugin.py), [`../inbox_mover_plugin.R`](../inbox_mover_plugin.R)). Easy to customize on the fly as plain scripts.
- **Rust Edition (`rust/` + `bin/rust/`)**: Standalone, high-performance native executables for **Windows** and **Linux** with a modern Material 3 GUI (including a built-in CPU/Win32 GDI software renderer for GPU-less Windows VMs / RDP), Material 3 TUI, and headless CLI—requiring zero runtime dependencies.

Both editions share the exact same `permit_configs/` directory format, `receipt.json` schema, Windows ↔ Linux path translation rules, and 3-tier audit logs (`process_log.jsonl`, `Process.log`, and `processing_logs` receipt injection), so you can switch between Python and Rust seamlessly.

---

## 📦 Prebuilt Executables

Ready-to-run compiled Rust binaries are committed in **[`../bin/rust/`](../bin/rust/)**:
- **Windows (`../bin/rust/windows/`)**:
  - `inbox-mover-rust-gui.exe` — Material 3 GUI (launches directly without a console window)
  - `inbox-mover-rust-tui.exe` — Material 3 Terminal UI (runs in PowerShell / Command Prompt / Windows Terminal; no `windows-curses` needed)
  - `inbox-mover-rust.exe` — All-in-One Launcher (GUI default, `--tui`, or `--cli`)
- **Linux (`../bin/rust/linux/`)**:
  - `inbox-mover-rust-gui` — Material 3 GUI (identical to Windows GUI)
  - `inbox-mover-rust-tui` — Material 3 Terminal UI (ideal for SSH and terminal sessions)
  - `inbox-mover-rust` — All-in-One Launcher (GUI default, `--tui`, or `--cli`)

For full user documentation and keyboard shortcuts for the Rust Edition, see **[`HELP.md`](./HELP.md)**.

---

## 🗂️ Rust Project Layout

```text
rust/
├── Cargo.toml               # Crate manifest & binary target definitions
├── Cargo.lock               # Locked dependency versions for reproducible builds
├── build.sh                 # Builds Linux & Windows releases and updates ../bin/rust/
├── README.md                # This developer & architecture overview
├── HELP.md                  # Complete Rust Edition User Guide & Reference Manual
└── src/
    ├── lib.rs               # Shared library root (inbox_mover_core)
    ├── models.rs            # Shared config, receipt, pattern, and audit log models
    ├── path_translator.rs   # Automatic Windows (I:\, Z:\) <-> Linux (/mnt/inbox, /mnt/data) path translation
    ├── core.rs              # Rule resolution, ZIP extraction, conflict handling, 3-tier logging
    ├── help.rs              # Contextual Smart Guidance engine & in-app Help Center content
    ├── gui.rs               # Material 3 GUI (egui/eframe) for Windows & Linux
    ├── sw_renderer.rs       # Pure-CPU software rasterizer + Win32 GDI presenter for GPU-less Windows VMs
    ├── tui.rs               # Material 3 Terminal UI (ratatui/crossterm)
    ├── cli.rs               # Headless CLI batch processor
    └── bin/
        ├── inbox-mover-rust.rs      # All-in-One binary entrypoint (GUI / --tui / --cli)
        ├── inbox-mover-rust-gui.rs  # Dedicated GUI binary entrypoint
        └── inbox-mover-rust-tui.rs  # Dedicated TUI binary entrypoint
```

---

## 🛠️ Building & Updating `bin/rust/`

### 1. One-Time Toolchain Setup (Ubuntu / Debian Linux)
To compile both the native Linux binaries and cross-compile the Windows `.exe` binaries from Linux:

```bash
# Install Rust toolchain + Windows GNU target
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-pc-windows-gnu

# Install system headers and MinGW cross-compiler
sudo apt update
sudo apt install -y build-essential pkg-config mingw-w64 \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev
```

### 2. Run Tests
```bash
cd rust
cargo test
```

### 3. Build & Package All Executables
From the repository root (or inside `rust/`), run:
```bash
./rust/build.sh
```
This compiles optimized release binaries for both Linux and Windows, strips debug symbols, and copies the updated executables directly into `bin/rust/linux/` and `bin/rust/windows/` ready to commit and push to GitHub.
