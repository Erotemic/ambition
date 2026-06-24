#!/usr/bin/env python3
"""Stamp detected static detail onto a fitted v15 sheet, then render.

Layers, in draw order:
  * dark substrate  (charcoal bodysuit/helmet)  -- PREPENDED, drawn behind plates
  * the fitted polygons (base + overlays + motif grid)
  * back/side carapace spots                    -- appended (on top)
  * cream face hull + eye slits                 -- appended (on top)

All stamped layers are locked and live in absolute reference coords; each pose
is already optimized TO the reference, so the detail lands correctly.  Renders
both the normal dark sheet and a white-background diagnostic (so the dark
silhouette can be verified).
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
from PIL import Image

import pca_fit as F
import pca_detect_spots as D
import pca_substrate as S
import pca_face as FACE

HERE = Path(__file__).resolve().parent


def _rect_poly(box, color, locked=True):
    x0, y0, x1, y1 = box
    pts = np.asarray([[x0, y0], [x1, y0], [x1, y1], [x0, y1]], np.float32)
    return F.Poly(color, False, pts, locked=locked)


def stamp_detail(geoms, ref: Image.Image, specs: dict):
    dfg = S.dark_foreground(ref)
    for name in F.POSE_NAMES:
        g = geoms[name]
        roi = specs[name]["roi"]
        # 1. dark substrate behind everything
        subs = [_rect_poly(r, "black") for r in S.substrate_rects(ref, roi, dfg)]
        g.polys = subs + g.polys
        # 2. carapace spots (top_back / top_side)
        if name in ("top_back", "top_side"):
            for r in D.detect(ref, roi)[:6]:
                g.polys.append(_rect_poly(r["global_box"], "dark_green"))
        # 3. cream face hull + eyes
        n_eyes = FACE.POSE_MAX_EYES.get(name, 0)
        if n_eyes:
            hull, eyes = FACE.detect(ref, roi, max_eyes=n_eyes)
            if hull and len(hull) >= 3:
                g.polys.append(F.Poly("cream", False,
                                      np.asarray(hull, np.float32), locked=True))
            for e in eyes:
                g.polys.append(_rect_poly(e, "black"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fitted", type=Path, required=True)
    ap.add_argument("--ref", type=Path, required=True)
    ap.add_argument("--out-json", type=Path, required=True)
    ap.add_argument("--out-png", type=Path, required=True)
    ap.add_argument("--out-white", type=Path, default=None)
    args = ap.parse_args()

    data = json.loads(args.fitted.read_text())
    data["palette"].update(F.PALETTE_FIX)
    geoms = F.load_geom_v15(data)
    ref = Image.open(args.ref).convert("RGB")
    specs = json.loads((HERE / "pca_roi_specs_v14.json").read_text())["rois"]

    stamp_detail(geoms, ref, specs)

    F.save_geom_v15(geoms, data["palette"], data.get("meta", {}), args.out_json)
    F.render_sheet(geoms, data["palette"]).convert("RGB").save(args.out_png)
    if args.out_white:
        F.render_sheet(geoms, data["palette"], bg=(255, 255, 255)).convert("RGB").save(args.out_white)
    print(f"wrote {args.out_json}\nwrote {args.out_png}")
    if args.out_white:
        print(f"wrote {args.out_white}")


if __name__ == "__main__":
    main()
