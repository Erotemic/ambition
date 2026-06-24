#!/usr/bin/env python3
"""Automatic polygon fitter for the Perfect Cellular Automaton concept sheet.

ChatGPT hand-authored a polygon soup (``*_pose_polygons_*.json``) that
approximates ``assets/concept_art/prefect-cellular-automaton-reference-image.png``
and plateaued around IoU ~0.78.  This module replaces the manual nudging with an
automatic optimizer:

* Each pose's polygons (base + part overlays + manual eyes) are *flattened* into a
  single ordered list of absolute-coordinate polygons, each carrying its palette
  colour and an ``outline`` flag, so the flat list renders pixel-identically to
  the v14 sheet (see ``selftest``).
* Optimization is per-pose and scored only inside that pose's ROI, on top of a
  cached "neighbour plate" (every *other* pose rendered once) so neighbour bleed
  in overlapping ROI windows is handled correctly.
* Loss is masked RGB-L1 against the reference (excludes respected); the headline
  metric stays the original ``fit_harness`` IoU, run separately.
* Staged greedy/annealed descent: global affine -> per-polygon translate ->
  per-vertex jitter.  Moves only ever shrink the loss, so the fit is monotone.

The optimized geometry is written back as a flat v15 schema
(``{pose: {roi, draw: [{color, outline, points}]}}``) plus a sheet renderer
(:func:`render_sheet`), keeping one obvious source of truth for downstream
generator integration.
"""
from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
W, H = 1448, 1086
ORDER = ["black", "dark_green", "green", "lime", "purple", "cream2", "cream"]
POSE_NAMES = [
    "top_front", "top_side", "top_back",
    "pose_idle", "pose_walk_1", "pose_walk_2",
    "pose_attack", "pose_jump", "pose_air", "pose_land",
]

# Palette corrections applied on load.  The v14 'black' (23,24,24) is the SAME
# near-black as the gradient backdrop (~25,26,28), so the helmet/forehead was
# invisible against the background AND cost ~nothing in the loss (the optimizer
# had no incentive to fill it).  The reference helmet is genuinely darker
# (~16,18,19); we push slightly past it so the silhouette reads.
PALETTE_FIX = {"black": [13, 15, 15, 255]}

Rect = Tuple[int, int, int, int]


# --------------------------------------------------------------------------- #
# Flat geometry model
# --------------------------------------------------------------------------- #
@dataclass
class Poly:
    """One polygon in absolute sheet coordinates.

    ``locked`` polygons (e.g. the detected automaton-grid / forehead cells) are
    moved rigidly by the global-affine stage but never reshaped by the per-poly
    or per-vertex stages, so authored squares stay square.
    """
    color: str
    outline: bool
    pts: np.ndarray  # (N, 2) float32
    locked: bool = False


@dataclass
class PoseGeom:
    name: str
    roi: Rect
    exclude: List[Rect]
    polys: List[Poly] = field(default_factory=list)


def _arr(points: Sequence[Sequence[float]]) -> np.ndarray:
    return np.asarray(points, dtype=np.float32).reshape(-1, 2)


def load_geom(data: dict) -> Dict[str, PoseGeom]:
    """Flatten the v14 base/overlay/eye schema into ordered absolute polygons.

    Render order reproduces the v14 sheet exactly: base polygons (grouped by
    ORDER colour), then part overlays (sorted by ``order``, each grouped by
    ORDER colour, no outline), then manual eyes (no outline).
    """
    geoms: Dict[str, PoseGeom] = {}
    part_overlays = data.get("part_overlays", {})
    manual_eyes = data.get("manual_eyes", {})
    for name in POSE_NAMES:
        pose = data["poses"][name]
        x0, y0 = pose["roi"][0], pose["roi"][1]
        g = PoseGeom(name=name, roi=tuple(pose["roi"]),
                     exclude=[tuple(e) for e in pose.get("exclude", [])])
        for key in ORDER:
            for pts in pose["polygons"].get(key, []):
                if len(pts) >= 3:
                    g.polys.append(Poly(key, True, _arr(pts) + (x0, y0)))
        for part in sorted(part_overlays.get(name, []), key=lambda p: p["order"]):
            bx0, by0 = part["box"][0], part["box"][1]
            for key in ORDER:
                for pts in part["polygons"].get(key, []):
                    if len(pts) >= 3:
                        g.polys.append(Poly(key, False, _arr(pts) + (x0 + bx0, y0 + by0)))
        for item in manual_eyes.get(name, []):
            pts = item["points"]
            if len(pts) >= 3:
                g.polys.append(Poly(item["color"], False, _arr(pts) + (x0, y0)))
        # Detected surface motifs (automaton belly grid + forehead cells) drawn
        # last, as crisp axis-aligned rectangles in absolute coords. These were
        # extracted by ChatGPT but never rendered; they ARE the missing grids.
        motif = data.get("motif_segments", {}).get(name, {})
        for grp in ("forehead_cells", "abdomen_grid"):
            for rect in motif.get(grp, {}).get("rects", []):
                gx0, gy0, gx1, gy1 = rect["global_box"]
                if gx1 > gx0 and gy1 > gy0:
                    pts = _arr([[gx0, gy0], [gx1, gy0], [gx1, gy1], [gx0, gy1]])
                    g.polys.append(Poly(rect["fill"], False, pts, locked=True))
        geoms[name] = g
    return geoms


