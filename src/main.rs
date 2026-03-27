mod render;

use clap::Parser;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

/// mdui — Render Markdown beautifully in your terminal
#[derive(Parser)]
#[command(name = "mdui", version, about)]
struct Cli {
    /// Markdown file to render (reads from stdin if omitted)
    file: Option<PathBuf>,
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
    renderer.render(&markdown)?;

    Ok(())
}
