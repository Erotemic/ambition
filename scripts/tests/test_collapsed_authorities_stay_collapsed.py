"""The collapse ratchet, against corpora with known answers."""

from __future__ import annotations

import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def _load():
    name = "check_collapsed_authorities_stay_collapsed"
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


GUARD = _load()


def test_every_collapse_is_still_a_collapse():
    back = {name: hits for name, hits in GUARD.code_occurrences().items() if hits}
    assert not back, f"deleted-to-collapse type(s) that are code again: {sorted(back)}"


def test_the_verdict_runs_and_passes():
    """⛔ The unit arms below test the PARTS. This is the thing the lane runs."""
    assert GUARD.main() == 0


def test_every_row_names_a_family_and_a_reason():
    for name, row in GUARD.COLLAPSED.items():
        family, why = row
        assert family and family.isupper() or "-" in family, name
        assert len(why) > 40, f"{name}'s row does not say what its return re-opens"


def test_a_comment_naming_the_type_is_not_a_return():
    """⛔⛤ THE WHOLE REASON THIS CHECK CAN EXIST.

    Every one of these names is still in the tree, in the comments that record
    the deletion — four times for `GameplaySessionLinks` alone. A check that
    counted the string would have been red on its first run and would have been
    deleted as broken, taking the real ratchet with it.
    """
    stripped = GUARD.frozen.code_only(
        "// GameplaySessionLinks held the scope\n"
        "/* and CandidateState was the second road */\n"
        'let s = "SpawnPlayerCloneRequest";\n'
        "let t = r#\"AdmittedCheckpointRestore\"#;\n"
    )
    for name in GUARD.COLLAPSED:
        assert name not in stripped, f"{name} survived comment/string stripping"


def test_a_real_declaration_is_a_return():
    """The other direction: code the stripper must NOT blank."""
    stripped = GUARD.frozen.code_only("pub struct GameplaySessionLinks { a: u8 }\n")
    assert "GameplaySessionLinks" in stripped


def test_the_floor_is_below_the_corpus_and_would_catch_a_collapse():
    assert GUARD.FLOOR < GUARD.scanned_files()
    assert GUARD.FLOOR > 0
