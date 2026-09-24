"""The presence-filter guard, and the poison that caught its first version.

⛔⛤ **THE ARM THAT MATTERS HERE IS `test_a_path_qualified_filter_is_seen`.** The
first version of the guard matched `With<Name>` with a bare name, and the poison
that should have proved it — deleting `Dormant`'s row from the recorded schema —
left the check GREEN. `Dormant` is the component the repository's own doctrine
paragraph is written about, and all three of its production filter sites spell it
`Without<crate::features::ecs::dormancy::Dormant>`. A guard that cannot see the
one example its docstring quotes is reporting on its regex.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_presence_filtered_state_is_rollback_registered as guard  # noqa: E402


def _filtered(text: str) -> list[str]:
    return guard._FILTER.findall(text)


def _components(text: str) -> list[str]:
    return [m.group(2) for m in guard._DERIVE_COMPONENT.finditer(text) if "Component" in m.group(1)]


def test_a_path_qualified_filter_is_seen():
    """The defect that made the guard's own poison pass."""
    assert _filtered("Without<crate::features::ecs::dormancy::Dormant>") == ["Dormant"]
    assert _filtered("With<ambition_combat::components::PostBossNpc>") == ["PostBossNpc"]


def test_has_is_a_presence_read_like_the_other_two():
    assert _filtered("Has<EncounterMob>") == ["EncounterMob"]
    assert _filtered("(With<A>, Without<B>, Has<C>)") == ["A", "B", "C"]


def test_a_non_component_generic_is_not_a_filter():
    """`Query<&T>` and `Option<&T>` read a VALUE; only the three filter forms
    read presence."""
    assert _filtered("Query<&BodyHealth>, Option<&CombatTuning>") == []


def test_a_derive_naming_component_is_a_component_and_others_are_not():
    assert _components("#[derive(Component, Clone)]\npub struct Marker;") == ["Marker"]
    assert _components("#[derive(Resource, Clone)]\npub struct NotOne;") == []
    assert _components("#[derive(Component)]\npub enum Mode { A }") == ["Mode"]


def test_an_attribute_between_the_derive_and_the_item_does_not_hide_it():
    text = "#[derive(Component, Default)]\n#[require(Transform)]\npub struct Gated;"
    assert _components(text) == ["Gated"]


def test_the_real_tree_reports_no_unwaived_presence_filtered_component():
    registered = guard.registered_type_names()
    defs = guard.component_definitions(guard.registering_crates())
    sites = guard.filter_sites()
    unregistered = sorted(n for n in sites if n in defs and n not in registered)
    new = [n for n in unregistered if n not in guard.WAIVERS and n not in guard.ACKNOWLEDGED]
    assert not new, (
        "a query filters on these components and no rollback registration restores "
        f"them: {new}. Register them, or add a WAIVER stating which filter sites "
        "read the marker and which schedule each runs in."
    )


def test_every_named_component_is_still_filtered_on_somewhere():
    """⛔ A STALE WAIVER IS A HOLE WITH A COMMENT OVER IT. When a component stops
    being filtered on, its row here stops describing anything and is free to
    cover whatever lands on the name next."""
    sites = guard.filter_sites()
    stale = sorted(n for n in (guard.WAIVERS | guard.ACKNOWLEDGED.keys()) if n not in sites)
    assert not stale, (
        f"these are named in WAIVERS or ACKNOWLEDGED and no production query "
        f"filters on them any more: {stale}. Delete the row rather than leaving it."
    )


def test_the_two_tables_never_name_the_same_component():
    both = sorted(set(guard.WAIVERS) & set(guard.ACKNOWLEDGED))
    assert not both, (
        f"{both} are both waived and acknowledged. A waiver says the presence is "
        "not authoritative; an acknowledgement says it is and is owed. They cannot "
        "both be the reading."
    )


def test_the_live_scan_reasoned_about_something_at_all():
    """⛔ THE FLOOR IS ON THE INTERSECTION. Two large operands with no overlap
    print the same `OK` as a clean tree."""
    registered = guard.registered_type_names()
    defs = guard.component_definitions(guard.registering_crates())
    sites = guard.filter_sites()
    intersection = [n for n in sites if n in defs]
    assert len(registered) >= guard.MIN_REGISTERED
    assert len(defs) >= guard.MIN_DEFINED
    assert len(sites) >= guard.MIN_FILTERED
    assert len(intersection) >= guard.MIN_INTERSECTION, (
        f"only {len(intersection)} component(s) are both defined in a registering "
        f"crate and filtered on, out of {len(defs)} definitions and {len(sites)} "
        "filtered names — the join is keyed wrongly and every verdict above is vacuous"
    )


