"""A derived picture must never read as an engine render, and must show something.

⛔⛔ THE INSPECTOR IS CAREFUL ABOUT THIS AND AN EXPORTED FILE LEAVES THE CONTEXT
THAT MADE IT OBVIOUS. A sheet on disk has no server beside it saying "sprites:
derived"; the only place that distinction can survive is on the picture.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "render_take_diagnostic", REPO / "scripts/render_take_diagnostic.py"
)
tool = importlib.util.module_from_spec(_spec)
sys.modules["render_take_diagnostic"] = tool
_spec.loader.exec_module(tool)


def _frame(x: float, hitbox: bool = False) -> dict:
    return {
        "bodies": [{"pos": [x, 100.0], "half": [12.0, 20.0], "seat": 0}],
        "hitboxes": [{"pos": [x + 20, 100.0], "half": [10.0, 8.0], "subject_owned": True}]
        if hitbox
        else [],
        "projectiles": [],
        "move": "jab",
        "pose": "Idle",
        "clip": "jab",
    }


def _take(frames: int = 40) -> dict:
    return {
        "character": "npc_pirate_admiral",
        "verb": "attack",
        "intended_move": "jab",
        "reached_intended_move": True,
        "view": [0.0, 0.0, 320.0, 240.0],
        "frames": [_frame(50.0 + i, hitbox=i > 20) for i in range(frames)],
    }


def test_the_sheet_says_it_is_derived() -> None:
    svg = tool.sheet(_take())
    assert tool.WATERMARK in svg
    assert "not an engine render" in svg


def test_it_draws_the_geometry_the_take_recorded() -> None:
    svg = tool.sheet(_take())
    # Bodies and hitboxes are different colours because they answer different
    # questions; a sheet with only one of them is not a diagnostic.
    assert "#7fb2e5" in svg, "the subject's body box is missing"
    assert "#e5554f" in svg, "the subject's hitbox is missing"
    assert "jab" in svg, "the caption must carry the move/pose/clip of the tick"


def test_frames_are_sampled_across_the_take_not_taken_from_the_front() -> None:
    """⛔ THE FIRST TWELVE TICKS OF A 150-TICK TAKE ARE THE WIND-UP.

    A strip of them shows a fighter standing still and says the move does
    nothing, which is the opposite of what the sheet is for.
    """
    picked = tool.sample([_frame(float(i)) for i in range(150)], 12)
    assert len(picked) == 12
    assert picked[0][0] == 0, "the first frame is always shown"
    assert picked[-1][0] > 100, f"the strip never reaches the end of the take: {picked[-1][0]}"


def test_a_short_take_shows_every_frame_rather_than_inventing_some() -> None:
    picked = tool.sample([_frame(float(i)) for i in range(5)], 12)
    assert [i for i, _ in picked] == [0, 1, 2, 3, 4]


def test_an_empty_take_draws_nothing_rather_than_dividing_by_zero() -> None:
    assert tool.sample([], 12) == []
    svg = tool.sheet({"character": "x", "verb": "y", "frames": []})
    assert svg.startswith("<svg"), "a take with no frames still produces a valid file"
