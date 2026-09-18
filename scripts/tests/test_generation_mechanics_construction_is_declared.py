"""Q144's enforcement, and the two shapes that read exactly like each other.

⛔⛤ A FILE THAT NAMES A CONSTRUCTOR IN PROSE READS EXACTLY LIKE A FILE THAT
CALLS IT, and that is not hypothetical: `Q144`'s own table counted four
`for_live_session` roads because `world/rooms/stage.rs` names the constructor in
the doc comment on an error variant. The arms below plant both shapes.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_generation_mechanics_construction_is_declared as guard  # noqa: E402
from lib.rust_source import strip_comments  # noqa: E402
from lib.test_paths import strip_test_modules  # noqa: E402


def _sources(files: dict[str, str]) -> list[tuple[str, str]]:
    """The same preparation `production_sources` applies, on synthetic text."""
    return [(rel, strip_test_modules(strip_comments(body))) for rel, body in files.items()]


def test_a_call_is_a_construction_site():
    sources = _sources({"crates/x/src/lib.rs": "fn f(g: &G) { GenerationMechanics::of(g) }"})
    assert guard.construction_sites(sources) == {("crates/x/src/lib.rs", "of"): 1}


def test_a_DOC_COMMENT_naming_the_constructor_is_not_a_site():
    """⭐ THE CONTROL, AND IT IS THE MISTAKE `Q144`'s TABLE MADE.

    `strip_comments` runs before the pattern, so the prose cannot be counted.
    Poisoning this is a one-word edit to `production_sources` — drop the
    `strip_comments` call — and it turns `stage.rs` and `spawn/mod.rs` back into
    construction sites the baseline has never declared.
    """
    sources = _sources({
        "crates/x/src/lib.rs": "\n".join([
            "/// Built by `GenerationMechanics::for_live_session(..)`, which refuses.",
            "// see also GenerationMechanics::new(None, ..)",
            "pub enum E { LiveGenerationMechanicsMissing }",
        ]),
    })
    assert guard.construction_sites(sources) == {}


def test_a_test_module_inside_a_production_file_is_not_a_site():
    sources = _sources({
        "crates/x/src/lib.rs": "\n".join([
            "pub fn live(g: &G) { GenerationMechanics::of(g) }",
            "#[cfg(test)]",
            "mod tests {",
            "    fn fixture() { GenerationMechanics::new(None, None, &s, &b) }",
            "}",
        ]),
    })
    assert guard.construction_sites(sources) == {("crates/x/src/lib.rs", "of"): 1}


def test_two_calls_in_one_file_are_two_readings():
    """The key carries a COUNT, so a road cannot quietly double."""
    sources = _sources({
        "crates/x/src/lib.rs": (
            "fn a(g: &G) { GenerationMechanics::of(g) }\n"
            "fn b(g: &G) { GenerationMechanics::of(g) }\n"
        ),
    })
    assert guard.construction_sites(sources) == {("crates/x/src/lib.rs", "of"): 2}


def test_every_live_construction_in_the_real_tree_is_declared():
    found = guard.construction_sites()
    undeclared = sorted(k for k in found if k not in guard.DECLARED)
    assert not undeclared, f"live GenerationMechanics constructions nobody has read: {undeclared}"


def test_no_declared_row_outlives_its_site():
    found = guard.construction_sites()
    stale = sorted(k for k in guard.DECLARED if k not in found)
    assert not stale, f"declared rows with no site left: {stale}"


def test_the_new_road_is_the_one_q144_option_2_must_delete():
    """⚠ AN ASSERTION ABOUT THE SHAPE OF THE ANSWER, NOT ABOUT A COUNT.

    `::new` is the only constructor that can read the App registries, so it is
    what makes `DUP-GENERATION-MECHANICS` reachable rather than theoretical. If
    this ever reads zero, option 2 has landed and the census row can close —
    which is a change to make deliberately, here and on the page, not a test to
    relax.
    """
    new_roads = {k for k in guard.construction_sites() if k[1] == "new"}
    assert len(new_roads) == 1, (
        "the live `GenerationMechanics::new` population moved. One road (the hot reload in "
        f"dev_runtime.rs) is the 2026-09-18 reading; found {sorted(new_roads)}. Zero means "
        "Q144 option 2 landed; more than one means a second App-registry reader appeared."
    )


def test_the_declared_table_is_load_bearing_and_says_why():
    for key, (count, reading) in guard.DECLARED.items():
        assert count >= 1, key
        assert "read 20" in reading, f"{key}'s reading is undated"
        assert len(reading) > 80, f"{key}'s reading is too short to be an argument"
