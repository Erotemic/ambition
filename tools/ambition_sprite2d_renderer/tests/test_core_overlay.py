"""Prove the alpha-clobber guard (`overlay_draw`) actually blends, and pin the
naive failure mode so the distinction is documented in a test."""
from __future__ import annotations

from PIL import Image, ImageDraw

from ambition_sprite2d_renderer.core.draw import overlay_draw


def _opaque_base() -> Image.Image:
    base = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    ImageDraw.Draw(base).rectangle((0, 0, 15, 15), fill=(255, 0, 0, 255))
    return base


def test_naive_translucent_draw_clobbers_alpha():
    # The bug: drawing a translucent fill straight onto the RGBA image REPLACES
    # the pixel, dropping the opaque base to the overlay's alpha.
    base = _opaque_base()
    ImageDraw.Draw(base).rectangle((4, 4, 11, 11), fill=(0, 0, 255, 100))
    assert base.getpixel((8, 8)) == (0, 0, 255, 100)  # clobbered (alpha 100)


def test_overlay_draw_blends_without_clobbering():
    base = _opaque_base()
    layer, d = overlay_draw(base)
    d.rectangle((4, 4, 11, 11), fill=(0, 0, 255, 100))
    base.alpha_composite(layer)
    r, g, b, a = base.getpixel((8, 8))
    assert a == 255  # base stayed opaque — no clobber
    assert b > 0 and r < 255  # blue blended over the red
