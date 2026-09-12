"""The clock×generosity join is static, per-clip, and never prints a pooled band alone.

⛔⛤ **THE FAILURE THIS PINS IS A TRUE NUMBER THAT DESCRIBES ONE FIGHTER.** MEASURED
2026-09-12: 7 of the 9 long-active bone-derived moves carry an `inflate`, against
4 of 53 and 8 of 63 in the shorter bands. Read pooled that says the long moves are
the generous ones. All seven are the performer's; the other two long-active moves
carry `inflate=0.0`, and excluding that one fighter the relationship does not
weaken — it inverts to nothing. Three separate claims in this repository have now
been a subpopulation wearing a roster's clothes, so the report REFUSES to print a
band without its per-character split and NAMES any fighter holding a majority.

⚠ Each arm BREAKS the input rather than reads the branch, and every fixture is
synthetic: the sprite stages live in a submodule, so a test that read the real
tree would pass vacuously wherever that submodule is absent — which is exactly
the condition this file is most likely to be run under.
"""
from __future__ import annotations

import importlib.util
import json
import pathlib
import sys

import pytest

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
_loader = importlib.util.spec_from_file_location(
    "measure_hitbox_authoring_coverage",
    REPO / "scripts" / "measure_hitbox_authoring_coverage.py",
)
cov = importlib.util.module_from_spec(_loader)
_loader.loader.exec_module(cov)


def _table(moves: list[tuple[str, str]]) -> str:
    """A moveset RON spelled the way `clip_of_move` reads it."""
    body = "".join(
        f'        id: "{move}",\n'
        f'        display_name: "whatever",\n'
        f"        clip: (\n"
        f'            clip: "{clip}",\n'
        f"        ),\n"
        for move, clip in moves
    )
    return "(\n" + body + ")\n"


def _spec(clip: str, *, extend=1.0, inflate=None, bone=True) -> dict:
    out: dict = {"clip": clip, "hitbox": {"extend": extend, "active": [2, 3]}}
    if inflate:
        out["hitbox"]["inflate"] = inflate
    if bone:
        out["strike"] = {"base": "near_arm_l", "tip": "near_arm_hand"}
    return out


def _world(tmp_path, monkeypatch, *, moves, specs_for, characters):
    """Point the module at a synthetic tree instead of the repository's."""
    tables = tmp_path / "movesets"
    tables.mkdir()
    (tables / "fake.ron").write_text(_table(moves))
    stage = tmp_path / "stages" / "fake_stage_v1" / "specs"
    stage.mkdir(parents=True)
    for spec in specs_for:
        (stage / f"{spec['clip']}.spec.json").write_text(json.dumps(spec))
    monkeypatch.setattr(cov, "TABLES", tables)
    monkeypatch.setattr(cov, "STAGES", tmp_path / "stages")
    monkeypatch.setattr(cov, "STAGE_CHARACTERS", {"fake_stage_v1": characters})


def _bundle(tmp_path, rows: list[tuple[str, str, float]]) -> pathlib.Path:
    by_character: dict[str, list[dict]] = {}
    for character, move, active in rows:
        by_character.setdefault(character, []).append(
            {"id": move, "derived": {"active_f": active}}
        )
    path = tmp_path / "bundle.json"
    path.write_text(
        json.dumps(
            {
                "characters": [
                    {"id": who, "moves": moves} for who, moves in by_character.items()
                ]
            }
        )
    )
    return path


def test_the_join_reads_only_authored_files_and_needs_no_recording(tmp_path, monkeypatch):
    """⭐ THE WHOLE POINT OF THE MODE: a ~30-minute grid recording answers the AREA
    question; this one is answered by files already in the tree."""
    _world(
        tmp_path,
        monkeypatch,
        moves=[("someone_tilt", "tilt_side")],
        specs_for=[_spec("tilt_side", extend=1.2, inflate=8.0)],
        characters=["someone"],
    )
    rows = cov.clock_join(_bundle(tmp_path, [("someone", "someone_tilt", 12.0)]))
    assert [(r["active_f"], r["extend"], r["inflate"]) for r in rows] == [(12.0, 1.2, 8.0)]


