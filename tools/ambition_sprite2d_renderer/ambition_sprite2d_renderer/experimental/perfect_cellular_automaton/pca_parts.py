#!/usr/bin/env python3
"""Phase C: label each extracted polygon by the body part it serves.

A polygon's part is inferred from its palette colour + its normalized position
within the sprite bbox.  Labels give cross-viewpoint identity (the cream blob
high-centre is the 'face' in every pose), which is what lets us recombine parts
into novel poses later.

This is a heuristic first pass: reliable for the upright views (front/back),
approximate for profile / action poses where limbs move -- the visualization
(distinct colour per part + legend) makes mislabels obvious to correct.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import pca_vectorize as V

# palette indices (from pca_vectorize k-means order)
CHARCOAL, GREEN, PURPLE, MIDGREEN, BLACK, CREAM, DARKGREEN = range(7)
DARK = {CHARCOAL, BLACK}
GREENS = {GREEN, MIDGREEN}

# part -> display colour for the visualization
PART_COLORS = {
    "horn": (120, 230, 60), "helmet": (40, 40, 48), "forehead_cell": (180, 240, 70),
    "face": (245, 240, 180), "eye": (10, 10, 10),
    "chest": (255, 210, 140), "belly_cell": (160, 220, 50), "core": (70, 70, 80),
    "shoulder": (60, 150, 40), "upper_arm": (90, 200, 70), "forearm": (150, 90, 170),
    "hand": (230, 220, 160),
    "thigh": (120, 70, 140), "shin": (70, 170, 60), "knee": (110, 210, 90),
    "foot": (220, 215, 150), "tail": (90, 180, 70), "other": (220, 40, 220),
}


def label_part(nx, ny, color, area_frac):
    is_dark = color in DARK
    is_green = color in GREENS
    is_cream = color == CREAM
    is_purple = color == PURPLE
    is_dg = color == DARKGREEN
    cell = area_frac < 0.012
    tiny = area_frac < 0.0025
    # ---- head (top ~30%) ----
    if ny < 0.30:
        if ny < 0.17 and (is_green or is_dg) and not cell:
            return "horn"
        if is_cream:
            return "face"
        if is_dark and tiny and ny > 0.17:
            return "eye"            # tiny dark slit low in the head = eye
        if is_dark:
            return "helmet"
        if (is_green or is_dg) and cell:
            return "forehead_cell"
    # ---- torso + arms (~0.28-0.62) ----
    if ny < 0.60:
        side = nx < 0.32 or nx > 0.68
        if is_cream:
            return "hand" if side and ny > 0.45 else "chest"
        if is_purple:
            return "forearm" if side else "thigh"
        if is_green:
            if side:
                return "shoulder" if ny < 0.42 else "upper_arm"
            return "belly_cell" if cell else "shoulder"
        if is_dg or (is_green and cell):
            return "belly_cell"
        if is_dark:
            return "core"
    # ---- legs / tail (lower) ----
    if is_purple:
        return "thigh"
    if is_cream:
        return "foot"
    if is_green or is_dg:
        # a far-side elongated green run low down reads as the segmented tail
        return "tail" if (nx > 0.78 or nx < 0.22) and ny > 0.7 else "shin"
    if is_dark:
        return "core"
    return "other"


def label_pose(pose: str, vec_dir: Path):
    d = json.loads((vec_dir / f"{pose}_polys.json").read_text())
    w, h = d["w"], d["h"]
    out = []
    for p in d["polys"]:
        pts = np.array(p["points"])
        cx, cy = pts[:, 0].mean() / w, pts[:, 1].mean() / h
        part = label_part(cx, cy, p["color"], p["area"] / (w * h))
        out.append({**p, "part": part})
    return out, d["palette"], w, h


def visualize(pose, vec_dir, out_dir):
    import cv2
    polys, palette, w, h = label_pose(pose, vec_dir)
    palette = np.array(palette)
    recon = V.render_polys(polys, palette, w, h)
    lab = np.full((h, w, 3), 255, np.uint8)
    for p in sorted(polys, key=lambda p: -p["area"]):
        col = PART_COLORS.get(p["part"], (200, 0, 200))
        cv2.fillPoly(lab, [np.array(p["points"], np.int32)], col)
        cv2.polylines(lab, [np.array(p["points"], np.int32)], True, col, 2)
    panel = Image.fromarray(np.concatenate([recon, lab], axis=1))
    d = ImageDraw.Draw(panel)
    try:
        f = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", 13)
    except Exception:
        f = ImageFont.load_default()
    used = sorted(set(p["part"] for p in polys))
    for i, part in enumerate(used):
        c = PART_COLORS.get(part, (200, 0, 200))
        d.rectangle((w + 4, 4 + i * 16, w + 18, 16 + i * 16), fill=c)
        d.text((w + 22, 3 + i * 16), part, fill=(0, 0, 0), font=f)
    panel.save(out_dir / f"{pose}_parts.png")
    counts = {}
    for p in polys:
        counts[p["part"]] = counts.get(p["part"], 0) + 1
    return counts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default=None)
    ap.add_argument("--vec", type=Path, default=Path("agent-scratch/vec"))
    ap.add_argument("--out", type=Path, default=Path("agent-scratch/parts"))
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    todo = [args.pose] if args.pose else V.POSES
    for pose in todo:
        counts = visualize(pose, args.vec, args.out)
        print(f"{pose:12s}", dict(sorted(counts.items())))


if __name__ == "__main__":
    main()
