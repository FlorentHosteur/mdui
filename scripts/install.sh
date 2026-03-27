#!/usr/bin/env bash
#
# Install mdui from GitHub releases or build from source.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/FlorentHosteur/mdui/main/scripts/install.sh | bash
#   ./scripts/install.sh            # from repo root
#   INSTALL_DIR=/usr/local/bin ./scripts/install.sh
#
set -euo pipefail

REPO="FlorentHosteur/mdui"
BIN_NAME="mdui"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        *)      echo "Unsupported OS: $os"; exit 1 ;;
    esac

    case "$arch" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)             echo "Unsupported architecture: $arch"; exit 1 ;;
    esac

    echo "${arch}-${os}"
}

install_from_release() {
    local target="$1"
    local latest_url="https://api.github.com/repos/${REPO}/releases/latest"

    echo "==> Detecting latest release..."
    local tag
    tag=$(curl -fsSL "$latest_url" 2>/dev/null | grep '"tag_name"' | sed 's/.*: "\(.*\)".*/\1/' || echo "")

    if [ -z "$tag" ]; then
        echo "No release found. Building from source instead."
        install_from_source
        return
    fi

    local archive_name="${BIN_NAME}-${tag}-${target}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/${tag}/${archive_name}"

    echo "==> Downloading ${archive_name}..."
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    if curl -fsSL -o "${tmp_dir}/${archive_name}" "$download_url" 2>/dev/null; then
        tar -xzf "${tmp_dir}/${archive_name}" -C "$tmp_dir"
        mkdir -p "$INSTALL_DIR"
        mv "${tmp_dir}/${BIN_NAME}" "$INSTALL_DIR/${BIN_NAME}"
        chmod +x "$INSTALL_DIR/${BIN_NAME}"
        echo "==> Installed ${BIN_NAME} to ${INSTALL_DIR}/${BIN_NAME}"
    else
        echo "Release binary not available for ${target}. Building from source."
        install_from_source
    fi
}

install_from_source() {
    if ! command -v cargo &>/dev/null; then
        echo "Rust is required. Install it from https://rustup.rs"
        exit 1
    fi

    echo "==> Building from source..."
    if [ -f "Cargo.toml" ] && grep -q 'name = "mdui"' Cargo.toml 2>/dev/null; then
        cargo install --path .
    else
        cargo install --git "https://github.com/${REPO}.git"
    fi
    echo "==> Installed via cargo install"
}

main() {
    local target
    target=$(detect_platform)
    echo "==> Platform: ${target}"

    install_from_release "$target"

    # Verify
    if command -v "$BIN_NAME" &>/dev/null; then
        echo "==> $($BIN_NAME --version)"
    else
        echo ""
        echo "NOTE: Make sure ${INSTALL_DIR} is in your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    fi
}

main
