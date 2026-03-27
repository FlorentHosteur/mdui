# Installation

## From source (recommended)

Requires [Rust](https://rustup.rs) 1.75+.

### Using cargo install

```bash
# Directly from GitHub
cargo install --git https://github.com/FlorentHosteur/mdui.git

# Or clone and install locally
git clone https://github.com/FlorentHosteur/mdui.git
cd mdui
cargo install --path .
```

This installs the binary to `~/.cargo/bin/mdui`. Make sure `~/.cargo/bin` is in your `PATH`.

### Manual build

```bash
git clone https://github.com/FlorentHosteur/mdui.git
cd mdui
cargo build --release
```

Then copy the binary wherever you like:

```bash
# To ~/.local/bin
cp target/release/mdui ~/.local/bin/

# Or system-wide
sudo cp target/release/mdui /usr/local/bin/
```

## From pre-built binaries

Download the archive for your platform from [GitHub Releases](https://github.com/FlorentHosteur/mdui/releases), then extract and install:

```bash
# Example for macOS arm64
tar xzf mdui-v0.1.0-aarch64-apple-darwin.tar.gz
chmod +x mdui
mv mdui ~/.local/bin/
```

Available platforms:

| Platform | Archive name |
|----------|-------------|
| macOS arm64 (Apple Silicon) | `mdui-v*-aarch64-apple-darwin.tar.gz` |
| Linux amd64 | `mdui-v*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux arm64 | `mdui-v*-aarch64-unknown-linux-gnu.tar.gz` |

## One-line installer script

```bash
curl -fsSL https://raw.githubusercontent.com/FlorentHosteur/mdui/main/scripts/install.sh | bash
```

This script auto-detects your platform, downloads the release binary if available, or falls back to building from source. By default it installs to `~/.local/bin`. Override with:

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/FlorentHosteur/mdui/main/scripts/install.sh | bash
```

## Uninstall

```bash
# If installed via cargo
cargo uninstall mdui

# If installed manually
rm $(which mdui)
```

## Verify

```bash
mdui --version
mdui --help
```
