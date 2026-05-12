from __future__ import annotations

import math
import random
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

from .profiles import BackgroundProfile, LayerSpec


Color = tuple[int, int, int, int]


@dataclass(frozen=True)
class SkyStyle:
    top: Color
    bottom: Color
    star_color: Color
    star_count: int
    glow_color: Color
    glow_center: tuple[float, float]


@dataclass(frozen=True)
class LayerStyle:
    far_colors: tuple[Color, Color]
    mid_fill: Color
    mid_detail: Color
    mid_glow: Color
    near_line: Color
    near_blob: Color
    accent: Color
    silhouette_kind: str
    foreground_kind: str


@dataclass(frozen=True)
class ProfileStyle:
    sky: SkyStyle
    layers: LayerStyle


PROFILE_STYLES: dict[str, ProfileStyle] = {
    "default": ProfileStyle(
        sky=SkyStyle((14, 20, 54, 255), (54, 76, 132, 255), (210, 235, 255, 190), 170, (146, 208, 255, 132), (0.74, 0.20)),
        layers=LayerStyle(
            far_colors=((62, 92, 150, 150), (38, 62, 120, 180)),
            mid_fill=(42, 72, 132, 190),
            mid_detail=(58, 90, 152, 165),
            mid_glow=(164, 210, 255, 90),
            near_line=(6, 12, 30, 92),
            near_blob=(20, 42, 80, 84),
            accent=(120, 180, 255, 80),
            silhouette_kind="spires",
            foreground_kind="branches",
        ),
    ),
    "hub": ProfileStyle(
        sky=SkyStyle((18, 28, 78, 255), (58, 88, 152, 255), (220, 238, 255, 180), 150, (164, 220, 255, 144), (0.72, 0.18)),
        layers=LayerStyle(
            far_colors=((60, 96, 164, 150), (44, 68, 130, 184)),
            mid_fill=(46, 80, 148, 192),
            mid_detail=(66, 104, 176, 172),
            mid_glow=(176, 224, 255, 96),
            near_line=(8, 18, 42, 92),
            near_blob=(28, 58, 106, 84),
            accent=(112, 180, 255, 88),
            silhouette_kind="spires",
            foreground_kind="branches",
        ),
    ),
    "lab": ProfileStyle(
        sky=SkyStyle((8, 30, 44, 255), (26, 78, 98, 255), (162, 255, 236, 150), 70, (64, 232, 210, 116), (0.78, 0.16)),
        layers=LayerStyle(
            far_colors=((26, 66, 86, 150), (18, 44, 66, 182)),
            mid_fill=(24, 70, 94, 194),
            mid_detail=(42, 108, 132, 176),
            mid_glow=(112, 248, 224, 96),
            near_line=(4, 16, 22, 96),
            near_blob=(16, 54, 62, 88),
            accent=(82, 220, 214, 90),
            silhouette_kind="pipes",
            foreground_kind="cables",
        ),
    ),
    "basement": ProfileStyle(
        sky=SkyStyle((20, 14, 30, 255), (54, 44, 76, 255), (242, 224, 196, 70), 24, (232, 170, 110, 72), (0.24, 0.16)),
        layers=LayerStyle(
            far_colors=((74, 64, 88, 150), (44, 34, 52, 184)),
            mid_fill=(74, 62, 78, 194),
            mid_detail=(112, 96, 108, 178),
            mid_glow=(244, 196, 126, 70),
            near_line=(24, 14, 24, 100),
            near_blob=(76, 54, 60, 86),
            accent=(216, 150, 96, 88),
            silhouette_kind="ruins",
            foreground_kind="pillars",
        ),
    ),
    "cove": ProfileStyle(
        sky=SkyStyle((12, 34, 58, 255), (26, 90, 114, 255), (206, 255, 250, 120), 80, (104, 244, 236, 92), (0.82, 0.22)),
        layers=LayerStyle(
            far_colors=((24, 82, 98, 150), (16, 56, 74, 184)),
            mid_fill=(26, 92, 112, 192),
            mid_detail=(42, 128, 146, 174),
            mid_glow=(120, 254, 240, 94),
            near_line=(8, 26, 30, 98),
            near_blob=(18, 72, 74, 88),
            accent=(124, 252, 228, 92),
            silhouette_kind="palms",
            foreground_kind="reeds",
        ),
    ),
    "skybridge": ProfileStyle(
        sky=SkyStyle((46, 94, 150, 255), (146, 198, 240, 255), (255, 255, 255, 80), 24, (255, 255, 220, 96), (0.18, 0.18)),
        layers=LayerStyle(
            far_colors=((170, 194, 224, 150), (126, 156, 208, 176)),
            mid_fill=(102, 138, 192, 168),
            mid_detail=(138, 176, 224, 154),
            mid_glow=(248, 250, 255, 86),
            near_line=(72, 92, 128, 84),
            near_blob=(158, 192, 230, 72),
            accent=(255, 255, 255, 90),
            silhouette_kind="bridges",
            foreground_kind="gusts",
        ),
    ),
    "boss": ProfileStyle(
        sky=SkyStyle((26, 6, 14, 255), (88, 18, 32, 255), (255, 188, 188, 80), 18, (255, 96, 128, 112), (0.50, 0.22)),
        layers=LayerStyle(
            far_colors=((96, 24, 44, 160), (54, 10, 24, 192)),
            mid_fill=(104, 18, 42, 204),
            mid_detail=(150, 36, 70, 188),
            mid_glow=(255, 126, 154, 88),
            near_line=(34, 2, 12, 102),
            near_blob=(124, 24, 50, 88),
            accent=(255, 110, 144, 96),
            silhouette_kind="shards",
            foreground_kind="spikes",
        ),
    ),
    "water": ProfileStyle(
        sky=SkyStyle((8, 42, 78, 255), (36, 122, 164, 255), (220, 255, 255, 60), 20, (168, 255, 255, 72), (0.76, 0.15)),
        layers=LayerStyle(
            far_colors=((34, 104, 132, 150), (20, 76, 98, 186)),
            mid_fill=(30, 112, 146, 190),
            mid_detail=(44, 148, 182, 170),
            mid_glow=(178, 250, 255, 88),
            near_line=(6, 22, 44, 98),
            near_blob=(26, 82, 104, 86),
            accent=(180, 248, 255, 88),
            silhouette_kind="sea",
            foreground_kind="kelp",
        ),
    ),
    "cave": ProfileStyle(
        sky=SkyStyle((8, 10, 20, 255), (24, 34, 54, 255), (216, 240, 255, 48), 16, (126, 182, 255, 54), (0.68, 0.16)),
        layers=LayerStyle(
            far_colors=((34, 42, 72, 154), (18, 24, 52, 188)),
            mid_fill=(32, 42, 86, 196),
            mid_detail=(54, 70, 116, 182),
            mid_glow=(154, 196, 255, 80),
            near_line=(10, 12, 26, 100),
            near_blob=(44, 56, 102, 90),
            accent=(124, 178, 255, 88),
            silhouette_kind="cave",
            foreground_kind="drips",
        ),
    ),
}


