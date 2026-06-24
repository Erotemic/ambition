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
# z-order: lower drawn first (behind). The torso CORE sits OVER the thighs
# (its lower outline -- the pelvis/crotch -- shapes how the upper legs read);
# pecs/chest_plate/belly sit OVER the core.
Z = {"bodysuit": 0, "horn": 0, "tail": 0,
     "upper_arm": 1, "thigh": 1,
     "core": 2,
     "chest_plate": 3, "belly_panel": 3, "forearm": 2, "shin": 2, "helmet": 5,
     "pec": 4, "belly_cell": 4, "knee": 2, "foot": 3, "hand": 3,
     "shoulder": 3, "shoulder_spot": 4, "neck": 5, "face": 6,
     "forehead_cell": 7, "eye": 8, "other": 2, "core_fill": 1}


def _in_head_tight(cx, cy, fb):
    """Tight head box: from the horns (above) down to the FACE BOTTOM only, and
    just past the face sides -- never into the neck/torso, so the helmet can't
    swallow them."""
    fx0, fy0, fx1, fy1 = fb
    fw, fh = fx1 - fx0, fy1 - fy0
    return (fx0 - 0.35 * fw <= cx <= fx1 + 0.35 * fw) and (fy0 - 2.0 * fh <= cy <= fy1)


def _head_label(cx, cy, ci, fb, palette):
    """Label a region as a head part by its position RELATIVE to the detected
    face (view-anchored), so the head is correct regardless of pose/tilt."""
    fx0, fy0, fx1, fy1 = fb
    fh, fw = fy1 - fy0, fx1 - fx0
    fcx = (fx0 + fx1) / 2
    c = palette[ci]
    is_dark = c.sum() < 130
    is_cream = c[0] > 200 and c[1] > 200 and c[2] > 150
    is_green = c[1] > c[0] + 15 and c[1] > 100
    if is_cream:
        return "face"
    if is_green:
        # horns sit high above the face and off-centre; the forehead cells are
        # lower (just above the face top) and central.
        if cy < fy0 - 0.45 * fh and abs(cx - fcx) > 0.22 * fw:
            return "horn"
        return "forehead_cell"
    if is_dark:
        return "helmet"
    return "forehead_cell"


