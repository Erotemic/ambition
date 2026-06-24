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

HERE = Path(__file__).resolve().parent
REFS = Path(__file__).resolve().parents[3] / "agent-scratch" / "refs"
POSES = ["top_front", "top_side", "top_back", "pose_idle", "pose_walk_1",
         "pose_walk_2", "pose_attack", "pose_jump", "pose_air", "pose_land"]


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


def vectorize_crop(path: Path, palette: np.ndarray, *, eps_frac: float = 0.01,
                   min_area: int = 8):
    a = np.asarray(Image.open(path).convert("RGBA"))
    rgb = a[:, :, :3]
    fg = a[:, :, 3] >= 127
    qi = quantize(rgb, fg, palette)
    polys = []
    for ci in range(len(palette)):
        mask = (qi == ci).astype(np.uint8)
        if mask.sum() < min_area:
            continue
        # RETR_CCOMP: outer contours + holes; keep only outer (level-0) here,
        # holes are covered by other colours drawn on top.
        contours, hier = cv2.findContours(mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
        for cnt in contours:
            area = cv2.contourArea(cnt)
            if area < min_area:
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


def process(pose: str, palette: np.ndarray, out_dir: Path, eps: float):
    path = REFS / f"{pose}.png"
    polys, w, h = vectorize_crop(path, palette, eps_frac=eps)
    rec = render_polys(polys, palette, w, h)
    qref, fg = quantized_ref(path, palette)
    ref = np.asarray(Image.open(path).convert("RGBA"))
    refc = ref[:, :, :3].copy(); refc[~fg] = (255, 255, 255)
    # diff vs quantized (isolates polygon-fit error from shading)
    dq = np.abs(qref.astype(int) - rec.astype(int)).sum(2)
    fit_iou = ((dq[fg] < 30).sum()) / max(1, fg.sum())
    panel = np.concatenate([refc, qref, rec,
                            np.stack([np.clip(dq, 0, 255)] * 3, -1).astype(np.uint8)], axis=1)
    Image.fromarray(panel).save(out_dir / f"{pose}_vec.png")
    (out_dir / f"{pose}_polys.json").write_text(json.dumps(
        {"palette": palette.tolist(), "w": w, "h": h, "polys": polys}))
    return {"polys": len(polys), "fit_match_frac": float(fit_iou),
            "mean_edges": float(np.mean([len(p["points"]) for p in polys]))}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default=None, help="one pose, or all if omitted")
    ap.add_argument("--k", type=int, default=7)
    ap.add_argument("--eps", type=float, default=0.01)
    ap.add_argument("--out", type=Path, default=Path("agent-scratch/vec"))
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    palette = build_palette(args.k)
    (args.out / "palette.json").write_text(json.dumps(palette.tolist()))
    print("palette:", palette.tolist())
    todo = [args.pose] if args.pose else POSES
    print(f"{'pose':12s} {'polys':>6s} {'mean_edges':>10s} {'fit_match%':>10s}")
    for pose in todo:
        r = process(pose, palette, args.out, args.eps)
        print(f"{pose:12s} {r['polys']:6d} {r['mean_edges']:10.1f} {r['fit_match_frac']*100:9.1f}%")


if __name__ == "__main__":
    main()
