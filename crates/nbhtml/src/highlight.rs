//! Syntax highlighting via syntect.
//!
//! `SyntaxSet` and `ThemeSet` are loaded once per process via [`OnceLock`].

use std::sync::OnceLock;

use syntect::highlighting::{Theme, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::{SyntaxReference, SyntaxSet};

const DEFAULT_THEME: &str = "InspiredGitHub";

static SS: OnceLock<SyntaxSet> = OnceLock::new();
static TS: OnceLock<ThemeSet> = OnceLock::new();

// The one-off loads get their own spans so their cost is not attributed to
// whichever cell happens to be highlighted first.
fn ss() -> &'static SyntaxSet {
    SS.get_or_init(|| {
        #[cfg(feature = "profiling")]
        let _span = tracing::info_span!("load_syntax_set").entered();
        SyntaxSet::load_defaults_newlines()
    })
}
fn ts() -> &'static ThemeSet {
    TS.get_or_init(|| {
        #[cfg(feature = "profiling")]
        let _span = tracing::info_span!("load_theme_set").entered();
        ThemeSet::load_defaults()
    })
}

fn resolve_theme(name: &str) -> &'static Theme {
    ts().themes
        .get(name)
        .or_else(|| ts().themes.get(DEFAULT_THEME))
        .expect("InspiredGitHub is always present in syntect defaults")
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Highlight `source` in `language` using the named syntect `theme`.
///
/// Returns inner `<span style="…">` HTML — no `<pre>` wrapper.
/// Falls back to plain HTML-escaped text on unknown language or error.
#[cfg_attr(feature = "profiling", tracing::instrument(skip_all))]
pub(crate) fn highlight(source: &str, language: &str, theme: &str) -> String {
    let ss = ss();
    let syn = find_syntax(ss, language);
    match highlighted_html_for_string(source, ss, syn, resolve_theme(theme)) {
        Ok(html) => strip_pre_wrapper(&html),
        Err(_) => crate::html_esc(source),
    }
}

/// Return the names of all built-in syntect themes, sorted.
pub(crate) fn available_themes() -> Vec<String> {
    let mut v: Vec<String> = ts().themes.keys().cloned().collect();
    v.sort_unstable();
    v
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn find_syntax<'a>(ss: &'a SyntaxSet, language: &str) -> &'a SyntaxReference {
    if let Some(s) = ss.find_syntax_by_name(language) {
        return s;
    }
    if let Some(s) = ss.find_syntax_by_name(&title_case(language)) {
        return s;
    }
    if let Some(ext) = lang_to_ext(language) {
        if let Some(s) = ss.find_syntax_by_extension(ext) {
            return s;
        }
    }
    ss.find_syntax_plain_text()
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn lang_to_ext(lang: &str) -> Option<&'static str> {
    Some(match lang.to_ascii_lowercase().as_str() {
        "python" | "python3" | "ipython" | "ipython3" | "pypy" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "rust" => "rs",
        "julia" => "jl",
        "r" => "r",
        "bash" | "shell" | "sh" => "sh",
        "c" => "c",
        "c++" | "cpp" => "cpp",
        "java" => "java",
        "go" => "go",
        "ruby" | "rb" => "rb",
        "sql" => "sql",
        "html" => "html",
        "css" => "css",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        _ => return None,
    })
}

fn strip_pre_wrapper(html: &str) -> String {
    let start = html.find('\n').map_or(0, |i| i + 1);
    let end = html.rfind("</pre>").unwrap_or(html.len());
    html[start..end].to_string()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_produces_spans() {
        let html = highlight("x = 1", "python", DEFAULT_THEME);
        assert!(html.contains("<span"), "html = {html:?}");
    }

    #[test]
    fn no_pre_wrapper_returned() {
        let html = highlight("x = 1", "python", DEFAULT_THEME);
        assert!(!html.contains("<pre"), "must not include <pre>");
    }

    #[test]
    fn unknown_language_does_not_panic() {
        let html = highlight("def foo(): pass", "xyzzy_not_real", DEFAULT_THEME);
        assert!(!html.is_empty());
    }

    #[test]
    fn special_chars_escaped_in_fallback() {
        let e = crate::html_esc("<b>bold</b> & \"q\"");
        assert!(e.contains("&lt;b&gt;") && e.contains("&amp;") && e.contains("&quot;"));
    }

    #[test]
    fn title_case_works() {
        assert_eq!(title_case("python"), "Python");
        assert_eq!(title_case("rust"), "Rust");
        assert_eq!(title_case(""), "");
    }

    #[test]
    fn available_themes_includes_defaults() {
        let themes = available_themes();
        assert!(themes.contains(&"InspiredGitHub".to_string()));
        assert!(themes.contains(&"Solarized (dark)".to_string()));
        assert!(!themes.is_empty());
    }

    #[test]
    fn solarized_dark_theme_works() {
        let html = highlight("x = 1", "python", "Solarized (dark)");
        assert!(
            html.contains("<span"),
            "Solarized (dark) should produce spans"
        );
    }

    #[test]
    fn unknown_theme_falls_back_to_default() {
        // Should not panic — falls back to InspiredGitHub
        let html = highlight("x = 1", "python", "NonExistentTheme_xyz");
        assert!(html.contains("<span"));
    }
}
