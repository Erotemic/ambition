#!/usr/bin/env python3
"""Hand-principled rebuild of top_back: COMPLETE, LAYERED, SYMMETRIC parts; clean
low-poly shapes; NO shading; keep the spot dots. Built from the symmetrised
quantised reference (the back is mirror-symmetric about its centreline).

Layers (back->front): core -> back_plate -> spots -> limbs -> head.
"""
from __future__ import annotations
import json
import numpy as np
import cv2
from PIL import Image
import pca_paths as P
from pca_vectorize import quantize

CHRCL, GREEN, PURPLE, MGREEN, BLACK, CREAM, DGREEN = range(7)
DARK = [CHRCL, BLACK]
GREENS = [GREEN, MGREEN]


def _largest(mask):
    n, lab, st, _ = cv2.connectedComponentsWithStats(mask.astype(np.uint8), 8)
    if n <= 1:
        return mask
    return (lab == 1 + int(np.argmax(st[1:, cv2.CC_STAT_AREA]))).astype(np.uint8)


def _clean(mask, max_edges=8, convex=False, close=5):
    m = cv2.morphologyEx(mask.astype(np.uint8), cv2.MORPH_CLOSE,
                         cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (close, close)))
    cnts = cv2.findContours(m, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
    if not cnts:
        return None
    pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
    if convex:
        pts = cv2.convexHull(pts).reshape(-1, 2)
    eps = 0.01 * cv2.arcLength(pts.reshape(-1, 1, 2), True)
    for _ in range(8):
        ap = cv2.approxPolyDP(pts.reshape(-1, 1, 2), eps, True).reshape(-1, 2)
        if len(ap) <= max_edges:
            return ap
        eps *= 1.3
    return ap


def build(pose="top_back", version="21_clean_back"):
    pal = np.array(json.loads(P.PALETTE_JSON.read_text()))
    crop = np.asarray(Image.open(P.REFS / f"{pose}.png").convert("RGBA"))
    h, w = crop.shape[:2]
    fg = crop[:, :, 3] >= 127
    qi = quantize(crop[:, :, :3], fg, pal)
    cx = int(round(np.where(fg)[1].mean()))           # symmetry centreline

    def mirror(mask):
        xs = np.arange(w); src = 2 * cx - xs
        v = (src >= 0) & (src < w)
        out = np.zeros_like(mask); out[:, xs[v]] = mask[:, src[v]]
        return out

    def colmask(cols, y0=0.0, y1=1.0, x0=0.0, x1=1.0):
        m = np.isin(qi, cols) & fg
        band = np.zeros((h, w), bool)
        band[int(y0 * h):int(y1 * h), int(x0 * w):int(x1 * w)] = True
        return (m & band).astype(np.uint8)

    def sym(mask):                                     # symmetric union
        return ((mask | mirror(mask)) > 0).astype(np.uint8)

    polys = []

    def add(part, color, mask, max_edges=8, convex=False, close=5):
        p = _clean(mask, max_edges, convex, close)
        if p is not None and len(p) >= 3 and cv2.contourArea(p.reshape(-1, 1, 2).astype(np.int32)) > 20:
            polys.append({"part": part, "color": int(color), "area": float(int(mask.sum())),
                          "points": p.astype(int).tolist()})

    def add_pair(part, color, mask, **kw):
        """left side from the mask, right side mirrored -> guaranteed symmetric."""
        left = mask.copy(); left[:, cx:] = 0
        if left.sum() > 20:
            add(part, color, left, **kw)
            add(part, color, mirror(left), **kw)

    # ---- CORE (COMPLETE dark backing: the whole central body silhouette, so the
    # dark shows through every gap between the layered colour parts, like the ref) ----
    body = sym(((fg) & np.isin(qi, [CHRCL, BLACK, GREEN, MGREEN, DGREEN, PURPLE, CREAM]).astype(bool)).astype(np.uint8))
    band = np.zeros((h, w), np.uint8); band[int(0.27 * h):int(0.72 * h), int(0.20 * w):int(0.80 * w)] = 1
    core = _largest(cv2.morphologyEx((body & band).astype(np.uint8), cv2.MORPH_CLOSE,
                                     cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (13, 13))))
    add("core", BLACK, core, max_edges=12)

    # ---- BACK PLATE (one clean symmetric octagon of green over the upper/mid back) ----
    back = sym(colmask(GREENS, 0.30, 0.58, 0.18, 0.82))
    back = _largest(back)
    add("back_plate", MGREEN, back, max_edges=8, convex=True)

    # ---- 4 SPOT DOTS (dgreen diamond on the back plate) ----
    spots = colmask([DGREEN], 0.30, 0.55, 0.20, 0.80)
    n, lab, st, ce = cv2.connectedComponentsWithStats(spots, 8)
    cand = sorted([(ce[i], st[i, 4]) for i in range(1, n) if st[i, 4] >= 60], key=lambda t: -t[1])
    for (sx, sy), _a in cand[:4]:
        r = 7
        polys.append({"part": "back_spot", "color": DGREEN, "area": float(4 * r * r),
                      "points": [[int(sx - r), int(sy - r)], [int(sx + r), int(sy - r)],
                                 [int(sx + r), int(sy + r)], [int(sx - r), int(sy + r)]]})

    # ---- LEGS (complete, paired, OVERLAPPING segments so no white seams) ----
    add_pair("thigh", PURPLE, sym(colmask([PURPLE], 0.60, 0.84, 0.20, 0.80)), max_edges=7)
    add_pair("shin", GREEN, sym(colmask(GREENS, 0.76, 0.97, 0.20, 0.80)), max_edges=7)
    add_pair("foot", CREAM, sym(colmask([CREAM], 0.90, 1.0, 0.18, 0.82)), max_edges=6)

    # ---- ARMS (complete, paired, overlapping): green upper, purple lower, cream hand ----
    add_pair("upper_arm", GREEN, sym(colmask(GREENS, 0.28, 0.52, 0.0, 0.32)), max_edges=7)
    add_pair("forearm", PURPLE, sym(colmask([PURPLE], 0.44, 0.70, 0.0, 0.32)), max_edges=7, close=9)
    add_pair("hand", CREAM, sym(colmask([CREAM], 0.60, 0.76, 0.0, 0.20)), max_edges=6)
    # shoulders (green pauldrons high on the sides)
    add_pair("shoulder", GREEN, sym(colmask(GREENS, 0.27, 0.41, 0.0, 0.30)), max_edges=7)

    # ---- HEAD (helmet, horns, forehead cells) ----
    add("helmet", CHRCL, _largest(sym(colmask(DARK, 0.03, 0.29, 0.24, 0.76))),
        max_edges=8, convex=False, close=11)
    # horns: green at the very top, off centre -> paired triangles
    add_pair("horn", GREEN, colmask(GREENS, 0.0, 0.12, 0.0, 1.0), max_edges=3)
    # forehead cells: small green squares on the helmet -> keep as a small grid
    fc = colmask(GREENS, 0.10, 0.26, 0.30, 0.70)
    nn, ll, ss, cc = cv2.connectedComponentsWithStats(fc, 8)
    for i in range(1, nn):
        if ss[i, 4] < 12:
            continue
        sx, sy = cc[i]; r = 5
        polys.append({"part": "forehead_cell", "color": GREEN, "area": float(4 * r * r),
                      "points": [[int(sx - r), int(sy - r)], [int(sx + r), int(sy - r)],
                                 [int(sx + r), int(sy + r)], [int(sx - r), int(sy + r)]]})

    Z = {"core": 0, "back_plate": 1, "back_spot": 2, "thigh": 1, "shin": 1, "foot": 2,
         "upper_arm": 1, "forearm": 2, "hand": 3, "shoulder": 3, "helmet": 4, "horn": 3,
         "forehead_cell": 5}
    polys.sort(key=lambda p: (Z.get(p["part"], 2), -p["area"]))
    vd = P.version_dir(version)
    out = {"w": w, "h": h, "palette": pal.tolist(), "polys": polys}
    (vd / f"{pose}_polys.json").write_text(json.dumps(out))
    # render flat (no shading), 1px line-art on non-accent parts
    ACC = {"back_spot", "forehead_cell"}
    img = np.full((h, w, 3), 255, np.uint8)
    for p in polys:
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], tuple(int(c) for c in pal[p["color"]]))
        if p["part"] not in ACC:
            cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_AA)
    rgba = np.dstack([img, np.where((img == 255).all(2), 0, 255).astype(np.uint8)])
    Image.fromarray(rgba, "RGBA").save(vd / "cand" / f"{pose}.png")
    print(f"built {pose}: {len(polys)} polys ->", {p: sum(1 for q in polys if q['part'] == p) for p in sorted(set(q['part'] for q in polys))})


if __name__ == "__main__":
    build()
