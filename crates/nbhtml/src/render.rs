//! Serialise an [`HtmlDocument`] to a complete, self-contained HTML page.

use std::fmt::Write as FmtWrite;

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
        let mut out = String::with_capacity(64 * 1024);

        out.push_str("<!DOCTYPE html>\n");
        write!(out, "<html lang=\"{}\">\n", he(&self.lang)).unwrap();
        out.push_str("<head>\n");
        out.push_str("  <meta charset=\"utf-8\">\n");
        out.push_str(
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
        );
        write!(out, "  <title>{}</title>\n", he(&self.title)).unwrap();
        out.push_str("  <style>\n");
        out.push_str(CSS);
        out.push_str("  </style>\n");

        if self.inject_ansi_css {
            out.push_str("  <style>\n");
            out.push_str(crate::ansi::CSS);
            out.push_str("  </style>\n");
        }

        if self.inject_mathjax {
            out.push_str("  <script>\n  ");
            out.push_str(MATHJAX_CONFIG);
            out.push_str("\n  </script>\n");
            write!(
                out,
                "  <script type=\"text/javascript\" id=\"MathJax-script\" async src=\"{}\"></script>\n",
                MATHJAX_CDN
            ).unwrap();
        }

        out.push_str("</head>\n<body>\n");
        out.push_str(
            "<div class=\"jp-Notebook\" data-jp-theme-light=\"true\" \
             data-jp-theme-name=\"JupyterLab Light\">\n",
        );
        for cell in &self.cells {
            render_cell(cell, &mut out);
        }
        out.push_str("</div>\n</body>\n</html>\n");
        out
    }
}

fn render_cell(cell: &HtmlCell, out: &mut String) {
    match cell.kind {
        CellKind::Code => render_code_cell(cell, out),
        CellKind::Markdown => render_markdown_cell(cell, out),
        CellKind::Raw => render_raw_cell(cell, out),
    }
}

fn render_code_cell(cell: &HtmlCell, out: &mut String) {
    out.push_str("<div class=\"jp-Cell jp-CodeCell jp-Notebook-cell\">\n");
    out.push_str("<div class=\"jp-Cell-inputWrapper\">\n");
    match &cell.input {
        CellInput::Source {
            text,
            prompt,
            highlighted,
        } => {
            write!(
                out,
                "<div class=\"jp-InputPrompt jp-InputArea-prompt\">{}</div>\n",
                fmt_in_prompt(*prompt)
            )
            .unwrap();
            out.push_str("<div class=\"jp-InputArea-editor\">\n<pre>");
            match highlighted {
                Some(html) => out.push_str(html), // syntect inline styles — not re-escaped
                None => out.push_str(&he(text)),
            }
            out.push_str("</pre>\n</div>\n");
        }
        CellInput::Empty => {
            out.push_str(
                "<div class=\"jp-InputPrompt jp-InputArea-prompt\">In&nbsp;[&nbsp;]:</div>\n",
            );
            out.push_str("<div class=\"jp-InputArea-editor\"><pre></pre></div>\n");
        }
    }
    out.push_str("</div>\n"); // inputWrapper

    if !cell.outputs.is_empty() {
        out.push_str("<div class=\"jp-Cell-outputWrapper\">\n");
        out.push_str("<div class=\"jp-OutputArea jp-Cell-outputArea\">\n");
        for output in &cell.outputs {
            render_output(output, out);
        }
        out.push_str("</div>\n</div>\n");
    }
    out.push_str("</div>\n"); // Cell
}

fn render_markdown_cell(cell: &HtmlCell, out: &mut String) {
    out.push_str("<div class=\"jp-Cell jp-MarkdownCell jp-Notebook-cell\">\n");
    out.push_str("<div class=\"jp-MarkdownCell jp-RenderedHTMLCommon jp-RenderedMarkdown\">\n");
    if let CellInput::Source { text, .. } = &cell.input {
        let html = crate::markdown::render(text, cell.attachments.as_ref());
        out.push_str(&html);
    }
    out.push_str("</div>\n</div>\n");
}

