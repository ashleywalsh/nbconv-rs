#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "nbconvert",
#   "nbformat",
# ]
# ///
"""
Run nbconv and nbconvert on every enabled notebook and save both HTML outputs.

For each enabled notebook, writes to comparisons/<repo>/<notebook>/:
  nbconv.html     — output from nbconv
  nbconvert.html  — output from nbconvert

If a tool fails, its stderr is saved as <tool>.stderr and the other tool's
output is still written. A summary of pass/fail results is printed at the end.

Open nbconv.html and nbconvert.html side-by-side in a browser to compare.
"""

import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import tomllib

# ── Paths ─────────────────────────────────────────────────────────────────────

BENCH_DIR = Path(__file__).parent.parent
CORPUS_TOML = BENCH_DIR / "corpus.toml"
CORPUS_DIR = BENCH_DIR / "corpus"
COMPARISONS_DIR = BENCH_DIR / "comparisons"
NBCONV = BENCH_DIR.parent / "target" / "release" / "nbconv"
NBCONVERT_SCRIPT = BENCH_DIR / "scripts" / "nbconvert_html.py"

# ── Data ──────────────────────────────────────────────────────────────────────


@dataclass
class ToolResult:
    success: bool
    html: str
    stderr: str


@dataclass
class NotebookResult:
    repo: str
    name: str
    nbconv: ToolResult = field(default=None)
    nbconvert: ToolResult = field(default=None)


# ── Corpus loading ────────────────────────────────────────────────────────────


def load_notebooks() -> list[tuple[str, Path]]:
    with open(CORPUS_TOML, "rb") as f:
        config = tomllib.load(f)

    notebooks = []
    for repo in config.get("repos", []):
        if not repo.get("enabled", False):
            continue
        repo_dir = CORPUS_DIR / repo["name"]
        if not repo_dir.exists():
            print(
                f"warning: {repo_dir} not found — run setup_corpus.sh first",
                file=sys.stderr,
            )
            continue
        for nb in sorted(repo_dir.glob(repo["glob"])):
            notebooks.append((repo["name"], nb))
    return notebooks


# ── Tool runners ──────────────────────────────────────────────────────────────


def run_nbconv(nb: Path) -> ToolResult:
    result = subprocess.run(
        [str(NBCONV), str(nb)],
        capture_output=True,
        text=True,
    )
    return ToolResult(
        success=result.returncode == 0,
        html=result.stdout,
        stderr=result.stderr,
    )


def run_nbconvert(nb: Path) -> ToolResult:
    result = subprocess.run(
        ["uv", "run", "--script", str(NBCONVERT_SCRIPT), str(nb)],
        capture_output=True,
        text=True,
    )
    return ToolResult(
        success=result.returncode == 0,
        html=result.stdout,
        stderr=result.stderr,
    )


# ── Per-notebook processing ───────────────────────────────────────────────────


def process_notebook(repo: str, nb: Path) -> NotebookResult:
    result = NotebookResult(repo=repo, name=nb.stem)

    print(f"  nbconv    ... ", end="", flush=True)
    result.nbconv = run_nbconv(nb)
    print("ok" if result.nbconv.success else "FAILED")

    print(f"  nbconvert ... ", end="", flush=True)
    result.nbconvert = run_nbconvert(nb)
    print("ok" if result.nbconvert.success else "FAILED")

    out_dir = COMPARISONS_DIR / repo / nb.stem
    out_dir.mkdir(parents=True, exist_ok=True)

    if result.nbconv.success:
        (out_dir / "nbconv.html").write_text(result.nbconv.html, encoding="utf-8")
    else:
        (out_dir / "nbconv.stderr").write_text(result.nbconv.stderr, encoding="utf-8")

    if result.nbconvert.success:
        (out_dir / "nbconvert.html").write_text(result.nbconvert.html, encoding="utf-8")
    else:
        (out_dir / "nbconvert.stderr").write_text(
            result.nbconvert.stderr, encoding="utf-8"
        )

    return result


# ── Main ──────────────────────────────────────────────────────────────────────


def main() -> None:
    if not NBCONV.exists():
        sys.exit(
            f"error: nbconv binary not found at {NBCONV}\n"
            "       Run 'cargo build --release' from the workspace root."
        )

    notebooks = load_notebooks()
    if not notebooks:
        sys.exit("No notebooks found. Run ./scripts/setup_corpus.sh first.")

    total = len(notebooks)
    print(f"==> Processing {total} notebook(s)\n")

    results: list[NotebookResult] = []
    for i, (repo, nb) in enumerate(notebooks, 1):
        print(f"[{i}/{total}] {repo}/{nb.stem}")
        results.append(process_notebook(repo, nb))
        print()

    # ── Summary ───────────────────────────────────────────────────────────────
    nbconv_ok = sum(1 for r in results if r.nbconv.success)
    nbconvert_ok = sum(1 for r in results if r.nbconvert.success)

    print("==> Summary")
    print(f"    nbconv    : {nbconv_ok}/{total} succeeded")
    print(f"    nbconvert : {nbconvert_ok}/{total} succeeded")
    print(f"\n    Output: {COMPARISONS_DIR}/")

    failures = [r for r in results if not r.nbconv.success or not r.nbconvert.success]
    if failures:
        print("\n    Failures:")
        for r in failures:
            tools = []
            if not r.nbconv.success:
                tools.append("nbconv")
            if not r.nbconvert.success:
                tools.append("nbconvert")
            print(f"      {r.repo}/{r.name}: {', '.join(tools)}")


if __name__ == "__main__":
    main()
