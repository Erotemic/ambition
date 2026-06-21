from __future__ import annotations

"""Small rendering helpers shared by package-level sheet composition."""

from typing import Optional, Sequence, Tuple

from PIL import ImageColor, ImageDraw, ImageFont
from ambition_sprite2d_renderer.core.draw import rgba

Color = Tuple[int, int, int, int]




def parse_color(value: str) -> Color:
    if value.lower() == "transparent":
        return (0, 0, 0, 0)
    return rgba(value)


def rounded_rect(
    draw: ImageDraw.ImageDraw,
    box: Sequence[float],
    radius: float,
    fill: Color,
    outline: Optional[Color] = None,
    width: int = 1,
) -> None:
    draw.rounded_rectangle(box, radius=radius, fill=fill, outline=outline, width=width)


def load_font(size: int):
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=max(8, int(size)))
        except OSError:
            pass
    return ImageFont.load_default()
