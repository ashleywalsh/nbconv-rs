//! Migrate a parsed nbformat v3 `serde_json::Value` into a v4 [`Notebook`].
//!
//! v3 notebooks use `worksheets[].cells` instead of a top-level `cells`
//! array, name code cell sources `input` (not `source`), use `prompt_number`
//! instead of `execution_count`, and name output types `pyout`/`pyerr`
//! instead of `execute_result`/`error`.
//!
//! This module is deliberately lenient: missing or malformed fields are
//! silently substituted with empty defaults rather than returning errors.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde_json::Value;

use crate::de::join_multiline;
use crate::{
    Cell, CellMetadata, CodeCell, DisplayDataOutput, ErrorOutput, ExecuteResultOutput,
    MarkdownCell, MimeBundle, MimeData, Notebook, NotebookMetadata, Output, RawCell, StreamName,
    StreamOutput,
};

/// Convert an already-parsed v3 notebook `Value` into a v4 [`Notebook`].
#[cfg_attr(
    feature = "profiling",
    tracing::instrument(name = "migrate_v3", skip_all)
)]
pub(crate) fn from_value(v: &Value) -> Notebook {
    Notebook {
        nbformat: 4,
        nbformat_minor: 0,
        metadata: collect_metadata(v),
        cells: collect_cells(v),
    }
}

// ── Metadata ──────────────────────────────────────────────────────────────────

fn collect_metadata(v: &Value) -> NotebookMetadata {
    let meta = v.get("metadata");
    NotebookMetadata {
        kernelspec: meta
            .and_then(|m| m.get("kernelspec"))
            .and_then(|ks| serde_json::from_value(ks.clone()).ok()),
        language_info: meta
            .and_then(|m| m.get("language_info"))
            .and_then(|li| serde_json::from_value(li.clone()).ok()),
        title: None,
        authors: None,
        extra: HashMap::new(),
    }
}

// ── Cells ─────────────────────────────────────────────────────────────────────

fn collect_cells(v: &Value) -> Vec<Cell> {
    v.get("worksheets")
        .and_then(|ws| ws.as_array())
        .into_iter()
        .flatten()
        .flat_map(|ws| {
            ws.get("cells")
                .and_then(|c| c.as_array())
                .into_iter()
                .flatten()
                .filter_map(migrate_cell)
        })
        .collect()
}

fn migrate_cell(cell: &Value) -> Option<Cell> {
    let cell_type = cell.get("cell_type")?.as_str()?;
    let meta = CellMetadata::default();

    match cell_type {
        "code" => {
            // v3 uses "input" for source; fall back to "source" for robustness
            let src_val = cell
                .get("input")
                .or_else(|| cell.get("source"))
                .cloned()
                .unwrap_or(Value::Array(vec![]));
            let source = join_multiline(src_val);
            let execution_count = cell
                .get("prompt_number")
                .and_then(Value::as_u64)
                .map(|n| u32::try_from(n).unwrap_or(u32::MAX));
            let outputs = cell
                .get("outputs")
                .and_then(|o| o.as_array())
                .map_or_default(|arr| arr.iter().filter_map(migrate_output).collect());
            Some(Cell::Code(CodeCell {
                id: None,
                source,
                execution_count,
                outputs,
                metadata: meta,
            }))
        }
        "markdown" => {
            let source =
                join_multiline(cell.get("source").cloned().unwrap_or(Value::Array(vec![])));
            Some(Cell::Markdown(MarkdownCell {
                id: None,
                source,
                attachments: None,
                metadata: meta,
            }))
        }
        "raw" => {
            let source =
                join_multiline(cell.get("source").cloned().unwrap_or(Value::Array(vec![])));
            Some(Cell::Raw(RawCell {
                id: None,
                source,
                attachments: None,
                metadata: meta,
            }))
        }
        "heading" => {
            // Convert to a Markdown cell with the appropriate ATX heading prefix.
            let level = cell
                .get("level")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .clamp(1, 6)
                .try_into()
                .unwrap(); // clamp to 1..=6 guarantees fits in usize
            let text = join_multiline(cell.get("source").cloned().unwrap_or(Value::Array(vec![])));
            let source = format!("{} {}", "#".repeat(level), text.trim_end_matches('\n'));
            Some(Cell::Markdown(MarkdownCell {
                id: None,
                source,
                attachments: None,
                metadata: meta,
            }))
        }
        _ => None, // skip unknown cell types
    }
}

// ── Outputs ───────────────────────────────────────────────────────────────────

fn migrate_output(output: &Value) -> Option<Output> {
    let output_type = output.get("output_type")?.as_str()?;

    match output_type {
        "pyout" => Some(Output::ExecuteResult(ExecuteResultOutput {
            execution_count: output
                .get("prompt_number")
                .and_then(Value::as_u64)
                .map_or(0, |n| u32::try_from(n).unwrap_or(u32::MAX)),
            data: build_mime_bundle(output),
            metadata: HashMap::new(),
        })),
        "pyerr" => Some(Output::Error(ErrorOutput {
            ename: string_field(output, "ename"),
            evalue: string_field(output, "evalue"),
            traceback: output
                .get("traceback")
                .and_then(|t| t.as_array())
                .map_or_default(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                }),
        })),
        "stream" => {
            // v3 "stream" field → v4 "name" field
            let raw = output
                .get("stream")
                .and_then(|s| s.as_str())
                .unwrap_or("stdout");
            let name = match raw {
                "stdout" => StreamName::Stdout,
                "stderr" => StreamName::Stderr,
                other => StreamName::Other(other.to_string()),
            };
            let text = join_multiline(output.get("text").cloned().unwrap_or(Value::Array(vec![])));
            Some(Output::Stream(StreamOutput { name, text }))
        }
        "display_data" => Some(Output::DisplayData(DisplayDataOutput {
            data: build_mime_bundle(output),
            metadata: HashMap::new(),
        })),
        _ => None,
    }
}

/// Map v3 per-type output fields to their v4 MIME-type equivalents.
fn build_mime_bundle(output: &Value) -> MimeBundle {
    const MAP: &[(&str, &str)] = &[
        ("text", "text/plain"),
        ("html", "text/html"),
        ("svg", "image/svg+xml"),
        ("png", "image/png"),
        ("jpeg", "image/jpeg"),
        ("latex", "text/latex"),
        ("json", "application/json"),
        ("javascript", "application/javascript"),
        ("pdf", "application/pdf"),
    ];
    let mut idx = IndexMap::new();
    for (v3_key, mime) in MAP {
        if let Some(val) = output.get(v3_key) {
            if val.is_null() {
                continue;
            }
            let data = if *mime == "application/json" {
                MimeData::Json(val.clone())
            } else {
                MimeData::String(join_multiline(val.clone()))
            };
            idx.insert((*mime).to_string(), data);
        }
    }
    MimeBundle(idx)
}

fn string_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}
