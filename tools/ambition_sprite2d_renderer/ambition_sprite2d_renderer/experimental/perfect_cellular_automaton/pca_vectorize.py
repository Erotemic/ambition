#!/usr/bin/env python3
"""Phase B: vectorize a cropped reference sprite into colored polygons.

For each pose crop (RGBA, transparent bg):
  1. quantize foreground pixels to a shared palette (k-means over all crops),
  2. for each palette colour, find connected regions and trace their contours
     (cv2.findContours), then simplify with Douglas-Peucker (approxPolyDP) to a
     low-edge polygon,
  3. emit ``{color, points, area}`` polygons (a few hundred per frame),
  4. re-render and diff against the crop (visual + programmatic).

Palettes/colours are matched to the reference (k-means centroids), so colour is
reproduced, not just silhouette.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

import pca_paths as P

HERE = Path(__file__).resolve().parent
REFS = P.REFS
POSES = P.POSES


def build_palette(k: int = 7, refs: Path = REFS) -> np.ndarray:
    """k-means palette over all foreground pixels of all crops."""
    pix = []
    for n in POSES:
        p = refs / f"{n}.png"
        if not p.exists():
            continue
        a = np.asarray(Image.open(p).convert("RGBA"))
        fg = a[:, :, 3] >= 127
        pix.append(a[fg][:, :3].astype(np.float32))
    pix = np.concatenate(pix, axis=0)
    crit = (cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 30, 0.5)
    _, labels, centers = cv2.kmeans(pix, k, None, crit, 4, cv2.KMEANS_PP_CENTERS)
    counts = np.bincount(labels.ravel(), minlength=k)
    order = np.argsort(-counts)
    return centers[order].astype(np.int32)


def quantize(rgb: np.ndarray, fg: np.ndarray, palette: np.ndarray) -> np.ndarray:
    """Per-pixel nearest-palette index; -1 for background."""
    h, w, _ = rgb.shape
    flat = rgb.reshape(-1, 3).astype(np.int32)
    d = ((flat[:, None, :] - palette[None, :, :]) ** 2).sum(axis=2)
    idx = d.argmin(axis=1).reshape(h, w)
    idx[~fg] = -1
    return idx


def _is_sliver(cnt, area, *, sliver_area=60, sliver_thin=40):
    """A thin small region (high perimeter^2/area): line-art edge noise, not a
    real part -- dropping it removes overcomplication; the neighbouring colour
    polygons' sealed borders cover the gap."""
    per = cv2.arcLength(cnt, True)
    return area < sliver_area and (per * per / max(1.0, area)) > sliver_thin


def vectorize_crop(path: Path, palette: np.ndarray, *, eps_frac: float = 0.01,
                   min_area: int = 8, drop_slivers=(0, 4)):
    a = np.asarray(Image.open(path).convert("RGBA"))
    rgb = a[:, :, :3]
    fg = a[:, :, 3] >= 127
    qi = quantize(rgb, fg, palette)
    polys = []
    for ci in range(len(palette)):
        mask = (qi == ci).astype(np.uint8)
        if mask.sum() < min_area:
            continue
        contours, hier = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        for cnt in contours:
            area = cv2.contourArea(cnt)
            if area < min_area:
                continue
            if ci in drop_slivers and _is_sliver(cnt, area):
                continue
            eps = eps_frac * cv2.arcLength(cnt, True)
            approx = cv2.approxPolyDP(cnt, eps, True).reshape(-1, 2)
            if len(approx) < 3:
                continue
            polys.append({"color": ci, "area": float(area),
                          "points": approx.astype(int).tolist()})
    # draw order: large regions first, small detail on top
    polys.sort(key=lambda p: -p["area"])
    return polys, a.shape[1], a.shape[0]


