# nbconv-rs

A Rust implementation of Jupyter notebook (`.ipynb`) → HTML conversion.

## Crates

| Crate             | Description                                          |
| ----------------- | ---------------------------------------------------- |
| `crates/nbformat` | Strongly-typed nbformat v3 + v4 deserialisation      |
| `crates/nbhtml`   | Notebook → `HtmlDocument` transform + HTML rendering |
| `crates/nbconv`   | CLI binary                                           |

## Usage

```
nbconv input.ipynb -o output.html
nbconv input.ipynb -o output.html --theme "Solarized (dark)"
nbconv input.ipynb -o output.html --no-mathjax
nbconv --list-themes
```
