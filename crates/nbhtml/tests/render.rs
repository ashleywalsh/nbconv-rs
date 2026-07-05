//! Integration tests: parse real notebooks, convert to HTML, check output.

use nbformat::parse;
use nbhtml::from_notebook_titled;

fn fixture(name: &str) -> String {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("nbformat/tests/fixtures")
        .join(name);
    std::fs::read_to_string(&base)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", base.display()))
}

fn rendered(name: &str) -> String {
    from_notebook_titled(&parse(&fixture(name)).unwrap(), "Test").render()
}

// ── Structure ─────────────────────────────────────────────────────────────────
#[test] fn html_starts_with_doctype()  { assert!(rendered("notebook1.ipynb").starts_with("<!DOCTYPE html>")); }
#[test] fn html_has_title_element()    { assert!(rendered("notebook1.ipynb").contains("<title>Test</title>")); }
#[test] fn html_has_notebook_wrapper() { assert!(rendered("notebook1.ipynb").contains("jp-Notebook")); }
#[test] fn html_ends_properly()        { assert!(rendered("notebook1.ipynb").contains("</html>")); }

// ── Code cells ────────────────────────────────────────────────────────────────
#[test]
fn code_source_appears_in_output() {
    assert!(rendered("notebook3_kernelspec.ipynb").contains("input"));
}
#[test] fn input_prompt_present()           { assert!(rendered("notebook3_kernelspec.ipynb").contains("In&nbsp;[1]:")); }
#[test] fn output_prompt_for_execute_result(){ assert!(rendered("notebook3_kernelspec.ipynb").contains("Out[1]:")); }

// ── M3: Syntax highlighting ───────────────────────────────────────────────────
#[test]
fn code_cells_have_syntax_highlighting() {
    assert!(rendered("notebook3_kernelspec.ipynb").contains("<span style=\""));
}
#[test]
fn highlighted_source_text_preserved() {
    let html = rendered("notebook3_kernelspec.ipynb");
    assert!(html.contains("input") && html.contains("test input"));
}
#[test]
fn multi_cell_notebook_all_highlighted() {
    let n = rendered("notebook1.ipynb").matches("<span style=\"").count();
    assert!(n > 5, "expected multiple highlighted spans, got {n}");
}

// ── Images ────────────────────────────────────────────────────────────────────
#[test] fn png_output_becomes_data_uri() { assert!(rendered("notebook1.ipynb").contains("data:image/png;base64,")); }
#[test] fn jpeg_output_becomes_data_uri(){ assert!(rendered("notebook4_jpeg.ipynb").contains("data:image/jpeg;base64,")); }
#[test]
fn png_data_has_no_whitespace() {
    let html  = rendered("notebook1.ipynb");
    let start = html.find("data:image/png;base64,").unwrap();
    let rest  = &html[start + "data:image/png;base64,".len()..];
    let b64   = &rest[..rest.find('"').unwrap_or(rest.len())];
    assert!(!b64.contains('\n') && !b64.contains(' '));
}

// ── Stream outputs ────────────────────────────────────────────────────────────
#[test]
fn stream_text_present() { assert!(rendered("notebook3_kernelspec.ipynb").contains("input value")); }

// ── M4: Markdown ─────────────────────────────────────────────────────────────
#[test] fn markdown_heading_h1()          { assert!(rendered("notebook1.ipynb").contains("<h1>")); }
#[test] fn markdown_cells_present()       { assert!(rendered("notebook1.ipynb").contains("jp-MarkdownCell")); }
#[test] fn markdown_raw_placeholder_gone(){ assert!(!rendered("notebook1.ipynb").contains("jp-MarkdownCell-source")); }
#[test] fn attachment_renders_as_data_uri(){ assert!(rendered("attachment.ipynb").contains("data:image/png;base64,")); }
#[test] fn math_inline_delimiters()       { let h = rendered("math_markdown.ipynb"); assert!(h.contains("\\(") && h.contains("\\)")); }
#[test] fn math_display_delimiters()      { let h = rendered("math_markdown.ipynb"); assert!(h.contains("\\[") && h.contains("\\]")); }
#[test] fn mathjax_injected_for_math()    { let h = rendered("math_markdown.ipynb"); assert!(h.contains("MathJax") && h.contains("mathjax@3")); }
#[test] fn mathjax_absent_for_code_only() { assert!(!rendered("notebook3_kernelspec.ipynb").contains("MathJax")); }

// ── M5: ANSI → HTML ───────────────────────────────────────────────────────────

#[test]
fn ansi_stream_output_red_colored() {
    let html = rendered("ansi_output.ipynb");
    // "\x1b[31mred text\x1b[0m" → <span class="ansi-red-fg">red text</span>
    assert!(html.contains("ansi-red-fg"), "ANSI red fg class missing");
    assert!(html.contains("red text"),    "colored text content missing");
}

#[test]
fn ansi_normal_text_after_reset_present() {
    let html = rendered("ansi_output.ipynb");
    assert!(html.contains("and normal"), "plain text after ANSI reset missing");
}

#[test]
fn ansi_bold_stream_output() {
    let html = rendered("ansi_output.ipynb");
    // "\x1b[1mbold text\x1b[0m" → <span class="ansi-bold">bold text</span>
    assert!(html.contains("ansi-bold"), "bold ANSI class missing");
    assert!(html.contains("bold text"), "bold text content missing");
}

#[test]
fn ansi_css_injected_when_outputs_present() {
    let html = rendered("ansi_output.ipynb");
    assert!(html.contains(".ansi-red-fg"), "ANSI red CSS rule missing");
    assert!(html.contains(".ansi-bold"),   "ANSI bold CSS rule missing");
}

#[test]
fn ansi_css_absent_for_no_output_notebooks() {
    // notebook3_with_errors has no outputs (unexecuted cells)
    let html = rendered("notebook3_with_errors.ipynb");
    assert!(!html.contains(".ansi-red-fg"), "ANSI CSS should not appear");
}

#[test]
fn ansi_error_traceback_has_color() {
    let html = rendered("ansi_output.ipynb");
    assert!(html.contains("ZeroDivisionError"), "error name missing");
    assert!(html.contains("ansi-red-fg"),       "traceback ANSI color missing");
}

#[test]
fn ansi_no_raw_escape_bytes_in_output() {
    // ESC characters must never appear in the rendered HTML
    assert!(!rendered("ansi_output.ipynb").contains('\x1b'));
}
