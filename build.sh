#!/usr/bin/env bash
# =============================================================================
# Inbox Mover — Build & Package Script
# Compiles release executables for Linux & Windows and places them into:
#   - bin/linux/   (inbox-mover, inbox-mover-gui, inbox-mover-tui)
#   - bin/windows/ (inbox-mover.exe, inbox-mover-gui.exe)
#
# Usage:
#   ./build.sh              # Build Linux + Windows releases and copy to bin/
#   ./build.sh --offline    # Build using cached crates only (no network)
#   ./build.sh --copy-only  # Skip cargo build; only copy existing target/ files to bin/
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

COPY_ONLY=false
CARGO_FLAGS=()

for arg in "$@"; do
    case "$arg" in
        --copy-only)
            COPY_ONLY=true
            ;;
        --offline)
            CARGO_FLAGS+=("--offline")
            ;;
        -h|--help)
            echo "Usage: ./build.sh [--offline] [--copy-only]"
            echo "  --offline    Build using locally cached Cargo dependencies"
            echo "  --copy-only  Only copy/strip already-compiled binaries from target/ into bin/"
            exit 0
            ;;
        *)
            CARGO_FLAGS+=("$arg")
            ;;
    esac
done

if [ "$COPY_ONLY" = false ]; then
    echo "============================================================"
    echo " [1/3] Building Linux Release Binaries (GUI, TUI, Launcher)"
    echo "============================================================"
    cargo build --release "${CARGO_FLAGS[@]}"

    echo ""
    echo "============================================================"
    echo " [2/3] Building Windows Release Binaries (GUI & Launcher)"
    echo "============================================================"
    cargo build --release --target x86_64-pc-windows-gnu --bin inbox-mover-gui --bin inbox-mover "${CARGO_FLAGS[@]}"
fi

echo ""
echo "============================================================"
echo " [3/3] Packaging Executables into bin/linux & bin/windows"
echo "============================================================"

mkdir -p bin/linux bin/windows

# Copy Linux executables
LINUX_BINS=("inbox-mover" "inbox-mover-gui" "inbox-mover-tui")
for bin_name in "${LINUX_BINS[@]}"; do
    src_path="target/release/${bin_name}"
    dest_path="bin/linux/${bin_name}"
    if [ -f "$src_path" ]; then
        cp -f "$src_path" "$dest_path"
        chmod +x "$dest_path"
        if command -v strip >/dev/null 2>&1; then
            strip -s "$dest_path" || true
        fi
        echo "  ✓ Updated $dest_path"
    else
        echo "  ! Warning: $src_path not found"
    fi
done

# Copy Windows executables
WIN_BINS=("inbox-mover.exe" "inbox-mover-gui.exe")
for bin_name in "${WIN_BINS[@]}"; do
    src_path="target/x86_64-pc-windows-gnu/release/${bin_name}"
    dest_path="bin/windows/${bin_name}"
    if [ -f "$src_path" ]; then
        cp -f "$src_path" "$dest_path"
        if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
            x86_64-w64-mingw32-strip -s "$dest_path" || true
        fi
        echo "  ✓ Updated $dest_path"
    else
        echo "  ! Warning: $src_path not found"
    fi
done

echo ""
echo "============================================================"
echo " Build Complete! Contents of bin/:"
echo "============================================================"
ls -lh bin/linux/ bin/windows/

