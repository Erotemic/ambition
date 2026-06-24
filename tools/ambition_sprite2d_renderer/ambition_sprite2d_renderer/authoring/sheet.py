from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable, List, Tuple

import yaml
from PIL import Image, ImageColor, ImageDraw

from .adapters import get_adapter
from .actor_contract import write_actor_contract_for_adapter
from ..registry import CharacterJob
from ..core.measure import measure_body_metrics
from ..core.manifest_ron import render_adapter, ron_tuning
from .rendering import load_font


def _parse_bg(value: str):
    if str(value).lower() == "transparent":
        return (0, 0, 0, 0)
    r, g, b = ImageColor.getrgb(value)
    return (r, g, b, 255)


def _measure_body_extent(frame: Image.Image) -> Dict[str, Any] | None:
    """Body bbox + feet anchor for one frame. Thin shim over the canonical
    measurement in :mod:`ambition_sprite2d_renderer.core.measure` (the one
    home for "measure by default")."""
    return measure_body_metrics(frame)


def _apply_body_inset(
    bbox: Dict[str, int], inset: Dict[str, float]
) -> Dict[str, int]:
    """Shrink a pixel bbox by per-edge fractions of its own size.

    ``inset`` keys (all optional, default 0): ``left``/``right`` as a fraction
    of width, ``top``/``bottom`` as a fraction of height. The origin moves in by
    the left/top trim; the result never has a non-positive dimension. Used to
    derive an authored gameplay body tighter than the measured alpha box — see
    ``BaseAdapter.body_inset``."""
    left = float(inset.get("left", 0.0))
    right = float(inset.get("right", 0.0))
    top = float(inset.get("top", 0.0))
    bottom = float(inset.get("bottom", 0.0))
    w, h = int(bbox["w"]), int(bbox["h"])
    return {
        "x": int(bbox["x"]) + int(round(w * left)),
        "y": int(bbox["y"]) + int(round(h * top)),
        "w": max(1, int(round(w * (1.0 - left - right)))),
        "h": max(1, int(round(h * (1.0 - top - bottom)))),
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
    # Render at `render_scale` x the authored canvas so the published texture
    # has more native pixels (the toon generator scales its 128-base design to
    # the frame width, so a bigger canvas = a bigger-drawn, higher-res
    # character). Display size in game is collision-driven and aspect-only, so
    # this only sharpens — it never changes how big the sprite appears.
    render_scale = max(1, int(getattr(job.render, "render_scale", 1)))
    src_fw = job.render.frame_width * render_scale
    src_fh = job.render.frame_height * render_scale
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
    ) * render_scale
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
        # Authored body-box inset (adapter-declared): trim the measured alpha
        # box to the intended gameplay body so every character from this adapter
        # shares a tighter collision / hurt body than its full silhouette.
        body_inset = adapter.body_inset()
        if body_inset:
            metrics["body_pixel_bbox"] = _apply_body_inset(
                metrics["body_pixel_bbox"], body_inset
            )
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
                    # Same authored inset as the base body box, so a pose's
                    # hurtbox stays consistent with the tighter gameplay body.
                    if body_inset:
                        entry["hurtbox"]["bbox"] = _apply_body_inset(
                            entry["hurtbox"]["bbox"], body_inset
                        )
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
                    # Attack hitboxes are NOT clamped to the sprite frame.
                    # A melee box (Hollow-Knight nail-style) reaches out in
                    # FRONT of the body, disjoint from it and usually past
                    # the drawn sprite's edge. Translate into cropped-frame
                    # space but keep the authored extent so a forward box
                    # survives. (Hurtboxes ARE the body and stay frame-bound
                    # via the auto alpha-bbox path — different code.)
                    cx0 = int(x) - crop_min_x
                    cy0 = int(y) - crop_min_y
                    cw = int(w)
                    ch = int(h)
                    if cw > 0 and ch > 0:
                        hitbox_out["bbox"] = {
                            "x": int(cx0),
                            "y": int(cy0),
                            "w": int(cw),
                            "h": int(ch),
                        }
                # Optional convex polygon — an effect-conforming hitbox shape
                # (blade arc, cone). Translated source canvas -> cropped frame,
                # NOT clamped (like the attack bbox, it can reach past the body).
                # The runtime reads it as a CombatVolume::Convex.
                poly = hitbox.get("poly")
                if isinstance(poly, list) and len(poly) >= 3:
                    hitbox_out["poly"] = [
                        (float(px) - crop_min_x, float(py) - crop_min_y)
                        for px, py in poly
                    ]
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
                # Optional per-frame active window: which frame indices
                # the attack hitbox is actually live on (e.g. [0] for a
                # hit-on-frame-0 swing). Absent = live on every frame
                # (back-compatible). Consumed by the debug-hitbox overlay
                # and available to the runtime for timing agreement.
                active = hitbox.get("active_frames")
                if isinstance(active, (list, tuple)) and active:
                    hitbox_out["active_frames"] = [int(i) for i in active]
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


# RON manifest emission is unified in core.manifest_ron (one writer for both
# spines). These aliases keep the names callers/tests import.
_adapter_manifest_to_ron = render_adapter
_adapter_tuning_to_ron = ron_tuning
