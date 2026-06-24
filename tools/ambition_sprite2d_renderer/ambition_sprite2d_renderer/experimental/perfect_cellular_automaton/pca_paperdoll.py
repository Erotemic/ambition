#!/usr/bin/env python3
"""Paper-doll assembly: one clean low-edge polygon per semantic part.

Jon's construction rules:
  * paper-doll character -> each PART is its own polygon, assembled by z-order
    layering; NO single non-convex silhouette substrate.
  * most polygons are convex and low-edge (horns are triangles, <10 edges);
    a few read as concave but can be convex + layered to look otherwise.
  * the automaton cells (belly grid, forehead pattern) are exact SQUARES.

Pipeline: quantize the crop -> per-colour regions -> semantic-label each ->
group by part -> emit one clean polygon per part (square for cells, convex hull
+ Douglas-Peucker for the rest), z-ordered dark-first.  Dark is broken into its
large components (helmet / torso core / pelvis), never one silhouette.
"""
from __future__ import annotations

import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import pca_paths as P
import pca_parts as PARTS
import pca_eyes

CELL_PARTS = {"belly_cell", "forehead_cell"}            # exact squares
CONVEX_SPOT = {"shoulder_spot"}                          # irregular convex
SINGLE_PLATE = {"chest_plate", "belly_panel"}            # one clean backing poly
# z-order: lower drawn first (behind)
Z = {"bodysuit": 0, "core": 0, "helmet": 1, "horn": 1, "tail": 1,
     "chest_plate": 2, "belly_panel": 2, "upper_arm": 2, "thigh": 2,
     "pec": 3, "belly_cell": 3, "forearm": 3, "shin": 3,
     "shoulder": 4, "knee": 4, "foot": 4, "hand": 4,
     "shoulder_spot": 5, "face": 5,
     "forehead_cell": 6, "eye": 7, "other": 3}


def _square(pts: np.ndarray) -> np.ndarray:
    (cx, cy), (w, h), ang = cv2.minAreaRect(pts.astype(np.float32))
    s = (w + h) / 2.0
    return cv2.boxPoints(((cx, cy), (s, s), ang)).astype(int)


def _clean(pts: np.ndarray, convex=True, max_edges=12, min_edges=5) -> np.ndarray:
    """Simplify to a low-but-honest edge count. The reference is noisy but not
    THAT noisy -- most parts want 5-12 sides, not 4. Start gentle and only
    coarsen until under max_edges."""
    hull = cv2.convexHull(pts.astype(np.int32)) if convex else pts.astype(np.int32)
    eps = 0.006 * cv2.arcLength(hull, True)
    approx = cv2.approxPolyDP(hull, eps, True).reshape(-1, 2)
    for _ in range(8):
        if len(approx) <= max_edges:
            break
        eps *= 1.35
        approx = cv2.approxPolyDP(hull, eps, True).reshape(-1, 2)
    return approx