def test_a_component_defined_below_its_files_test_declaration_is_in_the_population():
    """⛔⛤ THE POPULATION USED TO END AT THE FIRST `#[cfg(test)]` — 2026-09-17.

    `component_definitions` cut each file at `text.find("#[cfg(test)]")`, and in
    this tree a module declares its tests near the TOP:
    `shared_tangle/src/construction/mod.rs` writes `#[cfg(test)] mod tests;` and
    then defines most of A10's vocabulary underneath it. MEASURED: 288 component
    definitions became 305 and the intersection 95 became 104, and two of the
    nine that appeared were neither registered nor waived.

    ⇒ This arm pins a SUBJECT rather than a count, because the blindness failed
    in the GREEN direction: the guard reported `OK` over a population missing the
    whole second half of the construction crate, and no floor here was low enough
    to notice.
    """
    defs = guard.component_definitions(guard.registering_crates())
    assert "InactiveCandidate" in defs, (
        "`InactiveCandidate` is declared in `construction/mod.rs` below that "
        "file's `#[cfg(test)] mod tests;` line. If it is missing, the definition "
        "scan is cutting file tails again."
    )
    assert defs["InactiveCandidate"].endswith("construction/mod.rs")
    assert "PresentationOnly" in defs


def test_a_filter_through_a_path_with_a_digit_is_found():
    """⛔⛔ THE PREFILTER WAS A SECOND, NARROWER GRAMMAR — AND IT ATE THE MOST
    COMMON PATH IN THIS WORKSPACE.

    `filter_sites` used to shortlist candidate lines with
    `git grep -E '(With(out)?|Has)<[A-Za-z_:]*[A-Z][A-Za-z0-9_]*>'` and only then
    apply `_FILTER`. That character class has NO DIGITS, so every crate whose
    name carries one — `ambition_platformer2d`, `ambition_portal2d` — was
    unreachable through a qualified path. `Has<ambition_platformer2d::characters::
    actor::BodyWalletShield>` in `demo_sanic/src/lib.rs:1948` is exactly that,
    and it is how the guard came to print `OK` over a subject it had discarded.
    Found by review 2026-09-17; the intersection went 104 -> 114.

    ⭐ This arm pins the GRAMMAR rather than the count, because the count is what
    a tree change is allowed to move.
    """
    found = guard._FILTER.findall(
        "        bevy::prelude::Has<ambition_platformer2d::characters::actor::BodyWalletShield>,"
    )
    assert found == ["BodyWalletShield"], found
    # ⚠ AND THE SHAPE THE OLD PREFILTER COULD HANDLE MUST STILL WORK, or this
    # would be a swap rather than a widening.
    assert guard._FILTER.findall("With<Foo>, Without<Bar>") == ["Foo", "Bar"]


def test_neither_a_fixture_nor_a_paragraph_is_a_production_filter_site():
    """⛔⛤ THE TWO STRIPS `filter_sites` GAINED, PINNED SEPARATELY — BECAUSE THE
    ONE SUBJECT THEY REMOVED FROM THIS TREE IS COVERED BY BOTH.

    `HitboxLifetime` left the intersection when `filter_sites` stopped
    prefiltering, and MEASURED 2026-09-17 its only site in the workspace is a
    DOC COMMENT inside a `#[cfg(test)] mod`: `clash.rs:790`, *"`arbitrate_attack_clanks`
    filtered on `With<HitboxLifetime>`"*, under the test module opened at
    `clash.rs:212`. ⚠ Both the review that found this and my own first note said
    it was an inline test module; it is that AND prose, and either strip alone
    removes it.

    ⇒ So a poison of either strip leaves the tree-level reading green — verified,
    both passed — and the only poison that reddens it is both at once. A
    tree-level assertion therefore cannot tell me which rule is doing the work,
    which is why this arm pins the RULE over hand-built text instead.
    """
    from lib.rust_source import strip_comments
    from lib.test_paths import strip_test_modules

    fixture_only = (
        "fn prod(q: Query<Entity, With<Real>>) {}\n"
        "#[cfg(test)]\nmod tests {\n"
        "    fn f(q: Query<Entity, With<FixtureOnly>>) {}\n}\n"
    )
    kept = guard._FILTER.findall(strip_test_modules(strip_comments(fixture_only)))
    assert kept == ["Real"], kept

    prose_only = (
        "/// This used to filter on `Without<Gone>` before the carve.\n"
        "fn prod(q: Query<Entity, With<Real>>) {}\n"
    )
    kept = guard._FILTER.findall(strip_test_modules(strip_comments(prose_only)))
    assert kept == ["Real"], kept


