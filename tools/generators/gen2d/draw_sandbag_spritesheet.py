#!/usr/bin/env python3
"""Generate a procedural sandbag character spritesheet for Ambition.

The sandbag deliberately emits only the animations it actually owns:
idle, hit, and death. Runtime support for sparse animation rows is provided by
``apply_sandbag_runtime_patch.py``; after that patch, missing requests such as
walk/run/slash resolve to idle for this sheet instead of requiring alias rows.

Run from tools/generators/gen2d:
    python draw_sandbag_spritesheet.py
    python draw_sandbag_spritesheet.py --copy-to-sandbox
"""
from __future__ import annotations

import argparse
import math
import shutil
from pathlib import Path
from typing import Dict, Iterable, List, Tuple

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError as ex:  # pragma: no cover
    raise SystemExit("This generator needs Pillow. Install with: python -m pip install pillow") from ex

RGBA = Tuple[int, int, int, int]

FRAME_W = 128
FRAME_H = 128
LABEL_W = 100
SCALE = 4

# Native rows for this character. Do not pad with walk/run/etc. The runtime
# patch teaches CharacterSheetSpec to map missing rows to idle on demand.
SANDBAG_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 120),
    ("hit", 4, 75),
    ("death", 7, 112),
]

# Optional legacy output for old fixed-11-row runtime builds. The normal output
# is intentionally sparse; this exists only as an emergency compatibility shim.
LEGACY_RUNTIME_ROWS: List[Tuple[str, str, int, int]] = [
    ("idle", "idle", 6, 120),
    ("walk", "idle", 6, 120),
    ("run", "idle", 6, 120),
    ("jump", "idle", 6, 120),
    ("fall", "idle", 6, 120),
    ("slash", "idle", 6, 120),
    ("hit", "hit", 4, 75),
    ("death", "death", 7, 112),
    ("blink_out", "idle", 6, 120),
    ("blink_in", "idle", 6, 120),
    ("dash", "idle", 6, 120),
]


def _s(v: float) -> int:
    return int(round(v * SCALE))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _rgba(hex_color: str, alpha: int = 255) -> RGBA:
    hex_color = hex_color.lstrip("#")
    return (int(hex_color[0:2], 16), int(hex_color[2:4], 16), int(hex_color[4:6], 16), alpha)


def _font(size: int = 12):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def _dashed_line(draw: ImageDraw.ImageDraw, xy: Tuple[float, float, float, float], fill: RGBA, width: float = 1.0, dash: float = 4.0) -> None:
    x1, y1, x2, y2 = xy
    total = math.hypot(x2 - x1, y2 - y1)
    if total <= 0:
        return
    steps = max(1, int(total / dash))
    for k in range(0, steps, 2):
        a = k / steps
        b = min(1.0, (k + 1) / steps)
        xa, ya = x1 + (x2 - x1) * a, y1 + (y2 - y1) * a
        xb, yb = x1 + (x2 - x1) * b, y1 + (y2 - y1) * b
        draw.line((_s(xa), _s(ya), _s(xb), _s(yb)), fill=fill, width=max(1, _s(width)))


def _draw_eye(draw: ImageDraw.ImageDraw, cx: float, cy: float, *, scale_y: float = 1.0, expression: str = "normal") -> None:
    dark = _rgba("14131d")
    hi = _rgba("f7f8ff", 235)
    if expression == "x":
        for dx in (-4, 4):
            draw.line((_s(cx - dx), _s(cy - 4), _s(cx + dx), _s(cy + 4)), fill=dark, width=_s(2))
        return
    if expression == "sleepy":
        draw.line((_s(cx - 5), _s(cy), _s(cx + 5), _s(cy + 1)), fill=dark, width=_s(2))
        return
    if expression == "squint":
        draw.arc(_box(cx - 5, cy - 4, cx + 5, cy + 5), 18, 162, fill=dark, width=_s(2))
        return
    draw.rounded_rectangle(
        _box(cx - 4.6, cy - 12.0 * scale_y, cx + 4.6, cy + 11.0 * scale_y),
        radius=_s(4.2),
        fill=dark,
    )
    draw.ellipse(_box(cx - 2.1, cy - 8.5 * scale_y, cx + 1.0, cy - 4.0 * scale_y), fill=hi)