def build(pose: str, palette: np.ndarray, eps_quant=None):
    crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    rgb = crop[:, :, :3]
    fg = crop[:, :, 3] >= 127
    h, w = fg.shape
    from pca_vectorize import quantize
    qi = quantize(rgb, fg, palette)
    dark_idx = {int(np.argmin(palette.sum(1)))}
    dark_idx |= {i for i, c in enumerate(palette) if c.sum() < 130}

    # collect labelled regions (connected components per colour)
    regions = []  # (part, color, mask)
    face_box, _ = pca_eyes.detect(crop)
    for ci in range(len(palette)):
        mask = (qi == ci).astype(np.uint8)
        if mask.sum() < 10:
            continue
        n, lab, stats, cents = cv2.connectedComponentsWithStats(mask, 8)
        is_dark = ci in dark_idx
        for li in range(1, n):
            area = stats[li, cv2.CC_STAT_AREA]
            # dark: keep only the large structural parts (helmet/core/pelvis);
            # the thin line-art slivers are dropped -- in a paper doll the dark
            # reads through the gaps BETWEEN the layered colour plates.
            if is_dark and area < 200:
                continue
            if area < 12:
                continue
            cx, cy = cents[li]
            part = PARTS.label_part(cx / w, cy / h, ci, area / (w * h))
            regions.append((part, ci, (lab == li), area))

    # group same-part fragments into instances: OR the part's masks, bridge
    # small gaps (dilate), and split into spatially-separate instances. Cells
    # stay separate (the grid squares don't touch); limb/plate shading merges
    # into one polygon per instance.
    by_part = {}
    for part, ci, m, area in regions:
        by_part.setdefault(part, []).append((ci, m, area))
    polys = []
    bridge = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    # dark structural parts: ONE clean convex polygon each (the bodysuit base the
    # plates layer over), not jagged contours of fragments.
    # 'core' (dark torso base) is authored from the torso SILHOUETTE below, not
    # the jagged dark colour mask; helmet/pelvis still come from the dark mask.
    DARK_STRUCTURAL = {"helmet", "pelvis", "bodysuit"}
    dark_base_idx = int(np.argmin(palette.sum(1)))

    def dom_color(masks_items, inst):
        cols = [ci for ci, m, a in masks_items if (m & inst).sum() > 0]
        return max(set(cols), key=cols.count) if cols else masks_items[0][0]

    for part, items in by_part.items():
        union = np.zeros((h, w), np.uint8)
        for ci, m, a in items:
            union |= m.astype(np.uint8)
        if part in SINGLE_PLATE:
            # one clean backing polygon (largest closed component, simplified)
            closed = cv2.morphologyEx(union, cv2.MORPH_CLOSE,
                                      cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
            cnts = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
            if not cnts:
                continue
            pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
            poly = _clean(pts, convex=False, max_edges=12)
            polys.append({"part": part, "color": int(dom_color(items, union > 0)),
                          "area": float(union.sum()), "points": poly.astype(int).tolist()})
            continue
        if part in DARK_STRUCTURAL:
            # close gaps, then take the LARGEST component as a clean (non-convex)
            # torso/helmet base -- convex hull engulfs the figure, jagged
            # fragments look noisy; the largest closed blob simplified is right.
            closed = cv2.morphologyEx(union, cv2.MORPH_CLOSE,
                                      cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (7, 7)))
            cnts = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
            if not cnts:
                continue
            pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
            poly = _clean(pts, convex=False, max_edges=8)
            polys.append({"part": part, "color": int(dom_color(items, union > 0)),
                          "area": float(union.sum()), "points": poly.astype(int).tolist()})
            continue
        if part == "core":
            continue  # authored from the torso silhouette after the loop
        grouped = union if part in CELL_PARTS else cv2.dilate(union, bridge)
        n, lab, stats, cents = cv2.connectedComponentsWithStats(grouped, 8)
        instances = []
        for li in range(1, n):
            inst = (lab == li) & (union > 0)
            if int(inst.sum()) >= 12:
                instances.append(inst)
        # pecs: one wide cream blob -> split L/R into two pecs
        if part == "pec" and len(instances) == 1:
            inst = instances[0]
            xs = np.where(inst.any(0))[0]
            mid = int(xs.mean())
            left = inst.copy(); left[:, mid:] = False
            right = inst.copy(); right[:, :mid] = False
            instances = [m for m in (left, right) if m.sum() >= 12]
        for inst in instances:
            inst_area = int(inst.sum())
            cnts = cv2.findContours(inst.astype(np.uint8), cv2.RETR_EXTERNAL,
                                    cv2.CHAIN_APPROX_SIMPLE)[0]
            if not cnts:
                continue
            pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
            if part in CELL_PARTS:
                poly = _square(pts)
            elif part == "horn":
                ok, tri = cv2.minEnclosingTriangle(pts.astype(np.float32))
                poly = tri.reshape(-1, 2).astype(int) if tri is not None else _clean(pts, False, 4)
            elif part in CONVEX_SPOT:
                poly = _clean(pts, convex=True, max_edges=8)   # irregular convex
            else:
                poly = _clean(pts, convex=False, max_edges=12)
            if len(poly) < 3:
                continue
            polys.append({"part": part, "color": int(dom_color(items, inst)),
                          "area": float(inst_area), "points": poly.astype(int).tolist()})

    # authored dark torso core: trace the VISIBLE dark bodysuit (the dark area
    # the belly grid sits on) carefully -- ~15 edges, keeping the hip detail.
    dark_mask = np.isin(qi, list(dark_idx)).astype(np.uint8)
    band = np.zeros((h, w), np.uint8)
    band[int(0.24 * h):int(0.68 * h), int(0.22 * w):int(0.78 * w)] = 1
    core_mask = cv2.morphologyEx(dark_mask & band, cv2.MORPH_CLOSE,
                                 cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (7, 7)))
    # front/back are symmetric views -> enforce symmetry about the body centreline
    # (the reference torso core is symmetric; the raw dark mask is not).
    if pose in ("top_front", "top_back"):
        col_w = fg.sum(0).astype(float)
        cx = int(round((np.arange(w) * col_w).sum() / max(1.0, col_w.sum())))
        xs = np.arange(w)
        src = 2 * cx - xs
        valid = (src >= 0) & (src < w)
        mir = np.zeros_like(core_mask)
        mir[:, xs[valid]] = core_mask[:, src[valid]]
        # intersection of mask with its mirror -> a symmetric core (avoids the
        # union's lopsided bulges); close to fill the centreline seam.
        core_mask = cv2.morphologyEx((core_mask & mir).astype(np.uint8), cv2.MORPH_CLOSE,
                                     cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    cnts = cv2.findContours(core_mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
    if cnts:
        pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
        poly = _clean(pts, convex=False, max_edges=16)
        polys.append({"part": "core", "color": dark_base_idx,
                      "area": float(core_mask.sum()),
                      "points": poly.astype(int).tolist()})

    # explicit detected eyes on top -- slanted PARALLELOGRAMS (the slit's top
    # sheared toward the face centre) so the character reads a little mean.
    di = int(np.argmin(palette.sum(1)))
    _, eyes = pca_eyes.detect(crop)
    fc = np.mean([(e[0] + e[2]) / 2 for e in eyes]) if eyes else w / 2
    for x0, y0, x1, y1 in eyes:
        cx = (x0 + x1) / 2
        sh = -3 if cx < fc else 3            # shear top OUTWARD -> mean, not sad
        polys.append({"part": "eye", "color": di, "area": float((x1 - x0) * (y1 - y0)),
                      "points": [[x0 + sh, y0], [x1 + sh, y0], [x1, y1], [x0, y1]]})

    polys.sort(key=lambda p: (Z.get(p["part"], 5), -p["area"]))
    return polys, w, h


def render(polys, palette, w, h, outline=False):
    img = np.full((h, w, 3), 255, np.uint8)
    for p in polys:
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], tuple(int(c) for c in palette[p["color"]]))
        if outline:
            cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_AA)
    return img


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default="top_front")
    ap.add_argument("--version", default="09_paperdoll")
    args = ap.parse_args()
    vd = P.version_dir(args.version)
    palette = np.array(json.loads(P.PALETTE_JSON.read_text()))
    polys, w, h = build(args.pose, palette)
    json.dump({"palette": palette.tolist(), "w": w, "h": h, "polys": polys},
              open(vd / f"{args.pose}_polys.json", "w"))
    rec = render(polys, palette, w, h)
    rgba = np.dstack([rec, np.where((rec == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vd / "cand" / f"{args.pose}.png")
    edges = sorted([len(p["points"]) for p in polys], reverse=True)
    from collections import Counter
    print(f"{args.pose}: {len(polys)} polys; edges max={edges[0]} mean={np.mean(edges):.1f}")
    print("part counts:", dict(Counter(p["part"] for p in polys)))
