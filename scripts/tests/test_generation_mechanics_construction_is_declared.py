"""Q144's enforcement, and the two shapes that read exactly like each other.

⛔⛤ A FILE THAT NAMES A CONSTRUCTOR IN PROSE READS EXACTLY LIKE A FILE THAT
CALLS IT, and that is not hypothetical: `Q144`'s own table counted four
`for_live_session` roads because `world/rooms/stage.rs` names the constructor in
the doc comment on an error variant. The arms below plant both shapes.
"""

from __future__ import annotations

import re
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


def test_no_constructor_reads_the_app_registries_any_more():
    """Q144 OPTION 2 LANDED, AND THIS HOLDS IT SHUT.

    This arm used to assert `len(new_roads) == 1` and say that a zero *"means
    option 2 landed ... a change to make deliberately, here and on the page"*.
    It landed: `a49ae6654` moved the LDtk hot reload — the last caller — onto
    `for_live_session`, and `GenerationMechanics::new` was deleted with it, so
    `DUP-GENERATION-MECHANICS` reads RESOLVED in the census. The arm is
    re-pointed rather than relaxed.

    ⛔ TWO CHECKS, BECAUSE ZERO CALL SITES IS THE WEAKER CLAIM. A constructor
    that still EXISTS with no caller is one import away from coming back, and
    the census row's argument is *"there is no constructor left that accepts
    loose registries"* — a statement about the TYPE. So: nobody calls it, and
    there is nothing to call.
    """
    new_roads = {k for k in guard.construction_sites() if k[1] == "new"}
    assert not new_roads, (
        "a second App-registry reader appeared: `GenerationMechanics::new` is called at "
        f"{sorted(new_roads)}. `DUP-GENERATION-MECHANICS` reads RESOLVED in "
        "architecture-census.md on the strength of there being no such road."
    )

    home = REPO / "crates/ambition_platformer2d_actor_monolith/src/session/mechanics.rs"
    body = strip_comments(home.read_text())
    constructors = sorted(set(re.findall(r"pub fn (\w+)\(\s*\n?\s*(?:generation|active)", body)))
    assert constructors == ["for_live_session", "of"], (
        "the `GenerationMechanics` constructor set moved; the census row's claim is that "
        f"`of` and `for_live_session` are all there is. Found {constructors}."
    )
    assert "pub fn new(" not in body, (
        "`GenerationMechanics::new` is back. It is the constructor that accepts loose "
        "registries, which is the whole of `DUP-GENERATION-MECHANICS`."
    )


def test_the_declared_table_is_load_bearing_and_says_why():
    for key, (count, reading) in guard.DECLARED.items():
        assert count >= 1, key
        assert "read 20" in reading, f"{key}'s reading is undated"
        assert len(reading) > 80, f"{key}'s reading is too short to be an argument"
