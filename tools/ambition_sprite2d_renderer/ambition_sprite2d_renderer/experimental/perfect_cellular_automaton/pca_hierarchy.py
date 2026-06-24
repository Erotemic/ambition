#!/usr/bin/env python3
"""Hierarchical part-debug views: prove the semantic labeling is correct.

Two views per pose:
  * HIGH-LEVEL: every polygon coloured by its top-level group -- head, torso,
    left_arm, right_arm, left_leg, right_leg -- so the gross decomposition and
    the L/R split are obvious at a glance.
  * SUB-PART: every polygon coloured by its specific sub-part, with a legend
    grouped under its high-level parent (left_arm -> shoulder/upper_arm/
    forearm/hand ...), showing the full hierarchy.

L/R is assigned by centroid x (screen-left = left_*).
"""
from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont

import pca_paths as P
import pca_parts as PARTS

# sub-part -> gross group (arm/leg get split L/R by position)
GROUP = {
    "helmet": "head", "horn": "head", "face": "head", "eye": "head",
    "forehead_cell": "head",
    "core": "torso", "bodysuit": "torso", "chest_plate": "torso", "pec": "torso",
    "belly_panel": "torso", "belly_cell": "torso", "other": "torso",
    "neck": "torso", "core_fill": "torso",
    "tail": "tail",
    "shoulder": "arm", "shoulder_spot": "arm", "upper_arm": "arm",
    "forearm": "arm", "hand": "arm",
    "thigh": "leg", "shin": "leg", "knee": "leg", "foot": "leg",
}
HL_COLORS = {
    "head": (245, 130, 70), "torso": (70, 140, 240),
    "left_arm": (120, 225, 130), "right_arm": (40, 120, 55),
    "left_leg": (240, 205, 90), "right_leg": (170, 110, 40),
    "tail": (210, 90, 200),
}
HIERARCHY = {
    "head": ["helmet", "horn", "face", "eye", "forehead_cell"],
    "torso": ["neck", "core", "bodysuit", "chest_plate", "pec", "belly_panel", "belly_cell"],
    "left_arm": ["shoulder", "shoulder_spot", "upper_arm", "forearm", "hand"],
    "right_arm": ["shoulder", "shoulder_spot", "upper_arm", "forearm", "hand"],
    "left_leg": ["thigh", "knee", "shin", "foot"],
    "right_leg": ["thigh", "knee", "shin", "foot"],
    "tail": ["tail"],
}


def _font(sz):
    try:
        return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", sz)
    except Exception:
        return ImageFont.load_default()


def high_level(part: str, nx: float, body_cx: float = 0.5) -> str:
    # split L/R about the BODY centreline (torso centroid), not the image centre,
    # so offset / crouched / diving poses assign the correct side.
    if part.endswith("_shade"):                  # shading overlay -> group with its base part
        part = part[:-6]
    g = GROUP.get(part, "torso")
    if g == "arm":
        return "left_arm" if nx < body_cx else "right_arm"
    if g == "leg":
        return "left_leg" if nx < body_cx else "right_leg"
    return g


def _distinct(n):
    import colorsys
    return [tuple(int(c * 255) for c in colorsys.hsv_to_rgb(i / max(1, n), 0.65, 0.95))
            for i in range(n)]


def render_views(pose: str, version: str):
    vd = P.VERSIONS / version
    polys, palette, w, h = PARTS.label_pose(pose, vd)
    palette = np.array(palette)
    # body centreline = centroid x of the torso parts (falls back to image centre)
    tx = [np.array(p["points"])[:, 0].mean() / w for p in polys
          if GROUP.get(p["part"]) == "torso"]
    body_cx = float(np.mean(tx)) if tx else 0.5
    # high-level view
    hi = np.full((h, w, 3), 255, np.uint8)
    for p in sorted(polys, key=lambda p: -p["area"]):
        pts = np.array(p["points"], np.int32)
        nx = pts[:, 0].mean() / w
        col = HL_COLORS[high_level(p["part"], nx, body_cx)]
        cv2.fillPoly(hi, [pts], col)
        cv2.polylines(hi, [pts], True, (20, 20, 20), 1, cv2.LINE_AA)
    # sub-part view: distinct colour per (hl, subpart)
    keys = []
    for hl, subs in HIERARCHY.items():
        for s in subs:
            keys.append((hl, s))
    palette_sp = dict(zip(keys, _distinct(len(keys))))
    sp = np.full((h, w, 3), 255, np.uint8)
    used = set()
    for p in sorted(polys, key=lambda p: -p["area"]):
        pts = np.array(p["points"], np.int32)
        nx = pts[:, 0].mean() / w
        hl = high_level(p["part"], nx, body_cx)
        base = p["part"][:-6] if p["part"].endswith("_shade") else p["part"]
        key = (hl, base)
        col = palette_sp.get(key, (200, 0, 200))
        used.add(key)
        cv2.fillPoly(sp, [pts], col)
        cv2.polylines(sp, [pts], True, (20, 20, 20), 1, cv2.LINE_AA)
    return Image.fromarray(hi), Image.fromarray(sp), used, palette_sp


def make(pose: str, version: str, out: Path):
    hi, sp, used, palette_sp = render_views(pose, version)
    legend_w = 230
    TITLE_H = 22
    H = max(hi.height, sp.height) + 4 + TITLE_H
    panel = Image.new("RGB", (hi.width + sp.width + legend_w + 16, H), (250, 250, 250))
    d = ImageDraw.Draw(panel)
    # pose-name title bar so each frame is identifiable at a glance
    d.rectangle((0, 0, panel.width, TITLE_H), fill=(28, 28, 34))
    d.text((6, 4), f"{pose}    [ high-level  |  sub-part ]", fill=(255, 255, 255), font=_font(14))
    panel.paste(hi, (0, TITLE_H))
    panel.paste(sp, (hi.width + 8, TITLE_H))
    x0 = hi.width + sp.width + 16
    d.text((x0, TITLE_H + 2), "HIGH-LEVEL:", fill=(0, 0, 0), font=_font(12))
    y = TITLE_H + 18
    for hl, c in HL_COLORS.items():
        d.rectangle((x0, y, x0 + 12, y + 12), fill=c); d.text((x0 + 16, y), hl, fill=(0, 0, 0), font=_font(11)); y += 15
    y += 8
    d.text((x0, y), "SUB-PARTS:", fill=(0, 0, 0), font=_font(12)); y += 16
    for hl, subs in HIERARCHY.items():
        d.text((x0, y), hl, fill=(0, 0, 0), font=_font(11)); y += 13
        for s in subs:
            key = (hl, s)
            if key in used:
                d.rectangle((x0 + 8, y, x0 + 18, y + 10), fill=palette_sp[key])
                d.text((x0 + 22, y - 1), s, fill=(40, 40, 40), font=_font(10)); y += 12
    panel.save(out)
    return used


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default="top_front")
    ap.add_argument("--version", default="09_paperdoll")
    args = ap.parse_args()
    out = P.VERSIONS / args.version / "hierarchy"
    out.mkdir(parents=True, exist_ok=True)
    used = make(args.pose, args.version, out / f"{args.pose}.png")
    print(f"{args.pose}: groups+subparts used:", sorted(f"{hl}/{s}" for hl, s in used))