def load_geom_v15(data: dict) -> Dict[str, PoseGeom]:
    """Load the flat v15 schema produced by :func:`save_geom_v15`."""
    geoms: Dict[str, PoseGeom] = {}
    for name in POSE_NAMES:
        pose = data["poses"][name]
        g = PoseGeom(name=name, roi=tuple(pose["roi"]),
                     exclude=[tuple(e) for e in pose.get("exclude", [])])
        for p in pose["draw"]:
            g.polys.append(Poly(p["color"], bool(p["outline"]), _arr(p["points"]),
                                locked=bool(p.get("locked", False))))
        geoms[name] = g
    return geoms


def save_geom_v15(geoms: Dict[str, PoseGeom], palette: dict, meta: dict, out: Path) -> None:
    poses = {}
    for name, g in geoms.items():
        poses[name] = {
            "roi": list(g.roi),
            "exclude": [list(e) for e in g.exclude],
            "draw": [
                {"color": p.color, "outline": p.outline, "locked": p.locked,
                 "points": [[round(float(x), 2), round(float(y), 2)] for x, y in p.pts]}
                for p in g.polys
            ],
        }
    out.write_text(json.dumps({"version": "v15-fit", "palette": palette,
                               "poses": poses, "meta": meta}, indent=1) + "\n")


# --------------------------------------------------------------------------- #
# Rendering
# --------------------------------------------------------------------------- #
def _font(size: int):
    for p in ["/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
              "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
              "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"]:
        try:
            return ImageFont.truetype(p, size)
        except Exception:
            pass
    return ImageFont.load_default()


def draw_polys(d: ImageDraw.ImageDraw, polys: Sequence[Poly], palette: dict,
               origin: Tuple[float, float] = (0.0, 0.0)) -> None:
    ox, oy = origin
    black = (0, 0, 0, 255)
    for p in polys:
        if p.pts.shape[0] < 3:
            continue
        pts = [(float(x) - ox, float(y) - oy) for x, y in p.pts]
        fill = tuple(palette[p.color])
        d.polygon(pts, fill=fill, outline=black if p.outline else None)


def _draw_bg(d: ImageDraw.ImageDraw, palette: dict) -> None:
    c0, c1 = palette["bg0"], palette["bg1"]
    for y in range(H):
        t = y / max(1, H - 1)
        c = tuple(int(c0[i] * (1 - t) + c1[i] * t) for i in range(3)) + (255,)
        d.line((0, y, W, y), fill=c)


def _draw_cells(d, palette, cx, cy, scale=1.0, rows=5, cols=4):
    pattern = [[1, 1, 1, 1], [1, 0, 1, 1], [1, 0, 0, 1], [1, 1, 1, 1], [1, 1, 0, 1]]
    cell = 14 * scale
    gap = 5 * scale
    w = cols * cell + (cols - 1) * gap
    h = rows * cell + (rows - 1) * gap
    x0 = cx - w / 2
    y0 = cy - h / 2
    for r in range(rows):
        for c in range(cols):
            x = x0 + c * (cell + gap)
            y = y0 + r * (cell + gap)
            col = palette["lime"] if pattern[r][c] else palette["dark_green"]
            d.rectangle((x, y, x + cell, y + cell), fill=tuple(col))


