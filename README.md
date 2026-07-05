# nbconv-rs

A Rust implementation of Jupyter notebook (`.ipynb`) → HTML conversion.

## Crates

| Crate | Description |
|---|---|
| `crates/nbformat` | Strongly-typed nbformat v3 + v4 deserialisation |
| `crates/nbhtml` | Notebook → `HtmlDocument` transform + HTML rendering |
| `crates/nbconv` | CLI binary |

## Usage

```
nbconv input.ipynb -o output.html
nbconv input.ipynb -o output.html --theme "Solarized (dark)"
nbconv input.ipynb -o output.html --no-mathjax
nbconv --list-themes
```

## Milestones

- [x] **M1** – Parse nbformat v4 notebooks into typed Rust structures
- [x] **M2** – Generate plain HTML (code → `<pre>`, images → base64 `<img>`)
- [x] **M3** – Syntax highlighting via `syntect` (inline styles, 7 themes)
- [x] **M4** – Markdown rendering via `pulldown-cmark` with MathJax math passthrough
- [x] **M5** – ANSI → coloured `<span>` for stream outputs and error tracebacks
- [x] **M6** – nbformat v3 migration, `--theme`, `--no-mathjax`, `--list-themes`
