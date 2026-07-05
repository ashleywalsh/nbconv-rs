//! Custom serde deserializers used by the nbformat types.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Deserialise a field that may be either a plain `String` or a `Vec<String>`.
///
/// In nbformat, `source`, `text`, and MIME-bundle text entries may be stored
/// as a flat string **or** as an array of strings (one element per line).
/// In both cases we join them without an extra separator — each array element
/// already carries its own trailing `\n`.
pub fn multiline<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    Ok(join_multiline(value))
}

/// Join a JSON value that is either a bare `String` or an `Array` of strings.
///
/// Used by [`multiline`] and by the [`MimeBundle`](crate::MimeBundle)
/// deserialiser for all non-JSON MIME types.
pub(crate) fn join_multiline(value: Value) -> String {
    match value {
        Value::String(s) => s,
        Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s),
                _ => None,
            })
            .collect(),
        _ => String::new(),
    }
}
