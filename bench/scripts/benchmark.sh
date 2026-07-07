#!/usr/bin/env bash
# Run hyperfine benchmarks comparing nbconv against nbconvert for every
# enabled notebook in the corpus. Results are saved as JSON under
# results/benchmarks/<timestamp>/ and should be committed after each run.
set -euo pipefail

cd "$(dirname "$0")/.."

CORPUS_TOML="corpus.toml"
CORPUS_DIR="corpus"
RESULTS_DIR="results/benchmarks"
NBCONV="$(pwd)/../target/release/nbconv"
NBCONVERT_SCRIPT="$(pwd)/scripts/nbconvert_html.py"
TIMESTAMP=$(date +%Y-%m-%dT%H%M%S)
OUT_DIR="$RESULTS_DIR/$TIMESTAMP"

# ── Prerequisites ─────────────────────────────────────────────────────────────

check_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        echo "error: '$1' not found. $2"
        exit 1
    }
}

check_cmd hyperfine "Install with: brew install hyperfine"
check_cmd uv "Install with: brew install uv"

if [ ! -x "$NBCONV" ]; then
    echo "error: nbconv binary not found at $NBCONV"
    echo "       Run 'cargo build --release' from the workspace root first."
    exit 1
fi

# ── Collect notebooks from enabled repos ─────────────────────────────────────

notebooks=$(
    python3 - "$CORPUS_TOML" "$CORPUS_DIR" <<'EOF'
import sys, tomllib, pathlib

with open(sys.argv[1], "rb") as f:
    config = tomllib.load(f)

corpus_dir = pathlib.Path(sys.argv[2])
for repo in config.get("repos", []):
    if not repo.get("enabled", False):
        continue
    repo_dir = corpus_dir / repo["name"]
    if not repo_dir.exists():
        print(f"warning: {repo_dir} not found — run setup_corpus.sh first",
              file=sys.stderr)
        continue
    for nb in sorted(repo_dir.glob(repo["glob"])):
        print(nb)
EOF
)

if [ -z "$notebooks" ]; then
    echo "No notebooks found. Run ./scripts/setup_corpus.sh first."
    exit 1
fi

notebook_count=$(echo "$notebooks" | wc -l | tr -d ' ')
echo "==> $notebook_count notebook(s) found across enabled repos"
echo "==> Results will be written to $OUT_DIR/"
mkdir -p "$OUT_DIR"

# ── Pre-warm uv environment ───────────────────────────────────────────────────
# Ensures uv has installed the nbconvert environment before timing starts.

first_nb=$(echo "$notebooks" | head -1)
echo "==> Pre-warming uv environment (first run installs packages)..."
uv run --script "$NBCONVERT_SCRIPT" "$first_nb" >/dev/null 2>&1
echo "==> uv environment ready"

# ── Benchmark each notebook ───────────────────────────────────────────────────

n=0
while IFS= read -r nb; do
    n=$((n + 1))
    repo_name=$(basename "$(dirname "$nb")")
    nb_name=$(basename "$nb" .ipynb)
    json_out="$OUT_DIR/${repo_name}__${nb_name}.json"

    echo ""
    echo "==> [$n/$notebook_count] $repo_name/$nb_name"

    hyperfine \
        --warmup 1 \
        --export-json "$json_out" \
        --command-name "nbconv" \
        "$NBCONV $nb -o /dev/null 2>/dev/null" \
        --command-name "nbconvert" \
        "uv run --script $NBCONVERT_SCRIPT $nb > /dev/null 2>/dev/null"

done <<<"$notebooks"

echo ""
echo "==> Benchmark complete. $n notebook(s) measured."
echo "==> Results: $OUT_DIR/"
echo ""
echo "    To commit this run:"
echo "    git add bench/results/benchmarks/$TIMESTAMP"
echo "    git commit -m 'bench: benchmark run $TIMESTAMP'"
