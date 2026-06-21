from __future__ import annotations

"""Procedural intro cart sprite sheet.

A rustic side-view prisoner / travel cart for the opening sequence: heavy wood,
slatted side walls, iron-rimmed wheels, and a forward hitch pole. The look
nods toward a Nordic wagon / prisoner-cart motif while remaining a generic
procedural gameplay asset.
"""

import math
from pathlib import Path
from typing import List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFilter

from ...authoring.tackon_sheet import build_sheet

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "intro_cart"
SHEET_FILES = [f"{TARGET_NAME}_spritesheet.png", f"{TARGET_NAME}_spritesheet.yaml"]

ROWS: List[Tuple[str, int, int]] = [
    ("idle", 6, 145),
    ("roll", 8, 95),
    ("jolt", 6, 85),
]

FRAME_SIZE = (192, 128)
SUPER = 4
W, H = FRAME_SIZE[0] * SUPER, FRAME_SIZE[1] * SUPER


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _s(v: float) -> int:
    return int(round(v * SUPER))


def _pt(x: float, y: float) -> Tuple[int, int]:
    return (_s(x), _s(y))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize(FRAME_SIZE, Image.Resampling.LANCZOS)


def _draw_glow(base: Image.Image, bbox, fill: RGBA, blur: float = 3.0) -> None:
    layer = Image.new("RGBA", base.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer, "RGBA")
    draw.ellipse(bbox, fill=fill)
    layer = layer.filter(ImageFilter.GaussianBlur(radius=blur * SUPER / 2.0))
    base.alpha_composite(layer)


def _draw_wheel(
    draw: ImageDraw.ImageDraw, cx: float, cy: float, radius: float, spoke_angle: float
) -> None:
    rim_dark = _rgba("#281b14")
    rim = _rgba("#4d3424")
    iron = _rgba("#70727a")
    hub = _rgba("#c5a36a")
    draw.ellipse(
        _box(cx - radius, cy - radius, cx + radius, cy + radius),
        fill=rim_dark,
        outline=_rgba("#120c0a"),
        width=_s(1.2),
    )
    draw.ellipse(
        _box(
            cx - radius + 2.2, cy - radius + 2.2, cx + radius - 2.2, cy + radius - 2.2
        ),
        fill=rim,
        outline=iron,
        width=_s(1.3),
    )
    inner_r = radius - 5.5
    for i in range(8):
        ang = spoke_angle + i * (math.tau / 8.0)
        x2 = cx + math.cos(ang) * inner_r
        y2 = cy + math.sin(ang) * inner_r
        draw.line(
            (_s(cx), _s(cy), _s(x2), _s(y2)), fill=_rgba("#8b6d45"), width=_s(1.0)
        )
    draw.ellipse(
        _box(cx - 3.6, cy - 3.6, cx + 3.6, cy + 3.6),
        fill=hub,
        outline=_rgba("#493321"),
        width=_s(0.8),
    )