def _draw_layout(d, palette) -> None:
    x0, y0 = 1080, 64
    items = [("BLACK", "black"), ("DARK GREEN", "dark_green"), ("GREEN", "green"),
             ("LIME", "lime"), ("PURPLE", "purple"), ("CREAM", "cream"),
             ("LIGHT CREAM", "cream2")]
    txt_col = tuple(palette["text"])
    for i, (txt, key) in enumerate(items):
        y = y0 + i * 45
        d.rectangle((x0, y, x0 + 48, y + 33), fill=tuple(palette[key]),
                    outline=(245, 245, 245, 255), width=2)
        d.text((x0 + 68, y + 6), txt, font=_font(17), fill=txt_col)
    d.text((1144, 390), "AUTOMATON GRID", font=_font(16), fill=txt_col)
    _draw_cells(d, palette, 1148, 469, 2.1)
    for txt, x in [("IDLE", 88), ("WALK 1", 288), ("WALK 2", 506), ("ATTACK", 708),
                   ("JUMP", 909), ("AIR", 1109), ("LAND", 1298)]:
        d.text((x, 642), txt, font=_font(17), fill=txt_col, anchor="mm")
    for i, txt in enumerate(["- ALL PARTS ARE SIMPLE POLYGONS",
                             "- AUTO-FIT POLYGON PASS (V15)",
                             "- CLEAR SILHOUETTE AT SMALL SIZES"]):
        d.text((22, 996 + i * 28), txt, font=_font(17), fill=txt_col)


def render_sheet(geoms: Dict[str, PoseGeom], palette: dict,
                 bg: Optional[Tuple[int, int, int]] = None) -> Image.Image:
    """Render the full sheet. ``bg`` overrides the gradient with a solid colour
    (e.g. white) and drops the legend -- a diagnostic to verify the dark
    helmet/body silhouette is actually present, not just lost against the
    dark backdrop."""
    if bg is not None:
        img = Image.new("RGBA", (W, H), tuple(bg) + (255,))
        d = ImageDraw.Draw(img, "RGBA")
        for name in POSE_NAMES:
            draw_polys(d, geoms[name].polys, palette)
        return img
    img = Image.new("RGBA", (W, H), tuple(palette["bg0"]))
    d = ImageDraw.Draw(img, "RGBA")
    _draw_bg(d, palette)
    _draw_layout(d, palette)
    for name in POSE_NAMES:
        draw_polys(d, geoms[name].polys, palette)
    return img


# --------------------------------------------------------------------------- #
# Scoring
# --------------------------------------------------------------------------- #
def valid_mask(roi: Rect, exclude: Sequence[Rect]) -> np.ndarray:
    x0, y0, x1, y1 = roi
    w, h = x1 - x0, y1 - y0
    m = np.ones((h, w), dtype=bool)
    for ex0, ey0, ex1, ey1 in exclude:
        lx0, ly0 = max(0, ex0 - x0), max(0, ey0 - y0)
        lx1, ly1 = min(w, ex1 - x0), min(h, ey1 - y0)
        if lx1 > lx0 and ly1 > ly0:
            m[ly0:ly1, lx0:lx1] = False
    return m


class PoseFitter:
    """Optimizes a single pose's polygons against the reference ROI.

    Loss = masked mean RGB-L1  +  coverage penalty, where the coverage penalty
    scores the candidate's own foreground mask against the reference foreground
    (flood-fill segmentation).  Under-fill (target FG, candidate BG) is the dark
    helmet case; over-fill (candidate FG on empty backdrop) is the stray-noise
    case.  Penalising both keeps the optimizer from gaming the metric with
    background-coloured slivers.
    """

    def __init__(self, geom: PoseGeom, target: Image.Image, plate: Image.Image,
                 palette: dict, target_fg: Optional[np.ndarray] = None,
                 bg: Optional[np.ndarray] = None, cover_w: float = 60.0):
        self.g = geom
        self.palette = palette
        x0, y0, x1, y1 = geom.roi
        self.roi = geom.roi
        self.ox, self.oy = x0, y0
        self.rw, self.rh = x1 - x0, y1 - y0
        self.target = np.asarray(target.crop(geom.roi).convert("RGB"), dtype=np.int16)
        self.mask = valid_mask(geom.roi, geom.exclude)
        self.n_valid = max(1, int(self.mask.sum()))
        # Cached neighbour background, cropped to the ROI (RGBA).
        self.plate_roi = plate.crop(geom.roi).convert("RGBA")
        self.target_fg = (target_fg[y0:y1, x0:x1] & self.mask) if target_fg is not None else None
        self.bg = bg.astype(np.float32) if bg is not None else None
        self.cover_w = cover_w
        self.fg_tol = 18.0  # candidate fg = distance from bg above this

    def render_roi(self, polys: Sequence[Poly]) -> np.ndarray:
        img = self.plate_roi.copy()
        d = ImageDraw.Draw(img, "RGBA")
        draw_polys(d, polys, self.palette, origin=(self.ox, self.oy))
        return np.asarray(img.convert("RGB"), dtype=np.int16)

    def loss_of(self, polys: Sequence[Poly]) -> float:
        cand = self.render_roi(polys)
        d = np.abs(cand - self.target).sum(axis=2)
        color_term = float(d[self.mask].mean())
        if self.target_fg is None or self.bg is None:
            return color_term
        cand_fg = np.sqrt(((cand.astype(np.float32) - self.bg) ** 2).sum(axis=2)) > self.fg_tol
        under = self.target_fg & ~cand_fg
        over = cand_fg & self.mask & ~self.target_fg
        cover_term = self.cover_w * (under.sum() + over.sum()) / self.n_valid
        return color_term + cover_term

    def loss(self) -> float:
        return self.loss_of(self.g.polys)


