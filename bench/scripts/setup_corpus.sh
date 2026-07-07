#!/usr/bin/env bash
# Clone or update all enabled corpus repos defined in corpus.toml.
# Safe to re-run: existing repos are pulled, not re-cloned.
set -euo pipefail

cd "$(dirname "$0")/.."

CORPUS_DIR="corpus"
CORPUS_TOML="corpus.toml"

mkdir -p "$CORPUS_DIR"

echo "==> Reading $CORPUS_TOML..."

# Requires Python 3.11+ (tomllib is stdlib). Prints "name\turl" for each
# enabled repo, one per line.
enabled=$(python3 - "$CORPUS_TOML" <<'EOF'
import sys

try:
    import tomllib
except ImportError:
    sys.exit("error: Python 3.11+ is required (tomllib). Got: " + sys.version)

with open(sys.argv[1], "rb") as f:
    config = tomllib.load(f)

repos = [r for r in config.get("repos", []) if r.get("enabled", False)]
for r in repos:
    print(r["name"] + "\t" + r["url"])
EOF
)

if [ -z "$enabled" ]; then
    echo "No repos enabled in $CORPUS_TOML. Set enabled = true to activate one."
    exit 0
fi

count=0
while IFS=$'\t' read -r name url; do
    target="$CORPUS_DIR/$name"
    if [ -d "$target/.git" ]; then
        echo "==> Updating $name..."
        git -C "$target" pull --ff-only
    else
        echo "==> Cloning $name (shallow)..."
        git clone --depth 1 "$url" "$target"
    fi
    count=$((count + 1))
done <<< "$enabled"

echo "==> Done. $count repo(s) ready in $CORPUS_DIR/."
