from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw

from .adapters import get_adapter
from .actor_contract import write_actor_contract_for_adapter
from .config import CharacterJob
from .rendering import load_font


def _parse_bg(value: str):
    if str(value).lower() == "transparent":
        return (0, 0, 0, 0)
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, 255)


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
_DEFAULT_CROP_PADDING = 2


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
        raise KeyError(
            f"unsupported animations for {job.target}: {missing}; available={sorted(animations)}"
        )
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
            frame = adapter.render_frame(
                spec, animation, frame_index, (src_fw, src_fh), job
            )
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

    crop_padding = max(
        0, int(getattr(job.render, "crop_padding", _DEFAULT_CROP_PADDING))
    )
    if not getattr(job.render, "crop", True):
        crop_min_x, crop_min_y = 0, 0
        crop_max_x, crop_max_y = src_fw, src_fh
    elif any_visible:
        crop_min_x = max(0, union_min_x - crop_padding)
        crop_min_y = max(0, union_min_y - crop_padding)
        crop_max_x = min(src_fw, union_max_x + crop_padding)
        crop_max_y = min(src_fh, union_max_y + crop_padding)
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
    sheet = Image.new(
        "RGBA", (sheet_w, sheet_h), _parse_bg(job.render.sheet_background)
    )
    draw = ImageDraw.Draw(sheet)
    font = load_font(12)
    manifest: Dict[str, Any] = {
        "target": job.target,
        "name": job.name,
        "output_name": getattr(job, "output_name", None),
        "seed": job.seed,
        "archetype": job.archetype,
        "variant": job.variant,
        "held_item": job.held_item,
        "faction": job.faction,
        "role": job.role,
        "music_cue": job.music_cue,
        "tags": list(job.tags),
        "sheet_tuning": dict(job.sheet_tuning)
        if getattr(job, "sheet_tuning", None) is not None
        else None,
        "frame_width": fw,
        "frame_height": fh,
        "label_width": label_w,
        "border": border,
        "spec": adapter.spec_dict(spec),
        "crop": {
            "source_frame_width": src_fw,
            "source_frame_height": src_fh,
            "offset": {"x": int(crop_min_x), "y": int(crop_min_y)},
            "enabled": bool(getattr(job.render, "crop", True)),
            "padding_px": crop_padding,
        },
        "animations": {},
    }
    body_metric_frame: Image.Image | None = None
    # Per-animation union alpha bboxes in **source canvas** coords
    # (before the sheet-wide crop). Becomes per-animation hurtboxes
    # so the gameplay can use the right hurtbox extent for the
    # active animation — important when arms or other extensions
    # appear only during attack frames.
    anim_union_bbox_src: Dict[str, Tuple[int, int, int, int] | None] = {}
    for row_idx, animation in enumerate(selected):
        info = animations[animation]
        y = border + row_idx * (fh + border)
        if label_w:
            draw.text((8, y + 8), animation, fill=(255, 255, 255, 255), font=font)
            draw.text(
                (8, y + 23),
                f"{info['frames']}f/{info['duration_ms']}ms",
                fill=(190, 190, 190, 255),
                font=load_font(10),
            )
        frame_records: List[Dict[str, Any]] = []
        anim_bbox: Tuple[int, int, int, int] | None = None
        for frame_index, src_frame in enumerate(rendered[row_idx]):
            cropped = src_frame.crop((crop_min_x, crop_min_y, crop_max_x, crop_max_y))
            x = label_w + border + frame_index * (fw + border)
            sheet.alpha_composite(cropped, (x, y))
            frame_records.append(
                {
                    "index": frame_index,
                    "x": x,
                    "y": y,
                    "w": fw,
                    "h": fh,
                    "duration_ms": info["duration_ms"],
                }
            )
            # Per-animation alpha-bbox accumulator (source canvas coords).
            src_bbox = src_frame.getbbox()
            if src_bbox is not None:
                if anim_bbox is None:
                    anim_bbox = src_bbox
                else:
                    anim_bbox = (
                        min(anim_bbox[0], src_bbox[0]),
                        min(anim_bbox[1], src_bbox[1]),
                        max(anim_bbox[2], src_bbox[2]),
                        max(anim_bbox[3], src_bbox[3]),
                    )
            # Use the first frame of the first emitted animation as the
            # canonical reference pose for body-extent measurement. Idle/Rest
            # is what the gameplay code shows when the entity is at rest, so
            # its bbox is the most representative — and it's already in
            # cropped-frame pixel coordinates.
            if body_metric_frame is None:
                body_metric_frame = cropped
        anim_union_bbox_src[animation] = anim_bbox
        manifest["animations"][animation] = {
            "frames": frame_records,
            "duration_ms": info["duration_ms"],
        }
    metrics = (
        _measure_body_extent(body_metric_frame)
        if body_metric_frame is not None
        else None
    )
    if metrics is not None:
        # Per-animation hurtbox: each animation's alpha-bbox in
        # cropped-frame coords (subtract the sheet crop offset).
        # Per-animation hitbox: adapter-declared rects, also
        # translated to cropped-frame coords. Together they give
        # the gameplay layer a clean per-animation
        # {hurtbox, hitbox} pair for each row in the sheet.
        anim_metrics: Dict[str, Dict[str, Any]] = {}
        for animation, src_bbox in anim_union_bbox_src.items():
            entry: Dict[str, Any] = {}
            if src_bbox is not None:
                x0, y0, x1, y1 = src_bbox
                # Translate from source canvas → cropped frame.
                cx0 = max(0, x0 - crop_min_x)
                cy0 = max(0, y0 - crop_min_y)
                cw = min(fw, x1 - crop_min_x) - cx0
                ch = min(fh, y1 - crop_min_y) - cy0
                if cw > 0 and ch > 0:
                    entry["hurtbox"] = {
                        "bbox": {
                            "x": int(cx0),
                            "y": int(cy0),
                            "w": int(cw),
                            "h": int(ch),
                        }
                    }
            anim_metrics[animation] = entry
        # Adapter-declared per-animation hurtbox parts override
        # (head + body split for bosses, etc.). When present, the
        # parts REPLACE the auto-derived bbox above so the player's
        # attack registration only triggers on the central body —
        # not on cosmetic extensions like outstretched arms.
        try:
            hurtboxes_by_anim = adapter.hurtbox_parts((src_fw, src_fh))
        except Exception:
            hurtboxes_by_anim = {}
        for anim_name, hurtbox in (hurtboxes_by_anim or {}).items():
            if not isinstance(hurtbox, dict):
                continue
            parts_in = hurtbox.get("parts")
            if not isinstance(parts_in, list) or not parts_in:
                continue
            cropped_parts = []
            for part in parts_in:
                if not isinstance(part, dict):
                    continue
                x = int(part.get("x", 0))
                y = int(part.get("y", 0))
                w = int(part.get("w", 0))
                h = int(part.get("h", 0))
                cx0 = max(0, x - crop_min_x)
                cy0 = max(0, y - crop_min_y)
                cw = min(fw, x + w - crop_min_x) - cx0
                ch = min(fh, y + h - crop_min_y) - cy0
                if cw > 0 and ch > 0:
                    cropped_parts.append(
                        {
                            "name": str(part.get("name", "")),
                            "x": int(cx0),
                            "y": int(cy0),
                            "w": int(cw),
                            "h": int(ch),
                        }
                    )
            if not cropped_parts:
                continue
            if anim_name not in anim_metrics:
                anim_metrics[anim_name] = {}
            # Replace the auto-derived single-bbox hurtbox with the
            # adapter-declared multi-rect parts. Drop the bbox so
            # consumers (which prefer `parts` over `bbox`) get only
            # the authored shapes.
            anim_metrics[anim_name]["hurtbox"] = {"parts": cropped_parts}

        # Adapter-declared per-animation hitboxes (attack damage
        # geometry). Translated source canvas → cropped frame.
        try:
            hitboxes_by_anim = adapter.attack_hitboxes((src_fw, src_fh))
        except Exception:
            hitboxes_by_anim = {}
        for anim_name, hitbox in (hitboxes_by_anim or {}).items():
            if anim_name not in anim_metrics:
                anim_metrics[anim_name] = {}
            hitbox_out: Dict[str, Any] = {}
            if isinstance(hitbox, dict):
                if isinstance(hitbox.get("bbox"), tuple):
                    x, y, w, h = hitbox["bbox"]
                    cx0 = max(0, int(x) - crop_min_x)
                    cy0 = max(0, int(y) - crop_min_y)
                    cw = min(fw, int(x) + int(w) - crop_min_x) - cx0
                    ch = min(fh, int(y) + int(h) - crop_min_y) - cy0
                    if cw > 0 and ch > 0:
                        hitbox_out["bbox"] = {
                            "x": int(cx0),
                            "y": int(cy0),
                            "w": int(cw),
                            "h": int(ch),
                        }
                if isinstance(hitbox.get("parts"), list):
                    cropped_parts = []
                    for part in hitbox["parts"]:
                        if not isinstance(part, dict):
                            continue
                        x = int(part.get("x", 0))
                        y = int(part.get("y", 0))
                        w = int(part.get("w", 0))
                        h = int(part.get("h", 0))
                        cx0 = max(0, x - crop_min_x)
                        cy0 = max(0, y - crop_min_y)
                        cw = min(fw, x + w - crop_min_x) - cx0
                        ch = min(fh, y + h - crop_min_y) - cy0
                        if cw > 0 and ch > 0:
                            cropped_parts.append(
                                {
                                    "name": str(part.get("name", "")),
                                    "x": int(cx0),
                                    "y": int(cy0),
                                    "w": int(cw),
                                    "h": int(ch),
                                }
                            )
                    if cropped_parts:
                        hitbox_out["parts"] = cropped_parts
            if hitbox_out:
                anim_metrics[anim_name]["hitbox"] = hitbox_out
        # Drop animations with no data (rest, hit, death may end up empty
        # if the adapter declared no hitbox and the alpha bbox was empty).
        anim_metrics = {k: v for k, v in anim_metrics.items() if v}
        if anim_metrics:
            metrics["animations"] = anim_metrics
        manifest["body_metrics"] = metrics
    return sheet, manifest


