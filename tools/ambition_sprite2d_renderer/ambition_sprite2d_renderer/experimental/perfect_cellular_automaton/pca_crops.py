#!/usr/bin/env python3
"""Phase A: extract clean per-pose reference sprites from the manual segmentation.

Jon's ``pca-segment.png`` is the character on a transparent background (one big
image, 10 poses).  Their AABBs overlap (a side-view tail crosses a neighbour),
so we crop by the per-component *mask*, not the bbox.

  * alpha >= 127 is opaque (Jon's note); below is transparent.
  * drop connected components < MIN_AREA (noise Jon left in).
  * label the 10 components by layout: top row = front/side/back,
    bottom row = idle/walk_1/walk_2/attack/jump/air/land (left->right).
  * write each as an RGBA crop (transparent outside its own component) to
    ``agent-scratch/refs/<pose>.png``.
"""
from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import pca_paths as P

SEG = Path("/home/joncrall/code/ambition/assets/concept_art/pca-segment.png")
OUT = P.REFS
MIN_AREA = 100
TOP_ROW = ["top_front", "top_side", "top_back"]
BOTTOM_ROW = ["pose_idle", "pose_walk_1", "pose_walk_2", "pose_attack",
              "pose_jump", "pose_air", "pose_land"]


def load_clean_fg(seg_path: Path = SEG, min_area: int = MIN_AREA):
    """Return (rgb, fg_labels, stats, centroids) for the cleaned segmentation."""
    rgba = np.asarray(Image.open(seg_path).convert("RGBA"))
    fg = (rgba[:, :, 3] >= 127).astype(np.uint8)
    n, labels, stats, cents = cv2.connectedComponentsWithStats(fg, connectivity=8)
    keep = np.zeros(n, dtype=bool)
    for i in range(1, n):
        if stats[i, cv2.CC_STAT_AREA] >= min_area:
            keep[i] = True
    # relabel kept components 1..K by descending area
    kept = sorted([i for i in range(1, n) if keep[i]],
                  key=lambda i: -stats[i, cv2.CC_STAT_AREA])
    return rgba, labels, stats, cents, kept


def assign_poses(stats, cents, kept):
    """Map the kept components to pose names via the 2-row layout."""
    ys = np.array([cents[i][1] for i in kept])
    # split into two rows at the largest vertical gap between sorted centroids
    order = np.argsort(ys)
    sorted_y = ys[order]
    gap_idx = int(np.argmax(np.diff(sorted_y)))
    split = (sorted_y[gap_idx] + sorted_y[gap_idx + 1]) / 2
    top = [kept[i] for i in range(len(kept)) if cents[kept[i]][1] < split]
    bot = [kept[i] for i in range(len(kept)) if cents[kept[i]][1] >= split]
    top.sort(key=lambda i: cents[i][0])
    bot.sort(key=lambda i: cents[i][0])
    mapping = {}
    for name, comp in zip(TOP_ROW, top):
        mapping[name] = comp
    for name, comp in zip(BOTTOM_ROW, bot):
        mapping[name] = comp
    return mapping, top, bot


def extract(out_dir: Path = None):
    out_dir = out_dir or OUT
    out_dir.mkdir(parents=True, exist_ok=True)
    rgba, labels, stats, cents, kept = load_clean_fg()
    mapping, top, bot = assign_poses(stats, cents, kept)
    if len(top) != 3 or len(bot) != 7:
        print(f"WARNING: expected 3 top / 7 bottom, got {len(top)}/{len(bot)}")
    meta = {}
    for name, comp in mapping.items():
        x = stats[comp, cv2.CC_STAT_LEFT]
        y = stats[comp, cv2.CC_STAT_TOP]
        w = stats[comp, cv2.CC_STAT_WIDTH]
        h = stats[comp, cv2.CC_STAT_HEIGHT]
        comp_mask = (labels == comp)
        crop = rgba[y:y + h, x:x + w].copy()
        m = comp_mask[y:y + h, x:x + w]
        crop[~m] = (0, 0, 0, 0)  # mask out overlapping neighbours
        Image.fromarray(crop, "RGBA").save(out_dir / f"{name}.png")
        meta[name] = {"abs_box": [int(x), int(y), int(x + w), int(y + h)],
                      "area": int(stats[comp, cv2.CC_STAT_AREA])}
    (out_dir / "crops_meta.json").write_text(json.dumps(meta, indent=2))
    return meta


if __name__ == "__main__":
    meta = extract()
    for k, v in meta.items():
        print(f"{k:12s} box={v['abs_box']} area={v['area']}")
