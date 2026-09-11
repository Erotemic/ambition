"""The strike-area census must refuse an empty recording rather than report 0.0.

⛔⛔ **THE FAILURE THIS PINS IS "EVERY FIGHTER SWINGS AT NOTHING".** The statistic
is a ratio with a hitbox on top, so a take with no hitboxes — a recording that
never reached the move, a filter that kept no subject body — gives every character
`0.00` and prints a table that reads as a devastating finding about the roster
rather than as an empty file.

⚠ The arms below BREAK the input rather than read the branch. A guard that is only
read has not been shown to fire.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import measure_strike_area_over_body as census  # noqa: E402


def _take(frames):
    return {"takes": [{"character": "someone", "frames": frames}]}


def _frame(body_half, strike_half, *, subject_owned=True, role="subject"):
    return {
        "bodies": [{"role": role, "half": list(body_half)}],
        "hitboxes": [{"subject_owned": subject_owned, "half": list(strike_half)}],
    }


def test_the_ratio_is_areas_not_widths():
    # 4*2*2 = 16 over 4*4*4 = 64.
    assert census.ratios(_take([_frame((4, 4), (2, 2))])) == {"someone": 0.25}


def test_the_peak_frame_wins_not_the_mean():
    """⚠ A move whose box is live for two frames and enormous reads as small under
    a mean, and what a player feels is whether the box that IS live can reach."""
    rows = census.ratios(
        _take([_frame((4, 4), (1, 1)), _frame((4, 4), (8, 8)), _frame((4, 4), (1, 1))])
    )
    assert rows == {"someone": 4.0}


def test_a_target_owned_box_is_not_counted():
    """⛔ A take records the SANDBAG's boxes too; counting those measures the
    sandbag."""
    rows = census.ratios(_take([_frame((4, 4), (8, 8), subject_owned=False)]))
    assert rows == {"someone": 0.0}


def test_a_character_that_produced_no_box_is_reported_as_zero_not_dropped():
    """⛔ "This verb produced no hitbox" is the louder finding of the two, and a
    dropped row reads as nothing at all."""
    assert census.ratios(_take([])) == {"someone": 0.0}


def test_an_empty_recording_refuses_instead_of_printing_a_table_of_zeroes(tmp_path, monkeypatch):
    path = tmp_path / "takes.json"
    path.write_text(json.dumps({"takes": []}))
    monkeypatch.setattr(sys, "argv", ["census", str(path)])
    with pytest.raises(SystemExit) as exc:
        census.main()
    assert "REFUSING TO REPORT" in str(exc.value)


def test_a_file_with_takes_and_no_hitboxes_at_all_refuses(tmp_path, monkeypatch):
    """⛔⛔ THE HARDER HALF: "no takes" is easy, "takes with no boxes" prints a
    table of 0.00 that reads as a devastating finding about the ROSTER."""
    path = tmp_path / "takes.json"
    path.write_text(json.dumps(_take([])))
    monkeypatch.setattr(sys, "argv", ["census", str(path)])
    with pytest.raises(SystemExit) as exc:
        census.main()
    assert "NOT ONE live subject-owned" in str(exc.value)


def test_one_zero_among_many_is_reported_rather_than_refused(tmp_path, monkeypatch, capsys):
    """⚠ THE OTHER DIRECTION, and it matters: a single fighter whose verb produced
    no box is the loudest row in the table, not a reason to refuse the table."""
    path = tmp_path / "takes.json"
    path.write_text(
        json.dumps(
            {
                "takes": [
                    {"character": "silent", "frames": []},
                    {"character": "swinging", "frames": [_frame((4, 4), (2, 2))]},
                ]
            }
        )
    )
    monkeypatch.setattr(sys, "argv", ["census", str(path)])
    assert census.main() == 0
    out = capsys.readouterr().out
    assert "silent" in out and "0.00" in out


def test_the_extents_reported_come_from_the_peak_frame_not_the_last_one():
    """⛔⛔ THE EXTENTS MUST BE THE PEAK'S OWN. A row quoting "0.80, and it is
    38 x 17" where the 38 x 17 came from a different frame than the 0.80 is two
    measurements wearing one sentence — and that is exactly how a planning row
    got a body height it had never measured.

    The peak here is the FIRST frame and a smaller box follows it, so a reader
    that tracked extents independently of the max would report the second.
    """
    rows = census.peaks(
        _take([_frame((4, 4), (3, 3)), _frame((4, 4), (1, 1))])
    )
    ratio, strike, body = rows["someone"]
    assert ratio == pytest.approx(36 / 64)
    assert strike == (6.0, 6.0), "the extents came from a frame that was not the peak"
    assert body == (8.0, 8.0)


def test_detail_prints_the_extents_and_the_plain_form_does_not(tmp_path, monkeypatch, capsys):
    """`--detail` is what makes a quoted extent derived rather than copied."""
    path = tmp_path / "take.json"
    path.write_text(json.dumps(_take([_frame((4, 4), (2, 2))])))

    monkeypatch.setattr(sys, "argv", ["census", str(path)])
    assert census.main() == 0
    assert "against a" not in capsys.readouterr().out

    monkeypatch.setattr(sys, "argv", ["census", str(path), "--detail"])
    assert census.main() == 0
    out = capsys.readouterr().out
    assert "4.0 x   4.0 px against a 8 x 8 body" in out, out
