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
                         cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
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


def _densest_cluster(cells):
    """Keep only the largest cluster of cell-centres linked within ~2x the median
    nearest-neighbour spacing -- the tight belly grid -- and drop spread-out
    outliers (square leg-armour segments that pass the per-cell tests)."""
    if len(cells) < 4:
        return cells
    pts = np.array(cells, float)
    D = np.sqrt(((pts[:, None] - pts[None, :]) ** 2).sum(2))
    np.fill_diagonal(D, np.inf)
    s = float(np.median(D.min(1)))
    adj = D <= 2.2 * max(1.0, s)
    seen, best = set(), []
    for i in range(len(pts)):
        if i in seen:
            continue
        stack, comp = [i], []
        while stack:
            j = stack.pop()
            if j in seen:
                continue
            seen.add(j); comp.append(j)
            stack.extend(int(k) for k in np.where(adj[j])[0] if k not in seen)
        if len(comp) > len(best):
            best = comp
    return [tuple(pts[i]) for i in best]


def _square(pts: np.ndarray) -> np.ndarray:
    (cx, cy), (w, h), ang = cv2.minAreaRect(pts.astype(np.float32))
    s = (w + h) / 2.0
    return cv2.boxPoints(((cx, cy), (s, s), ang)).astype(int)


def _spike(pts: np.ndarray) -> np.ndarray:
    """A horn is a TAPERED spike, not a fat min-enclosing triangle. Find the
    region's long axis, take the narrow end as the TIP and the wide end as the
    base; emit [tip, base_left, base_right] so it reads as a pointed horn even
    when the green blob is chunky (side view)."""
    p = pts.astype(np.float32)
    c = p.mean(0)
    d = p - c
    cov = np.cov(d.T) if len(p) > 2 else np.eye(2)
    evals, evecs = np.linalg.eigh(cov)
    major = evecs[:, int(np.argmax(evals))]
    minor = np.array([-major[1], major[0]], np.float32)
    proj = d @ major
    mproj = d @ minor
    rng = max(1e-3, float(np.ptp(proj)))
    hi = proj > proj.max() - 0.35 * rng       # one end
    lo = proj < proj.min() + 0.35 * rng       # other end
    spread_hi = float(np.ptp(mproj[hi])) if hi.any() else 0.0
    spread_lo = float(np.ptp(mproj[lo])) if lo.any() else 0.0
    if spread_hi <= spread_lo:                # narrow end is the tip
        tip = p[hi][int(np.argmax(proj[hi]))]
        base = lo
    else:
        tip = p[lo][int(np.argmin(proj[lo]))]
        base = hi
    if not base.any():
        return _square(pts)[:3]
    bl = p[base][int(np.argmin(mproj[base]))]
    br = p[base][int(np.argmax(mproj[base]))]
    return np.array([tip, bl, br]).astype(int)


def _basepart(part):
    return part[:-6] if part.endswith("_shade") else part


