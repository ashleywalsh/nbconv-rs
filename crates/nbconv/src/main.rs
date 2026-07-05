//! `nbconv` — convert Jupyter notebooks (.ipynb) to self-contained HTML.

use std::io::Write as IoWrite;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(
    name    = "nbconv",
    about   = "Convert a Jupyter notebook (.ipynb) to self-contained HTML",
    version
)]
struct Args {
    /// Input `.ipynb` file (omit when using --list-themes).
    input: Option<PathBuf>,

    /// Write HTML to this file instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Syntect syntax-highlighting theme.
    ///
    /// Run with --list-themes to see available names.
    #[arg(long, default_value = "InspiredGitHub", value_name = "NAME")]
    theme: String,

    /// Disable MathJax injection for fully offline output.
    ///
    /// By default, a MathJax 3 CDN script is injected when any markdown cell
    /// contains LaTeX math.  This flag suppresses that entirely.
    #[arg(long)]
    no_mathjax: bool,

    /// Print available syntax-highlighting theme names and exit.
    #[arg(long)]
    list_themes: bool,
}

fn main() {
    let args = Args::parse();

    // ── --list-themes ─────────────────────────────────────────────────────────
    if args.list_themes {
        for name in nbhtml::available_themes() {
            println!("{name}");
        }
        return;
    }

    // ── require input when not listing themes ─────────────────────────────────
    let input = match &args.input {
        Some(p) => p,
        None => {
            eprintln!("error: an input .ipynb file is required (or use --list-themes)");
            std::process::exit(1);
        }
    };

    // ── 1. Parse ──────────────────────────────────────────────────────────────
    let json = std::fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", input.display());
        std::process::exit(1);
    });

    let notebook = nbformat::parse(&json).unwrap_or_else(|e| {
        eprintln!("error: failed to parse {}: {e}", input.display());
        std::process::exit(1);
    });

    // ── 2. Transform ──────────────────────────────────────────────────────────
    let title = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Notebook");

    let opts = nbhtml::HtmlOptions {
        theme:   args.theme.clone(),
        mathjax: if args.no_mathjax { Some(false) } else { None },
    };

    let doc = nbhtml::from_notebook_with_options(&notebook, title, &opts);

    // ── 3. Render ─────────────────────────────────────────────────────────────
    let html = doc.render();

    // ── 4. Write ──────────────────────────────────────────────────────────────
    match &args.output {
        Some(path) => {
            std::fs::write(path, html.as_bytes()).unwrap_or_else(|e| {
                eprintln!("error: cannot write {}: {e}", path.display());
                std::process::exit(1);
            });
            eprintln!(
                "wrote {} ({:.1} KiB, theme: {}{})",
                path.display(),
                html.len() as f64 / 1024.0,
                args.theme,
                if args.no_mathjax { ", no-mathjax" } else { "" },
            );
        }
        None => {
            std::io::stdout()
                .lock()
                .write_all(html.as_bytes())
                .unwrap_or_else(|e| {
                    eprintln!("error: write to stdout: {e}");
                    std::process::exit(1);
                });
        }
    }
}
