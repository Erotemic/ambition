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
    _names, modules, _by_module = guard._tree_facts()
    assert "ambition_platformer2d_rollback_ggrs::probes" in modules, "file-derived path missing"
    assert "ambition_boss_encounter::catalog" in modules


def test_an_INLINE_module_is_in_the_population():
    """⛔ 785 inline `mod x { .. }` declarations exist and have no file.

    ⛤ **THE FIRST VERSION OF THIS ARM COUNTED NESTED PATHS AND PASSED WITH THE
    INLINE SCAN REMOVED**, because file-derived paths like `crate::a::b` also
    have two `::`. Poisoned 2026-09-18. It now names a module that has no file,
    so only the inline scan can produce it.
    """
    _names, modules, _by_module = guard._tree_facts()
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
    names, _modules, _by_module = guard._tree_facts()
    leaked = [f for f in fixtures if f in names]
    assert not leaked, (
        f"{leaked} reached the type population from the waiver file itself, so that "
        "file is now evidence for its own rows"
    )


def test_a_live_leaf_under_a_dead_module_path_is_not_a_subject():
    """⛔⛤ THE LEAF WAS THE WHOLE TEST UNTIL 2026-09-18, FOUND BY REVIEW.

    A waiver naming `a::dead::path::LiveType` passed because `LiveType` is
    declared SOMEWHERE in the tree. So a waiver could keep pointing at a module
    that had been carved away and this check — whose entire job is finding
    waivers whose subject is gone — would never say so. The rot was one path
    segment above where it was looking.
    """
    names, modules, _by_module = guard._tree_facts()
    live = "ambition_platformer2d_actor_monolith::world::rooms::transaction::ActiveContentBinding"
    prefix, _, leaf = live.rpartition("::")
    assert leaf in names, "fixture drifted: pick another declared leaf"
    assert guard.module_is_live(prefix, modules), "the real path must read live"

    # The SAME leaf, under a module that does not exist. The old rule saw only
    # the leaf, so this read as a live subject.
    dead = f"ambition_platformer2d_runtime::no_such_module::deeper::{leaf}"
    assert dead.rpartition("::")[2] in names, "the poison must keep a LIVE leaf"
    assert not guard.module_is_live(dead.rpartition("::")[0], modules), (
        "a module path the tree does not have still read as live, which is the "
        "defect this arm exists for"
    )


def test_the_relative_spelling_is_still_live():
    """⚠ THE COST OF THE RULE ABOVE, PINNED. Waivers spell the path two ways and
    both are legitimate: fully qualified from a crate root, and RELATIVE with a
    leading `::`. Measured 2026-09-18: 37 of the 127 qualified subjects use the
    relative form, so an exact match would report all 37 as dead."""
    _names, modules, _by_module = guard._tree_facts()
    assert guard.module_is_live("::world::rooms::transaction", modules)
    assert guard.module_is_live("", modules), "an unqualified leaf has no path to check"


def test_every_shipped_qualified_subject_has_a_live_module():
    """⭐ THE RATCHET, AND IT LANDED GREEN. 0 of 127 qualified subjects across
    the shipped tables name a module the tree does not have, so this tightening
    is a ratchet rather than a repair — which is worth pinning, because a rule
    that finds nothing on the day it lands is the one nobody re-checks."""
    rows = guard.waiver_rows()
    names, modules, _by_module = guard._tree_facts()
    qualified = [
        n
        for subs in rows.values()
        for n in subs
        if "::" in n and not n.endswith("::") and not n.endswith("<")
    ]
    assert len(qualified) >= 100, f"only {len(qualified)} qualified subjects parsed"
    dead = [n for n in qualified if not guard.module_is_live(n.rpartition("::")[0], modules)]
    assert dead == [], dead


def test_a_live_leaf_under_a_LIVE_BUT_WRONG_module_is_not_a_subject():
    """⛔⛤ THE REVIEW FINDING (2026-09-19): TWO EXISTENCE FACTS ARE NOT A
    RELATION.

    The rule used to be `leaf in names and module_is_live(prefix)` — the type is
    declared somewhere, the module exists somewhere, neither claim about the
    other. This arm is the case that separates the two rules: both halves are
    TRUE and the pairing is still false, so the old rule passes it and the new
    one must not. `..::geometry::X` would fail under either, which is why that
    poison proves nothing here.
    """
    names, modules, by_module = guard._tree_facts()
    real = "ambition_portal2d::tuning::EditablePortalTuning"
    prefix, _, leaf = real.rpartition("::")
    assert guard.subject_is_declared_in_module(prefix, leaf, by_module), (
        "the shipped spelling must read live, or this arm is measuring a typo"
    )

    wrong = "ambition_portal2d::color"
    assert leaf in names, "the leaf must be declared SOMEWHERE for this to bite"
    assert guard.module_is_live(wrong, modules), "the module must be LIVE for this to bite"
    assert not guard.subject_is_declared_in_module(wrong, leaf, by_module)


def test_every_shipped_qualified_subject_is_declared_in_the_module_it_names():
    """⭐ THE RATCHET, AND THE MEASUREMENT THAT MADE THE STRICT RULE SAFE.

    The loose rule's stated reason was re-exports: a type published under a
    path other than where it is declared would redden. Measured across the
    shipped tables — 127 of 127 qualified subjects are declared in the module
    they name, so the worry has no instance and the strict rule costs nothing.
    """
    names, _modules, by_module = guard._tree_facts()
    qualified = [
        n
        for needles in guard.waiver_rows().values()
        for n in needles
        if not n.endswith("::") and not n.endswith("<") and "::" in n
    ]
    assert len(qualified) >= 100, f"population collapsed to {len(qualified)}"
    off = [
        n for n in qualified
        if not guard.subject_is_declared_in_module(
            n.rpartition("::")[0], n.rpartition("::")[2], by_module
        )
    ]
    assert not off, f"declared outside the module they name: {off}"
