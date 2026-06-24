#!/usr/bin/env python3
"""Detect the cream face mask + eye slits from the reference, per pose.

The face is defined as *the cream blob that has interior dark eye-slits* (hands,
feet and tail-tips are cream too but eyeless), which makes detection robust even
for crouched / 3-4 poses where "topmost cream" picks the wrong blob.

Returns, in absolute sheet coords:
  * a convex-hull polygon of the face (so the iconic head reads faithfully even
    where the optimizer/v14 left the cream face shrunken), and
  * merged eye boxes.

These are stamped as a LOCKED detail layer onto the fitted sheet, aligned by the
fact that each pose is already optimized TO the reference.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import List, Optional, Tuple

import numpy as np
from PIL import Image

import pca_detect_spots as D

HERE = Path(__file__).resolve().parent
CREAM = (240, 235, 180)


def _near(rgb, c, tol):
    return np.sqrt(((rgb.astype(np.float32) - np.array(c, np.float32)) ** 2).sum(2)) < tol


def _convex_hull(pts: np.ndarray) -> np.ndarray:
    """Andrew's monotone chain. pts: (N,2) int. Returns hull (M,2) CCW."""
    pts = np.unique(pts, axis=0)
    if len(pts) <= 3:
        return pts
    pts = pts[np.lexsort((pts[:, 1], pts[:, 0]))]

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower = []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    upper = []
    for p in pts[::-1]:
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return np.array(lower[:-1] + upper[:-1])


def _eye_boxes(face_crop: np.ndarray):
    bright = face_crop.astype(np.float32).sum(2)
    dark = bright < 185
    cream = _near(face_crop, CREAM, 74)
    cl = np.zeros_like(cream)
    cr = np.zeros_like(cream)
    for k in range(1, 6):
        cl[:, k:] |= cream[:, :-k]
        cr[:, :-k] |= cream[:, k:]
    em = dark & cl & cr
    boxes = [[c[0], c[1], c[2], c[3]] for c in D._components(em, 2) if c[4] <= 200]
    return boxes


def _merge(boxes, gap=4):
    """Merge boxes whose gap is < ``gap`` in both axes (de-fragment one eye)."""
    boxes = [list(b) for b in boxes]
    changed = True
    while changed:
        changed = False
        out = []
        while boxes:
            a = boxes.pop()
            merged = False
            for b in out:
                if (a[0] <= b[2] + gap and b[0] <= a[2] + gap and
                        a[1] <= b[3] + gap and b[1] <= a[3] + gap):
                    b[0] = min(a[0], b[0]); b[1] = min(a[1], b[1])
                    b[2] = max(a[2], b[2]); b[3] = max(a[3], b[3])
                    merged = True
                    changed = True
                    break
            if not merged:
                out.append(a)
        boxes = out
    return boxes


def detect(ref: Image.Image, roi, *, max_eyes=2):
    x0, y0, x1, y1 = roi
    crop = np.asarray(ref.crop((x0, y0, x1, y1)).convert("RGB"))
    h, w = crop.shape[:2]
    cream = _near(crop, CREAM, 72)
    best = None
    best_eyes = []
    for fx0, fy0, fx1, fy1, area in D._components(cream, 25):
        if (fy0 + fy1) / 2 > 0.62 * h:
            continue
        eyes = _eye_boxes(crop[fy0:fy1, fx0:fx1])
        if not eyes:
            continue
        score = len(eyes) + 0.0005 * area
        if best is None or score > best[0]:
            best = (score, fx0, fy0, fx1, fy1)
            best_eyes = [[fx0 + e[0], fy0 + e[1], fx0 + e[2], fy0 + e[3]] for e in eyes]
    if best is None:
        return None, []
    _, fx0, fy0, fx1, fy1 = best
    # face hull from the cream pixels inside the face component bbox
    sub = cream[fy0:fy1, fx0:fx1]
    ys, xs = np.where(sub)
    hull_local = _convex_hull(np.stack([xs + fx0, ys + fy0], axis=1))
    hull = [[int(x0 + px), int(y0 + py)] for px, py in hull_local]
    # merge + normalise eyes
    eyes = _merge(best_eyes)
    eyes.sort(key=lambda e: -((e[2] - e[0]) * (e[3] - e[1])))
    eyes = eyes[:max_eyes]
    out_eyes = []
    for ex0, ey0, ex1, ey1 in eyes:
        cx, cy = (ex0 + ex1) / 2, (ey0 + ey1) / 2
        ew = max(ex1 - ex0, 4)
        eh = max(ey1 - ey0, 8)
        out_eyes.append([int(x0 + cx - ew / 2), int(y0 + cy - eh / 2),
                         int(x0 + cx + ew / 2), int(y0 + cy + eh / 2)])
    out_eyes.sort(key=lambda e: e[0])
    return hull, out_eyes


# poses that face the viewer enough to show two eyes; side shows one
POSE_MAX_EYES = {
    "top_front": 2, "top_side": 1, "pose_idle": 2, "pose_walk_1": 2,
    "pose_walk_2": 2, "pose_attack": 1, "pose_jump": 2, "pose_air": 2,
    "pose_land": 2,
}


if __name__ == "__main__":
    import argparse
    from PIL import ImageDraw
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", type=Path, required=True)
    args = ap.parse_args()
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads((HERE / "pca_roi_specs_v14.json").read_text())["rois"]
    for nm, n in POSE_MAX_EYES.items():
        hull, eyes = detect(ref, specs[nm]["roi"], max_eyes=n)
        print(nm, "hull_pts", len(hull) if hull else 0, "eyes", eyes)
