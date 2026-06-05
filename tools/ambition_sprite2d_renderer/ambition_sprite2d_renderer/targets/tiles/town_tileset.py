from __future__ import annotations

"""Procedural side-scroller town tileset target.

This target replaces the earlier top-down town pass with a side-view town /
settlement construction atlas. The atlas is designed for facade-building and
platforming use cases: house exteriors, roofs, wall materials, doors, windows,
trim, stairs, balconies, supports, and town props.

Output:
- town_tileset.png: 96-tile atlas (8 columns x 12 rows)
- town_tileset.yaml: manifest with tile names, groups, and atlas coordinates
- town_tileset_contact_sheet.png: labeled visual review sheet
"""

import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Sequence, Tuple

import yaml
from PIL import Image, ImageDraw, ImageFont

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "town_tileset"
SHEET_FILES = ("town_tileset.png", "town_tileset.yaml")
CONTACT_FILE = "town_tileset_contact_sheet.png"

# DESIGN_TILE is the logical px the tile draw functions assume —
# coordinates and feature sizes were authored against 64×64 cells.
# OUTPUT_TILE is the final cell size in the published PNG / atlas;
# we downsample DESIGN_TILE → OUTPUT_TILE at the end. This split
# lets the world author tiles on a 16-px grid (matching the
# Collision IntGrid) without rewriting every tile drawing.
DESIGN_TILE = 64
OUTPUT_TILE = 16
TILE = DESIGN_TILE  # alias used by the draw functions
SCALE = 4
ATLAS_COLUMNS = 8
CONTACT_COLUMNS = 4


@dataclass(frozen=True)
class TileSpec:
    key: str
    display_name: str
    category: str
    description: str
    layer: str
    style: str
    params: Dict[str, Any]
    tags: Tuple[str, ...] = ()


# ---------------------------------------------------------------------------
# low-level helpers


def _s(v: float) -> int:
    return int(round(v * SCALE))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _rgba(hex_color: str, alpha: int = 255) -> RGBA:
    hex_color = hex_color.lstrip("#")
    return (
        int(hex_color[0:2], 16),
        int(hex_color[2:4], 16),
        int(hex_color[4:6], 16),
        alpha,
    )


def _mix(c1: RGBA, c2: RGBA, t: float, alpha: int | None = None) -> RGBA:
    t = max(0.0, min(1.0, t))
    a = alpha if alpha is not None else int(round(c1[3] * (1 - t) + c2[3] * t))
    return (
        int(round(c1[0] * (1 - t) + c2[0] * t)),
        int(round(c1[1] * (1 - t) + c2[1] * t)),
        int(round(c1[2] * (1 - t) + c2[2] * t)),
        a,
    )


def _font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=size)
        except OSError:
            pass
    return ImageFont.load_default()


def _outline_text(
    draw: ImageDraw.ImageDraw,
    xy: Tuple[int, int],
    text: str,
    *,
    font,
    fill: RGBA,
    outline: RGBA,
) -> None:
    x, y = xy
    for ox in (-1, 0, 1):
        for oy in (-1, 0, 1):
            if ox == 0 and oy == 0:
                continue
            draw.text((x + ox, y + oy), text, font=font, fill=outline)
    draw.text((x, y), text, font=font, fill=fill)


def _tile_canvas(transparent: bool = False) -> Image.Image:
    bg = (0, 0, 0, 0) if transparent else (0, 0, 0, 0)
    return Image.new("RGBA", (_s(TILE), _s(TILE)), bg)


def _tile_downsample(img: Image.Image) -> Image.Image:
    return img.resize((OUTPUT_TILE, OUTPUT_TILE), Image.Resampling.LANCZOS)


def _shadow(
    draw: ImageDraw.ImageDraw,
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    alpha: int = 40,
) -> None:
    draw.ellipse(_box(x1, y1, x2, y2), fill=(0, 0, 0, alpha))


# ---------------------------------------------------------------------------
# texture primitives


