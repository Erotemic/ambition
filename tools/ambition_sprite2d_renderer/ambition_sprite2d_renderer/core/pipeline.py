"""The render spine's per-frame primitive: :func:`render_frame`.

Every drawer-style sprite is produced the same way — rasterize a ``draw(d, s)``
callable onto a supersampled canvas, downsample, then crop. That logic was
copied across ``entities.py``, ``tackon_sheet.py``, ``sheet.py`` and others;
this is the one canonical copy.

The ``scale`` parameter shrinks every dimension proportionally (``scale=0.25``
→ a 32×32 thumbnail of a 128×128 sprite). Because the working scale ``s`` passed
to the drawer is ``scale * supersample`` and drawers multiply their coordinates
by ``s``, geometry tracks ``scale`` automatically — which is what makes
millisecond render tests possible (GOALS.md #1/#3).

Pillow + stdlib only.
"""
from __future__ import annotations

from typing import Callable, Tuple

from PIL import Image, ImageDraw

from .draw import downsample

DrawFn = Callable[["ImageDraw.ImageDraw", float], None]
Size = Tuple[int, int]

# Crop modes. ``tight`` crops to the alpha bbox + padding (most sprites);
# ``ground`` is ``tight`` but keeps the bottom edge flush so the lowest opaque
# row is the sprite's feet (doors / standing props); ``none`` returns the full
# canvas (tiles that must repeat seamlessly).
CROP_TIGHT = "tight"
CROP_GROUND = "ground"
CROP_NONE = "none"


def render_frame(
    draw: DrawFn,
    base_size: Size = (128, 128),
    *,
    scale: float = 1.0,
    supersample: int = 4,
    crop: str = CROP_TIGHT,
    crop_padding: int = 4,
) -> Image.Image:
    """Rasterize ``draw`` at ``base_size * scale`` (supersampled), then crop.

    At ``scale=1.0, supersample=4, crop_padding=4`` this is byte-for-byte the
    legacy ``_render_supersampled`` so existing sprites are unchanged.
    """
    bw, bh = base_size
    ss = max(1, int(supersample))
    s = scale * ss
    work = (max(1, round(bw * s)), max(1, round(bh * s)))
    img = Image.new("RGBA", work, (0, 0, 0, 0))
    draw(ImageDraw.Draw(img), float(s))
    out_size = (max(1, round(bw * scale)), max(1, round(bh * scale)))
    img = downsample(img, out_size)
    if crop == CROP_NONE:
        return img
    bbox = img.getchannel("A").getbbox()
    if bbox is None:
        return img
    pad = max(0, int(crop_padding))
    left, top, right, bottom = bbox
    left = max(0, left - pad)
    top = max(0, top - pad)
    right = min(out_size[0], right + pad)
    # Grounded sprites keep the bottom flush (no padding) so the texture's
    # bottom row == the feet; the runtime plants that edge on the floor.
    bottom = bottom if crop == CROP_GROUND else min(out_size[1], bottom + pad)
    return img.crop((left, top, right, bottom))
