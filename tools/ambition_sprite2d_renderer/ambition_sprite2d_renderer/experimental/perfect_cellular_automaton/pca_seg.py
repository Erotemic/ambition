#!/usr/bin/env python3
"""Foreground segmentation of the PCA reference image.

The naive "distance from border-median background" mask misclassifies the dark
helmet / forehead as background (it is the same near-black as the backdrop).
Here the background is instead the *border-connected* region of background-like
pixels (a flood fill from the image edges); anything the flood cannot reach is
foreground, so interior dark regions enclosed by the body (helmet, eye sockets,
tail seams) are correctly kept as foreground.

Pure numpy (no scipy): the flood fill is an iterative binary dilation of the
border seed, intersected each step with the "bg-like" admissible set.
"""
from __future__ import annotations

import numpy as np
from PIL import Image


def _dilate(mask: np.ndarray) -> np.ndarray:
    out = mask.copy()
    out[1:, :] |= mask[:-1, :]
    out[:-1, :] |= mask[1:, :]
    out[:, 1:] |= mask[:, :-1]
    out[:, :-1] |= mask[:, 1:]
    return out


def background_mask(rgb: np.ndarray, bg: np.ndarray, tol: float = 30.0,
                    max_iter: int = 4000) -> np.ndarray:
    """Border-connected background via flood fill over bg-like pixels."""
    dist = np.sqrt(((rgb.astype(np.float32) - bg) ** 2).sum(axis=2))
    admissible = dist < tol
    seed = np.zeros(rgb.shape[:2], dtype=bool)
    seed[0, :] = seed[-1, :] = seed[:, 0] = seed[:, -1] = True
    seed &= admissible
    cur = seed
    for _ in range(max_iter):
        nxt = _dilate(cur) & admissible
        if nxt.sum() == cur.sum():
            break
        cur = nxt
    return cur


def foreground_mask(im: Image.Image, tol: float = 30.0) -> np.ndarray:
    rgb = np.asarray(im.convert("RGB"))
    h, w, _ = rgb.shape
    strips = np.concatenate([
        rgb[:6].reshape(-1, 3), rgb[-6:].reshape(-1, 3),
        rgb[:, :6].reshape(-1, 3), rgb[:, -6:].reshape(-1, 3)])
    bg = np.median(strips, axis=0).astype(np.float32)
    return ~background_mask(rgb, bg, tol)


_PLATES = [(137, 188, 49), (164, 218, 42), (114, 78, 130), (240, 235, 180), (54, 88, 49)]


def replace_background_white(im: Image.Image, tol: float = 12.0,
                             grow: int = 3) -> Image.Image:
    """Reference with the backdrop replaced by white, keeping the dark charcoal
    helmet/body.

    The charcoal body (~34,34,34) is only ~14 from the backdrop (~26,27,28), so:
      1. flood-fill the border-connected background at a TIGHT tol (charcoal is
         not admissible, so it blocks the fill and survives) -- solid, no holes,
         but leaves a 1-2px anti-aliased halo fringe;
      2. grow the white region by ``grow`` px into that fringe, but PROTECT
         actual armour plates and the *thick* charcoal core (morphological
         opening), so the halo is removed without eating the helmet or edges.
    """
    rgb = np.asarray(im.convert("RGB")).astype(np.int32)
    bg = estimate_bg(im)
    flooded = background_mask(np.asarray(im.convert("RGB")), bg, tol)

    plate = np.zeros(rgb.shape[:2], dtype=bool)
    for c in _PLATES:
        plate |= np.sqrt(((rgb - np.array(c)) ** 2).sum(axis=2)) < 62
    s = rgb.sum(axis=2)
    neutral = (rgb.max(2) - rgb.min(2)) <= 22
    charcoal = neutral & (s >= 88) & (s <= 150)
    # thick charcoal only (opening) -> protect helmet/body, drop thin fringe
    core = charcoal
    for _ in range(2):
        e = core.copy()
        e[1:] &= core[:-1]; e[:-1] &= core[1:]
        e[:, 1:] &= core[:, :-1]; e[:, :-1] &= core[:, 1:]
        core = e
    for _ in range(3):
        d = core.copy()
        d[1:] |= core[:-1]; d[:-1] |= core[1:]
        d[:, 1:] |= core[:, :-1]; d[:, :-1] |= core[:, 1:]
        core = d
    protected = plate | core

    grown = flooded.copy()
    for _ in range(grow):
        d = grown.copy()
        d[1:] |= grown[:-1]; d[:-1] |= grown[1:]
        d[:, 1:] |= grown[:, :-1]; d[:, :-1] |= grown[:, 1:]
        grown = d & ~protected
    white = flooded | grown
    out = np.asarray(im.convert("RGB")).copy()
    out[white] = (255, 255, 255)
    return Image.fromarray(out)


def estimate_bg(im: Image.Image) -> np.ndarray:
    rgb = np.asarray(im.convert("RGB"))
    strips = np.concatenate([
        rgb[:6].reshape(-1, 3), rgb[-6:].reshape(-1, 3),
        rgb[:, :6].reshape(-1, 3), rgb[:, -6:].reshape(-1, 3)])
    return np.median(strips, axis=0).astype(np.float32)


if __name__ == "__main__":
    import argparse
    from pathlib import Path
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--tol", type=float, default=30.0)
    args = ap.parse_args()
    im = Image.open(args.ref).convert("RGB")
    fg = foreground_mask(im, args.tol)
    base = np.asarray(im).copy()
    # tint foreground magenta-ish overlay for inspection
    over = base.copy()
    over[fg] = (0.45 * over[fg] + np.array([0.55 * 255, 0, 0.55 * 255])).astype(np.uint8)
    Image.fromarray(over).save(args.out)
    print(f"foreground fraction {fg.mean():.4f}; wrote {args.out}")
