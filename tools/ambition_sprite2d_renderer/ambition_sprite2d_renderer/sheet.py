from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFont

from .adapters import get_adapter
from .config import CharacterJob


def _parse_bg(value: str):
    if str(value).lower() == "transparent":
        return (0, 0, 0, 0)
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, 255)


def _font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def _measure_body_extent(frame: Image.Image) -> Dict[str, Any] | None:
    """Compute the bounding box of opaque pixels in one frame, plus the
    derived feet/center anchor in Bevy-anchor convention.

    Bevy anchors are normalized in `[-0.5, +0.5]` with `0` at the sprite
    centre and `+0.5` at the top edge. Rust callers want the anchor of
    the rendered character's feet so that `transform.y` ≈ the bottom of
    the collision box. We compute that here so the runtime doesn't need
    to hand-tune `feet_anchor_y` per target.
    """
    bbox = frame.getbbox()
    if bbox is None:
        return None
    fw, fh = frame.size
    x_min, y_min, x_max, y_max = bbox
    # `getbbox` is half-open on the high side; subtract 1 for an inclusive
    # last row so the feet anchor sits on the last opaque pixel.
    feet_y = y_max - 1
    feet_x = (x_min + x_max - 1) / 2.0
    body_w = x_max - x_min
    body_h = y_max - y_min
    return {
        "frame_width": fw,
        "frame_height": fh,
        "body_pixel_bbox": {
            "x": int(x_min),
            "y": int(y_min),
            "w": int(body_w),
            "h": int(body_h),
        },
        "feet_pixel": {"x": float(feet_x), "y": float(feet_y)},
        # Bevy anchor convention: (0,0) = center, +0.5y = top edge.
        # Image-y grows downward; image_y=feet_y maps to anchor_y =
        # 0.5 - feet_y / fh. Rust uses this directly as `feet_anchor_y`.
        "feet_anchor_norm": {
            "x": float(feet_x / fw - 0.5),
            "y": float(0.5 - feet_y / fh),
        },
    }


# Pixels of safety padding kept around the union bbox before cropping. Anti-
# aliased character edges are only slightly transparent, so without a small
# pad bilinear sampling could clip them. Two pixels is enough at the current
# 128px source frames.
_CROP_PADDING = 2