def write_spritesheet(
    job: CharacterJob,
    image_out: str | Path,
    manifest_out: str | Path | None = None,
    *,
    source_config: str | Path | None = None,
) -> Tuple[Path, Path]:
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
    # Sidecar RON: same data, machine-readable shape for the sandbox's
    # SheetRegistry. The adapter pipeline's YAML is `animations:`-keyed,
    # so we translate to the row-ordered SheetRecord shape here. See
    # `tackon_sheet._emit_sheet_ron` for the tack-on equivalent.
    ron_path = manifest_out.with_suffix(".ron")
    manifest_for_sidecars = dict(manifest)
    manifest_for_sidecars["image"] = image_out.name
    ron_path.write_text(_adapter_manifest_to_ron(manifest_for_sidecars))
    # Optional future-facing runtime sidecar. Current sandbox builds ignore
    # this file, but publishing it now lets every adapter config start
    # declaring sparse actor/body/capability/action metadata without changing
    # the existing SheetRegistry contract.
    write_actor_contract_for_adapter(
        image_out=image_out,
        sheet_ron_out=ron_path,
        manifest=manifest_for_sidecars,
        job=job,
        source_config=source_config,
    )
    return image_out, manifest_out


def _ron_escape(s):
    return s.replace("\\", "\\\\").replace('"', '\\"')