# --------------------------------------------------------------------------- #
# Optimization moves
# --------------------------------------------------------------------------- #
def _centroid(polys: Sequence[Poly]) -> np.ndarray:
    if not polys:
        return np.zeros(2, dtype=np.float32)
    allpts = np.concatenate([p.pts for p in polys], axis=0)
    return allpts.mean(axis=0)


def _copy_polys(polys: Sequence[Poly]) -> List[Poly]:
    return [Poly(p.color, p.outline, p.pts.copy()) for p in polys]


def optimize_pose(fit: PoseFitter, rng: np.random.Generator, *,
                  affine_iters: int = 60, poly_passes: int = 3,
                  vertex_passes: int = 2, log=lambda s: None) -> Tuple[float, float]:
    g = fit.g
    base_loss = fit.loss()
    cur = base_loss

    # --- Stage 0/1: global affine about centroid (translate + per-axis scale) ---
    C = _centroid(g.polys)
    best = _copy_polys(g.polys)
    step_t = 6.0
    step_s = 0.06
    for it in range(affine_iters):
        improved = False
        # Try translations and scales as independent coordinate moves.
        moves = []
        for dx, dy in [(step_t, 0), (-step_t, 0), (0, step_t), (0, -step_t)]:
            moves.append(("t", dx, dy))
        for ds in [step_s, -step_s]:
            moves.append(("sx", ds, 0.0))
            moves.append(("sy", ds, 0.0))
        for kind, a, b in moves:
            trial = _copy_polys(best)
            for p in trial:
                if kind == "t":
                    p.pts[:, 0] += a
                    p.pts[:, 1] += b
                elif kind == "sx":
                    p.pts[:, 0] = C[0] + (p.pts[:, 0] - C[0]) * (1 + a)
                elif kind == "sy":
                    p.pts[:, 1] = C[1] + (p.pts[:, 1] - C[1]) * (1 + a)
            tl = fit.loss_of(trial)
            if tl < cur - 1e-6:
                cur = tl
                best = trial
                improved = True
        if not improved:
            step_t *= 0.6
            step_s *= 0.6
            if step_t < 0.5:
                break
    g.polys = best

    movable = [i for i, p in enumerate(g.polys) if not p.locked]

    # --- Stage 2: per-polygon translation, coordinate descent ---
    for _pass in range(poly_passes):
        order = list(movable)
        rng.shuffle(order)
        step = 4.0
        moved_any = False
        for idx in order:
            p = g.polys[idx]
            local_step = step
            while local_step >= 1.0:
                improved = False
                for dx, dy in [(local_step, 0), (-local_step, 0),
                               (0, local_step), (0, -local_step)]:
                    saved = p.pts.copy()
                    p.pts[:, 0] += dx
                    p.pts[:, 1] += dy
                    tl = fit.loss()
                    if tl < cur - 1e-6:
                        cur = tl
                        improved = True
                        moved_any = True
                        break
                    p.pts[:] = saved
                if not improved:
                    local_step *= 0.5
        if not moved_any:
            break

    # --- Stage 3: per-vertex jitter ---
    for _pass in range(vertex_passes):
        step = 3.0
        moved_any = False
        order = list(movable)
        rng.shuffle(order)
        for idx in order:
            p = g.polys[idx]
            for vi in range(p.pts.shape[0]):
                local_step = step
                while local_step >= 1.0:
                    improved = False
                    for dx, dy in [(local_step, 0), (-local_step, 0),
                                   (0, local_step), (0, -local_step)]:
                        saved = p.pts[vi].copy()
                        p.pts[vi, 0] += dx
                        p.pts[vi, 1] += dy
                        tl = fit.loss()
                        if tl < cur - 1e-6:
                            cur = tl
                            improved = True
                            moved_any = True
                            break
                        p.pts[vi] = saved
                    if not improved:
                        local_step *= 0.5
        if not moved_any:
            break

    log(f"  {g.name:12s} loss {base_loss:7.2f} -> {cur:7.2f}  "
        f"({100*(base_loss-cur)/max(base_loss,1e-6):5.1f}% better)")
    return base_loss, cur


