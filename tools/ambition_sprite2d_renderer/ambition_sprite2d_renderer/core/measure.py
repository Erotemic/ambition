"""Measure-by-default: derive body geometry from a rendered frame's pixels.

The heart of the metadata model — the renderer measures what it can from the
art instead of hand-authoring it. This is the one canonical body/feet
measurement; the two spines (``sheet.py``, ``tackon_sheet.py``) had divergent
copies (one used an inclusive last-opaque row for the feet, the other one-past).
We standardise on the **inclusive last opaque row**, the same "lowest opaque
pixel" rule the grounded-door fix uses, so feet planting is consistent.

Bevy anchor convention: ``(0, 0)`` = sprite centre, ``+0.5`` y = top edge. The
runtime uses ``feet_anchor_norm.y`` directly as ``feet_anchor_y`` so it never
hand-tunes feet per target.

Pillow + stdlib only — takes any object with ``.getbbox()`` and ``.size`` (a
``PIL.Image``), so this module imports nothing.
"""
from __future__ import annotations

from typing import Any, Dict, Optional


def measure_body_metrics(frame) -> Optional[Dict[str, Any]]:
    """Body bbox + feet anchor from ``frame``'s opaque pixels.

    Returns ``None`` for a fully transparent frame (callers decide the
    degenerate fallback). Byte-for-byte the legacy ``sheet._measure_body_extent``.
    """
    bbox = frame.getbbox()
    if bbox is None:
        return None
    fw, fh = frame.size
    x_min, y_min, x_max, y_max = bbox
    # `getbbox` is half-open on the high side; subtract 1 for the inclusive
    # last opaque row so the feet anchor sits on the last drawn pixel.
    feet_y = y_max - 1
    feet_x = (x_min + x_max - 1) / 2.0
    return {
        "frame_width": fw,
        "frame_height": fh,
        "body_pixel_bbox": {
            "x": int(x_min),
            "y": int(y_min),
            "w": int(x_max - x_min),
            "h": int(y_max - y_min),
        },
        "feet_pixel": {"x": float(feet_x), "y": float(feet_y)},
        # Image-y grows downward; image_y=feet_y → anchor_y = 0.5 - feet_y/fh.
        "feet_anchor_norm": {
            "x": float(feet_x / fw - 0.5),
            "y": float(0.5 - feet_y / fh),
        },
    }
