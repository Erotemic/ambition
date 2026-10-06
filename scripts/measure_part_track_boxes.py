#!/usr/bin/env python3
"""Where a character's named parts are drawn, frame by frame, measured from its
published part flipbook (`<target>_parts.ron`).

For each requested row and track, prints every frame's drawn bounding box of
that track's part (its rect turned and scaled about its pivot, placed at the
draw's `at` from the feet), in frame pixels (origin top left, +y down, art
facing right). A boss conductor's volumes are authored from these: the T-rex's
bite is where his jaw is on the bite's strike frames, not a guess.

    python3 scripts/measure_part_track_boxes.py trex_enemy --row bite --track jaw --track head
"""

from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tools" / "ambition_sprite2d_renderer"))

from ambition_sprite2d_renderer.authoring.part_flipbook import PartFlipbook  # noqa: E402

SPRITES = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "assets" / "sprites"


def draw_box(flipbook: PartFlipbook, draw) -> tuple[float, float, float, float]:
    """The draw's bounding box in frame pixels."""
    part = flipbook.parts[draw.part]
    w, h = part.image.size
    px, py = part.pivot
    sx, sy = draw.scale
    c, s = math.cos(draw.rotation), math.sin(draw.rotation)
    fx, fy = flipbook.feet
    xs, ys = [], []
    for cx, cy in ((0, 0), (w, 0), (0, h), (w, h)):
        lx, ly = (cx - px) * sx, (cy - py) * sy
        # Clockwise, +y down.
        xs.append(fx + draw.at[0] + lx * c - ly * s)
        ys.append(fy + draw.at[1] + lx * s + ly * c)
    return min(xs), min(ys), max(xs), max(ys)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("target")
    parser.add_argument("--row", action="append", required=True)
    parser.add_argument("--track", action="append", required=True)
    args = parser.parse_args()
    path = SPRITES / f"{args.target}_parts.ron"
    flipbook = PartFlipbook.from_published(path)
    names = sorted({d.track for _, frames in flipbook.clips.values() for draws in frames for d in draws if d.track})
    missing = [t for t in args.track if t not in names]
    if missing:
        print(f"{args.target} has no track {missing}; it has {names}", file=sys.stderr)
        return 2
    print(f"{args.target}: frame {flipbook.frame_size}, feet {flipbook.feet}")
    for row in args.row:
        if row not in flipbook.clips:
            print(f"{args.target} has no row {row!r}; it has {list(flipbook.clips)}", file=sys.stderr)
            return 2
        _, frames = flipbook.clips[row]
        for track in args.track:
            for i, draws in enumerate(frames):
                boxes = [draw_box(flipbook, d) for d in draws if d.track == track]
                if not boxes:
                    print(f"  {row}[{i}] {track}: not drawn")
                    continue
                x0 = min(b[0] for b in boxes); y0 = min(b[1] for b in boxes)
                x1 = max(b[2] for b in boxes); y1 = max(b[3] for b in boxes)
                print(f"  {row}[{i}] {track}: x {x0:6.1f}..{x1:6.1f}  y {y0:6.1f}..{y1:6.1f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