def _ron_some(inner):
    return f"Some({inner})"


def _ron_optional_rect(v):
    if not isinstance(v, dict):
        return "None"
    return _ron_some(
        f"(x: {int(v['x'])}, y: {int(v['y'])}, w: {int(v['w'])}, h: {int(v['h'])})"
    )


def _ron_optional_point(v):
    if not isinstance(v, dict):
        return "None"
    return _ron_some(f"(x: {float(v['x'])}, y: {float(v['y'])})")


def _ron_body_metrics(bm):
    if not isinstance(bm, dict):
        return "None"
    parts = [
        f"body_pixel_bbox: {_ron_optional_rect(bm.get('body_pixel_bbox'))}",
        f"feet_pixel: {_ron_optional_point(bm.get('feet_pixel'))}",
        f"feet_anchor_norm: {_ron_optional_point(bm.get('feet_anchor_norm'))}",
    ]
    anim_metrics = bm.get("animations")
    if isinstance(anim_metrics, dict) and anim_metrics:
        parts.append(f"animations: {_ron_anim_metrics_map(anim_metrics)}")
    return _ron_some(f"({', '.join(parts)})")


def _ron_anim_metrics_map(metrics):
    """RON-serialize per-animation hit + hurt box metadata.

    Shape of each entry::

        {
          "hurtbox": {"bbox": {x, y, w, h}, "parts": [...]},
          "hitbox":  {"bbox": {x, y, w, h}, "parts": [...]},
        }

    Either `hurtbox` or `hitbox` may be absent; within each,
    either `bbox` or `parts` may be absent. Empty entries are
    skipped by `build_spritesheet` upstream.
    """
    if not isinstance(metrics, dict) or not metrics:
        return "{}"
    items = []
    for anim_name, entry in sorted(metrics.items()):
        if not isinstance(entry, dict):
            continue
        inner = []
        for kind in ("hurtbox", "hitbox"):
            box = entry.get(kind)
            if not isinstance(box, dict):
                continue
            inner.append(f"{kind}: Some({_ron_animation_box(box)})")
        items.append(f'"{_ron_escape(anim_name)}": ({", ".join(inner)})')
    return "{" + ", ".join(items) + "}"


