mod pager;
mod render;

use clap::Parser;
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

/// mdui — Render Markdown beautifully in your terminal
#[derive(Parser)]
#[command(name = "mdui", version, about)]
struct Cli {
    /// Markdown file to render (reads from stdin if omitted)
    file: Option<PathBuf>,

    /// Open in interactive pager mode (scrollable, searchable)
    #[arg(short, long)]
    pager: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let markdown = match cli.file {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: file not found: {}", path.display());
                std::process::exit(1);
            }
            fs::read_to_string(&path).map_err(|e| {
                eprintln!("Error reading {}: {}", path.display(), e);
                e
            })?
        }
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let renderer = render::Renderer::new();

    // Use pager if requested, or auto-detect when output is large and terminal is interactive
    if cli.pager && io::stdout().is_terminal() {
        let rendered = renderer.render_to_bytes(&markdown)?;
        pager::run(&rendered)?;
    } else {
        renderer.render(&markdown)?;
    }

    Ok(())
}
