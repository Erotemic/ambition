"""Generic spritesheet builder for tack-on targets.

The tack-on render pipeline that most procedural characters and props
use under ``targets/``: builds a labeled spritesheet PNG, a per-row
YAML manifest, and a sidecar RON manifest that the sandbox's
`SheetRegistry` consumes at runtime.

This module is the *generic* piece — math, drawing primitives, the
`build_sheet` entry point, the RON emitters. Character-specific
drawing helpers (palettes, body parts, animation poses) live next to
the characters that use them — see
``targets/characters/_pirate_common.py`` for the pirate-family
helpers.

A target's ``render()`` function typically composes:

    from ...tackon_sheet import build_sheet
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=lambda anim, idx, n: _draw_my_frame(anim, idx, n),
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        auto_crop=True,
    )

`build_sheet` returns a dict of paths (``spritesheet`` / ``yaml`` /
``ron`` / ``preview`` / ``canonical`` / ``canonical_transparent``)
which the target's ``render()`` flattens into a `list[Path]` for the
discovery API.
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import List, Optional, Tuple

import yaml
from PIL import Image, ImageDraw, ImageFont

from .actor_contract import write_actor_contract_for_tackon
from ..core.measure import measure_body_metrics
from ..core.manifest_ron import record_to_ron, records_to_ron, ron_tuning

RGBA = Tuple[int, int, int, int]

SCALE = 4
BASE_FRAME = (128, 128)
LABEL_WIDTH = 100

ANIMATIONS = [
    ("idle", 6, 120),
    ("walk", 8, 90),
    ("slash", 6, 85),
    ("taunt", 6, 100),
    ("hurt", 4, 90),
    ("death", 8, 110),
]

# Mirror of the Rust `CharacterAnim::from_name` alias table at
# `crates/ambition_gameplay_core/src/presentation/character_sprites/anim.rs`.
# Two sets so `is_character_sheet` can detect *any* CharacterAnim row
# while `IDLE_ALIASES` flags the missing-Idle case specifically.
IDLE_ALIASES = frozenset(("idle", "opening", "rest", "front_idle", "side_idle"))
CHARACTER_ANIM_NAMES = frozenset(
    (
        *IDLE_ALIASES,
        "walk",
        "stable",
        "spin",
        "side_walk",
        "run",
        "closing",
        "jump",
        "fall",
        "slash",
        "hit",
        "hurt",
        "death",
        "blink_out",
        "blink_in",
        "dash",
        "fly",
        "hover",
        "taunt",
        "ledge_grab",
        "ledge_climb",
        "ledge_getup",
        "wall_grab",
        "float_glide",
        "land_hard",
        "land_recovery",
        "dash_startup",
        "attack_side",
        "attack_up",
        "attack_down",
        "air_neutral",
        "air_forward",
        "air_back",
        "air_down",
        "air_up",
        "ledge_roll",
        "ledge_getup_attack",
    )
)


def diagnose_idle_coverage(target: str, anim_names: List[str]) -> Optional[str]:
    """Return a publish-time warning string when the sheet's row names
    look like a character sheet (≥1 row maps to `CharacterAnim`) but
    none of them is an Idle alias. Returns `None` if the sheet is
    either non-character (props/gates: zero CharacterAnim hits) or
    already has an Idle row.

    Mirroring the runtime's `try_load_spec_for_character_id` filter:
    a character sheet without an Idle row renders as a placeholder
    in-game. Surfacing this at publish time turns the silent failure
    into a visible warning for the renderer author.
    """
    has_character_row = any(n in CHARACTER_ANIM_NAMES for n in anim_names)
    has_idle = any(n in IDLE_ALIASES for n in anim_names)
    if has_character_row and not has_idle:
        return (
            f"[tackon_sheet] WARN: {target!r} sheet has character-anim rows "
            f"{[n for n in anim_names if n in CHARACTER_ANIM_NAMES]!r} but no Idle alias "
            f"(any of {sorted(IDLE_ALIASES)}). The runtime will render this character "
            f"as a colored-rectangle placeholder. Add or rename one row to an Idle alias."
        )
    return None


def font(size: int = 14):
    try:
        return ImageFont.truetype("DejaVuSans.ttf", size)
    except Exception:
        return ImageFont.load_default()


def lerp(a, b, t):
    return a + (b - a) * t


def ease_in_out(t):
    return 0.5 - 0.5 * math.cos(math.pi * max(0.0, min(1.0, t)))


def oscillate(frame_idx: int, nframes: int, phase: float = 0.0) -> float:
    return math.sin((frame_idx / max(1, nframes)) * math.tau + phase)


def rot(pt, deg):
    rad = math.radians(deg)
    c = math.cos(rad)
    s = math.sin(rad)
    x, y = pt
    return (x * c - y * s, x * s + y * c)


def transform(pt, origin, deg=0.0, scale=1.0):
    x, y = pt
    x *= scale
    y *= scale
    x, y = rot((x, y), deg)
    return (origin[0] + x, origin[1] + y)


def poly(draw: ImageDraw.ImageDraw, points, fill, outline=None, width=1):
    draw.polygon(points, fill=fill)
    if outline is not None:
        draw.line(points + [points[0]], fill=outline, width=width, joint="curve")


def rotated_rect_points(center, w, h, deg):
    hw, hh = w / 2.0, h / 2.0
    pts = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
    return [transform(p, center, deg=deg) for p in pts]


def rotated_rect(draw, center, w, h, deg, fill, outline=None, width=1):
    pts = rotated_rect_points(center, w, h, deg)
    poly(draw, pts, fill, outline, width)
    return pts


def circle(draw, center, r, fill, outline=None, width=1):
    x, y = center
    draw.ellipse((x - r, y - r, x + r, y + r), fill=fill, outline=outline, width=width)


def ellipse(draw, bbox, fill, outline=None, width=1):
    draw.ellipse(bbox, fill=fill, outline=outline, width=width)


def line(draw, points, fill, width=1):
    draw.line(points, fill=fill, width=width, joint="curve")


def downsample(img: Image.Image, final_size=BASE_FRAME):
    alpha = img.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return img.resize(final_size, Image.Resampling.LANCZOS)
    x1, y1, x2, y2 = bbox
    crop = img.crop((x1, y1, x2, y2))
    fw, fh = final_size
    target_w = fw * 0.78
    target_h = fh * 0.88
    scale = min(target_w / max(1, crop.width), target_h / max(1, crop.height))
    new_size = (max(1, int(crop.width * scale)), max(1, int(crop.height * scale)))
    crop = crop.resize(new_size, Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", final_size, (0, 0, 0, 0))
    ox = int((fw - new_size[0]) / 2)
    oy = int(fh - new_size[1] - fh * 0.12)
    canvas.alpha_composite(crop, (ox, oy))
    return canvas


def alpha_bbox_metrics(frame: Image.Image):
    # Canonical "measure by default" (core.measure): inclusive last-opaque-row
    # feet + all-channel bbox, shared with the adapter spine. Preserve this
    # path's output shape — rounded values, no frame_width/height, and a
    # degenerate fallback for a fully transparent frame.
    metrics = measure_body_metrics(frame)
    if metrics is None:
        return {
            "body_pixel_bbox": {"x": 0, "y": 0, "w": 0, "h": 0},
            "feet_pixel": {"x": frame.width / 2.0, "y": frame.height},
            "feet_anchor_norm": {"x": 0.0, "y": -0.5},
        }
    feet, anchor = metrics["feet_pixel"], metrics["feet_anchor_norm"]
    return {
        "body_pixel_bbox": metrics["body_pixel_bbox"],
        "feet_pixel": {"x": round(feet["x"], 3), "y": round(feet["y"], 3)},
        "feet_anchor_norm": {
            "x": round(anchor["x"], 6),
            "y": round(anchor["y"], 6),
        },
    }


def build_sheet(
    target: str,
    rows: List[Tuple[str, int, int]],
    render_fn,
    out_dir: Path,
    frame_size=BASE_FRAME,
    label_width=LABEL_WIDTH,
    frame_meta_fn=None,
    auto_crop: bool = True,
    crop_margin: int = 2,
    actor_metadata=None,
    body_metrics_fn=None,
    sheet_tuning=None,
    animation_key_map=None,
    attack_hitboxes=None,
):
    """Build a labeled spritesheet + companion YAML manifest.

    ``frame_meta_fn`` is an optional callable ``(animation, frame_idx,
    nframes) -> dict``. When provided, the returned dict is merged
    into each frame's per-rect metadata, so callers can attach
    anchors, weapon-specific rig data, etc.

    ``auto_crop`` (default ``True``) computes the union alpha bbox
    across EVERY rendered frame and crops every frame (plus the
    canonical) to that bbox + ``crop_margin``. The resulting frame
    size hugs the actual art instead of relying on the caller to
    guess a tight ``frame_size``. Any positional anchors that
    ``frame_meta_fn`` reported under a top-level ``"anchors"`` key
    are automatically translated by the crop offset so the
    coordinates stay correct in the cropped frame.

    ``animation_key_map`` (optional) opts a sheet into publishing
    AUTHORITATIVE per-animation hurtboxes. Bosses look these up by a
    GENERIC gameplay key (``rest`` / ``side_sweep`` / ``floor_slam`` /
    ``spike_halo`` / ``dash_echo`` / ``eye_beam`` / ``hit`` / ``death``),
    NOT the sheet's own row names — so pass ``{row_name: gameplay_key}``.
    Each mapped row emits ``body_metrics.animations[gameplay_key].hurtbox``
    = the row's union alpha bbox (cropped-frame coords), so the player's
    attacks register on the per-pose body instead of the coarse idle bbox.
    Rows with no mapping are skipped. ``attack_hitboxes`` (optional,
    keyed by the same gameplay keys) supplies authored attack-damage
    rects, merged in as ``animations[key].hitbox`` — for boss attacks
    whose damage geometry the sprite author wants to pin.
    """
    fw, fh = frame_size

    # ---- Pass 1: render every frame + metadata into memory. -------------
    # We need all frames in hand before we can compute the union alpha
    # bbox for auto-crop.
    rendered_rows: List[Tuple[str, int, int, List[Tuple[Image.Image, dict]]]] = []
    for row_idx, (anim, nframes, duration_ms) in enumerate(rows):
        frames_data: List[Tuple[Image.Image, dict]] = []
        for frame_idx in range(nframes):
            frame = render_fn(anim, frame_idx, nframes)
            meta = {}
            if frame_meta_fn is not None:
                extra = frame_meta_fn(anim, frame_idx, nframes)
                if extra:
                    meta = dict(extra)
            frames_data.append((frame, meta))
        rendered_rows.append((anim, nframes, duration_ms, frames_data))
    # Canonical pose: use the first row (typically "idle", but pick the
    # first row defensively so targets that name their default animation
    # differently — e.g. galwah's "turn" — still get a useful canonical
    # instead of crashing on a hardcoded "idle" lookup). Frame index 1
    # rather than 0 because most idle cycles start at a neutral pose and
    # frame 1 has a touch more character; falls back to 0 for single-frame rows.
    canon_anim, canon_nframes, _ = rows[0]
    canonical_raw = render_fn(canon_anim, min(1, canon_nframes - 1), canon_nframes)

    # ---- Auto-crop pass (optional) --------------------------------------
    # Union alpha bbox across every frame in the sheet AND the canonical.
    # Cropping uniformly means each frame retains identical dimensions —
    # required for the spritesheet grid to tile correctly — and the
    # canonical also gets the same crop so still poses and animated
    # frames are visually consistent.
    if auto_crop:
        union_bbox: Optional[List[int]] = None
        all_frames_iter = []
        for _, _, _, frames_data in rendered_rows:
            all_frames_iter.extend(f for (f, _) in frames_data)
        all_frames_iter.append(canonical_raw)
        for frame in all_frames_iter:
            alpha = frame.getchannel("A")
            bbox = alpha.getbbox()
            if bbox is None:
                continue
            if union_bbox is None:
                union_bbox = list(bbox)
            else:
                union_bbox[0] = min(union_bbox[0], bbox[0])
                union_bbox[1] = min(union_bbox[1], bbox[1])
                union_bbox[2] = max(union_bbox[2], bbox[2])
                union_bbox[3] = max(union_bbox[3], bbox[3])

        if union_bbox is not None:
            crop_x = max(0, union_bbox[0] - crop_margin)
            crop_y = max(0, union_bbox[1] - crop_margin)
            crop_x1 = min(fw, union_bbox[2] + crop_margin)
            crop_y1 = min(fh, union_bbox[3] + crop_margin)
            new_fw = crop_x1 - crop_x
            new_fh = crop_y1 - crop_y

            cropped_rows: List[
                Tuple[str, int, int, List[Tuple[Image.Image, dict]]]
            ] = []
            for anim, nframes, duration_ms, frames_data in rendered_rows:
                new_data: List[Tuple[Image.Image, dict]] = []
                for frame, meta in frames_data:
                    cropped = frame.crop((crop_x, crop_y, crop_x1, crop_y1))
                    # Translate any positional anchors in `meta.anchors`
                    # by the crop offset so the metadata coordinates
                    # match the cropped frame. Non-anchor fields
                    # (`forward` unit vector, `blade_angle_deg`, …)
                    # pass through unchanged.
                    if meta and "anchors" in meta and isinstance(meta["anchors"], dict):
                        new_anchors = {}
                        for name, pos in meta["anchors"].items():
                            if isinstance(pos, dict) and "x" in pos and "y" in pos:
                                new_anchors[name] = {
                                    "x": round(pos["x"] - crop_x, 2),
                                    "y": round(pos["y"] - crop_y, 2),
                                }
                            else:
                                new_anchors[name] = pos
                        meta = {**meta, "anchors": new_anchors}
                    new_data.append((cropped, meta))
                cropped_rows.append((anim, nframes, duration_ms, new_data))
            rendered_rows = cropped_rows
            canonical_raw = canonical_raw.crop((crop_x, crop_y, crop_x1, crop_y1))
            fw, fh = new_fw, new_fh

    # ---- Pass 2: assemble the spritesheet from the (cropped) frames. ----
    max_frames = max(n for _, n, _ in rows)
    sheet = Image.new(
        "RGBA", (label_width + fw * max_frames, fh * len(rows)), (0, 0, 0, 0)
    )
    preview = Image.new(
        "RGBA", (label_width + fw * max_frames, fh * len(rows)), (34, 34, 40, 255)
    )
    draw_sheet = ImageDraw.Draw(sheet, "RGBA")
    draw_prev = ImageDraw.Draw(preview, "RGBA")
    draw_prev.rectangle((0, 0, preview.width, preview.height), fill=(43, 33, 40, 255))

    rows_meta = []
    first = None
    for row_idx, (anim, nframes, duration_ms, frames_data) in enumerate(rendered_rows):
        y = row_idx * fh
        for dr in [draw_sheet, draw_prev]:
            dr.rectangle((0, y, label_width - 1, y + fh - 1), fill=(18, 22, 30, 235))
            dr.text((8, y + 10), anim, fill=(236, 240, 244, 255), font=font(14))
            dr.text(
                (8, y + 30),
                f"{nframes}f @ {duration_ms}ms",
                fill=(160, 170, 184, 255),
                font=font(11),
            )
        rects = []
        for frame_idx, (frame, meta) in enumerate(frames_data):
            if first is None:
                first = frame.copy()
            x = label_width + frame_idx * fw
            sheet.alpha_composite(frame, (x, y))
            preview.alpha_composite(frame, (x, y))
            rect = {"x": x, "y": y, "w": fw, "h": fh}
            if meta:
                rect.update(meta)
            rects.append(rect)
        rows_meta.append(
            {
                "animation": anim,
                "row_index": row_idx,
                "frame_count": nframes,
                "duration_ms": duration_ms,
                "duration_secs": round(duration_ms / 1000.0, 6),
                "rects": rects,
            }
        )

    can = canonical_raw
    can_bg = Image.new("RGBA", (fw, fh), (43, 33, 40, 255))
    can_bg.alpha_composite(can, (0, 0))

    canonical_path = out_dir / f"{target}_canonical.png"
    canonical_transparent_path = out_dir / f"{target}_canonical_transparent.png"
    sheet_path = out_dir / f"{target}_spritesheet.png"
    yaml_path = out_dir / f"{target}_spritesheet.yaml"
    ron_path = out_dir / f"{target}_spritesheet.ron"
    preview_path = out_dir / f"{target}_preview_labeled.png"

    can_bg.save(canonical_path)
    can.save(canonical_transparent_path)
    sheet.save(sheet_path)
    preview.save(preview_path)

    body_metrics = alpha_bbox_metrics(first or can)
    if body_metrics_fn is not None:
        # Optional target-authored override for sheets whose visible alpha bbox
        # deliberately includes non-body adornments (for example a hat, halo, or
        # carried prop). The default remains alpha-bbox driven so existing
        # generators keep their current conventions; targets that pass this hook
        # can keep gameplay hurtboxes tight to the true character body while the
        # rendered frame still includes decorations.
        body_metrics = body_metrics_fn(fw, fh)

    # Authoritative per-animation hurtboxes (+ optional authored hitboxes), so a
    # boss can register hits on the per-pose body instead of the coarse idle
    # bbox. Keyed by GENERIC gameplay keys (see `animation_key_map` in the
    # docstring) because the boss combat looks animations up that way, not by the
    # sheet's row names. A row's hurtbox is the union alpha bbox across its
    # frames, in cropped-frame coords (same space as `body_pixel_bbox`).
    if animation_key_map:
        anim_metrics = {}
        for anim, _nframes, _duration_ms, frames_data in rendered_rows:
            key = animation_key_map.get(anim)
            if key is None:
                continue
            union = None
            for frame, _meta in frames_data:
                bbox = frame.getchannel("A").getbbox()
                if bbox is None:
                    continue
                union = (
                    list(bbox)
                    if union is None
                    else [
                        min(union[0], bbox[0]),
                        min(union[1], bbox[1]),
                        max(union[2], bbox[2]),
                        max(union[3], bbox[3]),
                    ]
                )
            entry = anim_metrics.setdefault(key, {})
            if union is not None:
                entry["hurtbox"] = {
                    "bbox": {
                        "x": int(union[0]),
                        "y": int(union[1]),
                        "w": int(union[2] - union[0]),
                        "h": int(union[3] - union[1]),
                    }
                }
            if attack_hitboxes and key in attack_hitboxes:
                entry["hitbox"] = attack_hitboxes[key]
        # Drop rows that produced no boxes (fully transparent + no hitbox).
        anim_metrics = {k: v for k, v in anim_metrics.items() if v}
        if anim_metrics:
            body_metrics = {**body_metrics, "animations": anim_metrics}

    manifest = {
        "target": target,
        "image": sheet_path.name,
        "label_width": label_width,
        "frame_width": fw,
        "frame_height": fh,
        "rows": rows_meta,
        "body_metrics": body_metrics,
    }
    if sheet_tuning:
        # Emitted to the RON `tuning` field (ron_tuning reads `sheet_tuning`);
        # the runtime SheetRegistry uses it for in-game display size /
        # sampling instead of the DEFAULT_TUNING fallback.
        manifest["sheet_tuning"] = dict(sheet_tuning)
    yaml_path.write_text(yaml.safe_dump(manifest, sort_keys=False, width=120))
    # Sidecar RON manifest consumed at runtime by the sandbox's
    # SheetRegistry. The YAML is the human-readable sidecar; RON is
    # what gameplay code deserializes. Keep both in lockstep — they
    # encode the same data structure.
    ron_path.write_text(_emit_sheet_ron(manifest))
    # Optional future-facing actor contract sidecar. Targets may pass
    # an `actor_metadata` dict to `build_sheet`; absent metadata is fine,
    # and the sidecar will record sparse inferred facts plus explicit gaps.
    actor_path = write_actor_contract_for_tackon(
        target=target,
        image_out=sheet_path,
        sheet_ron_out=ron_path,
        manifest=manifest,
        actor_metadata=actor_metadata,
    )
    # Surface the most common silent-publish-failure mode (character
    # sheets that lack any Idle alias and would render as a
    # colored-rectangle placeholder in-game) as a stderr warning so
    # the renderer author sees it during `regen_sprites.sh`.
    warning = diagnose_idle_coverage(target, [name for name, _, _ in rows])
    if warning:
        import sys as _sys

        print(warning, file=_sys.stderr)
    return {
        "canonical": canonical_path,
        "canonical_transparent": canonical_transparent_path,
        "spritesheet": sheet_path,
        "yaml": yaml_path,
        "ron": ron_path,
        "actor": actor_path,
        "preview": preview_path,
    }


# RON manifest emission is unified in core.manifest_ron (one writer for both
# spines). These aliases keep the names callers/tests import.
_ron_sheet_record = record_to_ron
_ron_tuning = ron_tuning


def _emit_sheet_ron(manifest):
    """Serialize a manifest to the `Vec<SheetRecord>` RON the game reads."""
    return records_to_ron(manifest["target"], [manifest])


def write_canonical(
    target: str,
    rows: List[Tuple[str, int, int]],
    render_fn,
    out_dir: Path,
    *,
    frame_size: Tuple[int, int] = BASE_FRAME,
    crop_margin: int = 4,
) -> Path:
    """Render ONLY the canonical frame for ``target`` and save it.

    Companion to [`build_sheet`] for callers that want just the
    canonical pose without paying for the full sheet build. Renders
    one frame (first row, frame index 1 — same pose `build_sheet` uses
    for its `*_canonical_transparent.png` side-output), auto-crops to
    the alpha bbox + ``crop_margin``, and saves it as a transparent
    PNG to ``out_dir/{target}_canonical_transparent.png``.

    Returns the saved path. This is the function each tack-on target's
    ``render_canonical(out_dir, **opts)`` hook should call.
    """
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    if not rows:
        raise ValueError(f"{target}: cannot render canonical with no rows")
    anim, nframes, _duration_ms = rows[0]
    frame_idx = min(1, nframes - 1)
    img = render_fn(anim, frame_idx, nframes)
    if img.mode != "RGBA":
        img = img.convert("RGBA")
    bbox = img.getchannel("A").getbbox()
    if bbox is not None:
        x1, y1, x2, y2 = bbox
        x1 = max(0, x1 - crop_margin)
        y1 = max(0, y1 - crop_margin)
        x2 = min(img.width, x2 + crop_margin)
        y2 = min(img.height, y2 + crop_margin)
        img = img.crop((x1, y1, x2, y2))
    # Silence unused-arg warning while keeping the signature future-
    # proof for callers that want to pass through frame_size for a
    # custom pre-crop pipeline.
    del frame_size
    out = out_dir / f"{target}_canonical_transparent.png"
    img.save(out)
    return out
