#!/usr/bin/env python3
"""Check that every published body sheet draws its grounded feet on its body's floor.

The game stands a body on the bottom of the body box its sheet publishes
(``body_pixel_bbox``: where the feet anchor is) and draws every row on that box;
a crouch keeps its feet planted on it. So paint the box holds under the soles is
air between the feet and the floor, and a sole drawn under it is in the floor
(Jon, 2026-10-26: "sanic, his feet hover over the ground"; "Fix the other hover
characters"). For each row a body stands in (``GROUNDED``), the lowest SOLID
pixel of the row (alpha >= ``SOLID``: paint, not a glow and not the resampling
fringe) must be within ``TOLERANCE`` px of the bottom of the box.

A sheet is a body when its actor sidecar names a body plan that is one: a sheet
with no sidecar, no plan, or a plan in ``NOT_BODIES`` (props, held items,
effects) has no floor and is skipped. A body whose drawing is off its floor by
design is named in ``HOVERS`` with its reason, so a new hovering character fails
until it is named there or fixed.

Usage::

    python3 scripts/check_sheet_feet_on_floor.py
    python3 scripts/check_sheet_feet_on_floor.py --sprites <dir> --all

Exit 1 lists each body off its floor, with the gap of each row in px
(positive: the feet are above the floor; negative: in it)."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

import numpy as np
from PIL import Image

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools/ambition_sprite2d_renderer"))
from ambition_sprite2d_renderer.core.equivalence import parse_ron  # noqa: E402

SPRITES = REPO / "game/ambition_content_library/generated/runtime/sprites"

#: The least alpha of paint that is part of the figure.
SOLID = 192
TOLERANCE = 2
#: The rows a body stands, walks and runs in: a foot on the ground in each
#: frame (in some frame, for a run). Not the crouch, the block or the skid: the
#: lowest paint of those is as often a wing, a tail or a cape as a sole.
GROUNDED = ("idle", "walk", "run")
#: Actor body plans of sheets that are not a body standing on a floor.
NOT_BODIES = {"Prop", "StaticProp", "Effect"}

#: Bodies whose drawing is off the floor of their box by design, and why.
HOVERS: dict[str, str] = {
    "agent_swarm": "seven small flying robots: the box reaches down to the ground under the cloud, so a body on the "
                   "ground runs into it, and the cloud floats above it",
    "burning_flying_shark": "a flyer: its fins and flames hang under its body",
    "exploding_mite": "the old bomb drone (the robot generator's diver): it hovers on its jets. The game draws "
                      "exploding_mite_v2",
    "flying_spaghetti_monster_boss": "a flyer: its body is the bell, and the noodles hang far under it",
    "flying_spaghetti_monster_boss_sauce": "the sauce of the boss's noodles, drawn on the boss's body (its sheet "
                                           "states the boss's body)",
    "mary_o_v2_cosmic": "her soles are on the floor; the tips of her wings hang down to it and a little past",
    "wolfenstein_inugami": "a floating flaming head: its body is the head, and the flame trails far under it",
}


def _body_plan(sprites: Path, target: str) -> str | None:
    actor = sprites / f"{target}_actor.ron"
    if not actor.exists():
        return None
    found = re.search(r'body_plan: Some\("(\w+)"\)', actor.read_text())
    return found.group(1) if found else None


def _row_bottom(pages: list[np.ndarray], row: dict) -> int | None:
    """One past the lowest solid pixel of ``row``, in frame pixels."""
    lowest = None
    for rect in row["rects"]:
        alpha = pages[rect.get("page", 0)][rect["y"]:rect["y"] + rect["h"], rect["x"]:rect["x"] + rect["w"], 3]
        solid = np.flatnonzero((alpha >= SOLID).any(axis=1))
        if solid.size:
            bottom = int(rect.get("off", (0, 0))[1]) + int(solid[-1]) + 1
            lowest = bottom if lowest is None else max(lowest, bottom)
    return lowest


def census(sprites: Path) -> list[tuple[str, dict[str, int]]]:
    """Every body with an idle row: ``(target, {row: gap px})``."""
    out = []
    for path in sorted(sprites.glob("*_spritesheet.ron")):
        target = path.name[: -len("_spritesheet.ron")]
        if _body_plan(sprites, target) in NOT_BODIES | {None}:
            continue
        sheet = parse_ron(path.read_text())
        sheet = sheet[0] if isinstance(sheet, list) else sheet
        rows = {row["animation"]: row for row in sheet.get("rows") or []}
        box = (sheet.get("body_metrics") or {}).get("body_pixel_bbox")
        if "idle" not in rows or not box or not box.get("h"):
            continue
        floor = int(box["y"]) + int(box["h"])
        images = sheet.get("images") or [sheet["image"]]
        pages = [np.asarray(Image.open(sprites / name).convert("RGBA")) for name in images]
        gaps = {}
        for name in GROUNDED:
            sole = _row_bottom(pages, rows[name]) if name in rows else None
            if sole is not None:
                gaps[name] = floor - sole
        out.append((target, gaps))
    return out


def worst(gaps: dict[str, int]) -> int:
    return max((abs(px) for px in gaps.values()), default=0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sprites", type=Path, default=SPRITES)
    parser.add_argument("--all", action="store_true", help="print every body, not only those off the floor")
    args = parser.parse_args()
    off = []
    rows = census(args.sprites)
    for target, gaps in rows:
        bad = worst(gaps) > TOLERANCE and target not in HOVERS
        if bad:
            off.append(target)
        if bad or args.all:
            mark = "OFF " if bad else ("hov " if target in HOVERS else "ok  ")
            print(f"{mark}{target:44s} worst {worst(gaps):3d} px  {gaps}")
    print(f"{len(rows)} bodies with an idle row; {len(off)} off the floor by more than {TOLERANCE} px")
    if off:
        print("Fix the body box or the art at its generator, or name a body that is off its floor by design "
              "in HOVERS with its reason.")
    return 1 if off else 0


if __name__ == "__main__":
    sys.exit(main())
