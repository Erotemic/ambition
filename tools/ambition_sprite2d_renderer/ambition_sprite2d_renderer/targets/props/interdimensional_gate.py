from __future__ import annotations

"""Procedural interdimensional portal / gate sprite sheets.

This target intentionally evokes a monumental sci-fi portal ring while staying
well clear of an exact screen-used reproduction. The ring carries the Greek
inscription "ΛΕΓΑΛΛΥ ΔΙΣΤΙΝΧΤ" on plaque-like chevron housings, with Λ anchored
at 12 o'clock. The wormhole / portal membrane is rendered as a separate
transparent overlay sheet so gameplay can layer it over the ring when active.
"""

import math
from pathlib import Path
from typing import List, Sequence, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter, ImageFont

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

TARGET_NAME = "interdimensional_gate"
RING_TARGET = f"{TARGET_NAME}_ring"
PORTAL_TARGET = f"{TARGET_NAME}_portal"

RING_ROWS: List[Tuple[str, int, int]] = [
    ("idle", 8, 140),
    ("spin", 12, 85),
]
PORTAL_ROWS: List[Tuple[str, int, int]] = [
    ("opening", 8, 80),
    ("stable", 8, 110),
    ("closing", 8, 80),
]

SHEET_FILES = [
    f"{RING_TARGET}_spritesheet.png",
    f"{RING_TARGET}_spritesheet.yaml",
    f"{PORTAL_TARGET}_spritesheet.png",
    f"{PORTAL_TARGET}_spritesheet.yaml",
]

