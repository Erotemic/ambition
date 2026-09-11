#!/usr/bin/env python3
"""How big is every authored strike volume, read straight off the content files?

⭐⭐ THIS COSTS MILLISECONDS AND USED TO COST AN APP BOOT. The extents census
behind `D-STRIKE-GENEROSITY` was computed from a `moveset_takes` recording —
build the engine, boot a headless app, drive a match, parse JSON. Since the move
tables became content (fast-iteration I2 step 5) the same numbers are in
`game/ambition_content/assets/data/movesets/*.ron`, so re-running the census
after a tuning edit is a file read.

⛔ IT IS THE AUTHORED VOLUME, NOT THE RESOLVED ONE, AND THE DIFFERENCE MATTERS
FOR EXACTLY ONE THING. A sprite-manifest fighter's drawn hitbox is inflated by
its spec's `hitbox.inflate` in FRAME space, which this file knows nothing about.
⇒ Read this as "what the move table asks for", which is the number a content
edit changes, and NOT as "what the player's box is" for manifest-road fighters.

⚠ AND IT SAYS NOTHING ABOUT THE BODY. The ratio that decides whether a swing
feels like it connects is strike-area-over-BODY-area, and a body's extents come
from its sprite, not from its move table. `scripts/measure_strike_area_over_body.py`
is the instrument for that and still needs a recording; this one answers the
cheaper half — "which authored boxes are small in absolute terms, and where".

    python3 scripts/measure_authored_strike_extents.py
    python3 scripts/measure_authored_strike_extents.py --verb attack_forward
"""
from __future__ import annotations

import argparse
import pathlib
import re

MOVESETS = pathlib.Path(__file__).resolve().parent.parent / (
    "game/ambition_content/assets/data/movesets"
)

# The authored RON is generated with one field per line, so a line-oriented read
# is exact here — and a parser dependency for a generated file this regular
# would be a cost with no finding behind it. The floors below are what turn a
# silent mis-parse into a failure.
ENTITY = re.compile(r'^\s*id: "([^"]+)",\s*$')
MOVE_ID = re.compile(r'^\s*id: "([^"]+)",\s*$')
VERB = re.compile(r'^\s*"([a-z_]+)": "([^"]+)",\s*$')
ACTIVE = re.compile(r"^\s*tag: Active,\s*$")
RECT = re.compile(
    r"^\s*shape: Rect\(\s*$|^\s*Rect\("
)
HALF = re.compile(r"^\s*half_extents: \(([-\d.]+), ([-\d.]+)\),\s*$")
OFFSET = re.compile(r"^\s*offset: \(([-\d.]+), ([-\d.]+)\),\s*$")


def read(path: pathlib.Path) -> tuple[dict[str, str], dict[str, list[tuple[float, float]]]]:
    """`(verb -> move id, move id -> the half-extents of its Active volumes)`."""
    verbs: dict[str, str] = {}
    volumes: dict[str, list[tuple[float, float]]] = {}
    move: str | None = None
    in_active = False
    in_verbs = False
    for line in path.read_text().splitlines():
        if "verbs: {" in line:
            in_verbs = True
            continue
        if in_verbs:
            if line.strip().startswith("}"):
                in_verbs = False
                continue
            m = VERB.match(line)
            if m:
                verbs[m.group(1)] = m.group(2)
            continue
        m = MOVE_ID.match(line)
        if m:
            move = m.group(1)
            volumes.setdefault(move, [])
            in_active = False
            continue
        if ACTIVE.match(line):
            in_active = True
            continue
        if line.strip().startswith("tag:"):
            in_active = False
            continue
        if in_active and move:
            h = HALF.match(line)
            if h:
                volumes[move].append((float(h.group(1)), float(h.group(2))))
    return verbs, volumes


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--verb", default=None, help="report this verb only")
    args = ap.parse_args()

    files = sorted(MOVESETS.glob("*.ron"))
    if not files:
        print(f"no move tables under {MOVESETS} — nothing to census")
        return 2

    rows: list[tuple[str, str, str, float, float, float]] = []
    for path in files:
        verbs, volumes = read(path)
        # ⛔ THE FLOOR. A mis-parse yields an empty map and a clean, empty
        # report, which is this repository's most repeated instrument failure.
        if len(verbs) < 5:
            print(f"{path.name}: parsed {len(verbs)} verb(s) — the reader is broken")
            return 2
        for verb, move in sorted(verbs.items()):
            if args.verb and verb != args.verb:
                continue
            for (hx, hy) in volumes.get(move, []):
                rows.append((path.stem, verb, move, hx, hy, 4.0 * hx * hy))
    if not rows:
        print("no Active volumes matched — nothing to report")
        return 2

    # ⛔ ONE ROW PER VOLUME, AND THE COUNT SAYS SO. A multihit authors several
    # Active volumes in one move, so a reader comparing "the performer's 1512"
    # against "carl's 560" is comparing one box against one box — not a move
    # against a move. The `n` column is how many that move authors.
    per_move: dict[tuple[str, str], int] = {}
    for table, _verb, move, _hx, _hy, _area in rows:
        per_move[(table, move)] = per_move.get((table, move), 0) + 1
    rows.sort(key=lambda r: r[5])
    print(
        f"{'table':<26}{'verb':<20}{'move':<28}{'half_x':>8}{'half_y':>8}"
        f"{'area':>10}{'n':>4}"
    )
    for table, verb, move, hx, hy, area in rows:
        print(
            f"{table:<26}{verb:<20}{move:<28}{hx:>8.1f}{hy:>8.1f}{area:>10.0f}"
            f"{per_move[(table, move)]:>4}"
        )
    print(f"\n{len(rows)} authored Active volume(s) across {len(files)} table(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