def _add_shading(polys, qi, fg, palette, w, h):
    """Each part is one flat colour, but the reference SHADES every part (lit +
    shadow). With perfect per-pixel colour our exact shapes would score ~+33pts,
    so the whole remaining gap is shading. Add it: for each main part, find the
    secondary palette shades that actually fall inside it and emit them as
    no-outline OVERLAY polygons (`<part>_shade`) drawn just above the base. Keeps
    the clean line-art silhouette; adds the lit/shadow tones."""
    extra = []
    for p in polys:
        part = p["part"]
        if part in ACCENTS or part.endswith("_shade") or part in ("eye", "neck"):
            continue
        mask = np.zeros((h, w), np.uint8)
        cv2.fillPoly(mask, [np.array(p["points"], np.int32)], 1)
        m = (mask > 0) & fg
        a = int(m.sum())
        if a < 90:
            continue
        base = p["color"]
        counts = np.bincount(qi[m], minlength=len(palette))
        for ci in range(len(palette)):
            if ci == base or counts[ci] < 0.10 * a:
                continue
            if int(np.abs(palette[ci].astype(int) - palette[base].astype(int)).sum()) < 55:
                continue
            reg = cv2.morphologyEx(((qi == ci) & m).astype(np.uint8), cv2.MORPH_OPEN,
                                   cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3)))
            cnts = cv2.findContours(reg, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
            for c in cnts:
                if cv2.contourArea(c) < max(20, 0.035 * a):
                    continue
                poly = _clean(c.reshape(-1, 2), convex=False, max_edges=10)
                if len(poly) >= 3:
                    extra.append({"part": part + "_shade", "color": int(ci),
                                  "area": float(cv2.contourArea(c)), "points": poly.astype(int).tolist()})
    return polys + extra


def _despike(pts: np.ndarray, min_angle=16.0) -> np.ndarray:
    """Drop needle vertices -- a corner whose interior angle is razor-sharp is a
    degenerate spike (a thin sliver poking out of an otherwise blobby part, e.g.
    the chest-plate / shoulder / helmet). Collapsing it removes the spike without
    moving the rest of the outline. (Horns use _spike and eyes/cells are authored
    directly, so intentional points never reach here.)"""
    pl = [tuple(map(int, p)) for p in pts]
    changed = True
    while changed and len(pl) > 3:
        changed = False
        for i in range(len(pl)):
            a = np.array(pl[i - 1], float); v = np.array(pl[i], float); b = np.array(pl[(i + 1) % len(pl)], float)
            e1, e2 = a - v, b - v
            n1, n2 = np.linalg.norm(e1), np.linalg.norm(e2)
            if n1 < 1 or n2 < 1 or np.degrees(np.arccos(np.clip(e1 @ e2 / (n1 * n2), -1, 1))) < min_angle:
                pl.pop(i); changed = True; break
    return np.array(pl)


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

    # ---- BODY FRAME (view-general torso/limb labelling) ----
    # label_part's bands are image fractions, so a limb that crosses the image
    # centre in a dive/crouch (air/land/walk) gets mislabelled torso. Instead map
    # each region to BODY-RELATIVE coords along the head->torso axis and feed THOSE
    # to label_part: for upright poses the axis is vertical so body-space == image-
    # space (no regression); for tilted poses the bands follow the body.
    ys, xs = np.where(fg)
    fg_cx, fg_cy = float(xs.mean()), float(ys.mean())
    if face_box is not None:
        hx = 0.5 * (face_box[0] + face_box[2]); hy = 0.5 * (face_box[1] + face_box[3])
    else:                                            # back: head = top-of-figure centroid
        top = ys.min(); hy = float(top); hx = float(xs[ys < top + 0.1 * h].mean())
    axis = np.array([fg_cx - hx, fg_cy - hy], float)
    nrm = float(np.hypot(*axis))
    axis = axis / nrm if nrm > 5 else np.array([0.0, 1.0])   # default straight down
    perp = np.array([-axis[1], axis[0]])
    pa = (xs - hx) * axis[0] + (ys - hy) * axis[1]   # along-body of every fg px
    pb = (xs - hx) * perp[0] + (ys - hy) * perp[1]   # across-body
    amin, arng = float(pa.min()), max(1.0, float(np.ptp(pa)))
    bmin, brng = float(pb.min()), max(1.0, float(np.ptp(pb)))

    def body_frac(cx, cy):
        va = (cx - hx) * axis[0] + (cy - hy) * axis[1]
        vb = (cx - hx) * perp[0] + (cy - hy) * perp[1]
        return (vb - bmin) / brng, (va - amin) / arng    # bnx, bny in [0,1]

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
            # HEAD parts are labelled relative to the DETECTED FACE (view-anchored)
            # so horns/cells/helmet are correct regardless of head tilt -- the fixed
            # image-band heuristic mislabels e.g. centre-x horns as forehead cells
            # when the head leans (pose_attack). Torso/limbs still use label_part.
            if face_box is not None and _in_head_tight(cx, cy, face_box):
                part = _head_label(cx, cy, ci, np.asarray(face_box, float), palette)
                # a forehead cell is a SMALL automaton square; a LARGE green blob in
                # the head box is a shoulder a crouch (low face) lifted into the head
                # region -- relabel it as a body part, not a giant floating cell.
                if part == "forehead_cell" and area > 0.005 * w * h:
                    bnx, bny = body_frac(cx, cy)
                    part = PARTS.label_part(bnx, max(0.30, bny), ci, area / (w * h))
            else:
                bnx, bny = body_frac(cx, cy)
                part = PARTS.label_part(bnx, bny, ci, area / (w * h))
                # the head is handled ONLY by the view-anchored branch above; a
                # region OUTSIDE the head box must not be a head part (the body
                # frame can misfire at the far end of an extreme dive). Push it
                # below the head band so it gets a torso/limb label instead.
                if face_box is not None and part in ("face", "horn", "forehead_cell", "helmet"):
                    part = PARTS.label_part(bnx, max(0.30, bny), ci, area / (w * h))
                elif face_box is None and part == "face":
                    # no face is visible from behind -> cream up here is collar /
                    # shoulder trim, never a face. Push it below the head band.
                    part = PARTS.label_part(bnx, max(0.30, bny), ci, area / (w * h))
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
            poly = _despike(_clean(pts, convex=False, max_edges=12))   # plate: no needles
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
                poly = _clean(pts, convex=False, max_edges=14)
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
                poly = _spike(pts)
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
    # the inner-thigh dark shadow continues down from the torso as a THIN spike,
    # shooting the core into the legs (degenerate sub-degree angles + dark where
    # the thighs are purple). Open it away -- the wide torso survives, the thin
    # leg spikes are severed -- then take the largest blob.
    core_mask = cv2.morphologyEx(core_mask, cv2.MORPH_OPEN,
                                 cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (7, 7)))
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

    # ---- belly grid (geometric, view-general) ----  [built BEFORE the chest plate
    # so the chest plate can exclude this footprint -- otherwise the cells merge
    # into the plate as one green blob ("M" bug).]
    # The automaton belly grid is the character's signature, but the fixed
    # center-band label loses it whenever the torso moves (idle/air/land had ~0
    # cells). Detect it instead by GEOMETRY: small, square, filled GREEN blobs that
    # sit on the dark CORE (lower 2/3 of the core bbox). Then fit a regular NxM
    # array of equal squares -- consistent cells, like the reference.
    green_idx = [i for i, c in enumerate(palette) if c[1] > c[0] and c[1] > 100]
    green = np.isin(qi, green_idx).astype(np.uint8) if green_idx else np.zeros((h, w), np.uint8)
    belly_region = np.zeros((h, w), np.uint8)     # footprint of the grid, for the chest plate
    cys, cxs = np.where(core_mask > 0)
    cells = []
    # the belly grid faces the camera only -- visible from the front, a little from
    # the side, NEVER from the back. The back view has no face, so skip it there.
    if face_box is not None and cys.size:
        cy0, cy1, cx0, cx1 = cys.min(), cys.max(), cxs.min(), cxs.max()
        ch = max(1, cy1 - cy0); cw = max(1, cx1 - cx0)
        belly_y0 = cy0 + 0.20 * ch
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
            if cx0 - 0.10 * cw <= gx <= cx1 + 0.10 * cw and belly_y0 <= gy <= cy1:
                cells.append((gx, gy))
        cells = _densest_cluster(cells)
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
            # the grid's bounding footprint (cells + their gaps), so the chest plate
            # can carve it out and the cells render as a clean grid on dark.
            belly_region[int(gy0 - cell):int(gy1 + cell), int(gx0 - cell):int(gx1 + cell)] = 1

    # ---- chest plate (geometric, view-general) ----
    # The UPPER torso is green chest armor over the dark bodysuit; the reference
    # torso is ~half green. Author it from the GREEN over the upper core (central,
    # not the lateral arms), EXCLUDING the belly-grid footprint, drawn OVER the
    # core so the dark no longer dominates the action poses.
    green_ids = green_idx
    green_all = green
    cpy, cpx = np.where(core_mask > 0)
    if cpy.size:
        ky0, ky1 = int(cpy.min()), int(cpy.max())
        kx0, kx1 = int(cpx.min()), int(cpx.max())
        kh = max(1, ky1 - ky0); kw = max(1, kx1 - kx0)
        band = np.zeros((h, w), np.uint8)
        band[int(ky0):int(ky0 + 0.6 * kh), int(kx0 - 0.05 * kw):int(kx1 + 0.05 * kw)] = 1
        chest = (green_all & band).astype(np.uint8)
        chest[belly_region > 0] = 0                       # don't swallow the grid
        cn, clab, cst, cce = cv2.connectedComponentsWithStats(chest, 8)
        keep = np.zeros((h, w), np.uint8)
        for i in range(1, cn):
            if cst[i, cv2.CC_STAT_AREA] < 30:
                continue
            gx = cce[i][0]
            if kx0 - 0.05 * kw <= gx <= kx1 + 0.05 * kw:   # central (over the core), not a far arm
                keep |= (clab == i).astype(np.uint8)
        if keep.sum() >= 0.05 * core_mask.sum():
            keep = cv2.morphologyEx(keep, cv2.MORPH_CLOSE,
                                    cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (5, 5)))
            keep[belly_region > 0] = 0                    # re-carve after the close
            cc2 = cv2.findContours(keep, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)[0]
            if cc2:
                cp = max(cc2, key=cv2.contourArea).reshape(-1, 2)
                if cv2.contourArea(cp) > 40:
                    gpx = qi[keep > 0]
                    gpx = gpx[np.isin(gpx, green_ids)]
                    cpc = int(np.bincount(gpx).argmax()) if gpx.size else cell_green
                    polys.append({"part": "chest_plate", "color": cpc,
                                  "area": float(keep.sum()),
                                  "points": _despike(_clean(cp, convex=False, max_edges=12)).astype(int).tolist()})

    # explicit detected eyes on top -- slanted PARALLELOGRAMS (the slit's top
    # sheared toward the face centre) so the character reads a little mean.
    di = int(np.argmin(palette.sum(1)))
    _, eyes = pca_eyes.detect(crop)
    # keep only eyes that actually sit in the detected face (a foreshortened dive
    # can produce a false cream-with-slits blob elsewhere -> a stray eye/face).
    if face_box is not None and eyes:
        fx0, fy0, fx1, fy1 = face_box
        mx, my = 0.5 * (fx1 - fx0), 0.5 * (fy1 - fy0)
        eyes = [e for e in eyes if fx0 - mx <= (e[0] + e[2]) / 2 <= fx1 + mx
                and fy0 - my <= (e[1] + e[3]) / 2 <= fy1 + my]
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
    polys = _brighten_limb_greens(polys, palette)
    polys.sort(key=lambda p: (Z.get(p["part"], 5), -p["area"]))
    return polys, w, h


