//! Strongly-typed in-memory representation of Jupyter notebooks.
//!
//! The public entry point is [`parse`], which accepts the JSON text of an
//! `.ipynb` file and returns a [`Notebook`].
//!
//! ## Supported versions
//!
//! | nbformat | Status |
//! |----------|--------|
//! | 4.x      | fully supported (direct deserialisation) |
//! | 3.x      | supported via automatic migration to v4   |
//! | 1, 2     | **not supported** — [`parse`] returns [`ParseError::UnsupportedVersion`] |

pub(crate) mod de;
mod migrate_v3;

use std::collections::HashMap;
use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

// ── Error type ────────────────────────────────────────────────────────────────

/// Errors returned by [`parse`].
#[derive(Debug)]
pub enum ParseError {
    /// The input is not valid JSON, or does not match the expected notebook
    /// structure.
    Json(serde_json::Error),
    /// The notebook uses an nbformat version that is not supported.
    ///
    /// Only versions **3** and **4** are supported.  Versions 1 and 2 predate
    /// Jupyter and are too rare to warrant a migration path; use the Python
    /// `nbformat` library to upgrade them first.
    UnsupportedVersion(u64),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Json(e) => write!(f, "{e}"),
            ParseError::UnsupportedVersion(v) => write!(
                f,
                "nbformat v{v} is not supported \
                 (only v3 and v4 are supported; \
                 use the Python `nbformat` library to upgrade this notebook first)"
            ),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Json(e) => Some(e),
            ParseError::UnsupportedVersion(_) => None,
        }
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(e: serde_json::Error) -> Self {
        ParseError::Json(e)
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Parse an `.ipynb` JSON string into a [`Notebook`].
///
/// nbformat v3 notebooks are automatically migrated to v4.
///
/// # Errors
///
/// Returns [`ParseError::Json`] if the input is not valid JSON or does not
/// match the expected notebook structure, and
/// [`ParseError::UnsupportedVersion`] for nbformat versions 1 and 2.
#[cfg_attr(feature = "profiling", tracing::instrument(skip_all))]
pub fn parse(json: &str) -> Result<Notebook, ParseError> {
    let value: Value = {
        #[cfg(feature = "profiling")]
        let _span = tracing::info_span!("json_to_value").entered();
        serde_json::from_str(json)?
    };
    let nbformat = value.get("nbformat").and_then(Value::as_u64).unwrap_or(4);

    match nbformat {
        1 | 2 => Err(ParseError::UnsupportedVersion(nbformat)),
        3 => Ok(migrate_v3::from_value(&value)),
        _ => {
            #[cfg(feature = "profiling")]
            let _span = tracing::info_span!("value_to_notebook").entered();
            serde_json::from_value(value).map_err(ParseError::Json)
        }
    }
}

// ── Notebook ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Notebook {
    pub nbformat: u32,
    pub nbformat_minor: u32,
    pub metadata: NotebookMetadata,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotebookMetadata {
    #[serde(default)]
    pub kernelspec: Option<KernelSpec>,
    #[serde(default)]
    pub language_info: Option<LanguageInfo>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub authors: Option<Vec<Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KernelSpec {
    pub name: String,
    pub display_name: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub pygments_lexer: Option<String>,
    #[serde(default)]
    pub codemirror_mode: Option<Value>,
    #[serde(default)]
    pub file_extension: Option<String>,
    #[serde(default)]
    pub mimetype: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

// ── Cells ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "cell_type", rename_all = "lowercase")]
pub enum Cell {
    Code(CodeCell),
    Markdown(MarkdownCell),
    Raw(RawCell),
}

impl Cell {
    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Cell::Code(c) => &c.source,
            Cell::Markdown(c) => &c.source,
            Cell::Raw(c) => &c.source,
        }
    }
    #[must_use]
    pub fn metadata(&self) -> &CellMetadata {
        match self {
            Cell::Code(c) => &c.metadata,
            Cell::Markdown(c) => &c.metadata,
            Cell::Raw(c) => &c.metadata,
        }
    }
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Cell::Code(c) => c.id.as_deref(),
            Cell::Markdown(c) => c.id.as_deref(),
            Cell::Raw(c) => c.id.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodeCell {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(deserialize_with = "de::multiline")]
    pub source: String,
    pub execution_count: Option<u32>,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(default)]
    pub metadata: CellMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkdownCell {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(deserialize_with = "de::multiline")]
    pub source: String,
    #[serde(default)]
    pub attachments: Option<Attachments>,
    #[serde(default)]
    pub metadata: CellMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawCell {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(deserialize_with = "de::multiline")]
    pub source: String,
    #[serde(default)]
    pub attachments: Option<Attachments>,
    #[serde(default)]
    pub metadata: CellMetadata,
}

