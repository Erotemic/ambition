"""The sky of the two-state room look, as parallax layers.

The `clean_corrupted` room look has one sky in two states. Both are authored
here, as the four layers each parallax theme has:

    hub_clean     a pale city of towers, floating islands and a viaduct, on
                  drawing paper, with gold construction lines
    hub_corrupt   the same city rebuilt in blocks, in violet air, with soft
                  beams of light

The two themes are drawn from one layout (one seed), so a tower of the clean
sky is the same tower in the corrupted one. The game draws `hub_clean` with
its parallax system and lays `hub_corrupt` over it where the air of the room
is corrupted (`room_sky.wgsl`). No shader draws this art.

The sky is behind the play, and it must read so. The play is sharp, has the
darkest values and the bright lines. So each layer here is out of focus (its
blur is a number of this file: `BLUR`), has mid values and low contrast, and
no hard bright line. A farther layer has more blur.

All sizes are in px of the 768 px panel. The game stretches a panel over a
little more than the view, so the middle band of the panel is what is seen.
"""

from __future__ import annotations

import math
import random
from typing import Dict, Tuple

from PIL import Image, ImageDraw, ImageFilter

SIZE = 768
RGB = Tuple[int, int, int]

THEME_KEYS = ("hub_clean", "hub_corrupt")

# How far out of focus each layer is: the radius of its blur, in panel px. A
# farther layer has more. To tune the look, change these and regenerate
# (`scripts/regen/backgrounds.sh`).
BLUR: Dict[str, float] = {
    "far_backplate": 1.6,
    "near_background": 0.9,
    "foreground_atmosphere": 3.0,
}

# The edge of a block of the corrupted sky, in panel px, for each layer.
BLOCK: Dict[str, int] = {
    "far_backplate": 12,
    "near_background": 24,
}


def _mix(a: RGB, b: RGB, t: float) -> RGB:
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))  # type: ignore[return-value]


def _rng(*parts: str) -> random.Random:
    # One layout for the two states: the seed does not name the state.
    return random.Random("room_look_sky/" + "/".join(parts))


def _gradient(top: RGB, bottom: RGB) -> Image.Image:
    image = Image.new("RGBA", (SIZE, SIZE))
    draw = ImageDraw.Draw(image)
    for y in range(SIZE):
        draw.line([(0, y), (SIZE, y)], fill=(*_mix(top, bottom, y / (SIZE - 1)), 255))
    return image


def _tower(draw: ImageDraw.ImageDraw, cx: float, half_w: float, top: float, spire: float, windows: bool) -> None:
    """One tower in a mask: a body to the floor, a cap, a spire, arched windows."""
    draw.rectangle([cx - half_w, top, cx + half_w, SIZE], fill=255)
    draw.rectangle([cx - half_w - 5, top - 9, cx + half_w + 5, top], fill=255)
    draw.polygon([(cx - half_w + 3, top - 9), (cx + half_w - 3, top - 9), (cx, top - 9 - spire)], fill=255)
    if windows and half_w >= 12:
        r = half_w * 0.34
        y = top + 34
        while y < SIZE:
            draw.rectangle([cx - r, y + r, cx + r, y + 36], fill=0)
            draw.ellipse([cx - r, y, cx + r, y + 2 * r], fill=0)
            y += 118


def _towers_mask(layer: str) -> Image.Image:
    """The towers of one layer. The far layer is a city of many thin ones."""
    rng = _rng(layer, "towers")
    mask = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(mask)
    if layer == "far_backplate":
        period, w_lo, w_hi, top_lo, top_hi = 52, 9, 17, 250, 470
    else:
        period, w_lo, w_hi, top_lo, top_hi = 172, 22, 40, 170, 430
    x = rng.uniform(-period * 0.5, 0.0)
    while x < SIZE + period:
        half_w = rng.uniform(w_lo, w_hi)
        top = rng.uniform(top_lo, top_hi)
        _tower(draw, x + rng.uniform(-0.2, 0.2) * period, half_w, top, rng.uniform(28, 70), layer != "far_backplate")
        x += period * rng.uniform(0.8, 1.25)
    return mask


def _viaduct_mask() -> Image.Image:
    """A viaduct of round arches on piers, across the panel."""
    mask = Image.new("L", (SIZE, SIZE), 0)
    draw = ImageDraw.Draw(mask)
    deck, foot, span = 452, 640, 128
    draw.rectangle([0, deck, SIZE, foot], fill=255)
    # A groove in the deck.
    draw.rectangle([0, deck + 6, SIZE, deck + 8], fill=0)
    radius = span * 0.5 - 14
    spring = deck + 20 + radius
    x = -span * 0.35
    while x < SIZE + span:
        draw.rectangle([x - radius, spring, x + radius, foot], fill=0)
        draw.ellipse([x - radius, spring - radius, x + radius, spring + radius], fill=0)
        x += span
    return mask


