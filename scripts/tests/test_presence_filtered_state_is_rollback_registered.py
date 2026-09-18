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