def test_a_clip_with_no_bone_spec_is_left_out(tmp_path, monkeypatch):
    """⛔ PER CLIP, NOT PER CHARACTER (`d59e982f7`). A clip with no `strike` block
    is decided by the MOVE TABLE, so counting it here would report the spec as
    deciding a box it does not decide."""
    _world(
        tmp_path,
        monkeypatch,
        moves=[("someone_tilt", "tilt_side"), ("someone_jab", "jab_clip")],
        specs_for=[_spec("tilt_side"), _spec("jab_clip", bone=False)],
        characters=["someone"],
    )
    rows = cov.clock_join(
        _bundle(tmp_path, [("someone", "someone_tilt", 4.0), ("someone", "someone_jab", 4.0)])
    )
    assert [r["move"] for r in rows] == ["someone_tilt"]


def test_nine_point_six_active_frames_is_not_ten():
    """⛔ Six authored moves sit at exactly 9.6f; `> 9` would band them long."""
    assert cov.band_of(9.6) == "6-9f"
    assert cov.band_of(10.0) == "10f+"
    assert cov.band_of(5.0) == "2-5f"


def test_a_band_one_character_dominates_is_named_and_recomputed(
    tmp_path, monkeypatch, capsys
):
    """⛔⛤ THE GUARD. Seven of nine long moves belonging to one fighter must not
    print as a roster property."""
    moves = [(f"hog_{n}", f"hog_clip_{n}") for n in range(7)]
    moves += [("other_a", "other_clip_a"), ("other_b", "other_clip_b")]
    _world(
        tmp_path,
        monkeypatch,
        moves=moves,
        specs_for=[_spec(f"hog_clip_{n}", inflate=3.0) for n in range(7)]
        + [_spec("other_clip_a"), _spec("other_clip_b")],
        characters=["hog", "other"],
    )
    rows = [("hog", f"hog_{n}", 12.0) for n in range(7)]
    rows += [("other", "other_a", 12.0), ("other", "other_b", 12.0)]
    monkeypatch.setattr(sys, "argv", ["cov", "--clock", str(_bundle(tmp_path, rows))])
    assert cov.main() == 0
    out = capsys.readouterr().out
    assert "hog holds 7 of 9 (78%)" in out
    assert "that fighter's number, not the roster's" in out
    # ⛔ AND THE THIN-BAND REFUSAL: 2 survivors cannot carry a claim either way.
    assert "without hog: n=2 ⇒ TOO THIN TO SUPPORT A CLAIM IN EITHER DIRECTION." in out


def test_an_empty_join_is_refused_rather_than_reported(tmp_path, monkeypatch):
    """⛔ An empty join reads as 'generosity does not track the clock'."""
    _world(
        tmp_path,
        monkeypatch,
        moves=[("someone_tilt", "tilt_side")],
        specs_for=[_spec("tilt_side")],
        characters=["someone"],
    )
    monkeypatch.setattr(
        sys, "argv", ["cov", "--clock", str(_bundle(tmp_path, [("nobody", "absent", 4.0)]))]
    )
    with pytest.raises(SystemExit) as caught:
        cov.main()
    assert "REFUSING TO REPORT" in str(caught.value), caught.value


def test_an_unnamed_sprite_stage_is_refused_in_clock_mode_too(tmp_path, monkeypatch):
    """⛔ The refusal `main` already had must reach this mode, or a new stage goes
    missing from the clock census while the ratio census still catches it."""
    _world(
        tmp_path,
        monkeypatch,
        moves=[("someone_tilt", "tilt_side")],
        specs_for=[_spec("tilt_side")],
        characters=["someone"],
    )
    monkeypatch.setattr(cov, "STAGE_CHARACTERS", {})
    with pytest.raises(SystemExit) as caught:
        cov.clock_join(_bundle(tmp_path, [("someone", "someone_tilt", 4.0)]))
    assert "REFUSING TO REPORT" in str(caught.value), caught.value
