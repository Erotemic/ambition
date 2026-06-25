#!/usr/bin/env python3
"""Polish a hand-authored pose: close the gaps WITHOUT distorting the shapes.

Each uncovered foreground pixel (internal seam between layered parts, or the thin
perimeter inside the reference silhouette) is assigned to its NEAREST part; that
part's boundary is then re-traced to swallow the pixels it gained -- i.e. the
vertices move outward just enough to close the gap. Draw order (z) is preserved
from the input, so growing a backing layer stays hidden behind the front layers.
"""
from __future__ import annotations
import argparse
import json
import numpy as np
import cv2
from scipy.ndimage import distance_transform_edt
from PIL import Image

import pca_paths as P


def _retrace(mask, n_src):
    """Largest contour of mask, simplified to about the source vertex count
    (keep the shape, just move/close the boundary)."""
    m = cv2.morphologyEx(mask.astype(np.uint8), cv2.MORPH_CLOSE,
                         cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    cnts = cv2.findContours(m, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
    if not cnts:
        return None
    pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
    target = max(4, min(n_src + 2, 14))
    eps = 0.004 * cv2.arcLength(pts.reshape(-1, 1, 2), True)
    for _ in range(10):
        ap = cv2.approxPolyDP(pts.reshape(-1, 1, 2), eps, True).reshape(-1, 2)
        if len(ap) <= target:
            return ap
        eps *= 1.3
    return ap


def polish(pose, in_version, out_version, grow_into_ref=True):
    d = json.loads((P.VERSIONS / in_version / f"{pose}_polys.json").read_text())
    w, h, pal, polys = d["w"], d["h"], np.array(d["palette"]), d["polys"]
    ref = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    rfg = ref[:, :, 3] >= 127

    masks = [cv2.fillPoly(np.zeros((h, w), np.uint8), [np.array(p["points"], np.int32)], 1).astype(bool)
             for p in polys]
    lab_top = np.full((h, w), -1, np.int32)
    for i, m in enumerate(masks):                 # later (front) overwrites -> topmost id
        lab_top[m] = i
    covered = lab_top >= 0

    target = (rfg | covered) if grow_into_ref else covered  # fill seams + the perimeter inside ref
    gap = target & ~covered
    # Assign each gap pixel to the EARLIEST (most-backing) part within reach, NOT the
    # topmost. A seam between two front parts (shoulder vs pec) should fill with the
    # BACKING behind them (the dark-green chest backing / dark core) -- which is what
    # the reference shows there -- not the green/cream of the front parts.
    REACH = 10
    k = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (2 * REACH + 1, 2 * REACH + 1))
    assign = np.full((h, w), -1, np.int32)
    free = gap.copy()
    for i in range(len(masks)):                      # document order: backing -> front
        if not free.any():
            break
        reach = (cv2.dilate(masks[i].astype(np.uint8), k) > 0) & free
        assign[reach] = i
        free &= ~reach

    grew = 0
    for i, p in enumerate(polys):
        add = assign == i
        if not add.any():
            continue
        new = _retrace(masks[i] | add, len(p["points"]))
        if new is not None and len(new) >= 3:
            p["points"] = new.astype(int).tolist()
            p["area"] = float((masks[i] | add).sum())
            grew += 1

    vd = P.version_dir(out_version)
    out = {"w": w, "h": h, "palette": pal.tolist(), "polys": polys}
    (vd / f"{pose}_polys.json").write_text(json.dumps(out))
    # render in the SAME order (preserve z); accents (no label match) get no outline
    ACC = {"pec", "back_spot", "belly_cell", "forehead_cell", "eye", "shoulder_spot"}
    img = np.full((h, w, 3), 255, np.uint8)
    for p in polys:
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], tuple(int(c) for c in pal[p["color"]]))
        lab = p["part"]
        if not (lab in ACC or "spot" in lab or "cell" in lab or "eye" in lab or lab == "pec"):
            cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_AA)
    rgba = np.dstack([img, np.where((img == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vd / "cand" / f"{pose}.png")
    cov2 = ~(img == 255).all(2)
    print(f"polished {pose}: grew {grew} parts; ref-uncovered {int((rfg & ~cov2).sum())} "
          f"(was {int((rfg & ~covered).sum())})")
    return out


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--pose", default="top_front")
    ap.add_argument("--in-version", default="20_jon")
    ap.add_argument("--version", default="22_jon_polish")
    a = ap.parse_args()
    polish(a.pose, a.in_version, a.version)
