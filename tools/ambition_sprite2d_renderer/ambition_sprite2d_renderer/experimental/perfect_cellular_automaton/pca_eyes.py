#!/usr/bin/env python3
"""Robust eye detection per pose crop -- eyes are critical and must be known.

The face is the cream blob (high in the sprite) that *contains* dark slits; the
eyes are those dark slits (cream on both sides).  Back views have no such blob
-> no eyes.  Returns the face bbox + eye boxes in crop coordinates, so eyes can
be authored as explicit, labelled 'eye' polygons in every frame.
"""
from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import pca_paths as P

CREAM = np.array([233, 231, 173])


def _comps(mask, min_area):
    n, lab, stats, cents = cv2.connectedComponentsWithStats(mask.astype(np.uint8), 8)
    out = []
    for i in range(1, n):
        if stats[i, cv2.CC_STAT_AREA] >= min_area:
            x, y, w, h, a = stats[i]
            out.append((x, y, x + w, y + h, int(a), cents[i]))
    return out


def detect(crop_rgba: np.ndarray):
    rgb = crop_rgba[:, :, :3].astype(np.int32)
    fg = crop_rgba[:, :, 3] >= 127
    h, w = fg.shape
    cream = (np.sqrt(((rgb - CREAM) ** 2).sum(2)) < 70) & fg
    dark = (rgb.sum(2) < 200) & fg
    best, best_eyes = None, []
    for fx0, fy0, fx1, fy1, area, cen in _comps(cream, 40):
        # the face is high in the sprite -- the head. A cream blob lower than this
        # is a hand/chest, not the face: in a foreshortened dive (pose_air) the
        # real face is occluded and a mid-body cream cluster would otherwise be
        # picked as a false face (and anchor the head-labelling wrongly). Every
        # genuine face here sits at <=0.43h; reject below 0.46h -> treat as
        # faceless (the build then anchors the head to the top of the figure).
        if cen[1] > 0.46 * h:
            continue
        sub_dark = dark[fy0:fy1, fx0:fx1]
        sub_cream = cream[fy0:fy1, fx0:fx1]
        cl = np.zeros_like(sub_cream); cr = np.zeros_like(sub_cream)
        for k in range(1, 6):
            cl[:, k:] |= sub_cream[:, :-k]; cr[:, :-k] |= sub_cream[:, k:]
        em = sub_dark & cl & cr                     # dark with cream on both sides
        eyes = [(fx0 + e[0], fy0 + e[1], fx0 + e[2], fy0 + e[3], e[4])
                for e in _comps(em, 2) if e[4] <= 220]
        if eyes and (best is None or len(eyes) + area * 1e-5 > best[0]):
            best = (len(eyes) + area * 1e-5, (fx0, fy0, fx1, fy1))
            best_eyes = eyes
    if best is None:
        return None, []
    # merge nearby fragments, normalise size, keep up to 2 by area
    boxes = [list(e[:4]) for e in best_eyes]
    merged = True
    while merged:
        merged = False
        out = []
        while boxes:
            a = boxes.pop()
            for b in out:
                if a[0] <= b[2] + 3 and b[0] <= a[2] + 3 and a[1] <= b[3] + 3 and b[1] <= a[3] + 3:
                    b[0], b[1] = min(a[0], b[0]), min(a[1], b[1])
                    b[2], b[3] = max(a[2], b[2]), max(a[3], b[3])
                    merged = True; break
            else:
                out.append(a)
        boxes = out
    boxes.sort(key=lambda b: -(b[2] - b[0]) * (b[3] - b[1]))
    boxes = boxes[:2]
    eyes = []
    for x0, y0, x1, y1 in boxes:
        cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
        ew, eh = max(x1 - x0, 4), max(y1 - y0, 7)
        eyes.append([int(cx - ew / 2), int(cy - eh / 2), int(cx + ew / 2), int(cy + eh / 2)])
    eyes.sort(key=lambda e: e[0])
    return best[1], eyes


def detect_pose(pose: str):
    crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    return detect(crop)


if __name__ == "__main__":
    out = P.DIAGNOSTICS / "eyes"
    out.mkdir(parents=True, exist_ok=True)
    cells = []
    for pose in P.POSES:
        crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
        face, eyes = detect(crop)
        bg = np.full((*crop.shape[:2], 3), 255, np.uint8)
        m = crop[:, :, 3] >= 127
        bg[m] = crop[:, :, :3][m]
        if face:
            cv2.rectangle(bg, (face[0], face[1]), (face[2], face[3]), (0, 180, 255), 1)
        for e in eyes:
            cv2.rectangle(bg, (e[0], e[1]), (e[2], e[3]), (255, 0, 255), 1)
        Image.fromarray(bg).save(out / f"{pose}.png")
        print(f"{pose:12s} eyes:{len(eyes)}  {eyes}")
