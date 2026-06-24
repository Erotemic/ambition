#!/usr/bin/env python3
"""Dark substrate layer for the PCA fit.

The reference has a black bodysuit + helmet that shows through the gaps between
the green/purple armour plates (torso core, neck, helmet dome, limb seams,
between the legs).  The v14 polygons under-build this, so on a white background
the silhouette has holes.  We reconstruct it by taking the reference's *dark
foreground* (flood-fill foreground ∩ dark pixels) and covering it with axis-
aligned rectangles, stamped as locked black polygons drawn FIRST (behind the
plates).  Wherever a plate doesn't cover, the substrate reads as the dark body --
exactly like the reference.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import List, Tuple

import numpy as np
from PIL import Image

import pca_seg

HERE = Path(__file__).resolve().parent
Rect = Tuple[int, int, int, int]


def rect_cover(mask: np.ndarray) -> List[Rect]:
    """Cover a binary mask with axis-aligned rects by merging identical
    horizontal runs across consecutive rows. Exact (lossless) cover."""
    h, w = mask.shape
    rects: List[Rect] = []
    # per row: list of (x0, x1) runs
    prev_runs: List[Tuple[int, int]] = []
    open_rects = {}  # (x0,x1) -> y_start
    for y in range(h):
        row = mask[y]
        runs = []
        x = 0
        while x < w:
            if row[x]:
                x0 = x
                while x < w and row[x]:
                    x += 1
                runs.append((x0, x))
            else:
                x += 1
        runs_set = set(runs)
        # close rects whose run is absent this row
        for key in list(open_rects):
            if key not in runs_set:
                ys = open_rects.pop(key)
                rects.append((key[0], ys, key[1], y))
        # open new rects
        for r in runs:
            if r not in open_rects:
                open_rects[r] = y
    for key, ys in open_rects.items():
        rects.append((key[0], ys, key[1], h))
    return rects


PLATES = {"green": (137, 188, 49), "lime": (164, 218, 42), "purple": (114, 78, 130),
          "cream": (240, 235, 180), "dark_green": (54, 88, 49)}


def _dilate_n(mask: np.ndarray, n: int) -> np.ndarray:
    m = mask.copy()
    for _ in range(n):
        d = m.copy()
        d[1:, :] |= m[:-1, :]; d[:-1, :] |= m[1:, :]
        d[:, 1:] |= m[:, :-1]; d[:, :-1] |= m[:, 1:]
        m = d
    return m


def dark_foreground(ref: Image.Image, sum_lo: int = 88, sum_hi: int = 150,
                    neutral_tol: int = 22, plate_grow: int = 9):
    """The black bodysuit/helmet is a *neutral charcoal* (~34,34,34): only a hair
    brighter than the backdrop (~26,27,28) -- so the flood-fill foreground eats
    it. Detect it directly by colour: low-saturation (R≈G≈B) pixels brighter than
    the backdrop but darker than any plate. This isolates helmet dome, torso
    core and the dark seams without catching bg, green, purple or cream.

    Then gate to pixels within ``plate_grow`` of an actual armour plate, so
    stray neutral specks in the backdrop are dropped (the body charcoal always
    hugs a plate; backdrop noise does not)."""
    rgb = np.asarray(ref.convert("RGB")).astype(np.int32)
    s = rgb.sum(axis=2)
    mx = rgb.max(axis=2)
    mn = rgb.min(axis=2)
    neutral = (mx - mn) <= neutral_tol
    charcoal = neutral & (s >= sum_lo) & (s <= sum_hi)
    plate = np.zeros(rgb.shape[:2], dtype=bool)
    for c in PLATES.values():
        plate |= np.sqrt(((rgb - np.array(c)) ** 2).sum(axis=2)) < 55
    return charcoal & _dilate_n(plate, plate_grow)


def substrate_rects(ref: Image.Image, roi: Rect, dark_fg=None,
                    min_area: int = 6) -> List[Rect]:
    x0, y0, x1, y1 = roi
    if dark_fg is None:
        dark_fg = dark_foreground(ref)
    sub = dark_fg[y0:y1, x0:x1]
    out = []
    for rx0, ry0, rx1, ry1 in rect_cover(sub):
        if (rx1 - rx0) * (ry1 - ry0) >= min_area:
            out.append((x0 + rx0, y0 + ry0, x0 + rx1, y0 + ry1))
    return out


if __name__ == "__main__":
    import argparse
    ref_default = ("/home/joncrall/code/ambition/assets/concept_art/"
                   "prefect-cellular-automaton-reference-image.png")
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", type=Path, default=Path(ref_default))
    args = ap.parse_args()
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads((HERE / "pca_roi_specs_v14.json").read_text())["rois"]
    dfg = dark_foreground(ref)
    for nm, meta in specs.items():
        r = substrate_rects(ref, meta["roi"], dfg)
        print(f"{nm:12s} {len(r):4d} substrate rects")
