#!/usr/bin/env python3
"""Re-format an LDtk JSON file produced by `json.dumps(indent=2)` into
the LDtk editor's mixed style:

- arrays of pure numbers / bools / nulls → inline (`[true]`,
  `[1, 2, 3]`, `[]`),
- arrays containing strings or objects → keep the multi-line indented
  form Python's `json.dumps(indent=2)` produces.

Run after any tool that writes the LDtk file via stock `json.dumps` so
the on-disk diff against an LDtk-editor-produced baseline stays small.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


# Match a multi-line array containing only numeric/bool/null scalars.
# Strings and objects keep the multi-line layout.
SCALAR_RE = r"(?:-?\d+(?:\.\d+)?|true|false|null)"

ARRAY_RE = re.compile(
    r"\[\n"
    r"(?P<indent>[ \t]+)"
    r"(?P<first>" + SCALAR_RE + r")"
    r"(?P<rest>(?:,\s*\n[ \t]+" + SCALAR_RE + r")*)"
    r"\s*\n[ \t]+\]"
)


def collapse_match(match: re.Match) -> str:
    first = match.group("first")
    rest_text = match.group("rest") or ""
    items = [first]
    for chunk in rest_text.split(","):
        item = chunk.strip()
        if item:
            items.append(item)
    return "[" + ", ".join(items) + "]"


def compact(text: str) -> tuple[str, int]:
    new_text, count = ARRAY_RE.subn(collapse_match, text)
    return new_text, count


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("path", type=Path)
    args = parser.parse_args(argv)

    text = args.path.read_text()
    new_text, count = compact(text)
    if count == 0:
        print(f"no compact-eligible arrays found in {args.path}")
        return 0
    args.path.write_text(new_text)
    print(f"compacted {count} multi-line arrays in {args.path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
