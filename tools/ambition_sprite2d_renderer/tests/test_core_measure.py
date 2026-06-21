"""Tests for the canonical body/feet measurement."""
from __future__ import annotations

from PIL import Image

from ambition_sprite2d_renderer.core.measure import measure_body_metrics


def test_transparent_frame_returns_none():
    assert measure_body_metrics(Image.new("RGBA", (64, 64), (0, 0, 0, 0))) is None


def test_feet_is_inclusive_last_opaque_row():
    # Opaque block rows [10, 40) → last opaque row is 39 (inclusive), not 40.
    img = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
    for y in range(10, 40):
        for x in range(20, 44):
            img.putpixel((x, y), (255, 0, 0, 255))
    m = measure_body_metrics(img)
    assert m["body_pixel_bbox"] == {"x": 20, "y": 10, "w": 24, "h": 30}
    assert m["feet_pixel"]["y"] == 39  # inclusive, matches the door "lowest pixel"
    # centre x of [20,44): (20 + 44 - 1) / 2 = 31.5
    assert m["feet_pixel"]["x"] == 31.5
    # anchor: 0.5 - 39/64
    assert abs(m["feet_anchor_norm"]["y"] - (0.5 - 39 / 64)) < 1e-9
