#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "beautifulsoup4",
#   "lxml",
# ]
# ///
"""
Compare nbconv and nbconvert output for every enabled notebook.

For each notebook:
  1. Runs both tools and records success / failure.
  2. Normalises both HTML outputs with BeautifulSoup (consistent whitespace
     and attribute ordering).
  3. Writes a unified diff of the normalised HTML.
  4. Writes a side-by-side HTML diff using difflib.HtmlDiff.

Outputs go to comparisons/<repo>/<notebook>/ (gitignored).
A summary of pass/fail results is printed to stdout.
"""

import difflib
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import tomllib
from bs4 import BeautifulSoup

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


# ── HTML normalisation ────────────────────────────────────────────────────────


def normalise(html: str) -> str:
    """Parse and pretty-print HTML with lxml for consistent formatting."""
    soup = BeautifulSoup(html, "lxml")
    return soup.prettify()


# ── Diff generation ───────────────────────────────────────────────────────────


def unified_diff(a: str, b: str, name_a: str, name_b: str) -> str:
    return "\n".join(
        difflib.unified_diff(
            a.splitlines(),
            b.splitlines(),
            fromfile=name_a,
            tofile=name_b,
            lineterm="",
        )
    )


def html_diff(a: str, b: str, name_a: str, name_b: str) -> str:
    """Render a unified diff as a standalone HTML page.

    difflib.HtmlDiff cannot be used here because its _split_line method
    recurses on '<' and '>' characters, causing a RecursionError when the
    input is itself HTML.
    """
    import html as html_lib

    lines = list(
        difflib.unified_diff(
            a.splitlines(),
            b.splitlines(),
            fromfile=name_a,
            tofile=name_b,
            lineterm="",
        )
    )

    def row(line: str) -> str:
        if line.startswith(("---", "+++")):
            cls = "header"
        elif line.startswith("@@"):
            cls = "hunk"
        elif line.startswith("+"):
            cls = "add"
        elif line.startswith("-"):
            cls = "del"
        else:
            cls = "ctx"
        return f'<div class="{cls}">{html_lib.escape(line)}</div>'

    body = "\n".join(row(l) for l in lines) if lines else "<p>No differences.</p>"

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Diff: {html_lib.escape(name_a)} vs {html_lib.escape(name_b)}</title>
  <style>
    body   {{ font-family: monospace; font-size: 12px; margin: 1rem 2rem; }}
    h1     {{ font-family: sans-serif; font-size: 16px; }}
    .diff  {{ white-space: pre-wrap; word-break: break-all; }}
    .add    {{ background: #e6ffed; }}
    .del    {{ background: #ffeef0; }}
    .hunk   {{ background: #dbedff; color: #0366d6; }}
    .header {{ color: #888; }}
  </style>
</head>
<body>
  <h1>Diff: {html_lib.escape(name_a)} vs {html_lib.escape(name_b)}</h1>
  <div class="diff">{body}</div>
</body>
</html>"""


# ── Per-notebook comparison ───────────────────────────────────────────────────


def compare_notebook(repo: str, nb: Path) -> NotebookResult:
    result = NotebookResult(repo=repo, name=nb.stem)

    print(f"  nbconv    ... ", end="", flush=True)
    result.nbconv = run_nbconv(nb)
    print("ok" if result.nbconv.success else "FAILED")

    print(f"  nbconvert ... ", end="", flush=True)
    result.nbconvert = run_nbconvert(nb)
    print("ok" if result.nbconvert.success else "FAILED")

    out_dir = COMPARISONS_DIR / repo / nb.stem
    out_dir.mkdir(parents=True, exist_ok=True)

    # Always write raw HTML outputs
    (out_dir / "nbconv.html").write_text(result.nbconv.html, encoding="utf-8")
    (out_dir / "nbconvert.html").write_text(result.nbconvert.html, encoding="utf-8")

    # Only diff if both tools succeeded
    if result.nbconv.success and result.nbconvert.success:
        norm_a = normalise(result.nbconv.html)
        norm_b = normalise(result.nbconvert.html)

        patch = unified_diff(norm_a, norm_b, "nbconv", "nbconvert")
        (out_dir / "diff_normalized.patch").write_text(patch, encoding="utf-8")

        side_by_side = html_diff(norm_a, norm_b, "nbconv", "nbconvert")
        (out_dir / "diff.html").write_text(side_by_side, encoding="utf-8")
    else:
        if not result.nbconv.success:
            (out_dir / "nbconv.stderr").write_text(
                result.nbconv.stderr, encoding="utf-8"
            )
        if not result.nbconvert.success:
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
    print(f"==> Comparing {total} notebook(s)\n")

    results: list[NotebookResult] = []
    for i, (repo, nb) in enumerate(notebooks, 1):
        print(f"[{i}/{total}] {repo}/{nb.stem}")
        results.append(compare_notebook(repo, nb))
        print()

    # ── Summary ───────────────────────────────────────────────────────────────
    nbconv_ok = sum(1 for r in results if r.nbconv.success)
    nbconvert_ok = sum(1 for r in results if r.nbconvert.success)
    both_ok = sum(1 for r in results if r.nbconv.success and r.nbconvert.success)

    print("==> Summary")
    print(f"    nbconv    : {nbconv_ok}/{total} succeeded")
    print(f"    nbconvert : {nbconvert_ok}/{total} succeeded")
    print(f"    both ok   : {both_ok}/{total} (diffs generated)")
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
