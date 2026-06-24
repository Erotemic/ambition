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


def _xform(pts, dx, dy, s, cx, cy):
    p = np.asarray(pts, np.float32)
    return ((p - (cx, cy)) * s + (cx, cy) + (dx, dy))


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

    moves = [(1, 0, 1.0), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0),
             (2, 0, 1.0), (-2, 0, 1.0), (0, 2, 1.0), (0, -2, 1.0),
             (0, 0, 1.04), (0, 0, 0.96), (0, 0, 1.08), (0, 0, 0.92)]
    base = loss_of(polys)
    cur = base
    for _ in range(passes):
        improved = False
        order = sorted(range(len(polys)), key=lambda i: -polys[i]["area"])
        for i in order:
            p = polys[i]
            pts = np.asarray(p["points"], np.float32)
            cx, cy = pts[:, 0].mean(), pts[:, 1].mean()
            best_pts, best_loss = None, cur
            for dx, dy, s in moves:
                trial = polys[:i] + [{**p, "points": _xform(pts, dx, dy, s, cx, cy).tolist()}] + polys[i + 1:]
                l = loss_of(trial)
                if l < best_loss - 1:
                    best_loss, best_pts = l, _xform(pts, dx, dy, s, cx, cy)
            if best_pts is not None:
                polys[i]["points"] = best_pts.astype(int).tolist()
                cur = best_loss
                improved = True
        if not improved:
            break
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
