"""The strike-area census peaks across a character's WHOLE moveset, not its last take.

⛔⛤ **THIS EXISTS BECAUSE THE CENSUS REPORTED A FIGHTER SWINGING AT NOTHING.**
`peaks()` computed a per-ROW best and then did `out[character] = ...`, so each
take overwrote the previous one and the printed number was the peak inside
whichever take happened to be recorded LAST. With one verb per character — the
way it was first used — those are the same number, which is why it survived.

MEASURED 2026-09-12 on a ten-verb grid take: it printed
`smash_george_booul 0.00, 0.0 x 0.0 px against a 0 x 0 body`, for a fighter whose
F-tilt is a 56 x 36 box over a 34 x 48 body. Its last recorded verb produced no
live box, and that zero became the whole character's answer. The roster-level
verdict moved with the defect: **11 of 21 "under its own body" before, 1 of 21
after.** A table saying half the roster swings at nothing is a devastating
finding, and it was an artifact of verb order.

⇒ A COUNT IS NOT A CHECK ON A LIST, and neither is a ratio. What pins this is a
fixture whose FIRST take is the generous one and whose LAST is empty: under the
defect the character reads 0.00, and only an across-takes peak reads the truth.
"""
from __future__ import annotations

import importlib.util
import pathlib

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
_spec = importlib.util.spec_from_file_location(
    "measure_strike_area_over_body",
    REPO / "scripts" / "measure_strike_area_over_body.py",
)
census = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(census)


def _frame(body_half, hit_half, move="swing"):
    frame = {
        "bodies": [{"role": "subject", "half": list(body_half)}],
        "hitboxes": [],
        "move": move,
    }
    if hit_half is not None:
        frame["hitboxes"].append({"subject_owned": True, "half": list(hit_half)})
    return frame


def _take(rows, intended=None):
    # ⛔⛤ **`intended_move` IS SET DELIBERATELY AND MY FIRST FIXTURE OMITTED IT.**
    # Without it, a poison swapping the per-move key to the take's INTENT changed
    # nothing — `row.get("intended_move")` was `None` and fell through to the
    # frame's own move, so the poison passed and the test certified nothing. A
    # field the subject reads must exist in the fixture, and it must DISAGREE
    # with the right answer, or the disagreement is unobservable.
    return {
        "takes": [
            {"character": c, "frames": f, "intended_move": intended or "the-verb-asked-for"}
            for c, f in rows
        ]
    }


def test_a_later_empty_take_does_not_erase_an_earlier_generous_one():
    # ⭐ THE PREMISE: the generous take comes FIRST and the empty one LAST, which
    # is the only order under which the overwrite is visible. A fixture with the
    # empty take first passes either way and certifies nothing.
    take = _take(
        [
            ("fighter", [_frame((10.0, 24.0), (28.0, 18.0), "tilt_forward")]),
            ("fighter", [_frame((10.0, 24.0), None, "air_down")]),
        ]
    )
    ratio, strike, body = census.peaks(take)["fighter"]
    expected = 4.0 * 28.0 * 18.0 / (4.0 * 10.0 * 24.0)
    assert ratio == expected, f"the last take erased the peak: {ratio} != {expected}"
    assert strike == (56.0, 36.0), strike
    assert body == (20.0, 48.0), body


def test_a_character_whose_every_take_is_empty_is_still_reported_as_zero():
    """⛔ THE CONTROL, and it is the half the fix could have broken. A missing row
    reads as "not measured"; `0.00` reads as "this fighter produced no live box",
    which is the louder of the two findings this file can make."""
    take = _take([("fighter", [_frame((10.0, 24.0), None)])])
    assert census.peaks(take)["fighter"][0] == 0.0
    assert "fighter" in census.ratios(take)


def test_per_move_files_boxes_under_the_move_that_was_playing():
    """⛔ AND THE PER-MOVE TABLE KEYS ON THE ACCEPTED MOVE, NOT THE TAKE'S INTENT.
    One take can fall back to another move, and filing its boxes under the verb
    that was asked for reports a number about a move that never played."""
    take = _take(
        [
            (
                "fighter",
                [
                    _frame((10.0, 24.0), (28.0, 18.0), "tilt_forward"),
                    _frame((10.0, 24.0), (5.0, 5.0), "air_down"),
                ],
            )
        ]
    )
    rows = census.per_move(take)
    assert set(rows) == {("fighter", "tilt_forward"), ("fighter", "air_down")}, (
        "the per-move table is keyed on something other than the move that was "
        f"playing — the take's intent is `the-verb-asked-for`: {sorted(rows)}"
    )
    assert rows[("fighter", "tilt_forward")][0] > rows[("fighter", "air_down")][0]
    # ⭐ AND THE PER-CHARACTER PEAK HIDES THE THIN ONE, which is why both exist.
    assert census.peaks(take)["fighter"][0] == rows[("fighter", "tilt_forward")][0]
