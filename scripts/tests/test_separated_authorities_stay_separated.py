"""The separation guard must FIRE, and one of its rules silently did not.

⛔ Both rules report by staying silent, so a green is only worth what the
poisons are worth. These arms pin them from outside.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_separated_authorities_stay_separated as guard  # noqa: E402


def test_the_shipped_tree_holds_both_separations():
    # ⭐ THE RATCHET: it runs against the real tree.
    assert guard.main() == 0


def test_the_derive_parser_sees_a_pub_declaration():
    """⛔⛤ THE RULE TESTED NOTHING AND ITS POISON PASSED.

    The first draft joined the attribute group to `struct|enum` with `\\s*`, so
    `#[derive(..)]\\npub enum X` never connected and the derive list parsed
    EMPTY for every `pub` type -- which is every type this guard names. Adding
    `Resource` to the real declaration changed nothing and the check stayed
    green. Found by running the poison, not by reading the regex.
    """
    found = guard.declaration_derives(
        "RollbackConfirmationState", guard.production_sources()
    )
    assert found is not None, "the subject is declared nowhere"
    assert found[1], "the derive list parsed empty, which is how this rule broke before"
    assert "Resource" not in found[1]


def test_a_forbidden_derive_is_caught():
    sources = [("x.rs", "#[derive(Clone, Resource)]\npub enum RollbackConfirmationState {}")]
    rel, derives = guard.declaration_derives("RollbackConfirmationState", sources)
    assert "Resource" in derives


def test_a_visibility_modifier_does_not_hide_the_derive():
    # `pub`, `pub(crate)` and bare all have to reach the keyword.
    for vis in ("", "pub ", "pub(crate) ", "pub(super) "):
        sources = [("x.rs", f"#[derive(Resource)]\n{vis}struct Subject {{}}")]
        found = guard.declaration_derives("Subject", sources)
        assert found is not None and "Resource" in found[1], vis


def test_a_production_read_of_a_write_only_diagnostic_is_caught():
    for spelling in (
        "fn f(c: Res<LastRoomConstructionCommit>) {}",
        "fn f(c: ResMut<LastRoomConstructionCommit>) {}",
        "let c = world.get_resource::<LastRoomConstructionCommit>();",
        "let c = app.world().resource::<LastRoomConstructionCommit>();",
    ):
        assert guard.production_reads(
            "LastRoomConstructionCommit", [("x.rs", spelling)]
        ), spelling


def test_writing_a_diagnostic_is_not_reading_it():
    """⚠ THE CONTROL. Writing one is what a diagnostic is FOR; a rule that
    counted `insert_resource` would redden the shipped tree on day one."""
    for spelling in (
        "world.insert_resource(LastRoomConstructionCommit { a: 1 });",
        "app.init_resource::<LastRoomConstructionCommit>();",
    ):
        assert not guard.production_reads(
            "LastRoomConstructionCommit", [("x.rs", spelling)]
        ), spelling


def test_a_vanished_subject_is_a_failure_not_a_pass():
    """⛔ ANTI-VACUITY PER ROW. A renamed subject makes a rule pass by having
    nothing to test, which is the one way a green here would be a lie."""
    assert guard.declaration_derives("NoSuchTypeAtAll", guard.production_sources()) is None


def test_the_floor_is_measured_on_this_corpus():
    """⚠ The first draft borrowed the sibling guard's floor of 1700, which is
    over all tracked `.rs`; this scan drops test files first and sees ~1,309.
    Borrowing it reddened on the first run."""
    today = len(guard.production_sources())
    assert guard.FLOOR < today, "a floor at or above today's count reddens on arrival"
    assert guard.FLOOR > today * 0.6, (
        f"a floor of {guard.FLOOR} against {today} files is so low that a corpus "
        "collapse would pass it, which is the only thing a floor is for"
    )