# green structural parts: should read GREEN, not as the dark-green SHADOW shade.
GREEN_LIMB = {"shin", "knee", "tail", "upper_arm", "shoulder", "chest_plate"}


def _brighten_limb_greens(polys, palette):
    """A whole limb/plate segment whose dominant colour is a dark-green SHADOW
    shade renders as a dark olive blob (the legs in air/land, the chest plate).
    Shadow is shading, not the part's identity -- flatten every green-family limb
    fill to the BRIGHTEST green so the part reads green and matches the reference
    (which is mostly the bright green; the dark shade is only deep shadow). Keeps
    cohesion (Jon: 'look better than the reference -- consistent, noise-free').

    Note the green test allows the dark-green shade (g>100 would drop it): a green
    is g>r and clearly greener than blue (g-b margin), regardless of brightness."""
    greens = [i for i, c in enumerate(palette) if c[1] > c[0] and c[1] - c[2] > 15]
    if len(greens) < 2:
        return polys
    brightest = max(greens, key=lambda i: int(palette[i].sum()))
    greenset = set(greens)
    for p in polys:
        if p["part"] in GREEN_LIMB and p["color"] in greenset and p["color"] != brightest:
            p["color"] = brightest
    return polys


# accents read as flat colour with NO outline; everything else (the main arm /
# torso / leg / head parts) gets a thick black line-art outline like the reference.
ACCENTS = {"belly_cell", "forehead_cell", "shoulder_spot", "eye"}

