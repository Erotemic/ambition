#!/usr/bin/env python3
"""How much of the picture changes between two captures of a still scene?

A look that flickers shows as pixels that change between two frames in which
nothing in the game moved. This script counts them, and says where they are.

    capture_scene central_hub_complex player /tmp/t.png 1280x720 --warmup 100 \\
        --flag look.central_hub_complex.pure --frames 3 --stride 1
    python scripts/measure_capture_frame_delta.py /tmp/t.0001.png /tmp/t.0002.png

It prints the count of pixels whose largest channel difference is more than
each threshold, and the cells of the picture (80 px) with the most change.
`--diff OUT.png` writes the difference, four times brighter.

What it does not measure:

- It does not tell a flicker from a motion. A frame in which the player's
  animation steps has a cluster of change at the player; read the cells. Two
  captures with the camera a part of a pixel apart differ at each hard edge,
  and that is the picture moving.
- A capture is not a window on a GPU (`docs/systems/room-looks.md`).

It needs Pillow and numpy: the sprite renderer's tool venv has them
(`~/.cache/ambition-tool-venvs/ambition_sprite2d_renderer/bin/python`).

Measured with it, the pure hub, camera still, 2026-10-09 (pixels over 24):
3008 and 351 between ticks before the architecture was drawn from plates and
the ivy stopped; 1724 (the player's animation step: each cell with more than
100 px is at the player) and 57 after.
"""

from __future__ import annotations

import argparse
import sys
from collections import Counter

THRESHOLDS = (0, 24, 64)
CELL = 80


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("first")
    parser.add_argument("second")
    parser.add_argument("--diff", help="write the difference image here")
    parser.add_argument("--cells", type=int, default=6, help="how many cells to list")
    args = parser.parse_args(argv)

    try:
        import numpy as np
        from PIL import Image
    except ImportError as error:
        print(f"this script needs Pillow and numpy: {error}", file=sys.stderr)
        return 2

    first = np.asarray(Image.open(args.first).convert("RGB")).astype(int)
    second = np.asarray(Image.open(args.second).convert("RGB")).astype(int)
    if first.shape != second.shape:
        print(f"the captures have different sizes: {first.shape} and {second.shape}", file=sys.stderr)
        return 2
    delta = np.abs(first - second).max(axis=2)
    total = delta.size
    for threshold in THRESHOLDS:
        count = int((delta > threshold).sum())
        print(f"changed by more than {threshold:3d}: {count:8d} px ({100.0 * count / total:.2f}%)")
    ys, xs = np.nonzero(delta > THRESHOLDS[1])
    cells = Counter(zip((xs // CELL).tolist(), (ys // CELL).tolist()))
    for (cx, cy), count in cells.most_common(args.cells):
        print(f"  cell x {cx * CELL}..{cx * CELL + CELL}, y {cy * CELL}..{cy * CELL + CELL}: {count} px")
    if args.diff:
        Image.fromarray(np.clip(delta * 4, 0, 255).astype("uint8")).save(args.diff)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
