"""Tests for the core render spine primitive (``render_frame``)."""
from __future__ import annotations

from ambition_sprite2d_renderer.core.pipeline import (
    CROP_GROUND,
    CROP_NONE,
    CROP_TIGHT,
    render_frame,
)


def _rect(d, s):
    # A rect that does NOT reach the canvas bottom (so 'ground' vs 'tight' differ).
    d.rectangle((10 * s, 10 * s, 50 * s, 118 * s), fill=(255, 0, 0, 255))


def test_no_crop_returns_full_canvas_at_scale():
    full = render_frame(_rect, (128, 128), scale=1.0, supersample=1, crop=CROP_NONE)
    assert full.size == (128, 128)


def test_scale_shrinks_every_dimension():
    # The fast-test enabler: scale=0.5 yields a half-size frame.
    half = render_frame(_rect, (128, 128), scale=0.5, supersample=1, crop=CROP_NONE)
    assert half.size == (64, 64)


def test_tight_crop_trims_to_artwork():
    tight = render_frame(_rect, (128, 128), scale=1.0, supersample=1, crop=CROP_TIGHT)
    # Cropped well inside the 128x128 canvas around the 40x108 rect (+padding).
    assert tight.width < 128 and tight.height < 128
    assert tight.width >= 40


def test_ground_crop_keeps_bottom_flush():
    tight = render_frame(_rect, (128, 128), scale=1.0, supersample=1, crop=CROP_TIGHT)
    ground = render_frame(_rect, (128, 128), scale=1.0, supersample=1, crop=CROP_GROUND)
    # 'ground' adds no bottom padding, so it is shorter than 'tight' by the pad.
    assert ground.height < tight.height
