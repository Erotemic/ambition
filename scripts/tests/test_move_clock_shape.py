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
