#!/usr/bin/env python3
"""Robot sprite component atlas utility.

Workflow:
  1. Start with a rough YAML metadata file.
  2. Refine rough boxes by locating actual non-green foreground components.
  3. Slice refined boxes and remove the green screen to alpha.
  4. Build a contact sheet for visual QA.

This script is intentionally deterministic. It does not ask an image model to
make layout, labels, crops, or alpha channels.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Tuple

import numpy as np
import yaml
from PIL import Image, ImageDraw, ImageFont

Rect = Tuple[int, int, int, int]
Point = Tuple[int, int]


@dataclass
class Component:
    label: int
    area: int
    bbox: Rect  # local x, y, w, h
    centroid: Tuple[float, float]  # local x, y


def load_metadata(path: Path) -> Dict[str, Any]:
    text = path.read_text(encoding="utf8")
    if path.suffix.lower() in {".yaml", ".yml"}:
        data = yaml.safe_load(text)
    else:
        data = json.loads(text)
    if not isinstance(data, dict):
        raise ValueError(f"Metadata must be a mapping: {path}")
    return data


def save_metadata(data: Dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.suffix.lower() in {".yaml", ".yml"}:
        path.write_text(
            yaml.safe_dump(data, sort_keys=False, allow_unicode=True), encoding="utf8"
        )
    else:
        path.write_text(json.dumps(data, indent=2), encoding="utf8")


def resolve_image_path(meta_path: Path, meta: Mapping[str, Any]) -> Path:
    image = meta.get("image", {})
    img_path = Path(image["file"])
    if not img_path.is_absolute():
        img_path = (meta_path.parent / img_path).resolve()
    return img_path


def iter_sprites(meta: Mapping[str, Any]) -> Iterable[tuple[str, Dict[str, Any]]]:
    sprites = meta.get("sprites", {})
    if not isinstance(sprites, dict):
        raise ValueError("metadata['sprites'] must be a mapping")
    for key, value in sprites.items():
        if not isinstance(value, dict):
            raise ValueError(f"sprite {key!r} must be a mapping")
        yield key, value


def clamp_rect(rect: Rect, width: int, height: int) -> Rect:
    x, y, w, h = [int(round(v)) for v in rect]
    x0 = max(0, min(width, x))
    y0 = max(0, min(height, y))
    x1 = max(0, min(width, x + max(0, w)))
    y1 = max(0, min(height, y + max(0, h)))
    return x0, y0, max(0, x1 - x0), max(0, y1 - y0)


def expand_rect(rect: Rect, pad: int, width: int, height: int) -> Rect:
    x, y, w, h = rect
    return clamp_rect((x - pad, y - pad, w + 2 * pad, h + 2 * pad), width, height)


def rect_xyxy(rect: Rect) -> Tuple[int, int, int, int]:
    x, y, w, h = rect
    return x, y, x + w, y + h


def rect_intersection(a: Rect, b: Rect) -> Rect:
    ax0, ay0, ax1, ay1 = rect_xyxy(a)
    bx0, by0, bx1, by1 = rect_xyxy(b)
    x0 = max(ax0, bx0)
    y0 = max(ay0, by0)
    x1 = min(ax1, bx1)
    y1 = min(ay1, by1)
    return x0, y0, max(0, x1 - x0), max(0, y1 - y0)


def rect_area(rect: Rect) -> int:
    return max(0, int(rect[2])) * max(0, int(rect[3]))


def union_rect(rects: Iterable[Rect]) -> Rect:
    xs0, ys0, xs1, ys1 = [], [], [], []
    for rect in rects:
        x0, y0, x1, y1 = rect_xyxy(rect)
        xs0.append(x0)
        ys0.append(y0)
        xs1.append(x1)
        ys1.append(y1)
    if not xs0:
        return 0, 0, 0, 0
    x0, y0, x1, y1 = min(xs0), min(ys0), max(xs1), max(ys1)
    return x0, y0, x1 - x0, y1 - y0


def estimate_background_rgb(rgba: np.ndarray) -> np.ndarray:
    """Estimate green-screen color from image borders."""
    rgb = rgba[..., :3].astype(np.float32)
    border = np.concatenate(
        [rgb[0, :, :], rgb[-1, :, :], rgb[:, 0, :], rgb[:, -1, :]], axis=0
    )
    return np.median(border, axis=0)


def background_distance(rgb: np.ndarray, bg_rgb: np.ndarray) -> np.ndarray:
    diff = rgb.astype(np.float32) - bg_rgb.reshape(1, 1, 3)
    return np.sqrt(np.sum(diff * diff, axis=2))


def foreground_mask(
    rgba: np.ndarray, bg_rgb: np.ndarray, threshold: float
) -> np.ndarray:
    """Return True for pixels that are not background-like."""
    dist = background_distance(rgba[..., :3], bg_rgb)
    alpha = rgba[..., 3] > 0
    return (dist > threshold) & alpha


def greenscreen_to_alpha(
    image: Image.Image,
    bg_rgb: np.ndarray,
    transparent: float = 38.0,
    opaque: float = 92.0,
    despill: bool = True,
) -> Image.Image:
    """Remove a green-screen background using a distance ramp.

    Pixels close to the estimated background become alpha 0. Pixels far from it
    stay opaque. The ramp preserves anti-aliased edges without keeping solid
    green boxes around components.
    """
    rgba = np.array(image.convert("RGBA"), dtype=np.float32)
    dist = background_distance(rgba[..., :3], bg_rgb)
    denom = max(1.0, opaque - transparent)
    a = np.clip((dist - transparent) / denom, 0.0, 1.0) * rgba[..., 3]
    if despill:
        rgb = rgba[..., :3]
        r = rgb[..., 0]
        g = rgb[..., 1]
        b = rgb[..., 2]
        spill = (a > 0) & (g > np.maximum(r, b))
        # Pull excessive green down toward the stronger non-green channel.
        rgb[..., 1] = np.where(spill, np.maximum(r, b), g)
        rgba[..., :3] = rgb
    rgba[..., 3] = a
    return Image.fromarray(np.clip(rgba, 0, 255).astype(np.uint8), mode="RGBA")


def connected_components(mask: np.ndarray, min_area: int = 1) -> List[Component]:
    """Find connected components in a boolean mask.

    Uses OpenCV when available, then scipy as fallback. Both paths are optional
    so the package stays easy to vendor into game repos.
    """
    if mask.dtype != np.uint8:
        mask_u8 = mask.astype(np.uint8)
    else:
        mask_u8 = mask
    comps: List[Component] = []
    try:
        import cv2  # type: ignore

        nlabels, labels, stats, centroids = cv2.connectedComponentsWithStats(mask_u8, 8)
        for label in range(1, nlabels):
            x, y, w, h, area = stats[label]
            area = int(area)
            if area < min_area:
                continue
            cx, cy = centroids[label]
            comps.append(
                Component(
                    label=int(label),
                    area=area,
                    bbox=(int(x), int(y), int(w), int(h)),
                    centroid=(float(cx), float(cy)),
                )
            )
        return comps
    except Exception:
        pass
    try:
        from scipy import ndimage  # type: ignore

        labels, nlabels = ndimage.label(mask_u8)
        objs = ndimage.find_objects(labels)
        for label, obj in enumerate(objs, start=1):
            if obj is None:
                continue
            ys, xs = obj
            sub = labels[ys, xs] == label
            area = int(sub.sum())
            if area < min_area:
                continue
            yy, xx = np.nonzero(sub)
            x0, x1 = int(xs.start), int(xs.stop)
            y0, y1 = int(ys.start), int(ys.stop)
            cx = float(x0 + xx.mean())
            cy = float(y0 + yy.mean())
            comps.append(
                Component(
                    label=label,
                    area=area,
                    bbox=(x0, y0, x1 - x0, y1 - y0),
                    centroid=(cx, cy),
                )
            )
        return comps
    except Exception as ex:
        raise RuntimeError(
            "Need either opencv-python-headless or scipy for connected components"
        ) from ex


def select_components(
    comps: List[Component],
    rough_rect_global: Rect,
    search_rect_global: Rect,
    min_overlap_fraction: float,
    keep_multiple: bool,
) -> List[Component]:
    rx, ry, rw, rh = rough_rect_global
    sx, sy, _sw, _sh = search_rect_global
    rough_local = (rx - sx, ry - sy, rw, rh)
    selected: List[Component] = []
    for comp in comps:
        cx, cy = comp.centroid
        center_inside = (rough_local[0] <= cx <= rough_local[0] + rough_local[2]) and (
            rough_local[1] <= cy <= rough_local[1] + rough_local[3]
        )
        inter = rect_intersection(comp.bbox, rough_local)
        overlap_fraction = rect_area(inter) / max(1, comp.area)
        if center_inside or overlap_fraction >= min_overlap_fraction:
            selected.append(comp)
    if not keep_multiple and selected:
        # Keep the component with largest rough-overlap, with area as tie-breaker.
        def score(comp: Component) -> Tuple[float, int]:
            inter = rect_intersection(comp.bbox, rough_local)
            return (rect_area(inter) / max(1, comp.area), comp.area)

        selected = [max(selected, key=score)]
    return selected


def bbox_from_mask(mask: np.ndarray) -> Optional[Rect]:
    ys, xs = np.nonzero(mask)
    if len(xs) == 0:
        return None
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    y0, y1 = int(ys.min()), int(ys.max()) + 1
    return x0, y0, x1 - x0, y1 - y0


def adjust_point(point: List[int], old_rect: Rect, new_rect: Rect) -> List[int]:
    old_x, old_y, _, _ = old_rect
    new_x, new_y, _, _ = new_rect
    return [int(round(point[0] + old_x - new_x)), int(round(point[1] + old_y - new_y))]


def validate_metadata(meta_path: Path) -> int:
    meta = load_metadata(meta_path)
    img_path = resolve_image_path(meta_path, meta)
    if not img_path.exists():
        raise FileNotFoundError(f"Image not found: {img_path}")
    with Image.open(img_path) as im:
        width, height = im.size
    expected = tuple(meta.get("image", {}).get("size", []))
    if expected and expected != (width, height):
        print(f"WARN image size metadata {expected} != actual {(width, height)}")

    errors = 0
    warnings = 0
    for sid, sprite in iter_sprites(meta):
        rect = sprite.get("rect")
        pivot = sprite.get("pivot")
        if not rect or len(rect) != 4:
            print(f"ERROR {sid}: missing/invalid rect")
            errors += 1
            continue
        x, y, w, h = [int(v) for v in rect]
        if x < 0 or y < 0 or w <= 0 or h <= 0 or x + w > width or y + h > height:
            print(f"ERROR {sid}: rect out of image bounds: {rect}")
            errors += 1
        if not pivot or len(pivot) != 2:
            print(f"ERROR {sid}: missing/invalid pivot")
            errors += 1
        else:
            px, py = pivot
            if not (0 <= px <= w and 0 <= py <= h):
                print(f"WARN {sid}: pivot outside crop: pivot={pivot} rect={rect}")
                warnings += 1
        for aname, point in sprite.get("anchors", {}).items():
            if len(point) != 2:
                print(f"ERROR {sid}.{aname}: invalid anchor {point}")
                errors += 1
            else:
                ax, ay = point
                if not (0 <= ax <= w and 0 <= ay <= h):
                    print(
                        f"WARN {sid}.{aname}: anchor outside crop: {point} rect={rect}"
                    )
                    warnings += 1
    total = len(meta.get("sprites", {}))
    if errors:
        print(
            f"Validation failed with {errors} error(s), {warnings} warning(s), across {total} sprites."
        )
        return 1
    print(
        f"Validation ok: {total} sprites, image={img_path.name}, size={(width, height)}, warnings={warnings}"
    )
    return 0


def refine_metadata(
    meta_path: Path, out_path: Path, report_path: Optional[Path] = None
) -> int:
    meta = load_metadata(meta_path)
    img_path = resolve_image_path(meta_path, meta)
    with Image.open(img_path) as im:
        im = im.convert("RGBA")
        rgba = np.array(im)
    height, width = rgba.shape[:2]
    bg_rgb = estimate_background_rgb(rgba)
    refine_cfg = meta.get("refinement", {}) or {}
    default_search_padding = int(refine_cfg.get("default_search_padding", 28))
    default_output_padding = int(refine_cfg.get("output_padding", 4))
    threshold = float(refine_cfg.get("background_distance_transparent", 38))
    min_overlap_fraction_default = float(
        refine_cfg.get("component_selection", {}).get("min_overlap_fraction", 0.18)
    )
    keep_multiple_default = bool(
        refine_cfg.get("component_selection", {}).get("keep_multiple_components", True)
    )
    min_area_default = int(
        refine_cfg.get("component_selection", {}).get("min_component_area", 18)
    )
    clamp_points_to_crop = bool(refine_cfg.get("clamp_points_to_crop", True))

    full_mask = foreground_mask(rgba, bg_rgb, threshold)
    out_meta = dict(meta)
    out_meta["version"] = str(out_meta.get("version", "0.2.0")) + "+refined"
    out_meta["image"] = dict(out_meta.get("image", {}))
    out_meta["image"]["estimated_background_rgb"] = [
        int(round(v)) for v in bg_rgb.tolist()
    ]
    out_meta["metadata_quality"] = dict(out_meta.get("metadata_quality", {}))
    out_meta["metadata_quality"]["rect_precision"] = (
        "programmatically_refined_from_rough_yaml"
    )
    out_meta["metadata_quality"]["refinement_source"] = str(meta_path.name)
    out_meta["sprites"] = {}
    report: Dict[str, Any] = {
        "source_metadata": str(meta_path),
        "source_image": str(img_path),
        "estimated_background_rgb": [float(v) for v in bg_rgb.tolist()],
        "sprites": {},
    }

    for sid, sprite in iter_sprites(meta):
        old_rect = tuple(int(v) for v in sprite.get("rect", [0, 0, 0, 0]))  # type: ignore[assignment]
        old_rect = clamp_rect(old_rect, width, height)
        sp_refine = dict(sprite.get("refine", {}) or {})
        search_padding = int(sp_refine.get("search_padding", default_search_padding))
        output_padding = int(sp_refine.get("output_padding", default_output_padding))
        min_area = int(sp_refine.get("min_component_area", min_area_default))
        min_overlap_fraction = float(
            sp_refine.get("min_overlap_fraction", min_overlap_fraction_default)
        )
        keep_multiple = bool(
            sp_refine.get("keep_multiple_components", keep_multiple_default)
        )

        search_rect = expand_rect(old_rect, search_padding, width, height)
        sx, sy, sw, sh = search_rect
        search_mask = full_mask[sy : sy + sh, sx : sx + sw]
        comps = connected_components(search_mask, min_area=min_area)
        selected = select_components(
            comps, old_rect, search_rect, min_overlap_fraction, keep_multiple
        )
        warnings: List[str] = []
        if not selected:
            rx, ry, rw, rh = old_rect
            rough_local_mask = full_mask[ry : ry + rh, rx : rx + rw]
            fallback = bbox_from_mask(rough_local_mask)
            if fallback is None:
                new_rect = old_rect
                warnings.append("no foreground found; kept rough rect")
            else:
                fx, fy, fw, fh = fallback
                new_rect = clamp_rect((rx + fx, ry + fy, fw, fh), width, height)
                warnings.append(
                    "no selected components in search window; used foreground bbox inside rough rect"
                )
        else:
            local_union = union_rect([c.bbox for c in selected])
            lx, ly, lw, lh = local_union
            new_rect = clamp_rect((sx + lx, sy + ly, lw, lh), width, height)

        new_rect = expand_rect(new_rect, output_padding, width, height)
        # Diagnostics: selected foreground should not hit search window boundary.
        if selected:
            for comp in selected:
                x, y, w, h = comp.bbox
                if x <= 0 or y <= 0 or x + w >= sw or y + h >= sh:
                    warnings.append(
                        "selected foreground touches search boundary; increase search_padding"
                    )
                    break
        refined_sprite = dict(sprite)
        refined_sprite["rough_rect"] = list(old_rect)
        refined_sprite["rect"] = [int(v) for v in new_rect]

        def maybe_clamp_point(pt: List[int]) -> List[int]:
            if not clamp_points_to_crop:
                return pt
            return [
                max(0, min(int(new_rect[2] - 1), int(pt[0]))),
                max(0, min(int(new_rect[3] - 1), int(pt[1]))),
            ]

        if "pivot" in sprite and len(sprite["pivot"]) == 2:
            refined_sprite["pivot"] = maybe_clamp_point(
                adjust_point(list(sprite["pivot"]), old_rect, new_rect)
            )
        if "anchors" in sprite:
            anchors = {}
            for aname, point in sprite.get("anchors", {}).items():
                if isinstance(point, list) and len(point) == 2:
                    anchors[aname] = maybe_clamp_point(
                        adjust_point(point, old_rect, new_rect)
                    )
                else:
                    anchors[aname] = point
            refined_sprite["anchors"] = anchors
        refined_sprite["refined"] = {
            "method": "greenscreen_connected_components",
            "search_rect": list(search_rect),
            "selected_component_count": len(selected),
            "selected_area_px": int(sum(c.area for c in selected)),
            "output_padding": output_padding,
            "warnings": warnings,
        }
        out_meta["sprites"][sid] = refined_sprite
        report["sprites"][sid] = refined_sprite["refined"] | {
            "rough_rect": list(old_rect),
            "rect": list(new_rect),
        }

    save_metadata(out_meta, out_path)
    if report_path:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2), encoding="utf8")
    print(f"Refined {len(out_meta['sprites'])} sprites")
    print(f"Input : {meta_path}")
    print(f"Output: {out_path}")
    if report_path:
        print(f"Report: {report_path}")
    return 0


def slice_components(meta_path: Path, out_dir: Path, *, remove_bg: bool = True) -> None:
    meta = load_metadata(meta_path)
    img_path = resolve_image_path(meta_path, meta)
    out_dir.mkdir(parents=True, exist_ok=True)
    with Image.open(img_path) as source_image:
        source_image = source_image.convert("RGBA")
        rgba = np.array(source_image)
        bg_rgb = np.array(
            meta.get("image", {}).get("estimated_background_rgb", []), dtype=np.float32
        )
        if bg_rgb.size != 3:
            bg_rgb = estimate_background_rgb(rgba)
        refine_cfg = meta.get("refinement", {}) or {}
        transparent = float(refine_cfg.get("background_distance_transparent", 38))
        opaque = float(refine_cfg.get("background_distance_opaque", 92))
        despill = bool(refine_cfg.get("despill_green", True))
        index: Dict[str, Any] = {}
        for sid, sprite in iter_sprites(meta):
            rect = tuple(int(v) for v in sprite["rect"])
            x, y, w, h = clamp_rect(rect, source_image.width, source_image.height)
            crop = source_image.crop((x, y, x + w, y + h))
            if remove_bg:
                crop = greenscreen_to_alpha(
                    crop,
                    bg_rgb,
                    transparent=transparent,
                    opaque=opaque,
                    despill=despill,
                )
            out_path = out_dir / f"{sid}.png"
            crop.save(out_path)
            index[sid] = {
                "file": out_path.name,
                "source_rect": [x, y, w, h],
                "rough_rect": sprite.get("rough_rect"),
                "pivot": sprite.get("pivot"),
                "anchors": sprite.get("anchors", {}),
                "tags": sprite.get("tags", []),
                "refined": sprite.get("refined", {}),
            }
    (out_dir / "slices.index.json").write_text(
        json.dumps({"source_metadata": str(meta_path), "sprites": index}, indent=2),
        encoding="utf8",
    )
    print(f"Wrote {len(index)} slices to {out_dir}")


def list_sprites(meta_path: Path) -> None:
    meta = load_metadata(meta_path)
    for sid, sprite in iter_sprites(meta):
        tags = ",".join(sprite.get("tags", []))
        rough = sprite.get("rough_rect")
        print(f"{sid:24s} rect={sprite.get('rect')} rough={rough} tags=[{tags}]")


def make_checkerboard(size: Tuple[int, int], cell: int = 12) -> Image.Image:
    w, h = size
    arr = np.zeros((h, w, 4), dtype=np.uint8)
    for y in range(h):
        for x in range(w):
            v = 235 if ((x // cell) + (y // cell)) % 2 == 0 else 210
            arr[y, x] = (v, v, v, 255)
    return Image.fromarray(arr, mode="RGBA")


def contact_sheet(
    src_dir: Path,
    out_path: Path,
    cell_w: int = 180,
    cell_h: int = 160,
    columns: int = 5,
) -> None:
    files = sorted(
        p for p in src_dir.glob("*.png") if p.name not in {"contact_sheet.png"}
    )
    files = [p for p in files if p.name != "slices.index.json"]
    if not files:
        raise FileNotFoundError(f"No PNG files found in {src_dir}")
    rows = int(math.ceil(len(files) / columns))
    sheet = Image.new("RGBA", (columns * cell_w, rows * cell_h), (255, 255, 255, 255))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("DejaVuSans.ttf", 12)
    except OSError:
        font = ImageFont.load_default()
    for idx, path in enumerate(files):
        col = idx % columns
        row = idx // columns
        x0 = col * cell_w
        y0 = row * cell_h
        draw.rectangle(
            [x0, y0, x0 + cell_w - 1, y0 + cell_h - 1], outline=(200, 200, 200, 255)
        )
        with Image.open(path) as im:
            im = im.convert("RGBA")
            max_w = cell_w - 20
            max_h = cell_h - 34
            scale = min(max_w / im.width, max_h / im.height, 1.0)
            if scale < 1.0:
                im = im.resize(
                    (max(1, int(im.width * scale)), max(1, int(im.height * scale))),
                    Image.Resampling.LANCZOS,
                )
            bg = make_checkerboard((cell_w - 12, cell_h - 32), cell=10)
            sheet.alpha_composite(bg, (x0 + 6, y0 + 6))
            px = x0 + (cell_w - im.width) // 2
            py = y0 + 8
            sheet.alpha_composite(im, (px, py))
        draw.text((x0 + 6, y0 + cell_h - 22), path.stem, fill=(0, 0, 0, 255), font=font)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.convert("RGB").save(out_path)
    print(f"Wrote contact sheet to {out_path}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_validate = sub.add_parser(
        "validate", help="validate metadata and source image bounds"
    )
    p_validate.add_argument("metadata", type=Path)

    p_refine = sub.add_parser(
        "refine", help="refine rough YAML boxes using green-screen foreground extents"
    )
    p_refine.add_argument("metadata", type=Path)
    p_refine.add_argument(
        "--out", type=Path, default=Path("metadata/robot_components.refined.yaml")
    )
    p_refine.add_argument(
        "--report", type=Path, default=Path("output/refinement_report.json")
    )

    p_slice = sub.add_parser("slice", help="slice components from refined metadata")
    p_slice.add_argument("metadata", type=Path)
    p_slice.add_argument("--out", type=Path, default=Path("output/slices"))
    p_slice.add_argument(
        "--keep-background", action="store_true", help="do not remove the green screen"
    )

    p_build = sub.add_parser(
        "build", help="refine, slice, and generate contact sheet in one command"
    )
    p_build.add_argument("metadata", type=Path)
    p_build.add_argument(
        "--refined", type=Path, default=Path("metadata/robot_components.refined.yaml")
    )
    p_build.add_argument("--slices", type=Path, default=Path("output/slices"))
    p_build.add_argument(
        "--contact", type=Path, default=Path("output/contact_sheet.png")
    )

    p_list = sub.add_parser("list", help="list sprite ids")
    p_list.add_argument("metadata", type=Path)

    p_contact = sub.add_parser(
        "contact-sheet", help="make a contact sheet from sliced PNGs"
    )
    p_contact.add_argument("src_dir", type=Path)
    p_contact.add_argument("--out", type=Path, default=Path("output/contact_sheet.png"))
    p_contact.add_argument("--cell-width", type=int, default=180)
    p_contact.add_argument("--cell-height", type=int, default=160)
    p_contact.add_argument("--columns", type=int, default=5)
    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.cmd == "validate":
        return validate_metadata(args.metadata)
    if args.cmd == "refine":
        return refine_metadata(args.metadata, args.out, args.report)
    if args.cmd == "slice":
        slice_components(args.metadata, args.out, remove_bg=not args.keep_background)
        return 0
    if args.cmd == "build":
        refine_metadata(
            args.metadata, args.refined, Path("output/refinement_report.json")
        )
        slice_components(args.refined, args.slices, remove_bg=True)
        contact_sheet(args.slices, args.contact)
        return 0
    if args.cmd == "list":
        list_sprites(args.metadata)
        return 0
    if args.cmd == "contact-sheet":
        contact_sheet(
            args.src_dir,
            args.out,
            cell_w=args.cell_width,
            cell_h=args.cell_height,
            columns=args.columns,
        )
        return 0
    parser.error(f"Unknown command: {args.cmd}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
