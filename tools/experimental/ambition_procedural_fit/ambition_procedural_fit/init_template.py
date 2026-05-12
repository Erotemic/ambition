from __future__ import annotations

import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter, ImageOps

from .schema import CanvasSpec, FitSpec, PrimitiveSpec, write_spec


def _load_rgb(path: str | Path, size: int) -> np.ndarray:
    img = Image.open(path).convert("RGB")
    img = ImageOps.exif_transpose(img)
    img = ImageOps.contain(img, (size, size), method=Image.Resampling.LANCZOS)
    canvas = Image.new("RGB", (size, size), (16, 17, 20))
    canvas.paste(img, ((size - img.width) // 2, (size - img.height) // 2))
    return np.asarray(canvas, dtype=np.float32) / 255.0


def _median_color(arr: np.ndarray) -> list[float]:
    return [float(v) for v in np.median(arr.reshape(-1, 3), axis=0)] + [1.0]


def _cell_candidates(arr: np.ndarray, count: int) -> list[tuple[int, int, int, int, float, np.ndarray]]:
    h, w = arr.shape[:2]
    aspect = w / max(1, h)
    cols = max(3, int(math.sqrt(count * 2 * aspect)))
    rows = max(3, int(math.ceil(count * 2 / cols)))
    bg = np.median(arr.reshape(-1, 3), axis=0)
    cands = []
    for r in range(rows):
        y0 = int(round(r * h / rows))
        y1 = int(round((r + 1) * h / rows))
        for c in range(cols):
            x0 = int(round(c * w / cols))
            x1 = int(round((c + 1) * w / cols))
            cell = arr[y0:y1, x0:x1]
            if cell.size == 0:
                continue
            mean = cell.reshape(-1, 3).mean(axis=0)
            std = cell.reshape(-1, 3).std(axis=0).mean()
            contrast = float(np.linalg.norm(mean - bg) + std * 0.6)
            cands.append((x0, y0, x1, y1, contrast, mean))
    cands.sort(key=lambda item: item[4], reverse=True)
    return cands


def init_template_from_image(
    target_path: str | Path,
    out_path: str | Path,
    *,
    size: int = 192,
    rects: int = 32,
    ellipses: int = 8,
    superellipses: int = 0,
    segments: int = 10,
) -> FitSpec:
    """Create a first-pass trainable template from a target crop.

    The initial template is intentionally simple: colored rectangles covering
    high-contrast cells, a few soft ellipses for blob-like regions, trainable
    superellipses for geometric-but-morphable regions, and edge segments seeded
    from a blurred edge map. This is only a starting point for gradient descent,
    not a final vectorization algorithm.
    """
    arr = _load_rgb(target_path, size)
    h, w = arr.shape[:2]
    bg = _median_color(arr)
    cands = _cell_candidates(arr, max(1, rects + superellipses * 2))
    primitives: list[PrimitiveSpec] = []

    for idx, (x0, y0, x1, y1, _score, mean) in enumerate(cands[:rects]):
        cx = ((x0 + x1) * 0.5) / w
        cy = ((y0 + y1) * 0.5) / h
        ww = max(0.02, (x1 - x0) / w * 1.05)
        hh = max(0.02, (y1 - y0) / h * 1.05)
        color = [float(v) for v in mean] + [0.56]
        primitives.append(
            PrimitiveSpec(
                kind="rect",
                name=f"rect_seed_{idx:03d}",
                params={"xy": [cx, cy], "wh": [ww, hh], "angle": 0.0, "color": color},
                train=["xy", "wh", "angle", "color"],
            )
        )

    used_superellipse: list[tuple[float, float]] = []
    for idx, (x0, y0, x1, y1, _score, mean) in enumerate(cands[: max(superellipses * 5, superellipses)]):
        if len(used_superellipse) >= superellipses:
            break
        cx = ((x0 + x1) * 0.5) / w
        cy = ((y0 + y1) * 0.5) / h
        if any((cx - ux) ** 2 + (cy - uy) ** 2 < 0.008 for ux, uy in used_superellipse):
            continue
        used_superellipse.append((cx, cy))
        ww = max(0.03, (x1 - x0) / w * 0.9)
        hh = max(0.03, (y1 - y0) / h * 0.9)
        exponent = 4.0 if ww / max(hh, 1e-6) < 1.6 and hh / max(ww, 1e-6) < 1.6 else 6.0
        color = [float(v) for v in mean] + [0.58]
        primitives.append(
            PrimitiveSpec(
                kind="superellipse",
                name=f"superellipse_seed_{len(used_superellipse) - 1:03d}",
                params={"xy": [cx, cy], "wh": [ww, hh], "angle": 0.0, "exponent": exponent, "color": color},
                train=["xy", "wh", "angle", "exponent", "color"],
            )
        )

    # Seed ellipses from the most saturated candidate cells.
    sat = arr.max(axis=2) - arr.min(axis=2)
    flat = sat.reshape(-1)
    if flat.size:
        quant = np.argpartition(flat, -min(flat.size, max(1, ellipses * 12)))[-min(flat.size, max(1, ellipses * 12)): ]
        ys, xs = np.unravel_index(quant, sat.shape)
        order = np.argsort(flat[quant])[::-1]
        used: list[tuple[float, float]] = []
        for idx in order:
            if len(used) >= ellipses:
                break
            x = float(xs[idx] / w)
            y = float(ys[idx] / h)
            if any((x - ux) ** 2 + (y - uy) ** 2 < 0.012 for ux, uy in used):
                continue
            used.append((x, y))
            color = [float(v) for v in arr[ys[idx], xs[idx]]] + [0.42]
            primitives.append(
                PrimitiveSpec(
                    kind="ellipse",
                    name=f"ellipse_seed_{len(used) - 1:03d}",
                    params={"xy": [x, y], "wh": [0.16, 0.10], "angle": 0.0, "color": color},
                    train=["xy", "wh", "angle", "color"],
                )
            )

    # Very lightweight edge line seeds using PIL filters to avoid optional CV deps.
    edge_img = Image.fromarray((arr * 255).astype(np.uint8), mode="RGB").convert("L")
    edge_img = edge_img.filter(ImageFilter.FIND_EDGES).filter(ImageFilter.GaussianBlur(radius=1.0))
    edge = np.asarray(edge_img, dtype=np.float32) / 255.0
    flat_edge = edge.reshape(-1)
    if flat_edge.size:
        quant = np.argpartition(flat_edge, -min(flat_edge.size, max(1, segments * 20)))[-min(flat_edge.size, max(1, segments * 20)): ]
        ys, xs = np.unravel_index(quant, edge.shape)
        order = np.argsort(flat_edge[quant])[::-1]
        used = []
        for idx in order:
            if len(used) >= segments:
                break
            x = float(xs[idx] / w)
            y = float(ys[idx] / h)
            if any((x - ux) ** 2 + (y - uy) ** 2 < 0.01 for ux, uy in used):
                continue
            used.append((x, y))
            length = 0.14
            angle = 0.0 if len(used) % 2 == 0 else math.pi / 2
            dx = math.cos(angle) * length * 0.5
            dy = math.sin(angle) * length * 0.5
            color = [float(v) for v in arr[ys[idx], xs[idx]]] + [0.72]
            primitives.append(
                PrimitiveSpec(
                    kind="segment",
                    name=f"edge_seed_{len(used) - 1:03d}",
                    params={
                        "p0": [min(0.98, max(0.02, x - dx)), min(0.98, max(0.02, y - dy))],
                        "p1": [min(0.98, max(0.02, x + dx)), min(0.98, max(0.02, y + dy))],
                        "width": 0.012,
                        "color": color,
                    },
                    train=["p0", "p1", "width", "color"],
                )
            )

    spec = FitSpec(
        canvas=CanvasSpec(width=size, height=size, background=bg),
        loss={"weights": {"rgb": 1.0, "pyramid": 0.45, "edge": 0.16, "detail": 0.0, "color_stats": 0.05}},
        metadata={
            "description": "Auto-seeded soft primitive template for ambition_procedural_fit.",
            "target_image": str(target_path),
        },
        primitives=primitives,
    )
    write_spec(spec, out_path)
    return spec
