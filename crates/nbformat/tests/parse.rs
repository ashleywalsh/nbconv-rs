//! Integration tests: parse real `.ipynb` fixture files and assert structure.

use nbformat::{parse, Cell, MimeData, Output, ParseError, StreamName};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

// ── notebook1.ipynb — SymPy example, nbformat 4.1 ────────────────────────────
// 10 cells: 4 markdown interleaved with 6 code cells.
// Code cells 6-8 have execute_result outputs with image/png + text/latex +
// text/plain MIME bundles.  Cell 9 is empty and unexecuted.

#[test]
fn notebook1_version() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    assert_eq!(nb.nbformat, 4);
    assert_eq!(nb.nbformat_minor, 1);
}

#[test]
fn notebook1_cell_count() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    assert_eq!(nb.cells.len(), 10);
}

#[test]
fn notebook1_cell_types() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    assert!(matches!(nb.cells[0], Cell::Markdown(_)));
    assert!(matches!(nb.cells[1], Cell::Markdown(_)));
    assert!(matches!(nb.cells[2], Cell::Code(_)));
    assert!(matches!(nb.cells[3], Cell::Markdown(_)));
    assert!(matches!(nb.cells[4], Cell::Code(_)));
    assert!(matches!(nb.cells[5], Cell::Markdown(_)));
    assert!(matches!(nb.cells[6], Cell::Code(_)));
    assert!(matches!(nb.cells[7], Cell::Code(_)));
    assert!(matches!(nb.cells[8], Cell::Code(_)));
    assert!(matches!(nb.cells[9], Cell::Code(_)));
}

#[test]
fn notebook1_source_joined_from_array() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    // Cell 2 stores source as an array of strings; they must be joined.
    let Cell::Code(c) = &nb.cells[2] else { panic!("expected Code cell at [2]") };
    assert!(c.source.contains("from sympy import *"), "source = {:?}", c.source);
    assert!(c.source.contains("init_printing()"), "source = {:?}", c.source);
    // Each array element already ends with \n, so the joined string has newlines.
    assert!(c.source.contains('\n'), "newlines should be preserved");
}

#[test]
fn notebook1_execution_count_integer_and_null() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    let Cell::Code(c) = &nb.cells[2] else { panic!() };
    assert_eq!(c.execution_count, Some(2));

    // Last cell is unexecuted.
    let Cell::Code(last) = &nb.cells[9] else { panic!() };
    assert_eq!(last.execution_count, None);
    assert_eq!(last.source, "");
    assert_eq!(last.outputs.len(), 0);
}

#[test]
fn notebook1_execute_result_mime_types() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    let Cell::Code(c) = &nb.cells[6] else { panic!() };
    assert_eq!(c.outputs.len(), 1);
    let Output::ExecuteResult(o) = &c.outputs[0] else {
        panic!("expected ExecuteResult, got something else")
    };
    assert_eq!(o.execution_count, 6);
    assert!(o.data.get("image/png").is_some(), "image/png missing from bundle");
    assert!(o.data.get("text/latex").is_some(), "text/latex missing from bundle");
    assert!(o.data.get("text/plain").is_some(), "text/plain missing from bundle");
}

#[test]
fn notebook1_text_mime_arrays_joined() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    let Cell::Code(c) = &nb.cells[6] else { panic!() };
    let Output::ExecuteResult(o) = &c.outputs[0] else { panic!() };

    // text/latex is stored as a single-element array in the fixture.
    let Some(MimeData::String(latex)) = o.data.get("text/latex") else {
        panic!("text/latex should be MimeData::String")
    };
    assert!(latex.contains("x^{2}"), "latex = {latex:?}");

    // text/plain is a two-element array; joined result must contain both parts.
    let Some(MimeData::String(plain)) = o.data.get("text/plain") else {
        panic!("text/plain should be MimeData::String")
    };
    assert!(plain.contains("sin"), "plain = {plain:?}");

    // image/png is a plain base64 string (not an array).
    let Some(MimeData::String(png)) = o.data.get("image/png") else {
        panic!("image/png should be MimeData::String")
    };
    assert!(!png.is_empty(), "base64 data must not be empty");
}

#[test]
fn notebook1_empty_notebook_metadata() {
    let nb = parse(&fixture("notebook1.ipynb")).expect("parse notebook1.ipynb");
    assert!(nb.metadata.kernelspec.is_none());
    assert!(nb.metadata.language_info.is_none());
}

// ── notebook3_with_errors.ipynb — unexecuted cells, nbformat 4.0 ─────────────
// 5 cells: 2 markdown + 3 code.  All code cells are unexecuted (no outputs).

