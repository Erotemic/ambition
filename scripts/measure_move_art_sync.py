#!/usr/bin/env python3
"""Does a move hit when its picture hits?

A move's sheet row is slaved to the move's progress (`ClipRequest::for_move`):
frame = progress * frames. So the part of the row where the limb is out must be
the part of the move where its volume is live (the `Active` window), or the
player sees a hit before or after the swing.

For each move of a table this prints: the live part of the move (from the
table), the part of the row where the picture reaches farthest (from the
published sheet's trimmed frame rects, against the row's first frame), how
much of the live part is inside it, and a window that would match the picture.

    python scripts/measure_move_art_sync.py \
        game/ambition_content/assets/data/movesets/gnu_ton.ron \
        game/ambition_content_library/generated/runtime/sprites/gnu_ton_scholar_fighter_spritesheet.yaml

A measure of the PUBLISHED sheet: publish the sheet before you trust it. It
reads reach from the trimmed rect of a frame, so an effect drawn far from the
body in a frame reads as reach.
"""
from __future__ import annotations

import re
import sys

import yaml

#: A frame is "out" when its reach is at least this part of the row's largest.
OUT = 0.75


def moves_of(text: str):
    """(id, clip, duration, [(start, end)] of the Active windows) per move."""
    out = []
    for block in re.split(r"\n {24}\(\n", text)[1:]:
        ident = re.search(r'\bid: "([^"]+)"', block)
        clip = re.search(r'clip: \(\s*clip: "([^"]+)"', block)
        duration = re.search(r"duration_s: ([0-9.]+)", block)
        if not (ident and clip and duration):
            continue
        active = []
        for window in re.finditer(r"start_s: ([0-9.]+),\s*end_s: ([0-9.]+),\s*tag: Active,\s*volumes: \[(\s*\()?", block):
            if window.group(3):
                active.append((float(window.group(1)), float(window.group(2))))
        out.append((ident.group(1), clip.group(1), float(duration.group(1)), active))
    return out


def reach_of(row) -> list[float]:
    """How far each frame of a row reaches past the row's first frame, any way."""
    rects = row["rects"]

    def box(r):
        x, y = r["off"]
        return x, y, x + r["w"], y + r["h"]

    x0, y0, x1, y1 = box(rects[0])
    out = []
    for r in rects:
        a, b, c, d = box(r)
        out.append(max(c - x1, x0 - a, y0 - b, d - y1, 0.0))
    return out


def main() -> int:
    table, sheet = sys.argv[1], sys.argv[2]
    rows = {r["animation"]: r for r in yaml.safe_load(open(sheet))["rows"]}
    print(f"{'move':24} {'row':16} {'dur':>5} {'live (s)':>13} {'picture out':>13} {'inside':>6}  matched window")
    worst = 0
    for ident, clip, duration, active in moves_of(open(table).read()):
        if not active or clip not in rows:
            continue
        reach = reach_of(rows[clip])
        top = max(reach)
        if top <= 0:
            continue
        n = len(reach)
        out_frames = [i for i, r in enumerate(reach) if r >= OUT * top]
        art = (out_frames[0] / n * duration, (out_frames[-1] + 1) / n * duration)
        start, end = active[0][0], active[-1][1]
        inside = max(0.0, min(end, art[1]) - max(start, art[0])) / max(end - start, 1e-6)
        worst += inside < 0.5
        print(f"{ident:24} {clip:16} {duration:5.2f} {start:6.3f}-{end:5.3f} {art[0]:6.3f}-{art[1]:5.3f} {inside:6.0%}  {art[0]:.3f}..{art[1]:.3f}")
    print(f"\n{worst} move(s) have less than half of their live part inside the picture's.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
