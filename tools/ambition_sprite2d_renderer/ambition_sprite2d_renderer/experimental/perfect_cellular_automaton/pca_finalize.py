#!/usr/bin/env python3
"""Stamp detected static detail (back/side cellular spots, eyes) onto a fitted
v15 sheet as locked polygons, then render the final sheet.

These details are extracted from the reference in absolute coordinates; because
the optimizer has already aligned each pose TO the reference, reference-absolute
detail lands correctly on the fitted body.  Locked => the (already-finished)
optimizer would leave them rigid on any later pass.
"""
from __future__ import annotations

import argparse
import json
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image

import pca_fit as F
import pca_detect_spots as D

HERE = Path(__file__).resolve().parent


def _near(rgb, color, tol):
    c = np.array(color, np.float32)
    return np.sqrt(((rgb.astype(np.float32) - c) ** 2).sum(axis=2)) < tol


def detect_eyes(ref: Image.Image, roi, *, min_area=8, max_area=400):
    """Eyes = dark blobs sitting inside / against the cream face mask."""
    x0, y0, x1, y1 = roi
    crop = np.asarray(ref.crop((x0, y0, x1, y1)).convert("RGB"))
    h, w = crop.shape[:2]
    dark = crop.astype(np.float32).sum(axis=2) < 95
    cream = _near(crop, (240, 235, 180), 70.0)
    near_cream = cream.copy()
    for _ in range(3):
        d = near_cream.copy()
        d[1:, :] |= near_cream[:-1, :]; d[:-1, :] |= near_cream[1:, :]
        d[:, 1:] |= near_cream[:, :-1]; d[:, :-1] |= near_cream[:, 1:]
        near_cream = d
    cand = dark & near_cream
    rects = []
    for cx0, cy0, cx1, cy1, area in D._components(cand, min_area):
        if area > max_area:
            continue
        bw, bh = cx1 - cx0, cy1 - cy0
        if bw > 0.35 * w or bh > 0.4 * h:
            continue
        rects.append([x0 + cx0, y0 + cy0, x0 + cx1, y0 + cy1])
    return rects


def add_locked_rect(geom, box, color):
    gx0, gy0, gx1, gy1 = box
    pts = np.asarray([[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]], np.float32)
    geom.polys.append(F.Poly(color, False, pts, locked=True))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fitted", type=Path, required=True, help="v15 json from pca_fit")
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--out-json", type=Path, required=True)
    ap.add_argument("--out-png", type=Path, required=True)
    args = ap.parse_args()

    data = json.loads(args.fitted.read_text())
    geoms = F.load_geom_v15(data)
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads((HERE / "pca_roi_specs_v14.json").read_text())["rois"]

    # Back / side carapace spots.
    for name in ("top_back", "top_side"):
        spots = D.detect(ref, specs[name]["roi"])[:6]
        for r in spots:
            add_locked_rect(geoms[name], r["global_box"], "dark_green")
        print(f"{name}: +{len(spots)} spots")

    # NOTE: eyes are already present from the v14 manual_eyes (carried into v15).
    # Auto-detecting "dark near cream" over-fires (helmet seams, segment gaps),
    # so we deliberately do NOT restamp eyes here -- placement is a manual nudge.

    F.save_geom_v15(geoms, data["palette"], data.get("meta", {}), args.out_json)
    F.render_sheet(geoms, data["palette"]).save(args.out_png)
    print(f"wrote {args.out_json}\nwrote {args.out_png}")


if __name__ == "__main__":
    main()
