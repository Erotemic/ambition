"""Procedural biome backgrounds and parallax atmosphere layers.

The generated assets are deliberately separate from gameplay sprites. Each
biome gets one mostly opaque sky/backdrop image plus transparent parallax plates
that add depth without competing with collision blocks, labels, hazards, or
projectiles.
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

RGBA = Tuple[int, int, int, int]
RGB = Tuple[int, int, int]
SIZE = 512


@dataclass(frozen=True)
class ParallaxTheme:
    key: str
    sky_top: RGB
    sky_mid: RGB
    sky_bottom: RGB
    base: RGBA
    accent: RGBA
    glow: RGBA
    motif: str
    celestial: str
    star_density: int = 0


@dataclass(frozen=True)
class ParallaxLayer:
    key: str
    factor: float
    z_hint: float
    alpha_scale: float
    blur: float
    opaque: bool = False


THEMES: Tuple[ParallaxTheme, ...] = (
    ParallaxTheme(
        "hub",
        (14, 14, 30),
        (24, 24, 52),
        (12, 16, 30),
        (24, 22, 54, 72),
        (70, 60, 122, 54),
        (112, 90, 170, 34),
        "braces",
        celestial="moon",
        star_density=64,
    ),
    ParallaxTheme(
        "lab",
        (7, 20, 30),
        (12, 36, 46),
        (6, 17, 27),
        (13, 30, 44, 70),
        (24, 104, 112, 54),
        (64, 200, 190, 32),
        "cables",
        celestial="stars",
        star_density=26,
    ),
    ParallaxTheme(
        "basement",
        (23, 18, 23),
        (41, 30, 31),
        (20, 16, 20),
        (50, 36, 34, 78),
        (105, 78, 54, 58),
        (188, 112, 40, 34),
        "ruins",
        celestial="ember",
        star_density=0,
    ),
    ParallaxTheme(
        "cove",
        (8, 31, 41),
        (14, 56, 61),
        (6, 25, 34),
        (13, 43, 48, 64),
        (34, 98, 76, 50),
        (118, 206, 183, 30),
        "reeds",
        celestial="moon",
        star_density=42,
    ),
    ParallaxTheme(
        "skybridge",
        (64, 90, 132),
        (103, 128, 162),
        (58, 78, 116),
        (84, 108, 134, 44),
        (158, 176, 195, 34),
        (235, 242, 248, 24),
        "clouds",
        celestial="sun",
        star_density=0,
    ),
    ParallaxTheme(
        "boss",
        (27, 8, 19),
        (48, 18, 30),
        (16, 8, 14),
        (52, 16, 31, 78),
        (122, 29, 50, 58),
        (230, 76, 44, 36),
        "spikes",
        celestial="eclipse",
        star_density=18,
    ),
    ParallaxTheme(
        "water",
        (4, 24, 42),
        (9, 54, 68),
        (3, 18, 34),
        (10, 48, 66, 68),
        (22, 104, 96, 52),
        (78, 185, 184, 32),
        "kelp",
        celestial="caustic",
        star_density=0,
    ),
    ParallaxTheme(
        "cave",
        (10, 11, 18),
        (22, 22, 32),
        (7, 8, 14),
        (25, 24, 32, 82),
        (58, 49, 62, 60),
        (102, 94, 122, 28),
        "stalactites",
        celestial="none",
        star_density=0,
    ),
)

LAYERS: Tuple[ParallaxLayer, ...] = (
    # Sky is opaque enough to fully cover the debug grid when present.
    ParallaxLayer("sky", 0.08, -18.0, 1.0, 0.0, opaque=True),
    ParallaxLayer("far_backplate", 0.18, -17.0, 0.70, 1.2),
    ParallaxLayer("near_background", 0.55, -16.0, 0.92, 1.5),
    ParallaxLayer("foreground_atmosphere", 0.82, -15.0, 0.75, 2.4),
)


def _seed(theme: str, layer: str) -> int:
    digest = hashlib.blake2b(f"{theme}:{layer}".encode(), digest_size=8).digest()
    return int.from_bytes(digest, "little")


def _mix(a: RGB, b: RGB, t: float) -> RGB:
    return tuple(int(round(x + (y - x) * t)) for x, y in zip(a, b))  # type: ignore[return-value]


def _scaled(color: RGBA, scale: float) -> RGBA:
    r, g, b, a = color
    return (r, g, b, max(0, min(255, int(a * scale))))


def _line(draw: ImageDraw.ImageDraw, pts: Iterable[Tuple[float, float]], color: RGBA, width: int) -> None:
    draw.line([(int(x), int(y)) for x, y in pts], fill=color, width=width, joint="curve")


def _poly(draw: ImageDraw.ImageDraw, pts: Iterable[Tuple[float, float]], color: RGBA) -> None:
    draw.polygon([(int(x), int(y)) for x, y in pts], fill=color)


def _soft_band(draw: ImageDraw.ImageDraw, rng: random.Random, color: RGBA, y: int, h: int, wiggle: int = 18) -> None:
    points = []
    for i in range(9):
        x = i * (SIZE / 8)
        points.append((x, y + rng.randint(-wiggle, wiggle)))
    lower = [(x, yy + h + rng.randint(-wiggle // 2, wiggle // 2)) for x, yy in reversed(points)]
    _poly(draw, points + lower, color)


def _draw_stars(draw: ImageDraw.ImageDraw, rng: random.Random, count: int, color: RGBA) -> None:
    for _ in range(count):
        x = rng.randint(8, SIZE - 8)
        y = rng.randint(10, int(SIZE * 0.56))
        r = rng.choice([1, 1, 1, 2])
        alpha = max(12, min(255, color[3] + rng.randint(-28, 18)))
        c = (color[0], color[1], color[2], alpha)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=c)
        if r == 2 and rng.random() < 0.5:
            draw.line([(x - 3, y), (x + 3, y)], fill=(color[0], color[1], color[2], alpha // 2), width=1)
            draw.line([(x, y - 3), (x, y + 3)], fill=(color[0], color[1], color[2], alpha // 2), width=1)


def _draw_moon(image: Image.Image, center: Tuple[int, int], radius: int, color: RGBA, phase_offset: int = 0) -> None:
    overlay = Image.new("RGBA", image.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")
    x, y = center
    glow = Image.new("RGBA", image.size, (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow, "RGBA")
    gdraw.ellipse([x - radius - 18, y - radius - 18, x + radius + 18, y + radius + 18], fill=(color[0], color[1], color[2], max(12, color[3] // 5)))
    glow = glow.filter(ImageFilter.GaussianBlur(12))
    draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=color)
    # Simple crescent cutout.
    shadow = (0, 0, 0, 110)
    draw.ellipse([x - radius + phase_offset, y - radius - 1, x + radius + phase_offset, y + radius + 1], fill=shadow)
    image.alpha_composite(glow)
    image.alpha_composite(overlay)


def _draw_sun(image: Image.Image, center: Tuple[int, int], radius: int, core: RGBA, glow_color: RGBA) -> None:
    overlay = Image.new("RGBA", image.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")
    x, y = center
    for grow, alpha in ((42, 22), (26, 34), (12, 52)):
        draw.ellipse([x - radius - grow, y - radius - grow, x + radius + grow, y + radius + grow], fill=(glow_color[0], glow_color[1], glow_color[2], alpha))
    draw.ellipse([x - radius, y - radius, x + radius, y + radius], fill=core)
    overlay = overlay.filter(ImageFilter.GaussianBlur(10))
    image.alpha_composite(overlay)


def _draw_eclipse(image: Image.Image, center: Tuple[int, int], radius: int) -> None:
    overlay = Image.new("RGBA", image.size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(overlay, "RGBA")
    x, y = center
    for grow, alpha in ((22, 34), (12, 48), (4, 80)):
        draw.ellipse([x - radius - grow, y - radius - grow, x + radius + grow, y + radius + grow], outline=(220, 86, 54, alpha), width=3)
    draw.ellipse([x - radius + 4, y - radius + 2, x + radius + 4, y + radius + 2], fill=(12, 8, 14, 220))
    draw.ellipse([x - radius - 8, y - radius - 8, x + radius - 8, y + radius - 8], fill=(0, 0, 0, 220))
    overlay = overlay.filter(ImageFilter.GaussianBlur(3))
    image.alpha_composite(overlay)


def _draw_caustics(draw: ImageDraw.ImageDraw, rng: random.Random, color: RGBA) -> None:
    for y in (76, 128, 190):
        for _ in range(4):
            x0 = rng.randint(-40, SIZE - 80)
            pts = [(x0, y + rng.randint(-8, 8)), (x0 + 80, y + rng.randint(-12, 12)), (x0 + 160, y + rng.randint(-8, 8))]
            _line(draw, pts, color, rng.randint(2, 3))


def _draw_embers(draw: ImageDraw.ImageDraw, rng: random.Random, count: int, color: RGBA) -> None:
    for _ in range(count):
        x = rng.randint(16, SIZE - 16)
        y = rng.randint(int(SIZE * 0.10), int(SIZE * 0.70))
        r = rng.randint(1, 3)
        alpha = max(18, min(255, color[3] + rng.randint(-18, 16)))
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(color[0], color[1], color[2], alpha))


def _render_sky(theme: ParallaxTheme) -> Image.Image:
    """Render a mostly opaque sky/backdrop image.

    This is the layer that hides the debug grid. It deliberately carries the
    main "old background" identity: gradient sky, haze, and occasional celestial
    detail such as sun / moon / stars.
    """
    image = Image.new("RGBA", (SIZE, SIZE), (*theme.sky_mid, 255))
    px = image.load()
    for y in range(SIZE):
        t = y / (SIZE - 1)
        if t < 0.55:
            c = _mix(theme.sky_top, theme.sky_mid, t / 0.55)
        else:
            c = _mix(theme.sky_mid, theme.sky_bottom, (t - 0.55) / 0.45)
        for x in range(SIZE):
            nx = math.sin((x / SIZE) * math.tau * 1.8 + 0.7) * 3.0
            ny = math.sin((y / SIZE) * math.tau * 0.85 + 1.4) * 2.0
            vignette = -11.0 * max(0.0, (abs(x - SIZE / 2) / (SIZE / 2) - 0.42))
            shade = int(round(nx + ny + vignette))
            px[x, y] = (
                max(0, min(255, c[0] + shade)),
                max(0, min(255, c[1] + shade)),
                max(0, min(255, c[2] + shade)),
                255,
            )

    sky_rng = random.Random(_seed(theme.key, "sky"))
    sky_overlay = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    sdraw = ImageDraw.Draw(sky_overlay, "RGBA")

    if theme.star_density > 0:
        star_color = (*_mix(theme.sky_mid, (255, 250, 240), 0.82), 120)
        _draw_stars(sdraw, sky_rng, theme.star_density, star_color)

    haze = (*_mix(theme.sky_mid, theme.glow[:3], 0.42), 36)
    _soft_band(sdraw, sky_rng, haze, 320, 48, 18)
    _soft_band(sdraw, sky_rng, (*theme.glow[:3], 18), 372, 34, 24)

    image.alpha_composite(sky_overlay)

    if theme.celestial == "moon":
        cx = 110 if theme.key in {"cove", "hub"} else 398
        cy = 92 if theme.key == "hub" else 112
        radius = 26 if theme.key == "hub" else 22
        _draw_moon(image, (cx, cy), radius, (236, 236, 244, 228), phase_offset=10)
    elif theme.celestial == "sun":
        _draw_sun(image, (390, 104), 30, (255, 241, 199, 228), (255, 244, 214, 80))
    elif theme.celestial == "eclipse":
        _draw_eclipse(image, (394, 94), 26)
    elif theme.celestial == "caustic":
        overlay = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        odraw = ImageDraw.Draw(overlay, "RGBA")
        _draw_caustics(odraw, sky_rng, (114, 223, 216, 34))
        overlay = overlay.filter(ImageFilter.GaussianBlur(1.2))
        image.alpha_composite(overlay)
    elif theme.celestial == "ember":
        overlay = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        odraw = ImageDraw.Draw(overlay, "RGBA")
        _draw_embers(odraw, sky_rng, 18, (228, 150, 84, 34))
        image.alpha_composite(overlay)

    # Add a few tiny technical glints for the lab sky, like distant panel LEDs.
    if theme.key == "lab":
        overlay = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
        odraw = ImageDraw.Draw(overlay, "RGBA")
        for x in (82, 118, 418, 452):
            odraw.rounded_rectangle([x, 70 + (x % 3) * 16, x + 12, 74 + (x % 3) * 16], radius=1, fill=(92, 214, 200, 26))
        overlay = overlay.filter(ImageFilter.GaussianBlur(1.0))
        image.alpha_composite(overlay)

    image = image.filter(ImageFilter.GaussianBlur(0.35))
    r, g, b, _a = image.split()
    return Image.merge("RGBA", (r, g, b, Image.new("L", (SIZE, SIZE), 255)))


def _draw_corner_vignette(draw: ImageDraw.ImageDraw, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    color = _scaled(theme.base, 0.38 * layer.alpha_scale)
    pad = 92 if layer.key == "foreground_atmosphere" else 64
    for sx in (0, 1):
        for sy in (0, 1):
            x0 = -pad if sx == 0 else SIZE - pad
            y0 = -pad if sy == 0 else SIZE - pad
            draw.ellipse([x0, y0, x0 + pad * 2, y0 + pad * 2], fill=color)


def _draw_braces(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    _poly(draw, [(0, 0), (SIZE, 0), (SIZE, 24), (430, 18), (330, 32), (190, 20), (80, 34), (0, 26)], c)
    for side in (0, 1):
        x = 22 if side == 0 else SIZE - 22
        lean = 34 if side == 0 else -34
        for y in (80, 190, 330):
            _line(draw, [(x, y), (x + lean, y + 76)], a, rng.randint(4, 7))
    for _ in range(4):
        x0 = rng.choice([rng.randint(0, 85), rng.randint(430, SIZE)])
        y0 = rng.choice([rng.randint(28, 95), rng.randint(420, 500)])
        _line(draw, [(x0, y0), (x0 + rng.randint(-45, 45), y0 + rng.randint(8, 30))], a, rng.randint(3, 5))


def _draw_cables(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    for i in range(6):
        x = rng.choice([rng.randint(20, 120), rng.randint(390, 500)])
        length = rng.randint(80, 220)
        sag = rng.randint(8, 28)
        pts = [(x, 0), (x + rng.randint(-8, 8), length * 0.45), (x + rng.randint(-18, 18), length + sag)]
        _line(draw, pts, c, rng.randint(2, 5))
        if i % 2 == 0:
            y = length + sag
            _line(draw, [(x - 8, y), (x + 8, y)], a, 2)
    for side in (0, 1):
        x0 = 0 if side == 0 else SIZE - 16
        draw.rounded_rectangle([x0, 85, x0 + 16, 440], radius=4, fill=_scaled(c, 0.72))


def _draw_ruins(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    side = rng.choice([0, 1])
    if side == 0:
        _poly(draw, [(0, 55), (72, 40), (88, SIZE), (0, SIZE)], c)
        _line(draw, [(60, 80), (72, 250), (55, 480)], a, 3)
    else:
        _poly(draw, [(SIZE, 60), (440, 44), (424, SIZE), (SIZE, SIZE)], c)
        _line(draw, [(455, 70), (438, 255), (462, 470)], a, 3)
    for _ in range(5):
        x = rng.choice([rng.randint(0, 130), rng.randint(380, 510)])
        y = rng.randint(410, 508)
        _poly(draw, [(x - 30, y), (x + 38, y + 8), (x + 18, SIZE), (x - 48, SIZE)], _scaled(a, 0.65))


def _draw_reeds(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    for side in (0, 1):
        xr = range(8, 120) if side == 0 else range(392, 505)
        for _ in range(11):
            x = rng.choice(list(xr))
            h = rng.randint(58, 160)
            bend = rng.randint(-18, 18)
            _line(draw, [(x, SIZE), (x + bend * 0.4, SIZE - h * 0.55), (x + bend, SIZE - h)], c, rng.randint(2, 4))
            if rng.random() < 0.35:
                _line(draw, [(x + bend, SIZE - h), (x + bend + rng.choice([-18, 18]), SIZE - h + 18)], a, 2)
    for _ in range(4):
        x = rng.randint(10, SIZE - 10)
        _line(draw, [(x, 0), (x + rng.randint(-60, 60), rng.randint(32, 86))], _scaled(c, 0.45), 4)


def _draw_clouds(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.glow, layer.alpha_scale)
    for y in (58, 404):
        _soft_band(draw, rng, c, y + rng.randint(-12, 12), rng.randint(18, 42), 22)
    for _ in range(8):
        x0 = rng.choice([rng.randint(-40, 125), rng.randint(385, 540)])
        y0 = rng.randint(80, 430)
        _line(draw, [(x0, y0), (x0 + rng.randint(80, 170), y0 + rng.randint(-18, 18))], a, rng.randint(2, 4))


def _draw_spikes(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    for top in (True, False):
        y_edge = 0 if top else SIZE
        sign = 1 if top else -1
        for _ in range(8):
            x = rng.choice([rng.randint(0, 150), rng.randint(360, 512), rng.randint(0, 512)])
            h = rng.randint(22, 78)
            w = rng.randint(10, 32)
            _poly(draw, [(x - w, y_edge), (x + w, y_edge), (x + rng.randint(-8, 8), y_edge + sign * h)], c)
    for _ in range(8):
        x = rng.choice([rng.randint(0, 120), rng.randint(390, 512)])
        y = rng.randint(60, 450)
        _poly(draw, [(x, y - 14), (x + 18, y), (x + 3, y + 22), (x - 15, y + 5)], _scaled(a, 0.55))


def _draw_kelp(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    for side in (0, 1):
        xr = range(8, 100) if side == 0 else range(415, 505)
        for _ in range(8):
            x = rng.choice(list(xr))
            pts = []
            for j in range(6):
                y = SIZE - j * rng.randint(38, 58)
                pts.append((x + math.sin(j * 1.6 + rng.random()) * rng.randint(9, 24), y))
            _line(draw, pts, c, rng.randint(3, 7))
            if rng.random() < 0.6:
                for px, py in pts[1:-1:2]:
                    draw.ellipse([px - 8, py - 4, px + 10, py + 6], fill=_scaled(a, 0.55))
    for _ in range(14):
        x = rng.choice([rng.randint(0, 140), rng.randint(370, 512)])
        y = rng.randint(45, 465)
        r = rng.randint(2, 6)
        draw.ellipse([x - r, y - r, x + r, y + r], outline=_scaled(theme.glow, 0.65), width=1)


def _draw_stalactites(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    c = _scaled(theme.base, layer.alpha_scale)
    a = _scaled(theme.accent, layer.alpha_scale)
    _poly(draw, [(0, 0), (SIZE, 0), (SIZE, 22), (430, 34), (330, 16), (240, 42), (140, 24), (0, 38)], c)
    for _ in range(13):
        x = rng.randint(0, SIZE)
        h = rng.randint(32, 112)
        w = rng.randint(8, 28)
        if 160 < x < 350 and rng.random() < 0.7:
            continue
        _poly(draw, [(x - w, 0), (x + w, 0), (x + rng.randint(-8, 8), h)], c)
    for side in (0, 1):
        x = 0 if side == 0 else SIZE
        sign = 1 if side == 0 else -1
        for y in (120, 260, 410):
            _line(draw, [(x, y), (x + sign * rng.randint(34, 72), y + rng.randint(-18, 24))], a, rng.randint(5, 9))


def _draw_theme_motif(draw: ImageDraw.ImageDraw, rng: random.Random, theme: ParallaxTheme, layer: ParallaxLayer) -> None:
    motif = theme.motif
    if motif == "braces":
        _draw_braces(draw, rng, theme, layer)
    elif motif == "cables":
        _draw_cables(draw, rng, theme, layer)
    elif motif == "ruins":
        _draw_ruins(draw, rng, theme, layer)
    elif motif == "reeds":
        _draw_reeds(draw, rng, theme, layer)
    elif motif == "clouds":
        _draw_clouds(draw, rng, theme, layer)
    elif motif == "spikes":
        _draw_spikes(draw, rng, theme, layer)
    elif motif == "kelp":
        _draw_kelp(draw, rng, theme, layer)
    elif motif == "stalactites":
        _draw_stalactites(draw, rng, theme, layer)
    else:
        _draw_braces(draw, rng, theme, layer)


def render_layer(theme: ParallaxTheme, layer: ParallaxLayer) -> Image.Image:
    if layer.opaque:
        return _render_sky(theme)

    rng = random.Random(_seed(theme.key, layer.key))
    scale = 2
    base = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(base, "RGBA")

    if layer.key == "far_backplate":
        for y in (36, 438):
            _soft_band(draw, rng, _scaled(theme.glow, 0.60 * layer.alpha_scale), y, rng.randint(28, 58), 24)
        _draw_corner_vignette(draw, theme, layer)
    elif layer.key == "near_background":
        _draw_theme_motif(draw, rng, theme, layer)
        _draw_corner_vignette(draw, theme, layer)
    else:
        _draw_theme_motif(draw, rng, theme, layer)
        mask = Image.new("L", (SIZE, SIZE), 0)
        md = ImageDraw.Draw(mask)
        md.ellipse([82, 70, 430, 442], fill=230)
        mask = mask.filter(ImageFilter.GaussianBlur(58))
        r, g, b, a = base.split()
        a = Image.composite(Image.new("L", (SIZE, SIZE), 0), a, mask.point(lambda v: min(255, int(v * 0.94))))
        base = Image.merge("RGBA", (r, g, b, a))
        draw = ImageDraw.Draw(base, "RGBA")
        _draw_corner_vignette(draw, theme, layer)

    enlarged = base.resize((SIZE * scale, SIZE * scale), Image.Resampling.BICUBIC)
    if layer.blur > 0:
        enlarged = enlarged.filter(ImageFilter.GaussianBlur(layer.blur * scale))
    out = enlarged.resize((SIZE, SIZE), Image.Resampling.LANCZOS)
    max_alpha = 66 if layer.key == "foreground_atmosphere" else 84
    r, g, b, a = out.split()
    a = a.point(lambda v: min(v, max_alpha))
    return Image.merge("RGBA", (r, g, b, a))


def write_background_layers(out_dir: str | Path) -> List[Path]:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    paths: List[Path] = []
    manifest = {
        "version": 3,
        "size": [SIZE, SIZE],
        "asset_root": "backgrounds/parallax_layers",
        "notes": (
            "Generated by ambition_parallax_renderer. The sky layer is opaque "
            "enough to hide the debug grid and now includes biome-specific sky "
            "features like sun, moon, stars, or underwater caustics. Transparent "
            "plates are designed to be tiled at runtime so near/far silhouettes "
            "remain visible in large rooms."
        ),
        "layers": [],
    }
    for theme in THEMES:
        for layer in LAYERS:
            image = render_layer(theme, layer)
            path = out / f"{theme.key}_{layer.key}.png"
            image.save(path)
            paths.append(path)
            alpha = image.getchannel("A")
            manifest["layers"].append(
                {
                    "theme": theme.key,
                    "layer": layer.key,
                    "path": path.name,
                    "parallax_factor": layer.factor,
                    "z_hint": layer.z_hint,
                    "opaque": layer.opaque,
                    "alpha_minmax": [int(alpha.getextrema()[0]), int(alpha.getextrema()[1])],
                }
            )
    manifest_path = out / "parallax_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    paths.append(manifest_path)
    return paths
