# mdui

A fast, single-binary CLI tool that renders Markdown beautifully in the terminal.

Built in Rust. No runtime dependencies.

## Features

- **Headings** (H1–H6) with colored borders and icons
- **Tables** with Unicode box-drawing, column alignment (left/center/right), and auto-sizing
- **Code blocks** with language-specific emoji icons (🦀 Rust, 🐍 Python, 🐚 Shell, ☕ Java, 🐳 Docker, and 20+ more)
- **Inline code** with highlighted background
- **Lists** — ordered, unordered (nested: `● ○ ■ □`), and task lists (`☑ ☐`)
- **Block quotes** with colored sidebar, nested support
- **Bold**, *italic*, ~~strikethrough~~ with composable ANSI styles
- **Links** with underlined text and URL display
- **Images** with 🖼 icon
- **Horizontal rules**, footnotes, raw HTML (dimmed)
- **Text wrapping** aware of terminal width (capped at 120 columns)
- **Interactive pager** (`-p`) with scrolling, search, and vim-style keybindings
- **Stdin support** — pipe markdown from other commands

## Quick Start

```bash
# Render a file
mdui README.md

# Pipe from another command
cat CHANGELOG.md | mdui

# Interactive pager mode
mdui -p README.md
```

## Pager Controls

| Key | Action |
|-----|--------|
| `↑`/`↓`, `j`/`k` | Scroll line by line |
| `PgUp`/`PgDn`, `Space`/`b` | Scroll by page |
| `Ctrl+d`/`Ctrl+u` | Half-page scroll |
| `g`/`G`, `Home`/`End` | Top / bottom |
| `/` | Search (type query, then Enter) |
| `n`/`N` | Next / previous search match |
| `q`, `Esc` | Quit |

## Installation

See [INSTALL.md](INSTALL.md) for all installation methods.

**Quick install (macOS / Linux):**

```bash
# Using cargo (recommended)
cargo install --git https://github.com/FlorentHosteur/mdui.git

# Or from a local clone
git clone https://github.com/FlorentHosteur/mdui.git
cd mdui
cargo install --path .
```

## Building from Source

Requires Rust 1.75+ (2024 edition).

```bash
git clone https://github.com/FlorentHosteur/mdui.git
cd mdui

# Development build
cargo build

# Optimized release build (~1MB binary with LTO + strip)
cargo build --release

# The binary is at target/release/mdui
./target/release/mdui --help
```

### Cross-compilation

Build for multiple platforms using the provided script:

```bash
./scripts/build.sh
```

This produces tarballs in `dist/` for:
- macOS arm64 (Apple Silicon)
- Linux amd64
- Linux arm64

Linux targets require either a cross-compilation toolchain or [`cross`](https://github.com/cross-rs/cross):

```bash
cargo install cross --git https://github.com/cross-rs/cross
./scripts/build.sh   # will use cross automatically if available
```

## Architecture

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point (clap). Reads file or stdin, routes to renderer or pager |
| `src/render.rs` | Core rendering engine. Walks pulldown-cmark events, writes ANSI output |
| `src/pager.rs` | Interactive pager. Crossterm raw mode, alternate screen, search |

The renderer uses a single-pass event loop with three collection modes (tables, headings, code blocks) that buffer events before rendering complete elements. Lists and blockquotes buffer text in a shared `buf` that flushes at item/paragraph boundaries.

### Dependencies

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | CommonMark + GFM parser (tables, strikethrough, tasklists, footnotes) |
| `crossterm` | Terminal styling, raw mode, alternate screen, key events |
| `clap` | CLI argument parsing |
| `terminal_size` | Terminal width detection |
| `textwrap` | Unicode-aware word wrapping |
| `unicode-width` | Display width calculation for CJK/emoji alignment |

## Supported Language Icons

Code fences display a language-specific icon:

| Language | Icon | Language | Icon |
|----------|------|----------|------|
| Rust | 🦀 | Python | 🐍 |
| JavaScript | 📜 | TypeScript | 📘 |
| Go | 🐹 | Bash/Shell | 🐚 |
| SQL | 🗃️ | HTML | 🌐 |
| CSS | 🎨 | JSON | 📋 |
| YAML/TOML | ⚙️ | Docker | 🐳 |
| C/C++ | ⚡ | Java | ☕ |
| Ruby | 💎 | Swift | 🐦 |
| Kotlin | 🟣 | Lua | 🌙 |
| PHP | 🐘 | R | 📊 |
| Markdown | 📝 | Svelte | 🔶 |
| Vue | 💚 | Elixir | 💧 |
| Haskell | λ | Other | 📄 |

To add a new icon, edit the `lang_to_icon()` function in `src/render.rs`.

## License

MIT
