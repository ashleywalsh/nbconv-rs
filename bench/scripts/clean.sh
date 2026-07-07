#!/usr/bin/env bash
# Remove generated files from the bench/ working directories.
#
# By default cleans comparisons/ only.
# Pass --corpus to also remove cloned corpus repos (they can be re-cloned
# with setup_corpus.sh, but this takes time on a slow connection).
#
# Never touches results/benchmarks/ — those are committed to git.
set -euo pipefail

cd "$(dirname "$0")/.."

clean_corpus=0
for arg in "$@"; do
    case "$arg" in
    --corpus) clean_corpus=1 ;;
    *)
        echo "usage: clean.sh [--corpus]"
        exit 1
        ;;
    esac
done

echo "==> Cleaning comparisons/..."
rm -rf comparisons/
mkdir -p comparisons/
echo "    done."

if [ "$clean_corpus" -eq 1 ]; then
    echo "==> Cleaning corpus/..."
    rm -rf corpus/
    mkdir -p corpus/
    echo "    done. Re-clone with: ./scripts/setup_corpus.sh"
fi

echo "==> Clean complete."
