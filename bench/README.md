# bench

Benchmarking and comparison infrastructure for `nbconv` vs `nbconvert`.

## Prerequisites

| Tool                                                | Purpose                | Install                   |
| --------------------------------------------------- | ---------------------- | ------------------------- |
| [`hyperfine`](https://github.com/sharkdp/hyperfine) | Benchmarking           | `brew install hyperfine`  |
| [`uv`](https://docs.astral.sh/uv/)                  | Running Python scripts | `brew install uv`         |
| `cargo`                                             | Building `nbconv`      | comes with Rust toolchain |

Python dependencies (`nbconvert`, `nbformat`, etc.) are declared inline in
each script using [PEP 723](https://peps.python.org/pep-0723/) and installed
automatically by `uv` on first run.

## Directory structure

```
bench/
  corpus.toml           # corpus configuration — the only file you normally edit
  scripts/              # all tooling
    setup_corpus.sh     # clone / update all repos in corpus.toml
    nbconvert_html.py   # nbconvert wrapper (PEP 723)
    benchmark.sh        # hyperfine runner → results/benchmarks/
    compare.py          # run both tools, save HTML outputs → comparisons/
    run_all.sh          # run all three steps in sequence
    clean.sh            # remove generated files
  results/
    benchmarks/         # hyperfine JSON results — committed to git
  corpus/               # cloned repos — gitignored
  comparisons/          # HTML outputs — gitignored, open in browser
```

## Corpus configuration

Edit `corpus.toml` to manage repos. All repos are **always cloned** by
`setup_corpus.sh` regardless of their `enabled` flag — `enabled` only controls
which repos are included in a given benchmark or comparison run. This means you
can flip `enabled` without re-cloning.

```toml
[[repos]]
name        = "handson-ml3"
url         = "https://github.com/ageron/handson-ml3"
license     = "Apache-2.0"
description = "Hands-on Machine Learning 3rd ed."
enabled     = true
glob        = "*.ipynb"
```

## Workflow

### Run everything

```sh
cargo build --release
./scripts/run_all.sh
```

Individual steps can be skipped if already done:

```sh
./scripts/run_all.sh --skip-setup --skip-benchmark
```

### Run steps individually

**1. Clone / update corpus repos**

```sh
./scripts/setup_corpus.sh
```

Clones all repos defined in `corpus.toml` into `corpus/`. Re-running pulls
the latest commits. Safe to run repeatedly.

**2. Build `nbconv`**

```sh
cargo build --release   # from the workspace root
```

The benchmark script expects the binary at `../target/release/nbconv`.

**3. Benchmark**

```sh
./scripts/benchmark.sh
```

Runs `hyperfine` on each enabled notebook, comparing both tools in a single
invocation. Hyperfine prints a timing comparison and saves one JSON file per
notebook into `results/benchmarks/<timestamp>/`. Commit the results directory
to record the run.

**4. HTML comparison**

```sh
uv run --script scripts/compare.py
```

Runs both tools on each enabled notebook and saves the HTML outputs to
`comparisons/<repo>/<notebook>/` for side-by-side inspection in a browser:

| File             | Contents         |
| ---------------- | ---------------- |
| `nbconv.html`    | nbconv output    |
| `nbconvert.html` | nbconvert output |

If a tool fails, its stderr is saved as `<tool>.stderr` instead.

### Clean up

```sh
./scripts/clean.sh            # remove comparisons/ only
./scripts/clean.sh --corpus   # also remove cloned corpus repos
```

`results/benchmarks/` is never touched by `clean.sh`.

## Committing a benchmark run

```sh
git add bench/results/benchmarks/
git commit -m "bench: benchmark run $(date +%Y-%m-%d)"
```
