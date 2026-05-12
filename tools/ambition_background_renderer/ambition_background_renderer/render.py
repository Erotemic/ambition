from __future__ import annotations

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

from .profiles import BackgroundProfile, LayerSpec


Color = tuple[int, int, int, int]


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


def _draw_stars(draw: ImageDraw.ImageDraw, rng: random.Random, width: int, height: int, count: int) -> None:
    for _ in range(count):
        x = rng.randrange(0, width)
        y = rng.randrange(0, int(height * 0.72))
        radius = rng.choice((1, 1, 1, 2))
        alpha = rng.randrange(72, 170)
        color = (190, 220, 255, alpha)
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=color)


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


def _render_sky(spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    image = _vertical_gradient(
        (spec.width, spec.height),
        top=(7, 10, 27, 255),
        bottom=(30, 38, 72, 255),
    )
    draw = ImageDraw.Draw(image, "RGBA")
    _draw_stars(draw, rng, spec.width, spec.height, 170)
    # A soft moon / glow anchor. It is intentionally low contrast so gameplay
    # sprites and tiles remain readable.
    glow = Image.new("RGBA", image.size, (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow, "RGBA")
    cx, cy = int(spec.width * 0.74), int(spec.height * 0.20)
    for radius, alpha in ((88, 18), (56, 26), (26, 68)):
        gdraw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=(142, 180, 255, alpha))
    glow = glow.filter(ImageFilter.GaussianBlur(8))
    return Image.alpha_composite(image, glow)


def _render_far(spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    # Distant mountain / skyline silhouettes, painted with low alpha.
    for band, (base, amp, color) in enumerate(
        (
            (spec.height * 0.61, 42, (28, 42, 79, 120)),
            (spec.height * 0.70, 36, (18, 30, 61, 145)),
        )
    ):
        points = _smooth_polyline(rng, spec.width, base, amp, 42, phase=band * 1.7)
        _fill_silhouette(draw, points, spec.width, spec.height, color)
    return image.filter(ImageFilter.GaussianBlur(0.4))


def _render_mid(spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    # Chunkier lab/ruin silhouettes with a few vertical structures.
    points = _smooth_polyline(rng, spec.width, spec.height * 0.76, 30, 32, phase=0.9)
    _fill_silhouette(draw, points, spec.width, spec.height, (20, 30, 58, 165))
    for i in range(11):
        x = int((i / 11.0) * spec.width + rng.uniform(-12, 12))
        w = rng.randrange(10, 26)
        h = rng.randrange(55, 165)
        y0 = int(spec.height * 0.76) - h
        draw.rounded_rectangle((x, y0, x + w, spec.height), radius=3, fill=(23, 36, 70, 132))
        if rng.random() < 0.55:
            draw.rectangle((x + 3, y0 + 8, x + w - 3, y0 + 12), fill=(88, 122, 170, 36))
    return image


def _render_near(spec: LayerSpec) -> Image.Image:
    rng = random.Random(spec.seed)
    image = Image.new("RGBA", (spec.width, spec.height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image, "RGBA")
    # Sparse foreground silhouettes. Keep alpha modest to avoid hiding gameplay.
    for i in range(12):
        x = rng.randrange(-30, spec.width + 30)
        y = rng.randrange(int(spec.height * 0.25), spec.height)
        length = rng.randrange(55, 170)
        angle = rng.uniform(-0.45, 0.45)
        x2 = x + int(math.sin(angle) * length)
        y2 = y + int(math.cos(angle) * length)
        draw.line((x, y, x2, y2), fill=(8, 14, 28, 72), width=rng.randrange(3, 8))
        # Small leaf/cable nub.
        if rng.random() < 0.7:
            r = rng.randrange(3, 9)
            draw.ellipse((x2 - r, y2 - r, x2 + r, y2 + r), fill=(10, 22, 38, 54))
    # Top vignette branch shapes.
    for _ in range(8):
        x0 = rng.randrange(-40, spec.width)
        y0 = rng.randrange(-8, 50)
        x1 = x0 + rng.randrange(60, 200)
        y1 = y0 + rng.randrange(5, 90)
        draw.line((x0, y0, x1, y1), fill=(6, 10, 22, 64), width=rng.randrange(5, 11))
    return image.filter(ImageFilter.GaussianBlur(0.2))


_RENDERERS = {
    "sky": _render_sky,
    "far": _render_far,
    "mid": _render_mid,
    "near": _render_near,
}


def render_layer(spec: LayerSpec) -> Image.Image:
    try:
        renderer = _RENDERERS[spec.name]
    except KeyError as ex:
        raise KeyError(f"no renderer registered for layer {spec.name!r}") from ex
    return renderer(spec)


def render_profile(profile: BackgroundProfile, out_root: Path) -> list[Path]:
    profile_dir = out_root / profile.name
    profile_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for layer in profile.layers:
        image = render_layer(layer)
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