# Use a larger canvas than the default character sheets so the gate has more
# room for legible inscription detail and cleaner anti-aliased edges.
FRAME_SIZE = (192, 192)
SUPER = 4
CENTER = (FRAME_SIZE[0] * SUPER / 2.0, FRAME_SIZE[1] * SUPER / 2.0)
OUTER_R = 69.0 * SUPER
INNER_R = 45.0 * SUPER
PORTAL_R = 39.5 * SUPER
RING_MID_R = (INNER_R + OUTER_R) / 2.0
INSCRIPTION = [c for c in "ΛΕΓΑΛΛΥ ΔΙΣΤΙΝΧΤ" if c != " "]
GLYPH_STEP_DEG = 360.0 / len(INSCRIPTION)


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def _ease(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def _downsample(
    img: Image.Image, final_size: Tuple[int, int] = FRAME_SIZE
) -> Image.Image:
    return img.resize(final_size, Image.Resampling.LANCZOS)


def _font(size: int) -> ImageFont.ImageFont:
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            continue
    return ImageFont.load_default()


def _polar(center: Point, radius: float, deg: float) -> Point:
    rad = math.radians(deg)
    return (center[0] + math.cos(rad) * radius, center[1] + math.sin(rad) * radius)


def _ring_bbox(radius: float) -> Tuple[float, float, float, float]:
    return (
        CENTER[0] - radius,
        CENTER[1] - radius,
        CENTER[0] + radius,
        CENTER[1] + radius,
    )


def _paste_center(dst: Image.Image, src: Image.Image, center: Point) -> None:
    x = int(round(center[0] - src.width / 2.0))
    y = int(round(center[1] - src.height / 2.0))
    dst.alpha_composite(src, (x, y))


def _draw_rotated_polygon(
    draw: ImageDraw.ImageDraw,
    center: Point,
    points: Sequence[Point],
    angle_deg: float,
    *,
    fill: RGBA,
    outline: RGBA | None = None,
    width: int = 1,
) -> None:
    rad = math.radians(angle_deg)
    c = math.cos(rad)
    s = math.sin(rad)
    tx, ty = center
    pts = []
    for x, y in points:
        pts.append((tx + x * c - y * s, ty + x * s + y * c))
    draw.polygon(pts, fill=fill, outline=outline)
    if outline is not None:
        draw.line(pts + [pts[0]], fill=outline, width=width, joint="curve")


def _rune_glow(
    frame_index: int, nframes: int, offset: int = 0, *, dim: bool = False
) -> float:
    phase = ((frame_index + offset) / max(1, nframes)) * math.tau
    base = 0.26 if dim else 0.46
    amp = 0.16 if dim else 0.34
    return max(0.0, min(1.0, base + amp * (0.5 + 0.5 * math.sin(phase))))


def _center_layer_to(layer: Image.Image, dest_center: Point) -> Image.Image:
    """Translate ``layer`` so the alpha-bbox center lands on ``dest_center``."""
    alpha = layer.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        return layer
    bx = (bbox[0] + bbox[2]) / 2.0
    by = (bbox[1] + bbox[3]) / 2.0
    dx = int(round(dest_center[0] - bx))
    dy = int(round(dest_center[1] - by))
    shifted = Image.new("RGBA", layer.size, (0, 0, 0, 0))
    shifted.alpha_composite(layer, (dx, dy))
    return shifted


def _make_glyph_chevron(glyph: str, glow_strength: float) -> Image.Image:
    """Build one upright icon tile, then rotate/translate it into place.

    The glyph is rendered upright in its own local layer, centered from its
    alpha bbox both horizontally and vertically, and only then embedded inside a
    slightly oversized chevron icon. Later, the *whole* icon is rotated and
    translated onto the ring.
    """
    tile_size = 40 * SUPER
    tile = Image.new("RGBA", (tile_size, tile_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(tile, "RGBA")
    cx = cy = tile_size / 2.0

    # Slightly bigger border / housing to give the glyph more visual breathing
    # room while keeping the glyph size itself unchanged.
    outer = [
        (-8.35 * SUPER, -5.8 * SUPER),
        (8.35 * SUPER, -5.8 * SUPER),
        (12.7 * SUPER, 2.55 * SUPER),
        (0.0, 12.2 * SUPER),
        (-12.7 * SUPER, 2.55 * SUPER),
    ]
    inner = [
        (-4.9 * SUPER, -3.2 * SUPER),
        (4.9 * SUPER, -3.2 * SUPER),
        (7.7 * SUPER, 1.95 * SUPER),
        (0.0, 8.35 * SUPER),
        (-7.7 * SUPER, 1.95 * SUPER),
    ]
    _draw_rotated_polygon(
        draw,
        (cx, cy),
        outer,
        0.0,
        fill=_rgba("#765f3f"),
        outline=_rgba("#151922"),
        width=4,
    )
    _draw_rotated_polygon(
        draw,
        (cx, cy),
        inner,
        0.0,
        fill=_rgba("#34261a"),
        outline=_rgba("#97784e"),
        width=2,
    )

    font = _font(12 * SUPER)

    # Render the glyph on a temporary layer, then recentre it by its actual
    # alpha bounds so the symbol center sits close to the icon center.
    core_layer = Image.new("RGBA", tile.size, (0, 0, 0, 0))
    core_draw = ImageDraw.Draw(core_layer, "RGBA")
    bbox = font.getbbox(glyph)
    tx = int(round(cx - (bbox[0] + bbox[2]) / 2.0))
    ty = int(round(cy - (bbox[1] + bbox[3]) / 2.0))
    core_color = _rgba("#fff7e1", int(255 * min(1.0, 0.78 + 0.22 * glow_strength)))
    core_draw.text((tx, ty), glyph, font=font, fill=core_color)
    core_layer = _center_layer_to(core_layer, (cx, cy))

    glow_layer = Image.new("RGBA", tile.size, (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow_layer, "RGBA")
    glow_color = _rgba("#ff9f3d", int(245 * glow_strength))
    alpha_bbox = core_layer.getchannel("A").getbbox()
    if alpha_bbox is not None:
        gx = int(round(cx - (alpha_bbox[0] + alpha_bbox[2]) / 2.0))
        gy = int(round(cy - (alpha_bbox[1] + alpha_bbox[3]) / 2.0))
        # Reuse the same nominal origin and recenter again to ensure the glow
        # tracks the core glyph exactly.
        glow_draw.text((tx, ty), glyph, font=font, fill=glow_color)
        glow_layer = _center_layer_to(glow_layer, (cx, cy))
    glow_layer = glow_layer.filter(ImageFilter.GaussianBlur(radius=max(3, SUPER + 1)))
    tile.alpha_composite(glow_layer)
    tile.alpha_composite(core_layer)

    ember = Image.new("RGBA", tile.size, (0, 0, 0, 0))
    ember_draw = ImageDraw.Draw(ember, "RGBA")
    dot_r = 0.95 * SUPER
    ember_draw.ellipse(
        (cx - dot_r, cy + 4.7 * SUPER - dot_r, cx + dot_r, cy + 4.7 * SUPER + dot_r),
        fill=_rgba("#ff9d43", int(170 * glow_strength)),
    )
    tile.alpha_composite(ember)
    return tile


def _draw_gate_ring_base(
    rotation_deg: float, *, frame_index: int, nframes: int, idle_mode: bool
) -> Image.Image:
    img = Image.new(
        "RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    draw = ImageDraw.Draw(img, "RGBA")

    shadow_bbox = (
        CENTER[0] - 50 * SUPER,
        CENTER[1] + 54 * SUPER,
        CENTER[0] + 50 * SUPER,
        CENTER[1] + 66 * SUPER,
    )
    draw.ellipse(shadow_bbox, fill=(0, 0, 0, 56))

    ring = Image.new("RGBA", img.size, (0, 0, 0, 0))
    rd = ImageDraw.Draw(ring, "RGBA")

    rd.ellipse(
        _ring_bbox(OUTER_R), fill=_rgba("#454b57"), outline=_rgba("#1a1e28"), width=12
    )
    rd.ellipse(_ring_bbox(OUTER_R - 5 * SUPER), outline=_rgba("#778091"), width=5)
    rd.ellipse(_ring_bbox(OUTER_R - 9 * SUPER), outline=_rgba("#2c313b"), width=6)
    rd.ellipse(
        _ring_bbox(OUTER_R - 14 * SUPER),
        fill=_rgba("#596170"),
        outline=_rgba("#8790a1"),
        width=3,
    )
    rd.ellipse(
        _ring_bbox(INNER_R + 7 * SUPER),
        fill=_rgba("#353b46"),
        outline=_rgba("#7f8797"),
        width=4,
    )
    rd.ellipse(
        _ring_bbox(INNER_R), fill=(0, 0, 0, 0), outline=_rgba("#151922"), width=8
    )

    for i in range(30):
        ang = rotation_deg + i * (360.0 / 30.0)
        p1 = _polar(CENTER, INNER_R + 4.5 * SUPER, ang)
        p2 = _polar(CENTER, OUTER_R - 4.5 * SUPER, ang)
        rd.line([p1, p2], fill=_rgba("#2a2f39", 115), width=2)

    for i in range(10):
        ang = rotation_deg + i * 36.0 + 6.0
        p1 = _polar(CENTER, INNER_R + 8.0 * SUPER, ang)
        p2 = _polar(CENTER, OUTER_R - 8.0 * SUPER, ang)
        rd.line([p1, p2], fill=_rgba("#98a2b1", 92), width=2)

    # Build each icon upright, then rotate and place it.  Rotation angle is
    # chosen so the bottom of the glyph points toward the centre of the gate.
    for idx, glyph in enumerate(INSCRIPTION):
        ang = rotation_deg - 90.0 + idx * GLYPH_STEP_DEG
        housing_center = _polar(CENTER, RING_MID_R, ang)
        glow_strength = _rune_glow(frame_index, nframes, idx * 2, dim=idle_mode)
        icon = _make_glyph_chevron(glyph, glow_strength)
        rotated_icon = icon.rotate(
            -(ang + 90.0), expand=True, resample=Image.Resampling.BICUBIC
        )
        _paste_center(ring, rotated_icon, housing_center)

    for ang in (-90, 30, 150):
        cap_center = _polar(CENTER, OUTER_R + 4.0 * SUPER, ang)
        _draw_rotated_polygon(
            rd,
            cap_center,
            [
                (-10 * SUPER, -4.5 * SUPER),
                (10 * SUPER, -4.5 * SUPER),
                (8 * SUPER, 5.5 * SUPER),
                (-8 * SUPER, 5.5 * SUPER),
            ],
            ang + 90.0,
            fill=_rgba("#6d7581"),
            outline=_rgba("#1a1e28"),
            width=4,
        )

    ring = ring.filter(ImageFilter.GaussianBlur(radius=0.25))
    img.alpha_composite(ring)

    rim = Image.new("RGBA", img.size, (0, 0, 0, 0))
    rim_draw = ImageDraw.Draw(rim, "RGBA")
    rim_alpha = 78 if idle_mode else 106
    rim_draw.ellipse(
        _ring_bbox(INNER_R + 2.5 * SUPER), outline=_rgba("#ff9d4b", rim_alpha), width=4
    )
    rim_draw.ellipse(
        _ring_bbox(INNER_R + 3.9 * SUPER),
        outline=_rgba("#ffbe73", rim_alpha // 2),
        width=2,
    )
    rim = rim.filter(ImageFilter.GaussianBlur(radius=2.5))
    img.alpha_composite(rim)
    return img


def render_ring_frame(animation: str, frame_index: int, nframes: int) -> Image.Image:
    if animation == "spin":
        rotation = 360.0 * (frame_index / max(1, nframes))
        idle_glow = False
    else:
        rotation = 0.0
        idle_glow = True
    high = _draw_gate_ring_base(
        rotation, frame_index=frame_index, nframes=nframes, idle_mode=idle_glow
    )
    return _downsample(high)


def _portal_mask(radius: float) -> Image.Image:
    mask = Image.new("L", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), 0)
    md = ImageDraw.Draw(mask)
    md.ellipse(_ring_bbox(radius), fill=255)
    return mask


def _draw_energy_ribbons(
    draw: ImageDraw.ImageDraw, t: float, radius: float, count: int
) -> None:
    for ridx in range(count):
        phase = t * math.tau + ridx * 0.9
        points: List[Point] = []
        turns = 24
        for step in range(turns + 1):
            u = step / turns
            ang = phase + u * math.tau * (1.4 + ridx * 0.06)
            rr = (
                radius * (0.20 + 0.75 * u)
                + math.sin(u * math.tau * 3.0 + phase) * (2.0 + ridx * 0.25) * SUPER
            )
            points.append(
                (
                    CENTER[0] + math.cos(ang) * rr,
                    CENTER[1] + math.sin(ang * 1.08) * rr * 0.72,
                )
            )
        color = [
            _rgba("#7ff7ff", 90),
            _rgba("#8a86ff", 70),
            _rgba("#d09cff", 60),
            _rgba("#9df8db", 72),
        ][ridx % 4]
        draw.line(points, fill=color, width=max(1, SUPER + (ridx % 2)), joint="curve")


def _draw_star_specks(
    draw: ImageDraw.ImageDraw, t: float, radius: float, opening: float
) -> None:
    for i in range(16):
        ang = i * 137.5 + t * 180.0
        rr = radius * (0.22 + ((i * 37) % 100) / 120.0)
        x, y = _polar(CENTER, rr, ang)
        spark = 1.1 + ((i * 13) % 7) * 0.22
        alpha = int(_lerp(0, 165, opening) * (0.55 + 0.45 * math.sin(t * math.tau + i)))
        draw.ellipse(
            (
                x - spark * SUPER,
                y - spark * SUPER,
                x + spark * SUPER,
                y + spark * SUPER,
            ),
            fill=(255, 255, 255, max(0, alpha)),
        )


def render_portal_frame(animation: str, frame_index: int, nframes: int) -> Image.Image:
    img = Image.new(
        "RGBA", (FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER), (0, 0, 0, 0)
    )
    t = frame_index / max(1, nframes - 1)
    if animation == "opening":
        strength = _ease(t)
    elif animation == "closing":
        strength = 1.0 - _ease(t)
    else:
        strength = 0.96
    if strength <= 0.001:
        return _downsample(img)

    portal = Image.new("RGBA", img.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(portal, "RGBA")
    radius = PORTAL_R * max(0.14, strength)

    inner_col = _rgba("#75e9ff", int(110 * strength))
    mid_col = _rgba("#5f75ff", int(85 * strength))
    outer_col = _rgba("#c084ff", int(56 * strength))
    draw.ellipse(_ring_bbox(radius), fill=inner_col)
    draw.ellipse(_ring_bbox(radius * 0.82), fill=mid_col)
    draw.ellipse(_ring_bbox(radius * 0.56), fill=outer_col)

    motion_t = t if animation != "stable" else frame_index / max(1, nframes)
    _draw_energy_ribbons(draw, motion_t, radius, 8)
    _draw_star_specks(draw, motion_t, radius, strength)

    rim = Image.new("RGBA", img.size, (0, 0, 0, 0))
    rd = ImageDraw.Draw(rim, "RGBA")
    for i in range(6):
        rr = radius * (0.92 + i * 0.06)
        alpha = int((130 - i * 16) * strength)
        rd.ellipse(
            _ring_bbox(rr),
            outline=_rgba("#8bf1ff", alpha),
            width=max(1, SUPER - i // 2),
        )
    for i in range(18):
        ang = i * (360.0 / 18.0) + t * 70.0
        p1 = _polar(CENTER, radius * 0.86, ang)
        p2 = _polar(
            CENTER,
            radius * (1.02 + 0.10 * math.sin(t * math.tau * 2.0 + i)),
            ang + 6.0 * math.sin(i + t * math.tau),
        )
        rd.line(
            [p1, p2],
            fill=_rgba("#c3f8ff", int(155 * strength)),
            width=max(1, SUPER - 1),
        )
    rim = rim.filter(ImageFilter.GaussianBlur(radius=3.2))
    portal.alpha_composite(rim)

    mask = _portal_mask(PORTAL_R * 1.04)
    clipped = Image.new("RGBA", portal.size, (0, 0, 0, 0))
    clipped.paste(portal, (0, 0), mask)
    img.alpha_composite(clipped)
    return _downsample(img)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    ring_outputs = build_sheet(
        target=RING_TARGET,
        rows=RING_ROWS,
        render_fn=render_ring_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=128,
    )
    portal_outputs = build_sheet(
        target=PORTAL_TARGET,
        rows=PORTAL_ROWS,
        render_fn=render_portal_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=128,
    )
    ordered = [
        ring_outputs["canonical"],
        ring_outputs["canonical_transparent"],
        ring_outputs["spritesheet"],
        ring_outputs["yaml"],
        ring_outputs["preview"],
        portal_outputs["canonical"],
        portal_outputs["canonical_transparent"],
        portal_outputs["spritesheet"],
        portal_outputs["yaml"],
        portal_outputs["preview"],
    ]
    return ordered
