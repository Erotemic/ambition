"""The move-clock census must refuse a bundle it could not classify.

⛔⛤ **THE FAILURE THIS PINS IS A TIDY TABLE OF DASHES.** The census groups moves
by the VERB they answer to, so a bundle whose vocabulary moved — a renamed verb, a
schema bump, a filter that kept the wrong fighters — classifies nothing and prints
every median as absent. That reads as "this roster authors no attacks" rather than
as "this script and this bundle no longer speak the same language".

⚠ Each arm BREAKS the input rather than reads the branch.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import measure_move_clock_shape as clock  # noqa: E402


def _move(verb: str, *, active=4.0, startup=5.0, endlag=9.0):
    return {
        "id": f"someone_{verb}",
        "verbs": [verb],
        "duration_f": startup + active + endlag,
        "derived": {"startup_f": startup, "active_f": active, "endlag_f": endlag},
    }


def _bundle(moves, *, on_grid=True):
    return {
        "sim_hz": 60.0,
        "characters": [{"id": "someone", "on_smash_grid": on_grid, "moves": moves}],
    }


def test_moves_are_classified_by_verb_not_by_id():
    """⛔ Ids are per-character (`run_up_kick`, `cipher_sweep`); a census keyed on a
    naming convention measures the convention."""
    grouped = clock.rows(_bundle([_move("attack_forward"), _move("smash_up")]))
    assert [m["id"] for m in grouped["tilt"]] == ["someone_attack_forward"]
    assert [m["id"] for m in grouped["smash"]] == ["someone_smash_up"]


def test_a_move_with_no_live_window_is_left_out():
    """⛔ A hitless special's "startup" is its whole duration; folding those in
    describes the utility moves rather than the attacks."""
    hitless = _move("special")
    hitless["derived"]["active_f"] = 0.0
    assert clock.rows(_bundle([hitless]))["special"] == []


def test_off_grid_fighters_are_excluded_unless_asked_for():
    """⛔ 48 fighters are authored and 21 are seatable; a median over the other 27
    describes movesets nobody fights."""
    b = _bundle([_move("attack_forward")], on_grid=False)
    assert clock.rows(b, grid_only=True)["tilt"] == []
    assert len(clock.rows(b, grid_only=False)["tilt"]) == 1


#: ⛔ THE TWENTY AUTHORED STRIKE VERBS, taken from the census this script exists
#: to corroborate — `measure_authored_strike_extents.py --clock`, which reads the
#: moveset RON directly and reports "343 move(s) with Active windows across 17
#: table(s)". Pinned here as a LIST rather than a count because four of them —
#: `grab`, `grab_dash`, `attack_dash`, `special_air_down` — were absent from
#: CLASSES and `rows` dropped them WITHOUT A WORD, which reads as "this roster
#: authors no grabs" rather than as "this map has no grab row".
AUTHORED_STRIKE_VERBS = (
    "attack",
    "attack_air",
    "attack_air_back",
    "attack_air_down",
    "attack_air_forward",
    "attack_air_up",
    "attack_dash",
    "attack_down",
    "attack_forward",
    "attack_up",
    "grab",
    "grab_dash",
    "smash_down",
    "smash_forward",
    "smash_up",
    "special",
    "special_air_down",
    "special_down",
    "special_forward",
    "special_up",
)


def test_every_authored_strike_verb_reaches_a_class():
    """⛔⛤ THE OMISSION THAT HAS NO SYMPTOM. An unmapped verb is not an error and
    not a dash in the table — the move simply never appears, and the totals stay
    plausible. Four of these were missing at once and the census still printed a
    tidy roster-wide median."""
    unmapped = [v for v in AUTHORED_STRIKE_VERBS if clock.class_of([v]) is None]
    assert unmapped == [], (
        f"{len(unmapped)} authored verb(s) reach no class and are dropped "
        f"silently: {unmapped}"
    )


def test_the_four_verbs_that_were_dropped_now_classify():
    """⭐ The other side of the guard above, by CLASS — covering the verb is not
    the same as putting it in the right row."""
    grouped = clock.rows(
        _bundle(
            [
                _move("grab"),
                _move("grab_dash"),
                _move("attack_dash"),
                _move("special_air_down"),
            ]
        )
    )
    assert [m["id"] for m in grouped["grab"]] == ["someone_grab", "someone_grab_dash"]
    assert [m["id"] for m in grouped["dash"]] == ["someone_attack_dash"]
    assert [m["id"] for m in grouped["special"]] == ["someone_special_air_down"]


def test_nine_point_six_active_frames_is_not_ten(tmp_path, monkeypatch, capsys):
    """⛔⛤ THE TOP BAND IS `>= 10`, NOT `> 9`, AND THE DIFFERENCE IS SIX MOVES.
    This roster authors six moves at exactly 9.6f; counted with `> 9` the census
    prints 24 where the committed figure is 18, and the extra six are simply
    mislabelled. The band edge is load-bearing, so it gets a test."""
    moves = [_move("attack_forward", active=9.6) for _ in range(20)]
    moves += [_move("smash_up", active=10.8) for _ in range(5)]
    path = tmp_path / "bundle.json"
    path.write_text(json.dumps(_bundle(moves)))
    monkeypatch.setattr(sys, "argv", ["clock", str(path)])
    assert clock.main() == 0
    out = capsys.readouterr().out
    assert "5 of 25 moves run ACTIVE at 10 frames or more (20%)." in out
    # ⛔ NOT MERELY THE COUNT: a 9.6f move must not be NAMED in the long list
    # either, which is the row a reader actually checks.
    assert "someone_attack_forward" not in out


def test_a_bundle_it_cannot_classify_refuses(tmp_path, monkeypatch):
    path = tmp_path / "bundle.json"
    path.write_text(json.dumps(_bundle([_move("a_verb_this_script_never_heard_of")])))
    monkeypatch.setattr(sys, "argv", ["clock", str(path)])
    with pytest.raises(SystemExit) as exc:
        clock.main()
    assert "REFUSING TO REPORT" in str(exc.value)


def test_a_real_population_reports(tmp_path, monkeypatch, capsys):
    """⭐ THE FLOOR'S OTHER SIDE: a population big enough to mean something prints,
    so the refusal above cannot be satisfied by refusing everything."""
    path = tmp_path / "bundle.json"
    path.write_text(
        json.dumps(_bundle([_move("attack_forward") for _ in range(25)]))
    )
    monkeypatch.setattr(sys, "argv", ["clock", str(path)])
    assert clock.main() == 0
    out = capsys.readouterr().out
    assert "25 moves with a live window" in out
    assert "RESEARCH, NOT MEASURED HERE" in out