fn render_raw_cell(cell: &HtmlCell, out: &mut String) {
    out.push_str("<div class=\"jp-Cell jp-RawCell jp-Notebook-cell\">\n");
    out.push_str("<div class=\"jp-RawCell\">\n");
    if let CellInput::Source { text, .. } = &cell.input {
        write!(out, "<pre>{}</pre>\n", he(text)).unwrap();
    }
    out.push_str("</div>\n</div>\n");
}

fn render_output(output: &CellOutput, out: &mut String) {
    match output {
        CellOutput::Stream { html, stderr } => {
            let extra = if *stderr { " stderr" } else { "" };
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\"></div>\n\
                 <div class=\"jp-OutputArea-output{extra}\">\
                 <div class=\"jp-RenderedText\"><pre>",
                extra = extra,
            )
            .unwrap();
            out.push_str(html); // ANSI-processed HTML — not re-escaped
            out.push_str("</pre></div></div>\n</div>\n");
        }
        CellOutput::Text {
            html,
            execution_count,
        } => {
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\">{prompt}</div>\n\
                 <div class=\"jp-OutputArea-output jp-RenderedText\"><pre>",
                prompt = fmt_out_prompt(*execution_count),
            )
            .unwrap();
            out.push_str(html); // ANSI-processed HTML — not re-escaped
            out.push_str("</pre></div>\n</div>\n");
        }
        CellOutput::Html {
            html,
            execution_count,
        } => {
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\">{prompt}</div>\n\
                 <div class=\"jp-OutputArea-output jp-RenderedHTML\">\n",
                prompt = fmt_out_prompt(*execution_count),
            )
            .unwrap();
            out.push_str(html);
            out.push_str("\n</div>\n</div>\n");
        }
        CellOutput::Image {
            mime,
            data,
            execution_count,
        } => {
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\">{prompt}</div>\n\
                 <div class=\"jp-OutputArea-output jp-RenderedImage\">\
                 <img src=\"data:{mime};base64,{data}\" alt=\"\">\
                 </div>\n\
                 </div>\n",
                prompt = fmt_out_prompt(*execution_count),
                mime = mime,
                data = data,
            )
            .unwrap();
        }
        CellOutput::Svg {
            svg,
            execution_count,
        } => {
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\">{prompt}</div>\n\
                 <div class=\"jp-OutputArea-output jp-RenderedSVG\">\n",
                prompt = fmt_out_prompt(*execution_count),
            )
            .unwrap();
            out.push_str(svg);
            out.push_str("\n</div>\n</div>\n");
        }
        CellOutput::Error {
            ename,
            evalue,
            traceback_html,
        } => {
            write!(
                out,
                "<div class=\"jp-OutputArea-child\">\n\
                 <div class=\"jp-OutputPrompt jp-OutputArea-prompt\"></div>\n\
                 <div class=\"jp-OutputArea-output jp-RenderedText jp-error\">\n\
                 <div class=\"jp-evalue\">{ename}: {evalue}</div>\n\
                 <pre class=\"jp-traceback\">",
                ename = he(ename),
                evalue = he(evalue),
            )
            .unwrap();
            out.push_str(traceback_html); // ANSI-processed HTML — not re-escaped
            out.push_str("</pre>\n</div>\n</div>\n");
        }
    }
}

fn he(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

fn fmt_in_prompt(n: Option<u32>) -> String {
    match n {
        Some(n) => format!("In&nbsp;[{n}]:"),
        None => "In&nbsp;[&nbsp;]:".to_string(),
    }
}

fn fmt_out_prompt(n: Option<u32>) -> String {
    match n {
        Some(n) => format!("Out[{n}]:"),
        None => String::new(),
    }
}
