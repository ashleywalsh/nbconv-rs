//! Serialise an [`HtmlDocument`] to a complete, self-contained HTML page.

use maud::{html, Markup, PreEscaped, DOCTYPE};

use crate::{CellInput, CellKind, CellOutput, HtmlCell, HtmlDocument};

const CSS: &str = "\
*, *::before, *::after { box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Helvetica, Arial, sans-serif;
  font-size: 14px; line-height: 1.6; color: #212121;
  background: #ffffff; margin: 0; padding: 0;
}
.jp-Notebook { max-width: 980px; margin: 0 auto; padding: 1.5rem 2rem 4rem; }
.jp-Cell { margin-bottom: 0.5rem; }
.jp-Cell-inputWrapper { display: flex; align-items: flex-start; }
.jp-InputPrompt, .jp-OutputPrompt {
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 12px; min-width: 7rem; text-align: right;
  padding: 5px 0.5rem 0 0; flex-shrink: 0; user-select: none;
}
.jp-InputPrompt { color: #303f9f; }
.jp-OutputPrompt { color: #e65100; }
.jp-InputArea-editor { flex: 1; min-width: 0; }
.jp-InputArea-editor pre {
  margin: 0; padding: 0.5rem 0.75rem;
  background: #f8f8f8; border: 1px solid #e0e0e0; border-radius: 2px;
  overflow-x: auto;
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 13px; line-height: 1.5; white-space: pre;
}
.jp-Cell-outputWrapper { margin-top: 0; }
.jp-OutputArea-child { display: flex; align-items: flex-start; }
.jp-OutputArea-output { flex: 1; min-width: 0; padding: 0.15rem 0; }
.jp-RenderedText pre {
  margin: 0; padding: 0; background: none; border: none;
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 13px; line-height: 1.5; white-space: pre-wrap; word-break: break-word;
}
.jp-OutputArea-output.stderr .jp-RenderedText pre {
  background: #fff8e1; border: 1px solid #ffe082;
  border-radius: 2px; padding: 0.4rem 0.5rem;
}
.jp-RenderedImage img { max-width: 100%; height: auto; display: block; }
.jp-RenderedHTML { overflow-x: auto; font-size: 13px; }
.jp-RenderedHTML table { border-collapse: collapse; margin: 0.25rem 0; }
.jp-RenderedHTML th, .jp-RenderedHTML td {
  border: 1px solid #d0d0d0; padding: 3px 8px; text-align: left; vertical-align: top;
}
.jp-RenderedHTML th { background: #f0f0f0; font-weight: 600; }
.jp-RenderedHTML tr:nth-child(even) { background: #fafafa; }
.jp-RenderedSVG { overflow-x: auto; }
.jp-RenderedSVG svg { max-width: 100%; height: auto; }
.jp-error .jp-evalue {
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 13px; color: #c62828; font-weight: bold; margin-bottom: 0.25rem;
}
.jp-error .jp-traceback {
  margin: 0; padding: 0.5rem 0.75rem;
  background: #fff5f5; border: 1px solid #ffcdd2; border-radius: 2px;
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 12px; line-height: 1.5; white-space: pre-wrap; word-break: break-all;
}
.jp-RawCell { padding: 0.25rem 0 0.25rem 7.5rem; }
.jp-RawCell pre { color: #666; font-size: 13px; white-space: pre-wrap; }
/* ── Markdown ─────────────────────────────────────────────────────────────── */
.jp-MarkdownCell { padding: 0.3rem 0 0.3rem 7.5rem; }
.jp-RenderedMarkdown h1, .jp-RenderedMarkdown h2, .jp-RenderedMarkdown h3,
.jp-RenderedMarkdown h4, .jp-RenderedMarkdown h5, .jp-RenderedMarkdown h6 {
  margin: 0.75rem 0 0.35rem; font-weight: 600; line-height: 1.3;
}
.jp-RenderedMarkdown h1 { font-size: 1.75em; border-bottom: 1px solid #e0e0e0; padding-bottom: 0.2em; }
.jp-RenderedMarkdown h2 { font-size: 1.4em;  border-bottom: 1px solid #eeeeee; padding-bottom: 0.15em; }
.jp-RenderedMarkdown h3 { font-size: 1.15em; }
.jp-RenderedMarkdown p  { margin: 0.4rem 0; }
.jp-RenderedMarkdown ul, .jp-RenderedMarkdown ol { margin: 0.4rem 0; padding-left: 1.8rem; }
.jp-RenderedMarkdown li { margin: 0.15rem 0; }
.jp-RenderedMarkdown code {
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 0.9em; background: #f3f3f3; border-radius: 2px; padding: 1px 4px;
}
.jp-RenderedMarkdown pre {
  background: #f8f8f8; border: 1px solid #e0e0e0; border-radius: 2px;
  padding: 0.5rem 0.75rem; overflow-x: auto;
  font-family: \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace;
  font-size: 13px; line-height: 1.5; white-space: pre;
}
.jp-RenderedMarkdown pre code { background: none; padding: 0; font-size: inherit; }
.jp-RenderedMarkdown blockquote {
  margin: 0.5rem 0; padding: 0.3rem 0.8rem; border-left: 4px solid #ccc; color: #555;
}
.jp-RenderedMarkdown img  { max-width: 100%; height: auto; }
.jp-RenderedMarkdown a    { color: #0366d6; text-decoration: none; }
.jp-RenderedMarkdown a:hover { text-decoration: underline; }
.jp-RenderedMarkdown hr   { border: none; border-top: 1px solid #ddd; margin: 1rem 0; }
.jp-RenderedMarkdown table { border-collapse: collapse; margin: 0.4rem 0; font-size: 13px; }
.jp-RenderedMarkdown th, .jp-RenderedMarkdown td { border: 1px solid #ccc; padding: 4px 10px; }
.jp-RenderedMarkdown th { background: #f0f0f0; font-weight: 600; }
.math-display { overflow-x: auto; text-align: center; margin: 0.75rem 0; }
";

const MATHJAX_CONFIG: &str =
    r#"MathJax = { tex: { inlineMath: [['\\(','\\)']], displayMath: [['\\[','\\]']] } };"#;
const MATHJAX_CDN: &str = "https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-chtml.js";

impl HtmlDocument {
    pub fn render(&self) -> String {
        html! {
            (DOCTYPE)
            html lang=(self.lang) {
                head {
                    meta charset="utf-8";
                    meta name="viewport" content="width=device-width, initial-scale=1";
                    title { (self.title) }
                    style { (PreEscaped(CSS)) }
                    @if self.inject_ansi_css {
                        style { (PreEscaped(crate::ansi::CSS)) }
                    }
                    @if self.inject_mathjax {
                        script { (PreEscaped(MATHJAX_CONFIG)) }
                        script type="text/javascript" id="MathJax-script"
                            "async"="" src=(MATHJAX_CDN) {}
                    }
                }
                body {
                    div class="jp-Notebook"
                        "data-jp-theme-light"="true"
                        "data-jp-theme-name"="JupyterLab Light"
                    {
                        @for cell in &self.cells {
                            (render_cell(cell))
                        }
                    }
                }
            }
        }
        .into_string()
    }
}

fn render_cell(cell: &HtmlCell) -> Markup {
    match cell.kind {
        CellKind::Code => render_code_cell(cell),
        CellKind::Markdown => render_markdown_cell(cell),
        CellKind::Raw => render_raw_cell(cell),
    }
}

fn render_code_cell(cell: &HtmlCell) -> Markup {
    html! {
        div class="jp-Cell jp-CodeCell jp-Notebook-cell" {
            div class="jp-Cell-inputWrapper" {
                @match &cell.input {
                    CellInput::Source { text, prompt, highlighted } => {
                        div class="jp-InputPrompt jp-InputArea-prompt" {
                            (in_prompt(*prompt))
                        }
                        div class="jp-InputArea-editor" {
                            pre {
                                @if let Some(html) = highlighted {
                                    (PreEscaped(html))  // syntect inline styles — not re-escaped
                                } @else {
                                    (text)
                                }
                            }
                        }
                    }
                    CellInput::Empty => {
                        div class="jp-InputPrompt jp-InputArea-prompt" {
                            (PreEscaped("In&nbsp;[&nbsp;]:"))
                        }
                        div class="jp-InputArea-editor" { pre {} }
                    }
                }
            }
            @if !cell.outputs.is_empty() {
                div class="jp-Cell-outputWrapper" {
                    div class="jp-OutputArea jp-Cell-outputArea" {
                        @for output in &cell.outputs {
                            (render_output(output))
                        }
                    }
                }
            }
        }
    }
}

fn render_markdown_cell(cell: &HtmlCell) -> Markup {
    let content = match &cell.input {
        CellInput::Source { text, .. } => {
            Some(crate::markdown::render(text, cell.attachments.as_ref()))
        }
        CellInput::Empty => None,
    };
    html! {
        div class="jp-Cell jp-MarkdownCell jp-Notebook-cell" {
            div class="jp-MarkdownCell jp-RenderedHTMLCommon jp-RenderedMarkdown" {
                @if let Some(rendered) = content {
                    (PreEscaped(rendered))
                }
            }
        }
    }
}

fn render_raw_cell(cell: &HtmlCell) -> Markup {
    let text = match &cell.input {
        CellInput::Source { text, .. } => Some(text.as_str()),
        CellInput::Empty => None,
    };
    html! {
        div class="jp-Cell jp-RawCell jp-Notebook-cell" {
            div class="jp-RawCell" {
                @if let Some(text) = text {
                    pre { (text) }
                }
            }
        }
    }
}

fn render_output(output: &CellOutput) -> Markup {
    match output {
        CellOutput::Stream { html, stderr } => {
            let cls = if *stderr { "jp-OutputArea-output stderr" } else { "jp-OutputArea-output" };
            html! {
                div class="jp-OutputArea-child" {
                    div class="jp-OutputPrompt jp-OutputArea-prompt" {}
                    div class=(cls) {
                        div class="jp-RenderedText" {
                            pre { (PreEscaped(html)) }  // ANSI-processed HTML — not re-escaped
                        }
                    }
                }
            }
        }
        CellOutput::Text {
            html,
            execution_count,
        } => html! {
            div class="jp-OutputArea-child" {
                div class="jp-OutputPrompt jp-OutputArea-prompt" { (out_prompt(*execution_count)) }
                div class="jp-OutputArea-output jp-RenderedText" {
                    pre { (PreEscaped(html)) }  // ANSI-processed HTML — not re-escaped
                }
            }
        },
        CellOutput::Html {
            html,
            execution_count,
        } => html! {
            div class="jp-OutputArea-child" {
                div class="jp-OutputPrompt jp-OutputArea-prompt" { (out_prompt(*execution_count)) }
                div class="jp-OutputArea-output jp-RenderedHTML" {
                    (PreEscaped(html))
                }
            }
        },
        CellOutput::Image {
            mime,
            data,
            execution_count,
        } => {
            let src = format!("data:{mime};base64,{data}");
            html! {
                div class="jp-OutputArea-child" {
                    div class="jp-OutputPrompt jp-OutputArea-prompt" { (out_prompt(*execution_count)) }
                    div class="jp-OutputArea-output jp-RenderedImage" {
                        img src=(src) alt="" {}
                    }
                }
            }
        }
        CellOutput::Svg {
            svg,
            execution_count,
        } => html! {
            div class="jp-OutputArea-child" {
                div class="jp-OutputPrompt jp-OutputArea-prompt" { (out_prompt(*execution_count)) }
                div class="jp-OutputArea-output jp-RenderedSVG" {
                    (PreEscaped(svg))
                }
            }
        },
        CellOutput::Error {
            ename,
            evalue,
            traceback_html,
        } => html! {
            div class="jp-OutputArea-child" {
                div class="jp-OutputPrompt jp-OutputArea-prompt" {}
                div class="jp-OutputArea-output jp-RenderedText jp-error" {
                    div class="jp-evalue" { (ename) ": " (evalue) }  // auto-escaped
                    pre class="jp-traceback" {
                        (PreEscaped(traceback_html))  // ANSI-processed HTML — not re-escaped
                    }
                }
            }
        },
    }
}

/// Render an input prompt, e.g. `In [1]:`. Contains `&nbsp;` entities so must
/// be wrapped in [`PreEscaped`].
fn in_prompt(n: Option<u32>) -> Markup {
    match n {
        Some(n) => PreEscaped(format!("In&nbsp;[{n}]:")),
        None => PreEscaped("In&nbsp;[&nbsp;]:".to_string()),
    }
}

/// Render an output prompt, e.g. `Out[1]:`, or nothing for `None`.
fn out_prompt(n: Option<u32>) -> Markup {
    match n {
        Some(n) => PreEscaped(format!("Out[{n}]:")),
        None => PreEscaped(String::new()),
    }
}
