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

Two complementary approaches are available. Neither affects release builds.

| Approach        | Shows                                          | Build                                                            |
| --------------- | ---------------------------------------------- | ---------------------------------------------------------------- |
| Sampling        | Where CPU time goes, down to library internals | `cargo build --profile profiling -p nbconv`                      |
| Instrumentation | Exact timings of named phases and cells        | `cargo build --profile profiling -p nbconv --features profiling` |

The workspace defines a `profiling` Cargo profile: the same optimisations as
`release` (LTO, single codegen unit) plus debug symbols, so profilers can show
readable stack traces.

### Sampling

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

### Instrumentation

The `profiling` Cargo feature records [`tracing`](https://docs.rs/tracing)
spans and writes them to `nbconv-trace.json` in the current directory, using
[`tracing-chrome`](https://docs.rs/tracing-chrome). Without the feature, the
spans are compiled out entirely, and the `tracing` crates are not dependencies
of the build.

```sh
cargo build --profile profiling -p nbconv --features profiling
target/profiling/nbconv input.ipynb -o output.html
# → nbconv-trace.json (overwritten on each run)
```

Open the trace in [Perfetto](https://ui.perfetto.dev) (drag and drop the file)
or `chrome://tracing`. A trace is written for failed runs too.

Recorded spans:

| Span                                  | Crate      | Covers                                          |
| ------------------------------------- | ---------- | ----------------------------------------------- |
| `main`                                | `nbconv`   | The whole run                                   |
| `read`, `write`                       | `nbconv`   | Reading input / writing output                  |
| `parse`                               | `nbformat` | Notebook parsing                                |
|  `json_to_value`, `value_to_notebook` | `nbformat` | JSON text → `Value`, `Value` → typed `Notebook` |
|  `migrate_v3`                         | `nbformat` | nbformat v3 → v4 migration                      |
| `transform` › `transform_cell`        | `nbhtml`   | Notebook → `HtmlDocument`, per cell             |
|  `highlight`                          | `nbhtml`   | Syntect highlighting of a code cell             |
|  `load_syntax_set`, `load_theme_set`  | `nbhtml`   | One-off syntect asset loading (first use)       |
|  `ansi_to_html`                       | `nbhtml`   | ANSI stream/traceback output conversion         |
| `render` › `render_cell`              | `nbhtml`   | HTML rendering, per cell                        |
|  `markdown`                           | `nbhtml`   | pulldown-cmark rendering of a markdown cell     |

To add a span, annotate a function with
`#[cfg_attr(feature = "profiling", tracing::instrument(skip_all))]`, or open a
scope with `#[cfg(feature = "profiling")] let _span = tracing::info_span!("name").entered();`.
Avoid tiny, frequently called helpers: each span costs on the order of a
microsecond and would distort the timings.
