"""Code-generated, pre-rendered parallax background assets.

These PNGs are generated offline by ``ambition_parallax_renderer`` and then
loaded by the game. This file therefore leans into heavier rendering passes and
more art-directed composition instead of runtime-cheap tile motifs.
"""

from __future__ import annotations

import hashlib
import json
import math
import random
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Tuple

from PIL import Image, ImageDraw, ImageFilter

RGB = Tuple[int, int, int]
RGBA = Tuple[int, int, int, int]
SIZE = 768


@dataclass(frozen=True)
class Theme:
    key: str
    sky_top: RGB
    sky_mid: RGB
    sky_bottom: RGB
    silhouette: RGB
    accent: RGB
    glow: RGB
    celestial: str
    mood: str
    star_density: int = 0


@dataclass(frozen=True)
class Layer:
    key: str
    factor: float
    z_hint: float
    opaque: bool = False


THEMES: tuple[Theme, ...] = (
    Theme(
        "hub",
        (16, 21, 54),
        (28, 36, 84),
        (7, 11, 24),
        (22, 28, 68),
        (110, 98, 188),
        (196, 218, 255),
        "moon",
        "station_night",
        72,
    ),
    Theme(
        "lab",
        (8, 30, 42),
        (12, 56, 68),
        (5, 18, 24),
        (14, 45, 56),
        (56, 168, 166),
        (148, 244, 228),
        "stars",
        "industrial_reactor",
        34,
    ),
    Theme(
        "basement",
        (34, 18, 18),
        (56, 32, 26),
        (14, 10, 10),
        (54, 30, 20),
        (168, 98, 62),
        (255, 176, 110),
        "embers",
        "ruined_furnace",
        0,
    ),
    Theme(
        "cove",
        (8, 34, 56),
        (18, 86, 94),
        (5, 26, 30),
        (18, 52, 58),
        (72, 150, 124),
        (194, 248, 228),
        "moon",
        "lagoon",
        40,
    ),
    Theme(
        "skybridge",
        (86, 124, 176),
        (132, 172, 208),
        (68, 92, 136),
        (94, 120, 160),
        (206, 216, 226),
        (255, 244, 218),
        "sun",
        "floating_bridge",
        0,
    ),
    Theme(
        "boss",
        (36, 8, 24),
        (76, 18, 34),
        (14, 5, 11),
        (64, 18, 30),
        (204, 56, 80),
        (255, 124, 78),
        "eclipse",
        "arena_spires",
        18,
    ),
    Theme(
        "water",
        (2, 24, 44),
        (7, 62, 80),
        (2, 14, 28),
        (8, 48, 66),
        (40, 132, 132),
        (128, 232, 236),
        "caustics",
        "underwater_ruins",
        0,
    ),
    Theme(
        "forest",
        (16, 24, 28),
        (42, 58, 46),
        (16, 24, 18),
        (30, 48, 32),
        (90, 126, 78),
        (222, 236, 196),
        "moon",
        "ninja_dojo_forest",
        18,
    ),
    Theme(
        "cave",
        (12, 14, 26),
        (22, 26, 40),
        (5, 7, 16),
        (22, 24, 36),
        (92, 86, 116),
        (186, 166, 224),
        "none",
        "crystal_cave",
        0,
    ),
)

LAYERS: tuple[Layer, ...] = (
    Layer("sky", 0.10, -18.0, opaque=True),
    Layer("far_backplate", 0.20, -17.0),
    Layer("near_background", 0.42, -16.0),
    Layer("foreground_atmosphere", 0.60, -15.0),
)


def _seed(*parts: str) -> int:
    digest = hashlib.blake2b(":".join(parts).encode(), digest_size=8).digest()
    return int.from_bytes(digest, "little")


def _clamp(v: float, lo: int = 0, hi: int = 255) -> int:
    return max(lo, min(hi, int(round(v))))


def _mix(a: RGB, b: RGB, t: float) -> RGB:
    return tuple(_clamp(x + (y - x) * t) for x, y in zip(a, b))  # type: ignore[return-value]


def _rgba(color: RGB, a: int) -> RGBA:
    return (color[0], color[1], color[2], a)


def _scale_alpha(color: RGBA, scale: float) -> RGBA:
    return (color[0], color[1], color[2], _clamp(color[3] * scale))


def _poly(
    draw: ImageDraw.ImageDraw, pts: Iterable[tuple[float, float]], fill: RGBA
) -> None:
    draw.polygon([(int(x), int(y)) for x, y in pts], fill=fill)


def _line(
    draw: ImageDraw.ImageDraw,
    pts: Iterable[tuple[float, float]],
    fill: RGBA,
    width: int,
) -> None:
    draw.line([(int(x), int(y)) for x, y in pts], fill=fill, width=width, joint="curve")


def _periodic_band(
    draw: ImageDraw.ImageDraw,
    fill: RGBA,
    base_y: float,
    thickness: float,
    amp1: float,
    amp2: float,
    phase1: float,
    phase2: float,
    steps: int = 32,
) -> None:
    top = []
    bot = []
    for i in range(steps + 1):
        x = SIZE * (i / steps)
        t = x / SIZE
        y = (
            base_y
            + math.sin(t * math.tau + phase1) * amp1
            + math.sin(t * math.tau * 2.0 + phase2) * amp2
        )
        top.append((x, y))
        bot.append((x, y + thickness))
    _poly(draw, top + list(reversed(bot)), fill)


def _new_overlay() -> tuple[Image.Image, ImageDraw.ImageDraw]:
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img, "RGBA")


def _blurred_alpha_composite(
    base: Image.Image, overlay: Image.Image, blur: float
) -> None:
    if blur > 0:
        overlay = overlay.filter(ImageFilter.GaussianBlur(blur))
    base.alpha_composite(overlay)


