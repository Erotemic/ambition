#!/usr/bin/env python3
"""Clean a character silhouette / segmentation for stable optimization.

Jon's manual segmentation (``assets/concept_art/pca-segment.png``, RGBA) is a
much better foreground than any auto threshold, but its alpha still carries
hundreds of tiny disconnected specks.  Optimizing against a noisy silhouette is
unstable (the loss chases isolated pixels), so we:

  * keep only connected components above ``min_area`` (the real figures), and
  * fill background pockets fully enclosed by the figure (not border-connected),

yielding a solid, noise-free mask.  Returns the cleaned alpha and a cleaned
RGBA (noise pixels made transparent).
"""
from __future__ import annotations

from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image

import pca_detect_spots as D


def _border_connected(mask: np.ndarray) -> np.ndarray:
    """Flood fill ``mask`` (True = candidate) inward from the border."""
    h, w = mask.shape
    seen = np.zeros_like(mask)
    q = deque()
    for x in range(w):
        for y in (0, h - 1):
            if mask[y, x] and not seen[y, x]:
                seen[y, x] = True; q.append((y, x))
    for y in range(h):
        for x in (0, w - 1):
            if mask[y, x] and not seen[y, x]:
                seen[y, x] = True; q.append((y, x))
    while q:
        y, x = q.popleft()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            ny, nx = y + dy, x + dx
            if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                seen[ny, nx] = True; q.append((ny, nx))
    return seen


def clean_mask(mask: np.ndarray, min_area: int = 500,
               fill_holes_below: int = 4000) -> np.ndarray:
    """Keep components >= ``min_area``; fill enclosed bg holes < ``fill_holes_below``."""
    h, w = mask.shape
    out = np.zeros_like(mask)
    # label foreground components by BFS, keep large ones
    seen = np.zeros_like(mask)
    for sy in range(h):
        row = mask[sy]
        for sx in range(w):
            if row[sx] and not seen[sy, sx]:
                q = deque([(sy, sx)]); seen[sy, sx] = True; pix = []
                while q:
                    y, x = q.popleft(); pix.append((y, x))
                    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        ny, nx = y + dy, x + dx
                        if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                            seen[ny, nx] = True; q.append((ny, nx))
                if len(pix) >= min_area:
                    for y, x in pix:
                        out[y, x] = True
    # fill holes: background not reachable from the border, if small
    bg = ~out
    border_bg = _border_connected(bg)
    enclosed = bg & ~border_bg
    # only fill small enclosed pockets (avoid filling legit concavities, though
    # by definition enclosed ones are holes; the size cap is a safety net)
    seen2 = np.zeros_like(enclosed)
    for sy in range(h):
        for sx in range(w):
            if enclosed[sy, sx] and not seen2[sy, sx]:
                q = deque([(sy, sx)]); seen2[sy, sx] = True; pix = []
                while q:
                    y, x = q.popleft(); pix.append((y, x))
                    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        ny, nx = y + dy, x + dx
                        if 0 <= ny < h and 0 <= nx < w and enclosed[ny, nx] and not seen2[ny, nx]:
                            seen2[ny, nx] = True; q.append((ny, nx))
                if len(pix) <= fill_holes_below:
                    for y, x in pix:
                        out[y, x] = True
    return _keep_large(out, min_area)


def _keep_large(mask: np.ndarray, min_area: int) -> np.ndarray:
    """Final pass: keep 4-connected components >= min_area. 4-connectivity is
    the strict reading -- any blob under min_area by either connectivity is
    dropped, so no <min_area component survives in the binary image."""
    h, w = mask.shape
    out = np.zeros_like(mask)
    seen = np.zeros_like(mask)
    nbrs = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    for sy in range(h):
        for sx in range(w):
            if mask[sy, sx] and not seen[sy, sx]:
                q = deque([(sy, sx)]); seen[sy, sx] = True; pix = []
                while q:
                    y, x = q.popleft(); pix.append((y, x))
                    for dy, dx in nbrs:
                        ny, nx = y + dy, x + dx
                        if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                            seen[ny, nx] = True; q.append((ny, nx))
                if len(pix) >= min_area:
                    for y, x in pix:
                        out[y, x] = True
    return out


def clean_segmentation(seg_path: Path, min_area: int = 500):
    seg = Image.open(seg_path).convert("RGBA")
    arr = np.asarray(seg).copy()
    alpha = arr[:, :, 3] > 8
    cleaned = clean_mask(alpha, min_area)
    arr[~cleaned] = (0, 0, 0, 0)
    return cleaned, Image.fromarray(arr, "RGBA")


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--seg", type=Path,
                    default=Path("/home/joncrall/code/ambition/assets/concept_art/pca-segment.png"))
    ap.add_argument("--out-rgba", type=Path, required=True)
    ap.add_argument("--out-white", type=Path, default=None)
    ap.add_argument("--min-area", type=int, default=500)
    args = ap.parse_args()
    mask, rgba = clean_segmentation(args.seg, args.min_area)
    rgba.save(args.out_rgba)
    removed = (np.asarray(Image.open(args.seg).convert("RGBA"))[:, :, 3] > 8).sum() - mask.sum()
    print(f"kept {int(mask.sum())} px; removed {int(removed)} noise px; wrote {args.out_rgba}")
    if args.out_white:
        white = Image.new("RGBA", rgba.size, (255, 255, 255, 255))
        Image.alpha_composite(white, rgba).convert("RGB").save(args.out_white)
        print("wrote", args.out_white)