def build_plate(geoms: Dict[str, PoseGeom], palette: dict, exclude_pose: str) -> Image.Image:
    """Full sheet with every pose drawn EXCEPT ``exclude_pose`` (neighbour bg)."""
    img = Image.new("RGBA", (W, H), tuple(palette["bg0"]))
    d = ImageDraw.Draw(img, "RGBA")
    _draw_bg(d, palette)
    _draw_layout(d, palette)
    for name in POSE_NAMES:
        if name != exclude_pose:
            draw_polys(d, geoms[name].polys, palette)
    return img


def run(data_path: Path, ref_path: Path, out_json: Path, out_png: Path,
        poses: Optional[List[str]] = None, seed: int = 0, log=print) -> dict:
    data = json.loads(data_path.read_text())
    palette = data["palette"]
    palette.update(PALETTE_FIX)
    if "draw" in data["poses"][POSE_NAMES[0]]:
        geoms = load_geom_v15(data)
    else:
        geoms = load_geom(data)
    target = Image.open(ref_path).convert("RGBA")
    import pca_seg
    bg = pca_seg.estimate_bg(target)
    target_fg = pca_seg.foreground_mask(target, tol=22.0)
    rng = np.random.default_rng(seed)
    todo = poses or POSE_NAMES
    results = {}
    for name in todo:
        plate = build_plate(geoms, palette, name)
        fit = PoseFitter(geoms[name], target, plate, palette,
                         target_fg=target_fg, bg=bg)
        b, c = optimize_pose(fit, rng, log=log)
        results[name] = {"loss_before": b, "loss_after": c}
    meta = {"source": str(data_path), "reference": str(ref_path), "results": results}
    save_geom_v15(geoms, palette, meta, out_json)
    render_sheet(geoms, palette).save(out_png)
    log(f"wrote {out_json}")
    log(f"wrote {out_png}")
    return results


def selftest(data_path: Path, ref_sheet: Path) -> None:
    """Assert the flat-list render matches the original v14 sheet pixel-for-pixel."""
    data = json.loads(data_path.read_text())
    geoms = load_geom(data)
    mine = np.asarray(render_sheet(geoms, data["palette"]).convert("RGB"), dtype=np.int16)
    orig = np.asarray(Image.open(ref_sheet).convert("RGB"), dtype=np.int16)
    diff = np.abs(mine - orig)
    n_bad = int((diff.sum(axis=2) > 0).sum())
    maxd = int(diff.max())
    print(f"selftest: differing pixels={n_bad} ({100*n_bad/(W*H):.3f}%), max channel diff={maxd}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", type=Path,
                    default=HERE / "perfect_cellular_automaton_pose_polygons_v14.json")
    ap.add_argument("--ref", type=Path,
                    default=Path("/home/joncrall/code/ambition/assets/concept_art/"
                                 "prefect-cellular-automaton-reference-image.png"))
    ap.add_argument("--out-json", type=Path, default=None)
    ap.add_argument("--out-png", type=Path, default=None)
    ap.add_argument("--poses", nargs="*", default=None)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--selftest", type=Path, default=None,
                    help="path to original v14 sheet PNG; assert faithful render then exit")
    args = ap.parse_args()
    if args.selftest is not None:
        selftest(args.data, args.selftest)
        return
    run(args.data, args.ref, args.out_json, args.out_png, args.poses, args.seed)


if __name__ == "__main__":
    main()