def _draw_sandbag_body(
    layer: Image.Image,
    *,
    cx: float,
    cy: float,
    sx: float,
    sy: float,
    eyes: str = "normal",
    tint: float = 1.0,
    strap_swing: float = 0.0,
) -> None:
    """Draw the original pale-cloth sandbag character.

    The art intentionally rhymes with familiar platform-fighter sandbags:
    pale sack, oval eyes, stitched top/bottom bands, and a small top strap.
    Proportions, strap shape, seam layout, and shading are distinct from the
    reference so this remains an original procedural asset.
    """
    draw = ImageDraw.Draw(layer, "RGBA")

    def tone(rgb: Tuple[int, int, int], alpha: int = 255) -> RGBA:
        return tuple(max(0, min(255, int(v * tint))) for v in rgb) + (alpha,)

    cloth = tone((225, 227, 242))
    cloth_mid = tone((204, 207, 226))
    cloth_shadow = tone((163, 166, 187))
    cloth_dark = tone((99, 101, 120))
    stitch = tone((75, 77, 96))
    highlight = tone((247, 248, 255), 185)

    w = 44 * sx
    h = 76 * sy
    top = cy - h / 2
    bottom = cy + h / 2
    left = cx - w / 2
    right = cx + w / 2

    # Main sack silhouette: a soft, slightly asymmetric rounded body, not a
    # perfect cylinder. The lower half bulges a bit more than the top.
    draw.rounded_rectangle(_box(left + 1, top + 5, right - 1, bottom - 2), radius=_s(20), fill=cloth_mid, outline=stitch, width=_s(2.0))
    draw.rounded_rectangle(_box(left + 4, top + 8, right - 7, bottom - 5), radius=_s(18), fill=cloth, width=0)

    # Side shading and a belly highlight make the simple shape read as stuffed
    # cloth without copying any exact source highlights.
    draw.pieslice(_box(right - 20, top + 9, right + 10, bottom - 2), 84, 276, fill=cloth_shadow)
    draw.pieslice(_box(left + 4, top + 19, right - 12, bottom - 15), 105, 258, fill=highlight)
    draw.arc(_box(right - 17, top + 14, right + 1, bottom - 6), 78, 278, fill=cloth_dark, width=_s(1.3))

    # Top and bottom stitched caps.
    draw.ellipse(_box(left + 2, top - 2, right - 2, top + 21), fill=tone((235, 237, 249)), outline=stitch, width=_s(1.6))
    draw.arc(_box(left + 5, top + 3, right - 5, top + 19), 10, 172, fill=highlight, width=_s(1.0))
    draw.arc(_box(left + 1, bottom - 20, right - 1, bottom + 1), 10, 170, fill=stitch, width=_s(1.5))
    draw.arc(_box(left + 4, bottom - 17, right - 4, bottom - 2), 13, 169, fill=cloth_shadow, width=_s(3.0))

    # Stitches on the cap bands. Small dashed arcs are enough at gameplay scale.
    for i in range(12):
        t = i / 11
        x = left + 7 + (w - 14) * t
        y_top = top + 17 + math.sin(t * math.pi) * 2.2
        y_bot = bottom - 9 + math.sin(t * math.pi) * 2.0
        draw.line((_s(x - 1.5), _s(y_top), _s(x + 1.4), _s(y_top + 0.5)), fill=cloth_dark, width=_s(0.9))
        draw.line((_s(x - 1.6), _s(y_bot), _s(x + 1.5), _s(y_bot + 0.4)), fill=cloth_dark, width=_s(0.9))

    # Hanging tab/strap: same visual vocabulary as the reference, but shorter,
    # wider, and attached at a different angle with an inset patch.
    strap_x = right - 9 + strap_swing
    strap_y = top + 3
    draw.rounded_rectangle(_box(strap_x - 5, strap_y - 1, strap_x + 8, strap_y + 28), radius=_s(4), fill=cloth_mid, outline=stitch, width=_s(1.4))
    draw.rounded_rectangle(_box(strap_x - 2, strap_y + 4, strap_x + 5, strap_y + 20), radius=_s(2), fill=cloth, width=0)
    draw.line((_s(strap_x - 3), _s(strap_y + 23), _s(strap_x + 7), _s(strap_y + 22)), fill=cloth_dark, width=_s(1.1))

    # A few cloth wrinkles, deliberately sparse.
    draw.arc(_box(left + 5, cy - 17, right - 11, cy + 1), 192, 340, fill=tone((150, 153, 175), 96), width=_s(1.0))
    draw.arc(_box(left + 3, cy + 5, right - 8, cy + 24), 195, 342, fill=tone((150, 153, 175), 90), width=_s(1.0))
    _dashed_line(draw, (left + 10, top + 28, left + 7, bottom - 19), tone((115, 117, 138), 110), width=0.8, dash=5)

    # Face. The eyes are the immediately recognizable rhyme; placement,
    # spacing, and highlights differ from the reference.
    eye_y = cy - 10 * sy
    if eyes == "x":
        _draw_eye(draw, cx - 10 * sx, eye_y, expression="x")
        _draw_eye(draw, cx + 10 * sx, eye_y, expression="x")
    elif eyes == "sleepy":
        _draw_eye(draw, cx - 10 * sx, eye_y + 1, expression="sleepy")
        _draw_eye(draw, cx + 10 * sx, eye_y + 1, expression="sleepy")
    elif eyes == "squint":
        _draw_eye(draw, cx - 10 * sx, eye_y, expression="squint")
        _draw_eye(draw, cx + 10 * sx, eye_y, expression="squint")
    else:
        _draw_eye(draw, cx - 10 * sx, eye_y, scale_y=sy)
        _draw_eye(draw, cx + 10 * sx, eye_y, scale_y=sy)