pub type Attachments = HashMap<String, HashMap<String, String>>;

// ── Cell metadata ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CellMetadata {
    #[serde(default)]
    pub collapsed: Option<bool>,
    #[serde(default)]
    pub scrolled: Option<Scrolled>,
    #[serde(default)]
    pub deletable: Option<bool>,
    #[serde(default)]
    pub editable: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub jupyter: Option<JupyterCellMetadata>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JupyterCellMetadata {
    #[serde(default)]
    pub source_hidden: Option<bool>,
    #[serde(default)]
    pub outputs_hidden: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum Scrolled {
    Bool(bool),
    Auto,
}

impl<'de> Deserialize<'de> for Scrolled {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl serde::de::Visitor<'_> for V {
            type Value = Scrolled;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a boolean or \"auto\"")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Scrolled, E> {
                Ok(Scrolled::Bool(v))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Scrolled, E> {
                if v == "auto" {
                    Ok(Scrolled::Auto)
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(v), &self))
                }
            }
        }
        d.deserialize_any(V)
    }
}

// ── Outputs ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "output_type", rename_all = "snake_case")]
pub enum Output {
    Stream(StreamOutput),
    DisplayData(DisplayDataOutput),
    ExecuteResult(ExecuteResultOutput),
    Error(ErrorOutput),
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamOutput {
    pub name: StreamName,
    #[serde(deserialize_with = "de::multiline")]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamName {
    Stdout,
    Stderr,
    Other(String),
}

impl<'de> Deserialize<'de> for StreamName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "stdout" => StreamName::Stdout,
            "stderr" => StreamName::Stderr,
            _ => StreamName::Other(s),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DisplayDataOutput {
    #[serde(default)]
    pub data: MimeBundle,
    #[serde(default)]
    pub metadata: MimeBundleMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteResultOutput {
    pub execution_count: u32,
    #[serde(default)]
    pub data: MimeBundle,
    #[serde(default)]
    pub metadata: MimeBundleMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorOutput {
    pub ename: String,
    pub evalue: String,
    #[serde(default)]
    pub traceback: Vec<String>,
}

// ── MIME bundles ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct MimeBundle(pub IndexMap<String, MimeData>);

impl MimeBundle {
    #[must_use]
    pub fn get(&self, mime: &str) -> Option<&MimeData> {
        self.0.get(mime)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &MimeData)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for MimeBundle {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw: IndexMap<String, Value> = IndexMap::deserialize(d)?;
        let mut bundle = IndexMap::with_capacity(raw.len());
        for (mime, value) in raw {
            bundle.insert(mime.clone(), MimeData::from_json_value(&mime, value));
        }
        Ok(MimeBundle(bundle))
    }
}

#[derive(Debug, Clone)]
pub enum MimeData {
    String(String),
    Json(Value),
}

impl MimeData {
    fn from_json_value(mime: &str, value: Value) -> Self {
        if mime == "application/json"
            || (mime.starts_with("application/") && mime.ends_with("+json"))
        {
            MimeData::Json(value)
        } else {
            MimeData::String(de::join_multiline(value))
        }
    }
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            MimeData::String(s) => Some(s),
            MimeData::Json(_) => None,
        }
    }
    #[must_use]
    pub fn as_json(&self) -> Option<&Value> {
        match self {
            MimeData::Json(v) => Some(v),
            MimeData::String(_) => None,
        }
    }
}

pub type MimeBundleMetadata = HashMap<String, Value>;
