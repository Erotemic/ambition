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

import numpy as np
from PIL import Image, ImageDraw, ImageFont

import pca_paths as P

WHITE = (255, 255, 255)


def _font(sz):
    try:
        return ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf", sz)
    except Exception:
        return ImageFont.load_default()


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


def eval_pose(pose: str, cand_dir: Path):
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
    metrics = {"iou": iou, "mean_diff": mean_diff, "color_match": match,
               "ref_px": int(ref_fg.sum()), "cand_px": int(cand_fg.sum())}
    heat = np.stack([np.clip(diff, 0, 255)] * 3, -1).astype(np.uint8)
    panel = np.concatenate([ref_rgb, cand_rgb, heat], axis=1)
    return metrics, Image.fromarray(panel)


def run(version: str, metrics_only: bool = False):
    vd = P.version_dir(version)
    cand_dir = vd / "cand"
    eval_dir = vd / "eval"
    metrics = {}
    panels = []
    for pose in P.POSES:
        r = eval_pose(pose, cand_dir)
        if r is None:
            continue
        m, panel = r
        metrics[pose] = m
        if not metrics_only:
            d = ImageDraw.Draw(panel)
            d.text((4, 2), f"{pose}  IoU {m['iou']:.3f}  diff {m['mean_diff']:.1f}"
                           f"  match {m['color_match']*100:.0f}%", fill=(255, 0, 255), font=_font(13))
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
        print(f"{'pose':12s} {'IoU':>6s} {'diff':>6s} {'match%':>7s}")
        for pose in P.POSES:
            if pose in metrics:
                m = metrics[pose]
                print(f"{pose:12s} {m['iou']:6.3f} {m['mean_diff']:6.1f} {m['color_match']*100:6.0f}%")
        print(f"{'MEAN':12s} {mean['iou']:6.3f} {mean['mean_diff']:6.1f} {mean['color_match']*100:6.0f}%")
    return metrics


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--version", required=True)
    ap.add_argument("--metrics-only", action="store_true")
    args = ap.parse_args()
    run(args.version, args.metrics_only)


if __name__ == "__main__":
    main()
