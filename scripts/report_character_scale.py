#!/usr/bin/env python3
"""**How tall does each character actually READ?**

Jon, in `docs/planning/JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`: *"In the hall of
characters, the humanoid characters are all dramatically out of scale with each
other. Alice and bob are great, but characters like the vikings, or jeff hinter
render as tiny little characters."*

⛔ **the obvious fix does not work, which is why this exists.** A character's
rendered height is `max(collision.x, collision.y) * collision_scale`
(`ambition_sprite_sheet::character::sheets::geometry::sprite_render_size`). The
frame's pixel size never enters it — only its ASPECT sets the width. So
rescaling art at the generator changes `frame_width` and `frame_height`
together, leaves the aspect alone, and moves nothing on screen.

What a viewer actually sees is the FIGURE, not the frame, so the comparable
number is

    figure_height = collision_scale * (body_pixel_bbox.h / frame_height)

per unit of collision box. Every sheet carries `body_pixel_bbox`, so this is
measured rather than eyeballed.

⭐ **Alice and Bob supply the reference, and Jon already endorsed it** — "Alice
and bob are great". Their figure height is ~1.2, and the compensating formula
`collision_scale = target / fill` reproduces Alice's authored 1.5 from her 81%
fill (1.2 / 0.81 = 1.48). A calibration that predicts the values Jon likes is
not taste; it is arithmetic with his reference in it.

⚠ **`collision_scale` is presentation-only.** The collision box is untouched, so
nothing here can change how a character plays.

⛔ **edit the YAML, never the RON.** The `.ron` sheets say "Auto-emitted from
…_spritesheet.yaml" in their first line and a regeneration would discard a RON
edit. `collision_scale` lives in the YAML too.

Usage:
    python3 scripts/report_character_scale.py            # the full table
    python3 scripts/report_character_scale.py --suggest  # + a suggested scale
"""

from __future__ import annotations

import argparse
import re
import statistics
import sys
from pathlib import Path

SHEETS = Path("crates/ambition_platformer2d_actor_monolith/assets/sprites")

# The reference Jon named. Their measured figure height is what everything else
# is compared against; it is not a number this script invented.
REFERENCE = ("alice", "bob")

FRAME_HEIGHT = re.compile(r"frame_height:\s*(\d+)")
BODY_BBOX = re.compile(
    r"body_pixel_bbox:\s*Some\(\(x:\s*\d+,\s*y:\s*\d+,\s*w:\s*\d+,\s*h:\s*(\d+)\)\)"
)
COLLISION_SCALE = re.compile(r"collision_scale:\s*([0-9.]+)")

# `SheetTuning`'s default, applied when a sheet declares no `collision_scale`.
# ⚠ 149 of 182 sheets inherit this, so "no value" is a real and common answer —
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
