#!/usr/bin/env bash
# =============================================================================
# Inbox Mover (Rust Edition) — Build & Package Script
# Compiles standalone Rust release executables for Linux & Windows and places
# them into the repository's prebuilt binaries folder:
#   - bin/rust/linux/   (inbox-mover-rust, inbox-mover-rust-gui, inbox-mover-rust-tui)
#   - bin/rust/windows/ (inbox-mover-rust.exe, inbox-mover-rust-gui.exe, inbox-mover-rust-tui.exe)
#
# Usage (from repository root or from inside rust/):
#   ./rust/build.sh              # Build Linux + Windows releases and copy to bin/rust/
#   ./rust/build.sh --offline    # Build using cached crates only (no network)
#   ./rust/build.sh --copy-only  # Skip cargo build; only copy existing target/ files to bin/rust/
# =============================================================================

set -euo pipefail

RUST_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$RUST_DIR/.." && pwd)"
OUT_DIR="$REPO_ROOT/bin/rust"

cd "$RUST_DIR"

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
            echo "Usage: ./rust/build.sh [--offline] [--copy-only]"
            echo "  --offline    Build using locally cached Cargo dependencies"
            echo "  --copy-only  Only copy/strip already-compiled binaries from rust/target/ into bin/rust/"
            exit 0
            ;;
        *)
            CARGO_FLAGS+=("$arg")
            ;;
    esac
done

if [ "$COPY_ONLY" = false ]; then
    echo "============================================================"
    echo " [1/3] Building Linux Rust Release Binaries"
    echo "============================================================"
    cargo build --release "${CARGO_FLAGS[@]}"

    echo ""
    echo "============================================================"
    echo " [2/3] Building Windows Rust Release Binaries"
    echo "============================================================"
    cargo build --release --target x86_64-pc-windows-gnu \
        --bin inbox-mover-rust-gui \
        --bin inbox-mover-rust-tui \
        --bin inbox-mover-rust \
        "${CARGO_FLAGS[@]}"
fi

echo ""
echo "============================================================"
echo " [3/3] Packaging Executables into bin/rust/{linux,windows}"
echo "============================================================"

mkdir -p "$OUT_DIR/linux" "$OUT_DIR/windows"

# Copy Linux Rust executables
LINUX_BINS=("inbox-mover-rust" "inbox-mover-rust-gui" "inbox-mover-rust-tui")
for bin_name in "${LINUX_BINS[@]}"; do
    src_path="$RUST_DIR/target/release/${bin_name}"
    dest_path="$OUT_DIR/linux/${bin_name}"
    if [ -f "$src_path" ]; then
        cp -f "$src_path" "$dest_path"
        chmod +x "$dest_path"
        if command -v strip >/dev/null 2>&1; then
            strip -s "$dest_path" || true
        fi
        echo "  ✓ Updated bin/rust/linux/${bin_name}"
    else
        echo "  ! Warning: $src_path not found"
    fi
done

# Copy Windows Rust executables
WIN_BINS=("inbox-mover-rust.exe" "inbox-mover-rust-gui.exe" "inbox-mover-rust-tui.exe")
for bin_name in "${WIN_BINS[@]}"; do
    src_path="$RUST_DIR/target/x86_64-pc-windows-gnu/release/${bin_name}"
    dest_path="$OUT_DIR/windows/${bin_name}"
    if [ -f "$src_path" ]; then
        cp -f "$src_path" "$dest_path"
        if command -v x86_64-w64-mingw32-strip >/dev/null 2>&1; then
            x86_64-w64-mingw32-strip -s "$dest_path" || true
        fi
        echo "  ✓ Updated bin/rust/windows/${bin_name}"
    else
        echo "  ! Warning: $src_path not found"
    fi
done

echo ""
echo "============================================================"
echo " Build Complete! Contents of bin/rust/:"
echo "============================================================"
ls -lh "$OUT_DIR/linux/" "$OUT_DIR/windows/"