def build_spritesheet(job: CharacterJob) -> Tuple[Image.Image, Dict[str, Any]]:
    """Render every frame at the configured canvas size, then crop the entire
    sheet to the *union* of all opaque-pixel bboxes across every frame.

    Uniform per-sheet cropping (rather than per-frame) keeps the character
    anchored consistently across animations: a wide-arm spike_halo pose and a
    compact rest pose share the same pixel-space frame so the runtime can
    place sprites at a single fixed anchor without compensating for shifting
    bbox origins. The downside is that the crop only saves the margin that
    *every* animation can spare; tall jumps and wide attacks pull the union
    out toward the original canvas size.

    The returned manifest carries the cropped frame dimensions in the
    standard `frame_width`/`frame_height` fields, plus a `crop` block that
    records the original canvas size and crop offset for debugging or for
    runtime loaders that need the unpadded dimensions.
    """
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    animations = adapter.animations()
    selected = [a for a in job.animations if a in animations]
    missing = [a for a in job.animations if a not in animations]
    if missing:
        raise KeyError(f"unsupported animations for {job.target}: {missing}; available={sorted(animations)}")
    src_fw, src_fh = job.render.frame_width, job.render.frame_height
    label_w = max(0, job.render.label_width)
    border = max(0, job.render.border)
    max_frames = max(animations[a]["frames"] for a in selected)

    # Pass 1: render every frame at full canvas size and accumulate the
    # union of opaque-pixel bboxes.
    rendered: List[List[Image.Image]] = []
    union_min_x, union_min_y = src_fw, src_fh
    union_max_x, union_max_y = 0, 0
    any_visible = False
    for animation in selected:
        info = animations[animation]
        row_frames: List[Image.Image] = []
        for frame_index in range(info["frames"]):
            frame = adapter.render_frame(spec, animation, frame_index, (src_fw, src_fh), job)
            row_frames.append(frame)
            bbox = frame.getbbox()
            if bbox is not None:
                any_visible = True
                x_min, y_min, x_max, y_max = bbox
                union_min_x = min(union_min_x, x_min)
                union_min_y = min(union_min_y, y_min)
                union_max_x = max(union_max_x, x_max)
                union_max_y = max(union_max_y, y_max)
        rendered.append(row_frames)

    if any_visible:
        crop_min_x = max(0, union_min_x - _CROP_PADDING)
        crop_min_y = max(0, union_min_y - _CROP_PADDING)
        crop_max_x = min(src_fw, union_max_x + _CROP_PADDING)
        crop_max_y = min(src_fh, union_max_y + _CROP_PADDING)
    else:
        # Defensive fallback: completely transparent input keeps the original
        # canvas size so downstream code never sees a zero-sized frame.
        crop_min_x, crop_min_y = 0, 0
        crop_max_x, crop_max_y = src_fw, src_fh
    fw = crop_max_x - crop_min_x
    fh = crop_max_y - crop_min_y

    # Pass 2: compose the sheet with cropped frames.
    sheet_w = label_w + max_frames * (fw + border) + border
    sheet_h = len(selected) * (fh + border) + border
    sheet = Image.new("RGBA", (sheet_w, sheet_h), _parse_bg(job.render.sheet_background))
    draw = ImageDraw.Draw(sheet)
    font = _font(12)
    manifest: Dict[str, Any] = {
        "target": job.target,
        "seed": job.seed,
        "archetype": job.archetype,
        "held_item": job.held_item,
        "frame_width": fw,
        "frame_height": fh,
        "label_width": label_w,
        "border": border,
        "spec": adapter.spec_dict(spec),
        "crop": {
            "source_frame_width": src_fw,
            "source_frame_height": src_fh,
            "offset": {"x": int(crop_min_x), "y": int(crop_min_y)},
            "padding_px": _CROP_PADDING,
        },
        "animations": {},
    }
    body_metric_frame: Image.Image | None = None
    for row_idx, animation in enumerate(selected):
        info = animations[animation]
        y = border + row_idx * (fh + border)
        if label_w:
            draw.text((8, y + 8), animation, fill=(255, 255, 255, 255), font=font)
            draw.text((8, y + 23), f"{info['frames']}f/{info['duration_ms']}ms", fill=(190, 190, 190, 255), font=_font(10))
        frame_records: List[Dict[str, Any]] = []
        for frame_index, src_frame in enumerate(rendered[row_idx]):
            cropped = src_frame.crop((crop_min_x, crop_min_y, crop_max_x, crop_max_y))
            x = label_w + border + frame_index * (fw + border)
            sheet.alpha_composite(cropped, (x, y))
            frame_records.append({
                "index": frame_index,
                "x": x,
                "y": y,
                "w": fw,
                "h": fh,
                "duration_ms": info["duration_ms"],
            })
            # Use the first frame of the first emitted animation as the
            # canonical reference pose for body-extent measurement. Idle/Rest
            # is what the gameplay code shows when the entity is at rest, so
            # its bbox is the most representative — and it's already in
            # cropped-frame pixel coordinates.
            if body_metric_frame is None:
                body_metric_frame = cropped
        manifest["animations"][animation] = {"frames": frame_records, "duration_ms": info["duration_ms"]}
    metrics = _measure_body_extent(body_metric_frame) if body_metric_frame is not None else None
    if metrics is not None:
        manifest["body_metrics"] = metrics
    return sheet, manifest


def write_spritesheet(job: CharacterJob, image_out: str | Path, manifest_out: str | Path | None = None) -> Tuple[Path, Path]:
    image_out = Path(image_out)
    if manifest_out is None:
        manifest_out = image_out.with_suffix(".yaml")
    manifest_out = Path(manifest_out)
    image_out.parent.mkdir(parents=True, exist_ok=True)
    manifest_out.parent.mkdir(parents=True, exist_ok=True)
    sheet, manifest = build_spritesheet(job)
    sheet.save(image_out)
    with open(manifest_out, "w", encoding="utf8") as file:
        yaml.safe_dump(manifest, file, sort_keys=False)
    return image_out, manifest_out
