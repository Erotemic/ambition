#!/usr/bin/env python3
"""Does every PUBLISHED part flipbook redraw its PUBLISHED sheet?
(`docs/planning/engine/mary-o-part-realization.md`.)

Reads only what shipped: each `<target>_parts.ron` with its part pages, and
the same target's `_spritesheet.yaml` with its atlas pages. Every frame of
every row is recomposed from the parts and diffed against the sheet's frame,
by the renderer's own measures at the tolerance of the flipbook's placement:

* snapped (a rig painted at frame resolution): 8 levels, no place;
* continuous (a supersampled rig): 64 levels, no place
  (`part_flipbook.CONTINUOUS_REPLAY_TOLERANCE`).

The bounds are D6's: at most 1% of drawn pixels wrong, no wrong blob over 6.
This is the census the publish's own replay guard makes, made again from the
files a build bakes in: a stale or mismatched page, a sheet republished
without its flipbook, or a row the flipbook dropped shows here.

    scripts/measure_published_flipbooks.py                 # every flipbook
    scripts/measure_published_flipbooks.py --target noether

Exit 0 when every frame is inside the bounds, 1 when one is not, 2 when it
measured nothing. Run it with the sprite renderer's interpreter.
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

import yaml  # noqa: E402
from PIL import Image  # noqa: E402

from ambition_sprite2d_renderer.authoring.part_flipbook import (  # noqa: E402
    CONTINUOUS_REPLAY_TOLERANCE,
    PLACEMENT_SNAPPED,
    PartFlipbook,
    largest_wrong_blob,
    parity,
)

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"
PARITY_BOUND = 0.01
BLOB_BOUND = 6
SNAPPED_TOLERANCE = {"threshold": 8, "radius": 0}


def _sheet_rows(sheet: dict) -> list:
    """The sheet's rows. A sparse module sheet (sandbag) lists its frames
    under ``animations`` in ``animation_order`` instead, untrimmed."""
    if "rows" in sheet:
        return sheet["rows"]
    return [
        {
            "animation": name,
            "rects": [
                {"x": f["x"], "y": f["y"], "w": f["w"], "h": f["h"], "off": [0, 0]}
                for f in sheet["animations"][name]["frames"]
            ],
        }
        for name in sheet["animation_order"]
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--target", action="append", help="repeatable; default every published flipbook")
    args = parser.parse_args()
    targets = args.target or sorted(path.name[: -len("_parts.ron")] for path in SPRITES.glob("*_parts.ron"))
    if not targets:
        print(f"no part flipbooks under {SPRITES}", file=sys.stderr)
        return 2
    failures = []
    for target in targets:
        started = time.perf_counter()
        ron = SPRITES / f"{target}_parts.ron"
        # Before schema 3 a flipbook named no placement; the only ones left
        # are the pirates', fitted (continuous).
        named = "placement:" in ron.read_text()
        flipbook = PartFlipbook.from_published(ron, None if named else "continuous")
        sheet = yaml.safe_load((SPRITES / f"{target}_spritesheet.yaml").read_text())
        pages = {}
        size = (int(sheet["frame_width"]), int(sheet["frame_height"]))
        # ⛔ Premise: the flipbook draws the sheet's frame. Compared at another
        # size, every frame would be measured against a shifted picture.
        if tuple(flipbook.frame_size) != size:
            failures.append(f"{target}: flipbook frame {flipbook.frame_size}, sheet frame {size}")
            continue
        tolerance = SNAPPED_TOLERANCE if flipbook.placement == PLACEMENT_SNAPPED else CONTINUOUS_REPLAY_TOLERANCE
        # The census is the SHEET's rows: a row the flipbook dropped is a
        # failure, not a frame skipped.
        rows = {row["animation"]: row for row in _sheet_rows(sheet)}
        missing = sorted(set(rows) - set(flipbook.clips) - set(flipbook.baked_clips))
        if missing:
            failures.append(f"{target}: rows the flipbook neither draws nor leaves baked: {missing}")
        worst = (0.0, 0, "")
        frames = 0
        for name, row in rows.items():
            if name not in flipbook.clips:
                continue
            drawn = flipbook.clips[name][1]
            if len(drawn) != len(row["rects"]):
                failures.append(f"{target} {name}: {len(drawn)} frames drawn, {len(row['rects'])} published")
                continue
            for index, rect in enumerate(row["rects"]):
                # A paged sheet names each frame's page `fpage`.
                page = int(rect.get("fpage", row.get("page", 0)))
                if page not in pages:
                    # A generator manifest names no image: it is the sheet's own.
                    names = sheet.get("images") or [sheet.get("image", f"{target}_spritesheet.png")]
                    pages[page] = Image.open(SPRITES / names[page]).convert("RGBA")
                reference = Image.new("RGBA", size, (0, 0, 0, 0))
                reference.paste(
                    pages[page].crop((rect["x"], rect["y"], rect["x"] + rect["w"], rect["y"] + rect["h"])),
                    tuple(rect.get("off", (0, 0))),
                )
                candidate = flipbook.recompose(name, index)
                wrong = parity(reference, candidate, **tolerance)
                blob = largest_wrong_blob(reference, candidate, **tolerance)
                frames += 1
                if (blob, wrong) > (worst[1], worst[0]):
                    worst = (wrong, blob, f"{name}[{index}]")
                if wrong > PARITY_BOUND or blob > BLOB_BOUND:
                    failures.append(f"{target} {name}[{index}]: {wrong:.4f} of pixels, blob {blob}")
        print(
            f"{target:32} {flipbook.placement:10} {frames:5d} frames  worst {worst[0] * 100:5.2f}% "
            f"blob {worst[1]:2d} ({worst[2]})  {time.perf_counter() - started:5.1f}s",
            flush=True,
        )
    if failures:
        print(f"{len(failures)} failure(s) outside A <= {PARITY_BOUND:.0%}, B <= {BLOB_BOUND} px:")
        for line in failures[:40]:
            print("  " + line)
        return 1
    print(f"ok: every published flipbook redraws its sheet inside A <= {PARITY_BOUND:.0%}, B <= {BLOB_BOUND} px")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