def _draw_stars(
    draw: ImageDraw.ImageDraw, rng: random.Random, count: int, color: RGBA
) -> None:
    for _ in range(count):
        x = rng.randint(12, SIZE - 12)
        y = rng.randint(12, int(SIZE * 0.55))
        r = rng.choice([1, 1, 1, 2])
        a = _clamp(color[3] + rng.randint(-20, 20))
        c = (color[0], color[1], color[2], a)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=c)
        if r == 2 and rng.random() < 0.55:
            draw.line(
                [(x - 4, y), (x + 4, y)], fill=(c[0], c[1], c[2], a // 2), width=1
            )
            draw.line(
                [(x, y - 4), (x, y + 4)], fill=(c[0], c[1], c[2], a // 2), width=1
            )


def _draw_moon(
    base: Image.Image, center: tuple[int, int], radius: int, tint: RGB
) -> None:
    glow, gdraw = _new_overlay()
    x, y = center
    gdraw.ellipse(
        [x - radius - 26, y - radius - 26, x + radius + 26, y + radius + 26],
        fill=_rgba(tint, 34),
    )
    _blurred_alpha_composite(base, glow, 16)
    moon, mdraw = _new_overlay()
    mdraw.ellipse(
        [x - radius, y - radius, x + radius, y + radius],
        fill=_rgba((240, 242, 248), 238),
    )
    mdraw.ellipse(
        [x - radius + 12, y - radius - 2, x + radius + 10, y + radius + 2],
        fill=(0, 0, 0, 128),
    )
    base.alpha_composite(moon)


def _draw_sun(
    base: Image.Image, center: tuple[int, int], radius: int, tint: RGB
) -> None:
    overlay, draw = _new_overlay()
    x, y = center
    for grow, alpha in ((54, 18), (30, 34), (12, 60)):
        draw.ellipse(
            [
                x - radius - grow,
                y - radius - grow,
                x + radius + grow,
                y + radius + grow,
            ],
            fill=_rgba(tint, alpha),
        )
    draw.ellipse(
        [x - radius, y - radius, x + radius, y + radius],
        fill=_rgba((255, 242, 202), 230),
    )
    _blurred_alpha_composite(base, overlay, 12)


def _draw_eclipse(base: Image.Image, center: tuple[int, int], radius: int) -> None:
    overlay, draw = _new_overlay()
    x, y = center
    for grow, alpha in ((22, 26), (10, 56), (4, 96)):
        draw.ellipse(
            [
                x - radius - grow,
                y - radius - grow,
                x + radius + grow,
                y + radius + grow,
            ],
            outline=(255, 108, 72, alpha),
            width=4,
        )
    draw.ellipse(
        [x - radius + 5, y - radius + 4, x + radius + 5, y + radius + 4],
        fill=(10, 7, 12, 232),
    )
    _blurred_alpha_composite(base, overlay, 3)


def _draw_caustics(draw: ImageDraw.ImageDraw, rng: random.Random, color: RGBA) -> None:
    for y in (90, 132, 190, 250, 322):
        for _ in range(3):
            x0 = rng.randint(-80, SIZE - 160)
            _line(
                draw,
                [
                    (x0, y),
                    (x0 + 120, y + rng.randint(-12, 12)),
                    (x0 + 220, y + rng.randint(-8, 8)),
                ],
                color,
                rng.randint(2, 4),
            )


def _draw_embers(
    draw: ImageDraw.ImageDraw, rng: random.Random, color: RGBA, count: int = 28
) -> None:
    for _ in range(count):
        x = rng.randint(16, SIZE - 16)
        y = rng.randint(40, int(SIZE * 0.78))
        r = rng.randint(1, 3)
        draw.ellipse(
            [x - r, y - r, x + r, y + r],
            fill=(
                color[0],
                color[1],
                color[2],
                _clamp(color[3] + rng.randint(-18, 18)),
            ),
        )


def _draw_sky(theme: Theme) -> Image.Image:
    rng = random.Random(_seed(theme.key, "sky"))
    image = Image.new("RGBA", (SIZE, SIZE), (*theme.sky_mid, 255))
    px = image.load()
    for y in range(SIZE):
        t = y / (SIZE - 1)
        c = (
            _mix(theme.sky_top, theme.sky_mid, min(1.0, t / 0.42))
            if t < 0.42
            else _mix(theme.sky_mid, theme.sky_bottom, (t - 0.42) / 0.58)
        )
        horizon_strength = max(0.0, 1.0 - abs(t - 0.58) / 0.18)
        c = _mix(c, theme.glow, 0.09 * horizon_strength)
        for x in range(SIZE):
            nx = math.sin(x / SIZE * math.tau * 1.4 + 0.4) * 2.8
            ny = math.sin(y / SIZE * math.tau * 0.9 + 1.1) * 2.0
            vignette = -14.0 * max(0.0, abs(x - SIZE / 2) / (SIZE / 2) - 0.35)
            shade = nx + ny + vignette
            px[x, y] = (
                _clamp(c[0] + shade),
                _clamp(c[1] + shade),
                _clamp(c[2] + shade),
                255,
            )

    haze, hdraw = _new_overlay()
    _periodic_band(
        hdraw,
        _rgba(_mix(theme.sky_mid, theme.glow, 0.4), 30),
        420,
        44,
        12,
        18,
        0.4,
        1.7,
    )
    _periodic_band(
        hdraw,
        _rgba(_mix(theme.sky_mid, theme.glow, 0.18), 20),
        510,
        32,
        14,
        10,
        2.4,
        0.8,
    )
    _blurred_alpha_composite(image, haze, 5)

    if theme.star_density:
        stars, sdraw = _new_overlay()
        _draw_stars(sdraw, rng, theme.star_density, _rgba((248, 248, 255), 110))
        image.alpha_composite(stars)

    if theme.celestial == "moon":
        _draw_moon(
            image,
            (136 if theme.key == "hub" else 574, 114 if theme.key == "hub" else 136),
            36 if theme.key == "hub" else 28,
            theme.glow,
        )
    elif theme.celestial == "sun":
        _draw_sun(image, (598, 106), 38, theme.glow)
    elif theme.celestial == "eclipse":
        _draw_eclipse(image, (602, 110), 34)
    elif theme.celestial == "caustics":
        over, odraw = _new_overlay()
        _draw_caustics(odraw, rng, _rgba(theme.glow, 42))
        _blurred_alpha_composite(image, over, 1.5)
    elif theme.celestial == "embers":
        over, odraw = _new_overlay()
        _draw_embers(odraw, rng, _rgba(theme.glow, 48), 26)
        image.alpha_composite(over)
    elif theme.celestial == "fireflies":
        over, odraw = _new_overlay()
        # Forest fireflies / pollen motes: small enough not to read as gameplay.
        for _ in range(36):
            x = rng.randint(34, SIZE - 34)
            y = rng.randint(110, int(SIZE * 0.72))
            r = rng.choice([1, 1, 2, 2, 3])
            odraw.ellipse(
                [x - r, y - r, x + r, y + r],
                fill=_rgba(theme.glow, rng.randint(20, 46)),
            )
        _blurred_alpha_composite(image, over, 1.4)

    # Distant horizon line / mood anchor.
    horizon, hdraw = _new_overlay()
    deep = _rgba(_mix(theme.sky_bottom, (0, 0, 0), 0.20), 220)
    mid = _rgba(_mix(theme.silhouette, theme.sky_mid, 0.25), 158)
    _periodic_band(hdraw, deep, 468, 80, 12, 18, 0.2, 1.4)
    _periodic_band(hdraw, mid, 514, 98, 16, 12, 1.6, 0.4)
    if theme.key in {"hub", "lab", "skybridge"}:
        _periodic_band(
            hdraw,
            _rgba(_mix(theme.glow, theme.sky_mid, 0.65), 28),
            446,
            22,
            8,
            6,
            1.4,
            2.2,
        )
    if theme.key == "water":
        _periodic_band(
            hdraw,
            _rgba(_mix(theme.glow, theme.sky_mid, 0.55), 36),
            438,
            30,
            10,
            14,
            0.8,
            1.8,
        )
    _blurred_alpha_composite(image, horizon, 1.0)
    if theme.key == "forest":
        canopy, cdraw = _new_overlay()
        _periodic_band(cdraw, _rgba((6, 24, 14), 112), 0, 86, 24, 18, 0.6, 1.8)
        _periodic_band(cdraw, _rgba((10, 36, 20), 78), 66, 44, 18, 12, 2.4, 0.7)
        for x in (92, 188, 304, 470, 610, 704):
            # soft shafts of light through the canopy
            _poly(
                cdraw,
                [(x - 18, 0), (x + 22, 0), (x + 92, SIZE), (x + 36, SIZE)],
                _rgba(theme.glow, 18),
            )
        _blurred_alpha_composite(image, canopy, 8.0)
    return image


def _add_scaffold(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    for side in (0, 1):
        x = 86 if side == 0 else SIZE - 86
        s = 1 if side == 0 else -1
        draw.arc(
            [x - 84, 50, x + 84, 236],
            96 if side == 0 else 264,
            264 if side == 0 else 96,
            fill=color,
            width=8,
        )
        for y in (106, 198, 306, 424):
            _line(draw, [(x + s * 4, y - 60), (x + s * 58, y + 24)], accent, 5)
            _line(draw, [(x - s * 10, y - 56), (x + s * 40, y + 30)], color, 3)


def _add_ring_station(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    cx, cy = 536, 350
    draw.arc([cx - 92, cy - 92, cx + 92, cy + 92], 20, 340, fill=color, width=18)
    draw.arc([cx - 72, cy - 72, cx + 72, cy + 72], 24, 336, fill=accent, width=8)
    draw.rounded_rectangle(
        [cx - 18, cy - 84, cx + 18, cy + 84], radius=6, fill=_scale_alpha(accent, 0.7)
    )
    draw.rounded_rectangle(
        [cx - 84, cy - 18, cx + 84, cy + 18], radius=6, fill=_scale_alpha(color, 0.8)
    )


def _add_city_horizon(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    rng = random.Random(1337)
    x = 210
    while x < SIZE - 70:
        w = rng.randint(18, 42)
        h = rng.randint(24, 92)
        draw.rectangle([x, 468 - h, x + w, 468], fill=color)
        if rng.random() < 0.7:
            for yy in range(468 - h + 8, 466, 12):
                draw.rectangle(
                    [x + 4, yy, x + w - 4, yy + 3], fill=_scale_alpha(accent, 0.45)
                )
        x += w + rng.randint(6, 12)


def _add_reactor_towers(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    towers = [(214, 490, 66, 246), (346, 480, 90, 284), (520, 498, 70, 216)]
    for cx, base_y, w, h in towers:
        draw.rounded_rectangle(
            [cx - w // 2, base_y - h, cx + w // 2, base_y], radius=10, fill=color
        )
        draw.rounded_rectangle(
            [cx - w // 2 + 12, base_y - h + 26, cx + w // 2 - 12, base_y - h + 44],
            radius=4,
            fill=_scale_alpha(accent, 0.7),
        )
        draw.rectangle(
            [cx - 6, base_y - h - 30, cx + 6, base_y - h + 4],
            fill=_scale_alpha(accent, 0.8),
        )
    for y in (142, 210, 296):
        _line(draw, [(0, y), (178, y + 22), (328, y + 8)], _scale_alpha(color, 0.8), 7)
        _line(
            draw, [(430, y + 18), (600, y), (768, y + 12)], _scale_alpha(color, 0.8), 7
        )


def _add_hanging_cables(
    draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA, alpha_scale: float = 1.0
) -> None:
    rng = random.Random(4242)
    for x in (58, 112, 656, 716):
        depth = rng.randint(180, 360)
        _line(
            draw,
            [
                (x, 0),
                (x + rng.randint(-18, 18), depth * 0.55),
                (x + rng.randint(-28, 28), depth),
            ],
            _scale_alpha(color, alpha_scale),
            6,
        )
        draw.rounded_rectangle(
            [x - 14, depth - 10, x + 14, depth + 10],
            radius=4,
            fill=_scale_alpha(accent, 0.8 * alpha_scale),
        )


def _add_furnace_arches(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    for cx, w, h in ((168, 156, 220), (390, 212, 264), (630, 128, 188)):
        draw.arc(
            [cx - w // 2, 246, cx + w // 2, 246 + h], 180, 360, fill=color, width=10
        )
        draw.arc(
            [cx - w // 2 + 18, 264, cx + w // 2 - 18, 246 + h - 18],
            180,
            360,
            fill=_scale_alpha(accent, 0.75),
            width=5,
        )
    for x in (120, 288, 458, 616):
        draw.rectangle([x - 12, 196, x + 12, 560], fill=_scale_alpha(color, 0.9))


def _add_pipes(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    for y in (86, 148, 622):
        _line(draw, [(0, y), (212, y + 6), (428, y - 8), (768, y + 10)], color, 8)
    for x in (84, 690):
        _line(
            draw,
            [(x, 0), (x, 216), (x + (12 if x < 200 else -12), 404), (x, 650)],
            accent,
            8,
        )


def _add_sea_cliffs(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    _poly(
        draw,
        [(0, 432), (88, 382), (152, 398), (206, 350), (240, 358), (282, 454), (0, 544)],
        color,
    )
    _poly(
        draw,
        [
            (768, 418),
            (654, 370),
            (588, 392),
            (520, 356),
            (468, 366),
            (432, 456),
            (768, 560),
        ],
        color,
    )
    draw.rectangle([324, 388, 348, 510], fill=_scale_alpha(accent, 0.8))
    draw.polygon([(336, 346), (316, 390), (356, 390)], fill=_scale_alpha(accent, 0.8))


def _add_reeds(
    draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA, alpha_scale: float = 1.0
) -> None:
    rng = random.Random(7373)
    for x_region in ((18, 148), (618, 748)):
        for _ in range(18):
            x = rng.randint(*x_region)
            h = rng.randint(88, 210)
            bend = rng.randint(-28, 28)
            _line(
                draw,
                [(x, SIZE), (x + bend * 0.35, SIZE - h * 0.6), (x + bend, SIZE - h)],
                _scale_alpha(color, alpha_scale),
                rng.randint(2, 4),
            )
            if rng.random() < 0.48:
                draw.ellipse(
                    [x + bend - 4, SIZE - h - 10, x + bend + 5, SIZE - h + 8],
                    fill=_scale_alpha(accent, 0.7 * alpha_scale),
                )


def _add_bridge_pylons(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    pylons = [(168, 496, 42, 236), (384, 470, 58, 282), (610, 488, 48, 220)]
    for cx, base_y, w, h in pylons:
        draw.polygon(
            [
                (cx - w, base_y),
                (cx + w, base_y),
                (cx + w // 3, base_y - h),
                (cx - w // 3, base_y - h),
            ],
            fill=color,
        )
        draw.rectangle(
            [cx - 6, base_y - h - 26, cx + 6, base_y - h + 10],
            fill=_scale_alpha(accent, 0.85),
        )
    _line(
        draw,
        [(90, 328), (220, 352), (384, 320), (542, 350), (676, 334)],
        _scale_alpha(accent, 0.85),
        8,
    )


def _add_cloud_decks(draw: ImageDraw.ImageDraw, color: RGBA, glow: RGBA) -> None:
    _periodic_band(draw, _scale_alpha(color, 0.72), 156, 44, 14, 12, 0.3, 1.6)
    _periodic_band(draw, _scale_alpha(color, 0.84), 516, 68, 18, 14, 1.2, 0.6)
    _periodic_band(draw, _scale_alpha(glow, 0.36), 498, 26, 10, 10, 2.4, 1.0)


def _add_monoliths(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    for pts in (
        [(126, 560), (162, 338), (210, 322), (248, 560)],
        [(332, 560), (364, 286), (414, 270), (452, 560)],
        [(560, 560), (596, 356), (642, 340), (680, 560)],
    ):
        _poly(draw, pts, color)
    for x1, y1, x2, y2 in (
        (164, 338, 366, 286),
        (414, 270, 604, 356),
        (366, 286, 610, 216),
    ):
        _line(
            draw,
            [(x1, y1), ((x1 + x2) // 2, (y1 + y2) // 2 - 18), (x2, y2)],
            _scale_alpha(accent, 0.65),
            4,
        )


def _add_shards(draw: ImageDraw.ImageDraw, color: RGBA) -> None:
    for pts in (
        [(84, 168), (110, 124), (128, 182)],
        [(620, 210), (654, 168), (666, 236)],
        [(534, 94), (560, 56), (580, 110)],
    ):
        _poly(draw, pts, color)


def _add_ruins_and_kelp(
    draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA, glow: RGBA
) -> None:
    for x, w, h in ((248, 62, 168), (398, 54, 204), (548, 44, 150)):
        draw.rectangle([x - w // 2, 472 - h, x + w // 2, 472], fill=color)
        draw.rectangle(
            [x - w // 2 + 10, 472 - h + 24, x + w // 2 - 10, 472 - h + 44],
            fill=_scale_alpha(accent, 0.6),
        )
    rng = random.Random(9090)
    for x_region in ((28, 174), (586, 736)):
        for _ in range(10):
            x = rng.randint(*x_region)
            pts = []
            step = rng.randint(42, 58)
            phase = rng.random() * math.tau
            for j in range(6):
                y = SIZE - j * step
                pts.append((x + math.sin(j * 1.6 + phase) * rng.randint(12, 24), y))
            _line(draw, pts, color, rng.randint(3, 6))
            for px, py in pts[1:-1:2]:
                draw.ellipse(
                    [px - 9, py - 4, px + 9, py + 4], fill=_scale_alpha(accent, 0.55)
                )
    for x, y, r in ((150, 212, 8), (182, 246, 5), (622, 182, 6), (652, 222, 9)):
        draw.ellipse(
            [x - r, y - r, x + r, y + r], outline=_scale_alpha(glow, 0.65), width=2
        )


def _add_crystals(draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA) -> None:
    for pts in (
        [(142, 560), (168, 418), (198, 404), (222, 560)],
        [(324, 560), (364, 334), (402, 320), (438, 560)],
        [(548, 560), (586, 378), (614, 366), (646, 560)],
    ):
        _poly(draw, pts, color)
    for pts in (
        [(182, 418), (210, 392), (232, 442)],
        [(384, 334), (420, 300), (438, 356)],
        [(596, 378), (622, 346), (644, 394)],
    ):
        _poly(draw, pts, accent)


def _add_stalactites(draw: ImageDraw.ImageDraw, color: RGBA) -> None:
    for x, w, h in (
        (66, 26, 120),
        (142, 18, 84),
        (604, 24, 118),
        (690, 18, 74),
        (386, 36, 142),
    ):
        _poly(draw, [(x - w, 0), (x + w, 0), (x, h)], color)


def _add_forest_canopy(
    draw: ImageDraw.ImageDraw,
    color: RGBA,
    accent: RGBA,
    glow: RGBA,
    alpha_scale: float = 1.0,
) -> None:
    # Dense overhead foliage with readable negative space in the middle.
    _periodic_band(
        draw, _scale_alpha(color, 1.05 * alpha_scale), -8, 124, 26, 18, 0.4, 1.8
    )
    _periodic_band(
        draw, _scale_alpha(accent, 0.58 * alpha_scale), 68, 54, 18, 12, 2.2, 0.7
    )
    for cx, cy, rx, ry in (
        (80, 72, 96, 52),
        (196, 48, 88, 44),
        (566, 54, 92, 46),
        (690, 84, 102, 58),
        (122, 146, 68, 28),
        (636, 138, 74, 26),
    ):
        draw.ellipse(
            [cx - rx, cy - ry, cx + rx, cy + ry],
            fill=_scale_alpha(color, 0.74 * alpha_scale),
        )
    for x in (124, 226, 550, 662):
        _poly(
            draw,
            [(x - 12, 0), (x + 18, 0), (x + 74, 260), (x + 22, 260)],
            _scale_alpha(glow, 0.10 * alpha_scale),
        )


def _add_tree_trunk(
    draw: ImageDraw.ImageDraw,
    x: int,
    top: int,
    bottom: int,
    width: int,
    trunk: RGBA,
    shadow: RGBA,
    highlight: RGBA,
    *,
    lean: int = 0,
    alpha_scale: float = 1.0,
    seed_offset: int = 0,
) -> None:
    rng = random.Random(9000 + x * 17 + width * 5 + seed_offset)
    left_top = x - width * 0.34
    right_top = x + width * 0.34
    left_bottom = x + lean - width * 0.5
    right_bottom = x + lean + width * 0.5
    pts = [
        (left_top, top),
        (right_top, top),
        (right_bottom, bottom),
        (left_bottom, bottom),
    ]
    _poly(draw, pts, _scale_alpha(trunk, alpha_scale))
    shadow_pts = [
        (x - width * 0.04, top),
        (right_top, top),
        (right_bottom, bottom),
        (x + lean + width * 0.02, bottom),
    ]
    _poly(draw, shadow_pts, _scale_alpha(shadow, 0.78 * alpha_scale))
    for _ in range(max(4, width // 8)):
        y0 = rng.randint(top + 20, max(top + 24, bottom - 48))
        y1 = min(bottom - 6, y0 + rng.randint(26, 84))
        dx = rng.randint(-width // 8, width // 8)
        xx = x + lean * ((y0 - top) / max(1, bottom - top)) + dx
        _line(
            draw,
            [(xx, y0), (xx + rng.randint(-4, 5), y1)],
            _scale_alpha(highlight, 0.45 * alpha_scale),
            max(1, width // 12),
        )
    if bottom - top > 140:
        knot_x = x + rng.randint(-width // 5, width // 5)
        knot_y = rng.randint(top + 70, bottom - 70)
        draw.ellipse(
            [knot_x - 6, knot_y - 10, knot_x + 6, knot_y + 10],
            fill=_scale_alpha(shadow, 0.9 * alpha_scale),
        )
    # Root flare.
    for s in (-1, 1):
        _line(
            draw,
            [
                (x + lean * 0.8, bottom - 10),
                (x + lean * 0.8 + s * (width // 2 + 16), bottom + 8),
            ],
            _scale_alpha(shadow, 0.72 * alpha_scale),
            max(2, width // 7),
        )


def _add_leaf_cluster(
    draw: ImageDraw.ImageDraw,
    cx: int,
    cy: int,
    leaf_dark: RGBA,
    leaf_mid: RGBA,
    scale: float = 1.0,
) -> None:
    for dx, dy, rx, ry, fill in (
        (-28, 4, 34, 22, leaf_dark),
        (0, -12, 42, 24, leaf_mid),
        (26, 6, 32, 20, leaf_dark),
        (-4, 20, 30, 18, leaf_mid),
    ):
        draw.ellipse(
            [
                cx + dx - rx * scale,
                cy + dy - ry * scale,
                cx + dx + rx * scale,
                cy + dy + ry * scale,
            ],
            fill=fill,
        )


def _add_bamboo_grove(
    draw: ImageDraw.ImageDraw,
    color: RGBA,
    accent: RGBA,
    alpha_scale: float = 1.0,
    foreground: bool = False,
) -> None:
    rng = random.Random(20260512 + (1 if foreground else 0))
    regions = ((0, 190), (588, SIZE)) if foreground else ((32, 182), (586, 736))
    count = 14 if foreground else 10
    for lo, hi in regions:
        for _ in range(count):
            x = rng.randint(lo, hi)
            w = rng.randint(6, 12) if foreground else rng.randint(4, 8)
            lean = rng.randint(-18, 18) if foreground else rng.randint(-12, 12)
            y0 = SIZE + rng.randint(8, 70)
            y1 = rng.randint(120, 260) if foreground else rng.randint(160, 320)
            stalk = _scale_alpha(color, (0.96 if foreground else 0.78) * alpha_scale)
            _line(
                draw,
                [(x, y0), (x + lean * 0.35, (y0 + y1) * 0.5), (x + lean, y1)],
                stalk,
                w,
            )
            for y in range(int(y1) + 42, int(y0), rng.randint(46, 66)):
                _line(
                    draw,
                    [(x + lean * 0.18 - w, y), (x + lean * 0.18 + w, y)],
                    _scale_alpha(accent, 0.5 * alpha_scale),
                    max(1, w // 3),
                )
            if rng.random() < 0.45:
                leaf_y = rng.randint(y1 + 40, min(460, int(y0) - 80))
                leaf_x = x + int(lean * 0.28)
                for side in (-1, 1):
                    _line(
                        draw,
                        [
                            (leaf_x, leaf_y),
                            (
                                leaf_x + side * rng.randint(22, 52),
                                leaf_y + rng.randint(-18, 18),
                            ),
                        ],
                        _scale_alpha(accent, 0.42 * alpha_scale),
                        max(1, w // 3),
                    )


def _add_forest_midstory(draw: ImageDraw.ImageDraw, alpha_scale: float = 1.0) -> None:
    trunk = (96, 66, 42, _clamp(170 * alpha_scale))
    shadow = (58, 38, 24, _clamp(156 * alpha_scale))
    highlight = (132, 94, 58, _clamp(144 * alpha_scale))
    leaf_dark = (22, 54, 30, _clamp(128 * alpha_scale))
    leaf_mid = (46, 84, 44, _clamp(116 * alpha_scale))
    for args in (
        (126, 118, 570, 44, -12),
        (252, 136, 566, 58, 18),
        (558, 130, 570, 54, -18),
        (672, 108, 572, 42, 8),
    ):
        _add_tree_trunk(
            draw,
            args[0],
            args[1],
            args[2],
            args[3],
            trunk,
            shadow,
            highlight,
            lean=args[4],
            alpha_scale=1.0,
        )
    for cx, cy in (
        (116, 162),
        (224, 136),
        (260, 188),
        (554, 150),
        (650, 170),
        (688, 126),
    ):
        _add_leaf_cluster(draw, cx, cy, leaf_dark, leaf_mid, 1.0)
    # Sparse bamboo only on the edges so the scene still reads as a forest.
    _add_bamboo_grove(
        draw,
        (26, 72, 40, _clamp(128 * alpha_scale)),
        (72, 126, 72, _clamp(112 * alpha_scale)),
        alpha_scale,
        foreground=False,
    )


def _add_torii_and_dojo(
    draw: ImageDraw.ImageDraw,
    color: RGBA,
    accent: RGBA,
    glow: RGBA,
    alpha_scale: float = 1.0,
) -> None:
    wood = (118, 62, 34, _clamp(182 * alpha_scale))
    wood_shadow = (70, 38, 22, _clamp(164 * alpha_scale))
    roof = (42, 52, 34, _clamp(188 * alpha_scale))
    paper = (212, 226, 188, _clamp(148 * alpha_scale))
    # Clearing / path leading toward the dojo.
    draw.polygon(
        [(306, 768), (462, 768), (430, 618), (392, 524), (360, 618)],
        fill=(86, 74, 52, _clamp(94 * alpha_scale)),
    )
    draw.line(
        [(306, 768), (392, 524), (462, 768)],
        fill=(124, 112, 86, _clamp(70 * alpha_scale)),
        width=3,
    )
    # Torii gate.
    draw.rounded_rectangle([222, 332, 250, 560], radius=6, fill=wood)
    draw.rounded_rectangle([530, 332, 558, 560], radius=6, fill=wood)
    draw.rounded_rectangle([190, 304, 590, 334], radius=8, fill=wood)
    draw.rounded_rectangle(
        [214, 280, 566, 304], radius=8, fill=(142, 78, 44, _clamp(196 * alpha_scale))
    )
    draw.rectangle([246, 334, 534, 352], fill=wood_shadow)
    # Dojo building beyond the gate.
    draw.rectangle([314, 452, 470, 548], fill=(42, 58, 34, _clamp(174 * alpha_scale)))
    draw.polygon(
        [(286, 454), (392, 384), (498, 454), (468, 474), (316, 474)], fill=roof
    )
    draw.line(
        [(286, 454), (392, 384), (498, 454)],
        fill=(82, 100, 58, _clamp(148 * alpha_scale)),
        width=5,
    )
    draw.rectangle([380, 482, 404, 548], fill=paper)
    draw.rectangle([334, 482, 366, 526], fill=(58, 76, 46, _clamp(124 * alpha_scale)))
    draw.rectangle([418, 482, 450, 526], fill=(58, 76, 46, _clamp(124 * alpha_scale)))
    # Stone lanterns and stepping stones help the background read as a place.
    for x in (160, 624):
        draw.rectangle(
            [x - 10, 484, x + 10, 554], fill=(88, 92, 80, _clamp(140 * alpha_scale))
        )
        draw.polygon(
            [(x - 26, 484), (x + 26, 484), (x + 14, 460), (x - 14, 460)],
            fill=(108, 112, 96, _clamp(148 * alpha_scale)),
        )
        draw.rectangle(
            [x - 12, 462, x + 12, 478], fill=_scale_alpha(glow, 0.38 * alpha_scale)
        )
    for sx, sy in ((350, 650), (372, 618), (394, 586), (408, 556)):
        draw.ellipse(
            [sx - 14, sy - 6, sx + 14, sy + 6],
            fill=(104, 96, 80, _clamp(90 * alpha_scale)),
        )


def _add_forest_floor(
    draw: ImageDraw.ImageDraw, color: RGBA, accent: RGBA, alpha_scale: float = 1.0
) -> None:
    _periodic_band(
        draw, _scale_alpha(color, 0.88 * alpha_scale), 560, 110, 18, 14, 0.5, 1.6
    )
    _periodic_band(
        draw, _scale_alpha(accent, 0.42 * alpha_scale), 534, 42, 14, 10, 1.8, 0.2
    )
    for x in (92, 176, 592, 676):
        _line(
            draw,
            [(x, 588), (x + 40, 548), (x + 108, 562)],
            _scale_alpha(accent, 0.28 * alpha_scale),
            3,
        )


def _add_forest_dojo(
    draw: ImageDraw.ImageDraw, theme: Theme, layer_key: str, alpha_scale: float
) -> None:
    _add_forest_canopy(
        draw,
        (16, 40, 22, _clamp(120 * alpha_scale)),
        (44, 82, 44, _clamp(108 * alpha_scale)),
        (208, 226, 176, _clamp(78 * alpha_scale)),
        alpha_scale,
    )
    if layer_key == "far_backplate":
        _add_forest_floor(
            draw,
            (18, 36, 22, _clamp(120 * alpha_scale)),
            (52, 86, 50, _clamp(98 * alpha_scale)),
            alpha_scale,
        )
        # Distant trunks and soft clearing.
        distant_trunk = (78, 50, 34, _clamp(88 * alpha_scale))
        distant_shadow = (46, 28, 18, _clamp(82 * alpha_scale))
        distant_highlight = (112, 78, 48, _clamp(76 * alpha_scale))
        for x, w, lean in ((98, 24, -8), (174, 30, 6), (598, 26, -4), (676, 22, 4)):
            _add_tree_trunk(
                draw,
                x,
                180,
                622,
                w,
                distant_trunk,
                distant_shadow,
                distant_highlight,
                lean=lean,
                alpha_scale=1.0,
                seed_offset=5,
            )
        for cx, cy in ((104, 188), (164, 162), (596, 174), (668, 152)):
            _add_leaf_cluster(
                draw,
                cx,
                cy,
                (24, 56, 30, _clamp(84 * alpha_scale)),
                (54, 88, 48, _clamp(74 * alpha_scale)),
                0.92,
            )
    elif layer_key == "near_background":
        _add_forest_floor(
            draw,
            (26, 52, 28, _clamp(154 * alpha_scale)),
            (70, 108, 62, _clamp(118 * alpha_scale)),
            alpha_scale,
        )
        _add_forest_midstory(draw, alpha_scale)
        _add_torii_and_dojo(
            draw,
            (40, 60, 38, _clamp(174 * alpha_scale)),
            (90, 126, 78, _clamp(128 * alpha_scale)),
            (220, 236, 196, _clamp(88 * alpha_scale)),
            alpha_scale,
        )
    else:
        # Foreground framing trunks and branch canopy.
        trunk = (118, 78, 46, _clamp(170 * alpha_scale))
        shadow = (74, 46, 28, _clamp(160 * alpha_scale))
        highlight = (154, 108, 66, _clamp(138 * alpha_scale))
        _add_tree_trunk(
            draw,
            42,
            -10,
            790,
            82,
            trunk,
            shadow,
            highlight,
            lean=18,
            alpha_scale=1.0,
            seed_offset=20,
        )
        _add_tree_trunk(
            draw,
            148,
            40,
            790,
            62,
            trunk,
            shadow,
            highlight,
            lean=-10,
            alpha_scale=1.0,
            seed_offset=21,
        )
        _add_tree_trunk(
            draw,
            638,
            10,
            790,
            76,
            trunk,
            shadow,
            highlight,
            lean=-18,
            alpha_scale=1.0,
            seed_offset=22,
        )
        _add_tree_trunk(
            draw,
            732,
            -20,
            790,
            64,
            trunk,
            shadow,
            highlight,
            lean=10,
            alpha_scale=1.0,
            seed_offset=23,
        )
        leaf_dark = (18, 48, 26, _clamp(146 * alpha_scale))
        leaf_mid = (52, 92, 50, _clamp(114 * alpha_scale))
        for cx, cy, sc in (
            (58, 116, 1.3),
            (134, 172, 1.1),
            (684, 114, 1.35),
            (624, 182, 1.1),
        ):
            _add_leaf_cluster(draw, cx, cy, leaf_dark, leaf_mid, sc)
        _line(draw, [(0, 128), (180, 170), (344, 132)], _scale_alpha(shadow, 0.75), 10)
        _line(
            draw, [(768, 118), (604, 162), (446, 140)], _scale_alpha(shadow, 0.75), 10
        )
        _add_bamboo_grove(
            draw,
            (32, 82, 42, _clamp(132 * alpha_scale)),
            (78, 132, 74, _clamp(114 * alpha_scale)),
            alpha_scale,
            foreground=True,
        )
        for x, y in ((214, 216), (548, 182), (646, 330), (146, 382), (332, 252)):
            draw.ellipse(
                [x - 3, y - 3, x + 3, y + 3],
                fill=(226, 238, 172, _clamp(126 * alpha_scale)),
            )


def _add_theme_landmark(
    draw: ImageDraw.ImageDraw, theme: Theme, layer_key: str, alpha_scale: float
) -> None:
    base = _rgba(theme.silhouette, _clamp(112 * alpha_scale))
    accent = _rgba(theme.accent, _clamp(96 * alpha_scale))
    glow = _rgba(theme.glow, _clamp(78 * alpha_scale))
    if theme.key == "hub":
        _add_scaffold(draw, base, accent)
        if layer_key != "foreground_atmosphere":
            _add_ring_station(
                draw, _scale_alpha(base, 1.05), _scale_alpha(accent, 1.05)
            )
            _add_city_horizon(draw, _scale_alpha(base, 0.85), _scale_alpha(glow, 0.65))
    elif theme.key == "lab":
        _add_reactor_towers(draw, base, glow)
        _add_hanging_cables(draw, _scale_alpha(base, 0.9), glow, alpha_scale)
    elif theme.key == "basement":
        _add_furnace_arches(draw, base, glow)
        _add_pipes(draw, _scale_alpha(base, 0.9), _scale_alpha(accent, 0.75))
    elif theme.key == "cove":
        _add_sea_cliffs(draw, base, accent)
        _add_reeds(
            draw, _scale_alpha(base, 0.9), _scale_alpha(accent, 0.9), alpha_scale
        )
    elif theme.key == "skybridge":
        _add_cloud_decks(draw, base, glow)
        if layer_key != "foreground_atmosphere":
            _add_bridge_pylons(
                draw, _scale_alpha(base, 1.05), _scale_alpha(accent, 0.85)
            )
    elif theme.key == "boss":
        _add_monoliths(draw, base, accent)
        _add_shards(draw, _scale_alpha(glow, 0.9))
    elif theme.key == "water":
        _add_ruins_and_kelp(draw, base, accent, glow)
    elif theme.key == "forest":
        _add_forest_dojo(draw, theme, layer_key, alpha_scale)
    elif theme.key == "cave":
        _add_stalactites(draw, _scale_alpha(base, 0.9))
        _add_crystals(draw, base, _scale_alpha(accent, 0.85))


def _draw_far_backplate(theme: Theme) -> Image.Image:
    rng = random.Random(_seed(theme.key, "far_backplate"))
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    mist = _rgba(_mix(theme.glow, theme.sky_mid, 0.65), 34)
    shadow = _rgba(_mix(theme.silhouette, theme.sky_bottom, 0.45), 58)
    _periodic_band(draw, mist, 124, 36, 12, 10, 0.4, 1.1)
    _periodic_band(draw, shadow, 408, 48, 16, 12, 1.7, 0.5)
    _periodic_band(draw, _scale_alpha(mist, 0.75), 472, 34, 14, 18, 2.0, 1.4)
    _add_theme_landmark(draw, theme, "far_backplate", 0.55)
    for _ in range(12):
        x = rng.randint(0, SIZE)
        y = rng.randint(60, SIZE - 60)
        r = rng.randint(18, 56)
        draw.ellipse(
            [x - r, y - r, x + r, y + r], fill=_rgba(theme.glow, rng.randint(4, 10))
        )
    return img.filter(ImageFilter.GaussianBlur(6))


def _draw_near_background(theme: Theme) -> Image.Image:
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    _add_theme_landmark(draw, theme, "near_background", 0.92)
    if theme.key in {"hub", "lab", "skybridge"}:
        _periodic_band(draw, _rgba(theme.glow, 18), 506, 16, 12, 8, 1.4, 0.2)
    elif theme.key in {"cove", "water", "cave"}:
        _periodic_band(draw, _rgba(theme.glow, 20), 544, 12, 16, 10, 0.8, 1.8)
    elif theme.key in {"boss", "basement"}:
        _periodic_band(draw, _rgba(theme.glow, 18), 532, 14, 10, 14, 1.7, 1.1)
    return img.filter(ImageFilter.GaussianBlur(2.2))


def _draw_foreground_atmosphere(theme: Theme) -> Image.Image:
    rng = random.Random(_seed(theme.key, "foreground_atmosphere"))
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img, "RGBA")
    _add_theme_landmark(draw, theme, "foreground_atmosphere", 0.72)
    # Stronger edge framing and local atmosphere.
    edge = _rgba(theme.silhouette, 48)
    draw.ellipse([-160, 60, 210, 728], fill=edge)
    draw.ellipse([SIZE - 210, 48, SIZE + 160, 730], fill=edge)
    for _ in range(8):
        y = rng.randint(90, 620)
        _line(
            draw,
            [
                (0, y),
                (SIZE * 0.35, y + rng.randint(-34, 34)),
                (SIZE, y + rng.randint(-22, 22)),
            ],
            _rgba(theme.glow, rng.randint(10, 22)),
            rng.randint(5, 9),
        )
    for _ in range(18):
        x = rng.randint(0, SIZE)
        y = rng.randint(24, SIZE - 24)
        r = rng.randint(3, 10)
        draw.ellipse(
            [x - r, y - r, x + r, y + r], fill=_rgba(theme.glow, rng.randint(6, 14))
        )
    if theme.key == "water":
        for _ in range(12):
            x = rng.randint(80, SIZE - 80)
            y = rng.randint(80, SIZE - 160)
            r = rng.randint(4, 10)
            draw.ellipse(
                [x - r, y - r, x + r, y + r], outline=_rgba(theme.glow, 26), width=2
            )
    if theme.key in {"basement", "boss"}:
        _draw_embers(draw, rng, _rgba(theme.glow, 36), 18)
    return img.filter(ImageFilter.GaussianBlur(5.0))


def render_layer(theme: Theme, layer: Layer) -> Image.Image:
    if layer.opaque:
        image = _draw_sky(theme)
    elif layer.key == "far_backplate":
        image = _draw_far_backplate(theme)
    elif layer.key == "near_background":
        image = _draw_near_background(theme)
    else:
        image = _draw_foreground_atmosphere(theme)

    if not layer.opaque:
        # Clamp alpha so gameplay remains readable.
        amax = 116 if layer.key == "near_background" else 84
        r, g, b, a = image.split()
        a = a.point(lambda v: min(v, amax))
        image = Image.merge("RGBA", (r, g, b, a))
    else:
        r, g, b, _a = image.split()
        image = Image.merge("RGBA", (r, g, b, Image.new("L", (SIZE, SIZE), 255)))
    return image


def write_background_layers(out_dir: str | Path) -> List[Path]:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []
    manifest = {
        "version": 6,
        "size": [SIZE, SIZE],
        "asset_root": "backgrounds/parallax_layers",
        "notes": (
            "Generated by ambition_parallax_renderer. These are code-generated, "
            "pre-rendered assets. v5 removes obvious tile repetition by using "
            "larger single-panel compositions at runtime and adds more specific "
            "biome landmarks instead of generic repeating shapes. Adds a distinct forest/ninja-dojo theme."
        ),
        "layers": [],
    }
    for theme in THEMES:
        for layer in LAYERS:
            img = render_layer(theme, layer)
            path = out / f"{theme.key}_{layer.key}.png"
            img.save(path)
            paths.append(path)
            alpha = img.getchannel("A").getextrema()
            manifest["layers"].append(
                {
                    "theme": theme.key,
                    "layer": layer.key,
                    "path": path.name,
                    "parallax_factor": layer.factor,
                    "z_hint": layer.z_hint,
                    "opaque": layer.opaque,
                    "alpha_minmax": [int(alpha[0]), int(alpha[1])],
                }
            )
    manifest_path = out / "parallax_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    paths.append(manifest_path)
    return paths
