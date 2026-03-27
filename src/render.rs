use crossterm::style::{Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::io::{self, Write};
use unicode_width::UnicodeWidthStr;

const HEADING_COLORS: [Color; 6] = [
    Color::Magenta,
    Color::Blue,
    Color::Cyan,
    Color::Green,
    Color::Yellow,
    Color::Red,
];

const HEADING_ICONS: [&str; 6] = ["◉", "◈", "◆", "▸", "▪", "▫"];

const LIST_BULLETS: [&str; 4] = ["●", "○", "■", "□"];

pub struct Renderer {
    term_width: usize,
}

/// Table being collected from events before rendering
struct TableState {
    alignments: Vec<Alignment>,
    head_cells: Vec<String>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    in_head: bool,
}

impl TableState {
    fn new(alignments: Vec<Alignment>) -> Self {
        Self {
            alignments,
            head_cells: Vec::new(),
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            in_head: false,
        }
    }
}

/// Track nested list state
struct ListCtx {
    ordered: bool,
    index: u64,
    depth: usize,
}

impl Renderer {
    pub fn new() -> Self {
        let width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        Self {
            term_width: width.min(120),
        }
    }

    pub fn render_to_bytes(&self, markdown: &str) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.render_to(&mut buf, markdown)?;
        Ok(buf)
    }

    pub fn render(&self, markdown: &str) -> io::Result<()> {
        let mut out = io::stdout().lock();
        self.render_to(&mut out, markdown)
    }

    fn render_to(&self, mut out: &mut (impl Write + ?Sized), markdown: &str) -> io::Result<()> {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_FOOTNOTES);

        let parser = Parser::new_ext(markdown, opts);
        let events: Vec<Event> = parser.collect();

        let mut table: Option<TableState> = None;
        let mut lists: Vec<ListCtx> = Vec::new();
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        let mut bq_depth: usize = 0;
        // Buffer for collecting plain text (lists, blockquotes, headings)
        let mut buf = String::new();
        let mut in_heading = false;
        let mut heading_level: u8 = 1;
        // Track whether we're in a context that buffers text
        let mut in_code_block = false;
        // Track link destination to show after link text
        let mut link_url: Option<String> = None;

        for event in events {
            // ── Table collection mode ──
            if let Some(ref mut ts) = table {
                match event {
                    Event::Start(Tag::TableHead) => { ts.in_head = true; }
                    Event::End(TagEnd::TableHead) => {
                        ts.in_head = false;
                        ts.head_cells = ts.current_row.drain(..).collect();
                    }
                    Event::Start(Tag::TableRow) => { ts.current_row.clear(); }
                    Event::End(TagEnd::TableRow) => {
                        ts.rows.push(ts.current_row.drain(..).collect());
                    }
                    Event::Start(Tag::TableCell) => { ts.current_cell.clear(); }
                    Event::End(TagEnd::TableCell) => {
                        ts.current_row.push(ts.current_cell.clone());
                    }
                    Event::Text(ref text) => { ts.current_cell.push_str(text); }
                    Event::Code(ref code) => {
                        ts.current_cell.push('`');
                        ts.current_cell.push_str(code);
                        ts.current_cell.push('`');
                    }
                    Event::SoftBreak | Event::HardBreak => { ts.current_cell.push(' '); }
                    Event::End(TagEnd::Table) => {
                        let ts = table.take().unwrap();
                        self.render_table(&mut out, &ts)?;
                        writeln!(out)?;
                    }
                    _ => {}
                }
                continue;
            }

            // ── Heading collection mode ──
            if in_heading {
                match event {
                    Event::Text(ref text) => { buf.push_str(text); continue; }
                    Event::Code(ref code) => { buf.push_str(code); continue; }
                    Event::SoftBreak => { buf.push(' '); continue; }
                    Event::End(TagEnd::Heading { .. }) => {
                        self.render_heading(&mut out, heading_level, &buf)?;
                        buf.clear();
                        in_heading = false;
                        continue;
                    }
                    _ => continue,
                }
            }

            // ── Code block collection mode ──
            if in_code_block {
                match event {
                    Event::Text(ref text) => { code_block_content.push_str(text); continue; }
                    Event::End(TagEnd::CodeBlock) => {
                        in_code_block = false;
                        self.render_code_block(&mut out, &code_block_lang, &code_block_content)?;
                        writeln!(out)?;
                        continue;
                    }
                    _ => continue,
                }
            }

            // Are we buffering text? (inside list items or blockquotes)
            let buffering = !lists.is_empty() || bq_depth > 0;

            match event {
                // ── Headings ──
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = level as u8;
                    buf.clear();
                }

                // ── Paragraphs ──
                Event::Start(Tag::Paragraph) => {}
                Event::End(TagEnd::Paragraph) => {
                    if !buf.is_empty() {
                        if bq_depth > 0 {
                            self.render_block_quote_line(&mut out, &buf, bq_depth)?;
                        } else if !lists.is_empty() {
                            // Don't flush here — Item end will handle it
                        } else {
                            self.render_wrapped(&mut out, &buf, "")?;
                        }
                        if lists.is_empty() {
                            buf.clear();
                        }
                    }
                    if lists.is_empty() {
                        writeln!(out)?;
                    }
                }

                // ── Emphasis / Strong / Strikethrough ──
                Event::Start(Tag::Emphasis) => {
                    if buffering {
                        // No ANSI in buffer — we keep it plain
                    } else {
                        write!(out, "{}", SetAttribute(Attribute::Italic))?;
                    }
                }
                Event::End(TagEnd::Emphasis) => {
                    if !buffering {
                        write!(out, "{}", SetAttribute(Attribute::NoItalic))?;
                    }
                }
                Event::Start(Tag::Strong) => {
                    if buffering {
                    } else {
                        write!(out, "{}", SetAttribute(Attribute::Bold))?;
                    }
                }
                Event::End(TagEnd::Strong) => {
                    if !buffering {
                        write!(out, "{}", SetAttribute(Attribute::NormalIntensity))?;
                    }
                }
                Event::Start(Tag::Strikethrough) => {
                    if buffering {
                    } else {
                        write!(out, "{}", SetAttribute(Attribute::CrossedOut))?;
                    }
                }
                Event::End(TagEnd::Strikethrough) => {
                    if !buffering {
                        write!(out, "{}", SetAttribute(Attribute::NotCrossedOut))?;
                    }
                }

                // ── Inline code ──
                Event::Code(code) => {
                    if buffering {
                        buf.push('`');
                        buf.push_str(&code);
                        buf.push('`');
                    } else {
                        write!(
                            out,
                            "{}{}{} {} {}",
                            SetBackgroundColor(Color::DarkGrey),
                            SetForegroundColor(Color::Yellow),
                            SetAttribute(Attribute::Bold),
                            code,
                            SetAttribute(Attribute::Reset),
                        )?;
                    }
                }

                // ── Code blocks ──
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_content.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                }

                // ── Block quotes ──
                Event::Start(Tag::BlockQuote(_)) => { bq_depth += 1; }
                Event::End(TagEnd::BlockQuote(_)) => {
                    bq_depth = bq_depth.saturating_sub(1);
                }

                // ── Lists ──
                Event::Start(Tag::List(first_number)) => {
                    // Flush any buffered text from a parent item before starting nested list
                    if !buf.is_empty() && !lists.is_empty() {
                        self.flush_list_item(&mut out, &mut lists, &mut buf)?;
                    }
                    let depth = lists.len();
                    lists.push(ListCtx {
                        ordered: first_number.is_some(),
                        index: first_number.unwrap_or(1),
                        depth,
                    });
                }
                Event::End(TagEnd::List(_)) => {
                    lists.pop();
                    if lists.is_empty() {
                        writeln!(out)?;
                    }
                }
                Event::Start(Tag::Item) => {
                    // Don't clear buf — it may already have been populated by
                    // paragraph text inside a previous item. We clear after flushing.
                }
                Event::End(TagEnd::Item) => {
                    self.flush_list_item(&mut out, &mut lists, &mut buf)?;
                }

                // ── Tables ──
                Event::Start(Tag::Table(alignments)) => {
                    table = Some(TableState::new(alignments));
                }

                // ── Horizontal rule ──
                Event::Rule => {
                    let rule_width = self.term_width.min(60);
                    write!(out, "{}", SetForegroundColor(Color::DarkGrey))?;
                    writeln!(out, "{}", "─".repeat(rule_width))?;
                    write!(out, "{}", SetAttribute(Attribute::Reset))?;
                    writeln!(out)?;
                }

                // ── Links ──
                Event::Start(Tag::Link { dest_url, .. }) => {
                    link_url = Some(dest_url.to_string());
                    if !buffering {
                        write!(out, "{}{}", SetForegroundColor(Color::Blue), SetAttribute(Attribute::Underlined))?;
                    }
                }
                Event::End(TagEnd::Link) => {
                    if let Some(url) = link_url.take() {
                        if !buffering {
                            write!(out, "{}", SetAttribute(Attribute::Reset))?;
                            write!(out, "{} ({}){}", SetForegroundColor(Color::DarkGrey), url, SetAttribute(Attribute::Reset))?;
                        } else {
                            buf.push_str(&format!(" ({})", url));
                        }
                    }
                }

                // ── Images ──
                Event::Start(Tag::Image { dest_url, .. }) => {
                    if buffering {
                        buf.push_str("🖼  ");
                    } else {
                        write!(out, "{}🖼  ", SetForegroundColor(Color::DarkYellow))?;
                    }
                    let _ = dest_url;
                }
                Event::End(TagEnd::Image) => {
                    if !buffering {
                        write!(out, "{}", SetAttribute(Attribute::Reset))?;
                    }
                }

                // ── Footnotes ──
                Event::Start(Tag::FootnoteDefinition(label)) => {
                    write!(out, "{}[{}]: ", SetForegroundColor(Color::DarkGrey), label)?;
                }
                Event::End(TagEnd::FootnoteDefinition) => {
                    write!(out, "{}", SetAttribute(Attribute::Reset))?;
                    writeln!(out)?;
                }
                Event::FootnoteReference(label) => {
                    if buffering {
                        buf.push_str(&format!("[{}]", label));
                    } else {
                        write!(out, "{}[{}]{}", SetForegroundColor(Color::DarkCyan), label, SetAttribute(Attribute::Reset))?;
                    }
                }

                // ── Task list markers ──
                Event::TaskListMarker(checked) => {
                    if checked {
                        buf.insert_str(0, "[x] ");
                    } else {
                        buf.insert_str(0, "[ ] ");
                    }
                }

                // ── Text ──
                Event::Text(text) => {
                    if buffering {
                        buf.push_str(&text);
                    } else {
                        write!(out, "{}", text)?;
                    }
                }

                Event::SoftBreak => {
                    if buffering {
                        buf.push(' ');
                    } else {
                        write!(out, " ")?;
                    }
                }
                Event::HardBreak => {
                    if buffering {
                        if bq_depth > 0 && !buf.is_empty() {
                            self.render_block_quote_line(&mut out, &buf, bq_depth)?;
                            buf.clear();
                        } else {
                            buf.push('\n');
                        }
                    } else {
                        writeln!(out)?;
                    }
                }

                Event::Html(html) => {
                    write!(out, "{}{}{}", SetForegroundColor(Color::DarkGrey), html, SetAttribute(Attribute::Reset))?;
                }

                _ => {}
            }
        }

        // Flush anything remaining
        if !buf.is_empty() {
            writeln!(out, "{}", buf)?;
        }

        write!(out, "{}", SetAttribute(Attribute::Reset))?;
        out.flush()?;
        Ok(())
    }

    fn flush_list_item(&self, w: &mut impl Write, lists: &mut Vec<ListCtx>, buf: &mut String) -> io::Result<()> {
        let text = buf.trim().to_string();
        buf.clear();

        if text.is_empty() {
            return Ok(());
        }

        let Some(list) = lists.last_mut() else {
            return Ok(());
        };

        let indent = "  ".repeat(list.depth);
        let bullet = if list.ordered {
            let b = format!("{}.", list.index);
            list.index += 1;
            b
        } else {
            LIST_BULLETS[list.depth.min(LIST_BULLETS.len() - 1)].to_string()
        };

        // Task list checkbox
        let display = if text.starts_with("[x] ") || text.starts_with("[X] ") {
            format!("☑ {}", &text[4..])
        } else if text.starts_with("[ ] ") {
            format!("☐ {}", &text[4..])
        } else {
            text
        };

        let prefix = format!("{}{} ", indent, bullet);
        let prefix_width = UnicodeWidthStr::width(prefix.as_str());

        // Print colored bullet
        write!(w, "{}{}", SetForegroundColor(Color::DarkCyan), SetAttribute(Attribute::Bold))?;
        write!(w, "{}", prefix)?;
        write!(w, "{}", SetAttribute(Attribute::Reset))?;

        // Wrap text after the bullet
        let avail = self.term_width.saturating_sub(prefix_width + 1);
        if UnicodeWidthStr::width(display.as_str()) <= avail {
            writeln!(w, "{}", display)?;
        } else {
            let continuation = " ".repeat(prefix_width);
            let wrapped = textwrap::wrap(&display, textwrap::Options::new(avail).subsequent_indent(&continuation));
            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    writeln!(w, "{}", line)?;
                } else {
                    writeln!(w, "{}", line)?;
                }
            }
        }

        Ok(())
    }

    fn render_heading(&self, w: &mut impl Write, level: u8, text: &str) -> io::Result<()> {
        let idx = (level as usize).saturating_sub(1).min(5);
        let color = HEADING_COLORS[idx];
        let icon = HEADING_ICONS[idx];

        writeln!(w)?;
        write!(w, "{}{}", SetForegroundColor(color), SetAttribute(Attribute::Bold))?;

        let bar_width = (UnicodeWidthStr::width(text) + 4).min(self.term_width);
        if level <= 2 {
            writeln!(w, "{}", "━".repeat(bar_width))?;
        }

        writeln!(w, " {} {}", icon, text)?;

        if level <= 2 {
            writeln!(w, "{}", "━".repeat(bar_width))?;
        }

        write!(w, "{}", SetAttribute(Attribute::Reset))?;
        writeln!(w)?;
        Ok(())
    }

    fn render_code_block(&self, w: &mut impl Write, lang: &str, code: &str) -> io::Result<()> {
        let max_width = self.term_width.saturating_sub(4);

        // Header
        write!(w, "{}{}", SetForegroundColor(Color::DarkGrey), SetAttribute(Attribute::Bold))?;
        if !lang.is_empty() {
            let lang_icon = lang_to_icon(lang);
            let fill_len = max_width.saturating_sub(lang.len() + 7);
            writeln!(w, "┌─ {} {} ─{}", lang_icon, lang, "─".repeat(fill_len))?;
        } else {
            writeln!(w, "┌{}", "─".repeat(max_width))?;
        }
        write!(w, "{}", SetAttribute(Attribute::Reset))?;

        // Code lines
        let code = code.trim_end_matches('\n');
        for line in code.lines() {
            write!(w, "{}│ ", SetForegroundColor(Color::DarkGrey))?;
            write!(w, "{}", SetForegroundColor(Color::Green))?;
            if UnicodeWidthStr::width(line) > max_width.saturating_sub(2) {
                write!(w, "{}", truncate_to_width(line, max_width.saturating_sub(2)))?;
            } else {
                write!(w, "{}", line)?;
            }
            writeln!(w)?;
        }

        // Footer
        write!(w, "{}", SetForegroundColor(Color::DarkGrey))?;
        writeln!(w, "└{}", "─".repeat(max_width))?;
        write!(w, "{}", SetAttribute(Attribute::Reset))?;

        Ok(())
    }

    fn render_table(&self, w: &mut impl Write, ts: &TableState) -> io::Result<()> {
        let ncols = ts.alignments.len();
        if ncols == 0 {
            return Ok(());
        }

        // Calculate column widths from content
        let mut col_widths: Vec<usize> = vec![0; ncols];
        for (i, cell) in ts.head_cells.iter().enumerate() {
            if i < ncols {
                col_widths[i] = col_widths[i].max(UnicodeWidthStr::width(cell.as_str()));
            }
        }
        for row in &ts.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < ncols {
                    col_widths[i] = col_widths[i].max(UnicodeWidthStr::width(cell.as_str()));
                }
            }
        }

        // Minimum 3, max 40
        for cw in &mut col_widths {
            *cw = (*cw).max(3).min(40);
        }

        // Shrink if doesn't fit terminal
        let total: usize = col_widths.iter().sum::<usize>() + ncols * 3 + 1;
        if total > self.term_width {
            let excess = total - self.term_width;
            let shrink_each = (excess / ncols) + 1;
            for cw in &mut col_widths {
                *cw = (*cw).saturating_sub(shrink_each).max(3);
            }
        }

        let sep = |w: &mut dyn Write, l: &str, m: &str, r: &str, f: &str| -> io::Result<()> {
            write!(w, "{}{}", SetForegroundColor(Color::DarkGrey), l)?;
            for (i, &cw) in col_widths.iter().enumerate() {
                write!(w, "{}", f.repeat(cw + 2))?;
                if i < ncols - 1 { write!(w, "{}", m)?; }
            }
            writeln!(w, "{}{}", r, SetAttribute(Attribute::Reset))
        };

        let row_fn = |w: &mut dyn Write, cells: &[String], bold: bool| -> io::Result<()> {
            write!(w, "{}│", SetForegroundColor(Color::DarkGrey))?;
            for (i, cw) in col_widths.iter().enumerate() {
                let cell_text = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                let truncated = if UnicodeWidthStr::width(cell_text) > *cw {
                    truncate_to_width(cell_text, *cw)
                } else {
                    cell_text.to_string()
                };
                let display_w = UnicodeWidthStr::width(truncated.as_str());
                let pad = cw.saturating_sub(display_w);
                let (lp, rp) = match ts.alignments.get(i) {
                    Some(Alignment::Center) => (pad / 2, pad - pad / 2),
                    Some(Alignment::Right) => (pad, 0),
                    _ => (0, pad),
                };

                write!(w, "{}", SetAttribute(Attribute::Reset))?;
                if bold {
                    write!(w, "{}{}", SetAttribute(Attribute::Bold), SetForegroundColor(Color::White))?;
                }
                write!(w, " {}{}{} ", " ".repeat(lp), truncated, " ".repeat(rp))?;
                write!(w, "{}│", SetForegroundColor(Color::DarkGrey))?;
            }
            writeln!(w, "{}", SetAttribute(Attribute::Reset))
        };

        sep(w as &mut dyn Write, "┌", "┬", "┐", "─")?;
        row_fn(w as &mut dyn Write, &ts.head_cells, true)?;
        sep(w as &mut dyn Write, "├", "┼", "┤", "─")?;
        for (i, row) in ts.rows.iter().enumerate() {
            row_fn(w as &mut dyn Write, row, false)?;
            if i < ts.rows.len() - 1 {
                sep(w as &mut dyn Write, "├", "┼", "┤", "╌")?;
            }
        }
        sep(w as &mut dyn Write, "└", "┴", "┘", "─")?;

        Ok(())
    }

    fn render_block_quote_line(&self, w: &mut impl Write, text: &str, depth: usize) -> io::Result<()> {
        let prefix: String = (0..depth).map(|_| "▐ ").collect();
        let indent_w = UnicodeWidthStr::width(prefix.as_str()) + 2;
        let wrap_width = self.term_width.saturating_sub(indent_w);
        let lines = textwrap::wrap(text, wrap_width);

        for line in lines {
            write!(w, "  {}{}", SetForegroundColor(Color::DarkYellow), prefix)?;
            write!(w, "{}{}", SetForegroundColor(Color::White), SetAttribute(Attribute::Italic))?;
            writeln!(w, "{}", line)?;
        }
        write!(w, "{}", SetAttribute(Attribute::Reset))?;
        Ok(())
    }

    fn render_wrapped(&self, w: &mut impl Write, text: &str, continuation: &str) -> io::Result<()> {
        let width = self.term_width.saturating_sub(2);
        let opts = textwrap::Options::new(width).subsequent_indent(continuation);
        for line in textwrap::wrap(text, opts) {
            writeln!(w, "{}", line)?;
        }
        Ok(())
    }
}

fn lang_to_icon(lang: &str) -> &'static str {
    match lang {
        "rust" | "rs" => "🦀",
        "python" | "py" => "🐍",
        "javascript" | "js" => "📜",
        "typescript" | "ts" => "📘",
        "go" => "🐹",
        "bash" | "sh" | "shell" | "zsh" => "🐚",
        "sql" => "🗃️",
        "html" => "🌐",
        "css" => "🎨",
        "json" => "📋",
        "yaml" | "yml" | "toml" => "⚙️",
        "dockerfile" | "docker" => "🐳",
        "c" | "cpp" | "c++" => "⚡",
        "java" => "☕",
        "ruby" | "rb" => "💎",
        "swift" => "🐦",
        "kotlin" | "kt" => "🟣",
        "lua" => "🌙",
        "php" => "🐘",
        "r" => "📊",
        "markdown" | "md" => "📝",
        "svelte" => "🔶",
        "vue" => "💚",
        "elixir" | "ex" => "💧",
        "haskell" | "hs" => "λ",
        "zig" => "⚡",
        _ => "📄",
    }
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width < 2 {
        return "…".to_string();
    }
    let mut result = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max_width - 1 {
            result.push('…');
            break;
        }
        result.push(ch);
        w += cw;
    }
    result
}
