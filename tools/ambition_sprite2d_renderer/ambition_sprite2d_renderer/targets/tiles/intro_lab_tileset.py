from __future__ import annotations

"""Procedural LDtk-friendly tileset for the intro laboratory.

This target emits a larger 32px tile atlas for the game's opening lab.  It is
structured for LDtk authoring rather than as a tiny prop sheet: solid / one-way
collision tiles are separated from background walls, decor, pipes, cables,
lights, doors, and interactive lab props via metadata in the YAML manifest.
"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Dict, Iterable, List, Tuple

import math
import yaml
from PIL import Image, ImageColor, ImageDraw, ImageFont

RGBA = Tuple[int, int, int, int]

TARGET_NAME = "intro_lab_tileset"
SHEET_FILES = [f"{TARGET_NAME}.png", f"{TARGET_NAME}.yaml"]

# DESIGN_TILE is the logical px the tile draw functions assume —
# coordinates and feature sizes were authored against 32×32 cells.
# OUTPUT_TILE is the final cell size in the published PNG / atlas;
# we downsample DESIGN_TILE → OUTPUT_TILE at the very end. This
# split lets the world author tiles on a 16-px grid (matching the
# Collision IntGrid) without rewriting every tile drawing.
DESIGN_TILE = 32
OUTPUT_TILE = 16
TILE = DESIGN_TILE  # alias used by the draw functions
SCALE = 4
COLS = 16
PREVIEW_PAD = 8
CANVAS_TILE = DESIGN_TILE * SCALE


@dataclass(frozen=True)
class TileSpec:
    key: str
    category: str
    collision: str
    draw: Callable[[ImageDraw.ImageDraw], None]
    description: str
    layer: str
    tags: Tuple[str, ...] = field(default_factory=tuple)


def _rgba(color: str, alpha: int = 255) -> RGBA:
    r, g, b = ImageColor.getrgb(color)
    return (r, g, b, alpha)


def _s(v: float) -> int:
    return int(round(v * SCALE))


def _pt(x: float, y: float) -> Tuple[int, int]:
    return (_s(x), _s(y))


def _box(x1: float, y1: float, x2: float, y2: float) -> Tuple[int, int, int, int]:
    return (_s(x1), _s(y1), _s(x2), _s(y2))


def _line(
    d: ImageDraw.ImageDraw,
    xy: Iterable[Tuple[float, float]],
    fill: RGBA,
    width: float = 1.0,
) -> None:
    d.line(
        [_pt(x, y) for x, y in xy], fill=fill, width=max(1, _s(width)), joint="curve"
    )


def _poly(
    d: ImageDraw.ImageDraw,
    pts: Iterable[Tuple[float, float]],
    fill: RGBA,
    outline: RGBA | None = None,
    width: float = 1.0,
) -> None:
    points = [_pt(x, y) for x, y in pts]
    d.polygon(points, fill=fill)
    if outline is not None:
        d.line(
            points + [points[0]], fill=outline, width=max(1, _s(width)), joint="curve"
        )


def _font(size: int = 7):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=max(6, int(size * SCALE)))
        except OSError:
            pass
    return ImageFont.load_default()


def _downsample(img: Image.Image) -> Image.Image:
    return img.resize((OUTPUT_TILE, OUTPUT_TILE), Image.Resampling.LANCZOS)


# Palette: cold lab surfaces with amber warning and cyan machine accents.
C_BG = _rgba("#111823")
C_BG_2 = _rgba("#182231")
C_PANEL = _rgba("#263241")
C_PANEL_2 = _rgba("#334253")
C_PANEL_3 = _rgba("#405164")
C_EDGE = _rgba("#111721")
C_HILITE = _rgba("#65798a")
C_FLOOR = _rgba("#1c252e")
C_FLOOR_2 = _rgba("#2b3641")
C_GRATE = _rgba("#151d24")
C_STEEL = _rgba("#56646d")
C_STEEL_DARK = _rgba("#2e3940")
C_CYAN = _rgba("#5de6ff")
C_CYAN_DIM = _rgba("#2c92a8")
C_CYAN_DARK = _rgba("#113845")
C_AMBER = _rgba("#ffb84c")
C_WARN = _rgba("#f06b32")
C_RED = _rgba("#d84e44")
C_GREEN = _rgba("#8af07c")
C_GLASS = _rgba("#8be6ff", 95)
C_GLASS_EDGE = _rgba("#b8f4ff", 150)
C_SHADOW = _rgba("#05070a", 120)
C_BLACK = _rgba("#05070a")
C_WOOD = _rgba("#5b4732")


def _blank_bg(d: ImageDraw.ImageDraw, color: RGBA = (0, 0, 0, 0)) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=color)


def _base_panel(d: ImageDraw.ImageDraw, *, fill: RGBA = C_PANEL) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=fill, outline=C_EDGE, width=_s(1))
    d.rectangle(_box(1.3, 1.3, 30.7, 30.7), outline=_rgba("#425164"), width=_s(0.55))
    d.line(_box(0, 31.1, 32, 31.1), fill=C_SHADOW, width=_s(0.8))


def _bolt(
    d: ImageDraw.ImageDraw, x: float, y: float, color: RGBA = _rgba("#8793a0")
) -> None:
    d.ellipse(
        _box(x - 1, y - 1, x + 1, y + 1), fill=color, outline=C_EDGE, width=_s(0.25)
    )


def _draw_hazard_stripes(
    d: ImageDraw.ImageDraw, x1: float, y1: float, x2: float, y2: float
) -> None:
    d.rectangle(
        _box(x1, y1, x2, y2),
        fill=_rgba("#2a2216"),
        outline=_rgba("#0d0b08"),
        width=_s(0.5),
    )
    for x in range(int(x1) - 12, int(x2) + 12, 7):
        pts = [(x, y1), (x + 4, y1), (x + 12, y2), (x + 8, y2)]
        _poly(d, pts, C_AMBER)


# ---------------------------------------------------------------------------
# Background wall / solid tile drawers


def _wall_plain(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(
        _box(5, 5, 27, 27), fill=C_PANEL_2, outline=_rgba("#202a36"), width=_s(0.8)
    )
    d.line(_box(7, 8, 25, 8), fill=C_HILITE, width=_s(0.7))
    for x, y in [(5.5, 5.5), (26.5, 5.5), (5.5, 26.5), (26.5, 26.5)]:
        _bolt(d, x, y)


def _wall_plain_alt(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d, fill=_rgba("#222d3b"))
    d.rectangle(
        _box(3, 3, 29, 29),
        fill=_rgba("#2b394a"),
        outline=_rgba("#171f2b"),
        width=_s(0.6),
    )
    _line(d, [(3, 17), (29, 17)], _rgba("#4d6071"), 0.45)
    _line(d, [(17, 3), (17, 29)], _rgba("#17202b"), 0.45)


def _wall_panel_light(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(
        _box(4, 5, 28, 24),
        fill=_rgba("#2c3a4d"),
        outline=_rgba("#161e29"),
        width=_s(0.8),
    )
    d.rounded_rectangle(
        _box(8, 10, 24, 15),
        radius=_s(1.5),
        fill=C_CYAN_DARK,
        outline=C_CYAN_DIM,
        width=_s(0.5),
    )
    d.rectangle(_box(9, 11, 23, 12.4), fill=_rgba("#8ff4ff", 175))
    for x in (8, 15, 22):
        d.rectangle(_box(x, 25.5, x + 2.5, 27.5), fill=_rgba("#526575"))


def _wall_dark_panel(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d, fill=_rgba("#1c2634"))
    d.rectangle(
        _box(6, 4, 26, 28),
        fill=_rgba("#151d28"),
        outline=_rgba("#344253"),
        width=_s(0.8),
    )
    for y in (9, 16, 23):
        _line(d, [(8, y), (24, y)], _rgba("#293645"), 0.55)


def _wall_pillar(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d, fill=_rgba("#202b39"))
    d.rectangle(
        _box(10, -1, 22, 33), fill=_rgba("#36475a"), outline=C_EDGE, width=_s(0.9)
    )
    d.rectangle(_box(12, 2, 20, 30), fill=_rgba("#465a70"))
    d.line(_box(10, 0, 10, 32), fill=_rgba("#718496"), width=_s(0.55))
    d.line(_box(22, 0, 22, 32), fill=C_SHADOW, width=_s(0.55))


def _wall_corner_left(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(_box(0, 0, 9, 32), fill=_rgba("#151d28"), outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(9, 4, 30, 28), fill=C_PANEL_2, outline=_rgba("#202a36"), width=_s(0.8)
    )
    d.line(_box(8, 0, 8, 32), fill=C_HILITE, width=_s(0.8))


def _wall_corner_right(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(_box(23, 0, 32, 32), fill=_rgba("#151d28"), outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(2, 4, 23, 28), fill=C_PANEL_2, outline=_rgba("#202a36"), width=_s(0.8)
    )
    d.line(_box(24, 0, 24, 32), fill=C_HILITE, width=_s(0.8))


def _wall_trim_top(d: ImageDraw.ImageDraw) -> None:
    _wall_plain_alt(d)
    d.rectangle(_box(0, 0, 32, 6), fill=_rgba("#536579"), outline=C_EDGE, width=_s(0.6))
    d.line(_box(0, 6.2, 32, 6.2), fill=_rgba("#7f93a5"), width=_s(0.5))


def _wall_trim_bottom(d: ImageDraw.ImageDraw) -> None:
    _wall_plain_alt(d)
    d.rectangle(
        _box(0, 25, 32, 32), fill=_rgba("#171f2b"), outline=C_EDGE, width=_s(0.6)
    )
    d.line(_box(0, 24.8, 32, 24.8), fill=_rgba("#7f93a5"), width=_s(0.5))


def _wall_trim_left(d: ImageDraw.ImageDraw) -> None:
    _wall_plain_alt(d)
    d.rectangle(_box(0, 0, 6, 32), fill=_rgba("#536579"), outline=C_EDGE, width=_s(0.6))
    d.line(_box(6.2, 0, 6.2, 32), fill=_rgba("#7f93a5"), width=_s(0.5))


def _wall_trim_right(d: ImageDraw.ImageDraw) -> None:
    _wall_plain_alt(d)
    d.rectangle(
        _box(26, 0, 32, 32), fill=_rgba("#171f2b"), outline=C_EDGE, width=_s(0.6)
    )
    d.line(_box(25.8, 0, 25.8, 32), fill=_rgba("#7f93a5"), width=_s(0.5))


def _wall_trim_tl(d: ImageDraw.ImageDraw) -> None:
    _wall_trim_top(d)
    d.rectangle(_box(0, 0, 6, 32), fill=_rgba("#536579"), outline=C_EDGE, width=_s(0.6))


def _wall_trim_tr(d: ImageDraw.ImageDraw) -> None:
    _wall_trim_top(d)
    d.rectangle(
        _box(26, 0, 32, 32), fill=_rgba("#171f2b"), outline=C_EDGE, width=_s(0.6)
    )


def _wall_trim_bl(d: ImageDraw.ImageDraw) -> None:
    _wall_trim_bottom(d)
    d.rectangle(_box(0, 0, 6, 32), fill=_rgba("#536579"), outline=C_EDGE, width=_s(0.6))


def _wall_trim_br(d: ImageDraw.ImageDraw) -> None:
    _wall_trim_bottom(d)
    d.rectangle(
        _box(26, 0, 32, 32), fill=_rgba("#171f2b"), outline=C_EDGE, width=_s(0.6)
    )


def _cracked_panel(d: ImageDraw.ImageDraw) -> None:
    _wall_plain(d)
    _line(d, [(17, 6), (14, 13), (20, 17), (12, 27)], _rgba("#090c10"), width=0.8)
    _line(d, [(20, 17), (25, 21), (27, 26)], _rgba("#090c10"), width=0.55)
    _line(d, [(14, 13), (8, 16), (6, 20)], _rgba("#090c10"), width=0.55)


def _wall_window(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(
        _box(4, 5, 28, 27),
        fill=_rgba("#08131d"),
        outline=_rgba("#5d7185"),
        width=_s(0.9),
    )
    d.rectangle(
        _box(7, 8, 25, 24), fill=_rgba("#102c3a"), outline=C_CYAN_DIM, width=_s(0.5)
    )
    d.line(_box(10, 9, 10, 23), fill=_rgba("#b8f4ff", 80), width=_s(0.4))
    d.line(_box(7, 19, 25, 19), fill=_rgba("#b8f4ff", 55), width=_s(0.4))


def _wall_warning_panel(d: ImageDraw.ImageDraw) -> None:
    _base_panel(d)
    d.rectangle(
        _box(5, 8, 27, 23),
        fill=_rgba("#25201a"),
        outline=_rgba("#796546"),
        width=_s(0.7),
    )
    _draw_hazard_stripes(d, 6, 11, 26, 18)
    d.text((_s(9), _s(19)), "LOW", font=_font(4.3), fill=_rgba("#ffd98f"))


# ---------------------------------------------------------------------------
# Floor / platform drawers


def _floor_plain(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_FLOOR, outline=C_EDGE, width=_s(1))
    for y in (8, 16, 24):
        d.line(_box(0, y, 32, y), fill=_rgba("#303b45"), width=_s(0.5))
    for x in (10, 21):
        d.line(_box(x, 0, x - 3, 32), fill=_rgba("#111922"), width=_s(0.45))
    d.rectangle(_box(2, 2, 30, 30), outline=_rgba("#3d4853"), width=_s(0.4))


def _floor_plain_alt(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=_rgba("#222b34"), outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(2, 2, 30, 30),
        fill=_rgba("#26313b"),
        outline=_rgba("#121921"),
        width=_s(0.4),
    )
    _line(d, [(2, 16), (30, 15)], _rgba("#3e4b56"), 0.45)


def _floor_grate(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_FLOOR_2, outline=C_EDGE, width=_s(1))
    d.rectangle(_box(4, 4, 28, 28), fill=C_GRATE, outline=_rgba("#44515d"), width=_s(1))
    for x in range(7, 28, 5):
        d.line(_box(x, 5, x - 5, 27), fill=_rgba("#5d6a76"), width=_s(0.7))
    for y in (10, 22):
        d.line(_box(5, y, 27, y), fill=_rgba("#0b1117"), width=_s(0.45))


def _floor_hazard(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    _draw_hazard_stripes(d, 0, 20, 32, 28)
    d.line(_box(0, 20, 32, 20), fill=_rgba("#ffd37a"), width=_s(0.45))


def _floor_hazard_left(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    _draw_hazard_stripes(d, 0, 0, 8, 32)


def _floor_hazard_right(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    _draw_hazard_stripes(d, 24, 0, 32, 32)


def _floor_stain(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    d.ellipse(_box(7, 18, 23, 27), fill=_rgba("#0a0d0c", 120))
    d.ellipse(_box(11, 13, 18, 18), fill=_rgba("#1b120b", 110))
    d.line(_box(16, 18, 27, 22), fill=_rgba("#0a0d0c", 95), width=_s(0.7))


def _floor_cracked(d: ImageDraw.ImageDraw) -> None:
    _floor_plain_alt(d)
    _line(d, [(5, 7), (12, 15), (11, 25), (20, 30)], C_BLACK, 0.55)
    _line(d, [(12, 15), (21, 13), (27, 18)], C_BLACK, 0.45)


def _floor_cable_trench(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    d.rectangle(
        _box(0, 12, 32, 21),
        fill=_rgba("#0b1017"),
        outline=_rgba("#3d4853"),
        width=_s(0.5),
    )
    for y, c in [(14, C_CYAN), (17, C_AMBER), (19, C_GREEN)]:
        _line(d, [(0, y), (10, y + 2), (21, y - 1), (32, y + 2)], c, 0.7)


def _floor_drain(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    d.ellipse(
        _box(8, 8, 24, 24),
        fill=_rgba("#0c1219"),
        outline=_rgba("#596673"),
        width=_s(0.8),
    )
    for a in range(0, 360, 45):
        x = 16 + math.cos(math.radians(a)) * 7
        y = 16 + math.sin(math.radians(a)) * 7
        _line(d, [(16, 16), (x, y)], _rgba("#4f5d6b"), 0.45)


def _platform_piece(kind: str) -> Callable[[ImageDraw.ImageDraw], None]:
    def draw(d: ImageDraw.ImageDraw) -> None:
        _blank_bg(d)
        if kind == "single":
            x1, x2, rad = 4, 28, 4
        elif kind == "left":
            x1, x2, rad = 0, 34, 2.5
        elif kind == "right":
            x1, x2, rad = -2, 32, 2.5
        else:
            x1, x2, rad = 0, 32, 0
        d.rounded_rectangle(
            _box(x1, 11, x2, 22),
            radius=_s(rad),
            fill=_rgba("#394653"),
            outline=C_EDGE,
            width=_s(1),
        )
        d.rectangle(_box(0, 10, 32, 13), fill=_rgba("#667887"))
        d.rectangle(_box(0, 22, 32, 25), fill=C_SHADOW)
        for x in (7, 19):
            if (
                kind in {"single", "mid"}
                or (kind == "left" and x == 7)
                or (kind == "right" and x == 19)
            ):
                d.rectangle(
                    _box(x, 14, x + 6, 20),
                    fill=_rgba("#24303b"),
                    outline=_rgba("#121820"),
                    width=_s(0.4),
                )

    return draw


def _platform_support(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(12, 0, 20, 32), fill=_rgba("#323f4b"), outline=C_EDGE, width=_s(0.8)
    )
    for y in range(4, 31, 7):
        d.line(_box(12, y, 20, y + 5), fill=_rgba("#617283"), width=_s(0.45))


# ---------------------------------------------------------------------------
# Decor / prop drawers


def _pipe_horizontal(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(-2, 12, 34, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rectangle(
        _box(10, 10, 15, 23), fill=C_STEEL_DARK, outline=_rgba("#121820"), width=_s(0.5)
    )
    d.line(_box(0, 13, 32, 13), fill=_rgba("#8a98a0"), width=_s(0.5))


def _pipe_vertical(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(12, -2, 21, 34),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rectangle(
        _box(10, 10, 23, 15), fill=C_STEEL_DARK, outline=_rgba("#121820"), width=_s(0.5)
    )
    d.line(_box(13, 0, 13, 32), fill=_rgba("#8a98a0"), width=_s(0.5))


def _pipe_corner_tl(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(12, 12, 34, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rounded_rectangle(
        _box(12, -2, 21, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.ellipse(
        _box(10, 10, 23, 23),
        fill=_rgba("#687681"),
        outline=_rgba("#141a20"),
        width=_s(0.8),
    )


def _pipe_corner_tr(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(-2, 12, 20, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rounded_rectangle(
        _box(12, -2, 21, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.ellipse(
        _box(9, 10, 22, 23),
        fill=_rgba("#687681"),
        outline=_rgba("#141a20"),
        width=_s(0.8),
    )


def _pipe_corner_bl(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(12, 12, 34, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rounded_rectangle(
        _box(12, 12, 21, 34),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.ellipse(
        _box(10, 10, 23, 23),
        fill=_rgba("#687681"),
        outline=_rgba("#141a20"),
        width=_s(0.8),
    )


def _pipe_corner_br(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(-2, 12, 20, 21),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.rounded_rectangle(
        _box(12, 12, 21, 34),
        radius=_s(4),
        fill=C_STEEL,
        outline=_rgba("#141a20"),
        width=_s(1),
    )
    d.ellipse(
        _box(9, 10, 22, 23),
        fill=_rgba("#687681"),
        outline=_rgba("#141a20"),
        width=_s(0.8),
    )


def _pipe_valve(d: ImageDraw.ImageDraw) -> None:
    _pipe_horizontal(d)
    d.ellipse(_box(9, 6, 23, 20), fill=_rgba("#6c7782"), outline=C_EDGE, width=_s(0.7))
    d.ellipse(_box(12, 9, 20, 17), outline=C_AMBER, width=_s(0.8))
    _line(d, [(16, 6), (16, 20)], C_AMBER, 0.5)
    _line(d, [(9, 13), (23, 13)], C_AMBER, 0.5)


def _cable_bundle(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    for offset, color in [
        (0, "#4ad7ff"),
        (3, "#ffb84c"),
        (6, "#8af07c"),
        (9, "#c89cff"),
    ]:
        pts = [(0, 7 + offset), (9, 13 + offset), (19, 8 + offset), (32, 16 + offset)]
        _line(d, pts, _rgba(color, 190), width=1.1)
        _line(d, [(x, y + 1.6) for x, y in pts], _rgba("#06080b", 95), width=0.55)
    d.rectangle(
        _box(5, 22, 28, 27), fill=_rgba("#212a34"), outline=C_EDGE, width=_s(0.6)
    )


def _cable_vertical(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    for x, color in [(10, C_CYAN), (15, C_AMBER), (20, C_GREEN)]:
        _line(d, [(x, -2), (x + 2, 9), (x - 1, 19), (x + 1, 34)], color, 0.9)
    d.rectangle(
        _box(7, 13, 24, 18), fill=_rgba("#212a34"), outline=C_EDGE, width=_s(0.55)
    )


def _vent(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 6, 27, 26), fill=_rgba("#101720"), outline=_rgba("#536270"), width=_s(1)
    )
    for y in range(9, 25, 4):
        d.rectangle(_box(7, y, 25, y + 1.4), fill=_rgba("#697987"))
    d.rectangle(_box(7, 7, 25, 9), fill=_rgba("#252f3a"))


def _fan_vent(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.ellipse(
        _box(4, 4, 28, 28),
        fill=_rgba("#0e151d"),
        outline=_rgba("#596a7b"),
        width=_s(0.9),
    )
    d.ellipse(_box(13, 13, 19, 19), fill=_rgba("#53616f"))
    for a in range(0, 360, 60):
        pts = [
            (16, 16),
            (16 + math.cos(math.radians(a)) * 10, 16 + math.sin(math.radians(a)) * 10),
            (
                16 + math.cos(math.radians(a + 28)) * 7,
                16 + math.sin(math.radians(a + 28)) * 7,
            ),
        ]
        _poly(d, pts, _rgba("#35414d"))


def _terminal(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(5, 5, 27, 27),
        radius=_s(2),
        fill=_rgba("#172230"),
        outline=_rgba("#667888"),
        width=_s(1),
    )
    d.rectangle(
        _box(8, 8, 24, 17), fill=_rgba("#062836"), outline=C_CYAN_DIM, width=_s(0.5)
    )
    d.rectangle(_box(9, 9, 18, 11), fill=_rgba("#8ff4ff", 180))
    d.rectangle(_box(9, 13, 22, 14), fill=_rgba("#5de6ff", 115))
    for x, c in [(9, C_AMBER), (14, C_CYAN), (19, C_WARN)]:
        d.ellipse(_box(x, 20, x + 2.8, 22.8), fill=c)


def _big_monitor(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(3, 5, 29, 24),
        radius=_s(2),
        fill=_rgba("#101720"),
        outline=_rgba("#657280"),
        width=_s(0.9),
    )
    d.rectangle(
        _box(6, 8, 26, 20), fill=_rgba("#07202d"), outline=C_CYAN_DIM, width=_s(0.45)
    )
    _line(d, [(8, 17), (12, 13), (15, 15), (18, 10), (23, 13)], C_CYAN, 0.55)
    d.rectangle(
        _box(12, 24, 20, 27), fill=_rgba("#4e5965"), outline=C_EDGE, width=_s(0.4)
    )
    d.rectangle(
        _box(9, 27, 23, 29), fill=_rgba("#2a333d"), outline=C_EDGE, width=_s(0.4)
    )


def _switch_box(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(7, 5, 25, 27),
        fill=_rgba("#2a3542"),
        outline=_rgba("#687887"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(10, 8, 22, 12), fill=_rgba("#151d25"), outline=C_CYAN_DIM, width=_s(0.4)
    )
    for i, c in enumerate([C_GREEN, C_AMBER, C_RED]):
        y = 16 + i * 4
        d.ellipse(_box(10, y, 13, y + 3), fill=c)
        d.rectangle(_box(15, y + 0.8, 22, y + 2), fill=_rgba("#8190a0"))


def _glass_tube_top(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(8, 1, 24, 31),
        radius=_s(5),
        fill=C_GLASS,
        outline=C_GLASS_EDGE,
        width=_s(1),
    )
    d.rectangle(_box(6, 0, 26, 6), fill=_rgba("#314250"), outline=C_EDGE, width=_s(0.8))
    d.line(_box(12, 7, 12, 28), fill=_rgba("#e6ffff", 80), width=_s(0.5))
    d.ellipse(_box(14, 16, 18, 20), fill=_rgba("#9affff", 120))


def _glass_tube_mid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(8, -2, 24, 34),
        radius=_s(5),
        fill=C_GLASS,
        outline=C_GLASS_EDGE,
        width=_s(1),
    )
    d.line(_box(12, 0, 12, 32), fill=_rgba("#e6ffff", 80), width=_s(0.5))
    for y in (9, 19, 27):
        d.ellipse(_box(16, y, 18, y + 2), fill=_rgba("#bafcff", 135))


def _glass_tube_bottom(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rounded_rectangle(
        _box(8, -2, 24, 28),
        radius=_s(5),
        fill=C_GLASS,
        outline=C_GLASS_EDGE,
        width=_s(1),
    )
    d.rectangle(
        _box(6, 26, 26, 32), fill=_rgba("#314250"), outline=C_EDGE, width=_s(0.8)
    )
    d.line(_box(12, 1, 12, 25), fill=_rgba("#e6ffff", 80), width=_s(0.5))


def _glass_tube_specimen(d: ImageDraw.ImageDraw) -> None:
    _glass_tube_mid(d)
    d.ellipse(
        _box(12, 13, 20, 23),
        fill=_rgba("#bcff9d", 110),
        outline=_rgba("#edffe1", 110),
        width=_s(0.35),
    )
    _line(d, [(16, 14), (16, 22), (13, 19), (19, 18)], _rgba("#e8ffdf", 110), 0.35)


def _lab_door_top(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_BG, outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(4, 2, 28, 32), fill=_rgba("#25313f"), outline=_rgba("#6c7884"), width=_s(1)
    )
    d.rectangle(
        _box(8, 5, 24, 11), fill=_rgba("#101821"), outline=C_CYAN_DIM, width=_s(0.5)
    )
    d.text((_s(10), _s(4.2)), "01", font=_font(5), fill=_rgba("#9df8ff"))


def _lab_door_bottom(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_BG, outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(4, 0, 28, 32), fill=_rgba("#25313f"), outline=_rgba("#6c7884"), width=_s(1)
    )
    d.line(_box(16, 0, 16, 32), fill=_rgba("#111820"), width=_s(1.2))
    d.rectangle(
        _box(20, 14, 24, 18), fill=C_AMBER, outline=_rgba("#26190a"), width=_s(0.4)
    )


def _blast_door_left(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_BG, outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(3, 2, 32, 30),
        fill=_rgba("#2b3440"),
        outline=_rgba("#778491"),
        width=_s(0.8),
    )
    _draw_hazard_stripes(d, 3, 3, 10, 29)
    d.rectangle(
        _box(12, 6, 29, 26),
        fill=_rgba("#1a222c"),
        outline=_rgba("#566675"),
        width=_s(0.5),
    )


def _blast_door_right(d: ImageDraw.ImageDraw) -> None:
    d.rectangle(_box(0, 0, 32, 32), fill=C_BG, outline=C_EDGE, width=_s(1))
    d.rectangle(
        _box(0, 2, 29, 30),
        fill=_rgba("#2b3440"),
        outline=_rgba("#778491"),
        width=_s(0.8),
    )
    _draw_hazard_stripes(d, 22, 3, 29, 29)
    d.rectangle(
        _box(3, 6, 20, 26),
        fill=_rgba("#1a222c"),
        outline=_rgba("#566675"),
        width=_s(0.5),
    )


def _gate_socket(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.ellipse(
        _box(5, 5, 27, 27), fill=_rgba("#15202b"), outline=_rgba("#7d8792"), width=_s(1)
    )
    d.ellipse(_box(9, 9, 23, 23), outline=C_CYAN_DIM, width=_s(1.2))
    d.ellipse(
        _box(13, 13, 19, 19), fill=_rgba("#ff9d43", 150), outline=C_AMBER, width=_s(0.5)
    )
    for a in range(0, 360, 60):
        x1 = 16 + math.cos(math.radians(a)) * 8
        y1 = 16 + math.sin(math.radians(a)) * 8
        x2 = 16 + math.cos(math.radians(a)) * 11
        y2 = 16 + math.sin(math.radians(a)) * 11
        _line(d, [(x1, y1), (x2, y2)], C_CYAN, width=0.5)


def _small_crate(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.ellipse(_box(5, 25, 27, 30), fill=(0, 0, 0, 45))
    d.rectangle(_box(7, 12, 25, 27), fill=C_WOOD, outline=_rgba("#20150d"), width=_s(1))
    d.line(_box(7, 17, 25, 17), fill=_rgba("#8b6a43"), width=_s(1))
    d.line(_box(12, 12, 12, 27), fill=_rgba("#2e2117"), width=_s(0.7))
    d.line(_box(20, 12, 20, 27), fill=_rgba("#2e2117"), width=_s(0.7))


def _barrel(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.ellipse(_box(8, 9, 24, 14), fill=_rgba("#4b5660"), outline=C_EDGE, width=_s(0.7))
    d.rectangle(
        _box(8, 11, 24, 27), fill=_rgba("#3c4853"), outline=C_EDGE, width=_s(0.8)
    )
    d.ellipse(_box(8, 24, 24, 29), fill=_rgba("#303a44"), outline=C_EDGE, width=_s(0.7))
    d.rectangle(_box(8, 15, 24, 18), fill=_rgba("#727f8b"))


def _sign_arrow(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 9, 27, 23),
        fill=_rgba("#222b32"),
        outline=_rgba("#697987"),
        width=_s(0.8),
    )
    _poly(
        d,
        [(10, 14), (18, 14), (18, 12), (25, 16), (18, 20), (18, 18), (10, 18)],
        C_AMBER,
    )
    d.text((_s(7.5), _s(9.5)), "EXIT", font=_font(4.6), fill=_rgba("#e8f4ff"))


def _warning_sign(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(6, 8, 26, 24), fill=_rgba("#251d13"), outline=C_AMBER, width=_s(0.8)
    )
    _poly(d, [(16, 10), (23, 22), (9, 22)], C_AMBER, _rgba("#1a1208"), 0.45)
    d.rectangle(_box(15.2, 14, 16.8, 19), fill=_rgba("#241609"))
    d.rectangle(_box(15.2, 20.2, 16.8, 21.4), fill=_rgba("#241609"))


def _ceiling_light(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(_box(4, 4, 28, 9), fill=_rgba("#53606d"), outline=C_EDGE, width=_s(0.6))
    d.rounded_rectangle(
        _box(7, 9, 25, 14),
        radius=_s(1.7),
        fill=_rgba("#c8fbff", 190),
        outline=C_CYAN_DIM,
        width=_s(0.4),
    )
    for y, a in [(15, 55), (19, 35), (23, 20)]:
        d.rectangle(_box(8, y, 24, y + 2), fill=_rgba("#8befff", a))


def _red_warning_light(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(10, 7, 22, 13), fill=_rgba("#3a4047"), outline=C_EDGE, width=_s(0.5)
    )
    d.ellipse(_box(9, 12, 23, 26), fill=_rgba("#6a1d1f"), outline=C_EDGE, width=_s(0.7))
    d.ellipse(_box(12, 15, 20, 23), fill=_rgba("#ff5f4a", 210))


def _lab_table_left(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(0, 15, 34, 22), fill=_rgba("#4a5864"), outline=C_EDGE, width=_s(0.8)
    )
    d.rectangle(
        _box(4, 22, 8, 30), fill=_rgba("#2a333c"), outline=C_EDGE, width=_s(0.4)
    )
    d.rectangle(
        _box(24, 22, 28, 30), fill=_rgba("#2a333c"), outline=C_EDGE, width=_s(0.4)
    )
    d.rectangle(
        _box(7, 11, 15, 15), fill=_rgba("#23313e"), outline=C_CYAN_DIM, width=_s(0.4)
    )


def _lab_table_mid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(-1, 15, 33, 22), fill=_rgba("#4a5864"), outline=C_EDGE, width=_s(0.8)
    )
    d.rectangle(
        _box(12, 22, 16, 30), fill=_rgba("#2a333c"), outline=C_EDGE, width=_s(0.4)
    )
    d.rectangle(
        _box(20, 10, 28, 15), fill=_rgba("#522e30"), outline=C_EDGE, width=_s(0.4)
    )


def _lab_table_right(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(-2, 15, 32, 22), fill=_rgba("#4a5864"), outline=C_EDGE, width=_s(0.8)
    )
    d.rectangle(
        _box(4, 22, 8, 30), fill=_rgba("#2a333c"), outline=C_EDGE, width=_s(0.4)
    )
    d.rectangle(
        _box(24, 22, 28, 30), fill=_rgba("#2a333c"), outline=C_EDGE, width=_s(0.4)
    )
    d.ellipse(
        _box(15, 8, 20, 15),
        fill=_rgba("#a8f3ff", 125),
        outline=C_CYAN_DIM,
        width=_s(0.4),
    )


def _locker(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(7, 3, 25, 30),
        fill=_rgba("#2c3947"),
        outline=_rgba("#657280"),
        width=_s(0.8),
    )
    d.line(_box(16, 4, 16, 29), fill=C_EDGE, width=_s(0.55))
    for x in (11, 20):
        d.rectangle(_box(x, 8, x + 3, 10), fill=_rgba("#71808f"))
        d.rectangle(_box(x, 16, x + 3, 18), fill=_rgba("#71808f"))


def _bg_dark_panel(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d, C_BG)
    d.rectangle(
        _box(2, 2, 30, 30),
        fill=_rgba("#131d2a"),
        outline=_rgba("#283545"),
        width=_s(0.45),
    )
    for y in (10, 21):
        _line(d, [(2, y), (30, y)], _rgba("#202c39"), 0.35)


def _bg_girder_cross(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d, C_BG)
    d.rectangle(_box(0, 0, 32, 32), outline=_rgba("#273645"), width=_s(0.45))
    _line(d, [(0, 0), (32, 32)], _rgba("#2c3b49"), 1.2)
    _line(d, [(32, 0), (0, 32)], _rgba("#1b2632"), 1.2)
    _line(d, [(0, 16), (32, 16)], _rgba("#526272"), 0.45)


def _bg_window_left(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d, C_BG)
    d.rectangle(
        _box(4, 5, 32, 28),
        fill=_rgba("#07101a"),
        outline=_rgba("#596b7e"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(7, 8, 32, 25), fill=_rgba("#092536"), outline=C_CYAN_DIM, width=_s(0.4)
    )
    _line(d, [(12, 8), (12, 25)], _rgba("#b8f4ff", 65), 0.35)


def _bg_window_mid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d, C_BG)
    d.rectangle(
        _box(0, 5, 32, 28),
        fill=_rgba("#07101a"),
        outline=_rgba("#596b7e"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(0, 8, 32, 25), fill=_rgba("#092536"), outline=C_CYAN_DIM, width=_s(0.4)
    )
    _line(d, [(0, 19), (32, 19)], _rgba("#b8f4ff", 50), 0.35)


def _bg_window_right(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d, C_BG)
    d.rectangle(
        _box(0, 5, 28, 28),
        fill=_rgba("#07101a"),
        outline=_rgba("#596b7e"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(0, 8, 25, 25), fill=_rgba("#092536"), outline=C_CYAN_DIM, width=_s(0.4)
    )
    _line(d, [(20, 8), (20, 25)], _rgba("#b8f4ff", 65), 0.35)


def _ceiling_rail_left(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(0, 5, 32, 13), fill=_rgba("#364451"), outline=C_EDGE, width=_s(0.65)
    )
    d.rectangle(
        _box(3, 13, 9, 24), fill=_rgba("#252f39"), outline=C_EDGE, width=_s(0.45)
    )
    _draw_hazard_stripes(d, 10, 6, 30, 10)


def _ceiling_rail_mid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(0, 5, 32, 13), fill=_rgba("#364451"), outline=C_EDGE, width=_s(0.65)
    )
    d.rectangle(
        _box(13, 13, 19, 24), fill=_rgba("#252f39"), outline=C_EDGE, width=_s(0.45)
    )
    d.line(_box(0, 9, 32, 9), fill=_rgba("#71808f"), width=_s(0.35))


def _ceiling_rail_right(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(0, 5, 32, 13), fill=_rgba("#364451"), outline=C_EDGE, width=_s(0.65)
    )
    d.rectangle(
        _box(23, 13, 29, 24), fill=_rgba("#252f39"), outline=C_EDGE, width=_s(0.45)
    )
    _draw_hazard_stripes(d, 2, 6, 22, 10)


def _hanging_cable(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    _line(d, [(13, -2), (12, 7), (15, 15), (14, 23), (17, 34)], C_CYAN, 0.7)
    _line(d, [(18, -2), (20, 8), (18, 18), (21, 30), (20, 34)], C_AMBER, 0.7)
    d.ellipse(
        _box(11, 23, 18, 30), fill=_rgba("#2b3540"), outline=C_EDGE, width=_s(0.45)
    )


def _machine_core_top(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 8, 27, 32),
        fill=_rgba("#293542"),
        outline=_rgba("#637383"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(8, 11, 24, 17), fill=C_CYAN_DARK, outline=C_CYAN_DIM, width=_s(0.4)
    )
    d.ellipse(
        _box(12, 20, 20, 28), fill=_rgba("#8befff", 130), outline=C_CYAN, width=_s(0.4)
    )


def _machine_core_mid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 0, 27, 32),
        fill=_rgba("#293542"),
        outline=_rgba("#637383"),
        width=_s(0.8),
    )
    d.ellipse(
        _box(9, 8, 23, 22), fill=_rgba("#173544"), outline=C_CYAN_DIM, width=_s(0.8)
    )
    d.ellipse(_box(13, 12, 19, 18), fill=_rgba("#8befff", 175))


def _machine_core_bottom(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 0, 27, 25),
        fill=_rgba("#293542"),
        outline=_rgba("#637383"),
        width=_s(0.8),
    )
    d.rectangle(
        _box(2, 24, 30, 31), fill=_rgba("#1a222c"), outline=C_EDGE, width=_s(0.55)
    )
    for x in (8, 16, 24):
        d.rectangle(_box(x, 21, x + 2, 28), fill=C_AMBER)


def _track_left(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(0, 20, 32, 25), fill=_rgba("#303944"), outline=C_EDGE, width=_s(0.5)
    )
    d.line(_box(0, 17, 32, 17), fill=_rgba("#76818c"), width=_s(0.7))
    d.line(_box(0, 28, 32, 28), fill=_rgba("#76818c"), width=_s(0.7))
    for x in (7, 17, 27):
        d.rectangle(_box(x, 17, x + 3, 28), fill=_rgba("#202831"))


def _track_mid(d: ImageDraw.ImageDraw) -> None:
    _track_left(d)


def _track_right(d: ImageDraw.ImageDraw) -> None:
    _track_left(d)


def _floor_arrow(d: ImageDraw.ImageDraw) -> None:
    _floor_plain(d)
    _poly(
        d,
        [(9, 12), (19, 12), (19, 9), (26, 16), (19, 23), (19, 20), (9, 20)],
        _rgba("#ffcf70", 150),
        _rgba("#281b0a", 180),
        0.4,
    )


def _floor_bolt_plate(d: ImageDraw.ImageDraw) -> None:
    _floor_plain_alt(d)
    d.rectangle(
        _box(7, 7, 25, 25),
        fill=_rgba("#303c48"),
        outline=_rgba("#607181"),
        width=_s(0.6),
    )
    for x, y in [(9, 9), (23, 9), (9, 23), (23, 23)]:
        _bolt(d, x, y)


def _tool_rack(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(5, 6, 27, 25),
        fill=_rgba("#1a222b"),
        outline=_rgba("#657280"),
        width=_s(0.65),
    )
    d.line(_box(7, 12, 25, 12), fill=_rgba("#7a8793"), width=_s(0.5))
    for x in (10, 15, 20):
        d.line(_box(x, 12, x - 1, 22), fill=_rgba("#c0a35f"), width=_s(0.55))
        d.ellipse(
            _box(x - 1.8, 20, x + 1.8, 24), outline=_rgba("#c0a35f"), width=_s(0.45)
        )


def _whiteboard(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(3, 6, 29, 23),
        fill=_rgba("#d8e7e8"),
        outline=_rgba("#4d5963"),
        width=_s(0.6),
    )
    _line(d, [(7, 17), (11, 12), (15, 16), (21, 9), (26, 12)], _rgba("#2d4f62"), 0.4)
    d.rectangle(_box(6, 24, 26, 26), fill=_rgba("#4d5963"))


def _first_aid(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.rectangle(
        _box(7, 8, 25, 24),
        fill=_rgba("#e7e9e5"),
        outline=_rgba("#59636c"),
        width=_s(0.7),
    )
    d.rectangle(_box(14, 11, 18, 21), fill=C_RED)
    d.rectangle(_box(11, 14, 21, 18), fill=C_RED)


def _spilled_beaker(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)
    d.polygon(
        [_pt(10, 12), _pt(18, 14), _pt(15, 22), _pt(7, 20)],
        fill=_rgba("#b8f4ff", 90),
        outline=C_GLASS_EDGE,
    )
    d.ellipse(_box(15, 21, 28, 27), fill=_rgba("#72f5cc", 115))
    for x, y in [(23, 18), (27, 23), (19, 26)]:
        d.ellipse(_box(x, y, x + 1.5, y + 1.5), fill=_rgba("#c8ffef", 160))


def _blank(d: ImageDraw.ImageDraw) -> None:
    _blank_bg(d)


TILES: List[TileSpec] = [
    # LDtk solid / wall authoring set.
    TileSpec(
        "wall_plain",
        "wall",
        "solid",
        _wall_plain,
        "plain segmented lab wall",
        "LabSolids",
        ("wall", "solid", "background"),
    ),
    TileSpec(
        "wall_plain_alt",
        "wall",
        "solid",
        _wall_plain_alt,
        "alternate segmented lab wall",
        "LabSolids",
        ("wall", "solid", "variant"),
    ),
    TileSpec(
        "wall_panel_light",
        "wall",
        "solid",
        _wall_panel_light,
        "wall panel with cyan inspection light",
        "LabSolids",
        ("wall", "solid", "light"),
    ),
    TileSpec(
        "wall_dark_panel",
        "wall",
        "solid",
        _wall_dark_panel,
        "dark inset wall panel",
        "LabSolids",
        ("wall", "solid", "variant"),
    ),
    TileSpec(
        "wall_pillar",
        "wall",
        "solid",
        _wall_pillar,
        "vertical structural pillar",
        "LabSolids",
        ("wall", "solid", "pillar"),
    ),
    TileSpec(
        "wall_corner_left",
        "wall",
        "solid",
        _wall_corner_left,
        "left wall edge / corner cap",
        "LabSolids",
        ("wall", "edge", "left"),
    ),
    TileSpec(
        "wall_corner_right",
        "wall",
        "solid",
        _wall_corner_right,
        "right wall edge / corner cap",
        "LabSolids",
        ("wall", "edge", "right"),
    ),
    TileSpec(
        "wall_trim_top",
        "wall",
        "solid",
        _wall_trim_top,
        "wall tile with top trim",
        "LabSolids",
        ("wall", "trim", "top"),
    ),
    TileSpec(
        "wall_trim_bottom",
        "wall",
        "solid",
        _wall_trim_bottom,
        "wall tile with bottom trim",
        "LabSolids",
        ("wall", "trim", "bottom"),
    ),
    TileSpec(
        "wall_trim_left",
        "wall",
        "solid",
        _wall_trim_left,
        "wall tile with left trim",
        "LabSolids",
        ("wall", "trim", "left"),
    ),
    TileSpec(
        "wall_trim_right",
        "wall",
        "solid",
        _wall_trim_right,
        "wall tile with right trim",
        "LabSolids",
        ("wall", "trim", "right"),
    ),
    TileSpec(
        "wall_trim_tl",
        "wall",
        "solid",
        _wall_trim_tl,
        "top-left trimmed wall corner",
        "LabSolids",
        ("wall", "trim", "corner"),
    ),
    TileSpec(
        "wall_trim_tr",
        "wall",
        "solid",
        _wall_trim_tr,
        "top-right trimmed wall corner",
        "LabSolids",
        ("wall", "trim", "corner"),
    ),
    TileSpec(
        "wall_trim_bl",
        "wall",
        "solid",
        _wall_trim_bl,
        "bottom-left trimmed wall corner",
        "LabSolids",
        ("wall", "trim", "corner"),
    ),
    TileSpec(
        "wall_trim_br",
        "wall",
        "solid",
        _wall_trim_br,
        "bottom-right trimmed wall corner",
        "LabSolids",
        ("wall", "trim", "corner"),
    ),
    TileSpec(
        "cracked_panel",
        "wall",
        "solid",
        _cracked_panel,
        "damaged lab wall panel",
        "LabSolids",
        ("wall", "damage"),
    ),
    TileSpec(
        "wall_window",
        "wall",
        "solid",
        _wall_window,
        "dark observation window wall tile",
        "LabBackground",
        ("wall", "window"),
    ),
    TileSpec(
        "wall_warning_panel",
        "wall",
        "solid",
        _wall_warning_panel,
        "wall caution placard",
        "LabBackground",
        ("wall", "warning"),
    ),
    # Floors and one-way platforms.
    TileSpec(
        "floor_plain",
        "floor",
        "solid",
        _floor_plain,
        "standard rubberized lab floor",
        "LabSolids",
        ("floor", "solid"),
    ),
    TileSpec(
        "floor_plain_alt",
        "floor",
        "solid",
        _floor_plain_alt,
        "alternate rubberized lab floor",
        "LabSolids",
        ("floor", "solid", "variant"),
    ),
    TileSpec(
        "floor_grate",
        "floor",
        "solid",
        _floor_grate,
        "metal grate floor tile",
        "LabSolids",
        ("floor", "solid", "grate"),
    ),
    TileSpec(
        "floor_hazard",
        "floor",
        "solid",
        _floor_hazard,
        "hazard-striped floor edge",
        "LabSolids",
        ("floor", "hazard"),
    ),
    TileSpec(
        "floor_hazard_left",
        "floor",
        "solid",
        _floor_hazard_left,
        "left hazard edge floor tile",
        "LabSolids",
        ("floor", "hazard", "left"),
    ),
    TileSpec(
        "floor_hazard_right",
        "floor",
        "solid",
        _floor_hazard_right,
        "right hazard edge floor tile",
        "LabSolids",
        ("floor", "hazard", "right"),
    ),
    TileSpec(
        "floor_stain",
        "floor",
        "solid",
        _floor_stain,
        "oil-stained floor variation",
        "LabSolids",
        ("floor", "stain"),
    ),
    TileSpec(
        "floor_cracked",
        "floor",
        "solid",
        _floor_cracked,
        "cracked floor variation",
        "LabSolids",
        ("floor", "damage"),
    ),
    TileSpec(
        "floor_cable_trench",
        "floor",
        "solid",
        _floor_cable_trench,
        "floor tile with exposed cable trench",
        "LabSolids",
        ("floor", "cable"),
    ),
    TileSpec(
        "floor_drain",
        "floor",
        "solid",
        _floor_drain,
        "round floor drain",
        "LabSolids",
        ("floor", "drain"),
    ),
    TileSpec(
        "platform_single",
        "platform",
        "one_way",
        _platform_piece("single"),
        "single raised platform tile",
        "LabPlatforms",
        ("platform", "one_way"),
    ),
    TileSpec(
        "platform_left",
        "platform",
        "one_way",
        _platform_piece("left"),
        "left cap for raised platform",
        "LabPlatforms",
        ("platform", "one_way", "left"),
    ),
    TileSpec(
        "platform_mid",
        "platform",
        "one_way",
        _platform_piece("mid"),
        "middle raised platform tile",
        "LabPlatforms",
        ("platform", "one_way", "mid"),
    ),
    TileSpec(
        "platform_right",
        "platform",
        "one_way",
        _platform_piece("right"),
        "right cap for raised platform",
        "LabPlatforms",
        ("platform", "one_way", "right"),
    ),
    TileSpec(
        "platform_support",
        "platform",
        "none",
        _platform_support,
        "vertical platform support strut",
        "LabDecorBack",
        ("platform", "support"),
    ),
    # Pipes, cables, vents, and machine decor.
    TileSpec(
        "pipe_horizontal",
        "pipe",
        "none",
        _pipe_horizontal,
        "horizontal service pipe",
        "LabDecorBack",
        ("pipe", "horizontal"),
    ),
    TileSpec(
        "pipe_vertical",
        "pipe",
        "none",
        _pipe_vertical,
        "vertical service pipe",
        "LabDecorBack",
        ("pipe", "vertical"),
    ),
    TileSpec(
        "pipe_corner_tl",
        "pipe",
        "none",
        _pipe_corner_tl,
        "service pipe corner top-left",
        "LabDecorBack",
        ("pipe", "corner"),
    ),
    TileSpec(
        "pipe_corner_tr",
        "pipe",
        "none",
        _pipe_corner_tr,
        "service pipe corner top-right",
        "LabDecorBack",
        ("pipe", "corner"),
    ),
    TileSpec(
        "pipe_corner_bl",
        "pipe",
        "none",
        _pipe_corner_bl,
        "service pipe corner bottom-left",
        "LabDecorBack",
        ("pipe", "corner"),
    ),
    TileSpec(
        "pipe_corner_br",
        "pipe",
        "none",
        _pipe_corner_br,
        "service pipe corner bottom-right",
        "LabDecorBack",
        ("pipe", "corner"),
    ),
    TileSpec(
        "pipe_valve",
        "pipe",
        "none",
        _pipe_valve,
        "service pipe with valve wheel",
        "LabDecorBack",
        ("pipe", "valve"),
    ),
    TileSpec(
        "cable_bundle",
        "decor",
        "none",
        _cable_bundle,
        "horizontal exposed cable bundle",
        "LabDecorBack",
        ("cable", "horizontal"),
    ),
    TileSpec(
        "cable_vertical",
        "decor",
        "none",
        _cable_vertical,
        "vertical exposed cable bundle",
        "LabDecorBack",
        ("cable", "vertical"),
    ),
    TileSpec(
        "vent",
        "decor",
        "none",
        _vent,
        "wall ventilation grate",
        "LabDecorBack",
        ("vent",),
    ),
    TileSpec(
        "fan_vent",
        "decor",
        "none",
        _fan_vent,
        "round fan ventilation tile",
        "LabDecorBack",
        ("vent", "fan"),
    ),
    TileSpec(
        "ceiling_light",
        "light",
        "none",
        _ceiling_light,
        "cool fluorescent ceiling light",
        "LabForeground",
        ("light", "cyan"),
    ),
    TileSpec(
        "red_warning_light",
        "light",
        "none",
        _red_warning_light,
        "red rotating warning light",
        "LabForeground",
        ("light", "warning"),
    ),
    # Interactive / prop set.
    TileSpec(
        "terminal",
        "interactive",
        "none",
        _terminal,
        "small lab control terminal",
        "LabProps",
        ("terminal", "interactive"),
    ),
    TileSpec(
        "big_monitor",
        "interactive",
        "none",
        _big_monitor,
        "large waveform monitor",
        "LabProps",
        ("monitor", "interactive"),
    ),
    TileSpec(
        "switch_box",
        "interactive",
        "none",
        _switch_box,
        "breaker switch box",
        "LabProps",
        ("switch", "interactive"),
    ),
    TileSpec(
        "gate_socket",
        "interactive",
        "none",
        _gate_socket,
        "intro gate calibration socket",
        "LabProps",
        ("gate", "socket", "interactive"),
    ),
    TileSpec(
        "lab_door_top",
        "door",
        "solid",
        _lab_door_top,
        "upper laboratory door tile",
        "LabSolids",
        ("door", "solid", "top"),
    ),
    TileSpec(
        "lab_door_bottom",
        "door",
        "solid",
        _lab_door_bottom,
        "lower laboratory door tile",
        "LabSolids",
        ("door", "solid", "bottom"),
    ),
    TileSpec(
        "blast_door_left",
        "door",
        "solid",
        _blast_door_left,
        "left half of heavy blast door",
        "LabSolids",
        ("door", "solid", "left"),
    ),
    TileSpec(
        "blast_door_right",
        "door",
        "solid",
        _blast_door_right,
        "right half of heavy blast door",
        "LabSolids",
        ("door", "solid", "right"),
    ),
    TileSpec(
        "glass_tube_top",
        "glass",
        "none",
        _glass_tube_top,
        "top of glass specimen tube",
        "LabProps",
        ("glass", "tube", "top"),
    ),
    TileSpec(
        "glass_tube_mid",
        "glass",
        "none",
        _glass_tube_mid,
        "middle of glass specimen tube",
        "LabProps",
        ("glass", "tube", "mid"),
    ),
    TileSpec(
        "glass_tube_bottom",
        "glass",
        "none",
        _glass_tube_bottom,
        "bottom of glass specimen tube",
        "LabProps",
        ("glass", "tube", "bottom"),
    ),
    TileSpec(
        "glass_tube_specimen",
        "glass",
        "none",
        _glass_tube_specimen,
        "specimen-filled glass tube middle",
        "LabProps",
        ("glass", "tube", "specimen"),
    ),
    TileSpec(
        "lab_table_left",
        "prop",
        "none",
        _lab_table_left,
        "left lab workbench segment",
        "LabProps",
        ("table", "left"),
    ),
    TileSpec(
        "lab_table_mid",
        "prop",
        "none",
        _lab_table_mid,
        "middle lab workbench segment",
        "LabProps",
        ("table", "mid"),
    ),
    TileSpec(
        "lab_table_right",
        "prop",
        "none",
        _lab_table_right,
        "right lab workbench segment",
        "LabProps",
        ("table", "right"),
    ),
    TileSpec(
        "locker",
        "prop",
        "solid",
        _locker,
        "lab locker prop",
        "LabProps",
        ("locker", "solid"),
    ),
    TileSpec(
        "small_crate",
        "prop",
        "solid",
        _small_crate,
        "small storage crate prop",
        "LabProps",
        ("crate", "solid"),
    ),
    TileSpec(
        "barrel",
        "prop",
        "solid",
        _barrel,
        "metal barrel prop",
        "LabProps",
        ("barrel", "solid"),
    ),
    TileSpec(
        "sign_arrow",
        "decor",
        "none",
        _sign_arrow,
        "exit arrow sign",
        "LabForeground",
        ("sign", "exit"),
    ),
    TileSpec(
        "warning_sign",
        "decor",
        "none",
        _warning_sign,
        "triangular warning sign",
        "LabForeground",
        ("sign", "warning"),
    ),
    # Backdrop, set dressing, and extra LDtk construction pieces.
    TileSpec(
        "bg_dark_panel",
        "background",
        "none",
        _bg_dark_panel,
        "dark recessed background panel",
        "LabBackground",
        ("background", "panel"),
    ),
    TileSpec(
        "bg_girder_cross",
        "background",
        "none",
        _bg_girder_cross,
        "cross-braced girder backdrop",
        "LabBackground",
        ("background", "girder"),
    ),
    TileSpec(
        "bg_window_left",
        "background",
        "none",
        _bg_window_left,
        "left segment of large observation window",
        "LabBackground",
        ("background", "window", "left"),
    ),
    TileSpec(
        "bg_window_mid",
        "background",
        "none",
        _bg_window_mid,
        "middle segment of large observation window",
        "LabBackground",
        ("background", "window", "mid"),
    ),
    TileSpec(
        "bg_window_right",
        "background",
        "none",
        _bg_window_right,
        "right segment of large observation window",
        "LabBackground",
        ("background", "window", "right"),
    ),
    TileSpec(
        "ceiling_rail_left",
        "decor",
        "none",
        _ceiling_rail_left,
        "left overhead utility rail",
        "LabForeground",
        ("ceiling", "rail", "left"),
    ),
    TileSpec(
        "ceiling_rail_mid",
        "decor",
        "none",
        _ceiling_rail_mid,
        "middle overhead utility rail",
        "LabForeground",
        ("ceiling", "rail", "mid"),
    ),
    TileSpec(
        "ceiling_rail_right",
        "decor",
        "none",
        _ceiling_rail_right,
        "right overhead utility rail",
        "LabForeground",
        ("ceiling", "rail", "right"),
    ),
    TileSpec(
        "hanging_cable",
        "decor",
        "none",
        _hanging_cable,
        "hanging loose cable bundle",
        "LabForeground",
        ("cable", "hanging"),
    ),
    TileSpec(
        "machine_core_top",
        "machine",
        "none",
        _machine_core_top,
        "top tile of tall glowing machine core",
        "LabProps",
        ("machine", "top"),
    ),
    TileSpec(
        "machine_core_mid",
        "machine",
        "none",
        _machine_core_mid,
        "middle tile of tall glowing machine core",
        "LabProps",
        ("machine", "mid"),
    ),
    TileSpec(
        "machine_core_bottom",
        "machine",
        "none",
        _machine_core_bottom,
        "bottom tile of tall glowing machine core",
        "LabProps",
        ("machine", "bottom"),
    ),
    TileSpec(
        "track_left",
        "floor",
        "solid",
        _track_left,
        "cart / equipment rail floor tile left",
        "LabSolids",
        ("floor", "track"),
    ),
    TileSpec(
        "track_mid",
        "floor",
        "solid",
        _track_mid,
        "cart / equipment rail floor tile middle",
        "LabSolids",
        ("floor", "track"),
    ),
    TileSpec(
        "track_right",
        "floor",
        "solid",
        _track_right,
        "cart / equipment rail floor tile right",
        "LabSolids",
        ("floor", "track"),
    ),
    TileSpec(
        "floor_arrow",
        "floor",
        "solid",
        _floor_arrow,
        "painted directional arrow on floor",
        "LabSolids",
        ("floor", "arrow"),
    ),
    TileSpec(
        "floor_bolt_plate",
        "floor",
        "solid",
        _floor_bolt_plate,
        "bolted floor access plate",
        "LabSolids",
        ("floor", "plate"),
    ),
    TileSpec(
        "tool_rack",
        "prop",
        "none",
        _tool_rack,
        "wall-mounted tool rack",
        "LabProps",
        ("tools", "rack"),
    ),
    TileSpec(
        "whiteboard",
        "decor",
        "none",
        _whiteboard,
        "small lab whiteboard with graph scribble",
        "LabForeground",
        ("whiteboard", "diagram"),
    ),
    TileSpec(
        "first_aid",
        "prop",
        "none",
        _first_aid,
        "first aid wall box",
        "LabProps",
        ("medical", "wallbox"),
    ),
    TileSpec(
        "spilled_beaker",
        "prop",
        "none",
        _spilled_beaker,
        "spilled glowing beaker prop",
        "LabProps",
        ("beaker", "spill"),
    ),
    TileSpec(
        "blank",
        "utility",
        "none",
        _blank,
        "empty transparent tile",
        "Utility",
        ("blank",),
    ),
]


def _rows() -> int:
    return math.ceil(len(TILES) / COLS)


def render_tile(tile: TileSpec) -> Image.Image:
    img = Image.new("RGBA", (CANVAS_TILE, CANVAS_TILE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    tile.draw(draw)
    return _downsample(img)


def _write_preview(tile_images: Dict[str, Image.Image], out_path: Path) -> None:
    rows = _rows()
    preview_w = COLS * (OUTPUT_TILE + PREVIEW_PAD) + PREVIEW_PAD
    preview_h = rows * (OUTPUT_TILE + 18 + PREVIEW_PAD) + PREVIEW_PAD
    preview = Image.new("RGBA", (preview_w, preview_h), _rgba("#1f2028"))
    d = ImageDraw.Draw(preview, "RGBA")
    font = ImageFont.load_default()
    for idx, tile in enumerate(TILES):
        col = idx % COLS
        row = idx // COLS
        x = PREVIEW_PAD + col * (OUTPUT_TILE + PREVIEW_PAD)
        y = PREVIEW_PAD + row * (OUTPUT_TILE + 18 + PREVIEW_PAD)
        d.rectangle(
            (x - 1, y - 1, x + OUTPUT_TILE, y + OUTPUT_TILE),
            fill=_rgba("#11151c"),
            outline=_rgba("#545a66"),
        )
        preview.alpha_composite(tile_images[tile.key], (x, y))
        d.text((x, y + OUTPUT_TILE + 2), str(idx), fill=_rgba("#d7dbe2"), font=font)
        d.text(
            (x + 14, y + OUTPUT_TILE + 2),
            tile.key[:5],
            fill=_rgba("#8e98a8"),
            font=font,
        )
    out_path.parent.mkdir(parents=True, exist_ok=True)
    preview.save(out_path)


def _ldtk_suggested_layers() -> List[Dict[str, object]]:
    return [
        {
            "name": "LabBackground",
            "type": "Tiles",
            "description": "flat wall/backdrop panels behind collision",
        },
        {
            "name": "LabSolids",
            "type": "Tiles",
            "intgrid_value": 1,
            "description": "solid collision walls, floors, and doors",
        },
        {
            "name": "LabPlatforms",
            "type": "Tiles",
            "intgrid_value": 2,
            "description": "one-way platform collision",
        },
        {
            "name": "LabDecorBack",
            "type": "Tiles",
            "description": "pipes, cables, vents, and behind-player dressing",
        },
        {
            "name": "LabProps",
            "type": "Tiles",
            "description": "interactive props and chunky foreground objects",
        },
        {
            "name": "LabForeground",
            "type": "Tiles",
            "description": "lights, signs, and foreground overlays",
        },
        {
            "name": "LabCollision",
            "type": "IntGrid",
            "values": {"0": "empty", "1": "solid", "2": "one_way"},
        },
    ]


def render(out_dir: str | Path, **opts) -> List[Path]:
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    sheet_path = out_dir / f"{TARGET_NAME}.png"
    yaml_path = out_dir / f"{TARGET_NAME}.yaml"
    preview_path = out_dir / f"{TARGET_NAME}_preview_labeled.png"

    tile_images = {tile.key: render_tile(tile) for tile in TILES}
    rows = _rows()
    sheet = Image.new("RGBA", (COLS * OUTPUT_TILE, rows * OUTPUT_TILE), (0, 0, 0, 0))
    tiles_meta = []
    by_category: Dict[str, List[str]] = {}
    by_layer: Dict[str, List[str]] = {}
    for idx, tile in enumerate(TILES):
        col = idx % COLS
        row = idx // COLS
        x, y = col * OUTPUT_TILE, row * OUTPUT_TILE
        sheet.alpha_composite(tile_images[tile.key], (x, y))
        by_category.setdefault(tile.category, []).append(tile.key)
        by_layer.setdefault(tile.layer, []).append(tile.key)
        tiles_meta.append(
            {
                "index": idx,
                "key": tile.key,
                "category": tile.category,
                "collision": tile.collision,
                "ldtk_layer": tile.layer,
                "tags": list(tile.tags),
                "rect": {"x": x, "y": y, "w": OUTPUT_TILE, "h": OUTPUT_TILE},
                "description": tile.description,
            }
        )

    sheet.save(sheet_path)
    _write_preview(tile_images, preview_path)
    manifest = {
        "target": TARGET_NAME,
        "image": sheet_path.name,
        "tile_width": OUTPUT_TILE,
        "tile_height": OUTPUT_TILE,
        "columns": COLS,
        "rows": rows,
        "tile_count": len(TILES),
        "tiles": tiles_meta,
        "groups": {
            "by_category": by_category,
            "by_ldtk_layer": by_layer,
        },
        "ldtk": {
            "suggested_tile_size": OUTPUT_TILE,
            "suggested_layers": _ldtk_suggested_layers(),
            "collision_values": {"none": 0, "solid": 1, "one_way": 2},
        },
        "notes": "Expanded procedural intro laboratory tileset for LDtk: solid walls/floors, one-way platforms, backdrop panels, pipes, cables, vents, doors, signs, lights, consoles, glass tubes, crates, and props.",
    }
    yaml_path.write_text(yaml.safe_dump(manifest, sort_keys=False, width=120))
    return [sheet_path, yaml_path, preview_path]
