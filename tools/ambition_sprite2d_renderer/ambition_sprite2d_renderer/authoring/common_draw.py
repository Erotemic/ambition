from __future__ import annotations

import math
from typing import Iterable, Tuple

from PIL import Image, ImageChops, ImageDraw

Point = Tuple[float, float]
Color = Tuple[int, int, int, int]

try:
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image


def scaled_color(color: Color) -> Color:
    return tuple(int(v) for v in color)  # type: ignore[return-value]


def draw_capsule(
    draw: ImageDraw.ImageDraw,
    a: Point,
    b: Point,
    radius: float,
    fill: Color,
    outline: Color,
    outline_w: float = 1.0,
) -> None:
    width_o = max(1, int(round(radius * 2.0 + outline_w * 2.0)))
    width_i = max(1, int(round(radius * 2.0)))
    draw.line([a, b], fill=outline, width=width_o)
    draw.line([a, b], fill=fill, width=width_i)
    for c in (a, b):
        x, y = c
        draw.ellipse(
            (
                x - radius - outline_w,
                y - radius - outline_w,
                x + radius + outline_w,
                y + radius + outline_w,
            ),
            fill=outline,
        )
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=fill)


def draw_rotated_rounded_rect(
    base: Image.Image,
    center: Point,
    size: Point,
    angle: float,
    radius: float,
    fill: Color,
    outline: Color | None = None,
    outline_w: float = 0.0,
) -> None:
    w, h = (
        max(2, int(math.ceil(size[0] + outline_w * 4))),
        max(2, int(math.ceil(size[1] + outline_w * 4))),
    )
    pad = int(max(w, h) * 0.35 + abs(outline_w) + 4)
    layer = Image.new("RGBA", (w + pad * 2, h + pad * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    box = (
        pad + outline_w,
        pad + outline_w,
        pad + outline_w + size[0],
        pad + outline_w + size[1],
    )
    if outline is not None and outline_w > 0:
        obox = (
            box[0] - outline_w,
            box[1] - outline_w,
            box[2] + outline_w,
            box[3] + outline_w,
        )
        d.rounded_rectangle(obox, radius=radius + outline_w, fill=outline)
    d.rounded_rectangle(box, radius=radius, fill=fill)
    layer = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(
        layer, (int(center[0] - layer.size[0] / 2), int(center[1] - layer.size[1] / 2))
    )


def draw_rotated_ellipse(
    base: Image.Image,
    center: Point,
    size: Point,
    angle: float,
    fill: Color,
    outline: Color | None = None,
    outline_w: float = 0.0,
) -> None:
    w, h = (
        max(2, int(math.ceil(size[0] + outline_w * 4))),
        max(2, int(math.ceil(size[1] + outline_w * 4))),
    )
    pad = int(max(w, h) * 0.35 + abs(outline_w) + 4)
    layer = Image.new("RGBA", (w + pad * 2, h + pad * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    box = (
        pad + outline_w,
        pad + outline_w,
        pad + outline_w + size[0],
        pad + outline_w + size[1],
    )
    if outline is not None and outline_w > 0:
        obox = (
            box[0] - outline_w,
            box[1] - outline_w,
            box[2] + outline_w,
            box[3] + outline_w,
        )
        d.ellipse(obox, fill=outline)
    d.ellipse(box, fill=fill)
    layer = layer.rotate(angle, resample=RESAMPLING.BICUBIC, expand=True)
    base.alpha_composite(
        layer, (int(center[0] - layer.size[0] / 2), int(center[1] - layer.size[1] / 2))
    )


def alpha_bbox(img: Image.Image):
    return img.getchannel("A").getbbox()


def force_opaque_inside(layer: Image.Image, mask: Image.Image) -> None:
    # Used by goblin parts: opaque filled silhouettes, semi-transparent details allowed only after compositing.
    layer.putalpha(ImageChops.lighter(layer.getchannel("A"), mask))
