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
