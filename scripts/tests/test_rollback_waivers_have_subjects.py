"""The waiver-subject check, and the four scopes a waiver row can be written in.

⛔⛤ Every one of those four was a separate wrong answer on this check's first
runs, and each looked like a page of dead rows rather than a rule gap.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_rollback_waivers_have_subjects as guard  # noqa: E402


def test_the_real_tree_has_no_subjectless_waiver():
    dead, _sizes = guard.subjectless()
    assert not dead, f"waiver rows with no subject: {dead}"


def test_the_tables_actually_parse():
    """⛔ THE FLOOR CAUGHT A REAL BUG ON THE FIRST RUN.

    `text.index("[", start)` found the `[` in `&[(&str, &str)]` rather than the
    array's, so the brace scan closed on the type signature and every table
    parsed as EMPTY — which reports zero dead rows, serenely, forever.
    """
    rows = guard.waiver_rows()
    assert set(rows) == set(guard.TABLES), sorted(rows)
    assert sum(len(v) for v in rows.values()) >= guard.FLOORS["waiver rows"]


def test_a_module_family_is_matched_by_a_derived_module_PATH():
    """⭐ NOT by searching source for the spelling `a::b::c`.

    The first rule did that and called NINE live modules dead: Rust writes
    `mod c;` and `use crate::b::c`, so a waiver's full path almost never appears
    anywhere. `crates/X/src/a/b.rs` IS the module `X::a::b`.
    """
    _names, modules = guard._tree_facts()
    assert "ambition_platformer2d_rollback_ggrs::probes" in modules, "file-derived path missing"
    assert "ambition_boss_encounter::catalog" in modules


def test_an_INLINE_module_is_in_the_population():
    """⛔ 785 inline `mod x { .. }` declarations exist and have no file.

    ⛤ **THE FIRST VERSION OF THIS ARM COUNTED NESTED PATHS AND PASSED WITH THE
    INLINE SCAN REMOVED**, because file-derived paths like `crate::a::b` also
    have two `::`. Poisoned 2026-09-18. It now names a module that has no file,
    so only the inline scan can produce it.
    """
    _names, modules = guard._tree_facts()
    assert "ambition_app::app::cli::cli_arg_tests" in modules, (
        "an inline `mod cli_arg_tests {` inside `app/cli.rs` is missing from the "
        "module population, so a waiver naming any inline module would read as dead"
    )


def test_an_EXTERNAL_crate_prefix_is_live_via_the_lockfile():
    """⭐ `bevy_asset::` waives types this workspace never declares."""
    assert 'name = "bevy_state"' in guard._lockfile()
    dead, _ = guard.subjectless()
    flat = [n for ns in dead.values() for n in ns]
    assert not [n for n in flat if n.startswith("bevy_")], flat


def test_the_waiver_FILE_is_not_evidence_for_its_own_rows():
    """⛔⛤ THE EXCLUSION IS LOAD-BEARING FOR TYPE NAMES, NOT FOR MODULE PATHS —
    AND MY FIRST VERSION OF THIS ARM TESTED THE WRONG HALF.

    It asserted no module path contains `player_clone`, and passed with the
    exclusion removed: `rollback_coverage.rs` lives under `tests/`, not `src/`,
    so it never contributes a module path at all. What it DOES contribute is two
    fixture types for its own negative controls — `DeliberatelyUnregistered` and
    `WrittenOffTheSimSchedule` — and without the exclusion a waiver row naming
    either would be vouched for by the waiver file itself.
    """
    source = guard.COVERAGE.read_text(errors="replace")
    fixtures = ["DeliberatelyUnregistered", "WrittenOffTheSimSchedule"]
    for fixture in fixtures:
        assert f"struct {fixture}" in source, (
            f"{fixture} is no longer declared in {guard.COVERAGE.name}, so this arm's "
            "premise is gone — point it at whatever fixture that file declares now"
        )
    names, _modules = guard._tree_facts()
    leaked = [f for f in fixtures if f in names]
    assert not leaked, (
        f"{leaked} reached the type population from the waiver file itself, so that "
        "file is now evidence for its own rows"
    )
