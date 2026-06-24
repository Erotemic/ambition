#!/usr/bin/env python3
"""Detect dark-green cellular spots on the body and emit them as motif rects.

ChatGPT's ``motif_segments`` covered the abdomen grid + forehead cells for the
action poses but NOT the back-carapace spots (top_back) or the shoulder/side
spots (top_side).  This finds connected ``dark_green`` blobs that sit *inside*
the green body (not the dark backdrop) and emits their bounding boxes in the
same ``{global_box, fill}`` schema, so they render through the existing motif
path as crisp cells.  Pure numpy connected components (BFS), no scipy.
"""
from __future__ import annotations

import argparse
import json
from collections import deque
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
PALETTE = {
    "dark_green": (54, 88, 49),
    "green": (137, 188, 49),
    "lime": (164, 218, 42),
}


def _near(rgb: np.ndarray, color, tol: float) -> np.ndarray:
    c = np.array(color, np.float32)
    return np.sqrt(((rgb.astype(np.float32) - c) ** 2).sum(axis=2)) < tol


def _components(mask: np.ndarray, min_area: int):
    h, w = mask.shape
    seen = np.zeros_like(mask)
    comps = []
    for sy in range(h):
        for sx in range(w):
            if mask[sy, sx] and not seen[sy, sx]:
                q = deque([(sy, sx)])
                seen[sy, sx] = True
                pix = []
                while q:
                    y, x = q.popleft()
                    pix.append((y, x))
                    for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                        ny, nx = y + dy, x + dx
                        if 0 <= ny < h and 0 <= nx < w and mask[ny, nx] and not seen[ny, nx]:
                            seen[ny, nx] = True
                            q.append((ny, nx))
                if len(pix) >= min_area:
                    ys = [p[0] for p in pix]
                    xs = [p[1] for p in pix]
                    comps.append((min(xs), min(ys), max(xs) + 1, max(ys) + 1, len(pix)))
    return comps


def detect(ref: Image.Image, roi, *, tol=34.0, min_area=40, max_frac=0.18,
           shrink=0.30):
    """Return motif rects for dark-green spots inside the green body in ``roi``."""
    x0, y0, x1, y1 = roi
    crop = np.asarray(ref.crop((x0, y0, x1, y1)).convert("RGB"))
    h, w = crop.shape[:2]
    dark = _near(crop, PALETTE["dark_green"], tol)
    green = _near(crop, PALETTE["green"], 60.0) | _near(crop, PALETTE["lime"], 60.0)
    # spot = dark-green region whose dilation touches green body (interior cell),
    # excluding huge regions (shadows / seams).
    body_near = green.copy()
    for _ in range(4):
        d = body_near.copy()
        d[1:, :] |= body_near[:-1, :]; d[:-1, :] |= body_near[1:, :]
        d[:, 1:] |= body_near[:, :-1]; d[:, :-1] |= body_near[:, 1:]
        body_near = d
    cand = dark & body_near
    rects = []
    area_cap = int(max_frac * w * h)
    for cx0, cy0, cx1, cy1, area in _components(cand, min_area):
        bw, bh = cx1 - cx0, cy1 - cy0
        if area > area_cap or bw > 0.6 * w or bh > 0.6 * h:
            continue
        # shrink toward centre so the cell sits cleanly inside the body
        sx = int(bw * shrink / 2); sy = int(bh * shrink / 2)
        gx0, gy0, gx1, gy1 = cx0 + sx, cy0 + sy, cx1 - sx, cy1 - sy
        if gx1 - gx0 < 2 or gy1 - gy0 < 2:
            gx0, gy0, gx1, gy1 = cx0, cy0, cx1, cy1
        rects.append({"global_box": [x0 + gx0, y0 + gy0, x0 + gx1, y0 + gy1],
                      "fill": "dark_green", "area": area})
    rects.sort(key=lambda r: -(r["global_box"][2] - r["global_box"][0])
               * (r["global_box"][3] - r["global_box"][1]))
    return rects


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--specs", type=Path, default=HERE / "pca_roi_specs_v14.json")
    ap.add_argument("--poses", nargs="*", default=["top_back", "top_side"])
    ap.add_argument("--top-n", type=int, default=6)
    args = ap.parse_args()
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads(args.specs.read_text())["rois"]
    out = {}
    for name in args.poses:
        rects = detect(ref, specs[name]["roi"])[: args.top_n]
        out[name] = rects
        print(f"{name}: {len(rects)} spots")
        for r in rects:
            print("  ", r["global_box"], "area", r["area"])
    print(json.dumps(out))
