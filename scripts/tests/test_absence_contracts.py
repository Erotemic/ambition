"""The absence guard has to FIRE, and has to stay quiet on prose.

Every contract in `ABSENCE_CONTRACTS` is green against the live tree — that is
the point of it — so a test that only ran the script would prove nothing. A
guard that is green at minute zero guards nothing (learned three times on this
repo's goal checks). These tests do the two things the live run cannot:

* feed each contract a line that VIOLATES it and require a hit, so the pattern
  is known to match the thing it forbids; and
* feed each contract that same text as a DOC COMMENT and require silence, which
  is the specific recurrence this mechanism exists to survive — three separate
  absence checks went red because somebody documented the removal.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_absence_contracts import (  # noqa: E402
    ABSENCE_CONTRACTS,
    strip_comments_for,
    violations,
)

# One line that each contract must reject, written the way real code would.
VIOLATING_LINE = {
    # The shortest path back to a 1,870-line function: adding a registration
    # where all the others used to be. Campaign 2 R5.
    "central-rollback-does-not-enumerate-domains":
        '    app.rollback_component_clone::<ambition_portal2d::PortalBody>(ENGINE, "portal.body");',
    "the-global-roster-is-retired-only-by-its-owner":
        "            commands.remove_resource::<MatchParticipantRoster>();",
    "the-seat-topology-has-one-engine-side-creator":
        "        commands.insert_resource(ambition_input::LocalSeatTopology::default());",
    "registration-does-not-demand-art": "    CharacterLoadDemand::request(&mut demand, id);",
    "no-string-keyed-sheet-row-lookup": "    let row = sheet.row_index_of(name)?;",
    "rollback-exit-oracle-is-not-quarantined": "#[ignore]",
    "fight-tests-do-not-hand-roll-damage": "    mary_hp -= 12;",
    # The line that really stood in `src/bin/dump.rs` until A4 retired it — and
    # the one that made the deleted path worth guarding rather than merely
    # tidying: it installed the WINDOWED host in a HEADLESS dump, and nothing
    # noticed, because the registries the dump prints do not come from the host.
    "outlander-does-not-hand-order-its-own-composition":
        "    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);",
    "the-catalog-default-action-set-is-confined-to-one-file":
        "    let authored = catalog.build_default_action_set(id);",
    "the-provider-resolver-is-confined-to-one-file":
        "    let p = provider_of_character(registry, owners, id);",
    "the-motion-model-resolver-is-confined-to-one-file":
        "    let m = motion_model_spec_for_character(registry, catalog, id);",
    "the-catalog-axis-tuning-is-confined-to-one-file":
        "    match catalog.axis_tuning(id) {",
    "the-movement-tuning-resolver-is-confined-to-one-file":
        "    let t = movement_tuning_for_character(registry, catalog, id);",
    "the-worlds-path-is-confined-to-ldtk-paths":
        '    DEFAULT_LDTK = REPO_ROOT / "assets" / "worlds" / "sandbox.ldtk"',
}

# The language each contract's subject is written in, which decides what a
# COMMENT looks like when the prose check strips one. Rust unless stated: the
# harness assumed Rust everywhere until the first Python contract arrived
# (2026-07-28), and a prose check that only ever tested `//` would have proved
# nothing about a `#`.
CONTRACT_LANGUAGE = {
    "the-worlds-path-is-confined-to-ldtk-paths": "py",
}


def comment_forms(language: str) -> tuple[str, list[str]]:
    """A representative source path and the comment shapes for `language`."""
    if language == "py":
        return "some/file.py", ["# {}", "    # {}"]
    return "some/file.rs", ["/// This used to be `{}` and no longer is.",
                            "//! {}", "    // {}"]


def confirm_patterns(contract: dict) -> list[re.Pattern]:
    compiled = []
    for pattern in contract["patterns"]:
        expression = pattern if isinstance(pattern, str) else pattern["match"]
        compiled.append(re.compile(expression))
    return compiled


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_each_contract_matches_a_real_violation(contract):
    line = VIOLATING_LINE[contract["id"]]
    path, _ = comment_forms(CONTRACT_LANGUAGE.get(contract["id"], "rs"))
    stripped = strip_comments_for(path, line)
    assert any(pattern.search(stripped) for pattern in confirm_patterns(contract)), (
        f"{contract['id']} would not notice {line!r} — a contract that cannot "
        "match its own violation is decoration"
    )


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_no_contract_fires_on_prose_describing_the_removal(contract):
    """Documenting a removal must not break the guard that verified it."""
    line = VIOLATING_LINE[contract["id"]]
    path, forms = comment_forms(CONTRACT_LANGUAGE.get(contract["id"], "rs"))
    for comment in (form.format(line.strip()) for form in forms):
        stripped = strip_comments_for(path, comment)
        assert not any(
            pattern.search(stripped) for pattern in confirm_patterns(contract)
        ), (
            f"{contract['id']} fires on the doc comment {comment!r}; that is the "
            "exact failure this mechanism replaced"
        )


def test_a_diagnostic_ignore_is_tooling_and_a_bare_one_is_a_disabled_guard():
    """The oracle contract's whole subtlety, pinned.

    Two `#[ignore]`s in that file are bisection tools you run WHEN the oracle is
    red. Forbidding them outright is the noise that gets a guard waived; the
    reason string is what distinguishes opt-in tooling from a switched-off guard.
    """
    contract = next(
        c for c in ABSENCE_CONTRACTS if c["id"] == "rollback-exit-oracle-is-not-quarantined"
    )
    patterns = confirm_patterns(contract)

    tooling = '#[ignore = "diagnostic bisection: five sim boots"]'
    assert not any(p.search(tooling) for p in patterns), "opt-in tooling is allowed"

    quarantine = '#[ignore = "flaky, re-enable later"]'
    assert any(p.search(quarantine) for p in patterns), "a switched-off guard is not"


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_every_contract_holds_against_the_live_tree(contract):
    root = Path(__file__).resolve().parents[2]
    found = violations(contract, root)
    assert not found, (
        f"{contract['id']} is violated:\n"
        + "\n".join(f"  {path}:{number}: {text}" for path, number, text in found)
        + f"\n{contract['reason']}"
    )


# ── Dependency-edge contracts ───────────────────────────────────────────────
#
# The half a grep cannot express. Same rule as above: the live tree is green, so
# a test that only ran it would prove nothing. These feed synthetic graphs.

from check_absence_contracts import (  # noqa: E402
    DEPENDENCY_CONTRACTS,
    dependency_violations,
    reachable,
    workspace_graph,
)


def test_a_transitive_edge_is_a_violation_not_just_a_direct_one():
    """The claim is that a foundation cannot REACH gameplay.

    A layering inversion almost never arrives as a direct dependency line — it
    arrives through an intermediary that looked harmless. A checker that only
    read direct edges would pass the graph below, which is the graph that
    matters.
    """
    graph = {
        "ambition_platformer2d_shared_tangle": {"innocent_helper"},
        "innocent_helper": {"ambition_platformer2d_actor_monolith"},
        "ambition_platformer2d_actor_monolith": set(),
    }
    contract = {
        "id": "test",
        "crate": "ambition_platformer2d_shared_tangle",
        "forbidden": ["ambition_platformer2d_actor_monolith"],
        "reason": "",
    }
    found = dependency_violations(contract, graph)
    assert found == [
        "ambition_platformer2d_shared_tangle -> innocent_helper -> ambition_platformer2d_actor_monolith"
    ], f"the two-hop inversion was not reported: {found}"


def test_the_floor_contract_rejects_any_workspace_edge():
    graph = {"ambition_platformer2d_core": {"anything_at_all"}, "anything_at_all": set()}
    contract = {
        "id": "test",
        "crate": "ambition_platformer2d_core",
        "forbidden": "*",
        "reason": "",
    }
    assert dependency_violations(contract, graph) == [
        "ambition_platformer2d_core -> anything_at_all"
    ]


def test_a_clean_graph_reports_nothing():
    graph = {"a": {"b"}, "b": set(), "forbidden_crate": set()}
    contract = {
        "id": "test",
        "crate": "a",
        "forbidden": ["forbidden_crate"],
        "reason": "",
    }
    assert dependency_violations(contract, graph) == []


def test_a_contract_naming_a_crate_that_does_not_exist_is_reported():
    """A renamed crate must not turn its contract into a silent pass — that is
    the failure mode where a guard keeps printing `ok` about nothing."""
    found = dependency_violations(
        {"id": "test", "crate": "ghost_crate", "forbidden": ["x"], "reason": ""},
        {"a": set()},
    )
    assert found and "not a workspace member" in found[0]


def test_reachable_reports_the_shortest_path_it_found():
    graph = {"a": {"b", "c"}, "b": {"d"}, "c": set(), "d": set()}
    assert reachable(graph, "a")["d"] == ["a", "b", "d"]


@pytest.mark.parametrize(
    "contract", DEPENDENCY_CONTRACTS, ids=lambda c: c["id"]
)
def test_every_dependency_contract_holds_against_the_live_workspace(contract):
    graph = workspace_graph(Path(__file__).resolve().parents[2])
    found = dependency_violations(contract, graph)
    assert not found, (
        f"{contract['id']} is violated:\n"
        + "\n".join(f"  {path}" for path in found)
        + f"\n{contract['reason']}"
    )


# ── The module allowlist ────────────────────────────────────────────────────
#
# The third table permits a set and forbids the rest, so it has a failure mode
# the other two do not: it can be evaded by writing the SAME import in a
# different syntax. These probe the two invariants, the evasion, and the prose
# recurrence.

from check_absence_contracts import (  # noqa: E402
    MODULE_ALLOWLISTS,
    allowlist_usage,
    allowlist_violations,
    facade_modules,
    strip_comments_for as _strip,
)


def modules_in(source: str) -> set[str]:
    """The facade modules a snippet names, through the real pipeline.

    Comment-stripped line by line and rejoined, which is exactly what
    `allowlist_usage` does to a file — so a snippet that survives here is a
    snippet that would be reported there.
    """
    stripped = "\n".join(_strip("some/file.rs", line) for line in source.splitlines())
    return {module for _, module in facade_modules(stripped, "ambition_platformer2d")}


def test_the_allowlist_sees_an_ordinary_import():
    assert modules_in("use ambition_platformer2d::runtime::rollback::put_f32;") == {"runtime"}


def test_the_allowlist_sees_a_brace_grouped_import():
    """The evasion that a line regex misses, silently.

    `\\bambition_platformer2d::([a-z_]+)` matches neither name below — it reaches `{` and
    stops. The fixture contains no braced facade import today, so a line regex
    would have been green, and wrong the first time somebody wrote idiomatic
    Rust. This is the probe that the parser is not decoration.
    """
    assert modules_in("use ambition_platformer2d::{time::Clock, audio::Bank};") == {
        "time",
        "audio",
    }


def test_the_allowlist_sees_through_nesting_and_across_lines():
    source = """
    use ambition_platformer2d::{
        world::prelude::*,
        actors::{features::Body, ecs::Damage},
        input::Action,
    };
    """
    assert modules_in(source) == {"world", "actors", "input"}


def test_the_allowlist_does_not_count_self_as_a_module():
    assert modules_in("use ambition_platformer2d::{self, world::Room};") == {"world"}


def test_the_allowlist_does_not_confuse_a_crate_name_with_the_facade():
    """`ambition_platformer2d_actor_monolith::` is an engine crate, not the facade.

    An engine developer may name it; a consumer depending only on `ambition_platformer2d`
    cannot reach it. Matching it here would report leaks that do not exist and
    the contract would be waived within a week.
    """
    assert modules_in("use ambition_platformer2d_actor_monolith::features::Body;") == set()


def test_the_allowlist_stays_silent_on_prose_naming_a_module():
    """Documenting a leak must not register as the leak.

    Three absence checks in this repo went red on the docstring explaining the
    removal. The allowlist has the same exposure and more of it — the campaign
    documents are FULL of `ambition_platformer2d::runtime` — so it gets the same probe.
    """
    for comment in (
        "/// Outlander used to reach for `ambition_platformer2d::runtime::rollback`.",
        "//! LEAK CLOSED: no longer names ambition_platformer2d::time.",
        "    // was ambition_platformer2d::{audio, presentation}",
    ):
        assert modules_in(comment) == set(), f"fired on prose: {comment!r}"


def test_the_allowlist_catches_a_module_outside_the_reviewed_surface():
    """Invariant 1: the set may not GAIN a member."""
    contract = {
        "allowed": {"app"},
        "baseline": {"runtime"},
    }
    usage = {"app": [("x.rs", 1)], "runtime": [("x.rs", 2)], "combat": [("x.rs", 3)]}
    new, stale = allowlist_violations(contract, usage)
    assert new == ["combat"], f"a new leak was not reported: {new}"
    assert stale == []


def test_the_allowlist_catches_a_stale_baseline_entry():
    """Invariant 2 — the one that makes this a ratchet and not a budget.

    Without it, migrating `time` away while leaving `time` in the baseline
    frees a slot: the COUNT still reads 2, and the next module to appear
    occupies the gap without ever tripping invariant 1. `Freeze the SET`
    (api-growth-method.md §5) is only enforced if the set is pruned as it
    shrinks, so an unpruned entry has to be as red as a new one.
    """
    contract = {"allowed": set(), "baseline": {"runtime", "time"}}
    usage = {"runtime": [("x.rs", 1)]}
    new, stale = allowlist_violations(contract, usage)
    assert stale == ["time"], f"the stale baseline entry was not reported: {stale}"
    assert new == []


def test_a_pruned_module_can_never_come_back():
    """The two invariants composed, which is the property being bought.

    Once `time` is migrated and pruned, re-adding it is invariant 1's problem.
    A ratchet is only a ratchet if this holds.
    """
    contract = {"allowed": set(), "baseline": {"runtime"}}
    new, stale = allowlist_violations(
        contract, {"runtime": [("x.rs", 1)], "time": [("x.rs", 2)]}
    )
    assert new == ["time"] and stale == []


@pytest.mark.parametrize("contract", MODULE_ALLOWLISTS, ids=lambda c: c["id"])
def test_every_module_allowlist_holds_against_the_live_tree(contract):
    root = Path(__file__).resolve().parents[2]
    new, stale = allowlist_violations(contract, allowlist_usage(contract, root))
    assert not new and not stale, (
        f"{contract['id']} is violated: new={new} stale={stale}\n"
        f"{contract['reason']}"
    )


@pytest.mark.parametrize("contract", MODULE_ALLOWLISTS, ids=lambda c: c["id"])
def test_the_allowlist_baseline_is_not_silently_empty(contract):
    """A measurement bug reads exactly like a finished migration.

    If `allowlist_usage` stopped finding anything — a path typo, a renamed
    fixture, a regex that no longer matches — every invariant above passes and
    the campaign's progress metric reports ZERO, which is the success
    condition. That failure is green in the direction that feels good, so the
    non-vacuity is asserted rather than assumed.
    """
    root = Path(__file__).resolve().parents[2]
    usage = allowlist_usage(contract, root)
    assert usage, (
        f"{contract['id']} measured NO facade usage at all. Either the "
        "campaign is over, or the instrument is broken — check the second one "
        "first."
    )


# ── The central-rollback-ownership ratchet ──────────────────────────────────

from check_absence_contracts import (  # noqa: E402
    ROLLBACK_SCHEMA_BASELINE,
    rollback_schema_usage,
    rollback_schema_violations,
)


def test_the_rollback_ratchet_holds_against_the_live_tree():
    root = Path(__file__).resolve().parents[2]
    new, stale = rollback_schema_violations(root)
    assert not new and not stale, f"new={new} stale={stale}"


def test_the_rollback_ratchet_is_not_silently_empty():
    """A measurement bug reads exactly like a finished federation.

    If the extraction stopped matching — a moved file, a changed spelling — both
    invariants pass and the ratchet reports ZERO encoded types, which used to be
    read as its SUCCESS condition. Green in the direction that feels good, so it
    is asserted.

    ⚠ This test EARNED its keep on 2026-07-30. Slice F moved the snapshot trait
    to the floor and renamed the collector's key from `central_codecs` to
    `encoded_types`; this assertion is what failed, rather than a ratchet
    quietly freezing an empty set and reporting 19 of 19.
    """
    root = Path(__file__).resolve().parents[2]
    current = rollback_schema_usage(root)
    assert len(current["stable_schema_names"]) > 100, current["stable_schema_names"][:5]
    assert len(current["encoded_types"]) > 20, current["encoded_types"][:5]


def test_the_wire_format_is_encoded_where_the_types_live():
    """The carve's shape, asserted — not just its output count.

    Before slice F every `impl SnapshotState` was in ONE file in
    `ambition_platformer2d_runtime`, forced there by the orphan rule because the trait sat
    above every crate whose types it encoded. After the carve the impls live
    beside their types, and the count alone cannot tell those two worlds apart:
    63 impls in one file and 63 spread across nine crates both satisfy the
    assertion above.

    So the property to hold is the federation itself. If a future change pulls
    the trait back up the graph, the impls have to re-centralise to compile, and
    this fails — which is the only warning that would arrive before the next
    reader concludes a 2688-line codec file is simply how it must be.
    """
    root = Path(__file__).resolve().parents[2]
    encoded = rollback_schema_usage(root)["encoded_types"]
    crates = {entry.split("::")[0] for entry in encoded}
    assert len(crates) >= 5, (
        f"the rollback wire format has re-centralised into {sorted(crates)}. "
        "Slice F federated it across nine crates by moving `SnapshotState` into "
        "`ambition_platformer2d_core::snapshot`; a trait that moves back above the "
        "domains drags every impl with it."
    )
    assert "ambition_platformer2d_runtime" not in crates, (
        "`ambition_platformer2d_runtime` is encoding types again. It sits above twenty "
        "domain crates, so anything it encodes is a type some other crate owns "
        "— which is exactly the arrangement slice F removed."
    )


def test_the_rollback_ratchet_catches_a_new_central_registration(tmp_path, monkeypatch):
    """Invariant 1: central ownership may not GROW.

    Federating rollback means moving schema OUT of `ambition_platformer2d_runtime`. A new
    stable name appearing there is the migration running backwards.
    """
    import json

    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
    shrunk = dict(baseline)
    shrunk["stable_schema_names"] = baseline["stable_schema_names"][:-1]
    dropped = baseline["stable_schema_names"][-1]

    fake = tmp_path / "baseline.json"
    fake.write_text(json.dumps(shrunk))
    monkeypatch.setattr(
        contracts, "ROLLBACK_SCHEMA_BASELINE", str(fake.relative_to(tmp_path))
    )
    # `root` is only used to resolve the two source files plus the baseline, so
    # point the baseline lookup at the temp copy by faking the whole root.
    monkeypatch.setattr(
        contracts,
        "rollback_schema_usage",
        lambda _root: rollback_schema_usage(root),
    )
    new, stale = contracts.rollback_schema_violations(tmp_path)
    assert new == [f"stable_schema_names: {dropped}"], new
    assert stale == []


def test_the_rollback_ratchet_catches_an_unpruned_baseline(tmp_path, monkeypatch):
    """Invariant 2: a name that LEFT must be pruned in the migrating commit.

    Without this the baseline is a budget: federate one component out, leave it
    listed, and the freed slot can be reoccupied while the count still reads
    319.
    """
    import json

    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    baseline = json.loads((root / ROLLBACK_SCHEMA_BASELINE).read_text())
    grown = dict(baseline)
    grown["stable_schema_names"] = baseline["stable_schema_names"] + ["ghost.never_existed"]

    fake = tmp_path / "baseline.json"
    fake.write_text(json.dumps(grown))
    monkeypatch.setattr(
        contracts, "ROLLBACK_SCHEMA_BASELINE", str(fake.relative_to(tmp_path))
    )
    monkeypatch.setattr(
        contracts,
        "rollback_schema_usage",
        lambda _root: rollback_schema_usage(root),
    )
    new, stale = contracts.rollback_schema_violations(tmp_path)
    assert stale == ["stable_schema_names: ghost.never_existed"], stale
    assert new == []


# ── The capability-footprint ratchet ────────────────────────────────────────

from check_absence_contracts import (  # noqa: E402
    CAPABILITY_FOOTPRINT_BASELINE,
    capability_footprint_violations,
)


def test_the_footprint_ratchet_holds_against_the_live_tree():
    root = Path(__file__).resolve().parents[2]
    assert not capability_footprint_violations(root)


def test_the_footprint_ratchet_catches_a_new_linked_crate(tmp_path, monkeypatch):
    """The one invariant: the closure may not GROW.

    A new dependency edge anywhere under the facade enlarges what EVERY consumer
    links, and §2e exists because nothing else notices — no forbidden path is
    named and the module allowlist stays green.
    """
    import json

    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    # Capture the real closure BEFORE patching, or the stub recurses into itself.
    real_closure = contracts.sentinel_linked_closure(root)

    baseline = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
    dropped = baseline["ambition_closure"][-1]
    shrunk = dict(baseline, ambition_closure=baseline["ambition_closure"][:-1])

    fake = tmp_path / "footprint.json"
    fake.write_text(json.dumps(shrunk))
    monkeypatch.setattr(
        contracts, "CAPABILITY_FOOTPRINT_BASELINE", str(fake.relative_to(tmp_path))
    )
    monkeypatch.setattr(
        contracts, "sentinel_linked_closure", lambda _root: real_closure
    )

    assert contracts.capability_footprint_violations(tmp_path) == [dropped]


def test_the_footprint_baseline_is_not_silently_empty():
    """A measurement bug reads as a finished decomposition.

    If the graph walk ever returns nothing, the ratchet passes and the campaign's
    §2e counter reports ZERO unwanted crates — its success condition. Green in
    the flattering direction, so it is asserted.
    """
    import json

    root = Path(__file__).resolve().parents[2]
    baseline = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
    assert baseline["closure_size"] > 20, baseline["closure_size"]
    assert baseline["never_asked_for_count"] > 0, baseline["never_asked_for_count"]