def _islands(layer: str) -> list[tuple[float, float, float, float]]:
    """Floating islands: `(cx, top, half_w, depth)`."""
    rng = _rng(layer, "islands")
    return [
        (rng.uniform(60, SIZE - 60), rng.uniform(190, 400), rng.uniform(22, 54), rng.uniform(34, 70))
        for _ in range(5)
    ]


def _island_polygon(cx: float, top: float, half_w: float, depth: float) -> list[tuple[float, float]]:
    """A flat top and steps that go in below it."""
    left, right = [], []
    steps = max(2, int(depth // 10))
    for i in range(steps + 1):
        k = i / steps
        w = half_w * (1.0 - k) * (1.0 - 0.25 * k)
        y0, y1 = top + depth * k, top + depth * min(1.0, k + 1.0 / steps)
        left += [(cx - w, y0), (cx - w, y1)]
        right += [(cx + w, y0), (cx + w, y1)]
    return left + right[::-1]


def _colourise(mask: Image.Image, colour: RGB, alpha: int) -> Image.Image:
    layer = Image.new("RGBA", (SIZE, SIZE), (*colour, 0))
    layer.putalpha(mask.point(lambda v: v * alpha // 255))
    return layer


def _blocks(mask: Image.Image, block: int, base: RGB, alpha: int, layer: str) -> Image.Image:
    """`mask` rebuilt in blocks: each block is there or not, has its own shade,
    and some are missing. A block of the sky has no lit edge."""
    rng = _rng(layer, "blocks")
    cells = SIZE // block
    small = mask.resize((cells, cells), Image.BOX)
    out = Image.new("RGBA", (cells, cells), (0, 0, 0, 0))
    src, dst = small.load(), out.load()
    for y in range(cells):
        for x in range(cells):
            shade = rng.uniform(0.86, 1.08)
            missing = rng.random() < 0.12
            if src[x, y] > 128 and not missing:
                dst[x, y] = (*(min(255, int(c * shade)) for c in base), alpha)
    return out.resize((SIZE, SIZE), Image.NEAREST)


def _clean_sky() -> Image.Image:
    image = _gradient((248, 243, 231), (221, 228, 239))
    # Drawing paper: a faint grid, soft.
    grid = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(grid)
    for v in range(0, SIZE, 48):
        draw.line([(v, 0), (v, SIZE)], fill=(150, 150, 170, 22), width=2)
        draw.line([(0, v), (SIZE, v)], fill=(150, 150, 170, 22), width=2)
    image.alpha_composite(grid.filter(ImageFilter.GaussianBlur(1.2)))
    # Light from the upper left, in wide soft shafts.
    shafts = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(shafts)
    for x in range(-SIZE, SIZE, 250):
        draw.polygon([(x, 0), (x + 70, 0), (x + 70 + SIZE * 0.7, SIZE), (x + SIZE * 0.7, SIZE)], fill=(255, 250, 232, 34))
    image.alpha_composite(shafts.filter(ImageFilter.GaussianBlur(26)))
    return image


def _corrupt_sky() -> Image.Image:
    image = _gradient((40, 23, 72), (100, 48, 134))
    # The lattice the world is written on.
    grid = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(grid)
    for v in range(0, SIZE, 48):
        draw.line([(v, 0), (v, SIZE)], fill=(90, 150, 190, 20), width=2)
        draw.line([(0, v), (SIZE, v)], fill=(90, 150, 190, 20), width=2)
    image.alpha_composite(grid.filter(ImageFilter.GaussianBlur(1.2)))
    # Haze, lighter low in the room.
    haze = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(haze)
    rng = _rng("corrupt_sky", "haze")
    for _ in range(9):
        x, y, r = rng.uniform(0, SIZE), rng.uniform(260, SIZE), rng.uniform(90, 200)
        draw.ellipse([x - r, y - r * 0.5, x + r, y + r * 0.5], fill=(150, 80, 190, 30))
    image.alpha_composite(haze.filter(ImageFilter.GaussianBlur(40)))
    return image


def _construction_lines(colour: RGB, alpha: int) -> Image.Image:
    """Rings and axes: gold lines in the clean sky, a lit glyph in the other."""
    rng = _rng("atmosphere", "lines")
    layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    for _ in range(2):
        cx, cy, r = rng.uniform(80, SIZE - 80), rng.uniform(200, 520), rng.uniform(70, 120)
        fill = (*colour, alpha)
        for k in (1.0, 0.62, 0.14):
            draw.ellipse([cx - r * k, cy - r * k, cx + r * k, cy + r * k], outline=fill, width=2)
        draw.line([(cx - r * 1.45, cy), (cx + r * 1.45, cy)], fill=fill, width=2)
        draw.line([(cx, cy - r * 1.45), (cx, cy + r * 1.45)], fill=fill, width=2)
        d = r * 0.62
        draw.polygon([(cx, cy - d), (cx + d, cy), (cx, cy + d), (cx - d, cy)], outline=fill)
    return layer


def _clean_atmosphere() -> Image.Image:
    rng = _rng("atmosphere", "mist")
    layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    for _ in range(10):
        x, y = rng.uniform(-60, SIZE), rng.uniform(180, 600)
        w, h = rng.uniform(160, 340), rng.uniform(22, 54)
        draw.ellipse([x, y, x + w, y + h], fill=(250, 247, 240, rng.randint(40, 80)))
    layer = layer.filter(ImageFilter.GaussianBlur(14))
    layer.alpha_composite(_construction_lines((204, 176, 107), 84))
    # Gold dust in the light.
    draw = ImageDraw.Draw(layer)
    for _ in range(40):
        x, y, r = rng.uniform(0, SIZE), rng.uniform(120, 660), rng.uniform(1.5, 3.0)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(237, 204, 120, rng.randint(70, 150)))
    return layer.filter(ImageFilter.GaussianBlur(BLUR["foreground_atmosphere"] * 0.5))


def _corrupt_atmosphere() -> Image.Image:
    rng = _rng("atmosphere", "mist")
    layer = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(layer)
    for _ in range(10):
        x, y = rng.uniform(-60, SIZE), rng.uniform(180, 600)
        w, h = rng.uniform(160, 340), rng.uniform(22, 54)
        draw.ellipse([x, y, x + w, y + h], fill=(150, 84, 196, rng.randint(30, 60)))
    layer = layer.filter(ImageFilter.GaussianBlur(14))
    # Light leaks: soft vertical beams.
    beams = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(beams)
    beam_rng = _rng("atmosphere", "beams")
    for _ in range(4):
        x = beam_rng.uniform(40, SIZE - 40)
        tone = (255, 70, 180) if beam_rng.random() < 0.7 else (70, 210, 255)
        draw.rectangle([x - 2, 0, x + 2, SIZE], fill=(*tone, 110))
    layer.alpha_composite(beams.filter(ImageFilter.GaussianBlur(5)))
    layer.alpha_composite(_construction_lines((214, 92, 204), 70))
    # Motes that rise.
    draw = ImageDraw.Draw(layer)
    for _ in range(40):
        x, y, r = rng.uniform(0, SIZE), rng.uniform(120, 660), rng.uniform(1.5, 3.0)
        tone = (255, 70, 180) if rng.random() < 0.5 else (70, 210, 255)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(*tone, rng.randint(60, 130)))
    return layer.filter(ImageFilter.GaussianBlur(BLUR["foreground_atmosphere"] * 0.5))


def silhouette(layer: str) -> Image.Image:
    """What is built in one layer of the sky, as a mask. One for both states."""
    mask = _towers_mask(layer)
    if layer == "near_background":
        draw = ImageDraw.Draw(mask)
        for island in _islands(layer):
            draw.polygon(_island_polygon(*island), fill=255)
        mask.paste(255, mask=_viaduct_mask())
    return mask


def render_look_layer(theme_key: str, layer_key: str) -> Image.Image:
    """One layer of one state of the sky, as an RGBA panel."""
    if theme_key not in THEME_KEYS:
        raise ValueError(f"not a room look sky: {theme_key}")
    clean = theme_key == "hub_clean"
    if layer_key == "sky":
        return _clean_sky() if clean else _corrupt_sky()
    if layer_key == "foreground_atmosphere":
        return _clean_atmosphere() if clean else _corrupt_atmosphere()
    mask = silhouette(layer_key)
    far = layer_key == "far_backplate"
    if clean:
        colour = (198, 208, 228) if far else (172, 186, 212)
        image = _colourise(mask, colour, 215 if far else 235)
    else:
        colour = (66, 40, 110) if far else (52, 31, 92)
        image = _blocks(mask, BLOCK[layer_key], colour, 200 if far else 225, layer_key)
    return image.filter(ImageFilter.GaussianBlur(BLUR[layer_key]))


if __name__ == "__main__":
    # A look at the two states, one beside the other, each layer over the last.
    import sys

    sheet = Image.new("RGBA", (SIZE * 2, SIZE))
    for index, theme in enumerate(THEME_KEYS):
        panel = render_look_layer(theme, "sky")
        for layer in ("far_backplate", "near_background", "foreground_atmosphere"):
            panel.alpha_composite(render_look_layer(theme, layer))
        sheet.paste(panel, (index * SIZE, 0))
    out = sys.argv[1] if len(sys.argv) > 1 else "room_look_sky_preview.png"
    sheet.save(out)
    print(out)