def vectorize_substrate(path: Path, palette: np.ndarray, *, eps_frac: float = 0.01,
                        min_area: int = 8, dark_idx=(0, 4)):
    """Reference-faithful, low-poly construction: a dark *substrate* = the whole
    silhouette filled with the charcoal colour, then the coloured plates on top.

    The dark helmet / torso core / line-art seams are all just exposed substrate
    -- captured by ONE silhouette polygon instead of dozens of charcoal slivers.
    Coloured plates are drawn slightly *inset* (eroded 1px) so a thin dark seam
    shows between adjacent plates, reproducing the line-art."""
    a = np.asarray(Image.open(path).convert("RGBA"))
    rgb = a[:, :, :3]
    fg = (a[:, :, 3] >= 127).astype(np.uint8)
    qi = quantize(rgb, fg.astype(bool), palette)
    polys = []
    # substrate: silhouette (filled), charcoal colour
    sil = cv2.morphologyEx(fg, cv2.MORPH_CLOSE,
                           cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
    contours, _ = cv2.findContours(sil, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    for cnt in contours:
        area = cv2.contourArea(cnt)
        if area < 40:
            continue
        approx = cv2.approxPolyDP(cnt, eps_frac * cv2.arcLength(cnt, True), True).reshape(-1, 2)
        if len(approx) >= 3:
            polys.append({"color": int(dark_idx[0]), "area": float(area),
                          "points": approx.astype(int).tolist()})
    for ci in range(len(palette)):
        if ci in dark_idx:
            continue  # dark is the substrate
        mask = (qi == ci).astype(np.uint8)
        if mask.sum() < min_area:
            continue
        # no erosion: plates fill their quantized region exactly; the substrate
        # shows only where the reference is actually dark (real line-art) and at
        # the corners DP rounds -- which reads as line-art, not a forced seam.
        contours, _ = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        for cnt in contours:
            area = cv2.contourArea(cnt)
            if area < min_area:
                continue
            approx = cv2.approxPolyDP(cnt, eps_frac * cv2.arcLength(cnt, True), True).reshape(-1, 2)
            if len(approx) < 3:
                continue
            polys.append({"color": ci, "area": float(area),
                          "points": approx.astype(int).tolist()})
    # substrate first, then plates largest->smallest
    sub = [p for p in polys if p["color"] == dark_idx[0] and p["area"] > 0.2 * fg.sum()]
    rest = sorted([p for p in polys if p not in sub], key=lambda p: -p["area"])
    return sub + rest, a.shape[1], a.shape[0]


def render_polys(polys, palette, w, h, bg=(255, 255, 255), seal=2):
    """Fill each polygon; also stroke its border in its own colour to seal the
    1-2px seams that Douglas-Peucker opens between adjacent regions."""
    img = np.full((h, w, 3), bg, np.uint8)
    for p in polys:
        col = tuple(int(c) for c in palette[p["color"]])
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], col)
        if seal:
            cv2.polylines(img, [pts], True, col, seal, cv2.LINE_8)
    return img


def quantized_ref(path: Path, palette: np.ndarray):
    a = np.asarray(Image.open(path).convert("RGBA"))
    fg = a[:, :, 3] >= 127
    qi = quantize(a[:, :, :3], fg, palette)
    out = np.full((*qi.shape, 3), 255, np.uint8)
    for ci in range(len(palette)):
        out[qi == ci] = palette[ci]
    return out, fg


def add_eyes(pose, polys, palette):
    """Append detected eyes as explicit, semantically-tagged 'eye' polygons
    (darkest palette colour, drawn last/on top) so they are guaranteed in the
    canonical polygon set -- eval reconstruction and part map both show them."""
    import pca_eyes
    dark_idx = int(np.argmin(np.array(palette).sum(1)))
    _, eyes = pca_eyes.detect_pose(pose)
    for x0, y0, x1, y1 in eyes:
        polys.append({"color": dark_idx, "area": float((x1 - x0) * (y1 - y0)),
                      "part": "eye", "points": [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]})
    return polys


def process(pose: str, palette: np.ndarray, vdir: Path, eps: float, substrate=False):
    path = REFS / f"{pose}.png"
    if substrate:
        polys, w, h = vectorize_substrate(path, palette, eps_frac=eps)
        polys = add_eyes(pose, polys, palette)
        rec = render_polys(polys, palette, w, h, seal=0)
    else:
        polys, w, h = vectorize_crop(path, palette, eps_frac=eps)
        polys = add_eyes(pose, polys, palette)
        rec = render_polys(polys, palette, w, h)
    # candidate render (RGBA: white->transparent so eval masks fg correctly)
    rgba = np.dstack([rec, np.where((rec == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vdir / "cand" / f"{pose}.png")
    (vdir / f"{pose}_polys.json").write_text(json.dumps(
        {"palette": palette.tolist(), "w": w, "h": h, "polys": polys}))
    return {"polys": len(polys),
            "mean_edges": float(np.mean([len(p["points"]) for p in polys]))}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default=None, help="one pose, or all if omitted")
    ap.add_argument("--k", type=int, default=7)
    ap.add_argument("--eps", type=float, default=0.006)
    ap.add_argument("--version", default="04_vectorized")
    ap.add_argument("--substrate", action="store_true",
                    help="dark-substrate + inset plates (low-poly line-art mode)")
    args = ap.parse_args()
    vdir = P.version_dir(args.version)
    palette = build_palette(args.k)
    P.PALETTE_JSON.write_text(json.dumps(palette.tolist()))
    (vdir / "palette.json").write_text(json.dumps(palette.tolist()))
    print("palette:", palette.tolist())
    todo = [args.pose] if args.pose else POSES
    print(f"{'pose':12s} {'polys':>6s} {'mean_edges':>10s}")
    for pose in todo:
        r = process(pose, palette, vdir, args.eps, substrate=args.substrate)
        print(f"{pose:12s} {r['polys']:6d} {r['mean_edges']:10.1f}")


if __name__ == "__main__":
    main()
