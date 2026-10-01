#!/usr/bin/env python3
"""Find test fixtures that pick a subject with the first match of an UNFILTERED query.

`world.query::<&T>()` followed by `.iter(world).next()` returns whichever entity
comes first in archetype order. Archetype ids follow creation order, so one
unrelated resource (an entity in Bevy 0.19) or a new component type can move
the subject. Measured 2026-10-01: that moved a rollback arm's subject from the
player to an NPC (queue row `RESOURCE-SET-SENSITIVE-RESIM`, retracted).

A filtered query (`query_filtered`, or `With<...>` in the data) is reported
separately: a filter to one entity cannot move. A site is a CANDIDATE, not a
defect: a test may not care which member it reads. Read each one.

Usage: scripts/measure_first_match_subjects.py [--list]
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ROOTS = ("game", "crates")
FIRST = re.compile(r"\.iter\((?:&?world|sim\.world\(\)|app\.world\(\))\)\s*\.next\(\)")
QUERY = re.compile(r"\.query(_filtered)?::<")


def scan(path: Path) -> list[tuple[int, bool, str]]:
    """`(line, filtered, text)` for each first-match site in `path`."""
    lines = path.read_text(encoding="utf8", errors="replace").split("\n")
    found = []
    for i, line in enumerate(lines):
        if not FIRST.search(line):
            continue
        # The query this iterator came from: the nearest `.query::<` above, in
        # the same statement window. Not a parser; a window of 6 lines.
        window = "\n".join(lines[max(0, i - 6) : i + 1])
        queries = list(QUERY.finditer(window))
        filtered = bool(queries) and (
            queries[-1].group(1) is not None or "With<" in window[queries[-1].start() :]
        )
        found.append((i + 1, filtered, line.strip()))
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--list", action="store_true", help="print each unfiltered site")
    args = parser.parse_args()
    total = unfiltered = 0
    rows = []
    for root in ROOTS:
        for path in sorted((REPO / root).rglob("*.rs")):
            if "/target/" in str(path):
                continue
            for line, filtered, text in scan(path):
                total += 1
                if not filtered:
                    unfiltered += 1
                    rows.append(f"{path.relative_to(REPO)}:{line}  {text}")
    if args.list:
        print("\n".join(rows))
    print(f"{total} first-match sites; {unfiltered} over an unfiltered query (candidates to read)")
    # ⛔ A ZERO FROM THIS SCANNER IS A FLOOR: it sees one spelling of the pattern.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
