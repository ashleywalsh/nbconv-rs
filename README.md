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

## Profiling

The workspace defines a `profiling` Cargo profile: the same optimisations as
`release` (LTO, single codegen unit) plus debug symbols, so profilers can show
readable stack traces. It adds no profiling code to the binary, and release
builds are unaffected.

Profile a single run with [samply](https://github.com/mstange/samply), which
opens the recording in the Firefox Profiler (flame graph, call tree, timeline):

```sh
cargo install --locked samply   # one-off

cargo build --profile profiling -p nbconv
samply record target/profiling/nbconv input.ipynb -o output.html
```

Tips:

- A short run may collect few samples at the default rate (~1 kHz). Raise it
  with `samply record --rate 10000 …`, or profile a larger notebook.
- Save a recording without opening the browser using
  `samply record --save-only -o profile.json.gz …`, and view it later with
  `samply load profile.json.gz`.
