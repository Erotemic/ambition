"""The writer-set ratchet must run in a lane, and must be able to FAIL.

⛔ A guard with no test beside it runs in no lane — that happened in this
repository on 2026-09-16 to a checker that was committed, correct, and executed
by nothing.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_sim_consumed_request_writers as guard  # noqa: E402


def test_the_writer_set_is_adjudicated_today():
    assert guard.main() == 0


def test_every_adjudicated_writer_names_a_file_that_exists():
    """⛔ A KEY IS A CITATION, AND A FILE MOVE BREAKS IT IN SILENCE.

    The ratchet compares two sets. A table key pointing at a vanished path drops
    out of the intersection as "no longer exists" — which is the right answer for
    a DELETED writer and the wrong one for a MOVED file, and the two read
    identically. This arm keeps the paths honest independently.
    """
    for _type_name, (_why, table) in guard.SUBJECTS.items():
        for key in table:
            rel = key.rsplit("::", 1)[0]
            assert (guard.ROOT / rel).is_file(), f"{key}: {rel} does not exist"


def test_the_pattern_finds_a_lifetime_qualified_system_param():
    """⚠ THE SHAPE THAT HIDES A WRITER IS THE ONE INSIDE A `SystemParam`.

    `ResMut<'w, T>` is how a parameter bundle declares a write, and a pattern
    without the optional lifetime misses exactly that. This subject HAS such a
    writer (the session-teardown bundle), so a regression here would silently
    shrink the population rather than fail.
    """
    pattern = guard.mutable_write_pattern("Thing")
    assert pattern.search("    thing: ResMut<'w, other::Thing>,")
    assert pattern.search("    thing: ResMut<Thing>,")
    assert pattern.search("fn f(q: &mut crate::Thing) {}")
    assert not pattern.search("    thing: Res<Thing>,")
    assert not pattern.search("    thing: ResMut<ThingElse>,")


def test_a_struct_owner_is_reported_as_the_struct():
    """⛔ CREDITING THE PREVIOUS `fn` WOULD NAME THE WRONG THING.

    A previous census in this repository credited two systems to helper functions
    defined earlier in the file. Here the miss is worse: the write is declared on
    a `SystemParam` struct, and the nearest `fn` above it can be in a different
    concern entirely.
    """
    lines = [
        "fn something_else() {}",
        "",
        "pub struct Bundle<'w> {",
        "    thing: ResMut<'w, Thing>,",
        "}",
    ]
    assert guard.owner_of(lines, 3) == "struct Bundle"
    assert guard.owner_of(lines, 0) == "something_else"


def test_a_test_path_is_recognised_as_one():
    assert guard.is_test_path("game/ambition_app/tests/a.rs")
    assert guard.is_test_path("crates/x/src/session/teardown/tests.rs")
    assert not guard.is_test_path("crates/x/src/session/teardown.rs")
