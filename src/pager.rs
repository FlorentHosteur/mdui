use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::io::{self, Write};
use std::time::Duration;

pub fn run(rendered: &[u8]) -> io::Result<()> {
    let content = String::from_utf8_lossy(rendered);
    let lines: Vec<&str> = content.lines().collect();

    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Clear(ClearType::All))?;

    let result = pager_loop(&mut stdout, &lines);

    // Always clean up
    execute!(stdout, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn pager_loop(stdout: &mut io::Stdout, lines: &[&str]) -> io::Result<()> {
    let mut offset: usize = 0;
    let mut search_query = String::new();
    let mut search_matches: Vec<usize> = Vec::new();
    let mut current_match: usize = 0;
    let mut in_search = false;
    let mut search_input = String::new();

    draw(stdout, lines, offset, &search_query, &search_matches, current_match)?;

    loop {
        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let (_, term_h) = terminal::size()?;
        let viewport = (term_h as usize).saturating_sub(1); // reserve 1 for status bar
        let max_offset = lines.len().saturating_sub(viewport);

        match event::read()? {
            Event::Key(key) if in_search => {
                match key.code {
                    KeyCode::Enter => {
                        in_search = false;
                        search_query = search_input.clone();
                        search_input.clear();
                        // Find all matching lines (strip ANSI for comparison)
                        search_matches.clear();
                        let query_lower = search_query.to_lowercase();
                        for (i, line) in lines.iter().enumerate() {
                            let plain = strip_ansi(line);
                            if plain.to_lowercase().contains(&query_lower) {
                                search_matches.push(i);
                            }
                        }
                        current_match = 0;
                        // Jump to first match
                        if let Some(&line_idx) = search_matches.first() {
                            offset = line_idx.min(max_offset);
                        }
                    }
                    KeyCode::Esc => {
                        in_search = false;
                        search_input.clear();
                    }
                    KeyCode::Backspace => {
                        search_input.pop();
                    }
                    KeyCode::Char(c) => {
                        search_input.push(c);
                    }
                    _ => {}
                }
                draw_search_bar(stdout, &search_input, term_h)?;
                continue;
            }
            Event::Key(KeyEvent { code, modifiers, .. }) => match code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => break,

                // Scroll
                KeyCode::Down | KeyCode::Char('j') => {
                    offset = (offset + 1).min(max_offset);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    offset = offset.saturating_sub(1);
                }
                KeyCode::PageDown | KeyCode::Char(' ') => {
                    offset = (offset + viewport).min(max_offset);
                }
                KeyCode::PageUp | KeyCode::Char('b') => {
                    offset = offset.saturating_sub(viewport);
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    offset = 0;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    offset = max_offset;
                }

                // Half page
                KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                    offset = (offset + viewport / 2).min(max_offset);
                }
                KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => {
                    offset = offset.saturating_sub(viewport / 2);
                }

                // Search
                KeyCode::Char('/') => {
                    in_search = true;
                    search_input.clear();
                    draw_search_bar(stdout, &search_input, term_h)?;
                    continue;
                }
                KeyCode::Char('n') => {
                    // Next match
                    if !search_matches.is_empty() {
                        current_match = (current_match + 1) % search_matches.len();
                        offset = search_matches[current_match].min(max_offset);
                    }
                }
                KeyCode::Char('N') => {
                    // Previous match
                    if !search_matches.is_empty() {
                        current_match = if current_match == 0 {
                            search_matches.len() - 1
                        } else {
                            current_match - 1
                        };
                        offset = search_matches[current_match].min(max_offset);
                    }
                }

                _ => {}
            },
            Event::Resize(_, _) => {
                // Redraw on resize
            }
            _ => continue,
        }

        draw(stdout, lines, offset, &search_query, &search_matches, current_match)?;
    }

    Ok(())
}

fn draw(
    stdout: &mut io::Stdout,
    lines: &[&str],
    offset: usize,
    search_query: &str,
    search_matches: &[usize],
    current_match: usize,
) -> io::Result<()> {
    let (term_w, term_h) = terminal::size()?;
    let viewport = (term_h as usize).saturating_sub(1);

    execute!(stdout, BeginSynchronizedUpdate)?;

    // Draw content lines
    for row in 0..viewport {
        execute!(stdout, MoveTo(0, row as u16))?;
        let line_idx = offset + row;
        if line_idx < lines.len() {
            write!(stdout, "{}", lines[line_idx])?;
        }
        // Clear rest of line
        write!(stdout, "\x1B[K")?;
    }

    // Status bar
    execute!(stdout, MoveTo(0, term_h - 1))?;
    write!(
        stdout,
        "{}{}",
        SetBackgroundColor(Color::DarkGrey),
        SetForegroundColor(Color::White),
    )?;

    let pct = if lines.is_empty() {
        100
    } else {
        ((offset + viewport).min(lines.len()) * 100) / lines.len()
    };

    let search_info = if !search_query.is_empty() && !search_matches.is_empty() {
        format!(
            " │ \"{}\" {}/{}",
            search_query,
            current_match + 1,
            search_matches.len()
        )
    } else if !search_query.is_empty() {
        format!(" │ \"{}\" no match", search_query)
    } else {
        String::new()
    };

    let status = format!(
        " {}/{} ({}%){} │ q:quit ↑↓:scroll /:search ",
        (offset + 1).min(lines.len()),
        lines.len(),
        pct,
        search_info,
    );

    // Pad to full width
    let padded = format!("{:<width$}", status, width = term_w as usize);
    write!(stdout, "{}", padded)?;
    write!(stdout, "{}", SetAttribute(Attribute::Reset))?;

    execute!(stdout, EndSynchronizedUpdate)?;
    stdout.flush()
}

fn draw_search_bar(stdout: &mut io::Stdout, input: &str, term_h: u16) -> io::Result<()> {
    execute!(stdout, MoveTo(0, term_h - 1))?;
    write!(
        stdout,
        "{}{}",
        SetBackgroundColor(Color::DarkYellow),
        SetForegroundColor(Color::Black),
    )?;
    write!(stdout, " /{}\x1B[K", input)?;
    write!(stdout, "{}", SetAttribute(Attribute::Reset))?;
    stdout.flush()
}

/// Strip ANSI escape sequences for search matching
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1B' {
            // Skip until we find the terminating letter
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() || c == 'm' {
                        break;
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}
