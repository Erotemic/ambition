#!/usr/bin/env python3
"""How far each sheet's authored body sits from its frame centre.

⭐⭐ WHY THIS EXISTS. `sync_sprite_posed_bodies` publishes
`sprite_offset = (frame_w*0.5 - cx, frame_h*0.5 - cy) * world_per_pixel`, and
`sync_visuals` applies it as `translation.y -= offset.y` — a correction computed
for the UNTRIMMED LOGICAL FRAME. `animate_player` then overwrites the sprite's
size and anchor from the TRIMMED basis. So the correction outlives the quad it
was computed for, on every sheet-authored body.

⇒ THE ERROR IS THE OFFSET ITSELF, so its size per sheet IS the blast radius of
any fix. Measured 2026-09-06 while chasing Jon's "Mary-O is not standing on the
ground": `mary_o_v2` is **61.9%** of her body height, and the next-worst real
character is 48%. Most sheets are under 10% and many are exactly 0 — which is
why this reads as one broken character rather than a systemic tilt.

⚠ IT REPORTS, IT DOES NOT JUDGE. What offset a sheet SHOULD have is an art
question; this only says how far each one would move.

Usage: python3 scripts/measure_sheet_body_offsets.py [--top N]
"""

from __future__ import annotations

import argparse
import glob
import pathlib
import re

FRAME_W = re.compile(r"frame_width:\s*(\d+)")
FRAME_H = re.compile(r"frame_height:\s*(\d+)")
BBOX = re.compile(
    r"body_pixel_bbox:\s*Some\(\(x:\s*(\d+),\s*y:\s*(\d+),\s*w:\s*(\d+),\s*h:\s*(\d+)\)\)"
)


def rows(root: pathlib.Path):
    seen: set[str] = set()
    # ⛔ BASE SHEETS ONLY. The tree also carries `sprites_0_5x/`, `sprites_0_25x/`
    # and `sprites_potato/` variants of every sheet under the same target, so a
    # recursive walk reports one character three or four times and inflates every
    # count. Measured that way first: `puppy_slug` appeared three times.
    for path in sorted(glob.glob(str(root / "**/sprites/*_spritesheet.ron"), recursive=True)):
        text = pathlib.Path(path).read_text(errors="replace")
        fw, fh, bbox = FRAME_W.search(text), FRAME_H.search(text), BBOX.search(text)
        if not (fw and fh and bbox):
            continue
        frame_h = int(fh.group(1))
        _x, y, _w, h = (int(g) for g in bbox.groups())
        if h == 0:
            # A zero-height body is a VFX sheet with no character in it; a
            # percentage of zero is not a finding, it is a division.
            continue
        offset_y = frame_h * 0.5 - (y + h / 2)
        name = pathlib.Path(path).stem.replace("_spritesheet", "")
        # ⛔ ONE ROW PER SHEET. The same sheet is published into several asset
        # trees (`game/ambition_content/assets/sprites`, the web bundle, the
        # engine's own copy), so keying on the path counts one character three
        # times. Measured that way first: 578 "sheets" for ~190 characters.
        if name in seen:
            continue
        seen.add(name)
        yield name, int(fw.group(1)), frame_h, offset_y, h, abs(offset_y) / h * 100


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--top", type=int, default=0, help="only the N worst")
    args = parser.parse_args()
    repo = pathlib.Path(__file__).resolve().parent.parent
    measured = sorted(rows(repo / "game"), key=lambda r: -r[5])
    if not measured:
        print("⛔ no sheet manifests with a body bbox were found; the walk is empty")
        return 1
    shown = measured[: args.top] if args.top else measured
    print(f"{'sheet':38} frame          offset_y  bbox_h   as % of body")
    for name, fw, fh, oy, h, pct in shown:
        print(f"{name:38} {fw}x{fh:<9} {oy:+7.1f}  {h:>5}   {pct:5.1f}%")
    # ⚠ A PERCENTAGE OF A TINY BODY IS NOT A FINDING. A 2px VFX body is 150% off
    # by two pixels; the ratio is real and means nothing. Characters only.
    over_25 = [r for r in measured if r[5] > 25.0 and r[4] >= 24]
    print(f"\n{len(measured)} sheets measured; {len(over_25)} move more than 25% of their own body height")
    for name, _fw, _fh, oy, h, pct in over_25:
        print(f"  ⚠ {name}: {oy:+.1f}px on a {h}px body ({pct:.1f}%)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
