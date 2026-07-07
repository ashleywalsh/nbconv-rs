#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "nbconvert",
#   "nbformat",
# ]
# ///
"""Convert a Jupyter notebook to HTML using nbconvert and write to stdout."""

import sys

import nbformat
from nbconvert import HTMLExporter


def main() -> None:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <notebook.ipynb>", file=sys.stderr)
        sys.exit(1)

    path = sys.argv[1]
    try:
        with open(path, encoding="utf-8") as f:
            nb = nbformat.read(f, as_version=4)
    except FileNotFoundError:
        print(f"error: file not found: {path}", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"error: failed to read {path}: {e}", file=sys.stderr)
        sys.exit(1)

    try:
        exporter = HTMLExporter()
        body, _ = exporter.from_notebook_node(nb)
        sys.stdout.write(body)
    except Exception as e:
        print(f"error: conversion failed for {path}: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