def test_the_comment_strip_removes_four_real_names_from_this_corpus():
    """⛔⛤ I WROTE THIS ARM TO RECORD A NEGATIVE AND THE MEASUREMENT REFUTED IT.

    The claim was that stripping comments in `filter_sites` is a precaution with
    no effect on this tree. It removes **four** names that nothing else does:

        Camera2d        render/.../camera.rs:141 and view_isolation.rs:26 —
                        two comments contrasting `With<MainCamera>` with the
                        broad `With<Camera2d>`
        FrameTimeGraph  a dev-overlay comment
        HomingDash      demo_smash/src/homing.rs:106 — a comment explaining why
                        the fix is NOT `Without<HomingDash>`
        MovePlayback    boss_encounter/.../tick.rs:146 — prose about what a
                        filter prevents

    ⇒ Every one is prose ABOUT a filter, and three of the four are prose about a
    filter that was deliberately NOT written — the exact shape that makes a
    text-scanning census invent subjects.

    ⚠ **AND THE HONEST BOUNDARY, WHICH I GOT WRONG TWICE BEFORE MEASURING IT.**
    Poisoning the strip leaves the guard GREEN, and the first explanation — *"none
    of them is a defined component in a registering crate"* — is false:
    `MovePlayback` (`combat/src/moveset/mod.rs`) is, so without this strip it
    enters the intersection. The verdict survives because it is already
    REGISTERED — so prose promotes a component into a census that then correctly
    says nothing is owed about it. (`ActorConfig` was a fifth, prose-only in
    `shared_tangle/src/body.rs:4`, until the residency claim began filtering
    `With<ActorConfig>` in code: a real site now, so the strip no longer moves it.)
    ⇒ That is the failure direction to worry about, stated precisely: prose cannot
    currently invent an OWED subject here, but it can inflate the population a
    planning page quotes, and the day one of those names is unregistered it would
    invent a finding. What moved the verdict 104 -> 114 was dropping the `git
    grep` prefilter; what this strip moves is the count.
    """
    from lib.rust_source import strip_comments
    from lib.test_paths import strip_test_modules

    with_comments = set()
    without = set()
    for root in ("crates", "game"):
        for path in sorted((guard.REPO / root).rglob("*.rs")):
            rel = path.relative_to(guard.REPO)
            if any(part == "target" for part in rel.parts):
                continue
            text = path.read_text(errors="replace")
            if guard.is_test_path(rel, text):
                continue
            with_comments.update(guard._FILTER.findall(strip_test_modules(text)))
            without.update(
                guard._FILTER.findall(strip_test_modules(strip_comments(text)))
            )
    assert sorted(with_comments - without) == [
        "Camera2d",
        "FrameTimeGraph",
        "HomingDash",
        "MovePlayback",
    ], sorted(with_comments - without)
    assert not (without - with_comments), "stripping comments cannot ADD a site"
    # ⭐ THE BOUNDARY, ASSERTED RATHER THAN ASSUMED. One of the four IS a defined
    # component in a registering crate, so prose does move the population; the
    # verdict survives only because it is already registered. If it stops
    # being registered, prose alone would invent an owed subject — and this is
    # the arm that says so.
    defs = set(guard.component_definitions(guard.registering_crates()))
    promoted = sorted((with_comments - without) & defs)
    assert promoted == ["MovePlayback"], promoted
    registered = guard.registered_type_names()
    assert all(name in registered for name in promoted), [
        name for name in promoted if name not in registered
    ]
    # ⭐ Anti-vacuity: both scans must have found a real population, or the set
    # difference above is a statement about two nearly-empty sets.
    assert len(without) > 150, len(without)


def test_the_live_intersection_has_a_population_and_no_unwaived_subject():
    sites = guard.filter_sites()
    assert "HitboxLifetime" not in sites, (
        "a doc comment inside a test module is being read as a filter site again"
    )
    assert "PresentationOf" in sites and len(sites["PresentationOf"]) == 3
    assert len(sites) > 150, len(sites)
    # ⚠ `main` builds an `ArgumentParser` and parses `sys.argv`, which under
    # pytest is pytest's own — it exits 2 rather than running the check.
    import sys

    saved = sys.argv
    sys.argv = [saved[0]]
    try:
        assert guard.main() == 0
    finally:
        sys.argv = saved


def test_every_acknowledged_subject_is_named_by_the_row_it_is_owed_to():
    """⛔⛤ A HEADING IS A SECOND OWNER OF THE BODY'S FACT, and nothing checked it.

    `Q142` read *"the question is down to `PostBossNpc`"* in its heading for a
    day after this guard started reporting TWO subjects: widening the component
    population to `game/` added `SmirkingBehemothVictoryNpc`, the body recorded
    it in four places, and the heading — the part a reader sees first — still
    said one.

    The check cannot verify a COUNT and does not try. It asks the cheapest
    version: does the row an acknowledgement CITES name the subject at all?
    """
    missing = guard.subjects_missing_from_their_row()
    assert not missing, (
        f"acknowledged subject(s) owed to a row that never names them: {missing}"
    )
    owed = guard.owed_rows()
    assert owed, "no acknowledgement names a question row, so the check is vacuous"
    assert set(owed) == set(guard.ACKNOWLEDGED), (
        "an acknowledgement stopped naming the row it is owed to: "
        f"{sorted(set(guard.ACKNOWLEDGED) - set(owed))}"
    )


def test_the_row_match_is_word_bounded():
    """⚠ THE FIRST VERSION'S OWN POISON PASSED, because it used `in`.

    Renaming the subject to `SmirkingBehemothVictoryNpcXX` on the page left the
    old name as a SUBSTRING of the new one, so the check reported the row still
    naming a component that no longer existed. A containment test between two
    identifiers is almost never the test you want.
    """
    import re

    for subject in guard.ACKNOWLEDGED:
        assert not re.search(rf"\b{re.escape(subject)}\b", f"{subject}XX"), (
            f"`{subject}` still matches `{subject}XX` — the boundary is gone"
        )
