#!/usr/bin/env python3
"""Per-part nudge optimizer over the authored paper-doll polygons.

The construction is authored (semantic parts, clean shapes); this layer adds the
"optimized nudges" -- a small affine per part (translate + uniform scale) found
by greedy coordinate descent that lowers the pixel diff against the FLAT
quantized reference (shading removed, so we optimize placement/coverage, not
the reference's gradients).  Semantics are preserved: a part only moves/scales,
it never changes which part it is -- the regularizer holds.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image

import pca_paths as P
import pca_paperdoll as PD
from pca_vectorize import quantize


def _quant_target(pose, palette, w, h):
    crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    fg = crop[:, :, 3] >= 127
    qi = quantize(crop[:, :, :3], fg, palette)
    tgt = np.full((h, w, 3), 255, np.uint8)
    for ci in range(len(palette)):
        tgt[qi == ci] = palette[ci]
    return tgt


def _xform(pts, dx, dy, s, rot, cx, cy, kx=0.0):
    p = np.asarray(pts, np.float32) - (cx, cy)
    if rot:
        a = np.radians(rot)
        R = np.array([[np.cos(a), -np.sin(a)], [np.sin(a), np.cos(a)]], np.float32)
        p = p @ R.T
    if kx:                                   # horizontal shear (skew)
        p[:, 0] = p[:, 0] + kx * p[:, 1]
    return p * s + (cx, cy) + (dx, dy)


def optimize(pose: str, version: str, passes: int = 3, log=print):
    vd = P.VERSIONS / version
    d = json.loads((vd / f"{pose}_polys.json").read_text())
    palette = np.array(d["palette"])
    polys = d["polys"]
    w, h = d["w"], d["h"]
    target = _quant_target(pose, palette, w, h).astype(np.int16)

    def loss_of(ps):
        rec = PD.render(ps, palette, w, h).astype(np.int16)
        return int(np.abs(rec - target).sum())

    # GENTLE, shape-preserving moves only (no skew, no big scale/rotation -- those
    # distort and break the spirit of the authored sprite).
    moves = [(1, 0, 1, 0), (-1, 0, 1, 0), (0, 1, 1, 0), (0, -1, 1, 0),
             (2, 0, 1, 0), (-2, 0, 1, 0), (0, 2, 1, 0), (0, -2, 1, 0),
             (0, 0, 1.03, 0), (0, 0, 0.97, 0), (0, 0, 1, 3), (0, 0, 1, -3)]
    # per-part budget: the optimizer may only NUDGE near the manual placement.
    MAX_SHIFT = 6.0          # px the centroid may drift from the authored spot
    SCALE_LO, SCALE_HI = 0.85, 1.15
    orig = []
    for p in polys:
        a = np.asarray(p["points"], np.float32)
        orig.append((a[:, 0].mean(), a[:, 1].mean(),
                     max(1.0, np.ptp(a[:, 0])), max(1.0, np.ptp(a[:, 1]))))
    base = loss_of(polys)
    cur = base
    for _ in range(passes):
        improved = False
        order = sorted(range(len(polys)), key=lambda i: -polys[i]["area"])
        for i in order:
            p = polys[i]
            pts = np.asarray(p["points"], np.float32)
            cx, cy = pts[:, 0].mean(), pts[:, 1].mean()
            ox, oy, ow, oh = orig[i]
            best_pts, best_loss = None, cur
            for dx, dy, s, rot in moves:
                cand = _xform(pts, dx, dy, s, rot, cx, cy)
                ncx, ncy = cand[:, 0].mean(), cand[:, 1].mean()
                # reject anything that drifts/scales beyond the nudge budget --
                # the manual placement stays first-class.
                if (ncx - ox) ** 2 + (ncy - oy) ** 2 > MAX_SHIFT ** 2:
                    continue
                if not (SCALE_LO <= np.ptp(cand[:, 0]) / ow <= SCALE_HI and
                        SCALE_LO <= np.ptp(cand[:, 1]) / oh <= SCALE_HI):
                    continue
                trial = polys[:i] + [{**p, "points": cand.tolist()}] + polys[i + 1:]
                l = loss_of(trial)
                if l < best_loss - 1:
                    best_loss, best_pts = l, cand
            if best_pts is not None:
                polys[i]["points"] = best_pts.astype(int).tolist()
                cur = best_loss
                improved = True
        if not improved:
            break
    # FINAL completeness pass: fill any reference foreground the (nudged) parts
    # still don't cover, so no piece is missing in the saved result.
    crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    fg = crop[:, :, 3] >= 127
    qi = quantize(crop[:, :, :3], fg, palette)
    polys = PD.fill_gaps(polys, qi, fg, palette, w, h)
    d["polys"] = polys
    (vd / f"{pose}_polys.json").write_text(json.dumps(d))
    rec = PD.render(polys, palette, w, h)
    rgba = np.dstack([rec, np.where((rec == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vd / "cand" / f"{pose}.png")
    log(f"{pose:12s} loss {base} -> {cur}  ({100*(base-cur)/max(base,1):.1f}% better)")
    return base, cur


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default="top_front")
    ap.add_argument("--version", default="09_paperdoll")
    ap.add_argument("--passes", type=int, default=3)
    args = ap.parse_args()
    optimize(args.pose, args.version, args.passes)