def _impact_marks(canvas: Image.Image, frame_index: int) -> None:
    draw = ImageDraw.Draw(canvas, "RGBA")
    alpha = max(50, 215 - frame_index * 45)
    yellow = _rgba("ffe56f", alpha)
    orange = _rgba("ff8740", alpha)
    cx, cy = 29 + frame_index * 2, 58 - frame_index * 2
    rays = [(-13, 0), (13, 0), (0, -12), (0, 12), (-9, -8), (9, 8), (-9, 8), (9, -8)]
    for dx, dy in rays:
        draw.line((_s(cx), _s(cy), _s(cx + dx), _s(cy + dy)), fill=yellow if abs(dx) + abs(dy) > 14 else orange, width=_s(2.0))
    for k in range(3):
        draw.line((_s(36 + k * 11), _s(47 + k * 12), _s(55 + k * 9), _s(47 + k * 12)), fill=_rgba("9ba0ba", max(0, alpha - 75)), width=_s(1.0))


def _dust(canvas: Image.Image, frame_index: int, base_x: float = 68.0, base_y: float = 112.0) -> None:
    draw = ImageDraw.Draw(canvas, "RGBA")
    for i in range(5):
        x = base_x - 20 + i * 10 + frame_index * (1.4 - i * 0.25)
        y = base_y + (i % 2) * 3 - frame_index * 0.45
        r = 1.8 + (i % 3)
        alpha = max(0, 92 - frame_index * 14)
        draw.ellipse(_box(x - r, y - r, x + r, y + r), fill=_rgba("9f937f", alpha))


