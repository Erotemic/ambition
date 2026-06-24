"""Tests for the adapter-authored body-box inset.

The robot adapter shrinks the measured body to half its width (25% off each
side) and 80% of its height (20% off the top), so every robot-adapter character
— the player and the robot enemies/variants — gets a tighter gameplay body than
its full rendered silhouette.
"""
from __future__ import annotations

from ambition_sprite2d_renderer.authoring.adapters import (
    BaseAdapter,
    RobotAdapter,
    get_adapter,
)
from ambition_sprite2d_renderer.authoring.sheet import _apply_body_inset


def test_base_adapter_has_no_inset():
    # Non-robot adapters keep the raw measured alpha box.
    assert BaseAdapter().body_inset() is None


def test_robot_adapter_inset_is_half_width_top_trim():
    assert RobotAdapter().body_inset() == {
        "left": 0.25,
        "right": 0.25,
        "top": 0.20,
        "bottom": 0.0,
    }
    # The shared registry instance the renderer actually uses.
    assert get_adapter("robot").body_inset() == RobotAdapter().body_inset()


def test_apply_inset_halves_width_and_trims_top():
    # The player_robot idle box measured today.
    box = {"x": 73, "y": 38, "w": 117, "h": 165}
    out = _apply_body_inset(box, RobotAdapter().body_inset())
    assert out == {"x": 102, "y": 71, "w": 58, "h": 132}
    # Width is half, height is 80%; the bottom edge is unchanged (no feet drift).
    assert out["w"] == round(box["w"] * 0.5)
    assert out["h"] == round(box["h"] * 0.8)
    assert out["y"] + out["h"] == box["y"] + box["h"]


def test_apply_inset_never_degenerate():
    # A pathological tiny box still yields a positive dimension.
    out = _apply_body_inset({"x": 0, "y": 0, "w": 1, "h": 1}, {"left": 0.9, "right": 0.9})
    assert out["w"] >= 1 and out["h"] >= 1


def test_empty_inset_is_identity():
    box = {"x": 5, "y": 6, "w": 40, "h": 50}
    assert _apply_body_inset(box, {}) == box
