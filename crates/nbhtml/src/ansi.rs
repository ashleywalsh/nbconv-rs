//! ANSI terminal escape-code handling.
//!
//! Two public functions:
//!
//! * [`strip`]   – remove all escape sequences (kept for completeness).
//! * [`to_html`] – convert SGR colour codes to `<span class="ansi-…">` elements.
//!
//! The implementation follows Python nbconvert's `filters/ansi.py` closely,
//! including the bold-colour-shift rule (bold + standard colour → bright colour).

// ── ANSI CSS (injected into <head> when needed) ───────────────────────────────

pub(crate) const CSS: &str = "\
/* ANSI terminal colours — used by stream outputs and error tracebacks */
.ansi-black-fg   { color: #3e424d; }
.ansi-red-fg     { color: #e75c58; }
.ansi-green-fg   { color: #00a250; }
.ansi-yellow-fg  { color: #ddb62b; }
.ansi-blue-fg    { color: #208ffb; }
.ansi-magenta-fg { color: #d160c4; }
.ansi-cyan-fg    { color: #60c6c8; }
.ansi-white-fg   { color: #c5c1b4; }
.ansi-black-intense-fg   { color: #626262; }
.ansi-red-intense-fg     { color: #ef6155; }
.ansi-green-intense-fg   { color: #99e343; }
.ansi-yellow-intense-fg  { color: #f4d30c; }
.ansi-blue-intense-fg    { color: #70a5f9; }
.ansi-magenta-intense-fg { color: #e488a0; }
.ansi-cyan-intense-fg    { color: #88e0e0; }
.ansi-white-intense-fg   { color: #f7f7f7; }
.ansi-black-bg   { background-color: #3e424d; }
.ansi-red-bg     { background-color: #e75c58; }
.ansi-green-bg   { background-color: #00a250; }
.ansi-yellow-bg  { background-color: #ddb62b; }
.ansi-blue-bg    { background-color: #208ffb; }
.ansi-magenta-bg { background-color: #d160c4; }
.ansi-cyan-bg    { background-color: #60c6c8; }
.ansi-white-bg   { background-color: #c5c1b4; }
.ansi-black-intense-bg   { background-color: #626262; }
.ansi-red-intense-bg     { background-color: #ef6155; }
.ansi-green-intense-bg   { background-color: #99e343; }
.ansi-yellow-intense-bg  { background-color: #f4d30c; }
.ansi-blue-intense-bg    { background-color: #70a5f9; }
.ansi-magenta-intense-bg { background-color: #e488a0; }
.ansi-cyan-intense-bg    { background-color: #88e0e0; }
.ansi-white-intense-bg   { background-color: #f7f7f7; }
.ansi-bold      { font-weight: bold; }
.ansi-underline { text-decoration: underline; }
.ansi-default-inverse-fg { color: #ffffff; }
.ansi-default-inverse-bg { background-color: #000000; }
";

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum AnsiColor {
    Index(u8),
    Rgb(u8, u8, u8),
}

const FG: &[&str] = &[
    "ansi-black-fg",
    "ansi-red-fg",
    "ansi-green-fg",
    "ansi-yellow-fg",
    "ansi-blue-fg",
    "ansi-magenta-fg",
    "ansi-cyan-fg",
    "ansi-white-fg",
];
const FG_INT: &[&str] = &[
    "ansi-black-intense-fg",
    "ansi-red-intense-fg",
    "ansi-green-intense-fg",
    "ansi-yellow-intense-fg",
    "ansi-blue-intense-fg",
    "ansi-magenta-intense-fg",
    "ansi-cyan-intense-fg",
    "ansi-white-intense-fg",
];
const BG: &[&str] = &[
    "ansi-black-bg",
    "ansi-red-bg",
    "ansi-green-bg",
    "ansi-yellow-bg",
    "ansi-blue-bg",
    "ansi-magenta-bg",
    "ansi-cyan-bg",
    "ansi-white-bg",
];
const BG_INT: &[&str] = &[
    "ansi-black-intense-bg",
    "ansi-red-intense-bg",
    "ansi-green-intense-bg",
    "ansi-yellow-intense-bg",
    "ansi-blue-intense-bg",
    "ansi-magenta-intense-bg",
    "ansi-cyan-intense-bg",
    "ansi-white-intense-bg",
];

// ── Public API ────────────────────────────────────────────────────────────────

/// Remove all ANSI escape sequences from `input`.
#[allow(dead_code)]
pub(crate) fn strip(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            out.push(ch);
            continue;
        }
        match chars.peek().copied() {
            Some('[') => {
                chars.next();
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {
                chars.next();
            }
        }
    }
    out
}

/// Convert ANSI SGR colour codes to `<span class="ansi-…">` elements.
///
/// Plain text content is HTML-escaped.  Non-SGR escape sequences are
/// silently discarded.  Returns an HTML fragment safe to embed inside `<pre>`.
pub(crate) fn to_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);

    // SGR state
    let mut fg: Option<AnsiColor> = None;
    let mut bg: Option<AnsiColor> = None;
    let mut bold = false;
    let mut underline = false;
    let mut inverse = false;

    let mut buf = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            buf.push(ch);
            continue;
        }

        // Flush pending text before changing state.
        if !buf.is_empty() {
            push_span(&mut out, &buf, fg, bg, bold, underline, inverse);
            buf.clear();
        }

        match chars.peek().copied() {
            Some('[') => {
                chars.next();
                let mut param = String::new();
                let mut end = 'm';
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        end = c;
                        break;
                    }
                    param.push(c);
                }
                if end == 'm' {
                    let codes: Vec<u32> = if param.is_empty() {
                        vec![0]
                    } else {
                        param.split(';').filter_map(|s| s.parse().ok()).collect()
                    };
                    apply_sgr(
                        &codes,
                        &mut fg,
                        &mut bg,
                        &mut bold,
                        &mut underline,
                        &mut inverse,
                    );
                }
            }
            Some(']') => {
                chars.next();
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {
                chars.next();
            }
        }
    }
    if !buf.is_empty() {
        push_span(&mut out, &buf, fg, bg, bold, underline, inverse);
    }
    out
}

// ── SGR state machine ─────────────────────────────────────────────────────────

fn apply_sgr(
    codes: &[u32],
    fg: &mut Option<AnsiColor>,
    bg: &mut Option<AnsiColor>,
    bold: &mut bool,
    underline: &mut bool,
    inverse: &mut bool,
) {
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0 => {
                *fg = None;
                *bg = None;
                *bold = false;
                *underline = false;
                *inverse = false;
            }
            1 => *bold = true,
            4 => *underline = true,
            5 => *bold = true, // blink → bold (matches Python nbconvert)
            7 => *inverse = true,
            21 | 22 => *bold = false,
            24 => *underline = false,
            27 => *inverse = false,
            30..=37 => *fg = Some(AnsiColor::Index((codes[i] - 30) as u8)),
            38 => {
                if let Some(c) = ext_color(codes, &mut i) {
                    *fg = Some(c);
                }
            }
            39 => *fg = None,
            40..=47 => *bg = Some(AnsiColor::Index((codes[i] - 40) as u8)),
            48 => {
                if let Some(c) = ext_color(codes, &mut i) {
                    *bg = Some(c);
                }
            }
            49 => *bg = None,
            90..=97 => *fg = Some(AnsiColor::Index((codes[i] - 90 + 8) as u8)),
            100..=107 => *bg = Some(AnsiColor::Index((codes[i] - 100 + 8) as u8)),
            _ => {}
        }
        i += 1;
    }
}

fn ext_color(codes: &[u32], i: &mut usize) -> Option<AnsiColor> {
    *i += 1;
    if *i >= codes.len() {
        return None;
    }
    match codes[*i] {
        5 => {
            // 256-colour: 38;5;n
            *i += 1;
            codes.get(*i).map(|&n| AnsiColor::Index(n as u8))
        }
        2 if *i + 3 < codes.len() => {
            // True colour: 38;2;r;g;b
            let r = codes[*i + 1] as u8;
            let g = codes[*i + 2] as u8;
            let b = codes[*i + 3] as u8;
            *i += 3;
            Some(AnsiColor::Rgb(r, g, b))
        }
        _ => None,
    }
}

// ── Span emission ─────────────────────────────────────────────────────────────

fn push_span(
    out: &mut String,
    text: &str,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    bold: bool,
    underline: bool,
    inverse: bool,
) {
    let esc = crate::html_esc(text);

    if !has_style(fg, bg, bold, underline, inverse) {
        out.push_str(&esc);
        return;
    }

    let mut classes: Vec<&str> = Vec::new();
    let mut styles: Vec<String> = Vec::new();

    // Bold-colour shift: bold + standard colour → bright colour (matches
    // Python nbconvert: `fg + 8 if bold and fg in range(8) else fg`).
    let shifted_fg = match fg {
        Some(AnsiColor::Index(i)) if bold && i < 8 => Some(AnsiColor::Index(i + 8)),
        other => other,
    };

    // Inverse swaps fg and bg (applied after the shift).
    let (draw_fg, draw_bg) = if inverse { (bg, shifted_fg) } else { (shifted_fg, bg) };

    // Foreground
    match draw_fg {
        None if inverse => classes.push("ansi-default-inverse-fg"),
        Some(AnsiColor::Index(i)) if i < 8 => classes.push(FG[i as usize]),
        Some(AnsiColor::Index(i)) if i < 16 => classes.push(FG_INT[(i - 8) as usize]),
        Some(AnsiColor::Index(i)) => {
            let (r, g, b) = idx_rgb(i);
            styles.push(format!("color:rgb({},{},{})", r, g, b));
        }
        Some(AnsiColor::Rgb(r, g, b)) => styles.push(format!("color:rgb({},{},{})", r, g, b)),
        _ => {}
    }

    // Background
    match draw_bg {
        None if inverse => classes.push("ansi-default-inverse-bg"),
        Some(AnsiColor::Index(i)) if i < 8 => classes.push(BG[i as usize]),
        Some(AnsiColor::Index(i)) if i < 16 => classes.push(BG_INT[(i - 8) as usize]),
        Some(AnsiColor::Index(i)) => {
            let (r, g, b) = idx_rgb(i);
            styles.push(format!("background-color:rgb({},{},{})", r, g, b));
        }
        Some(AnsiColor::Rgb(r, g, b)) => {
            styles.push(format!("background-color:rgb({},{},{})", r, g, b))
        }
        _ => {}
    }

    if bold {
        classes.push("ansi-bold");
    }
    if underline {
        classes.push("ansi-underline");
    }

    out.push_str("<span");
    if !classes.is_empty() {
        out.push_str(" class=\"");
        out.push_str(&classes.join(" "));
        out.push('"');
    }
    if !styles.is_empty() {
        out.push_str(" style=\"");
        out.push_str(&styles.join(";"));
        out.push('"');
    }
    out.push('>');
    out.push_str(&esc);
    out.push_str("</span>");
}

fn has_style(
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    bold: bool,
    underline: bool,
    inverse: bool,
) -> bool {
    fg.is_some() || bg.is_some() || bold || underline || inverse
}

/// Map a 256-colour palette index ≥ 16 to an RGB triple.
fn idx_rgb(idx: u8) -> (u8, u8, u8) {
    debug_assert!(idx >= 16, "idx_rgb called with index < 16");
    if idx < 232 {
        let i = idx - 16;
        let c = |v: u8| if v == 0 { 0u8 } else { 55 + v * 40 };
        (c(i / 36), c((i % 36) / 6), c(i % 6))
    } else {
        let v = (idx - 232) * 10 + 8;
        (v, v, v)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // strip ────────────────────────────────────────────────────────────────────

    #[test]
    fn strip_plain_unchanged() {
        assert_eq!(strip("hello\nworld"), "hello\nworld");
    }

    #[test]
    fn strip_sgr_color() {
        assert_eq!(strip("\x1b[31mred\x1b[0m normal"), "red normal");
    }

    #[test]
    fn strip_bold() {
        assert_eq!(strip("\x1b[1mBold\x1b[22m text"), "Bold text");
    }

    #[test]
    fn strip_256_color() {
        assert_eq!(strip("\x1b[38;5;196mcolored\x1b[0m"), "colored");
    }

    #[test]
    fn strip_osc_hyperlink() {
        let s = "\x1b]8;;http://example.com\x07link\x1b]8;;\x07";
        assert_eq!(strip(s), "link");
    }

    #[test]
    fn strip_bare_esc_at_end() {
        assert_eq!(strip("text\x1b"), "text");
    }

    // to_html ─────────────────────────────────────────────────────────────────

    #[test]
    fn to_html_plain_text_escaped() {
        let html = to_html("plain <text> & more");
        assert!(html.contains("&lt;"), "html = {html:?}");
        assert!(html.contains("&amp;"), "html = {html:?}");
        assert!(!html.contains('<'), "< must be escaped");
    }

    #[test]
    fn to_html_red_foreground() {
        let html = to_html("\x1b[31mred\x1b[0m");
        assert!(html.contains("ansi-red-fg"), "html = {html:?}");
        assert!(html.contains("red"), "html = {html:?}");
    }

    #[test]
    fn to_html_bold() {
        let html = to_html("\x1b[1mbold\x1b[0m");
        assert!(html.contains("ansi-bold"), "html = {html:?}");
    }

    #[test]
    fn to_html_reset_separates_styled_and_plain() {
        let html = to_html("\x1b[31mred\x1b[0m normal");
        let span_end = html.find("</span>").expect("closing span");
        let plain_pos = html.find("normal").expect("plain text");
        assert!(plain_pos > span_end, "'normal' must come after </span>");
    }

    #[test]
    fn to_html_256_color_uses_rgb_style() {
        // index 196 = rgb(255,0,0)
        let html = to_html("\x1b[38;5;196mtext\x1b[0m");
        assert!(
            html.contains("color:rgb("),
            "expected rgb() style: {html:?}"
        );
        assert!(html.contains("255,0,0"), "expected (255,0,0): {html:?}");
    }

    #[test]
    fn to_html_combined_codes_reset_then_color() {
        // ESC[0;31m = reset + red
        let html = to_html("\x1b[0;31mred\x1b[0m");
        assert!(html.contains("ansi-red-fg"), "html = {html:?}");
    }

    #[test]
    fn to_html_bright_color_via_90_97() {
        // ESC[91m = bright red fg (code 91 → index 9 → FG_INT[1])
        let html = to_html("\x1b[91mbright\x1b[0m");
        assert!(html.contains("ansi-red-intense-fg"), "html = {html:?}");
    }

    #[test]
    fn to_html_bold_shifts_to_bright_color() {
        // bold + fg=red(1) → shifted to bright-red(9) → ansi-red-intense-fg
        let html = to_html("\x1b[1;31mtext\x1b[0m");
        assert!(html.contains("ansi-red-intense-fg"), "html = {html:?}");
        assert!(html.contains("ansi-bold"), "html = {html:?}");
    }

    #[test]
    fn to_html_true_color() {
        let html = to_html("\x1b[38;2;255;128;0morange\x1b[0m");
        assert!(html.contains("color:rgb(255,128,0)"), "html = {html:?}");
    }
}
