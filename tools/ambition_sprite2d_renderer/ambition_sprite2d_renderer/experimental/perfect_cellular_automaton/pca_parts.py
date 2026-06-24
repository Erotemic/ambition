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

import pca_paths as P
import pca_vectorize as V

# palette indices (from pca_vectorize k-means order)
CHARCOAL, GREEN, PURPLE, MIDGREEN, BLACK, CREAM, DARKGREEN = range(7)
DARK = {CHARCOAL, BLACK}
GREENS = {GREEN, MIDGREEN}

# part -> display colour for the visualization
PART_COLORS = {
    "horn": (120, 230, 60), "helmet": (40, 40, 48), "forehead_cell": (180, 240, 70),
    "face": (245, 240, 180), "eye": (10, 10, 10), "bodysuit": (55, 55, 64),
    "neck": (48, 48, 56), "core_fill": (60, 60, 70),
    "chest_plate": (70, 110, 50), "pec": (255, 210, 140), "belly_panel": (250, 235, 170),
    "belly_cell": (160, 220, 50), "core": (70, 70, 80),
    "shoulder": (60, 150, 40), "shoulder_spot": (40, 90, 40),
    "upper_arm": (90, 200, 70), "forearm": (150, 90, 170), "hand": (230, 220, 160),
    "thigh": (120, 70, 140), "shin": (70, 170, 60), "knee": (110, 210, 90),
    "foot": (220, 215, 150), "tail": (90, 180, 70), "other": (220, 40, 220),
}


def label_part(nx, ny, color, area_frac):
    is_dark = color in DARK
    is_green = color in GREENS
    is_cream = color == CREAM
    is_purple = color == PURPLE
    is_dg = color == DARKGREEN
    side = nx < 0.34 or nx > 0.66
    center = 0.36 < nx < 0.64
    if is_dark and area_frac > 0.12:
        return "bodysuit"
    # ---- head (top ~28%) ----  (eyes assigned separately by detection)
    if ny < 0.28:
        if ny < 0.14 and (is_green or is_dg) and side:
            return "horn"
        if is_cream:
            return "face"
        if is_dark:
            return "helmet"
        if is_green or is_dg:
            return "forehead_cell"
    # ---- chest band (~0.28-0.46): dark-green chest plate, cream pecs on it,
    #      green shoulder pauldrons with dark-green spots on the sides ----
    if ny < 0.46:
        if is_dg:
            # only dark-green clearly ON the far-side pauldrons is a spot; the
            # rest is the single dark-green upper-torso backing (chest_plate).
            return "shoulder_spot" if (nx < 0.28 or nx > 0.72) else "chest_plate"
        if is_green:
            return "shoulder" if side else "chest_plate"
        if is_cream:
            return "hand" if side and ny > 0.42 else "pec"
        if is_purple:
            return "forearm"
        if is_dark:
            return "core"
    # ---- belly band (~0.46-0.62): cream panel + automaton grid cells ----
    if ny < 0.62:
        if is_cream:
            return "belly_panel" if center else "hand"
        if (is_green or is_dg) and center:
            return "belly_cell"
        if is_green:
            return "upper_arm"
        if is_purple:
            return "thigh" if center else "forearm"
        if is_dark:
            return "core"
    # ---- legs / tail (lower) ----
    if is_purple:
        return "thigh"
    if is_cream:
        # cream at the SIDES above the very bottom is a HAND (the arms hang past
        # the waist); only cream near the bottom is a foot. Otherwise hands that
        # hang low get mislabelled as feet -> grouped into the legs.
        return "hand" if side and ny < 0.85 else "foot"
    if is_green or is_dg:
        return "tail" if (nx > 0.78 or nx < 0.22) and ny > 0.7 else "shin"
    if is_dark:
        return "core"
    return "other"


def label_pose(pose: str, vec_dir: Path):
    d = json.loads((vec_dir / f"{pose}_polys.json").read_text())
    w, h = d["w"], d["h"]
    palette = d["palette"]
    out = []
    for p in d["polys"]:
        if p.get("part"):                      # pre-tagged (e.g. eyes) -> keep
            out.append(dict(p))
            continue
        pts = np.array(p["points"])
        cx, cy = pts[:, 0].mean() / w, pts[:, 1].mean() / h
        part = label_part(cx, cy, p["color"], p["area"] / (w * h))
        out.append({**p, "part": part})
    return out, palette, w, h


def visualize(pose, vec_dir, out_dir):
    import cv2
    polys, palette, w, h = label_pose(pose, vec_dir)
    palette = np.array(palette)
    recon = V.render_polys(polys, palette, w, h, seal=0)
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
    ap.add_argument("--version", default="04_vectorized")
    args = ap.parse_args()
    vdir = P.VERSIONS / args.version
    args.vec = vdir
    args.out = vdir / "parts"
    args.out.mkdir(parents=True, exist_ok=True)
    todo = [args.pose] if args.pose else V.POSES
    for pose in todo:
        counts = visualize(pose, args.vec, args.out)
        print(f"{pose:12s}", dict(sorted(counts.items())))


if __name__ == "__main__":
    main()