def _style_for_profile(profile_name: str) -> ProfileStyle:
    return PROFILE_STYLES.get(profile_name, PROFILE_STYLES["default"])


def _lerp(a: int, b: int, t: float) -> int:
    return int(round(a + (b - a) * t))


def _vertical_gradient(size: tuple[int, int], top: Color, bottom: Color) -> Image.Image:
    width, height = size
    image = Image.new("RGBA", size)
    draw = ImageDraw.Draw(image)
    for y in range(height):
        t = y / max(1, height - 1)
        color = tuple(_lerp(top[i], bottom[i], t) for i in range(4))
        draw.line([(0, y), (width, y)], fill=color)
    return image


def _draw_stars(draw: ImageDraw.ImageDraw, rng: random.Random, width: int, height: int, count: int, color: Color) -> None:
    for _ in range(count):
        x = rng.randrange(0, width)
        y = rng.randrange(0, int(height * 0.72))
        radius = rng.choice((1, 1, 1, 2))
        alpha = rng.randrange(max(16, color[3] // 2), max(20, color[3]))
        tinted = (color[0], color[1], color[2], alpha)
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=tinted)


def _smooth_polyline(
    rng: random.Random,
    width: int,
    base_y: float,
    amplitude: float,
    step: int,
    phase: float,
) -> list[tuple[int, int]]:
    points: list[tuple[int, int]] = []
    for x in range(-step, width + step * 2, step):
        noise = rng.uniform(-amplitude, amplitude)
        wave = math.sin((x / max(1, width)) * math.tau + phase) * amplitude * 0.5
        y = int(round(base_y + noise + wave))
        points.append((x, y))
    return points


def _fill_silhouette(draw: ImageDraw.ImageDraw, points: list[tuple[int, int]], width: int, height: int, fill: Color) -> None:
    polygon = [(0, height), *points, (width, height)]
    draw.polygon(polygon, fill=fill)


def _render_sky(profile_name: str, spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    style = _style_for_profile(profile_name).sky
    image = _vertical_gradient((spec.width, spec.height), style.top, style.bottom)
    draw = ImageDraw.Draw(image, "RGBA")
    if style.star_count > 0:
        _draw_stars(draw, rng, spec.width, spec.height, style.star_count, style.star_color)
    glow = Image.new("RGBA", image.size, (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow, "RGBA")
    cx = int(spec.width * style.glow_center[0])
    cy = int(spec.height * style.glow_center[1])
    for radius, alpha_scale in ((118, 0.35), (74, 0.55), (34, 1.0)):
        fill = (*style.glow_color[:3], int(style.glow_color[3] * alpha_scale))
        gdraw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=fill)
    glow = glow.filter(ImageFilter.GaussianBlur(8))
    base = Image.alpha_composite(image, glow)
    if profile_name == "water":
        water = ImageDraw.Draw(base, "RGBA")
        for band in range(9):
            y = int(spec.height * (0.58 + band * 0.035))
            water.line((0, y, spec.width, y), fill=(180, 255, 255, 12), width=2)
    elif profile_name == "skybridge":
        cloud = Image.new("RGBA", base.size, (0, 0, 0, 0))
        cdraw = ImageDraw.Draw(cloud, "RGBA")
        for _ in range(10):
            x = rng.randrange(-30, spec.width)
            y = rng.randrange(10, int(spec.height * 0.45))
            w = rng.randrange(54, 120)
            h = rng.randrange(18, 34)
            for k in range(4):
                ox = x + k * (w // 5)
                oy = y + rng.randrange(-4, 5)
                cdraw.ellipse((ox, oy, ox + w // 3, oy + h), fill=(255, 255, 255, 38))
        base = Image.alpha_composite(base, cloud.filter(ImageFilter.GaussianBlur(2.0)))
    return base


def _render_far(profile_name: str, spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    style = _style_for_profile(profile_name).layers
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    for band, (base, amp, color) in enumerate(
        (
            (spec.height * 0.61, 42, style.far_colors[0]),
            (spec.height * 0.70, 36, style.far_colors[1]),
        )
    ):
        points = _smooth_polyline(rng, spec.width, base, amp, 42, phase=band * 1.7)
        _fill_silhouette(draw, points, spec.width, spec.height, color)
    if style.silhouette_kind in {"cave", "sea"}:
        for _ in range(8):
            x = rng.randrange(-20, spec.width + 20)
            y = rng.randrange(int(spec.height * 0.18), int(spec.height * 0.50))
            w = rng.randrange(24, 70)
            h = rng.randrange(40, 110)
            if style.silhouette_kind == "cave":
                draw.polygon([(x, 0), (x + w // 2, y), (x + w, 0)], fill=style.far_colors[1])
            else:
                draw.arc((x, y, x + w, y + h), 0, 180, fill=style.accent, width=2)
    return image.filter(ImageFilter.GaussianBlur(0.4))


def _draw_mid_detail(draw: ImageDraw.ImageDraw, rng: random.Random, spec: LayerSpec, style: LayerStyle) -> None:
    kind = style.silhouette_kind
    if kind in {"spires", "ruins"}:
        for i in range(11):
            x = int((i / 11.0) * spec.width + rng.uniform(-12, 12))
            w = rng.randrange(10, 26)
            h = rng.randrange(55, 165)
            y0 = int(spec.height * 0.76) - h
            draw.rounded_rectangle((x, y0, x + w, spec.height), radius=3, fill=style.mid_detail)
            if rng.random() < 0.55:
                draw.rectangle((x + 3, y0 + 8, x + w - 3, y0 + 12), fill=style.mid_glow)
    elif kind == "pipes":
        for i in range(7):
            x = int((i / 7.0) * spec.width + rng.uniform(-18, 18))
            w = rng.randrange(20, 44)
            h = rng.randrange(80, 180)
            y0 = int(spec.height * 0.74) - h
            draw.rounded_rectangle((x, y0, x + w, spec.height), radius=8, fill=style.mid_detail)
            draw.line((x + w // 2, y0, x + w // 2, y0 - 40), fill=style.accent, width=4)
            if rng.random() < 0.7:
                draw.rectangle((x + 5, y0 + 12, x + w - 5, y0 + 16), fill=style.mid_glow)
    elif kind == "palms":
        for _ in range(9):
            x = rng.randrange(10, spec.width - 10)
            trunk_h = rng.randrange(70, 140)
            base_y = int(spec.height * 0.80)
            top_y = base_y - trunk_h
            draw.line((x, base_y, x + rng.randrange(-10, 10), top_y), fill=style.mid_detail, width=6)
            for angle in (-1.1, -0.6, -0.2, 0.2, 0.7):
                dx = int(math.cos(angle) * rng.randrange(30, 60))
                dy = int(math.sin(angle) * rng.randrange(18, 34))
                draw.line((x, top_y, x + dx, top_y + dy), fill=style.accent, width=3)
    elif kind == "bridges":
        for _ in range(6):
            x0 = rng.randrange(-20, spec.width - 40)
            y0 = rng.randrange(int(spec.height * 0.42), int(spec.height * 0.70))
            w = rng.randrange(80, 150)
            draw.rounded_rectangle((x0, y0, x0 + w, y0 + 10), radius=4, fill=style.mid_detail)
            draw.line((x0 + 10, y0 + 10, x0 + 10, spec.height), fill=style.mid_detail, width=6)
            draw.line((x0 + w - 10, y0 + 10, x0 + w - 10, spec.height), fill=style.mid_detail, width=6)
    elif kind == "shards":
        for _ in range(16):
            x = rng.randrange(-20, spec.width + 20)
            y = rng.randrange(int(spec.height * 0.34), int(spec.height * 0.84))
            size = rng.randrange(18, 60)
            draw.polygon(
                [(x, y - size), (x + int(size * 0.4), y), (x, y + size), (x - int(size * 0.6), y)],
                fill=style.mid_detail,
            )
    elif kind == "sea":
        for _ in range(8):
            x = rng.randrange(0, spec.width)
            y = rng.randrange(int(spec.height * 0.42), int(spec.height * 0.72))
            w = rng.randrange(54, 120)
            h = rng.randrange(20, 60)
            draw.arc((x, y, x + w, y + h), 0, 180, fill=style.mid_glow, width=3)
    elif kind == "cave":
        for _ in range(10):
            x = rng.randrange(0, spec.width)
            y = rng.randrange(int(spec.height * 0.46), int(spec.height * 0.78))
            size = rng.randrange(16, 44)
            draw.polygon([(x, y - size), (x + size, y), (x, y + size), (x - size, y)], fill=style.mid_detail)


def _render_mid(profile_name: str, spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    style = _style_for_profile(profile_name).layers
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    points = _smooth_polyline(rng, spec.width, spec.height * 0.76, 30, 32, phase=0.9)
    _fill_silhouette(draw, points, spec.width, spec.height, style.mid_fill)
    _draw_mid_detail(draw, rng, spec, style)
    return image


def _draw_foreground_motif(draw: ImageDraw.ImageDraw, rng: random.Random, spec: LayerSpec, style: LayerStyle) -> None:
    kind = style.foreground_kind
    if kind in {"branches", "cables"}:
        for i in range(12):
            x = rng.randrange(-30, spec.width + 30)
            y = rng.randrange(int(spec.height * 0.25), spec.height)
            length = rng.randrange(55, 170)
            angle = rng.uniform(-0.45, 0.45)
            x2 = x + int(math.sin(angle) * length)
            y2 = y + int(math.cos(angle) * length)
            width = rng.randrange(3, 8 if kind == "branches" else 6)
            draw.line((x, y, x2, y2), fill=style.near_line, width=width)
            if rng.random() < 0.7:
                r = rng.randrange(3, 9)
                fill = style.near_blob if kind == "branches" else style.accent
                draw.ellipse((x2 - r, y2 - r, x2 + r, y2 + r), fill=fill)
        if kind == "cables":
            for _ in range(4):
                y = rng.randrange(20, 120)
                last = None
                for x in range(-20, spec.width + 40, 40):
                    point = (x, int(y + math.sin(x / 65) * 18))
                    if last is not None:
                        draw.line((*last, *point), fill=style.accent, width=2)
                    last = point
    elif kind == "pillars":
        for _ in range(8):
            x = rng.randrange(-10, spec.width)
            w = rng.randrange(10, 22)
            h = rng.randrange(110, 260)
            y0 = spec.height - h
            draw.rectangle((x, y0, x + w, spec.height), fill=style.near_line)
            if rng.random() < 0.5:
                draw.rectangle((x + 2, y0 + 10, x + w - 2, y0 + 16), fill=style.accent)
    elif kind == "reeds":
        for _ in range(28):
            x = rng.randrange(-10, spec.width + 10)
            y = rng.randrange(int(spec.height * 0.45), spec.height)
            length = rng.randrange(60, 160)
            sway = rng.uniform(-0.35, 0.35)
            x2 = x + int(math.sin(sway) * length)
            y2 = y - int(math.cos(sway) * length)
            draw.line((x, y, x2, y2), fill=style.near_line, width=rng.randrange(2, 5))
    elif kind == "gusts":
        for _ in range(14):
            x = rng.randrange(-40, spec.width)
            y = rng.randrange(10, spec.height - 20)
            w = rng.randrange(60, 150)
            h = rng.randrange(12, 30)
            draw.arc((x, y, x + w, y + h), 0, 180, fill=style.near_blob, width=3)
    elif kind == "spikes":
        for _ in range(16):
            x = rng.randrange(-10, spec.width + 10)
            base_y = rng.choice((0, spec.height))
            size = rng.randrange(24, 90)
            if base_y == 0:
                poly = [(x, 0), (x + size, 0), (x + size // 2, size)]
            else:
                poly = [(x, spec.height), (x + size, spec.height), (x + size // 2, spec.height - size)]
            draw.polygon(poly, fill=style.near_line)
    elif kind == "kelp":
        for _ in range(18):
            x = rng.randrange(-20, spec.width)
            base_y = spec.height + rng.randrange(-10, 10)
            last = (x, base_y)
            for step in range(1, 8):
                point = (x + int(math.sin((step / 2.0) + x * 0.02) * 18), base_y - step * rng.randrange(12, 20))
                draw.line((*last, *point), fill=style.near_line, width=3)
                last = point
    elif kind == "drips":
        for _ in range(18):
            x = rng.randrange(0, spec.width)
            y = rng.randrange(0, 50)
            h = rng.randrange(30, 120)
            draw.line((x, y, x, y + h), fill=style.near_line, width=rng.randrange(3, 6))
        for _ in range(12):
            x0 = rng.randrange(-40, spec.width)
            y0 = rng.randrange(-8, 50)
            x1 = x0 + rng.randrange(60, 200)
            y1 = y0 + rng.randrange(5, 90)
            draw.line((x0, y0, x1, y1), fill=style.near_line, width=rng.randrange(5, 11))


def _render_near(profile_name: str, spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    style = _style_for_profile(profile_name).layers
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    _draw_foreground_motif(draw, rng, spec, style)
    return image.filter(ImageFilter.GaussianBlur(0.2))


def render_layer(profile: BackgroundProfile, spec: LayerSpec) -> Image.Image:
    profile_name = profile.name
    renderers = {
        "sky": _render_sky,
        "far": _render_far,
        "mid": _render_mid,
        "near": _render_near,
    }
    try:
        renderer = renderers[spec.name]
    except KeyError as ex:
        raise KeyError(f"no renderer registered for layer {spec.name!r}") from ex
    return renderer(profile_name, spec)


def render_profile(profile: BackgroundProfile, out_root: Path) -> list[Path]:
    profile_dir = out_root / profile.name
    profile_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for layer in profile.layers:
        image = render_layer(profile, layer)
        path = profile_dir / f"{layer.name}.png"
        image.save(path)
        written.append(path)

    manifest = profile_dir / "manifest.txt"
    manifest.write_text(
        "\n".join(
            [
                "# Generated by tools/ambition_background_renderer.",
                "# Replace these PNGs with hand-painted layers using the same names.",
                *(path.name for path in written),
            ]
        )
        + "\n"
    )
    written.append(manifest)
    return written