def _draw_cart(anim: str, frame_idx: int, nframes: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")

    t = frame_idx / max(1, nframes)
    cyc = math.tau * t
    bounce = math.sin(cyc) * (0.7 if anim == "idle" else 1.6)
    sway = math.sin(cyc * 0.5) * (0.4 if anim == "idle" else 0.8)
    wheel_spin = cyc if anim == "roll" else cyc * 0.25
    if anim == "jolt":
        bounce = math.sin(t * math.pi * 3.0) * 2.8
        sway = math.sin(t * math.pi * 2.0) * 1.15
        wheel_spin = cyc * 0.55

    # NO baked drop shadow. Foot alignment is the engine's job —
    # baked shadows shift the alpha bbox bottom away from the true
    # foot of the silhouette, so a cart with a shadow appears to
    # hover above the floor. Cast shadows belong in the ECS visual
    # layer as a separate primitive if a room wants them.

    cart = Image.new("RGBA", img.size, (0, 0, 0, 0))
    cd = ImageDraw.Draw(cart, "RGBA")

    wood_dark = _rgba("#402918")
    wood = _rgba("#7a5432")
    wood_light = _rgba("#9a7248")
    rope = _rgba("#b39668")
    iron = _rgba("#71737c")
    cloth = _rgba("#78543b")
    hay = _rgba("#b49455")

    yoff = bounce
    body_y = 60.0 + yoff
    floor_y = 74.0 + yoff
    front_x = 118.0
    back_x = 39.0
    roof_y = 40.0 + yoff - sway

    # Wheels first (behind body slightly).
    rear_wheel = (54.0, 86.0 + yoff)
    front_wheel = (112.0, 86.0 + yoff)
    _draw_wheel(cd, rear_wheel[0], rear_wheel[1], 16.0, wheel_spin)
    _draw_wheel(cd, front_wheel[0], front_wheel[1], 16.0, wheel_spin + 0.35)

    # Chassis beams.
    cd.rounded_rectangle(
        _box(back_x + 4.0, floor_y - 4.5, front_x - 2.5, floor_y + 2.2),
        radius=_s(1.8),
        fill=wood_dark,
    )
    cd.line(
        _box(back_x + 11.0, floor_y + 1.0, front_x - 3.0, floor_y + 7.0),
        fill=_rgba("#24160f"),
        width=_s(1.0),
    )

    # Floor planks.
    cd.rounded_rectangle(
        _box(back_x, body_y + 4.5, front_x, floor_y),
        radius=_s(3.0),
        fill=wood,
        outline=_rgba("#20140e"),
        width=_s(1.2),
    )
    for x in (46.0, 58.0, 70.0, 82.0, 94.0, 106.0):
        cd.line(_box(x, body_y + 6.0, x, floor_y - 1.0), fill=wood_dark, width=_s(0.7))
    cd.rectangle(
        _box(back_x + 5.0, body_y + 9.0, front_x - 5.0, body_y + 17.0), fill=hay
    )
    for x in range(0, 8):
        hx = 48.0 + x * 8.4
        cd.arc(
            _box(hx, body_y + 8.0 + (x % 2), hx + 11.0, body_y + 18.0 + (x % 2)),
            205,
            345,
            fill=_rgba("#d8be70"),
            width=_s(0.8),
        )

    # Side walls / slats.
    wall_poly = [
        (back_x + 1.0, body_y + 2.0),
        (front_x - 2.0, body_y + 2.0),
        (front_x - 6.0, body_y - 16.0),
        (back_x + 7.0, body_y - 16.0),
    ]
    cd.polygon([_pt(*p) for p in wall_poly], fill=wood_light, outline=_rgba("#21150f"))
    for x in (47.0, 58.0, 69.0, 80.0, 91.0, 102.0, 113.0):
        top_x = x + 2.4
        cd.line(
            _box(x, body_y + 2.0, top_x, body_y - 16.0), fill=wood_dark, width=_s(1.15)
        )
    cd.line(
        _box(back_x + 6.0, body_y - 16.0, front_x - 6.0, body_y - 16.0),
        fill=wood_dark,
        width=_s(1.2),
    )
    cd.line(
        _box(back_x + 2.0, body_y - 6.5, front_x - 2.5, body_y - 6.5),
        fill=wood_dark,
        width=_s(1.0),
    )

    # Back panel.
    back_panel = [
        (back_x + 2.0, body_y + 1.5),
        (back_x + 12.0, body_y - 1.5),
        (back_x + 16.5, body_y - 14.0),
        (back_x + 7.0, body_y - 16.0),
    ]
    cd.polygon(
        [_pt(*p) for p in back_panel], fill=_rgba("#8c653f"), outline=_rgba("#24150d")
    )
    cd.line(
        _box(back_x + 7.5, body_y - 14.0, back_x + 12.0, body_y),
        fill=wood_dark,
        width=_s(1.0),
    )

    # Bench / occupant crossbar.
    cd.rounded_rectangle(
        _box(59.0, body_y - 1.0, 91.0, body_y + 4.5),
        radius=_s(1.6),
        fill=_rgba("#6e4a2c"),
        outline=_rgba("#27180f"),
    )
    cd.line(
        _box(60.0, body_y + 5.0, 60.0, floor_y - 1.5), fill=wood_dark, width=_s(0.9)
    )
    cd.line(
        _box(90.0, body_y + 5.0, 90.0, floor_y - 1.5), fill=wood_dark, width=_s(0.9)
    )

    # Canopy frame / arch.
    posts = [
        ((50.0, body_y - 15.5), (48.0, roof_y)),
        ((107.0, body_y - 15.5), (109.0, roof_y + 1.5)),
    ]
    for (x1, y1), (x2, y2) in posts:
        cd.line(_box(x1, y1, x2, y2), fill=_rgba("#5e4127"), width=_s(1.1))
    arch = [
        (48.0, roof_y),
        (61.0, roof_y - 7.5 - sway * 0.6),
        (80.0, roof_y - 10.5 - sway * 0.2),
        (96.0, roof_y - 8.0 + sway * 0.2),
        (109.0, roof_y + 1.5),
    ]
    cd.line(
        [_pt(*p) for p in arch], fill=_rgba("#5e4127"), width=_s(1.15), joint="curve"
    )

    # Simple leather / cloth roof roll.
    roof_poly = [
        (49.0, roof_y + 2.0),
        (59.0, roof_y - 4.0),
        (81.0, roof_y - 7.5),
        (101.0, roof_y - 4.5),
        (108.0, roof_y + 1.5),
        (102.0, roof_y + 6.0),
        (56.0, roof_y + 6.0),
    ]
    cd.polygon([_pt(*p) for p in roof_poly], fill=cloth, outline=_rgba("#2c1d14"))
    cd.arc(
        _box(56.0, roof_y - 2.0, 98.0, roof_y + 8.5),
        190,
        350,
        fill=_rgba("#967255"),
        width=_s(0.8),
    )

    # Rope bindings.
    for x in (57.0, 70.0, 87.0, 99.0):
        cd.line(_box(x, roof_y - 0.5, x, roof_y + 6.5), fill=rope, width=_s(0.55))

    # Forward hitch pole and yoke extension.
    pole = [(front_x - 1.0, floor_y - 2.0), (157.0, 72.0 + yoff - sway * 0.2)]
    cd.line([_pt(*p) for p in pole], fill=_rgba("#6b472c"), width=_s(1.5))
    cd.line(
        _box(154.0, 69.5 + yoff, 165.0, 66.0 + yoff),
        fill=_rgba("#6b472c"),
        width=_s(1.1),
    )
    cd.line(
        _box(154.0, 74.5 + yoff, 165.0, 80.0 + yoff),
        fill=_rgba("#6b472c"),
        width=_s(1.1),
    )

    # Small hanging lantern near the front to help the intro read at night.
    lantern_x = 119.0
    lantern_y = body_y - 12.0 + sway
    cd.line(
        _box(lantern_x, body_y - 16.0, lantern_x, lantern_y - 4.0),
        fill=iron,
        width=_s(0.6),
    )
    cd.rounded_rectangle(
        _box(lantern_x - 3.2, lantern_y - 3.0, lantern_x + 3.2, lantern_y + 5.2),
        radius=_s(1.1),
        fill=_rgba("#59452a"),
        outline=_rgba("#201812"),
        width=_s(0.7),
    )
    _draw_glow(
        img,
        _box(lantern_x - 4.0, lantern_y - 0.5, lantern_x + 4.0, lantern_y + 7.5),
        _rgba("#ffb14a", 110),
        blur=3.8,
    )
    cd.ellipse(
        _box(lantern_x - 1.8, lantern_y + 0.4, lantern_x + 1.8, lantern_y + 4.4),
        fill=_rgba("#ffd580", 200),
    )

    # Front shield plate for a slightly more fortified silhouette.
    nose = [
        (116.0, body_y + 2.0),
        (126.0, body_y - 2.0),
        (126.0, body_y - 14.0),
        (116.0, body_y - 17.0),
    ]
    cd.polygon([_pt(*p) for p in nose], fill=_rgba("#8a633d"), outline=_rgba("#2c1b10"))

    # Reins / hanging strap shapes.
    cd.arc(
        _box(107.0, 60.0 + yoff, 132.0, 82.0 + yoff),
        280,
        30,
        fill=_rgba("#5a3f29"),
        width=_s(0.75),
    )

    # Composite and downsample.
    cart = cart.filter(ImageFilter.GaussianBlur(radius=0.15))
    img.alpha_composite(cart)
    return _downsample(img)


def render_frame(animation: str, frame_idx: int, nframes: int) -> Image.Image:
    return _draw_cart(animation, frame_idx, nframes)


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    outputs = build_sheet(
        target=TARGET_NAME,
        rows=ROWS,
        render_fn=render_frame,
        out_dir=out_dir,
        frame_size=FRAME_SIZE,
        label_width=112,
    )
    return [
        outputs["canonical"],
        outputs["canonical_transparent"],
        outputs["spritesheet"],
        outputs["yaml"],
        outputs["preview"],
    ]
