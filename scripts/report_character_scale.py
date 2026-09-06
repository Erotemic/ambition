#!/usr/bin/env python3
"""Report rendered character scale from published sprite metadata.

The report compares logical body/visual dimensions across characters so scale
regressions are visible without opening each sheet manually."""

from __future__ import annotations

import argparse
import re
import statistics
import sys
from pathlib import Path

SHEETS = Path("crates/ambition_platformer2d_actor_monolith/assets/sprites")

REFERENCE = ("alice", "bob")

FRAME_HEIGHT = re.compile(r"frame_height:\s*(\d+)")
BODY_BBOX = re.compile(
    r"body_pixel_bbox:\s*Some\(\(x:\s*\d+,\s*y:\s*\d+,\s*w:\s*\d+,\s*h:\s*(\d+)\)\)"
)
COLLISION_SCALE = re.compile(r"collision_scale:\s*([0-9.]+)")

# `SheetTuning`'s default, applied when a sheet declares no `collision_scale`.
# 149 of 182 sheets inherit this, so "no value" is a real and common answer —
# and every one of the smallest characters declares something instead.
DEFAULT_COLLISION_SCALE = 1.5


def measure(path: Path):
    text = path.read_text()
    frame = FRAME_HEIGHT.search(text)
    bbox = BODY_BBOX.search(text)
    if not (frame and bbox):
        return None
    frame_h, body_h = int(frame.group(1)), int(bbox.group(1))
    if frame_h == 0 or body_h == 0:
        # A sheet whose art is not a body — a portal, an explosion. It has no
        # figure to be out of scale with anybody.
        return None
    scale_match = COLLISION_SCALE.search(text)
    scale = float(scale_match.group(1)) if scale_match else DEFAULT_COLLISION_SCALE
    fill = body_h / frame_h
    return {
        "name": path.name[: -len("_spritesheet.ron")],
        "scale": scale,
        "explicit": scale_match is not None,
        "fill": fill,
        "figure": scale * fill,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--suggest",
        action="store_true",
        help="also print the collision_scale that would put each figure at the reference height "
        "(⚠ meaningful ONLY for characters that are supposed to be human-height)",
    )
    args = parser.parse_args()

    if not SHEETS.is_dir():
        print(f"no sheet directory at {SHEETS} — run from the repo root", file=sys.stderr)
        return 2

    rows = [m for p in sorted(SHEETS.glob("*_spritesheet.ron")) if (m := measure(p))]
    if not rows:
        print("no sheets carried a body bbox", file=sys.stderr)
        return 2

    reference = [r["figure"] for r in rows if r["name"] in REFERENCE]
    target = statistics.mean(reference) if reference else statistics.median(
        r["figure"] for r in rows
    )

    rows.sort(key=lambda r: r["figure"])
    width = max(len(r["name"]) for r in rows)
    header = f"{'figure':>7}  {'fill':>5}  {'scale':>6}  name"
    if args.suggest:
        header += "".ljust(width - 4) + "  suggested"
    print(header)
    for r in rows:
        mark = " " if r["explicit"] else "·"
        line = f"{r['figure']:>7.2f}  {r['fill']:>4.0%}  {r['scale']:>5}{mark}  {r['name']:<{width}}"
        if args.suggest:
            line += f"  {target / r['fill']:.2f}"
        print(line)

    figures = [r["figure"] for r in rows]
    print()
    print(f"{len(rows)} sheets with a figure; · = inherits the {DEFAULT_COLLISION_SCALE} default")
    print(
        f"reference ({'/'.join(REFERENCE)}) = {target:.2f}   "
        f"median {statistics.median(figures):.2f}   "
        f"spread {min(figures):.2f}..{max(figures):.2f} = {max(figures) / min(figures):.1f}x"
    )
    print("⛔ apply changes to the .yaml, never the auto-emitted .ron")
    if args.suggest:
        print(
            "⚠ the suggested column assumes the character SHOULD read at the reference\n"
            "  height. That is false for a snake, an explosion, a pipe, or a deliberately\n"
            "  chibi robot — Jon's report is about the HUMANOID cast, and this script\n"
            "  cannot tell a viking from a puppy slug. Read it as 'what would make this\n"
            "  the same height as Alice', not as 'what this should be.'"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
