"""Debug overlay tool: draw per-animation hurt + hit boxes on a spritesheet.

Reads the sidecar YAML manifest for a rendered spritesheet (e.g.
``boss_spritesheet.yaml``) and writes a sibling
``boss_spritesheet_debug.png`` with each animation's hurtbox
(cyan) and hitbox (red) drawn over every frame in that row. Each
animation row is labelled with the box dimensions.

Used by sprite authors to verify that the renderer's
auto-derived hurtboxes line up with the visible body and that
the adapter-declared attack hitboxes are positioned correctly
for the strike pose.

Run via the CLI::

    python -m ambition_sprite2d_renderer debug-hitboxes <sheet.yaml>
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Tuple

import yaml
from PIL import Image, ImageDraw

from ..authoring.canonical import load_font

HURTBOX_OUTLINE = (0, 230, 255, 230)  # cyan
HURTBOX_FILL = (0, 230, 255, 40)
HITBOX_OUTLINE = (255, 60, 60, 240)  # red — hitbox LIVE this frame
HITBOX_FILL = (255, 60, 60, 60)
HITBOX_OUTLINE_OFF = (255, 90, 90, 80)  # dim red — hitbox declared but not live
HITBOX_FILL_OFF = (255, 90, 90, 16)
ACTIVE_BADGE = (255, 224, 64, 245)  # yellow per-frame "live" marker
INACTIVE_BADGE = (150, 150, 160, 200)
PART_LABEL = (255, 255, 255, 220)


def _coerce_box(
    box: Any,
) -> Optional[Tuple[List[Dict[str, Any]], Optional[Dict[str, Any]]]]:
    """Normalize a hit-or-hurt box descriptor into (parts, bbox)."""
    if not isinstance(box, dict):
        return None
    parts = box.get("parts")
    if not isinstance(parts, list):
        parts = []
    bbox = box.get("bbox")
    if not isinstance(bbox, dict):
        bbox = None
    if not parts and bbox is None:
        return None
    return parts, bbox


def _rects_from_box(box: Any) -> List[Tuple[int, int, int, int, str]]:
    """Return ``[(x, y, w, h, label)]`` for every rect in the box."""
    coerced = _coerce_box(box)
    if coerced is None:
        return []
    parts, bbox = coerced
    rects: List[Tuple[int, int, int, int, str]] = []
    for part in parts:
        if not isinstance(part, dict):
            continue
        rects.append(
            (
                int(part.get("x", 0)),
                int(part.get("y", 0)),
                int(part.get("w", 0)),
                int(part.get("h", 0)),
                str(part.get("name", "")),
            )
        )
    if bbox is not None and not rects:
        # Only emit the bbox if there are no parts — parts take
        # priority. (When both are present the parts are
        # authoritative; bbox is the fallback.)
        rects.append(
            (
                int(bbox.get("x", 0)),
                int(bbox.get("y", 0)),
                int(bbox.get("w", 0)),
                int(bbox.get("h", 0)),
                "",
            )
        )
    return rects


def _draw_rect_with_label(
    draw: ImageDraw.ImageDraw,
    rect: Tuple[int, int, int, int],
    outline: Tuple[int, int, int, int],
    fill: Tuple[int, int, int, int],
    label: str,
) -> None:
    x, y, w, h = rect
    if w <= 0 or h <= 0:
        return
    # Translucent fill so overlapping rects still read as distinct
    # shapes (rather than a single solid color).
    draw.rectangle((x, y, x + w - 1, y + h - 1), fill=fill, outline=outline, width=1)
    if label:
        font = load_font(9)
        draw.text((x + 2, y + 1), label, fill=PART_LABEL, font=font)


def render_debug_overlay(yaml_path: Path, out_path: Optional[Path] = None) -> Path:
    """Render the debug overlay PNG. Returns the written path."""
    yaml_path = Path(yaml_path)
    if not yaml_path.exists():
        raise FileNotFoundError(yaml_path)
    manifest = yaml.safe_load(yaml_path.read_text())
    image_name = manifest.get("image") or (yaml_path.stem + ".png")
    image_path = yaml_path.parent / image_name
    if not image_path.exists():
        raise FileNotFoundError(image_path)
    sheet = Image.open(image_path).convert("RGBA")

    metrics = manifest.get("body_metrics") or {}
    anim_metrics = metrics.get("animations") or {}
    animations = manifest.get("animations") or {}

    overlay = Image.new("RGBA", sheet.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")

    legend_font = load_font(11)

    badge_font = load_font(9)
    for animation, info in animations.items():
        anim_entry = anim_metrics.get(animation) or {}
        hurtbox = anim_entry.get("hurtbox")
        hitbox = anim_entry.get("hitbox")
        hurt_rects = _rects_from_box(hurtbox)
        hit_rects = _rects_from_box(hitbox)
        if not hurt_rects and not hit_rects:
            continue
        # Which frame indices the attack hitbox is actually live on.
        # Absent => live on every frame (back-compatible). This is what
        # makes a 2-frame swing legible: "hit on frame 0, recover on
        # frame 1" instead of one static box smeared across the row.
        active_frames: Optional[set] = None
        if isinstance(hitbox, dict):
            af = hitbox.get("active_frames")
            if isinstance(af, (list, tuple)):
                active_frames = {int(i) for i in af}
        for frame_idx, frame in enumerate(info.get("frames", [])):
            fx = int(frame.get("x", 0))
            fy = int(frame.get("y", 0))
            for rx, ry, rw, rh, label in hurt_rects:
                _draw_rect_with_label(
                    draw,
                    (fx + rx, fy + ry, rw, rh),
                    HURTBOX_OUTLINE,
                    HURTBOX_FILL,
                    f"H {label}" if label else "H",
                )
            hit_live = active_frames is None or frame_idx in active_frames
            for rx, ry, rw, rh, label in hit_rects:
                outline = HITBOX_OUTLINE if hit_live else HITBOX_OUTLINE_OFF
                fill = HITBOX_FILL if hit_live else HITBOX_FILL_OFF
                marker = "X" if hit_live else "x"
                _draw_rect_with_label(
                    draw,
                    (fx + rx, fy + ry, rw, rh),
                    outline,
                    fill,
                    f"{marker} {label}" if label else marker,
                )
            # Per-frame badge in the frame's top-left: index + live state,
            # but only when this animation actually has an attack hitbox
            # to reason about (keeps idle/walk rows uncluttered).
            if hit_rects:
                badge = f"f{frame_idx} {'HIT' if hit_live else '-'}"
                color = ACTIVE_BADGE if hit_live else INACTIVE_BADGE
                draw.text((fx + 2, fy + 2), badge, fill=color, font=badge_font)

    # Legend block in the label column on the left so each row's
    # bare row-name is supplemented with its (hurt + hit) shapes.
    legend_y = 0
    for animation, info in animations.items():
        anim_entry = anim_metrics.get(animation) or {}
        frames = info.get("frames", [])
        if not frames:
            continue
        # First frame's y is the row's top edge.
        row_y = int(frames[0].get("y", 0))
        hurtbox = anim_entry.get("hurtbox")
        hitbox = anim_entry.get("hitbox")
        legend_lines = []
        if hurtbox:
            bbox = hurtbox.get("bbox")
            parts = hurtbox.get("parts") or []
            if isinstance(bbox, dict):
                legend_lines.append(f"H {bbox['w']}x{bbox['h']}")
            if parts:
                legend_lines.append(f"H[{len(parts)}]")
        if hitbox:
            bbox = hitbox.get("bbox")
            parts = hitbox.get("parts") or []
            if isinstance(bbox, dict):
                legend_lines.append(f"X {bbox['w']}x{bbox['h']}")
            if parts:
                legend_lines.append(f"X[{len(parts)}]")
        for i, line in enumerate(legend_lines):
            color = HURTBOX_OUTLINE if line.startswith("H") else HITBOX_OUTLINE
            draw.text((4, row_y + 38 + i * 12), line, fill=color, font=legend_font)
        legend_y = row_y

    annotated = Image.alpha_composite(sheet, overlay)
    if out_path is None:
        out_path = image_path.with_name(image_path.stem + "_debug.png")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    annotated.save(out_path)
    return out_path
