//! Transform a parsed [`Notebook`] into a typed [`HtmlDocument`].
//!
//! See [`HtmlOptions`] for configuration (theme, MathJax override) and
//! [`from_notebook_with_options`] for the full API.

pub(crate) mod ansi;
pub(crate) mod highlight;
pub(crate) mod markdown;
mod render;

use std::collections::HashMap;

use nbformat::{Cell, MimeBundle, MimeData, Notebook, Output, StreamName};

const DISPLAY_PRIORITY: &[&str] = &[
    "text/html",
    "image/svg+xml",
    "image/png",
    "image/jpeg",
    "text/plain",
];

// ── Configuration ─────────────────────────────────────────────────────────────

/// Options that control how a notebook is converted to HTML.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Syntect theme name for code-cell highlighting (default: `"InspiredGitHub"`).
    ///
    /// Use [`available_themes`] to enumerate valid names.
    pub theme: String,
    /// Override MathJax injection.
    ///
    /// * `None`         – auto-detect: inject only when a markdown cell
    ///                    contains `$`, `\[`, or `\(`.
    /// * `Some(true)`   – always inject (force online math rendering).
    /// * `Some(false)`  – never inject (fully offline output).
    pub mathjax: Option<bool>,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        HtmlOptions {
            theme: "InspiredGitHub".to_string(),
            mathjax: None,
        }
    }
}

/// Return the names of all available syntax-highlighting themes, sorted.
pub fn available_themes() -> Vec<String> {
    highlight::available_themes()
}

// ── Public types ──────────────────────────────────────────────────────────────

pub type Attachments = HashMap<String, HashMap<String, String>>;

#[derive(Debug)]
pub struct HtmlDocument {
    pub title: String,
    pub lang: String,
    pub language: String,
    pub cells: Vec<HtmlCell>,
    pub inject_mathjax: bool,
    pub inject_ansi_css: bool,
}

#[derive(Debug)]
pub struct HtmlCell {
    pub kind: CellKind,
    pub input: CellInput,
    pub outputs: Vec<CellOutput>,
    pub attachments: Option<Attachments>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellKind { Code, Markdown, Raw }

#[derive(Debug)]
pub enum CellInput {
    Source {
        text: String,
        prompt: Option<u32>,
        highlighted: Option<String>,
    },
    Empty,
}

#[derive(Debug)]
pub enum CellOutput {
    Stream       { html: String, stderr: bool },
    Text         { html: String, execution_count: Option<u32> },
    Html         { html: String, execution_count: Option<u32> },
    Image        { mime: String, data: String, execution_count: Option<u32> },
    Svg          { svg: String, execution_count: Option<u32> },
    Error        { ename: String, evalue: String, traceback_html: String },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Convert a notebook using default options (InspiredGitHub theme, auto MathJax).
pub fn from_notebook(nb: &Notebook) -> HtmlDocument {
    let title = nb.metadata.title.as_deref().unwrap_or("Notebook").to_string();
    build(nb, title, &HtmlOptions::default())
}

/// Like [`from_notebook`] but with an explicit page title.
pub fn from_notebook_titled(nb: &Notebook, title: impl Into<String>) -> HtmlDocument {
    build(nb, title.into(), &HtmlOptions::default())
}

/// Full API: explicit title + configuration.
pub fn from_notebook_with_options(
    nb: &Notebook,
    title: impl Into<String>,
    opts: &HtmlOptions,
) -> HtmlDocument {
    build(nb, title.into(), opts)
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn build(nb: &Notebook, title: String, opts: &HtmlOptions) -> HtmlDocument {
    let language = nb
        .metadata.language_info.as_ref()
        .map(|li| li.name.clone())
        .unwrap_or_else(|| "python".to_string());

    let inject_mathjax = match opts.mathjax {
        Some(v) => v,
        None    => nb.cells.iter().any(|c| match c {
            Cell::Markdown(m) => markdown::has_math(&m.source),
            _ => false,
        }),
    };

    let inject_ansi_css = nb.cells.iter().any(|c| match c {
        Cell::Code(code) => !code.outputs.is_empty(),
        _ => false,
    });

    let cells = nb.cells.iter()
        .map(|c| transform_cell(c, &language, &opts.theme))
        .collect();

    HtmlDocument { title, lang: "en".to_string(), language, cells, inject_mathjax, inject_ansi_css }
}

fn transform_cell(cell: &Cell, language: &str, theme: &str) -> HtmlCell {
    match cell {
        Cell::Code(c) => HtmlCell {
            kind: CellKind::Code,
            input: if c.source.is_empty() { CellInput::Empty } else {
                CellInput::Source {
                    highlighted: Some(highlight::highlight(&c.source, language, theme)),
                    text: c.source.clone(),
                    prompt: c.execution_count,
                }
            },
            outputs: c.outputs.iter().filter_map(transform_output).collect(),
            attachments: None,
        },
        Cell::Markdown(c) => HtmlCell {
            kind: CellKind::Markdown,
            input: if c.source.is_empty() { CellInput::Empty } else {
                CellInput::Source { text: c.source.clone(), prompt: None, highlighted: None }
            },
            outputs: vec![],
            attachments: c.attachments.clone(),
        },
        Cell::Raw(c) => HtmlCell {
            kind: CellKind::Raw,
            input: if c.source.is_empty() { CellInput::Empty } else {
                CellInput::Source { text: c.source.clone(), prompt: None, highlighted: None }
            },
            outputs: vec![],
            attachments: c.attachments.clone(),
        },
    }
}

fn transform_output(output: &Output) -> Option<CellOutput> {
    match output {
        Output::Stream(s) => Some(CellOutput::Stream {
            html:   ansi::to_html(&s.text),
            stderr: s.name == StreamName::Stderr,
        }),
        Output::DisplayData(d)   => transform_mime(&d.data, None),
        Output::ExecuteResult(e) => transform_mime(&e.data, Some(e.execution_count)),
        Output::Error(e) => {
            let traceback_html: String = e.traceback.iter()
                .map(|l| ansi::to_html(l))
                .collect();
            Some(CellOutput::Error {
                ename: e.ename.clone(),
                evalue: e.evalue.clone(),
                traceback_html,
            })
        }
    }
}

fn transform_mime(bundle: &MimeBundle, ec: Option<u32>) -> Option<CellOutput> {
    for &mime in DISPLAY_PRIORITY {
        if let Some(data) = bundle.get(mime) {
            match (mime, data) {
                ("text/html",     MimeData::String(h)) => return Some(CellOutput::Html { html: h.clone(), execution_count: ec }),
                ("image/svg+xml", MimeData::String(s)) => return Some(CellOutput::Svg  { svg:  s.clone(), execution_count: ec }),
                ("image/png",  MimeData::String(r))
                | ("image/jpeg", MimeData::String(r)) => {
                    let data: String = r.chars().filter(|c| !c.is_whitespace()).collect();
                    return Some(CellOutput::Image { mime: mime.to_string(), data, execution_count: ec });
                }
                ("text/plain", MimeData::String(t)) => {
                    let html = ansi::to_html(t);
                    if html.is_empty() { return None; }
                    return Some(CellOutput::Text { html, execution_count: ec });
                }
                _ => {}
            }
        }
    }
    None
}