def _ron_animation_box(box):
    """Serialize one animation's hit-or-hurt box (parts + bbox)."""
    inner = []
    parts = box.get("parts")
    if isinstance(parts, list) and parts:
        formatted = ", ".join(
            f'(name: "{_ron_escape(str(p.get("name", "")))}", '
            f"x: {int(p['x'])}, y: {int(p['y'])}, w: {int(p['w'])}, h: {int(p['h'])})"
            for p in parts
            if isinstance(p, dict)
        )
        inner.append(f"parts: [{formatted}]")
    bbox = box.get("bbox")
    if isinstance(bbox, dict):
        inner.append(f"bbox: {_ron_optional_rect(bbox)}")
    return "(" + ", ".join(inner) + ")"


def _ron_anchors(anchors):
    if not isinstance(anchors, dict) or not anchors:
        return "{}"
    items = []
    for name, pos in sorted(anchors.items()):
        if isinstance(pos, dict) and "x" in pos and "y" in pos:
            items.append(
                f'"{_ron_escape(str(name))}": (x: {float(pos["x"])}, y: {float(pos["y"])})'
            )
    return "{" + ", ".join(items) + "}" if items else "{}"


def _ron_rect_from_adapter_frame(fr):
    base = f"x: {int(fr['x'])}, y: {int(fr['y'])}, w: {int(fr['w'])}, h: {int(fr['h'])}"
    anchors = fr.get("anchors")
    if anchors:
        return f"({base}, anchors: {_ron_anchors(anchors)})"
    return f"({base})"


def _ron_row_from_adapter(animation_name: str, row_index: int, info: dict) -> str:
    frames = info.get("frames", []) if isinstance(info, dict) else []
    duration_ms = int(info.get("duration_ms", 0)) if isinstance(info, dict) else 0
    rects = []
    for fr in frames:
        if not isinstance(fr, dict):
            continue
        try:
            rects.append(_ron_rect_from_adapter_frame(fr))
        except (KeyError, TypeError, ValueError):
            continue
    rects_str = ",\n            ".join(rects)
    return (
        f"(\n"
        f'        animation: "{_ron_escape(animation_name)}",\n'
        f"        row_index: {int(row_index)},\n"
        f"        frame_count: {len(rects)},\n"
        f"        duration_ms: {duration_ms},\n"
        f"        duration_secs: {round(duration_ms / 1000.0, 6)},\n"
        f"        rects: [\n            {rects_str},\n        ],\n"
        f"    )"
    )


