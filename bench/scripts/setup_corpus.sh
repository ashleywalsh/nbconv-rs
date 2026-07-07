#!/usr/bin/env bash
# Clone or update all repos defined in corpus.toml.
#
# All repos are cloned regardless of their 'enabled' flag — 'enabled' only
# controls which repos are used during benchmarking and comparison, not which
# ones are stored locally. This means you can flip 'enabled' without re-cloning.
#
# Safe to re-run: existing repos are pulled, not re-cloned.
set -euo pipefail

cd "$(dirname "$0")/.."

CORPUS_DIR="corpus"
CORPUS_TOML="corpus.toml"

mkdir -p "$CORPUS_DIR"

echo "==> Reading $CORPUS_TOML..."

# Requires Python 3.11+ (tomllib is stdlib). Prints "name\turl" for every
# repo in corpus.toml, regardless of enabled status.
all_repos=$(
    python3 - "$CORPUS_TOML" <<'EOF'
import sys

try:
    import tomllib
except ImportError:
    sys.exit("error: Python 3.11+ is required (tomllib). Got: " + sys.version)

with open(sys.argv[1], "rb") as f:
    config = tomllib.load(f)

for r in config.get("repos", []):
    print(r["name"] + "\t" + r["url"])
EOF
)

if [ -z "$all_repos" ]; then
    echo "No repos defined in $CORPUS_TOML."
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
done <<<"$all_repos"

echo "==> Done. $count repo(s) ready in $CORPUS_DIR/."
echo "    Use 'enabled = true/false' in corpus.toml to select which are benchmarked."
