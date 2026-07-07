# bench

Benchmarking and comparison infrastructure for `nbconv` vs `nbconvert`.

## Prerequisites

| Tool                                                | Purpose                | Install                   |
| --------------------------------------------------- | ---------------------- | ------------------------- |
| [`hyperfine`](https://github.com/sharkdp/hyperfine) | Benchmarking           | `brew install hyperfine`  |
| [`uv`](https://docs.astral.sh/uv/)                  | Running Python scripts | `brew install uv`         |
| `cargo`                                             | Building `nbconv`      | comes with Rust toolchain |

## Directory structure

```
bench/
  corpus.toml           # corpus configuration — the only file you edit
  scripts/              # all tooling lives here
  results/
    benchmarks/         # hyperfine JSON results, committed to git
  corpus/               # cloned repos, gitignored
  comparisons/          # generated HTML and image diffs, gitignored
```

## Corpus configuration

Edit `corpus.toml` to control which repos are used. Set `enabled = true` to
include a repo, `enabled = false` to exclude it. Changes take effect on the
next run of any script.

```toml
[[repos]]
name    = "handson-ml3"
url     = "https://github.com/ageron/handson-ml3"
enabled = true
glob    = "*.ipynb"
```

## Workflow

### 1. Set up the corpus (first time, or to pull updates)

```sh
./scripts/setup_corpus.sh
```

Clones enabled repos into `corpus/`. Re-running pulls the latest commits.

### 2. Build `nbconv`

```sh
cargo build --release
```

The benchmark scripts expect the binary at `../target/release/nbconv`.

### 3. Run benchmarks

```sh
./scripts/benchmark.sh
```

Runs `hyperfine` on every enabled notebook, comparing `nbconv` against
`nbconvert`. Saves one JSON file per notebook into
`results/benchmarks/YYYY-MM-DD-HHMMSS/`. Commit the results directory to
record the run.

### 4. Run comparisons

```sh
./scripts/compare.py
```

For each enabled notebook: checks both tools succeed, produces a normalised
HTML diff and a side-by-side HTML diff. Output goes to `comparisons/`
(gitignored).

### 5. Run visual diffs

```sh
./scripts/visual_diff.py
```

Screenshots both HTML outputs at a fixed viewport using Playwright and
produces a per-pixel difference image. Output goes to `comparisons/`
(gitignored).

### 6. Run everything

```sh
./scripts/run_all.sh
```

Runs steps 1–5 in sequence.

## Committing a benchmark run

After running `./scripts/benchmark.sh`, commit the new JSON files:

```sh
git add bench/results/benchmarks/
git commit -m "bench: add benchmark run
```
