#!/usr/bin/env bash
# Run the full benchmark and comparison pipeline in sequence.
#
# Steps:
#   1. setup_corpus.sh  — clone / update all repos in corpus.toml
#   2. benchmark.sh     — hyperfine timing comparison → results/benchmarks/
#   3. compare.py       — run both tools, save HTML outputs + diffs → comparisons/
#
# Flags:
#   --skip-setup      skip corpus clone/update (use if corpus is already current)
#   --skip-benchmark  skip hyperfine benchmarks
#   --skip-compare    skip HTML output and diff generation
set -euo pipefail

cd "$(dirname "$0")/.."

skip_setup=0
skip_benchmark=0
skip_compare=0

for arg in "$@"; do
    case "$arg" in
    --skip-setup) skip_setup=1 ;;
    --skip-benchmark) skip_benchmark=1 ;;
    --skip-compare) skip_compare=1 ;;
    *)
        echo "usage: run_all.sh [--skip-setup] [--skip-benchmark] [--skip-compare]"
        exit 1
        ;;
    esac
done

# ── Helper ────────────────────────────────────────────────────────────────────

section() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  $1"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# ── Steps ─────────────────────────────────────────────────────────────────────

if [ "$skip_setup" -eq 0 ]; then
    section "1/3  Corpus setup"
    bash scripts/setup_corpus.sh
else
    echo "(skipping corpus setup)"
fi

if [ "$skip_benchmark" -eq 0 ]; then
    section "2/3  Benchmarks"
    bash scripts/benchmark.sh
else
    echo "(skipping benchmarks)"
fi

if [ "$skip_compare" -eq 0 ]; then
    section "3/3  HTML comparison"
    uv run --script scripts/compare.py
else
    echo "(skipping HTML comparison)"
fi

section "Done"
echo "  Benchmark results : results/benchmarks/  (commit these)"
echo "  HTML outputs      : comparisons/          (gitignored, open in browser)"
