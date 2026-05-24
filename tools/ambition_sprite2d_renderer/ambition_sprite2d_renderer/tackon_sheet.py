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
    alpha = frame.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return {
            "body_pixel_bbox": {"x": 0, "y": 0, "w": 0, "h": 0},
            "feet_pixel": {"x": frame.width / 2.0, "y": frame.height},
            "feet_anchor_norm": {"x": 0.0, "y": -0.5},
        }
    x1, y1, x2, y2 = bbox
    feet_x = (x1 + x2) / 2.0
    feet_y = float(y2)
    return {
        "body_pixel_bbox": {"x": int(x1), "y": int(y1), "w": int(x2 - x1), "h": int(y2 - y1)},
        "feet_pixel": {"x": round(feet_x, 3), "y": round(feet_y, 3)},
        "feet_anchor_norm": {
            "x": round(feet_x / frame.width - 0.5, 6),
            "y": round(0.5 - feet_y / frame.height, 6),
        },
    }

def build_sheet(target: str, rows: List[Tuple[str, int, int]], render_fn, out_dir: Path, frame_size=BASE_FRAME, label_width=LABEL_WIDTH, frame_meta_fn=None, auto_crop: bool = True, crop_margin: int = 2):
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
        for (_, _, _, frames_data) in rendered_rows:
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

            cropped_rows: List[Tuple[str, int, int, List[Tuple[Image.Image, dict]]]] = []
            for (anim, nframes, duration_ms, frames_data) in rendered_rows:
                new_data: List[Tuple[Image.Image, dict]] = []
                for (frame, meta) in frames_data:
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
    sheet = Image.new("RGBA", (label_width + fw * max_frames, fh * len(rows)), (0, 0, 0, 0))
    preview = Image.new("RGBA", (label_width + fw * max_frames, fh * len(rows)), (34, 34, 40, 255))
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
            dr.text((8, y + 30), f"{nframes}f @ {duration_ms}ms", fill=(160, 170, 184, 255), font=font(11))
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
        rows_meta.append({
            "animation": anim,
            "row_index": row_idx,
            "frame_count": nframes,
            "duration_ms": duration_ms,
            "duration_secs": round(duration_ms / 1000.0, 6),
            "rects": rects,
        })

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

    manifest = {
        "target": target,
        "image": sheet_path.name,
        "label_width": label_width,
        "frame_width": fw,
        "frame_height": fh,
        "rows": rows_meta,
        "body_metrics": alpha_bbox_metrics(first or can),
    }
    yaml_path.write_text(yaml.safe_dump(manifest, sort_keys=False, width=120))
    # Sidecar RON manifest consumed at runtime by the sandbox's
    # SheetRegistry. The YAML is the human-readable sidecar; RON is
    # what gameplay code deserializes. Keep both in lockstep — they
    # encode the same data structure.
    ron_path.write_text(_emit_sheet_ron(manifest))
    return {
        "canonical": canonical_path,
        "canonical_transparent": canonical_transparent_path,
        "spritesheet": sheet_path,
        "yaml": yaml_path,
        "ron": ron_path,
        "preview": preview_path,
    }


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
    return _ron_some(f"({', '.join(parts)})")


def _ron_anchors(anchors):
    if not isinstance(anchors, dict) or not anchors:
        return "{}"
    items = []
    for name, pos in sorted(anchors.items()):
        if not isinstance(pos, dict) or "x" not in pos or "y" not in pos:
            continue
        items.append(
            f'"{_ron_escape(str(name))}": (x: {float(pos["x"])}, y: {float(pos["y"])})'
        )
    return "{" + ", ".join(items) + "}" if items else "{}"


def _ron_rect(r):
    base = f"x: {int(r['x'])}, y: {int(r['y'])}, w: {int(r['w'])}, h: {int(r['h'])}"
    anchors = r.get("anchors")
    if anchors:
        return f"({base}, anchors: {_ron_anchors(anchors)})"
    return f"({base})"


def _ron_row(row):
    rects = ",\n            ".join(_ron_rect(r) for r in row.get("rects", []))
    return (
        f"(\n"
        f'        animation: "{_ron_escape(row["animation"])}",\n'
        f"        row_index: {int(row['row_index'])},\n"
        f"        frame_count: {int(row['frame_count'])},\n"
        f"        duration_ms: {int(row['duration_ms'])},\n"
        f"        duration_secs: {float(row['duration_secs'])},\n"
        f"        rects: [\n            {rects},\n        ],\n"
        f"    )"
    )


def _emit_sheet_ron(manifest):
    """Serialize the manifest dict to RON in the shape
    `Vec<SheetRecord>` (defined in
    `crates/ambition_sandbox/src/presentation/character_sprites/registry.rs`)
    expects. Even for single-target sheets the top-level is a list — the
    loader always parses `Vec<SheetRecord>`, and shared PNGs (lab props)
    use the same emitter to write multiple records.

    Kept as a hand-rolled emitter (no python-ron dep) because the
    output shape is small, fixed, and easy to inspect in a diff.
    """
    target = manifest["target"]
    return (
        f"// Auto-emitted from {target}_spritesheet.yaml — see\n"
        f"// `presentation::character_sprites::registry`.\n"
        f"[\n"
        f"{_ron_sheet_record(manifest)},\n"
        f"]\n"
    )


def _ron_sheet_record(manifest):
    """Render one `SheetRecord` as RON. Caller wraps in `[...]`.

    A separate helper so callers that need to emit a multi-record list
    (e.g. the lab-props sheet with 8 props on one PNG) can join several
    `_ron_sheet_record(...)` strings with `,\n` between them.
    """
    target = manifest["target"]
    row_entries = list(manifest.get("rows", []))
    if row_entries:
        rows_inner = "\n    ".join(_ron_row(r) + "," for r in row_entries)
        rows_field = f"    rows: [\n    {rows_inner}\n    ],\n"
    else:
        rows_field = "    rows: [],\n"
    y_offset = int(manifest.get("y_offset", 0))
    y_offset_field = f"    y_offset: {y_offset},\n" if y_offset else ""
    return (
        f"(\n"
        f'    target: "{_ron_escape(target)}",\n'
        f'    image: "{_ron_escape(manifest.get("image", f"{target}_spritesheet.png"))}",\n'
        f"    label_width: {int(manifest.get('label_width', 0))},\n"
        f"    frame_width: {int(manifest['frame_width'])},\n"
        f"    frame_height: {int(manifest['frame_height'])},\n"
        f"{y_offset_field}"
        f"    body_metrics: {_ron_body_metrics(manifest.get('body_metrics'))},\n"
        f"{rows_field}"
        f")"
    )


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
