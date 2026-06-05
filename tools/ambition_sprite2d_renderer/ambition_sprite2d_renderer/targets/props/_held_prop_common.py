from __future__ import annotations

"""Shared plumbing for the hand-held weapon props (axe / javelin / bow /
arrow).

These targets all share the same authoring contract:

- The art is drawn **axis-aligned with the business end pointing RIGHT
  (+X)**. The game pins the prop to a character's hand at the ``grip``
  anchor and rotates the whole sprite to face the swing / aim / throw
  direction at runtime, so we never bake rotation frames into the
  sheet (same convention the laser-sword family uses — see
  ``lasersword_common.py``).
- Each frame is drawn on a supersampled canvas (``SUPER`` × the final
  size) and LANCZOS-downsampled, which folds the anti-aliasing away
  without changing the output pixel count.
- Per-frame ``anchors`` (``grip`` / ``tip`` / …) are reported in
  **final-frame design pixels**. ``build_sheet``'s auto-crop pass
  translates those anchors by the crop offset for us, so we just
  report them in the pre-crop canvas frame and let the builder keep
  them correct.

This module is intentionally only the *plumbing* — the colour palette,
the supersample canvas helpers, and the anchor-meta builder. The
silhouette of each weapon lives next to that weapon's ``render()`` so
each prop stays legible on its own.
"""

from typing import Dict, Tuple

from PIL import Image

RGBA = Tuple[int, int, int, int]
Point = Tuple[float, float]

# Supersample factor. Drawing happens at SUPER × the final frame size,
# then a LANCZOS downsample folds it away for smooth edges.
SUPER = 4

# ---- Shared palette ---------------------------------------------------------
# Grounded, slightly desaturated material colours so the props read as
# physical objects rather than glowing energy. Each weapon picks the
# subset it needs.
OUTLINE = (26, 24, 28, 255)

WOOD_DARK = (86, 56, 32, 255)
WOOD = (124, 84, 48, 255)
WOOD_HI = (164, 120, 74, 255)

IRON_DARK = (66, 72, 84, 255)
IRON = (108, 116, 130, 255)
IRON_HI = (176, 184, 198, 255)
STEEL_EDGE = (226, 232, 240, 255)

BRASS_DARK = (122, 87, 31, 255)
BRASS = (201, 150, 71, 255)
BRASS_HI = (255, 214, 138, 255)

LEATHER_DARK = (70, 44, 30, 255)
LEATHER = (104, 68, 44, 255)

FLETCH = (196, 64, 56, 255)
FLETCH_HI = (236, 132, 120, 255)


def px(v: float) -> float:
    """Design units → supersample-canvas pixels."""
    return v * SUPER


def new_super(frame_size: Tuple[int, int]) -> Image.Image:
    """Fresh transparent supersample canvas for ``frame_size``."""
    return Image.new(
        "RGBA", (frame_size[0] * SUPER, frame_size[1] * SUPER), (0, 0, 0, 0)
    )


def finalize(canvas: Image.Image, frame_size: Tuple[int, int]) -> Image.Image:
    """LANCZOS-downsample a supersample canvas to the final frame size."""
    return canvas.resize(frame_size, Image.Resampling.LANCZOS)


def anchor_meta(
    anchors: Dict[str, Point],
    *,
    forward: Point = (1.0, 0.0),
    angle_deg: float = 0.0,
) -> dict:
    """Build the per-frame metadata dict ``build_sheet`` merges into the
    YAML/RON rects.

    ``anchors`` maps a name (``grip`` / ``tip`` / …) to a point in
    final-frame design pixels. ``forward`` is the unit vector along the
    weapon's business direction (default +X, since props are authored
    pointing right). ``angle_deg`` is the rendered rotation about the
    grip (0 for the axis-aligned canonical pose).
    """
    return {
        "anchors": {
            name: {"x": round(p[0], 2), "y": round(p[1], 2)}
            for name, p in anchors.items()
        },
        "forward": {"x": round(forward[0], 4), "y": round(forward[1], 4)},
        "angle_deg": round(angle_deg, 2),
    }