#[test]
fn notebook3_errors_version() {
    let nb = parse(&fixture("notebook3_with_errors.ipynb")).unwrap();
    assert_eq!(nb.nbformat, 4);
    assert_eq!(nb.nbformat_minor, 0);
}

#[test]
fn notebook3_errors_cell_count() {
    let nb = parse(&fixture("notebook3_with_errors.ipynb")).unwrap();
    assert_eq!(nb.cells.len(), 5);
}

#[test]
fn notebook3_errors_code_cells_have_no_outputs() {
    let nb = parse(&fixture("notebook3_with_errors.ipynb")).unwrap();
    for cell in &nb.cells {
        if let Cell::Code(c) = cell {
            assert_eq!(c.execution_count, None, "cells should be unexecuted");
            assert_eq!(c.outputs.len(), 0, "unexecuted cells must have no outputs");
        }
    }
}

// ── notebook3_kernelspec.ipynb — kernelspec + stdin stream, nbformat 4.4 ─────
// 2 code cells.  Cell 0 has a stdin stream + execute_result.
// Notebook metadata contains kernelspec and language_info whose
// codemirror_mode is an *object* (not a plain string).

#[test]
fn notebook3_kernelspec_version() {
    let nb = parse(&fixture("notebook3_kernelspec.ipynb")).unwrap();
    assert_eq!(nb.nbformat, 4);
    assert_eq!(nb.nbformat_minor, 4);
}

#[test]
fn notebook3_kernelspec_fields() {
    let nb = parse(&fixture("notebook3_kernelspec.ipynb")).unwrap();
    let ks = nb.metadata.kernelspec.as_ref().expect("kernelspec missing");
    assert_eq!(ks.name, "python3");
    assert_eq!(ks.display_name, "Python 3");
}

#[test]
fn notebook3_language_info_codemirror_mode_is_object() {
    let nb = parse(&fixture("notebook3_kernelspec.ipynb")).unwrap();
    let li = nb.metadata.language_info.as_ref().expect("language_info missing");
    assert_eq!(li.name, "python");
    let cm = li.codemirror_mode.as_ref().expect("codemirror_mode missing");
    assert!(
        cm.is_object(),
        "codemirror_mode should be a JSON object, got {cm}"
    );
}

#[test]
fn notebook3_stream_output_unknown_name_preserved() {
    let nb = parse(&fixture("notebook3_kernelspec.ipynb")).unwrap();
    let Cell::Code(c) = &nb.cells[0] else { panic!() };
    // The first output is a "stdin" stream (unusual but valid in practice).
    let Output::Stream(s) = &c.outputs[0] else {
        panic!("expected Stream output, got {:?}", c.outputs[0])
    };
    assert!(
        matches!(&s.name, StreamName::Other(n) if n == "stdin"),
        "expected Other(\"stdin\"), got {:?}",
        s.name
    );
    assert!(s.text.contains("input value"), "text = {:?}", s.text);
}

#[test]
fn notebook3_execute_result_text_plain() {
    let nb = parse(&fixture("notebook3_kernelspec.ipynb")).unwrap();
    let Cell::Code(c) = &nb.cells[0] else { panic!() };
    let Output::ExecuteResult(o) = &c.outputs[1] else {
        panic!("expected ExecuteResult at index 1, got {:?}", c.outputs[1])
    };
    assert_eq!(o.execution_count, 1);
    let Some(MimeData::String(plain)) = o.data.get("text/plain") else {
        panic!("text/plain missing from execute_result")
    };
    assert!(plain.contains("input value"), "plain = {plain:?}");
}

// ── notebook4_jpeg.ipynb — JPEG image output ──────────────────────────────────

#[test]
fn notebook4_jpeg_output_present() {
    let nb = parse(&fixture("notebook4_jpeg.ipynb")).unwrap();
    let found = nb.cells.iter().any(|cell| {
        if let Cell::Code(c) = cell {
            c.outputs.iter().any(|o| match o {
                Output::ExecuteResult(er) => er.data.get("image/jpeg").is_some(),
                Output::DisplayData(dd) => dd.data.get("image/jpeg").is_some(),
                _ => false,
            })
        } else {
            false
        }
    });
    assert!(found, "expected at least one JPEG output in notebook4_jpeg");
}

#[test]
fn notebook4_jpeg_is_nonempty_string() {
    let nb = parse(&fixture("notebook4_jpeg.ipynb")).unwrap();
    for cell in &nb.cells {
        if let Cell::Code(c) = cell {
            for output in &c.outputs {
                let data = match output {
                    Output::ExecuteResult(er) => Some(&er.data),
                    Output::DisplayData(dd) => Some(&dd.data),
                    _ => None,
                };
                if let Some(bundle) = data {
                    if let Some(MimeData::String(b64)) = bundle.get("image/jpeg") {
                        assert!(!b64.is_empty(), "base64 JPEG data must not be empty");
                        return;
                    }
                }
            }
        }
    }
    panic!("no JPEG output found");
}