def _cranium(mask: np.ndarray, h: int, face_box=None) -> np.ndarray | None:
    """Carve the dark CRANIUM out of the dark head-band mask so the helmet TRACES
    the head instead of ballooning into a black rectangle.

    The head is the topmost dark blob and it PINCHES at the neck -- the cranium is
    wide, the neck narrow. So: take the top-most connected component, then cut it
    at the first row below its widest point where the row-width collapses (the neck
    pinch). View-general -- needs no face, so it fixes the back view too -- with a
    hard head-height cap and an optional face-bottom cap as backstops."""
    m = cv2.morphologyEx(mask, cv2.MORPH_CLOSE,
                         cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    n, lab, stats, _ = cv2.connectedComponentsWithStats(m, 8)
    if n <= 1:
        return None
    top = 1 + int(np.argmin(stats[1:, cv2.CC_STAT_TOP]))   # component reaching highest
    cm = (lab == top)
    widths = cm.sum(1).astype(int)
    rows = np.where(widths > 0)[0]
    if rows.size == 0:
        return None
    ytop = int(rows[0])
    wmax = int(widths.max())
    ywmax = int(np.argmax(widths))
    ycut = h
    for y in range(ywmax, h):                              # neck pinch below crown
        if widths[y] < 0.40 * wmax:
            ycut = y
            break
    ycut = min(ycut, ytop + int(0.32 * h))                 # hard cap: never tower
    if face_box is not None:
        ycut = min(ycut, int(face_box[3] + 0.30 * (face_box[3] - face_box[1])))
    cm = cm.copy()
    cm[ycut:, :] = False
    return cm.astype(np.uint8)


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
    face_box, eyes = pca_eyes.detect(crop)
    # Eye count = view: front shows 2 eyes, profile 1, back 0. The cream chest
    # has TWO pecs only when the chest faces us (front); in profile a single pec
    # reads, and there is none from the back.
    is_front_view = len(eyes) >= 2
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
    # bridge gaps up to ~4px (the thin dark part-outlines) so same-part fragments
    # merge into ONE clean polygon per instance, not many slivers.
    bridge = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5))
    # dark structural parts: ONE clean convex polygon each (the bodysuit base the
    # plates layer over), not jagged contours of fragments.
    # 'core' (dark torso base) is authored from the torso SILHOUETTE below, not
    # the jagged dark colour mask; helmet/pelvis still come from the dark mask.
    DARK_STRUCTURAL = {"helmet", "pelvis", "bodysuit"}
    dark_base_idx = int(np.argmin(palette.sum(1)))
    # the automaton-cell green (belly + forehead cells): the brightest green in
    # the palette, so cells never read as near-black dark-green.
    _greens = [i for i, c in enumerate(palette) if c[1] > c[0] and c[1] > 100]
    cell_green = max(_greens, key=lambda i: int(palette[i][1])) if _greens else dark_base_idx

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
            # the helmet must TRACE the head, not engulf the whole dark upper
            # figure. Carve the cranium out by the NECK PINCH (view-general: works
            # for the back view, which has no detected face) rather than a fixed
            # box. Trace it with enough edges (10) to follow the real silhouette.
            if part == "helmet":
                cm = _cranium(union, h, face_box)
                if cm is None:
                    continue
                cnts = cv2.findContours(cm, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
                if not cnts:
                    continue
                pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
                poly = _clean(pts, convex=False, max_edges=10)
                polys.append({"part": part, "color": int(dom_color(items, cm > 0)),
                              "area": float(cm.sum()), "points": poly.astype(int).tolist()})
                continue
            # other dark structural parts: close gaps, take the LARGEST component
            # as a clean (non-convex) base -- convex hull engulfs the figure.
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
        if part in ("core", "belly_cell"):
            continue  # authored cleanly after the loop
        grouped = union if part in CELL_PARTS else cv2.dilate(union, bridge)
        n, lab, stats, cents = cv2.connectedComponentsWithStats(grouped, 8)
        instances = []
        for li in range(1, n):
            inst = (lab == li) & (union > 0)
            if int(inst.sum()) >= 12:
                instances.append(inst)
        # pecs: one wide cream blob -> split L/R into two pecs, but ONLY when the
        # chest faces us (front view). In profile a single pec reads as one.
        if part == "pec" and len(instances) == 1 and is_front_view:
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
            color = int(dom_color(items, inst))
            if part in CELL_PARTS:
                poly = _square(pts)
                # forehead cells are the automaton pattern on the SKULL: they read
                # as bright-green squares in the reference. Quantisation splits some
                # to near-black dark-green and shrinks them, so the back-of-head
                # reads as a featureless black box -- snap to the green cell colour
                # and floor the square so the celled skull traces like the ref.
                if part == "forehead_cell":
                    color = cell_green
                    s = max(3, int(0.45 * np.sqrt(max(inst_area, 1))))
                    cx0, cy0 = pts[:, 0].mean(), pts[:, 1].mean()
                    poly = np.array([[cx0 - s, cy0 - s], [cx0 + s, cy0 - s],
                                     [cx0 + s, cy0 + s], [cx0 - s, cy0 + s]])
            elif part == "horn":
                ok, tri = cv2.minEnclosingTriangle(pts.astype(np.float32))
                poly = tri.reshape(-1, 2).astype(int) if tri is not None else _clean(pts, False, 4)
            elif part in CONVEX_SPOT:
                poly = _clean(pts, convex=True, max_edges=8)   # irregular convex
            else:
                poly = _clean(pts, convex=False, max_edges=12)
            if len(poly) < 3:
                continue
            polys.append({"part": part, "color": color,
                          "area": float(inst_area), "points": poly.astype(int).tolist()})

    # (belly grid is authored AFTER the core below -- it is detected geometrically
    # as small square green cells sitting ON the dark core, so it survives action
    # poses where the fixed center-band labelling loses it.)

    # authored dark torso core: the central dark bodysuit (neck -> chest/abdomen
    # -> waist -> pelvis). The raw dark mask tangles every thin part-outline into
    # the core, so OPEN it to drop the thin lines and keep the thick central
    # blob, take the largest component, then trace ~15 edges with hip detail.
    #
    # VIEW-ANCHORED (roadmap #1/#2): the head is always the topmost feature, so we
    # anchor both the neck and the core to the DETECTED FACE rather than to fixed
    # image fractions. The old fixed bands (0.22h-0.67h) only matched the upright
    # front view and dropped a misplaced black blob on every crouched / diving /
    # profile pose. Anchoring to the face bottom keeps the helmet out (it lives
    # above the face) while following the torso wherever the pose puts it.
    dark_mask = np.isin(qi, list(dark_idx)).astype(np.uint8)
    opened = cv2.morphologyEx(dark_mask, cv2.MORPH_OPEN,
                              cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
    if face_box is not None:
        fx0, fy0, fx1, fy1 = [float(v) for v in face_box]
    else:                                   # back view (no face): fall back to fg top
        ys, xs = np.where(fg)
        fy0 = float(ys.min()) if ys.size else 0.0
        fy1 = fy0 + 0.18 * h
        cxw = fg.sum(0).astype(float)
        fcx0 = (np.arange(w) * cxw).sum() / max(1.0, cxw.sum())
        fx0, fx1 = fcx0 - 0.12 * w, fcx0 + 0.12 * w
    fcx = 0.5 * (fx0 + fx1)
    fch = max(1.0, fy1 - fy0)
    fcw = max(1.0, fx1 - fx0)
    face_bottom = fy1

    # dark neck: trapezoid just below the CHIN, in a face-sized box (the character
    # was neck-less); follows the detected face. Only authored when a face is
    # actually visible (front/side) -- the back view has no chin, so the fg-top
    # fallback would otherwise drop a bogus dark square in the middle of the skull.
    neck_band = np.zeros((h, w), np.uint8)
    ny0 = int(max(0, face_bottom - 0.15 * fch)); ny1 = int(min(h, face_bottom + 0.7 * fch))
    nx0 = int(max(0, fcx - 0.55 * fcw)); nx1 = int(min(w, fcx + 0.55 * fcw))
    neck_band[ny0:ny1, nx0:nx1] = 1
    neck_mask = cv2.morphologyEx(dark_mask & neck_band, cv2.MORPH_OPEN,
                                 cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
    ncn, nlab, nstats, _ = cv2.connectedComponentsWithStats(neck_mask, 8)
    if face_box is not None and ncn > 1:
        li = 1 + int(np.argmax(nstats[1:, cv2.CC_STAT_AREA]))
        nc = cv2.findContours((nlab == li).astype(np.uint8), cv2.RETR_EXTERNAL,
                              cv2.CHAIN_APPROX_SIMPLE)[0]
        if nc:
            npts = max(nc, key=cv2.contourArea).reshape(-1, 2)
            if cv2.contourArea(npts) > 20:
                polys.append({"part": "neck", "color": dark_base_idx,
                              "area": float(cv2.contourArea(npts)),
                              "points": _clean(npts, convex=False, max_edges=7).astype(int).tolist()})
    # core = the largest thick dark blob BELOW the face bottom (the helmet, above
    # the face, is cut away by the anchored top edge). No fixed band / no x-bounds:
    # the torso may sit anywhere the pose puts it, so we follow the dark pixels.
    core_mask = opened.copy()
    cut = int(max(0, face_bottom - 0.1 * fch))
    core_mask[:cut, :] = 0
    n, lab, stats, _ = cv2.connectedComponentsWithStats(core_mask, 8)
    if n > 1:
        li = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
        core_mask = (lab == li).astype(np.uint8)
    # front/back are symmetric views -> union with the mirror about the centreline
    # makes a symmetric core that keeps full width on both sides.
    if pose in ("top_front", "top_back"):
        col_w = fg.sum(0).astype(float)
        cx = int(round((np.arange(w) * col_w).sum() / max(1.0, col_w.sum())))
        xs = np.arange(w)
        src = 2 * cx - xs
        valid = (src >= 0) & (src < w)
        mir = np.zeros_like(core_mask)
        mir[:, xs[valid]] = core_mask[:, src[valid]]
        core_mask = (core_mask | mir).astype(np.uint8)
    core_mask = cv2.morphologyEx(core_mask, cv2.MORPH_CLOSE,
                                 cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
    cnts = cv2.findContours(core_mask, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
    if cnts:
        pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
        poly = _clean(pts, convex=False, max_edges=16)
        polys.append({"part": "core", "color": dark_base_idx,
                      "area": float(core_mask.sum()),
                      "points": poly.astype(int).tolist()})

    # ---- belly grid (geometric, view-general) ----
    # The automaton belly grid is the character's signature, but the fixed
    # center-band label loses it whenever the torso moves (idle/air/land had ~0
    # cells). Detect it instead by GEOMETRY: small, square, filled GREEN blobs that
    # sit on the dark CORE (lower half of the core bbox). Then fit a regular NxM
    # array of equal squares -- consistent cells, like the reference.
    green_idx = [i for i, c in enumerate(palette) if c[1] > c[0] and c[1] > 100]
    green = np.isin(qi, green_idx).astype(np.uint8) if green_idx else np.zeros((h, w), np.uint8)
    cys, cxs = np.where(core_mask > 0)
    cells = []
    if cys.size:
        cy0, cy1, cx0, cx1 = cys.min(), cys.max(), cxs.min(), cxs.max()
        ch = max(1, cy1 - cy0); cw = max(1, cx1 - cx0)
        belly_y0 = cy0 + 0.20 * ch          # belly = lower ~2/3 of the core
        gn, glab, gst, gce = cv2.connectedComponentsWithStats(green, 8)
        for i in range(1, gn):
            a = gst[i, cv2.CC_STAT_AREA]
            bw, bh = gst[i, cv2.CC_STAT_WIDTH], gst[i, cv2.CC_STAT_HEIGHT]
            if a < 6 or a > 0.012 * w * h:                 # cell-sized only
                continue
            if max(bw, bh) > 2.6 * max(1, min(bw, bh)):    # square-ish
                continue
            if a < 0.5 * bw * bh:                          # filled (not an L/ring)
                continue
            gx, gy = gce[i]
            if cx0 - 0.10 * cw <= gx <= cx1 + 0.10 * cw and belly_y0 <= gy <= cy1 + 0.18 * ch:
                cells.append((gx, gy))
    if len(cells) >= 4:
        cxa = np.array([c[0] for c in cells]); cya = np.array([c[1] for c in cells])
        gx0, gy0, gx1, gy1 = cxa.min(), cya.min(), cxa.max(), cya.max()
        gw, gh = max(1, gx1 - gx0), max(1, gy1 - gy0)
        ncols = max(1, int(round(np.sqrt(len(cells) * gw / gh))))
        nrows = max(1, int(round(len(cells) / ncols)))
        pitch_x = gw / max(1, ncols - 1)
        pitch_y = gh / max(1, nrows - 1)
        cell = 0.66 * min(pitch_x, pitch_y) if ncols > 1 and nrows > 1 else 8
        degenerate = cell > 0.18 * w or pitch_x > 0.22 * w or pitch_y > 0.30 * h
        if not degenerate:
            for r in range(nrows):
                for c in range(ncols):
                    ux, uy = gx0 + c * pitch_x, gy0 + r * pitch_y
                    iy, ix = int(round(uy)), int(round(ux))
                    rad = max(1, int(cell * 0.4))
                    if not green[max(0, iy - rad):iy + rad + 1, max(0, ix - rad):ix + rad + 1].any():
                        continue
                    s = cell / 2
                    polys.append({"part": "belly_cell", "color": cell_green, "area": float(cell * cell),
                                  "points": [[int(ux - s), int(uy - s)], [int(ux + s), int(uy - s)],
                                             [int(ux + s), int(uy + s)], [int(ux - s), int(uy + s)]]})

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

    # cap ONLY the decorative shoulder spots (keep the largest 4). Never cap
    # structural parts -- area-based dropping can discard a real foot/claw and
    # keep a sliver, which is how the feet vanished. Limb fragments are cleaned
    # by merging (the bridge), never by dropping content.
    inst = sorted([p for p in polys if p["part"] == "shoulder_spot"], key=lambda p: -p["area"])
    if len(inst) > 4:
        drop = set(id(p) for p in inst[4:])
        polys = [p for p in polys if id(p) not in drop]
    polys.sort(key=lambda p: (Z.get(p["part"], 5), -p["area"]))
    return polys, w, h


# accents read as flat colour with NO outline; everything else (the main arm /
# torso / leg / head parts) gets a thick black line-art outline like the reference.
ACCENTS = {"belly_cell", "forehead_cell", "shoulder_spot", "eye"}


def fill_gaps(polys, qi, fg, palette, w, h, min_area=28):
    """COMPLETENESS (run LAST, after the optimizer): any reference foreground not
    covered by a candidate polygon becomes a polygon of its dominant reference
    colour, labelled by position. The reference interior is pristine, so a gap is
    a missing piece -- feet-darks, tail connectors, stray segments are never lost.
    Conservative: opened to drop thin edge slivers, and only gaps >= min_area."""
    rec = render(polys, palette, w, h, outline=False)
    covered = ~(rec == 255).all(axis=2)
    gap = (fg & ~covered).astype(np.uint8)
    gap = cv2.morphologyEx(gap, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
    gn, glab, gst, gce = cv2.connectedComponentsWithStats(gap, 8)
    for li in range(1, gn):
        if gst[li, cv2.CC_STAT_AREA] < min_area:
            continue
        m = (glab == li)
        cnts = cv2.findContours(m.astype(np.uint8), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
        if not cnts:
            continue
        pts = max(cnts, key=cv2.contourArea).reshape(-1, 2)
        col = int(np.bincount(qi[m], minlength=len(palette)).argmax())
        cx, cy = gce[li]
        area = gst[li, cv2.CC_STAT_AREA]
        part = PARTS.label_part(cx / w, cy / h, col, area / (w * h))
        if part == "core":
            part = "core_fill"
        # A genuine belly cell is tiny; a LARGE green gap labelled belly_cell is
        # really uncovered limb/torso (common in profile/back) -- filling it as a
        # big outlined SQUARE makes a floating block. Keep it a flat clean poly
        # (still belly_cell -> accent, no outline) so it blends into the figure.
        is_cell = part in CELL_PARTS and area < (0.05 * w) ** 2
        poly = _square(pts) if is_cell else _clean(pts, convex=False, max_edges=10)
        if len(poly) >= 3:
            polys.append({"part": part, "color": col, "area": float(gst[li, cv2.CC_STAT_AREA]),
                          "points": poly.astype(int).tolist()})
    polys.sort(key=lambda p: (Z.get(p["part"], 5), -p["area"]))
    return polys


def render(polys, palette, w, h, outline=False):
    """outline=False -> line-art look: main parts get a thick black outline,
    accents none.  outline=True -> diagnostic: every polygon stroked 1px."""
    img = np.full((h, w, 3), 255, np.uint8)
    for p in polys:
        pts = np.array(p["points"], np.int32)
        cv2.fillPoly(img, [pts], tuple(int(c) for c in palette[p["color"]]))
        if outline:
            cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_AA)
        elif p.get("part") not in ACCENTS:
            cv2.polylines(img, [pts], True, (0, 0, 0), 2, cv2.LINE_AA)
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
