"""The `Q132` root-construction ratchet must FIRE, and must not count a declaration.

⛔ This guard reports by staying silent, so these arms pin it from outside: a
new constructor, a declared one vanishing, and the tuple-struct declaration
that looks exactly like a spawn to a naive scan.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_session_root_construction_is_declared as guard  # noqa: E402


def test_the_shipped_tree_declares_every_constructor():
    # ⭐ THE RATCHET: it runs against the real tree.
    assert guard.main() == 0


def test_the_tuple_struct_DECLARATION_is_not_a_construction():
    """⚠ `pub struct SessionRoot(pub SessionScopeId);` matches `SessionRoot(`
    exactly as a spawn does. The first scan counted it and read FIVE sites where
    there are four."""
    sources = [("a.rs", "#[derive(Component)]\npub struct SessionRoot(pub SessionScopeId);\n")]
    assert guard.construction_sites(sources) == {}


def test_a_real_construction_is_found():
    sources = [("b.rs", "world.spawn((Name::new(\"x\"), SessionRoot(owner)));\n")]
    assert guard.construction_sites(sources) == {"b.rs": [1]}


def test_a_qualified_construction_is_found():
    """A host in another crate spells the path in full, and that is the case
    this guard most needs to see — it is how a new composition mints a root."""
    sources = [
        ("c.rs", "world.spawn(shared_tangle::lifecycle::SessionRoot(SessionScopeId(7)));\n")
    ]
    assert guard.construction_sites(sources) == {"c.rs": [1]}


def test_the_declared_set_matches_the_tree_exactly():
    found = set(guard.construction_sites(guard.production_sources()))
    assert found == set(guard.DECLARED), (
        f"undeclared: {sorted(found - set(guard.DECLARED))}; "
        f"declared but gone: {sorted(set(guard.DECLARED) - found)}"
    )


def test_the_floor_is_measured_on_this_corpus():
    assert len(guard.production_sources()) >= guard.FLOOR


def test_the_shipped_tree_still_has_both_invariant_witnesses():
    """⭐ THE RATCHET for `Q132`'s consequence 7, which says add AND RETAIN."""
    assert guard.missing_witnesses() == []


def test_a_deleted_witness_is_reported(tmp_path, monkeypatch):
    """⛔ A witness that can vanish silently has an expiry date nobody set."""
    monkeypatch.setitem(
        guard.WITNESSES,
        "an_arm_that_was_never_written",
        ("game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs", "does nothing"),
    )
    gone = guard.missing_witnesses()
    assert any("an_arm_that_was_never_written" in line for line in gone)


def test_a_vanished_file_is_reported_differently_from_a_vanished_arm():
    """The two failures want different repairs: a moved file is a citation fix,
    a deleted arm is lost coverage."""
    guard.WITNESSES["x"] = ("game/ambition_app/tests/no_such_file.rs", "does nothing")
    try:
        gone = guard.missing_witnesses()
        assert any("is gone" in line for line in gone)
    finally:
        del guard.WITNESSES["x"]


def test_both_arms_are_required_because_one_can_pass_vacuously():
    """⚠ `InactiveCandidate` is a Bevy DISABLING component, so the counting arm
    is green whether or not a candidate was ever prepared. Pinning only that one
    would pin the half that cannot fail on its own."""
    assert set(guard.WITNESSES) == {
        "the_shipped_app_never_holds_two_session_roots_across_a_handoff",
        "a_prepared_candidate_never_counts_as_a_canonical_session_root",
    }