def _adapter_tuning_to_ron(manifest_or_tuning: dict | None) -> str:
    """Serialize optional adapter sheet tuning into a full RON field.

    Compatibility helper used by the sheet-tuning tests and older callers.
    Callers may pass either a full adapter manifest containing `sheet_tuning:`
    / `tuning:` or the tuning dictionary itself. Missing tuning returns the
    empty string sentinel so emitters can omit the `tuning:` field. An explicit
    empty tuning dict is meaningful: it emits default tuning values so the
    generated RON records the authoring decision.
    """
    if not isinstance(manifest_or_tuning, dict):
        return ""

    # Full manifests use these keys. Treat a manifest with no explicit tuning
    # as absent, but treat a bare `{}` tuning dict as "emit defaults".
    full_manifest_keys = {
        "target",
        "image",
        "animations",
        "rows",
        "frame_width",
        "frame_height",
        "label_width",
        "body_metrics",
        "crop",
    }
    has_sheet_tuning_key = "sheet_tuning" in manifest_or_tuning
    has_tuning_key = "tuning" in manifest_or_tuning

    if has_sheet_tuning_key:
        tuning = manifest_or_tuning.get("sheet_tuning")
    elif has_tuning_key:
        tuning = manifest_or_tuning.get("tuning")
    elif any(key in manifest_or_tuning for key in full_manifest_keys):
        return ""
    else:
        tuning = manifest_or_tuning

    if tuning is None or not isinstance(tuning, dict):
        return ""

    collision_scale = float(tuning.get("collision_scale", 1.0))
    frame_sample_inset = int(tuning.get("frame_sample_inset", 0))
    feet_anchor_y = tuning.get("feet_anchor_y_override", tuning.get("feet_anchor_y"))

    fields: list[str] = [
        f"collision_scale: {collision_scale}",
    ]
    if feet_anchor_y is not None:
        fields.append(f"feet_anchor_y_override: Some({float(feet_anchor_y)})")
    fields.append(f"frame_sample_inset: {frame_sample_inset}")

    inner = "\n".join(f"        {field}," for field in fields)
    return f"    tuning: Some((\n{inner}\n    )),\n"


def _adapter_manifest_to_ron(manifest: dict) -> str:
    """Translate the adapter-pipeline YAML manifest (which uses
    `animations: {name: {frames, duration_ms}}`) into the row-ordered
    RON shape consumed by `SheetRegistry`.

    The top-level RON shape is always a list `[SheetRecord, …]` — even
    for single-target adapter sheets — to match the universal
    `Vec<SheetRecord>` loader contract.
    """
    target = manifest["target"]
    anims = manifest.get("animations") or {}
    rows = []
    for row_index, (name, info) in enumerate(
        anims.items() if isinstance(anims, dict) else []
    ):
        rows.append(_ron_row_from_adapter(name, row_index, info))
    if rows:
        rows_inner = "\n    ".join(r + "," for r in rows)
        rows_field = f"    rows: [\n    {rows_inner}\n    ],\n"
    else:
        rows_field = "    rows: [],\n"
    y_offset = int(manifest.get("y_offset", 0))
    y_offset_field = f"    y_offset: {y_offset},\n" if y_offset else ""
    tuning_field = _adapter_tuning_to_ron(manifest)
    return (
        f"// Auto-emitted from {target}_spritesheet.yaml — see\n"
        f"// `presentation::character_sprites::registry`.\n"
        f"[\n"
        f"(\n"
        f'    target: "{_ron_escape(target)}",\n'
        f'    image: "{_ron_escape(manifest.get("image") or f"{target}_spritesheet.png")}",\n'
        f"    label_width: {int(manifest.get('label_width', 0))},\n"
        f"    frame_width: {int(manifest['frame_width'])},\n"
        f"    frame_height: {int(manifest['frame_height'])},\n"
        f"{y_offset_field}"
        f"{tuning_field}"
        f"    body_metrics: {_ron_body_metrics(manifest.get('body_metrics'))},\n"
        f"{rows_field}"
        f"),\n"
        f"]\n"
    )
