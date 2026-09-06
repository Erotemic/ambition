#!/usr/bin/env python3
"""How much must an author SAY to use a shipped technique?

`EASE OF AUTHORING is the acceptance test` (Jon, 2026-09-05) — so it wants a
number, and the number wants a method. This prints one: for every keyed technique
the smash ruleset publishes, the size of the params struct an author fills in.

⭐ WHY FIELD COUNT AND NOT LINES. This repo's authored moves are mostly prose —
the goblin's Limit dive is nine lines of data under forty of reasoning — so a line
count measures how much a contributor explained, not how much the engine required.
The params struct is what the engine REQUIRES: name the key, fill the fields, and
the technique runs. Nothing else is owed — no system, no schedule row, no rollback
registration, no new component.

⚠ IT IS A FLOOR, NOT THE WHOLE COST. A move still authors its own timeline,
windows and volumes like any other, and a technique whose fields are cheap can
still be hard to TUNE. What this measures is the seam: what using an existing
authority costs an author who has decided to use it.

Usage:  python3 scripts/authoring_surface.py
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[1]
TECHNIQUES = REPO / "crates/ambition_characters/src"

PARAMS = re.compile(r"pub struct (\w*Params)\s*\{(.*?)\n\}", re.S)
FIELD = re.compile(r"^\s*pub (\w+):", re.M)
KEY = re.compile(r'pub const (\w+): &str = "([^"]+)"')


def surfaces():
    rows = []
    for path in sorted(TECHNIQUES.glob("smash_*.rs")):
        source = path.read_text()
        keys = [key for _, key in KEY.findall(source)]
        for match in PARAMS.finditer(source):
            rows.append(
                {
                    "module": path.stem,
                    "params": match.group(1),
                    "fields": len(FIELD.findall(match.group(2))),
                    "keys": keys,
                }
            )
    return rows


def main():
    rows = sorted(surfaces(), key=lambda r: r["fields"])
    for row in rows:
        print(f"  {row['module']:24s} {row['params']:24s} {row['fields']:2d} fields")
    counts = [row["fields"] for row in rows]
    if not counts:
        print("no techniques found — this script is measuring nothing")
        return
    counts_sorted = sorted(counts)
    median = counts_sorted[len(counts_sorted) // 2]
    print(
        f"\n{len(rows)} authored technique params across "
        f"{len({r['module'] for r in rows})} modules; "
        f"{min(counts)}–{max(counts)} fields, median {median}."
    )
    print(
        "⇒ That is the whole seam: name the key, fill the fields. No system, no "
        "schedule row, no rollback registration, no new component."
    )


if __name__ == "__main__":
    main()
