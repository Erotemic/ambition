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


def _v2_frame() -> dict:
    """A take recorded with roles, hurtboxes and a move clock."""
    return {
        "bodies": [
            {
                "pos": [50.0, 100.0],
                "half": [12.0, 20.0],
                "seat": 0,
                "role": "subject",
                "hurtboxes": [{"pos": [50.0, 96.0], "half": [8.0, 16.0], "shape": {"kind": "aabb"}}],
                "hurtbox_source": "published",
                "move_state": {"id": "jab", "phase": "Active", "elapsed_s": 0.1, "duration_s": 0.4},
            },
            {
                "pos": [110.0, 100.0],
                "half": [12.0, 20.0],
                "seat": 1,
                "role": "target",
                "hurtboxes": [],
                "hurtbox_source": "intangible",
            },
        ],
        "hitboxes": [
            {
                "pos": [70.0, 100.0],
                "half": [10.0, 8.0],
                "role": "subject_owned",
                "shape": {"kind": "circle", "center": [70.0, 100.0], "radius": 10.0},
            },
            {
                "pos": [100.0, 100.0],
                "half": [6.0, 6.0],
                "role": "target_owned",
                "shape": {"kind": "aabb"},
            },
        ],
        "projectiles": [],
        "move": "jab",
    }


def _v2_take() -> dict:
    return {
        "character": "npc_pirate_admiral",
        "verb": "attack",
        "subject": "npc_pirate_admiral",
        "target": "npc_sandbag",
        "target_behavior": "passive",
        "view": [0.0, 0.0, 320.0, 240.0],
        "frames": [_v2_frame()],
    }


def test_the_sheet_names_the_subject_and_the_target() -> None:
    """⛔⛔ A SHEET READ WITHOUT ITS CONTEXT MUST STILL SAY WHOSE MOVE IT IS.

    Two fighters in one picture, told apart by a seat index nothing wrote down,
    is a diagnostic that needs a convention to read.
    """
    svg = tool.sheet(_v2_take())
    assert "SUBJECT" in svg and "TARGET" in svg
    assert "vs npc_sandbag" in svg
    assert "(passive)" in svg, "how the target behaved changes what the sheet MEANS"


def test_it_draws_damageable_geometry_beside_the_attack_volumes() -> None:
    """Half the interaction. Attack volumes alone cannot say whether an apparent
    overlap could have connected at all."""
    svg = tool.sheet(_v2_take())
    assert "#49c8d8" in svg, "the hurtbox is missing"
    assert "INTANGIBLE" in svg, (
        "an empty hurtbox list is a DECISION — a body nothing can hit this "
        "frame — and it is invisible unless the sheet says so"
    )


def test_a_strike_is_drawn_in_its_real_shape() -> None:
    """⛔⛔ A DISC IS NOT THE BOX AROUND IT. For a sweeping arc the containing
    rectangle is a great deal larger than the thing that can actually hit you."""
    svg = tool.sheet(_v2_take())
    assert "<circle" in svg, "a circular strike was drawn as a rectangle"


def test_the_targets_own_strike_is_not_drawn_as_the_subjects() -> None:
    svg = tool.sheet(_v2_take())
    assert "#e5554f" in svg, "the subject's strike is missing"
    assert "#8a5f5c" in svg, "the target's strike must be visibly not the subject's"


def test_the_caption_carries_the_authored_window_not_just_the_move() -> None:
    svg = tool.sheet(_v2_take())
    assert "Active" in svg, '"a red box appeared" is not frame data; the phase is'


def test_a_v1_take_still_draws() -> None:
    """⛔ WHAT AN OLD FILE MAY CONTAIN DOES NOT DEFINE WHAT A NEW ONE EMITS —
    but an artifact recorded before roles existed must still render."""
    svg = tool.sheet(_take())
    assert "#7fb2e5" in svg, "a v1 subject (seat 0) is still drawn as the subject"
    assert "#e5554f" in svg, "a v1 subject-owned hitbox is still the subject's"


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


def test_the_key_frame_strip_picks_the_ticks_that_mean_something() -> None:
    """⛔⛔ AN EVEN STRIP SAMPLES THE CLOCK, NOT THE MOVE.

    A jab is live for five of a hundred and fifty ticks, so twelve evenly spaced
    frames usually miss every one of them — and a fighter standing still in every
    cell is the picture that makes somebody conclude the move does nothing.
    """
    frames = []
    for tick in range(60):
        phase = "Startup" if tick < 10 else "Active" if tick < 14 else "Recovery"
        frame = {
            "bodies": [
                {
                    "pos": [50.0, 100.0],
                    "half": [10.0, 20.0],
                    "role": "subject",
                    "hurtboxes": [],
                    "move_state": {"id": "jab", "phase": phase, "elapsed_s": 0.0, "duration_s": 1.0},
                },
                {"pos": [90.0, 100.0], "half": [10.0, 20.0], "role": "target",
                 "hurtboxes": [{"pos": [90.0, 100.0], "half": [8.0, 18.0], "shape": {"kind": "aabb"}}]},
            ],
            "hitboxes": [{"pos": [75.0, 100.0], "half": [12.0, 6.0], "role": "subject_owned",
                          "shape": {"kind": "aabb"}}]
            if 10 <= tick < 14
            else [],
            "projectiles": [],
            "contacts": [{"strike": "s", "owner_role": "subject_owned", "victim": "t",
                          "victim_role": "target"}]
            if tick >= 11
            else [],
        }
        frames.append(frame)
    take = {"character": "x", "verb": "attack", "view": [0.0, 0.0, 320.0, 240.0], "frames": frames}

    picked = tool.key_frames(take, 12)
    labelled = {label: tick for tick, _, label in picked if label}
    assert any("first live volume" in label for label in labelled), labelled
    assert any("FIRST CONTACT" in label for label in labelled), labelled
    # The live window is four ticks out of sixty; an even strip of twelve steps
    # by five and would land in it at most once by luck.
    live = [tick for tick, _, _ in picked if 10 <= tick < 14]
    assert len(live) >= 2, f"the strip must show the move being live: {picked!r}"
    assert picked[0][0] == 0, "the opening pose is always shown"


def test_an_even_strip_is_still_available_for_an_old_take() -> None:
    svg = tool.sheet(_take(), select="even")
    assert svg.startswith("<svg")


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
