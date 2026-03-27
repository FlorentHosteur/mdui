#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DIST_DIR="dist"
BIN_NAME="mdui"

echo "==> Building mdui v${VERSION}"
mkdir -p "$DIST_DIR"

build_target() {
    local target="$1"
    local label="$2"

    echo ""
    echo "--- Building for ${label} (${target}) ---"

    if command -v cross &>/dev/null; then
        cross build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi

    local bin_path="target/${target}/release/${BIN_NAME}"
    if [ ! -f "$bin_path" ]; then
        echo "WARN: binary not found at ${bin_path}, skipping"
        return 1
    fi

    local archive="${DIST_DIR}/${BIN_NAME}-v${VERSION}-${target}.tar.gz"
    tar -czf "$archive" -C "target/${target}/release" "$BIN_NAME"
    echo "  -> ${archive} ($(du -h "$archive" | cut -f1))"
}

# macOS arm64 (Apple Silicon) — native on M-series Macs
build_target "aarch64-apple-darwin" "macOS arm64" || true

# Linux amd64
build_target "x86_64-unknown-linux-gnu" "Linux amd64" || true

# Linux arm64
build_target "aarch64-unknown-linux-gnu" "Linux arm64" || true

echo ""
echo "==> Done. Archives in ${DIST_DIR}/:"
ls -lh "$DIST_DIR"/*.tar.gz 2>/dev/null || echo "(no archives built)"
