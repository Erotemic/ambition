#!/usr/bin/env python3
"""Standard diagnostic for ANY tactic: ref | candidate | diff, per pose + montage.

A candidate is a directory of per-pose PNGs (``cand/<pose>.png``) aligned to the
reference crops in ``inputs/refs/``.  This computes the SAME diagnostics for
every version so they are comparable over time:

  * per-pose panel: reference | candidate | diff-heatmap (+ metric caption)
  * a montage of all poses
  * metrics.json: per-pose silhouette IoU, mean colour diff over the union,
    and colour-match%% (fraction of union pixels within 30 L1 of the reference)

Usage:
  pca_eval.py --version 04_vectorized            # eval an existing cand/ dir
  pca_eval.py --version X --metrics-only         # just print the table
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image, ImageDraw, ImageFont

import pca_paths as P

WHITE = (255, 255, 255)
LABEL_H = 20


def _font(sz):
    try:
        return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", sz)
    except Exception:
        return ImageFont.load_default()


def _labeled(arr: np.ndarray, text: str) -> Image.Image:
    """Add a header bar with ``text`` above an image array."""
    h, w = arr.shape[:2]
    panel = Image.new("RGB", (w, h + LABEL_H), (24, 24, 28))
    panel.paste(Image.fromarray(arr), (0, LABEL_H))
    d = ImageDraw.Draw(panel)
    d.text((4, 3), text, fill=(255, 255, 255), font=_font(13))
    return panel


def _outlined(polys, palette, w, h) -> np.ndarray:
    """Filled polygons, each with a 1px BLACK outline -- exposes how many
    polygons / how much overlap we are using (overcomplication)."""
    img = np.full((h, w, 3), WHITE, np.uint8)
    for p in sorted(polys, key=lambda p: -p["area"]):
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], tuple(int(c) for c in palette[p["color"]]))
        cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_8)
    return img


def _on_white(path: Path):
    """Return (rgb_on_white uint8, fg_mask bool)."""
    a = np.asarray(Image.open(path).convert("RGBA"))
    if a.shape[2] == 4:
        fg = a[:, :, 3] >= 127
    else:
        fg = np.ones(a.shape[:2], bool)
    rgb = a[:, :, :3].copy()
    rgb[~fg] = WHITE
    return rgb, fg


def eval_pose(pose: str, vd: Path):
    cand_dir = vd / "cand"
    ref_rgb, ref_fg = _on_white(P.REFS / f"{pose}.png")
    cand_path = cand_dir / f"{pose}.png"
    if not cand_path.exists():
        return None
    cand_rgb, cand_fg = _on_white(cand_path)
    if cand_rgb.shape[:2] != ref_rgb.shape[:2]:
        im = Image.fromarray(cand_rgb).resize((ref_rgb.shape[1], ref_rgb.shape[0]), Image.NEAREST)
        cand_rgb = np.asarray(im)
        fim = Image.fromarray((cand_fg * 255).astype(np.uint8)).resize(
            (ref_rgb.shape[1], ref_rgb.shape[0]), Image.NEAREST)
        cand_fg = np.asarray(fim) >= 127
    union = ref_fg | cand_fg
    inter = ref_fg & cand_fg
    diff = np.abs(ref_rgb.astype(int) - cand_rgb.astype(int)).sum(2)
    iou = float(inter.sum() / max(1, union.sum()))
    mean_diff = float(diff[union].mean()) if union.any() else 0.0
    match = float((diff[union] < 30).mean()) if union.any() else 1.0

    # optional polygons -> count + outlined view
    n_poly = None
    outlined = None
    pj = vd / f"{pose}_polys.json"
    if pj.exists():
        d = json.loads(pj.read_text())
        n_poly = len(d["polys"])
        outlined = _outlined(d["polys"], np.array(d["palette"]), d["w"], d["h"])
    metrics = {"iou": iou, "mean_diff": mean_diff, "color_match": match,
               "n_poly": n_poly, "ref_px": int(ref_fg.sum()), "cand_px": int(cand_fg.sum())}

    heat = np.stack([np.clip(diff, 0, 255)] * 3, -1).astype(np.uint8)
    recon_label = "reconstruction" + (f"  ({n_poly} poly)" if n_poly is not None else "")
    cols = [_labeled(ref_rgb, "reference"),
            _labeled(cand_rgb, recon_label),
            _labeled(heat, f"difference  IoU {iou:.3f} diff {mean_diff:.0f} match {match*100:.0f}%")]
    if outlined is not None:
        cols.append(_labeled(outlined, "outlined (no diff)"))
    H = max(c.height for c in cols)
    panel = Image.new("RGB", (sum(c.width for c in cols), H), (24, 24, 28))
    x = 0
    for c in cols:
        panel.paste(c, (x, 0)); x += c.width
    return metrics, panel


def run(version: str, metrics_only: bool = False):
    vd = P.version_dir(version)
    eval_dir = vd / "eval"
    metrics = {}
    panels = []
    for pose in P.POSES:
        r = eval_pose(pose, vd)
        if r is None:
            continue
        m, panel = r
        metrics[pose] = m
        if not metrics_only:
            panel.save(eval_dir / f"{pose}.png")
            panels.append(panel)
    if metrics:
        mean = {k: float(np.mean([metrics[p][k] for p in metrics]))
                for k in ("iou", "mean_diff", "color_match")}
        metrics["_mean"] = mean
        (eval_dir / "metrics.json").write_text(json.dumps(metrics, indent=2))
        if panels:
            w = max(p.width for p in panels)
            montage = Image.new("RGB", (w, sum(p.height + 4 for p in panels)), (20, 20, 24))
            y = 0
            for p in panels:
                montage.paste(p, (0, y)); y += p.height + 4
            montage.save(eval_dir / "montage.png")
        print(f"=== {version} ===")
        print(f"{'pose':12s} {'n_poly':>6s} {'IoU':>6s} {'diff':>6s} {'match%':>7s}")
        npolys = []
        for pose in P.POSES:
            if pose in metrics:
                m = metrics[pose]
                nps = "-" if m["n_poly"] is None else str(m["n_poly"])
                if m["n_poly"] is not None:
                    npolys.append(m["n_poly"])
                print(f"{pose:12s} {nps:>6s} {m['iou']:6.3f} {m['mean_diff']:6.1f} {m['color_match']*100:6.0f}%")
        npm = f"{np.mean(npolys):.0f}" if npolys else "-"
        print(f"{'MEAN':12s} {npm:>6s} {mean['iou']:6.3f} {mean['mean_diff']:6.1f} {mean['color_match']*100:6.0f}%")
    return metrics


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--version", required=True)
    ap.add_argument("--metrics-only", action="store_true")
    args = ap.parse_args()
    run(args.version, args.metrics_only)


if __name__ == "__main__":
    main()