def render_frame(animation: str, frame_index: int, frame_count: int) -> Image.Image:
    canvas = Image.new("RGBA", (FRAME_W * SCALE, FRAME_H * SCALE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas, "RGBA")

    phase = math.sin((frame_index / max(1, frame_count)) * math.tau)
    cx = 65.0
    cy = 71.0
    sx = 1.0
    sy = 1.0
    angle = 0.0
    eyes = "normal"
    tint = 1.0
    strap_swing = phase * 0.8
    shadow_w = 45.0
    shadow_a = 70

    if animation == "idle":
        cy += phase * 1.5
        sx = 1.0 - phase * 0.018
        sy = 1.0 + phase * 0.028
        angle = phase * 1.4
        shadow_w = 45 + phase * 2.0
    elif animation == "hit":
        poses = [
            (-5, -1, 1.06, 0.91, -10, "squint", 1.04, -3),
            (8, 3, 0.91, 1.10, 9, "squint", 0.97, 4),
            (1, 0, 1.04, 0.96, -5, "normal", 1.01, -1),
            (0, 0, 1.00, 1.00, 0, "normal", 1.00, 0),
        ][frame_index % 4]
        dx, dy, sx, sy, angle, eyes, tint, strap_swing = poses
        cx += dx
        cy += dy
        shadow_w = 49 + abs(dx) * 1.1
        shadow_a = 88
    elif animation == "death":
        poses = [
            (0, 0, 1.00, 1.00, 0, "sleepy", 1.00, 0),
            (7, 8, 1.03, 0.95, 15, "sleepy", 0.99, 2),
            (16, 17, 1.03, 0.92, 34, "sleepy", 0.97, 3),
            (23, 27, 1.03, 0.84, 57, "sleepy", 0.95, 3),
            (24, 35, 1.08, 0.72, 77, "x", 0.93, 4),
            (22, 38, 1.13, 0.61, 88, "x", 0.91, 5),
            (22, 39, 1.15, 0.56, 91, "x", 0.89, 5),
        ]
        dx, dy, sx, sy, angle, eyes, tint, strap_swing = poses[min(frame_index, len(poses) - 1)]
        cx += dx
        cy += dy
        shadow_w = 53 + frame_index * 5.0
        shadow_a = 90
        _dust(canvas, frame_index, base_x=69 + dx * 0.3)
    else:
        raise KeyError(f"unknown sandbag animation: {animation!r}")

    draw.ellipse(_box(64 - shadow_w / 2, 108, 64 + shadow_w / 2, 118), fill=(35, 35, 48, shadow_a))

    body_layer = Image.new("RGBA", canvas.size, (0, 0, 0, 0))
    _draw_sandbag_body(body_layer, cx=cx, cy=cy, sx=sx, sy=sy, eyes=eyes, tint=tint, strap_swing=strap_swing)
    if angle:
        body_layer = body_layer.rotate(
            angle,
            center=(_s(cx), _s(cy + 18)),
            resample=Image.Resampling.BICUBIC,
            fillcolor=(0, 0, 0, 0),
        )
    canvas.alpha_composite(body_layer)

    if animation == "hit":
        _impact_marks(canvas, frame_index)
        _dust(canvas, frame_index, base_x=72, base_y=113)
    if animation == "death" and frame_index >= 3:
        _dust(canvas, frame_index, base_x=76, base_y=114)

    return canvas.resize((FRAME_W, FRAME_H), Image.Resampling.LANCZOS)


def _measure_body_extent(frame: Image.Image) -> Dict[str, object] | None:
    bbox = frame.getchannel("A").getbbox()
    if bbox is None:
        return None
    x1, y1, x2, y2 = bbox
    feet_y = y2 - 1
    feet_x = (x1 + x2 - 1) / 2.0
    return {
        "frame_width": frame.width,
        "frame_height": frame.height,
        "body_pixel_bbox": {"x": int(x1), "y": int(y1), "w": int(x2 - x1), "h": int(y2 - y1)},
        "feet_pixel": {"x": round(feet_x, 3), "y": round(float(feet_y), 3)},
        "feet_anchor_norm": {"x": round(feet_x / frame.width - 0.5, 6), "y": round(0.5 - feet_y / frame.height, 6)},
    }


def _yaml_scalar(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    if text.replace("_", "").replace("-", "").replace(".", "").isalnum():
        return text
    return repr(text)


def _write_manifest(path: Path, manifest: Dict[str, object]) -> None:
    # Small hand-rolled YAML writer to avoid requiring PyYAML for this one-off script.
    lines: List[str] = []

    def emit(key: str, value: object, indent: int = 0) -> None:
        pad = " " * indent
        if isinstance(value, dict):
            lines.append(f"{pad}{key}:")
            for k, v in value.items():
                emit(str(k), v, indent + 2)
        elif isinstance(value, list):
            lines.append(f"{pad}{key}:")
            for item in value:
                if isinstance(item, dict):
                    lines.append(f"{pad}-")
                    for k, v in item.items():
                        emit(str(k), v, indent + 2)
                else:
                    lines.append(f"{pad}- {_yaml_scalar(item)}")
        else:
            lines.append(f"{pad}{key}: {_yaml_scalar(value)}")

    for key, value in manifest.items():
        emit(key, value, 0)
    path.write_text("\n".join(lines) + "\n", encoding="utf8")


def _rows_for_legacy() -> List[Tuple[str, str, int, int]]:
    return LEGACY_RUNTIME_ROWS


def _rows_for_sparse() -> List[Tuple[str, str, int, int]]:
    return [(name, name, frames, duration_ms) for name, frames, duration_ms in SANDBAG_ROWS]


def build_sheet(rows: List[Tuple[str, str, int, int]], *, sheet_background: RGBA = (0, 0, 0, 0)) -> Tuple[Image.Image, Dict[str, object]]:
    max_frames = max(frames for _, _, frames, _ in rows)
    sheet = Image.new("RGBA", (LABEL_W + max_frames * FRAME_W, len(rows) * FRAME_H), sheet_background)
    draw = ImageDraw.Draw(sheet, "RGBA")
    font = _font(12)
    small = _font(10)
    manifest: Dict[str, object] = {
        "target": "sandbag",
        "frame_width": FRAME_W,
        "frame_height": FRAME_H,
        "label_width": LABEL_W,
        "border": 0,
        "animation_order": [row_name for row_name, _, _, _ in rows],
        "notes": "Sparse sheet: only rows listed in animation_order are emitted. Runtime should resolve missing animations to idle.",
        "style_notes": "Pale stitched cloth sandbag with strap and oval eyes; references the uploaded sandbag visually without copying its exact proportions or seams.",
        "animations": {},
    }
    first_frame: Image.Image | None = None
    for row_idx, (row_name, source_name, frame_count, duration_ms) in enumerate(rows):
        y = row_idx * FRAME_H
        draw.rectangle((0, y, LABEL_W, y + FRAME_H), fill=(24, 24, 38, 188))
        draw.text((8, y + 9), row_name, fill=(238, 239, 255, 255), font=font)
        label = f"{frame_count}f/{duration_ms}ms"
        if row_name != source_name:
            label += f" -> {source_name}"
        draw.text((8, y + 28), label, fill=(186, 189, 214, 255), font=small)
        frame_records = []
        for frame_index in range(frame_count):
            frame = render_frame(source_name, frame_index % frame_count, frame_count)
            x = LABEL_W + frame_index * FRAME_W
            sheet.alpha_composite(frame, (x, y))
            if first_frame is None and row_name == "idle" and frame_index == 0:
                first_frame = frame
            frame_records.append({"index": frame_index, "x": x, "y": y, "w": FRAME_W, "h": FRAME_H, "duration_ms": duration_ms})
        manifest["animations"][row_name] = {
            "source_animation": source_name,
            "frames": frame_records,
            "duration_ms": duration_ms,
        }
    metrics = _measure_body_extent(first_frame) if first_frame is not None else None
    if metrics is not None:
        manifest["body_metrics"] = metrics
    return sheet, manifest


def write_outputs(out_dir: Path, *, legacy_aliases: bool = False) -> Tuple[Path, Path]:
    rows = _rows_for_legacy() if legacy_aliases else _rows_for_sparse()
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = "sandbag_legacy_11row_spritesheet" if legacy_aliases else "sandbag_spritesheet"
    png_path = out_dir / f"{stem}.png"
    yaml_path = out_dir / f"{stem}.yaml"
    sheet, manifest = build_sheet(rows)
    sheet.save(png_path)
    _write_manifest(yaml_path, manifest)
    return png_path, yaml_path


def copy_to_sandbox(paths: Iterable[Path], script_dir: Path) -> List[Path]:
    repo_root = script_dir.resolve().parents[2]
    sandbox_dir = repo_root / "crates" / "ambition_sandbox" / "assets" / "sprites"
    sandbox_dir.mkdir(parents=True, exist_ok=True)
    copied = []
    for path in paths:
        dst = sandbox_dir / path.name
        shutil.copy2(path, dst)
        copied.append(dst)
    return copied


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out-dir", default="assets", help="Output directory relative to this script unless absolute.")
    parser.add_argument("--legacy-aliases", action="store_true", help="Also emit an old-runtime 11-row alias sheet named sandbag_legacy_11row_spritesheet.*")
    parser.add_argument("--copy-to-sandbox", action="store_true", help="Copy generated sparse PNG/YAML into crates/ambition_sandbox/assets/sprites.")
    args = parser.parse_args(argv)

    script_dir = Path(__file__).resolve().parent
    out_dir = Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = script_dir / out_dir

    outputs: List[Path] = list(write_outputs(out_dir, legacy_aliases=False))
    if args.legacy_aliases:
        outputs.extend(write_outputs(out_dir, legacy_aliases=True))

    for path in outputs:
        print(path)
    if args.copy_to_sandbox:
        copied = copy_to_sandbox(outputs[:2], script_dir)
        for path in copied:
            print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
