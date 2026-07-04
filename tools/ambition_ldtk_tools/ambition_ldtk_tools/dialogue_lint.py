#!/usr/bin/env python3
"""Lint Yarn dialogue for malformed markup tags.

Yarn parses `[...]` as **markup** and does so lazily — when a line is
*delivered* at runtime, not at compile time. So a bracketed stage direction
like `[MULTIPLE VOICES]` compiles fine but panics the running game the
moment that line is shown:

    Error in Yarn Spinner plugin: Expected a = inside markup in line
    "Agent Swarm: [MULTIPLE VOICES] ..."

Inside `[name ...]`, after the tag name every whitespace-separated token must
be a `key=value` property (or the tag ends in `]` / `/]`). A bare word makes
the parser expect a `=` and fail. This is the authoritative Rust guard
`ambition_gameplay_core::dialog_lint::no_malformed_yarn_markup_tags` ported to
Python so `run_game.sh` can fail fast before a build + launch.

Fixes: use `(parens)` for stage directions, escape literal brackets as
`\\[...\\]`, or write a real paired tag like `[shout]...[/shout]`.

Usage:
    ambition_ldtk_tools dialogue lint [--root DIR]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/dialogue_lint.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_ROOT = REPO_ROOT / "crates" / "ambition_content" / "assets" / "dialogue"

# Unescaped `[` ... up to the next `]`.
_SPAN_RE = re.compile(r"(?<!\\)\[([^\]]*)\]")


def markup_inner_well_formed(inner: str) -> bool:
    """True if the text between `[` and `]` is a well-formed Yarn marker."""
    if inner == "/":
        return True  # close-all
    if inner.startswith("/"):
        name = inner[1:]
        return bool(name) and not any(c.isspace() for c in name)
    body = inner[:-1] if inner.endswith("/") else inner  # strip self-close
    tokens = body.split()
    if not tokens:
        return False  # `[]`
    # First token is the tag name (maybe `name=value`); the rest must be
    # `key=value` properties.
    return all("=" in t for t in tokens[1:])


def lint_text(label: str, text: str) -> list[str]:
    violations: list[str] = []
    for n, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        if (
            stripped.startswith("title:")
            or stripped in ("---", "===")
            or stripped.startswith("//")
        ):
            continue
        for m in _SPAN_RE.finditer(line):
            if not markup_inner_well_formed(m.group(1)):
                violations.append(
                    f"{label}:{n}: malformed Yarn markup tag `{m.group(0)}` — a "
                    f"bracketed token without `=` makes the runtime panic "
                    f'("Expected a = inside markup") when this line is shown. '
                    f"Use (parens) for stage directions, escape as \\[...\\], or "
                    f"write a real [tag]...[/tag]."
                )
    return violations


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    args = parser.parse_args(argv)

    files = sorted(args.root.rglob("*.yarn"))
    if not files:
        print(f"error: no .yarn files under {args.root}", file=sys.stderr)
        return 2

    violations: list[str] = []
    for f in files:
        try:
            label = str(f.relative_to(REPO_ROOT))
        except ValueError:
            label = str(f)
        violations.extend(lint_text(label, f.read_text()))

    if violations:
        print("Yarn markup violations (each crashes the running game):", file=sys.stderr)
        for v in violations:
            print(f"  {v}", file=sys.stderr)
        return 1

    print(f"OK: {len(files)} Yarn file(s) have no malformed markup tags")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
