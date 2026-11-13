//! Render Markdown to an HTML fragment using pulldown-cmark.
//!
//! Two Jupyter-specific concerns are handled on top of the `CommonMark` baseline:
//!
//! **Math** — `$...$` (inline) and `$$...$$` (display) are converted to the
//! `MathJax` 3 delimiters `\(...\)` / `\[...\]`.  The content is HTML-escaped so
//! that TeX expressions containing `<`, `>`, or `&` round-trip correctly.
//!
//! **Attachments** — `attachment:filename` image URLs are replaced with inline
//! `data:` URIs built from the cell's attachment map.

use std::collections::HashMap;

use pulldown_cmark::html::push_html;
use pulldown_cmark::{Event, Options, Parser};

// Type alias to avoid repeating the nested HashMap type everywhere.
type AttMap = HashMap<String, HashMap<String, String>>;

// ── Public API ────────────────────────────────────────────────────────────────

/// Render `source` Markdown to an HTML fragment.
///
/// `attachments` maps `filename → mime_type → base64_data` as stored on a
/// [`nbformat::MarkdownCell`].
#[cfg_attr(
    feature = "profiling",
    tracing::instrument(name = "markdown", skip_all)
)]
pub(crate) fn render(source: &str, attachments: Option<&AttMap>) -> String {
    let opts = Options::ENABLE_MATH
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TASKLISTS;

    // Pre-process: substitute attachment: URLs with data URIs in the source
    // before feeding to pulldown-cmark.  This avoids lifetime complications
    // when constructing new Tag::Image events inside a map() closure.
    let owned_src: String;
    let src: &str = match attachments {
        Some(att) if !att.is_empty() => {
            owned_src = substitute_attachments(source, att);
            &owned_src
        }
        _ => source,
    };

    // Collect events so the math-adapting closure does not need to outlive
    // the parser iterator.
    let events: Vec<Event<'_>> = Parser::new_ext(src, opts).map(adapt_math).collect();

    let mut html = String::with_capacity(src.len() * 2);
    push_html(&mut html, events.into_iter());
    html
}

/// Return `true` if `source` appears to contain LaTeX math.
///
/// Used by the transform step to decide whether to inject a `MathJax` script tag.
/// This is a conservative check: a single `$` or `\[` triggers injection even
/// if it is not actually a math delimiter.
pub(crate) fn has_math(source: &str) -> bool {
    source.contains('$') || source.contains("\\[") || source.contains("\\(")
}

// ── Math event adaptation ─────────────────────────────────────────────────────

fn adapt_math(event: Event<'_>) -> Event<'_> {
    match event {
        // $...$  →  \(...\)
        Event::InlineMath(content) => {
            let escaped = crate::html_esc(&content);
            Event::Html(format!("\\({escaped}\\)").into())
        }
        // $$...$$ or fenced math  →  \[...\]
        Event::DisplayMath(content) => {
            let escaped = crate::html_esc(&content);
            // Rust raw string so we do not have to escape the backslashes.
            Event::Html(format!(r#"<div class="math-display">\[{escaped}\]</div>"#).into())
        }
        other => other,
    }
}

// ── Attachment URL substitution ───────────────────────────────────────────────

fn substitute_attachments(source: &str, att: &AttMap) -> String {
    let mut result = source.to_owned();
    for (filename, mime_map) in att {
        let placeholder = format!("attachment:{filename}");
        if !result.contains(&placeholder) {
            continue;
        }
        if let Some(data_uri) = pick_data_uri(mime_map) {
            result = result.replace(&placeholder, &data_uri);
        }
    }
    result
}

fn pick_data_uri(mime_map: &HashMap<String, String>) -> Option<String> {
    for preferred in &["image/png", "image/jpeg", "image/gif", "image/svg+xml"] {
        if let Some(data) = mime_map.get(*preferred) {
            return Some(format!("data:{preferred};base64,{data}"));
        }
    }
    mime_map
        .iter()
        .next()
        .map(|(mime, data)| format!("data:{mime};base64,{data}"))
}

// ── HTML escape for math content ──────────────────────────────────────────────
// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_h1() {
        let html = render("# Hello", None);
        assert!(html.contains("<h1>Hello</h1>"), "html = {html:?}");
    }

    #[test]
    fn bold_and_italic() {
        let html = render("**bold** and *italic*", None);
        assert!(html.contains("<strong>bold</strong>"), "html = {html:?}");
        assert!(html.contains("<em>italic</em>"), "html = {html:?}");
    }

    #[test]
    fn inline_math_wrapped_in_parens() {
        let html = render("This is $x^2$ math", None);
        // \( and \) must appear in the output
        assert!(html.contains("\\(x^2\\)"), "html = {html:?}");
    }

    #[test]
    fn display_math_wrapped_in_brackets() {
        let html = render("$$E = mc^2$$", None);
        assert!(html.contains("\\[E = mc^2\\]"), "html = {html:?}");
    }

    #[test]
    fn math_lt_gt_html_escaped() {
        let html = render("$a < b$", None);
        assert!(html.contains("\\(a &lt; b\\)"), "html = {html:?}");
    }

    #[test]
    fn attachment_url_substituted() {
        let mut att = AttMap::new();
        let mut mime_map = HashMap::new();
        mime_map.insert("image/png".to_string(), "AAAA".to_string());
        att.insert("img.png".to_string(), mime_map);

        let html = render("![alt](attachment:img.png)", Some(&att));
        assert!(
            html.contains("data:image/png;base64,AAAA"),
            "attachment not resolved: {html:?}"
        );
    }

    #[test]
    fn has_math_dollar() {
        assert!(has_math("Inline $x$"), "single dollar should trigger");
        assert!(has_math("Display $$x$$"), "double dollar should trigger");
        assert!(!has_math("plain text"), "plain text should not trigger");
    }

    #[test]
    fn has_math_backslash_bracket() {
        assert!(has_math("\\[x\\]"), "backslash-bracket should trigger");
    }
}
