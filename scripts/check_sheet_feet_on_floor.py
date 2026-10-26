#!/usr/bin/env python3
"""Check that every published character sheet draws its grounded feet on its body's floor.

The game stands a body on the bottom of the body box its sheet publishes and
draws the frame on that box, so paint the box holds under the soles is air
between the feet and the floor (Jon, 2026-10-26: "sanic, his feet hover over
the ground"; "Fix the other hover characters"). For each sheet with an
``idle`` row, and for each grounded row it draws (``GROUNDED``), the lowest
SOLID pixel of the row (alpha >= ``SOLID``: paint, not a glow and not the
resampling fringe) must be within ``TOLERANCE`` px of:

* the standing floor, the bottom of ``body_pixel_bbox`` (where the feet anchor
  is, and the idle body a posed body stands on), for the rows a standing body
  is drawn in (``LOCOMOTION``); and
* the row's own body bottom, when the sheet publishes one for the row.

A sheet is a body when its actor sidecar names a body plan that is one: a
sheet with no sidecar, no plan, or a plan in ``NOT_BODIES`` (props, held
items, effects) has no floor and is skipped. A
body that hovers by design is named in ``HOVERS`` with its reason; a new
hovering character fails until it is named there or fixed.

Usage::

    python3 scripts/check_sheet_feet_on_floor.py
    python3 scripts/check_sheet_feet_on_floor.py --sprites <dir> --all

Exit 1 lists each sheet off its floor, with the gap of each row in px
(positive: the feet are above the floor)."""

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

SPRITES = REPO / "crates/ambition_platformer2d_actor_monolith/assets/sprites"

#: The least alpha of paint that is part of the figure.
SOLID = 192
TOLERANCE = 2
#: Rows a standing body is drawn in on its standing box.
LOCOMOTION = ("idle", "walk", "run")
#: Rows with a foot on the ground in each frame.
GROUNDED = LOCOMOTION + ("crouch", "skid", "block")

#: Actor body plans of sheets that are not a body standing on a floor.
NOT_BODIES = {"Prop", "StaticProp", "Effect"}

#: Bodies whose drawn feet are off the floor of their box by design, and why.
HOVERS: dict[str, str] = {}


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


def _box_bottom(box: dict | None) -> int | None:
    return None if not box or not box.get("h") else int(box["y"]) + int(box["h"])


def census(sprites: Path) -> list[tuple[str, dict[str, dict[str, int]]]]:
    """Every sheet with an idle row: ``(target, {row: {floor: gap px}})``."""
    out = []
    for path in sorted(sprites.glob("*_spritesheet.ron")):
        if _body_plan(sprites, path.name[: -len("_spritesheet.ron")]) in NOT_BODIES | {None}:
            continue
        sheet = parse_ron(path.read_text())
        sheet = sheet[0] if isinstance(sheet, list) else sheet
        rows = {row["animation"]: row for row in sheet.get("rows") or []}
        metrics = sheet.get("body_metrics") or {}
        standing = _box_bottom(metrics.get("body_pixel_bbox"))
        if "idle" not in rows or standing is None:
            continue
        images = sheet.get("images") or [sheet["image"]]
        pages = [np.asarray(Image.open(sprites / name).convert("RGBA")) for name in images]
        poses = metrics.get("animations") or {}
        gaps: dict[str, dict[str, int]] = {}
        for name in GROUNDED:
            if name not in rows:
                continue
            sole = _row_bottom(pages, rows[name])
            if sole is None:
                continue
            gap = {}
            if name in LOCOMOTION:
                gap["standing"] = standing - sole
            own = _box_bottom(((poses.get(name) or {}).get("hurtbox") or {}).get("bbox"))
            if own is not None:
                gap["own"] = own - sole
            gaps[name] = gap
        out.append((path.name[: -len("_spritesheet.ron")], gaps))
    return out


def worst(gaps: dict[str, dict[str, int]]) -> int:
    return max((abs(px) for gap in gaps.values() for px in gap.values()), default=0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sprites", type=Path, default=SPRITES)
    parser.add_argument("--all", action="store_true", help="print every sheet, not only those off the floor")
    args = parser.parse_args()
    off = []
    rows = census(args.sprites)
    for target, gaps in rows:
        bad = worst(gaps) > TOLERANCE and target not in HOVERS
        if bad:
            off.append(target)
        if bad or args.all:
            shown = ", ".join(f"{row} {gap}" for row, gap in gaps.items())
            print(f"{'OFF ' if bad else 'ok  '}{target:44s} worst {worst(gaps):3d} px  {shown}")
    print(f"{len(rows)} bodies with an idle row; {len(off)} off the floor by more than {TOLERANCE} px")
    if off:
        print("Fix the body box at its generator (measure the body, not what it gives off), "
              "or name a body that hovers by design in HOVERS with its reason.")
    return 1 if off else 0


if __name__ == "__main__":
    sys.exit(main())
