"""Shared PIL drawing helpers — the one home for primitives that were
reimplemented ~21× across targets (``rgba`` alone had 13 copies, all
behaviourally identical).

Pillow + stdlib only (see :mod:`ambition_sprite2d_renderer.core`). Each helper
here is the canonical version of a primitive that already existed, byte-for-byte
identical to the majority implementation, so targets can adopt it with no pixel
change (verified by the parity harness as each file is migrated).

Coordinate/colour conventions match the existing drawers: colours are RGBA
4-tuples, ``bbox`` is centre-based, geometry is authored in logical pixels and
multiplied by the working scale ``s`` at paint time.
"""
from __future__ import annotations

from typing import Iterable, List, Tuple

from PIL import Image, ImageColor, ImageDraw, ImageFont

Color = Tuple[int, int, int, int]
Point = Tuple[float, float]
Box = Tuple[float, float, float, float]

try:  # Pillow ≥ 9.1
    RESAMPLING = Image.Resampling
except AttributeError:  # pragma: no cover
    RESAMPLING = Image


def rgba(hex_color: str, alpha: int = 255) -> Color:
    """``"#RRGGBB"`` (or any PIL colour string) → an RGBA 4-tuple."""
    r, g, b = ImageColor.getrgb(hex_color)
    return (r, g, b, alpha)


def with_alpha(color: Color, alpha: int) -> Color:
    """Replace the alpha channel of an RGB/RGBA tuple."""
    return (color[0], color[1], color[2], alpha)


def bbox(cx: float, cy: float, w: float, h: float) -> Box:
    """Centre-based bounding box: ``(left, top, right, bottom)``."""
    return (cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0)


def bbox_from_center(center: Point, w: float, h: float) -> Box:
    """``bbox`` taking a ``(cx, cy)`` tuple (the ``_bbox(center, w, h)`` form)."""
    return bbox(center[0], center[1], w, h)


def poly_scaled(points: Iterable[Point], s: float) -> List[Point]:
    """Scale a list of logical points by the working scale ``s``."""
    return [(x * s, y * s) for x, y in points]


def downsample(img: Image.Image, size: Tuple[int, int]) -> Image.Image:
    """LANCZOS resize — the supersample → final-resolution step."""
    return img.resize(size, RESAMPLING.LANCZOS)


def font(size: int):
    """A bold DejaVu TrueType font at ``size`` px, falling back gracefully."""
    for name in ("DejaVuSans-Bold.ttf", "DejaVuSans.ttf"):
        try:
            return ImageFont.truetype(name, size=max(8, int(size)))
        except OSError:
            pass
    return ImageFont.load_default()


def overlay_draw(img: Image.Image):
    """Return ``(scratch_layer, ImageDraw)`` for safe translucent compositing.

    THE alpha-clobber guard. Drawing a translucent fill straight onto an RGBA
    image with ``ImageDraw.Draw(img)`` *replaces* the destination pixel —
    including its alpha — instead of blending over it, so anything already drawn
    underneath is clobbered. The fix (the "gnu_ton rule") is to paint the
    translucent shapes onto a fresh transparent layer and then blend::

        layer, d = overlay_draw(img)
        d.polygon(pts, fill=(200, 40, 40, 90))   # translucent
        img.alpha_composite(layer)               # real blend, no clobber

    This is the one canonical home for the scratch-layer pattern that was
    re-implemented in ``skeleton.composite_polygon``,
    ``generic_explosions._composite_polygon``, and the rigdoc painter. Those
    have subtly different scratch modes (plain vs ``"RGBA"``), so unifying them
    onto this is a deliberate, parity-checked change — not a blind swap.

    The scratch uses ``"RGBA"`` blend mode so overlapping translucent shapes
    drawn *within* one overlay also composite correctly (not just the final
    blend onto ``img``).
    """
    layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
    return layer, ImageDraw.Draw(layer, "RGBA")


def composite_polygon(img, pts, fill, outline=None, width=0):
    """Translucent polygon via real alpha compositing (see :func:`overlay_draw`)."""
    layer, d = overlay_draw(img)
    d.polygon(list(pts), fill=fill, outline=outline)
    if outline is not None and width and len(pts) > 1:
        d.line(list(pts) + [pts[0]], fill=outline, width=int(width), joint="curve")
    img.alpha_composite(layer)