def _stone_pattern(
    draw: ImageDraw.ImageDraw, *, base: RGBA, dark: RGBA, light: RGBA
) -> None:
    draw.rectangle(_box(0, 0, TILE, TILE), fill=base)
    stones = [
        (4, 8, 12, 9),
        (18, 6, 13, 10),
        (35, 7, 15, 8),
        (52, 8, 9, 9),
        (6, 20, 15, 10),
        (23, 19, 13, 11),
        (39, 20, 16, 10),
        (4, 34, 14, 11),
        (20, 33, 17, 10),
        (40, 34, 12, 11),
        (54, 33, 8, 12),
        (7, 48, 13, 9),
        (24, 47, 15, 10),
        (42, 47, 14, 10),
    ]
    for x, y, w, h in stones:
        fill = light if ((x + y) // 3) % 2 == 0 else _mix(base, dark, 0.35)
        draw.rounded_rectangle(
            _box(x, y, x + w, y + h),
            radius=_s(1.8),
            fill=fill,
            outline=dark,
            width=_s(0.7),
        )
        draw.line(
            [(_s(x + 1.5), _s(y + 2)), (_s(x + w - 2), _s(y + 2))],
            fill=_rgba("FFFFFF", 70),
            width=_s(0.5),
        )


def _brick_pattern(
    draw: ImageDraw.ImageDraw, *, base: RGBA, mortar: RGBA, light: RGBA, offset: int = 0
) -> None:
    draw.rectangle(_box(0, 0, TILE, TILE), fill=mortar)
    brick_w = 14
    brick_h = 8
    for row, y in enumerate(range(0, TILE, brick_h)):
        shift = offset if row % 2 else 0
        for x in range(-shift, TILE + brick_w, brick_w):
            bx1 = x + 1
            bx2 = x + brick_w - 1
            by1 = y + 1
            by2 = y + brick_h - 1
            fill = _mix(base, light, 0.15 if (row + x // brick_w) % 2 == 0 else 0.04)
            draw.rectangle(
                _box(bx1, by1, bx2, by2),
                fill=fill,
                outline=_mix(base, mortar, 0.45),
                width=_s(0.45),
            )
            draw.line(
                [(_s(bx1 + 1), _s(by1 + 1)), (_s(bx2 - 2), _s(by1 + 1))],
                fill=_rgba("FFFFFF", 40),
                width=_s(0.45),
            )


def _plaster_pattern(
    draw: ImageDraw.ImageDraw, *, base: RGBA, shade: RGBA, speck: RGBA
) -> None:
    draw.rectangle(_box(0, 0, TILE, TILE), fill=base)
    for y in range(0, TILE, 10):
        draw.line(
            [(_s(0), _s(y + 2)), (_s(TILE), _s(y + 2))],
            fill=_mix(base, shade, 0.18, 55),
            width=_s(0.55),
        )
    for x, y, w, h in (
        (8, 11, 6, 4),
        (22, 18, 4, 3),
        (39, 12, 5, 4),
        (49, 24, 6, 4),
        (16, 41, 6, 5),
        (44, 48, 5, 4),
    ):
        draw.ellipse(_box(x, y, x + w, y + h), fill=speck)


def _grass_pattern(draw: ImageDraw.ImageDraw, *, flowers: bool = False) -> None:
    base = _rgba("6AA45D")
    light = _rgba("85C16D")
    dark = _rgba("477A40")
    draw.rectangle(_box(0, 0, TILE, TILE), fill=base)
    for y in range(0, TILE, 8):
        for x in range(0, TILE, 8):
            if ((x // 8) + (y // 8)) % 2 == 0:
                draw.rectangle(_box(x, y, x + 8, y + 8), fill=_mix(base, light, 0.08))
    for y in range(5, TILE, 9):
        for x in range(2 + (y % 5), TILE, 10):
            draw.line(
                [(_s(x), _s(y + 3)), (_s(x + 1), _s(y))], fill=dark, width=_s(0.8)
            )
            draw.line(
                [(_s(x + 3), _s(y + 4)), (_s(x + 2), _s(y + 1))],
                fill=light,
                width=_s(0.7),
            )
    for x, y in ((10, 16), (22, 40), (47, 14), (52, 46), (32, 29)):
        draw.ellipse(_box(x, y, x + 4, y + 3), fill=_rgba("7A926F", 110))
    if flowers:
        for cx, cy, color in (
            (12, 15, _rgba("FFD8EF")),
            (27, 42, _rgba("FFF0A7")),
            (47, 18, _rgba("DAF2FF")),
            (53, 40, _rgba("F6D4AA")),
            (18, 55, _rgba("EFD5FF")),
        ):
            for ox, oy in ((0, -1.6), (1.6, 0), (0, 1.6), (-1.6, 0)):
                draw.ellipse(
                    _box(cx + ox - 1.2, cy + oy - 1.2, cx + ox + 1.2, cy + oy + 1.2),
                    fill=color,
                )
            draw.ellipse(
                _box(cx - 0.9, cy - 0.9, cx + 0.9, cy + 0.9), fill=_rgba("FFFCE8")
            )


def _dirt_pattern(draw: ImageDraw.ImageDraw, *, base: RGBA | None = None) -> None:
    base = base or _rgba("9F7A55")
    draw.rectangle(_box(0, 0, TILE, TILE), fill=base)
    for y in range(0, TILE, 8):
        for x in range(0, TILE, 8):
            fill = (
                _mix(base, _rgba("C8A27C"), 0.18)
                if ((x // 8 + y // 8) % 2 == 0)
                else _mix(base, _rgba("765741"), 0.14)
            )
            draw.rectangle(_box(x, y, x + 8, y + 8), fill=fill)
    for x, y, w, h in (
        (7, 12, 4, 3),
        (23, 33, 5, 4),
        (42, 22, 4, 3),
        (48, 50, 4, 3),
        (15, 55, 4, 3),
        (35, 8, 3, 2),
    ):
        draw.ellipse(_box(x, y, x + w, y + h), fill=_rgba("6B5341", 150))


def _roof_pattern(
    draw: ImageDraw.ImageDraw, *, base: RGBA, light: RGBA, dark: RGBA
) -> None:
    draw.rectangle(_box(0, 0, TILE, TILE), fill=dark)
    row_h = 8
    for row, y in enumerate(range(0, TILE + row_h, row_h)):
        offset = 0 if row % 2 == 0 else 6
        for x in range(-offset, TILE + 12, 12):
            pts = [
                (_s(x), _s(y + row_h)),
                (_s(x + 6), _s(y)),
                (_s(x + 12), _s(y + row_h)),
                (_s(x + 6), _s(y + row_h + 3)),
            ]
            fill = light if ((x // 12 + row) % 2 == 0) else base
            draw.polygon(pts, fill=fill, outline=dark)


def _wall_shadow(draw: ImageDraw.ImageDraw, *, right: bool = False) -> None:
    if right:
        draw.rectangle(_box(54, 0, 64, 64), fill=_rgba("000000", 28))
    else:
        draw.rectangle(_box(0, 0, 10, 64), fill=_rgba("000000", 20))


# ---------------------------------------------------------------------------
# shared facade bases


def _plaster_wall_base(draw: ImageDraw.ImageDraw, *, tone: str = "warm") -> None:
    if tone == "warm":
        base = _rgba("E6D9C6")
        shade = _rgba("C8B79E")
        speck = _rgba("F0E5D8", 80)
        stone = _rgba("988B73")
        stone2 = _rgba("837760")
    else:
        base = _rgba("D7E0E6")
        shade = _rgba("B3C0CA")
        speck = _rgba("EDF4F8", 80)
        stone = _rgba("93A0AA")
        stone2 = _rgba("78848D")
    _plaster_pattern(draw, base=base, shade=shade, speck=speck)
    draw.rectangle(_box(0, 54, TILE, TILE), fill=stone)
    for x in range(0, TILE, 10):
        fill = stone if (x // 10) % 2 == 0 else stone2
        draw.rectangle(
            _box(x, 54, x + 10, 64),
            fill=fill,
            outline=_mix(stone2, stone, 0.5),
            width=_s(0.4),
        )


def _timber_wall_base(draw: ImageDraw.ImageDraw, *, infill: RGBA | None = None) -> None:
    infill = infill or _rgba("E7D9C3")
    timber = _rgba("6B4A33")
    dark = _rgba("493121")
    draw.rectangle(_box(0, 0, TILE, TILE), fill=infill)
    for x in (0, 20, 44, 62):
        draw.rectangle(_box(x, 0, x + 4, TILE), fill=timber)
    for y in (0, 22, 44, 62):
        draw.rectangle(_box(0, y, TILE, y + 4), fill=timber)
    draw.line([(_s(20), _s(22)), (_s(44), _s(0))], fill=timber, width=_s(3))
    draw.line([(_s(20), _s(44)), (_s(44), _s(22))], fill=timber, width=_s(3))
    draw.line([(_s(20), _s(22)), (_s(44), _s(44))], fill=timber, width=_s(3))
    draw.rectangle(_box(0, 54, TILE, TILE), fill=_rgba("7E715D"))
    draw.rectangle(_box(0, 54, TILE, 56), fill=dark)


def _brick_wall_base(draw: ImageDraw.ImageDraw, *, color: str = "red") -> None:
    if color == "red":
        base = _rgba("AA5A48")
        mortar = _rgba("C8B8A7")
        light = _rgba("C97762")
        trim = _rgba("855142")
    else:
        base = _rgba("85738D")
        mortar = _rgba("CFC8D4")
        light = _rgba("9C8BA8")
        trim = _rgba("625467")
    _brick_pattern(draw, base=base, mortar=mortar, light=light, offset=7)
    draw.rectangle(_box(0, 54, TILE, 64), fill=trim)
    draw.rectangle(_box(0, 52, TILE, 54), fill=_mix(trim, _rgba("FFFFFF"), 0.18))


# ---------------------------------------------------------------------------
# tile renderers


def _render_terrain(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    if variant == "grass_top":
        _dirt_pattern(d)
        _grass_pattern(d)
        d.rectangle(_box(0, 18, TILE, TILE), fill=(0, 0, 0, 0))
        _dirt_pattern(d, base=_rgba("9F7A55"))
        d.rectangle(_box(0, 0, TILE, 22), fill=(0, 0, 0, 0))
        _grass_pattern(d)
        d.rectangle(_box(0, 22, TILE, TILE), fill=(0, 0, 0, 0))
        # simpler explicit repaint
        _dirt_pattern(d)
        d.rectangle(_box(0, 0, TILE, 22), fill=(0, 0, 0, 0))
        _grass_pattern(d)
        draw_grass = ImageDraw.Draw(img, "RGBA")
        draw_grass.rectangle(_box(0, 22, TILE, TILE), fill=(0, 0, 0, 0))
        # build final top cap
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.rectangle(_box(0, 0, TILE, 22), fill=255)
        md.polygon(
            [
                (_s(0), _s(22)),
                (_s(9), _s(18)),
                (_s(21), _s(22)),
                (_s(30), _s(18)),
                (_s(43), _s(22)),
                (_s(52), _s(18)),
                (_s(64), _s(22)),
                (_s(64), _s(0)),
                (_s(0), _s(0)),
            ],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    elif variant == "grass_fill":
        _dirt_pattern(d)
    elif variant == "grass_left":
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.rectangle(_box(0, 0, 22, 28), fill=255)
        md.polygon(
            [
                (_s(0), _s(28)),
                (_s(6), _s(24)),
                (_s(12), _s(26)),
                (_s(18), _s(24)),
                (_s(22), _s(28)),
                (_s(22), _s(0)),
                (_s(0), _s(0)),
            ],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    elif variant == "grass_right":
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.rectangle(_box(42, 0, 64, 28), fill=255)
        md.polygon(
            [
                (_s(42), _s(28)),
                (_s(46), _s(24)),
                (_s(52), _s(26)),
                (_s(58), _s(24)),
                (_s(64), _s(28)),
                (_s(64), _s(0)),
                (_s(42), _s(0)),
            ],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    elif variant == "slope_up":
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.polygon(
            [(_s(0), _s(22)), (_s(64), _s(0)), (_s(64), _s(16)), (_s(0), _s(38))],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
        # dirt body under slope
        dd = ImageDraw.Draw(img, "RGBA")
        dd.polygon(
            [(_s(0), _s(22)), (_s(64), _s(0)), (_s(64), _s(64)), (_s(0), _s(64))],
            fill=_rgba("9F7A55", 0),
            outline=None,
        )
    elif variant == "slope_down":
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.polygon(
            [(_s(0), _s(0)), (_s(64), _s(22)), (_s(64), _s(38)), (_s(0), _s(16))],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    elif variant == "grass_flowers":
        _dirt_pattern(d)
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd, flowers=True)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.rectangle(_box(0, 0, TILE, 24), fill=255)
        md.polygon(
            [
                (_s(0), _s(24)),
                (_s(9), _s(20)),
                (_s(21), _s(24)),
                (_s(30), _s(20)),
                (_s(43), _s(24)),
                (_s(52), _s(20)),
                (_s(64), _s(24)),
                (_s(64), _s(0)),
                (_s(0), _s(0)),
            ],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    elif variant == "grass_cliff":
        _dirt_pattern(d, base=_rgba("8B6C4E"))
        stone = _rgba("837261")
        for x, y, w, h in (
            (6, 25, 9, 6),
            (19, 35, 10, 7),
            (35, 28, 11, 6),
            (49, 40, 9, 6),
            (24, 49, 12, 7),
        ):
            d.rounded_rectangle(
                _box(x, y, x + w, y + h),
                radius=_s(1.4),
                fill=stone,
                outline=_rgba("5F5446"),
                width=_s(0.5),
            )
        cap = Image.new("RGBA", img.size, (0, 0, 0, 0))
        cd = ImageDraw.Draw(cap, "RGBA")
        _grass_pattern(cd)
        mask = Image.new("L", img.size, 0)
        md = ImageDraw.Draw(mask)
        md.rectangle(_box(0, 0, TILE, 20), fill=255)
        md.polygon(
            [
                (_s(0), _s(20)),
                (_s(10), _s(18)),
                (_s(18), _s(22)),
                (_s(28), _s(18)),
                (_s(40), _s(22)),
                (_s(50), _s(19)),
                (_s(64), _s(20)),
                (_s(64), _s(0)),
                (_s(0), _s(0)),
            ],
            fill=255,
        )
        img = Image.composite(cap, img, mask)
    return _tile_downsample(img)


def _render_foundation(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    if variant == "cobble_walk":
        _stone_pattern(
            d, base=_rgba("8C949A"), dark=_rgba("5D646B"), light=_rgba("AAB2BA")
        )
    elif variant == "cobble_border":
        _stone_pattern(
            d, base=_rgba("8C949A"), dark=_rgba("5D646B"), light=_rgba("AAB2BA")
        )
        d.rectangle(_box(0, 0, TILE, 10), fill=_rgba("CDBEA3"))
        d.line(
            [(_s(0), _s(10)), (_s(TILE), _s(10))], fill=_rgba("6B6258"), width=_s(1.0)
        )
    elif variant == "stone_foundation":
        _stone_pattern(
            d, base=_rgba("878179"), dark=_rgba("58524B"), light=_rgba("AAA39A")
        )
        d.rectangle(_box(0, 0, TILE, 6), fill=_rgba("6A645E"))
    elif variant == "stone_foundation_cracked":
        _stone_pattern(
            d, base=_rgba("878179"), dark=_rgba("58524B"), light=_rgba("AAA39A")
        )
        d.line(
            [
                (_s(20), _s(13)),
                (_s(16), _s(26)),
                (_s(23), _s(34)),
                (_s(14), _s(46)),
                (_s(19), _s(60)),
            ],
            fill=_rgba("342F2A"),
            width=_s(1.1),
        )
        d.line(
            [(_s(23), _s(34)), (_s(36), _s(41)), (_s(49), _s(53))],
            fill=_rgba("342F2A"),
            width=_s(0.8),
        )
    elif variant == "brick_pavers":
        _brick_pattern(
            d,
            base=_rgba("A46C56"),
            mortar=_rgba("D1C1B5"),
            light=_rgba("C4876D"),
            offset=7,
        )
    elif variant == "boardwalk":
        d.rectangle(_box(0, 0, TILE, TILE), fill=_rgba("8A623E"))
        for x in range(0, TILE, 10):
            fill = _rgba("996B43") if (x // 10) % 2 == 0 else _rgba("7B5537")
            d.rectangle(
                _box(x, 0, x + 10, TILE),
                fill=fill,
                outline=_rgba("5D3E28"),
                width=_s(0.5),
            )
            d.line(
                [(_s(x + 2), _s(6)), (_s(x + 2), _s(TILE - 6))],
                fill=_rgba("C49262", 90),
                width=_s(0.4),
            )
    elif variant == "stoop_steps":
        d.rectangle(_box(0, 0, TILE, TILE), fill=(0, 0, 0, 0))
        for idx, y in enumerate((40, 48, 56)):
            d.rectangle(
                _box(10 + idx * 4, y, 54 - idx * 4, y + 8),
                fill=_rgba("8E8A84"),
                outline=_rgba("5A5652"),
                width=_s(0.8),
            )
            d.line(
                [(_s(10 + idx * 4), _s(y + 2)), (_s(54 - idx * 4), _s(y + 2))],
                fill=_rgba("D4D0CA", 80),
                width=_s(0.45),
            )
        _shadow(d, 10, 55, 54, 61, 26)
    elif variant == "drain_grate":
        _stone_pattern(
            d, base=_rgba("80868C"), dark=_rgba("535960"), light=_rgba("9DA6AF")
        )
        d.rounded_rectangle(
            _box(16, 16, 48, 48),
            radius=_s(2.5),
            fill=_rgba("464B50"),
            outline=_rgba("222527"),
            width=_s(0.8),
        )
        for x in range(20, 48, 5):
            d.line(
                [(_s(x), _s(18)), (_s(x), _s(46))], fill=_rgba("99A3AA"), width=_s(0.8)
            )
        d.line(
            [(_s(18), _s(23)), (_s(46), _s(23))], fill=_rgba("99A3AA"), width=_s(0.8)
        )
        d.line(
            [(_s(18), _s(41)), (_s(46), _s(41))], fill=_rgba("99A3AA"), width=_s(0.8)
        )
    return _tile_downsample(img)


def _render_plaster_wall(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    tone = spec.params.get("tone", "warm")
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _plaster_wall_base(d, tone=tone)
    trim = _rgba("B5956A") if tone == "warm" else _rgba("8AA0B2")
    dark = _rgba("6A5038") if tone == "warm" else _rgba("60707E")
    if variant == "plain":
        pass
    elif variant == "top_trim":
        d.rectangle(_box(0, 0, TILE, 10), fill=trim, outline=dark, width=_s(0.8))
        d.line(
            [(_s(0), _s(8)), (_s(TILE), _s(8))], fill=_rgba("FFFFFF", 60), width=_s(0.5)
        )
    elif variant == "bottom_trim":
        d.rectangle(_box(0, 48, TILE, 54), fill=trim, outline=dark, width=_s(0.8))
    elif variant == "left":
        d.rectangle(_box(0, 0, 10, 54), fill=trim, outline=dark, width=_s(0.8))
        _wall_shadow(d, right=False)
    elif variant == "right":
        d.rectangle(_box(54, 0, 64, 54), fill=trim, outline=dark, width=_s(0.8))
        _wall_shadow(d, right=True)
    elif variant == "cracked":
        d.line(
            [
                (_s(22), _s(10)),
                (_s(18), _s(24)),
                (_s(26), _s(32)),
                (_s(17), _s(42)),
                (_s(21), _s(54)),
            ],
            fill=_rgba("5B4A3B"),
            width=_s(1.0),
        )
        d.line(
            [(_s(26), _s(32)), (_s(36), _s(38)), (_s(46), _s(47))],
            fill=_rgba("5B4A3B"),
            width=_s(0.7),
        )
    elif variant == "notice":
        d.rectangle(
            _box(16, 16, 47, 38),
            fill=_rgba("E9D8B8"),
            outline=_rgba("755F42"),
            width=_s(0.8),
        )
        d.line(
            [(_s(21), _s(22)), (_s(43), _s(22))], fill=_rgba("866E50"), width=_s(0.7)
        )
        d.line(
            [(_s(21), _s(27)), (_s(40), _s(27))], fill=_rgba("866E50"), width=_s(0.7)
        )
        d.line(
            [(_s(21), _s(32)), (_s(44), _s(32))], fill=_rgba("866E50"), width=_s(0.7)
        )
        d.ellipse(_box(19, 18, 22, 21), fill=_rgba("A43E33"))
        d.ellipse(_box(41, 18, 44, 21), fill=_rgba("A43E33"))
    elif variant == "arch":
        d.rounded_rectangle(
            _box(18, 12, 46, 48),
            radius=_s(10),
            fill=_rgba("D6C6B2"),
            outline=dark,
            width=_s(1.0),
        )
        d.rectangle(_box(22, 16, 42, 48), fill=_rgba("CBBCA9"))
        d.rectangle(_box(0, 48, TILE, 54), fill=trim, outline=dark, width=_s(0.8))
    return _tile_downsample(img)


def _render_timber_wall(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _timber_wall_base(d)
    timber = _rgba("6B4A33")
    if variant == "plain":
        pass
    elif variant == "cross":
        d.line([(_s(0), _s(0)), (_s(TILE), _s(TILE))], fill=timber, width=_s(3.0))
        d.line([(_s(TILE), _s(0)), (_s(0), _s(TILE))], fill=timber, width=_s(3.0))
    elif variant == "vertical":
        for x in (14, 32, 50):
            d.rectangle(_box(x, 0, x + 4, 54), fill=timber)
    elif variant == "horizontal":
        for y in (12, 28, 44):
            d.rectangle(_box(0, y, 64, y + 4), fill=timber)
    elif variant == "left":
        d.rectangle(_box(0, 0, 12, 54), fill=timber)
        _wall_shadow(d, right=False)
    elif variant == "right":
        d.rectangle(_box(52, 0, 64, 54), fill=timber)
        _wall_shadow(d, right=True)
    elif variant == "upper":
        d.rectangle(_box(0, 0, 64, 10), fill=timber)
        d.line(
            [(_s(0), _s(10)), (_s(TILE), _s(10))],
            fill=_rgba("FFFFFF", 35),
            width=_s(0.5),
        )
    elif variant == "sign":
        d.rounded_rectangle(
            _box(18, 16, 46, 31),
            radius=_s(2.2),
            fill=_rgba("91663D"),
            outline=_rgba("4F321A"),
            width=_s(1.0),
        )
        d.line(
            [(_s(23), _s(22)), (_s(41), _s(22))], fill=_rgba("E6D5B7"), width=_s(0.8)
        )
        d.line(
            [(_s(23), _s(26)), (_s(38), _s(26))], fill=_rgba("E6D5B7"), width=_s(0.8)
        )
    return _tile_downsample(img)


def _render_brick_wall(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    color = spec.params.get("color", "red")
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _brick_wall_base(d, color=color)
    trim = _rgba("855142") if color == "red" else _rgba("625467")
    light_trim = _rgba("B37460") if color == "red" else _rgba("86769A")
    if variant == "plain":
        pass
    elif variant == "top_trim":
        d.rectangle(_box(0, 0, 64, 10), fill=trim)
        d.rectangle(_box(0, 8, 64, 10), fill=light_trim)
    elif variant == "bottom_trim":
        d.rectangle(_box(0, 48, 64, 54), fill=trim)
    elif variant == "left":
        d.rectangle(_box(0, 0, 10, 54), fill=trim)
        _wall_shadow(d, right=False)
    elif variant == "right":
        d.rectangle(_box(54, 0, 64, 54), fill=trim)
        _wall_shadow(d, right=True)
    elif variant == "arched":
        d.rounded_rectangle(
            _box(16, 12, 48, 48),
            radius=_s(10),
            fill=_rgba("CAB6AB"),
            outline=trim,
            width=_s(1.0),
        )
        d.rectangle(_box(20, 20, 44, 48), fill=_rgba("BAA89A"))
    elif variant == "cracked":
        d.line(
            [
                (_s(19), _s(12)),
                (_s(16), _s(24)),
                (_s(24), _s(32)),
                (_s(15), _s(45)),
                (_s(19), _s(54)),
            ],
            fill=_rgba("3E2B25"),
            width=_s(1.0),
        )
        d.line(
            [(_s(24), _s(32)), (_s(36), _s(39)), (_s(48), _s(46))],
            fill=_rgba("3E2B25"),
            width=_s(0.7),
        )
    elif variant == "vines":
        vine = _rgba("4D7A48")
        for pts in (
            ((8, 6), (14, 14), (12, 25), (18, 36), (17, 53)),
            ((43, 0), (48, 10), (46, 20), (53, 32), (50, 54)),
        ):
            d.line([(_s(x), _s(y)) for x, y in pts], fill=vine, width=_s(1.2))
        for cx, cy in ((13, 14), (11, 25), (17, 36), (48, 10), (46, 20), (53, 32)):
            d.ellipse(_box(cx - 2, cy - 1, cx + 2, cy + 1), fill=_rgba("7FA85B"))
    return _tile_downsample(img)


def _window_frame(
    draw: ImageDraw.ImageDraw,
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    *,
    shutter: bool = False,
    planter: bool = False,
    round_top: bool = False,
    arch: bool = False,
    lattice: bool = False,
    balcony: bool = False,
) -> None:
    frame = _rgba("C7B08D")
    outline = _rgba("6C573B")
    glass = _rgba("6EA8C5")
    hi = _rgba("D8F2FF", 130)
    if arch:
        draw.rounded_rectangle(
            _box(x1, y1, x2, y2),
            radius=_s(6),
            fill=frame,
            outline=outline,
            width=_s(0.9),
        )
        inset = (x1 + 3, y1 + 4, x2 - 3, y2 - 4)
        draw.rounded_rectangle(
            _box(*inset),
            radius=_s(4.6),
            fill=glass,
            outline=_rgba("396079"),
            width=_s(0.8),
        )
    elif round_top:
        draw.rounded_rectangle(
            _box(x1, y1, x2, y2),
            radius=_s(8),
            fill=frame,
            outline=outline,
            width=_s(0.9),
        )
        draw.ellipse(
            _box(x1 + 3, y1 + 3, x2 - 3, y1 + 16),
            fill=glass,
            outline=_rgba("396079"),
            width=_s(0.8),
        )
        draw.rectangle(
            _box(x1 + 3, y1 + 11, x2 - 3, y2 - 3),
            fill=glass,
            outline=_rgba("396079"),
            width=_s(0.8),
        )
    else:
        draw.rounded_rectangle(
            _box(x1, y1, x2, y2),
            radius=_s(2),
            fill=frame,
            outline=outline,
            width=_s(0.9),
        )
        draw.rectangle(
            _box(x1 + 3, y1 + 3, x2 - 3, y2 - 3),
            fill=glass,
            outline=_rgba("396079"),
            width=_s(0.8),
        )
    gx1, gy1, gx2, gy2 = x1 + 4, y1 + 4, x2 - 4, y2 - 4
    if lattice:
        for t in range(4):
            xx = gx1 + (gx2 - gx1) * (t + 1) / 5
            draw.line([(_s(xx), _s(gy1)), (_s(xx), _s(gy2))], fill=hi, width=_s(0.6))
        for t in range(2):
            yy = gy1 + (gy2 - gy1) * (t + 1) / 3
            draw.line([(_s(gx1), _s(yy)), (_s(gx2), _s(yy))], fill=hi, width=_s(0.6))
    else:
        draw.line(
            [(_s((x1 + x2) / 2), _s(y1 + 4)), (_s((x1 + x2) / 2), _s(y2 - 4))],
            fill=hi,
            width=_s(0.7),
        )
        draw.line(
            [(_s(x1 + 4), _s((y1 + y2) / 2)), (_s(x2 - 4), _s((y1 + y2) / 2))],
            fill=hi,
            width=_s(0.7),
        )
    if shutter:
        shutter_col = _rgba("5B7B49")
        draw.rounded_rectangle(
            _box(x1 - 8, y1 + 2, x1 - 1, y2 - 2),
            radius=_s(1.5),
            fill=shutter_col,
            outline=_rgba("314423"),
            width=_s(0.7),
        )
        draw.rounded_rectangle(
            _box(x2 + 1, y1 + 2, x2 + 8, y2 - 2),
            radius=_s(1.5),
            fill=shutter_col,
            outline=_rgba("314423"),
            width=_s(0.7),
        )
    if planter:
        draw.rounded_rectangle(
            _box(x1 + 2, y2 + 1, x2 - 2, y2 + 7),
            radius=_s(1.6),
            fill=_rgba("8F6847"),
            outline=_rgba("523521"),
            width=_s(0.7),
        )
        for px, col in (
            (x1 + 6, _rgba("D4799B")),
            (x1 + 12, _rgba("F5E183")),
            (x1 + 18, _rgba("B1E078")),
            (x1 + 24, _rgba("E7A3D1")),
        ):
            draw.ellipse(_box(px - 2, y2 + 1, px + 2, y2 + 5), fill=col)
    if balcony:
        draw.rectangle(_box(x1 - 4, y2 + 2, x2 + 4, y2 + 4), fill=_rgba("7B5A3D"))
        for bx in range(int(x1 - 2), int(x2 + 2), 6):
            draw.rectangle(_box(bx, y2 + 4, bx + 2, y2 + 11), fill=_rgba("7B5A3D"))
        draw.rectangle(_box(x1 - 4, y2 + 11, x2 + 4, y2 + 13), fill=_rgba("5B3F29"))


def _window_wall(
    draw: ImageDraw.ImageDraw, *, wall: str = "plaster", tone: str = "warm"
) -> None:
    if wall == "plaster":
        _plaster_wall_base(draw, tone=tone)
    elif wall == "timber":
        _timber_wall_base(draw)
    else:
        _brick_wall_base(draw, color="red")


def _render_window(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    wall = spec.params.get("wall", "plaster")
    tone = spec.params.get("tone", "warm")
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _window_wall(d, wall=wall, tone=tone)
    if variant == "small_green":
        _window_frame(d, 20, 14, 44, 42, shutter=True)
    elif variant == "tall_blue":
        _window_frame(d, 24, 8, 40, 46, shutter=False)
    elif variant == "round_attic":
        _window_frame(d, 20, 12, 44, 42, round_top=True)
    elif variant == "shopwide":
        _window_frame(d, 10, 16, 54, 40)
        d.rounded_rectangle(
            _box(8, 40, 56, 48),
            radius=_s(2),
            fill=_rgba("8F6847"),
            outline=_rgba("533520"),
            width=_s(0.7),
        )
    elif variant == "double":
        _window_frame(d, 9, 16, 26, 42)
        _window_frame(d, 38, 16, 55, 42)
    elif variant == "boxflowers":
        _window_frame(d, 18, 14, 46, 40, shutter=True, planter=True)
    elif variant == "lattice":
        _window_frame(d, 18, 12, 46, 44, lattice=True)
    elif variant == "balcony":
        _window_frame(d, 19, 11, 45, 37, arch=True, balcony=True)
    return _tile_downsample(img)


def _door_arch(
    draw: ImageDraw.ImageDraw,
    x1: float,
    y1: float,
    x2: float,
    y2: float,
    *,
    color: RGBA,
    arch: bool = False,
    double: bool = False,
    sign: str | None = None,
    awning: RGBA | None = None,
) -> None:
    outline = _rgba("442615")
    if awning is not None:
        stripe2 = _rgba("F7E8DB")
        for idx, x in enumerate(range(int(x1 - 4), int(x2 + 4), 6)):
            fill = awning if idx % 2 == 0 else stripe2
            draw.polygon(
                [
                    (_s(x), _s(y1 - 8)),
                    (_s(x + 6), _s(y1 - 8)),
                    (_s(x + 5), _s(y1 - 1)),
                    (_s(x + 1), _s(y1 - 1)),
                ],
                fill=fill,
                outline=_rgba("5C3634"),
            )
        draw.line(
            [(_s(x1 - 4), _s(y1 - 1)), (_s(x2 + 4), _s(y1 - 1))],
            fill=_rgba("5C3634"),
            width=_s(0.8),
        )
    if arch:
        draw.rounded_rectangle(
            _box(x1, y1, x2, y2),
            radius=_s(5),
            fill=color,
            outline=outline,
            width=_s(1.0),
        )
    else:
        draw.rounded_rectangle(
            _box(x1, y1, x2, y2),
            radius=_s(2),
            fill=color,
            outline=outline,
            width=_s(1.0),
        )
    inset = 3
    draw.rounded_rectangle(
        _box(x1 + inset, y1 + inset, x2 - inset, y2 - inset),
        radius=_s(1.5 if not arch else 3.5),
        fill=_mix(color, _rgba("000000"), 0.06),
        outline=_rgba("5C3724"),
        width=_s(0.8),
    )
    if double:
        mid = (x1 + x2) / 2
        draw.line(
            [(_s(mid), _s(y1 + 4)), (_s(mid), _s(y2 - 4))],
            fill=_rgba("E8D8B5", 120),
            width=_s(0.8),
        )
        draw.ellipse(
            _box(mid - 5, (y1 + y2) / 2 - 1, mid - 3, (y1 + y2) / 2 + 1),
            fill=_rgba("DFC46F"),
        )
        draw.ellipse(
            _box(mid + 3, (y1 + y2) / 2 - 1, mid + 5, (y1 + y2) / 2 + 1),
            fill=_rgba("DFC46F"),
        )
    else:
        draw.ellipse(
            _box(x2 - 7, (y1 + y2) / 2 - 1, x2 - 5, (y1 + y2) / 2 + 1),
            fill=_rgba("DFC46F"),
        )
    if sign:
        font = _font(9)
        _outline_text(
            draw,
            (_s(x1 + 4), _s(y1 - 17)),
            sign,
            font=font,
            fill=_rgba("F6EFDD"),
            outline=_rgba("000000", 140),
        )


def _render_door(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    wall = spec.params.get("wall", "plaster")
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _window_wall(d, wall=wall)
    d.rectangle(_box(0, 54, 64, 64), fill=_rgba("8E8170"))
    if variant == "wood":
        _door_arch(d, 18, 12, 46, 54, color=_rgba("835133"))
    elif variant == "arched":
        _door_arch(d, 17, 9, 47, 54, color=_rgba("7E4D31"), arch=True)
    elif variant == "double":
        _door_arch(d, 14, 12, 50, 54, color=_rgba("87583A"), double=True)
    elif variant == "shop":
        _door_arch(
            d,
            21,
            18,
            43,
            54,
            color=_rgba("7D4A32"),
            sign="SHOP",
            awning=_rgba("9F443E"),
        )
    elif variant == "cellar":
        d.rectangle(_box(12, 38, 52, 64), fill=_rgba("84786C"))
        d.polygon(
            [(_s(14), _s(40)), (_s(50), _s(40)), (_s(44), _s(64)), (_s(20), _s(64))],
            fill=_rgba("61574D"),
            outline=_rgba("40382F"),
        )
        for x in (25, 31, 37):
            d.line(
                [(_s(x), _s(42)), (_s(x + 8), _s(60))],
                fill=_rgba("989184"),
                width=_s(0.9),
            )
    elif variant == "blue":
        _door_arch(d, 18, 12, 46, 54, color=_rgba("4B6D8A"))
    elif variant == "red":
        _door_arch(d, 18, 12, 46, 54, color=_rgba("97453D"))
    elif variant == "tavern":
        _door_arch(
            d,
            19,
            18,
            45,
            54,
            color=_rgba("704626"),
            sign="TAVERN",
            awning=_rgba("C17A2E"),
        )
    return _tile_downsample(img)


def _render_roof(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    color = spec.params.get("color", "red")
    if color == "red":
        base, light, dark = _rgba("95423E"), _rgba("C3665D"), _rgba("632A28")
        wood = _rgba("6B432A")
    else:
        base, light, dark = _rgba("546E8B"), _rgba("7796BF"), _rgba("394B60")
        wood = _rgba("5A4F46")
    img = _tile_canvas()
    d = ImageDraw.Draw(img, "RGBA")
    _roof_pattern(d, base=base, light=light, dark=dark)
    if variant == "mid":
        pass
    elif variant == "eave":
        d.rectangle(_box(0, 50, 64, 60), fill=wood)
        d.line(
            [(_s(0), _s(50)), (_s(64), _s(50))], fill=_rgba("D9C4A8", 70), width=_s(0.7)
        )
    elif variant == "ridge":
        d.rectangle(_box(0, 0, 64, 8), fill=wood)
        d.line(
            [(_s(0), _s(8)), (_s(64), _s(8))], fill=_rgba("FFFFFF", 50), width=_s(0.6)
        )
    elif variant == "left_end":
        d.polygon(
            [(_s(0), _s(64)), (_s(18), _s(64)), (_s(0), _s(0))],
            fill=_rgba("000000", 45),
        )
        d.rectangle(_box(0, 50, 28, 60), fill=wood)
    elif variant == "right_end":
        d.polygon(
            [(_s(64), _s(64)), (_s(46), _s(64)), (_s(64), _s(0))],
            fill=_rgba("000000", 45),
        )
        d.rectangle(_box(36, 50, 64, 60), fill=wood)
    elif variant == "dormer":
        d.rectangle(
            _box(22, 20, 42, 50), fill=_rgba("DCCFBC"), outline=wood, width=_s(0.8)
        )
        d.polygon(
            [(_s(19), _s(24)), (_s(32), _s(12)), (_s(45), _s(24))],
            fill=base,
            outline=dark,
        )
        d.rectangle(
            _box(26, 28, 38, 44),
            fill=_rgba("6EA8C5"),
            outline=_rgba("396079"),
            width=_s(0.8),
        )
    elif variant == "chimney":
        d.rectangle(
            _box(34, 10, 47, 34),
            fill=_rgba("A06048"),
            outline=_rgba("613829"),
            width=_s(0.8),
        )
        d.rectangle(_box(32, 8, 49, 12), fill=_rgba("7A4837"))
        d.ellipse(_box(36, 5, 47, 12), fill=_rgba("E5E7EC", 60))
    elif variant == "gutter":
        d.rectangle(_box(0, 50, 64, 53), fill=_rgba("7F8A96"))
        d.rectangle(_box(53, 50, 56, 64), fill=_rgba("7F8A96"))
        d.line(
            [(_s(55), _s(50)), (_s(55), _s(64))],
            fill=_rgba("C8D3DE", 70),
            width=_s(0.6),
        )
    return _tile_downsample(img)


def _render_platform(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas(transparent=True)
    d = ImageDraw.Draw(img, "RGBA")
    wood = _rgba("8D633E")
    dark = _rgba("583821")
    stone = _rgba("8B8C90")
    if variant == "wood_platform":
        _shadow(d, 8, 36, 58, 48, 22)
        d.rectangle(_box(6, 26, 58, 36), fill=wood, outline=dark, width=_s(0.8))
        for x in range(10, 58, 10):
            d.line(
                [(_s(x), _s(28)), (_s(x), _s(34))],
                fill=_rgba("C69565", 90),
                width=_s(0.5),
            )
    elif variant == "stone_platform":
        _shadow(d, 8, 36, 58, 49, 22)
        d.rectangle(
            _box(6, 26, 58, 37), fill=stone, outline=_rgba("56585D"), width=_s(0.8)
        )
        for x in (16, 31, 46):
            d.line(
                [(_s(x), _s(28)), (_s(x), _s(36))],
                fill=_rgba("C8CCD2", 85),
                width=_s(0.6),
            )
    elif variant == "balcony_rail":
        d.rectangle(_box(8, 20, 56, 23), fill=wood)
        d.rectangle(_box(8, 34, 56, 37), fill=dark)
        for x in range(12, 56, 8):
            d.rectangle(_box(x, 23, x + 2, 34), fill=wood)
    elif variant == "support_beam":
        d.rectangle(_box(28, 4, 36, 60), fill=wood, outline=dark, width=_s(0.8))
        d.rectangle(
            _box(24, 8, 40, 14), fill=_rgba("A6784E"), outline=dark, width=_s(0.6)
        )
    elif variant == "support_brace":
        d.rectangle(_box(6, 8, 58, 14), fill=wood)
        d.polygon(
            [(_s(14), _s(14)), (_s(22), _s(14)), (_s(50), _s(58)), (_s(42), _s(58))],
            fill=wood,
            outline=dark,
        )
    elif variant == "ladder":
        d.rectangle(_box(22, 4, 26, 60), fill=wood)
        d.rectangle(_box(38, 4, 42, 60), fill=wood)
        for y in range(10, 56, 9):
            d.rectangle(_box(24, y, 40, y + 3), fill=_rgba("B18258"))
    elif variant == "stairs_up":
        _shadow(d, 4, 50, 60, 60, 22)
        for idx in range(5):
            x1 = 8 + idx * 10
            d.rectangle(
                _box(x1, 46 - idx * 8, x1 + 12, 56 - idx * 8),
                fill=wood,
                outline=dark,
                width=_s(0.8),
            )
    elif variant == "stairs_down":
        _shadow(d, 4, 50, 60, 60, 22)
        for idx in range(5):
            x1 = 44 - idx * 10
            d.rectangle(
                _box(x1, 46 - idx * 8, x1 + 12, 56 - idx * 8),
                fill=wood,
                outline=dark,
                width=_s(0.8),
            )
    return _tile_downsample(img)


def _render_street_prop(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas(transparent=True)
    d = ImageDraw.Draw(img, "RGBA")
    if variant == "lamp_post":
        _shadow(d, 24, 54, 40, 60, 30)
        d.rectangle(_box(30, 12, 34, 56), fill=_rgba("37414A"))
        d.arc(_box(28, 8, 43, 22), 180, 320, fill=_rgba("37414A"), width=_s(1.7))
        d.rounded_rectangle(
            _box(36, 10, 47, 23),
            radius=_s(2.2),
            fill=_rgba("F2D47E"),
            outline=_rgba("7A6434"),
            width=_s(0.8),
        )
        d.ellipse(_box(38, 13, 45, 20), fill=_rgba("FFF4B8", 170))
        d.rectangle(_box(26, 56, 38, 60), fill=_rgba("4B5560"))
    elif variant == "signpost":
        _shadow(d, 20, 54, 44, 60, 24)
        d.rectangle(_box(30, 14, 34, 56), fill=_rgba("8A613C"))
        d.polygon(
            [
                (_s(34), _s(20)),
                (_s(51), _s(20)),
                (_s(57), _s(25)),
                (_s(51), _s(30)),
                (_s(34), _s(30)),
            ],
            fill=_rgba("A97648"),
            outline=_rgba("492E1A"),
        )
        d.polygon(
            [
                (_s(30), _s(34)),
                (_s(13), _s(34)),
                (_s(7), _s(39)),
                (_s(13), _s(44)),
                (_s(30), _s(44)),
            ],
            fill=_rgba("A97648"),
            outline=_rgba("492E1A"),
        )
    elif variant == "bench":
        _shadow(d, 12, 49, 52, 59, 25)
        wood = _rgba("90643C")
        metal = _rgba("49545E")
        d.rectangle(_box(14, 30, 50, 35), fill=wood)
        d.rectangle(_box(14, 37, 50, 42), fill=wood)
        d.rectangle(_box(16, 18, 48, 26), fill=wood)
        d.arc(_box(15, 39, 25, 56), 180, 330, fill=metal, width=_s(1.5))
        d.arc(_box(39, 39, 49, 56), 210, 360, fill=metal, width=_s(1.5))
    elif variant == "mailbox":
        _shadow(d, 20, 52, 46, 60, 25)
        d.rectangle(_box(30, 18, 34, 56), fill=_rgba("6B4A33"))
        d.rounded_rectangle(
            _box(18, 20, 46, 38),
            radius=_s(4),
            fill=_rgba("A13E34"),
            outline=_rgba("5A211D"),
            width=_s(0.9),
        )
        d.rectangle(_box(20, 22, 44, 27), fill=_rgba("B74E43"))
        d.rectangle(_box(25, 27, 39, 29), fill=_rgba("F0E5D4"))
    elif variant == "crate_stack":
        _shadow(d, 16, 49, 48, 59, 25)

        def crate(x: int, y: int, w: int = 16, h: int = 13):
            d.rectangle(
                _box(x, y, x + w, y + h),
                fill=_rgba("A26B3E"),
                outline=_rgba("5D391D"),
                width=_s(0.7),
            )
            d.line(
                [(_s(x + 2), _s(y + 2)), (_s(x + w - 2), _s(y + h - 2))],
                fill=_rgba("D69D62", 110),
                width=_s(0.6),
            )
            d.line(
                [(_s(x + w - 2), _s(y + 2)), (_s(x + 2), _s(y + h - 2))],
                fill=_rgba("6A4527", 110),
                width=_s(0.6),
            )

        crate(18, 36)
        crate(31, 39)
        crate(24, 24)
    elif variant == "barrel":
        _shadow(d, 20, 50, 44, 60, 25)
        d.ellipse(
            _box(22, 18, 42, 26),
            fill=_rgba("8A5F3E"),
            outline=_rgba("52331D"),
            width=_s(0.8),
        )
        d.rectangle(
            _box(22, 22, 42, 50),
            fill=_rgba("8A5F3E"),
            outline=_rgba("52331D"),
            width=_s(0.8),
        )
        d.ellipse(
            _box(22, 44, 42, 52),
            fill=_rgba("785034"),
            outline=_rgba("52331D"),
            width=_s(0.8),
        )
        for y in (27, 37):
            d.rectangle(_box(21, y, 43, y + 3), fill=_rgba("4C4E52"))
    elif variant == "planter":
        _shadow(d, 12, 50, 52, 60, 24)
        d.rounded_rectangle(
            _box(14, 40, 50, 54),
            radius=_s(2),
            fill=_rgba("8B6240"),
            outline=_rgba("55351F"),
            width=_s(0.8),
        )
        for cx, cy, r, color in (
            (22, 34, 7, _rgba("5AA34C")),
            (32, 31, 8, _rgba("6AB55D")),
            (41, 35, 7, _rgba("4E8E43")),
        ):
            d.ellipse(
                _box(cx - r, cy - r, cx + r, cy + r),
                fill=color,
                outline=_rgba("2D5B2E"),
                width=_s(0.5),
            )
        for cx, cy, col in (
            (23, 33, _rgba("F1D47F")),
            (30, 30, _rgba("D6799F")),
            (39, 34, _rgba("B2E17A")),
        ):
            d.ellipse(_box(cx - 1.5, cy - 1.5, cx + 1.5, cy + 1.5), fill=col)
    elif variant == "fence_iron":
        d.rectangle(_box(8, 18, 56, 21), fill=_rgba("495059"))
        d.rectangle(_box(8, 42, 56, 45), fill=_rgba("495059"))
        for x in range(12, 56, 8):
            d.rectangle(_box(x, 21, x + 2, 42), fill=_rgba("5D6670"))
            d.polygon(
                [(_s(x - 1), _s(21)), (_s(x + 3), _s(21)), (_s(x + 1), _s(16))],
                fill=_rgba("7A8590"),
            )
    return _tile_downsample(img)


def _render_civic_prop(spec: TileSpec) -> Image.Image:
    variant = spec.params["variant"]
    img = _tile_canvas(transparent=True)
    d = ImageDraw.Draw(img, "RGBA")
    if variant == "well":
        _shadow(d, 12, 50, 52, 60, 24)
        d.rounded_rectangle(
            _box(16, 32, 48, 50),
            radius=_s(4),
            fill=_rgba("8D949D"),
            outline=_rgba("50545C"),
            width=_s(0.9),
        )
        d.rectangle(
            _box(20, 36, 44, 46),
            fill=_rgba("5FA7C9"),
            outline=_rgba("3F6B84"),
            width=_s(0.7),
        )
        d.rectangle(_box(18, 18, 22, 34), fill=_rgba("6C4932"))
        d.rectangle(_box(42, 18, 46, 34), fill=_rgba("6C4932"))
        d.polygon(
            [
                (_s(14), _s(22)),
                (_s(32), _s(12)),
                (_s(50), _s(22)),
                (_s(46), _s(28)),
                (_s(18), _s(28)),
            ],
            fill=_rgba("8A413D"),
            outline=_rgba("4C211F"),
        )
        d.line(
            [(_s(32), _s(18)), (_s(32), _s(30))], fill=_rgba("6A543A"), width=_s(0.8)
        )
        d.ellipse(_box(30, 28, 34, 32), fill=_rgba("9E7750"))
    elif variant == "fountain":
        _shadow(d, 10, 50, 54, 60, 24)
        d.ellipse(
            _box(12, 37, 52, 55),
            fill=_rgba("9EA7B2"),
            outline=_rgba("626973"),
            width=_s(0.9),
        )
        d.ellipse(
            _box(17, 40, 47, 51),
            fill=_rgba("6CB7D6"),
            outline=_rgba("3F7491"),
            width=_s(0.7),
        )
        d.rectangle(_box(29, 23, 35, 41), fill=_rgba("8E98A3"))
        d.ellipse(
            _box(24, 18, 40, 28),
            fill=_rgba("A8B2BE"),
            outline=_rgba("666E79"),
            width=_s(0.8),
        )
        d.arc(_box(25, 11, 39, 27), 200, 340, fill=_rgba("CDEFFF", 160), width=_s(1.0))
        d.arc(_box(22, 9, 42, 31), 210, 330, fill=_rgba("CDEFFF", 90), width=_s(0.8))
    elif variant == "tree_small":
        _shadow(d, 14, 52, 50, 60, 24)
        d.rectangle(_box(29, 34, 35, 54), fill=_rgba("7D5333"))
        for cx, cy, r, color in (
            (24, 30, 11, _rgba("529B4A")),
            (39, 28, 12, _rgba("66B05D")),
            (32, 20, 13, _rgba("447C3D")),
            (28, 38, 10, _rgba("66B05D")),
            (39, 38, 9, _rgba("529B4A")),
        ):
            d.ellipse(
                _box(cx - r, cy - r, cx + r, cy + r),
                fill=color,
                outline=_rgba("2D5B2E"),
                width=_s(0.5),
            )
    elif variant == "hedge":
        d.rounded_rectangle(
            _box(6, 22, 58, 50),
            radius=_s(8),
            fill=_rgba("4A8C45"),
            outline=_rgba("245126"),
            width=_s(0.8),
        )
        for x, y in ((14, 28), (22, 38), (37, 31), (44, 41), (52, 33)):
            d.ellipse(_box(x, y, x + 6, y + 5), fill=_rgba("64AF5B", 160))
    elif variant == "market_red":
        _shadow(d, 10, 53, 54, 60, 24)
        d.rectangle(
            _box(14, 28, 50, 48),
            fill=_rgba("845C39"),
            outline=_rgba("4C301A"),
            width=_s(0.9),
        )
        d.rectangle(_box(16, 48, 20, 58), fill=_rgba("694529"))
        d.rectangle(_box(44, 48, 48, 58), fill=_rgba("694529"))
        for x in (20, 28, 36, 44):
            d.rectangle(
                _box(x, 32, x + 5, 42),
                fill=_rgba("C38E52") if x % 16 == 4 else _rgba("D6AA62"),
                outline=_rgba("7F5B33"),
                width=_s(0.4),
            )
        canopy = (_rgba("A94543"), _rgba("F1D8C7"))
        for i, x in enumerate(range(12, 52, 8)):
            fill = canopy[i % 2]
            d.polygon(
                [
                    (_s(x), _s(28)),
                    (_s(x + 8), _s(28)),
                    (_s(x + 6), _s(14)),
                    (_s(x + 2), _s(14)),
                ],
                fill=fill,
                outline=_rgba("5F3533"),
            )
    elif variant == "market_blue":
        _shadow(d, 10, 53, 54, 60, 24)
        d.rectangle(
            _box(14, 28, 50, 48),
            fill=_rgba("845C39"),
            outline=_rgba("4C301A"),
            width=_s(0.9),
        )
        d.rectangle(_box(16, 48, 20, 58), fill=_rgba("694529"))
        d.rectangle(_box(44, 48, 48, 58), fill=_rgba("694529"))
        canopy = (_rgba("4A6A9E"), _rgba("E4ECF7"))
        for i, x in enumerate(range(12, 52, 8)):
            fill = canopy[i % 2]
            d.polygon(
                [
                    (_s(x), _s(28)),
                    (_s(x + 8), _s(28)),
                    (_s(x + 6), _s(14)),
                    (_s(x + 2), _s(14)),
                ],
                fill=fill,
                outline=_rgba("324760"),
            )
    elif variant == "banner_red":
        d.rectangle(_box(26, 6, 30, 56), fill=_rgba("4E5560"))
        d.polygon(
            [
                (_s(30), _s(10)),
                (_s(50), _s(12)),
                (_s(50), _s(42)),
                (_s(40), _s(37)),
                (_s(30), _s(42)),
            ],
            fill=_rgba("A13E3A"),
            outline=_rgba("5C2321"),
        )
        d.circle = None
        d.ellipse(_box(36, 18, 44, 26), fill=_rgba("E9D89A"))
        d.line(
            [(_s(40), _s(18)), (_s(40), _s(26))], fill=_rgba("8E7B38"), width=_s(0.7)
        )
        d.line(
            [(_s(36), _s(22)), (_s(44), _s(22))], fill=_rgba("8E7B38"), width=_s(0.7)
        )
    elif variant == "banner_blue":
        d.rectangle(_box(26, 6, 30, 56), fill=_rgba("4E5560"))
        d.polygon(
            [
                (_s(30), _s(10)),
                (_s(50), _s(12)),
                (_s(50), _s(42)),
                (_s(40), _s(37)),
                (_s(30), _s(42)),
            ],
            fill=_rgba("4A6A9E"),
            outline=_rgba("324760"),
        )
        d.ellipse(_box(35, 18, 45, 28), fill=_rgba("D3E9F7"))
        d.ellipse(_box(38, 16, 42, 30), fill=_rgba("B7D7EC"))
    return _tile_downsample(img)


RENDERERS: Dict[str, Callable[[TileSpec], Image.Image]] = {
    "terrain": _render_terrain,
    "foundation": _render_foundation,
    "plaster_wall": _render_plaster_wall,
    "timber_wall": _render_timber_wall,
    "brick_wall": _render_brick_wall,
    "window": _render_window,
    "door": _render_door,
    "roof": _render_roof,
    "platform": _render_platform,
    "street_prop": _render_street_prop,
    "civic_prop": _render_civic_prop,
}


# ---------------------------------------------------------------------------
# spec construction


def _spec(
    key: str,
    display_name: str,
    category: str,
    description: str,
    layer: str,
    style: str,
    params: Dict[str, Any],
    tags: Iterable[str],
) -> TileSpec:
    return TileSpec(
        key, display_name, category, description, layer, style, params, tuple(tags)
    )


TILES: List[TileSpec] = [
    # terrain (8)
    _spec(
        "grass_top",
        "Grass Top",
        "terrain",
        "Side-view grassy terrain cap with dirt body.",
        "ground",
        "terrain",
        {"variant": "grass_top"},
        ["terrain", "grass", "top"],
    ),
    _spec(
        "grass_fill",
        "Grass Fill",
        "terrain",
        "Packed dirt / earth fill for terrain interiors.",
        "ground",
        "terrain",
        {"variant": "grass_fill"},
        ["terrain", "dirt", "fill"],
    ),
    _spec(
        "grass_left_edge",
        "Grass Left Edge",
        "terrain",
        "Left-facing grassy ledge edge tile.",
        "ground",
        "terrain",
        {"variant": "grass_left"},
        ["terrain", "edge", "left"],
    ),
    _spec(
        "grass_right_edge",
        "Grass Right Edge",
        "terrain",
        "Right-facing grassy ledge edge tile.",
        "ground",
        "terrain",
        {"variant": "grass_right"},
        ["terrain", "edge", "right"],
    ),
    _spec(
        "grass_slope_up",
        "Grass Slope Up",
        "terrain",
        "Ascending grassy slope tile for side-scroller terrain.",
        "ground",
        "terrain",
        {"variant": "slope_up"},
        ["terrain", "slope", "up"],
    ),
    _spec(
        "grass_slope_down",
        "Grass Slope Down",
        "terrain",
        "Descending grassy slope tile for side-scroller terrain.",
        "ground",
        "terrain",
        {"variant": "slope_down"},
        ["terrain", "slope", "down"],
    ),
    _spec(
        "grass_flowers",
        "Grass Flowers",
        "terrain",
        "Flowered grassy top variant for gardens and softer outskirts.",
        "ground",
        "terrain",
        {"variant": "grass_flowers"},
        ["terrain", "variant", "flowers"],
    ),
    _spec(
        "grass_cliff",
        "Grass Cliff",
        "terrain",
        "Rockier grass-topped cliff fill tile.",
        "ground",
        "terrain",
        {"variant": "grass_cliff"},
        ["terrain", "cliff", "stone"],
    ),
    # foundation / street (8)
    _spec(
        "cobble_walk",
        "Cobble Walk",
        "foundation",
        "Town cobblestone walking surface.",
        "ground",
        "foundation",
        {"variant": "cobble_walk"},
        ["street", "cobble"],
    ),
    _spec(
        "cobble_border",
        "Cobble Border",
        "foundation",
        "Cobblestone surface with lighter border trim.",
        "ground",
        "foundation",
        {"variant": "cobble_border"},
        ["street", "cobble", "trim"],
    ),
    _spec(
        "stone_foundation",
        "Stone Foundation",
        "foundation",
        "Neutral stone wall / foundation block.",
        "structure",
        "foundation",
        {"variant": "stone_foundation"},
        ["foundation", "stone"],
    ),
    _spec(
        "stone_foundation_cracked",
        "Stone Foundation Cracked",
        "foundation",
        "Weathered stone foundation variant.",
        "structure",
        "foundation",
        {"variant": "stone_foundation_cracked"},
        ["foundation", "stone", "cracked"],
    ),
    _spec(
        "brick_pavers",
        "Brick Pavers",
        "foundation",
        "Warm brick paving tile for sidewalks or interior floors.",
        "ground",
        "foundation",
        {"variant": "brick_pavers"},
        ["street", "brick", "floor"],
    ),
    _spec(
        "boardwalk",
        "Boardwalk",
        "foundation",
        "Wood plank deck / boardwalk floor tile.",
        "ground",
        "foundation",
        {"variant": "boardwalk"},
        ["wood", "floor"],
    ),
    _spec(
        "stoop_steps",
        "Stoop Steps",
        "foundation",
        "Front stoop stone steps for house entries.",
        "overlay",
        "foundation",
        {"variant": "stoop_steps"},
        ["stairs", "entry"],
    ),
    _spec(
        "drain_grate",
        "Drain Grate",
        "foundation",
        "Metal street drain / service cover tile.",
        "ground",
        "foundation",
        {"variant": "drain_grate"},
        ["street", "industrial"],
    ),
    # plaster walls (8)
    _spec(
        "wall_plaster_plain",
        "Wall Plaster Plain",
        "wall_plaster",
        "Basic plaster facade wall tile.",
        "structure",
        "plaster_wall",
        {"variant": "plain", "tone": "warm"},
        ["wall", "plaster"],
    ),
    _spec(
        "wall_plaster_top_trim",
        "Wall Plaster Top Trim",
        "wall_plaster",
        "Plaster wall with top lintel / trim.",
        "structure",
        "plaster_wall",
        {"variant": "top_trim", "tone": "warm"},
        ["wall", "plaster", "trim"],
    ),
    _spec(
        "wall_plaster_bottom_trim",
        "Wall Plaster Bottom Trim",
        "wall_plaster",
        "Plaster wall with decorative lower trim.",
        "structure",
        "plaster_wall",
        {"variant": "bottom_trim", "tone": "warm"},
        ["wall", "plaster", "trim"],
    ),
    _spec(
        "wall_plaster_left",
        "Wall Plaster Left Border",
        "wall_plaster",
        "Plaster wall left border / corner tile.",
        "structure",
        "plaster_wall",
        {"variant": "left", "tone": "warm"},
        ["wall", "plaster", "edge"],
    ),
    _spec(
        "wall_plaster_right",
        "Wall Plaster Right Border",
        "wall_plaster",
        "Plaster wall right border / corner tile.",
        "structure",
        "plaster_wall",
        {"variant": "right", "tone": "warm"},
        ["wall", "plaster", "edge"],
    ),
    _spec(
        "wall_plaster_cracked",
        "Wall Plaster Cracked",
        "wall_plaster",
        "Aged plaster wall variant.",
        "structure",
        "plaster_wall",
        {"variant": "cracked", "tone": "warm"},
        ["wall", "plaster", "aged"],
    ),
    _spec(
        "wall_plaster_notice",
        "Wall Plaster Notice",
        "wall_plaster",
        "Plaster wall with mounted notice board.",
        "structure",
        "plaster_wall",
        {"variant": "notice", "tone": "warm"},
        ["wall", "plaster", "notice"],
    ),
    _spec(
        "wall_plaster_arch",
        "Wall Plaster Arch",
        "wall_plaster",
        "Blank plaster arch recess for visual breakup.",
        "structure",
        "plaster_wall",
        {"variant": "arch", "tone": "warm"},
        ["wall", "plaster", "arch"],
    ),
    # timber walls (8)
    _spec(
        "wall_timber_plain",
        "Wall Timber Plain",
        "wall_timber",
        "Half-timber wall basic panel.",
        "structure",
        "timber_wall",
        {"variant": "plain"},
        ["wall", "timber"],
    ),
    _spec(
        "wall_timber_cross",
        "Wall Timber Cross",
        "wall_timber",
        "Half-timber wall with cross bracing.",
        "structure",
        "timber_wall",
        {"variant": "cross"},
        ["wall", "timber", "brace"],
    ),
    _spec(
        "wall_timber_vertical",
        "Wall Timber Vertical",
        "wall_timber",
        "Timber wall with strong vertical members.",
        "structure",
        "timber_wall",
        {"variant": "vertical"},
        ["wall", "timber", "vertical"],
    ),
    _spec(
        "wall_timber_horizontal",
        "Wall Timber Horizontal",
        "wall_timber",
        "Timber wall with extra horizontal bands.",
        "structure",
        "timber_wall",
        {"variant": "horizontal"},
        ["wall", "timber", "horizontal"],
    ),
    _spec(
        "wall_timber_left",
        "Wall Timber Left Border",
        "wall_timber",
        "Timber wall left edge / corner tile.",
        "structure",
        "timber_wall",
        {"variant": "left"},
        ["wall", "timber", "edge"],
    ),
    _spec(
        "wall_timber_right",
        "Wall Timber Right Border",
        "wall_timber",
        "Timber wall right edge / corner tile.",
        "structure",
        "timber_wall",
        {"variant": "right"},
        ["wall", "timber", "edge"],
    ),
    _spec(
        "wall_timber_upper",
        "Wall Timber Upper Band",
        "wall_timber",
        "Timber wall with upper banding.",
        "structure",
        "timber_wall",
        {"variant": "upper"},
        ["wall", "timber", "trim"],
    ),
    _spec(
        "wall_timber_sign",
        "Wall Timber Sign Mount",
        "wall_timber",
        "Timber wall with a hanging sign plate.",
        "structure",
        "timber_wall",
        {"variant": "sign"},
        ["wall", "timber", "sign"],
    ),
    # brick walls (8)
    _spec(
        "wall_brick_plain",
        "Wall Brick Plain",
        "wall_brick",
        "Basic red-brick wall tile.",
        "structure",
        "brick_wall",
        {"variant": "plain", "color": "red"},
        ["wall", "brick"],
    ),
    _spec(
        "wall_brick_top_trim",
        "Wall Brick Top Trim",
        "wall_brick",
        "Brick wall with upper trim band.",
        "structure",
        "brick_wall",
        {"variant": "top_trim", "color": "red"},
        ["wall", "brick", "trim"],
    ),
    _spec(
        "wall_brick_bottom_trim",
        "Wall Brick Bottom Trim",
        "wall_brick",
        "Brick wall with lower trim band.",
        "structure",
        "brick_wall",
        {"variant": "bottom_trim", "color": "red"},
        ["wall", "brick", "trim"],
    ),
    _spec(
        "wall_brick_left",
        "Wall Brick Left Border",
        "wall_brick",
        "Brick wall left edge / corner tile.",
        "structure",
        "brick_wall",
        {"variant": "left", "color": "red"},
        ["wall", "brick", "edge"],
    ),
    _spec(
        "wall_brick_right",
        "Wall Brick Right Border",
        "wall_brick",
        "Brick wall right edge / corner tile.",
        "structure",
        "brick_wall",
        {"variant": "right", "color": "red"},
        ["wall", "brick", "edge"],
    ),
    _spec(
        "wall_brick_arched",
        "Wall Brick Arched",
        "wall_brick",
        "Brick wall with a blind arch detail.",
        "structure",
        "brick_wall",
        {"variant": "arched", "color": "red"},
        ["wall", "brick", "arch"],
    ),
    _spec(
        "wall_brick_cracked",
        "Wall Brick Cracked",
        "wall_brick",
        "Weathered brick wall variant.",
        "structure",
        "brick_wall",
        {"variant": "cracked", "color": "red"},
        ["wall", "brick", "aged"],
    ),
    _spec(
        "wall_brick_vines",
        "Wall Brick Vines",
        "wall_brick",
        "Brick wall with creeping vines.",
        "structure",
        "brick_wall",
        {"variant": "vines", "color": "red"},
        ["wall", "brick", "vines"],
    ),
    # windows (8)
    _spec(
        "window_small_green",
        "Window Small Green",
        "window",
        "Small shuttered town window.",
        "structure",
        "window",
        {"variant": "small_green", "wall": "plaster"},
        ["window", "shutter"],
    ),
    _spec(
        "window_tall_blue",
        "Window Tall Blue",
        "window",
        "Tall narrow window for upper stories.",
        "structure",
        "window",
        {"variant": "tall_blue", "wall": "plaster", "tone": "cool"},
        ["window", "tall"],
    ),
    _spec(
        "window_round_attic",
        "Window Round Attic",
        "window",
        "Attic / dormer round-top window.",
        "structure",
        "window",
        {"variant": "round_attic", "wall": "timber"},
        ["window", "attic"],
    ),
    _spec(
        "window_shopwide",
        "Window Shopwide",
        "window",
        "Wide storefront display window.",
        "structure",
        "window",
        {"variant": "shopwide", "wall": "brick"},
        ["window", "shop"],
    ),
    _spec(
        "window_double",
        "Window Double",
        "window",
        "Pair of side-by-side windows.",
        "structure",
        "window",
        {"variant": "double", "wall": "plaster"},
        ["window", "double"],
    ),
    _spec(
        "window_boxflowers",
        "Window Boxflowers",
        "window",
        "Flower-box window for cheerful homes.",
        "structure",
        "window",
        {"variant": "boxflowers", "wall": "plaster"},
        ["window", "flowers"],
    ),
    _spec(
        "window_lattice",
        "Window Lattice",
        "window",
        "Decorative lattice window.",
        "structure",
        "window",
        {"variant": "lattice", "wall": "timber"},
        ["window", "lattice"],
    ),
    _spec(
        "window_balcony",
        "Window Balcony",
        "window",
        "French window with small balcony rail.",
        "structure",
        "window",
        {"variant": "balcony", "wall": "brick"},
        ["window", "balcony"],
    ),
    # doors (8)
    _spec(
        "door_wood",
        "Door Wood",
        "door",
        "Standard wooden house door.",
        "structure",
        "door",
        {"variant": "wood", "wall": "plaster"},
        ["door", "house"],
    ),
    _spec(
        "door_arched",
        "Door Arched",
        "door",
        "Arched stone-framed door.",
        "structure",
        "door",
        {"variant": "arched", "wall": "brick"},
        ["door", "arched"],
    ),
    _spec(
        "door_double",
        "Door Double",
        "door",
        "Double-front civic or manor entry.",
        "structure",
        "door",
        {"variant": "double", "wall": "timber"},
        ["door", "double"],
    ),
    _spec(
        "door_shop",
        "Door Shop",
        "door",
        "Shopfront entry with awning.",
        "structure",
        "door",
        {"variant": "shop", "wall": "brick"},
        ["door", "shop"],
    ),
    _spec(
        "door_cellar",
        "Door Cellar",
        "door",
        "Cellar / basement stair hatch.",
        "structure",
        "door",
        {"variant": "cellar", "wall": "stone"},
        ["door", "cellar"],
    ),
    _spec(
        "door_blue",
        "Door Blue",
        "door",
        "Painted blue home door.",
        "structure",
        "door",
        {"variant": "blue", "wall": "plaster"},
        ["door", "painted"],
    ),
    _spec(
        "door_red",
        "Door Red",
        "door",
        "Painted red home door.",
        "structure",
        "door",
        {"variant": "red", "wall": "plaster"},
        ["door", "painted"],
    ),
    _spec(
        "door_tavern",
        "Door Tavern",
        "door",
        "Tavern entry with sign and awning.",
        "structure",
        "door",
        {"variant": "tavern", "wall": "timber"},
        ["door", "tavern"],
    ),
    # red roofs (8)
    _spec(
        "roof_red_mid",
        "Roof Red Mid",
        "roof_red",
        "Main red roof field tile.",
        "structure",
        "roof",
        {"variant": "mid", "color": "red"},
        ["roof", "red"],
    ),
    _spec(
        "roof_red_eave",
        "Roof Red Eave",
        "roof_red",
        "Red roof eave / lower edge tile.",
        "structure",
        "roof",
        {"variant": "eave", "color": "red"},
        ["roof", "red", "eave"],
    ),
    _spec(
        "roof_red_ridge",
        "Roof Red Ridge",
        "roof_red",
        "Red roof ridge / top edge tile.",
        "structure",
        "roof",
        {"variant": "ridge", "color": "red"},
        ["roof", "red", "ridge"],
    ),
    _spec(
        "roof_red_left_end",
        "Roof Red Left End",
        "roof_red",
        "Red roof left end cap.",
        "structure",
        "roof",
        {"variant": "left_end", "color": "red"},
        ["roof", "red", "edge"],
    ),
    _spec(
        "roof_red_right_end",
        "Roof Red Right End",
        "roof_red",
        "Red roof right end cap.",
        "structure",
        "roof",
        {"variant": "right_end", "color": "red"},
        ["roof", "red", "edge"],
    ),
    _spec(
        "roof_red_dormer",
        "Roof Red Dormer",
        "roof_red",
        "Red roof with dormer window.",
        "structure",
        "roof",
        {"variant": "dormer", "color": "red"},
        ["roof", "red", "dormer"],
    ),
    _spec(
        "roof_red_chimney",
        "Roof Red Chimney",
        "roof_red",
        "Red roof with chimney stack.",
        "structure",
        "roof",
        {"variant": "chimney", "color": "red"},
        ["roof", "red", "chimney"],
    ),
    _spec(
        "roof_red_gutter",
        "Roof Red Gutter",
        "roof_red",
        "Red roof with gutter / downspout detail.",
        "structure",
        "roof",
        {"variant": "gutter", "color": "red"},
        ["roof", "red", "gutter"],
    ),
    # blue roofs (8)
    _spec(
        "roof_blue_mid",
        "Roof Blue Mid",
        "roof_blue",
        "Main blue roof field tile.",
        "structure",
        "roof",
        {"variant": "mid", "color": "blue"},
        ["roof", "blue"],
    ),
    _spec(
        "roof_blue_eave",
        "Roof Blue Eave",
        "roof_blue",
        "Blue roof eave / lower edge tile.",
        "structure",
        "roof",
        {"variant": "eave", "color": "blue"},
        ["roof", "blue", "eave"],
    ),
    _spec(
        "roof_blue_ridge",
        "Roof Blue Ridge",
        "roof_blue",
        "Blue roof ridge / top edge tile.",
        "structure",
        "roof",
        {"variant": "ridge", "color": "blue"},
        ["roof", "blue", "ridge"],
    ),
    _spec(
        "roof_blue_left_end",
        "Roof Blue Left End",
        "roof_blue",
        "Blue roof left end cap.",
        "structure",
        "roof",
        {"variant": "left_end", "color": "blue"},
        ["roof", "blue", "edge"],
    ),
    _spec(
        "roof_blue_right_end",
        "Roof Blue Right End",
        "roof_blue",
        "Blue roof right end cap.",
        "structure",
        "roof",
        {"variant": "right_end", "color": "blue"},
        ["roof", "blue", "edge"],
    ),
    _spec(
        "roof_blue_dormer",
        "Roof Blue Dormer",
        "roof_blue",
        "Blue roof with dormer window.",
        "structure",
        "roof",
        {"variant": "dormer", "color": "blue"},
        ["roof", "blue", "dormer"],
    ),
    _spec(
        "roof_blue_chimney",
        "Roof Blue Chimney",
        "roof_blue",
        "Blue roof with chimney stack.",
        "structure",
        "roof",
        {"variant": "chimney", "color": "blue"},
        ["roof", "blue", "chimney"],
    ),
    _spec(
        "roof_blue_gutter",
        "Roof Blue Gutter",
        "roof_blue",
        "Blue roof with gutter / downspout detail.",
        "structure",
        "roof",
        {"variant": "gutter", "color": "blue"},
        ["roof", "blue", "gutter"],
    ),
    # platform / structural overlays (8)
    _spec(
        "platform_wood",
        "Platform Wood",
        "platform",
        "Simple wooden platform / ledge.",
        "overlay",
        "platform",
        {"variant": "wood_platform"},
        ["platform", "wood"],
    ),
    _spec(
        "platform_stone",
        "Platform Stone",
        "platform",
        "Stone ledge or balcony slab.",
        "overlay",
        "platform",
        {"variant": "stone_platform"},
        ["platform", "stone"],
    ),
    _spec(
        "balcony_rail",
        "Balcony Rail",
        "platform",
        "Balcony or porch railing section.",
        "overlay",
        "platform",
        {"variant": "balcony_rail"},
        ["balcony", "rail"],
    ),
    _spec(
        "support_beam",
        "Support Beam",
        "platform",
        "Vertical support post.",
        "overlay",
        "platform",
        {"variant": "support_beam"},
        ["support", "beam"],
    ),
    _spec(
        "support_brace",
        "Support Brace",
        "platform",
        "Angled structural brace.",
        "overlay",
        "platform",
        {"variant": "support_brace"},
        ["support", "brace"],
    ),
    _spec(
        "ladder",
        "Ladder",
        "platform",
        "Wooden ladder tile.",
        "overlay",
        "platform",
        {"variant": "ladder"},
        ["ladder", "climb"],
    ),
    _spec(
        "stairs_up",
        "Stairs Up",
        "platform",
        "Ascending wooden steps.",
        "overlay",
        "platform",
        {"variant": "stairs_up"},
        ["stairs", "up"],
    ),
    _spec(
        "stairs_down",
        "Stairs Down",
        "platform",
        "Descending wooden steps.",
        "overlay",
        "platform",
        {"variant": "stairs_down"},
        ["stairs", "down"],
    ),
    # street props (8)
    _spec(
        "lamp_post",
        "Lamp Post",
        "street_prop",
        "Town street lamp.",
        "overlay",
        "street_prop",
        {"variant": "lamp_post"},
        ["prop", "street", "light"],
    ),
    _spec(
        "signpost",
        "Signpost",
        "street_prop",
        "Wooden direction sign.",
        "overlay",
        "street_prop",
        {"variant": "signpost"},
        ["prop", "street", "sign"],
    ),
    _spec(
        "bench",
        "Bench",
        "street_prop",
        "Town bench.",
        "overlay",
        "street_prop",
        {"variant": "bench"},
        ["prop", "street", "bench"],
    ),
    _spec(
        "mailbox",
        "Mailbox",
        "street_prop",
        "Mailbox or drop box.",
        "overlay",
        "street_prop",
        {"variant": "mailbox"},
        ["prop", "street", "mail"],
    ),
    _spec(
        "crate_stack",
        "Crate Stack",
        "street_prop",
        "Stack of crates for alleys and loading zones.",
        "overlay",
        "street_prop",
        {"variant": "crate_stack"},
        ["prop", "storage"],
    ),
    _spec(
        "barrel",
        "Barrel",
        "street_prop",
        "Wooden barrel prop.",
        "overlay",
        "street_prop",
        {"variant": "barrel"},
        ["prop", "storage"],
    ),
    _spec(
        "planter",
        "Planter",
        "street_prop",
        "Flower planter box.",
        "overlay",
        "street_prop",
        {"variant": "planter"},
        ["prop", "garden"],
    ),
    _spec(
        "fence_iron",
        "Fence Iron",
        "street_prop",
        "Short iron fence segment.",
        "overlay",
        "street_prop",
        {"variant": "fence_iron"},
        ["prop", "fence"],
    ),
    # civic / market / greenery props (8)
    _spec(
        "well",
        "Well",
        "civic_prop",
        "Village well.",
        "overlay",
        "civic_prop",
        {"variant": "well"},
        ["prop", "water"],
    ),
    _spec(
        "fountain",
        "Fountain",
        "civic_prop",
        "Civic fountain.",
        "overlay",
        "civic_prop",
        {"variant": "fountain"},
        ["prop", "water", "civic"],
    ),
    _spec(
        "tree_small",
        "Tree Small",
        "civic_prop",
        "Small town tree.",
        "overlay",
        "civic_prop",
        {"variant": "tree_small"},
        ["prop", "nature"],
    ),
    _spec(
        "hedge",
        "Hedge",
        "civic_prop",
        "Garden hedge block.",
        "overlay",
        "civic_prop",
        {"variant": "hedge"},
        ["prop", "nature", "garden"],
    ),
    _spec(
        "market_stall_red",
        "Market Stall Red",
        "civic_prop",
        "Red-canopy market stall.",
        "overlay",
        "civic_prop",
        {"variant": "market_red"},
        ["prop", "market"],
    ),
    _spec(
        "market_stall_blue",
        "Market Stall Blue",
        "civic_prop",
        "Blue-canopy market stall.",
        "overlay",
        "civic_prop",
        {"variant": "market_blue"},
        ["prop", "market"],
    ),
    _spec(
        "banner_red",
        "Banner Red",
        "civic_prop",
        "Red hanging town banner.",
        "overlay",
        "civic_prop",
        {"variant": "banner_red"},
        ["prop", "banner"],
    ),
    _spec(
        "banner_blue",
        "Banner Blue",
        "civic_prop",
        "Blue hanging town banner.",
        "overlay",
        "civic_prop",
        {"variant": "banner_blue"},
        ["prop", "banner"],
    ),
]

assert len(TILES) == 96, f"Expected 96 tiles, found {len(TILES)}"


def render_tile(spec: TileSpec) -> Image.Image:
    return RENDERERS[spec.style](spec)


def build_sheet(
    tiles: Sequence[TileSpec] = TILES,
) -> Tuple[Image.Image, Dict[str, Any]]:
    cols = ATLAS_COLUMNS
    rows = math.ceil(len(tiles) / cols)
    atlas = Image.new("RGBA", (cols * OUTPUT_TILE, rows * OUTPUT_TILE), (0, 0, 0, 0))
    groups: Dict[str, List[str]] = {}
    manifest: Dict[str, Any] = {
        "target": TARGET_NAME,
        "sheet_kind": "tileset",
        "view": "side_scroller",
        "tile_width": OUTPUT_TILE,
        "tile_height": OUTPUT_TILE,
        "atlas_columns": cols,
        "atlas_rows": rows,
        "tile_count": len(tiles),
        "notes": (
            "Side-scroller town construction tileset with 96 variants. "
            "Includes terrain, foundations, facade materials, doors, windows, roofs, "
            "platform elements, and town props. Overlay-layer props are intended to be "
            "placed over ground or structure tiles."
        ),
        "tile_order": [tile.key for tile in tiles],
        "tile_groups": groups,
        "tiles": {},
    }
    for idx, spec in enumerate(tiles):
        tile = render_tile(spec)
        col = idx % cols
        row = idx // cols
        x = col * OUTPUT_TILE
        y = row * OUTPUT_TILE
        atlas.alpha_composite(tile, (x, y))
        groups.setdefault(spec.category, []).append(spec.key)
        manifest["tiles"][spec.key] = {
            "display_name": spec.display_name,
            "category": spec.category,
            "description": spec.description,
            "layer": spec.layer,
            "tags": list(spec.tags),
            "x": x,
            "y": y,
            "w": OUTPUT_TILE,
            "h": OUTPUT_TILE,
            "atlas_col": col,
            "atlas_row": row,
        }
    return atlas, manifest


def build_contact_sheet(tiles: Sequence[TileSpec] = TILES) -> Image.Image:
    cell_w = OUTPUT_TILE + 56
    cell_h = OUTPUT_TILE + 34
    cols = CONTACT_COLUMNS
    rows = math.ceil(len(tiles) / cols)
    img = Image.new("RGBA", (cols * cell_w, rows * cell_h), (18, 20, 27, 255))
    d = ImageDraw.Draw(img, "RGBA")
    name_font = _font(11)
    meta_font = _font(10)
    for idx, spec in enumerate(tiles):
        col = idx % cols
        row = idx // cols
        ox = col * cell_w
        oy = row * cell_h
        step = max(2, OUTPUT_TILE // 4)
        for yy in range(0, OUTPUT_TILE, step):
            for xx in range(0, OUTPUT_TILE, step):
                fill = (
                    (44, 47, 56, 255)
                    if ((xx // step + yy // step) % 2 == 0)
                    else (34, 37, 44, 255)
                )
                d.rectangle(
                    (ox + 8 + xx, oy + 6 + yy, ox + 8 + xx + step, oy + 6 + yy + step),
                    fill=fill,
                )
        tile = render_tile(spec)
        img.alpha_composite(tile, (ox + 8, oy + 6))
        _outline_text(
            d,
            (ox + 8, oy + OUTPUT_TILE + 10),
            spec.display_name,
            font=name_font,
            fill=(240, 242, 255, 255),
            outline=(0, 0, 0, 150),
        )
        d.text(
            (ox + 8, oy + OUTPUT_TILE + 22),
            spec.category,
            font=meta_font,
            fill=(151, 215, 235, 255),
        )
    return img


def write_outputs(out_dir: Path) -> List[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    atlas, manifest = build_sheet(TILES)
    png_path = out_dir / SHEET_FILES[0]
    yaml_path = out_dir / SHEET_FILES[1]
    contact_path = out_dir / CONTACT_FILE
    atlas.save(png_path)
    yaml_path.write_text(
        yaml.safe_dump(manifest, sort_keys=False, allow_unicode=True), encoding="utf8"
    )
    build_contact_sheet(TILES).save(contact_path)
    return [png_path, yaml_path, contact_path]


def render(out_dir: Path) -> List[Path]:
    return write_outputs(out_dir)
