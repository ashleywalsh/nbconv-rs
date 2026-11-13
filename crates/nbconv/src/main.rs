//! `nbconv` — convert Jupyter notebooks (.ipynb) to self-contained HTML.

use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "nbconv",
    about = "Convert a Jupyter notebook (.ipynb) to self-contained HTML",
    version
)]
struct Args {
    /// Input `.ipynb` file (omit when using --list-themes).
    /// Uses stdin if not provided.
    input: Option<PathBuf>,

    /// Write HTML to this file instead of stdout.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Syntect syntax-highlighting theme.
    ///
    /// Run with --list-themes to see available names.
    #[arg(long, default_value = "InspiredGitHub", value_name = "NAME")]
    theme: String,

    /// Disable `MathJax` injection for fully offline output.
    ///
    /// By default, a `MathJax` 3 CDN script is injected when any markdown cell
    /// contains LaTeX math.  This flag suppresses that entirely.
    #[arg(long)]
    no_mathjax: bool,

    /// Print available syntax-highlighting theme names and exit.
    #[arg(long)]
    list_themes: bool,
}

/// Enter a named profiling span until the end of the enclosing block.
/// Expands to nothing unless the `profiling` feature is enabled.
macro_rules! phase {
    ($name:literal) => {
        #[cfg(feature = "profiling")]
        let _phase = tracing::info_span!($name).entered();
    };
}

fn main() -> ExitCode {
    // Declared before `run` so it is dropped (flushing the trace file) after
    // `run` returns, on both success and failure.
    #[cfg(feature = "profiling")]
    let _trace_guard = init_tracing();

    match run(&Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

/// Install a Chrome-trace subscriber writing to `nbconv-trace.json` in the
/// current directory. The file is written when the returned guard is dropped.
#[cfg(feature = "profiling")]
fn init_tracing() -> tracing_chrome::FlushGuard {
    use tracing_subscriber::prelude::*;

    let (layer, guard) = tracing_chrome::ChromeLayerBuilder::new()
        .file("nbconv-trace.json")
        .build();
    tracing_subscriber::registry().with(layer).init();
    guard
}

fn run(args: &Args) -> Result<(), String> {
    phase!("main");

    // ── --list-themes ─────────────────────────────────────────────────────────
    if args.list_themes {
        for name in nbhtml::available_themes() {
            println!("{name}");
        }
        return Ok(());
    }

    // ── 1. Read source ───────────────────────────────────────────────────────
    // Use the input file when provided; otherwise read from stdin.
    let (json, title) = {
        phase!("read");
        if let Some(path) = &args.input {
            let json = std::fs::read_to_string(path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Notebook")
                .to_string();
            (json, title)
        } else {
            use std::io::Read as _;
            let mut json = String::new();
            std::io::stdin()
                .lock()
                .read_to_string(&mut json)
                .map_err(|e| format!("reading stdin: {e}"))?;
            (json, "Notebook".to_string())
        }
    };

    let notebook = nbformat::parse(&json).map_err(|e| {
        let src = args
            .input
            .as_ref()
            .map_or_else(|| "stdin".to_string(), |p| p.display().to_string());
        format!("failed to parse {src}: {e}")
    })?;

    // ── 2. Transform ──────────────────────────────────────────────────────────
    let title = title.as_str();

    let opts = nbhtml::HtmlOptions {
        theme: args.theme.clone(),
        mathjax: if args.no_mathjax { Some(false) } else { None },
    };

    let doc = nbhtml::from_notebook_with_options(&notebook, title, &opts);

    // ── 3. Render ─────────────────────────────────────────────────────────────
    let html = doc.render();

    // ── 4. Write ──────────────────────────────────────────────────────────────
    phase!("write");
    match &args.output {
        Some(path) => {
            std::fs::write(path, html.as_bytes())
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            #[allow(clippy::cast_precision_loss)] // file size display: precision loss is acceptable
            let kib = html.len() as f64 / 1024.0;
            eprintln!(
                "wrote {} ({kib:.1} KiB, theme: {}{})",
                path.display(),
                args.theme,
                if args.no_mathjax { ", no-mathjax" } else { "" },
            );
        }
        None => {
            std::io::stdout()
                .lock()
                .write_all(html.as_bytes())
                .map_err(|e| format!("write to stdout: {e}"))?;
        }
    }

    Ok(())
}