PART_GROUP = {
    "helmet": "head", "horn": "head", "face": "head", "eye": "head", "forehead_cell": "head",
    "neck": "torso", "core": "torso", "core_fill": "torso", "bodysuit": "torso",
    "chest_plate": "torso", "pec": "torso", "belly_panel": "torso", "belly_cell": "torso",
    "tail": "tail", "shoulder": "arm", "shoulder_spot": "arm", "upper_arm": "arm",
    "forearm": "arm", "hand": "arm", "thigh": "leg", "shin": "leg", "knee": "leg", "foot": "leg",
}
_CREAM_PARTS = {"face", "pec", "hand", "foot", "belly_panel"}


def _reclassify_cells_by_base(polys, w, h):
    """A green automaton CELL must belong to whatever body part it sits ON -- the
    head-box / band heuristics mislabel cells that land on the shoulders (head),
    torso, etc. Look up the BASE part beneath each cell (z-ordered) and relabel it
    to that group's accent: head->forehead_cell, torso->belly_cell, arm->
    shoulder_spot, leg/tail->the base part itself. A green cell that lands on a
    CREAM part (the face) does not belong -> drop it."""
    bases = [p for p in polys if p["part"] not in ACCENTS and not p["part"].endswith("_shade")]
    if not bases:
        return polys
    pid = np.full((h, w), -1, np.int32)
    for i in sorted(range(len(bases)), key=lambda i: (Z.get(bases[i]["part"], 5), bases[i]["area"])):
        cv2.fillPoly(pid, [np.array(bases[i]["points"], np.int32)], i)
    cell_for = {"head": "forehead_cell", "torso": "belly_cell", "arm": "shoulder_spot"}
    out = []
    for p in polys:
        if p["part"] in ("forehead_cell", "belly_cell"):
            # majority base part under the cell's FOOTPRINT (centroid alone lands in
            # 1px seams), not a single pixel.
            cm = np.zeros((h, w), np.uint8)
            cv2.fillPoly(cm, [np.array(p["points"], np.int32)], 1)
            ids = pid[(cm > 0) & (pid >= 0)]
            if ids.size < 0.25 * max(1, int(cm.sum())):   # cell floats over nothing -> drop
                continue
            basepart = bases[int(np.bincount(ids).argmax())]["part"]
            if basepart in _CREAM_PARTS:        # green square on a cream part -> noise, drop
                continue
            g = PART_GROUP.get(basepart, "torso")
            p = {**p, "part": cell_for.get(g, basepart)}   # leg/tail cell -> its base part
        out.append(p)
    return out