// ── attachment.ipynb — markdown cells with attachments ───────────────────────
// 2 markdown cells, both with an inline image.png attachment.

#[test]
fn attachment_cell_has_attachments() {
    let nb = parse(&fixture("attachment.ipynb")).expect("parse attachment.ipynb");
    let Cell::Markdown(c) = &nb.cells[0] else {
        panic!("cell[0] should be Markdown")
    };
    let att = c.attachments.as_ref().expect("attachments missing on cell[0]");
    assert!(att.contains_key("image.png"), "'image.png' key missing");
    let img = &att["image.png"];
    assert!(img.contains_key("image/png"), "image/png MIME missing");
    assert!(!img["image/png"].is_empty(), "base64 attachment data must not be empty");
}

// ── mixed_markdown.ipynb — smoke test ────────────────────────────────────────

#[test]
fn mixed_markdown_parses() {
    parse(&fixture("mixed_markdown.ipynb")).expect("parse mixed_markdown.ipynb");
}

// ── notebook_v3.ipynb — nbformat 3 migration ─────────────────────────────────
// 4 cells after migration: heading→markdown, 2 code, 1 markdown.

#[test]
fn parse_v3_upgrades_nbformat_to_4() {
    let nb = parse(&fixture("notebook_v3.ipynb")).expect("parse v3 notebook");
    assert_eq!(nb.nbformat, 4, "v3 must be upgraded to nbformat 4");
}

#[test]
fn parse_v3_correct_cell_count() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    assert_eq!(nb.cells.len(), 4, "expected 4 cells after migration");
}

#[test]
fn parse_v3_heading_converted_to_markdown_h1() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    let Cell::Markdown(m) = &nb.cells[0] else { panic!("cell[0] should be Markdown") };
    assert!(m.source.starts_with("# "),  "source = {:?}", m.source);
    assert!(m.source.contains("v3 Notebook"), "source = {:?}", m.source);
}

#[test]
fn parse_v3_code_cell_input_field_mapped() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    let Cell::Code(c) = &nb.cells[1] else { panic!("cell[1] should be Code") };
    assert!(c.source.contains("print"), "source = {:?}", c.source);
    assert_eq!(c.execution_count, Some(1));
}

#[test]
fn parse_v3_stream_output_migrated() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    let Cell::Code(c) = &nb.cells[1] else { panic!() };
    let Output::Stream(s) = &c.outputs[0] else { panic!("expected Stream") };
    assert_eq!(s.name, StreamName::Stdout);
    assert!(s.text.contains("hello"), "text = {:?}", s.text);
}

#[test]
fn parse_v3_pyout_becomes_execute_result() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    let Cell::Code(c) = &nb.cells[2] else { panic!() };
    let Output::ExecuteResult(o) = &c.outputs[0] else { panic!("expected ExecuteResult") };
    assert_eq!(o.execution_count, 2);
    assert!(o.data.get("text/plain").is_some(), "text/plain missing from pyout bundle");
}

#[test]
fn parse_v3_markdown_cell_preserved() {
    let nb = parse(&fixture("notebook_v3.ipynb")).unwrap();
    let Cell::Markdown(m) = &nb.cells[3] else { panic!("cell[3] should be Markdown") };
    assert!(m.source.contains("markdown"), "source = {:?}", m.source);
}

// ── v1 / v2 — explicit unsupported-version errors ────────────────────────────

#[test]
fn parse_v1_returns_unsupported_error() {
    let json = r#"{"nbformat": 1, "cells": []}"#;
    let err = parse(json).unwrap_err();
    assert!(
        matches!(err, ParseError::UnsupportedVersion(1)),
        "expected UnsupportedVersion(1), got {err}"
    );
    assert!(err.to_string().contains("v1"), "message should mention v1: {err}");
    assert!(err.to_string().contains("not supported"), "message: {err}");
}

#[test]
fn parse_v2_returns_unsupported_error() {
    let json = r#"{"nbformat": 2, "worksheets": []}"#;
    let err = parse(json).unwrap_err();
    assert!(
        matches!(err, ParseError::UnsupportedVersion(2)),
        "expected UnsupportedVersion(2), got {err}"
    );
    assert!(err.to_string().contains("v2"), "message should mention v2: {err}");
}

#[test]
fn parse_v3_still_succeeds() {
    parse(&fixture("notebook_v3.ipynb")).expect("v3 must be supported");
}

#[test]
fn parse_v4_still_succeeds() {
    parse(&fixture("notebook1.ipynb")).expect("v4 must be supported");
}
