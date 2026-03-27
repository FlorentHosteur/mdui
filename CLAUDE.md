# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is mdui

A Rust CLI tool that renders Markdown files beautifully in the terminal with colored output, Unicode box-drawing tables, language-specific code block icons, and emoji support.

## Build & Run

```bash
cargo build              # dev build
cargo build --release    # optimized release (~1MB binary, LTO + strip)
cargo run -- file.md     # render a file
echo "# Hi" | cargo run  # pipe from stdin
```

## Architecture

Two source files in `src/`:

- **`main.rs`** — CLI entry point using `clap` derive. Reads file arg or stdin, passes content to renderer.
- **`render.rs`** — The core rendering engine. Single `Renderer` struct that walks `pulldown-cmark` events and writes ANSI-styled output via `crossterm`.

### Rendering approach

The renderer uses a single-pass event loop over `pulldown_cmark::Event` with three collection modes that intercept events before the main `match`:

1. **Table mode** — When `Start(Table)` is seen, all events are buffered into a `TableState` (headers, rows, cells) until `End(Table)`, then the complete table is rendered with Unicode box-drawing characters and column alignment.
2. **Heading mode** — Heading text is collected into a buffer, then rendered with colored borders and icons at `End(Heading)`.
3. **Code block mode** — Code content is accumulated until `End(CodeBlock)`, then rendered in a bordered box with a language-specific emoji icon.

For lists and blockquotes, text is buffered in `buf` and flushed at `End(Item)` or `End(Paragraph)` boundaries. The `flush_list_item` helper handles bullet rendering, task list checkboxes, indentation, and word wrapping.

Inline styles (bold/italic/strikethrough) use targeted ANSI attribute toggles (`Bold`/`NoBold`) rather than full reset, so they compose correctly.

### Key crates

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | CommonMark parser with GFM extensions (tables, strikethrough, tasklists, footnotes) |
| `crossterm` | Cross-platform terminal styling (colors, attributes) |
| `clap` | CLI argument parsing |
| `terminal_size` | Terminal width detection (capped at 120 cols) |
| `textwrap` | Unicode-aware word wrapping |
| `unicode-width` | Display width calculation for alignment |

## Adding a new language icon

Add a pattern to the `lang_to_icon()` function in `render.rs`. It maps code fence language tags to emoji strings.