def fill_gaps(polys, qi, fg, palette, w, h, min_area=28):
    """COMPLETENESS (run LAST, after the optimizer): any reference foreground not
    covered by a candidate polygon becomes a polygon of its dominant reference
    colour, labelled by position. The reference interior is pristine, so a gap is
    a missing piece -- feet-darks, tail connectors, stray segments are never lost.
    Conservative: opened to drop the thin AA-perimeter slivers, and only gaps
    >= min_area. Do NOT chase the perimeter / small seams -- those are the soft-
    edge vs hard-edge artifact, not real missing polygons."""
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
    polys = _brighten_limb_greens(polys, palette)
    polys = _reclassify_cells_by_base(polys, w, h)
    polys = _add_shading(polys, qi, fg, palette, w, h)
    polys.sort(key=lambda p: (Z.get(_basepart(p["part"]), 5), p["part"].endswith("_shade"), -p["area"]))
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
        elif p.get("part") not in ACCENTS and not p.get("part", "").endswith("_shade"):
            # 1px line-art: a 2px stroke was the single biggest source of colour
            # error (~50% of it) -- on these ~200px crops the reference line-art is
            # ~1px, so a 2px black band landed where the reference is colour. 1px
            # still reads as line-art and lifts mean colour-match ~+5%.
            # _shade overlays are interior tones -> filled, never outlined.
            cv2.polylines(img, [pts], True, (0, 0, 0), 1, cv2.LINE_AA)
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
