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

import json
import re
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

REPO = Path(__file__).resolve().parents[2]

from check_absence_contracts import (  # noqa: E402
    ABSENCE_CONTRACTS,
    strip_comments_for,
    violations,
)

# ⭐ THREE CONTRACT FAMILIES, GUARDED THREE DIFFERENT WAYS — AND THAT ASYMMETRY
# IS PRINCIPLED, not an oversight anyone should tidy. Checked 2026-09-02:
#
#   ABSENCE_CONTRACTS (25)   each carries its OWN patterns, so each can be
#                            individually wrong and each needs its own fixture.
#                            That is why VIOLATING_LINE is parametrized over all
#                            of them, in both directions.
#   DEPENDENCY_CONTRACTS (6) share ONE mechanism (graph reachability) and differ
#                            only in data. Proving the algorithm fires once
#                            proves it for all six, so the fire tests are
#                            synthetic graphs and the per-contract test is the
#                            LIVE one — plus `a_contract_naming_a_crate_that_
#                            does_not_exist_is_reported`, which is what stops a
#                            renamed crate turning a contract into a silent pass.
#   MODULE_ALLOWLISTS (4)    share a mechanism too, but permit-a-set-and-forbid-
#                            the-rest can be EVADED by re-spelling an import, so
#                            they get evasion tests AND a per-contract vacuity
#                            control (`..._baseline_is_not_silently_empty`).
#
# ⇒ Do not "make them consistent". A per-contract fire test for the dependency
# family would restate one algorithm six times; the absence family cannot borrow
# that argument because there is no shared algorithm to prove.

# One line that each contract must reject, written the way real code would.
#
# ⭐ THE EIGHT BELOW WERE ADDED 2026-09-02, closing the last of the skips. A
# contract with no line here is SKIPPED by both tests in this file — so it is
# asserted green against the live tree and has never been shown capable of
# firing at all. Eight of twenty-five were in that state; the tree being clean
# is exactly what makes a never-fired pattern indistinguishable from a working
# one.
VIOLATING_LINE = {
    "only-the-candidate-builder-hides-a-root":
        "    ambition_platformer2d_shared_tangle::construction::hide_candidate_session_root(&mut commands, root);",
    "only-the-publication-authority-publishes-a-candidate":
        "    ambition_platformer2d_shared_tangle::construction::publish_candidate(world, &t);",
    "the-two-move-drivers-do-not-author-their-own-presses":
        "    if frame.attack_pressed { return Ok(()); }",
    "the-recorders-do-not-resolve-their-own-combat-geometry":
        "    let volumes = world.resource::<DamageableVolumes>();",
    "ending-a-move-goes-through-the-one-teardown-path":
        "        commands.entity(body).remove::<MovePlayback>();",
    "the-brain-codec-names-the-fighter-only-through-the-enum-variant":
        "        let decoded = fighter::BrainState::decode(bytes)?;",
    "the-character-domain-is-not-named-after-a-character":
        "pub const PLAYER_ROBOT_REACH: f32 = 48.0;",
    "the-character-fold-is-not-a-public-capability":
        "pub fn finalize_cast(world: &mut World) -> StagedCharacter { todo!() }",
    "the-actor-mirrors-stay-deleted":
        "    let intent = ActorIntent::default();",
    "no-build-legacy-body-then-patch-it":
        "    adopt_character_intrinsics(&mut body, &spec);",
    "the-generic-brain-does-not-grow-new-platform-fighter-edges":
        "    let cfg: fighter::FighterCfg = todo!();",
    "player-input-frame-mirror-does-not-return": "pub struct PlayerInputFrame;",
    "central-rollback-does-not-enumerate-domains":
        '    app.rollback_component_clone::<ambition_portal2d::PortalBody>(ENGINE, "portal.body");',
    "the-global-roster-is-retired-only-by-its-owner":
        "            commands.remove_resource::<MatchParticipantRoster>();",
    "the-seat-topology-has-one-engine-side-creator":
        "        commands.insert_resource(ambition_input::LocalSeatTopology::default());",
    "a-second-writer-of-a-match-global-must-answer-ownership":
        "            commands.remove_resource::<ActiveMatch>();",
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
        "    let p = provider_of_character(registry, id);",
    "the-catalog-owners-map-is-not-a-provider-authority":
        "            .and_then(|owners| owners.provider_for(id))",
    "the-catalog-only-motion-model-resolver-stays-deleted":
        "    let m = motion_model_spec_for_character_id(catalog, id);",
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
# and a prose check that only ever tested `//` would have proved
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


# AGENTS.md names this class by name: *"Poison tests are for realistic harmful states, not for
# proving that every scanner detects its own fixture."*
#
# That is not covered by the live run — only 3 of the 17 contracts have any text in the tree
# that their grep prefilter hits and the comment-stripper then discards, so 14 of them have never
# had their prose path exercised by anything but this.
@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_every_path_a_contract_names_still_exists(contract):
    """⛔⛔ A CARVE MOVES THE FILE AND THE CONTRACT KEEPS THE OLD PATH.

    Eleven of these contracts pin *"this belongs to ONE file"* by EXCLUDING that
    file by path (`:!crates/…/presentation.rs`). Both halves rot silently when a
    carve moves code, and they rot in opposite directions:

      an INCLUDE path that no longer exists  -> the contract scans NOTHING and
                                                passes forever. Vacuity.
      an EXCLUDE path that no longer exists  -> the owner it protected has moved,
                                                and the exclusion now excuses a
                                                path nobody writes to. The rule
                                                looks intact and guards a ghost.

    ⚠ THIS IS NOT THE LOUD CASE. A carve that moves the owner usually makes the
    contract flag the NEW location, which is red and obvious. This catches the
    quiet one: the exclusion left behind, pointing at nothing. The durable path→contract map lives in
    `docs/planning/engine/actor-monolith-decomposition.md` so a queue cleanup cannot
    erase the carve safety rule.

    ⇒ When this fails, MOVE the path. Widening it to a directory, or deleting
    the contract, launders the rule the carve was supposed to preserve.
    """
    for raw in contract.get("paths", []):
        path = re.sub(r"^:(!|\(exclude\))", "", raw)
        assert (REPO / path).exists(), (
            f"{contract['id']} names `{raw}`, which does not exist. If a carve "
            "moved it, point this entry at the new path in the same commit — do "
            "not widen it and do not drop it."
        )


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_every_contract_fires_on_a_line_that_violates_it(contract):
    """⛔⛔ THIS TEST DID NOT EXIST, AND THE MODULE DOCSTRING SAID IT DID.

    The header above promises two things — *"feed each contract a line that
    VIOLATES it and require a hit"* and *"feed each contract that same text as a
    DOC COMMENT and require silence"*. Only the second was implemented.
    `VIOLATING_LINE` was read in exactly one place, the silence test, so **no
    entry in it had ever been checked for actually violating anything**.

    Found 2026-09-02 by poisoning: changing `ActorIntent` to
    `SomethingHarmless` in the table left the whole file green at 87 passed. A
    fixture that no longer matches its pattern is indistinguishable from one
    that does, which makes the silence test's green mean less than it reads:
    prose is silent for a pattern that never fires either.

    ⇒ Together the two directions are the guard: this one says the pattern
    catches the thing it forbids, the next says it stays quiet on prose
    describing the removal. Neither is worth much alone.
    """
    line = VIOLATING_LINE.get(contract["id"])
    if line is None:
        pytest.skip(f"{contract['id']} states no violating line")
    assert any(pattern.search(line) for pattern in confirm_patterns(contract)), (
        f"{contract['id']}'s fixture {line!r} matches none of its patterns, so "
        "the silence test below is comparing prose against a pattern that never "
        "fires — and the contract has never been shown able to catch anything"
    )


@pytest.mark.parametrize("contract", ABSENCE_CONTRACTS, ids=lambda c: c["id"])
def test_no_contract_fires_on_prose_describing_the_removal(contract):
    """Documenting a removal must not break the guard that verified it."""
    # a contract with no fixture is SKIPPED, not a failure — that tax is what made this file
    # red. Adding an architectural contract must not require writing prose for a meta-test
    # first; what a fixture buys, when somebody writes one, is the check below.
    line = VIOLATING_LINE.get(contract["id"])
    if line is None:
        pytest.skip(f"{contract['id']} states no violating line to comment out")
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
    """Documenting a leak must not register as the leak."""
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
    """Invariant 2: baseline entries must be pruned when usage disappears.

    Otherwise a vacated slot could be reused without increasing the baseline
    count, turning the set ratchet into a budget.
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
    """The allowlist census must remain non-vacuous.

    An empty census could mean a broken path or matcher while all ratchets appear
    green, so the measurement itself is asserted.
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
    """The rollback-schema census must remain non-vacuous.

    A broken extractor could otherwise report an empty, apparently green wire
    format. Assert both the encoded-type and peer-visible populations directly.
    """
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    current = rollback_schema_usage(root)
    assert len(current["encoded_types"]) > 20, current["encoded_types"][:5]
    # The NAMES moved owner on 2026-09-16: a source scan was 15% blind about the
    # rows peers compare, so the runtime dump owns them and this baseline holds
    # only the peer-visible slice of it. Floor that slice here, in the same test
    # whose whole job is refusing an apparently green empty census.
    version, rows = contracts.peer_checksum_schema(root)
    assert len(rows) > 100, rows[:5]
    assert version.startswith("ggrs-rollback-schema-v"), version


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
    shrunk["encoded_types"] = baseline["encoded_types"][:-1]
    dropped = baseline["encoded_types"][-1]

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
    assert new == [f"encoded_types: {dropped}"], new
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
    grown["encoded_types"] = baseline["encoded_types"] + ["ghost_crate::NeverExisted"]

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
    assert stale == ["encoded_types: ghost_crate::NeverExisted"], stale
    assert new == []


# ── The peer-visible schema may not move without the version ────────────────
#
# The rows two peers actually compare are the ones whose kind answers
# `RollbackEntryKind::feeds_peer_checksum()`. If that set changes and
# `GGRS_ROLLBACK_SCHEMA_VERSION` does not, both builds advertise the same
# mechanical identity while checksumming different state: they will not refuse
# each other before play, they will diverge during it.


def _fake_tree(tmp_path, version, rows):
    """A minimal root holding just the dump the invariant reads."""
    dump = tmp_path / "dump.txt"
    dump.write_text(f"{version}\n" + "\n".join(rows) + "\n")
    return dump


def test_the_peer_visible_schema_ratchet_holds_against_the_live_tree():
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    assert contracts.peer_checksum_schema_violations(root) == []


def test_the_frozen_prior_is_at_the_live_version_or_this_ratchet_compares_nothing():
    """⛔⛤ **THE ARM ABOVE WAS VACUOUS FOR EIGHT SCHEMA VERSIONS, AND GREEN.**

    `peer_checksum_schema_violations` compares row sets only when
    `frozen["version"] == version`; a version bump is the whole permission to
    move rows. So the moment a commit bumps `GGRS_ROLLBACK_SCHEMA_VERSION` and
    does not RE-FREEZE this baseline, the live-tree arm stops comparing
    anything and keeps passing — the baseline was last frozen at `v194` on
    2026-09-16 and the tree reached `v202`, with three rows entering the peer
    checksum unwitnessed in between.

    ⚠ The synthetic arms below could not see it: each builds its own baseline
    at the live version, so they exercise the comparison the real tree had
    stopped performing. A guard's unit tests are not a claim about the corpus
    it guards.

    ⇒ Re-freezing IS the maintenance this invariant costs: bump the version,
    re-record the rows. This arm is what says so out loud.
    """
    import json

    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    frozen = json.loads(
        (root / contracts.ROLLBACK_SCHEMA_BASELINE).read_text()
    )["peer_checksum_schema"]
    version, _rows = contracts.peer_checksum_schema(root)
    assert frozen["version"] == version, (
        f"the frozen prior is at {frozen['version']} and the tree is at {version}, so "
        "`test_the_peer_visible_schema_ratchet_holds_against_the_live_tree` compares "
        "nothing and passes. Re-freeze: set both `version` and `rows` in "
        f"{contracts.ROLLBACK_SCHEMA_BASELINE} from the live dump."
    )


def test_a_checksum_feeding_row_that_lands_without_a_version_bump_is_caught(
    tmp_path, monkeypatch
):
    """⛔ The failure this invariant exists for."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, rows = contracts.peer_checksum_schema(root)
    dump = _fake_tree(
        tmp_path, version, rows + ["poison.new_canonical\tcomponent-canonical\tPoison"]
    )
    baseline = tmp_path / "baseline.json"
    baseline.write_text(
        json.dumps({"peer_checksum_schema": {"version": version, "rows": rows}})
    )
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_DUMP", dump.name)
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_BASELINE", baseline.name)
    kinds = contracts.checksum_feeding_kinds(root)
    monkeypatch.setattr(contracts, "checksum_feeding_kinds", lambda _root: kinds)
    breaches = contracts.peer_checksum_schema_violations(tmp_path)
    assert breaches == [
        f"ENTERED the peer checksum at {version}: "
        "poison.new_canonical\tcomponent-canonical\tPoison"
    ], breaches


def test_the_same_row_with_the_version_moved_is_allowed(tmp_path, monkeypatch):
    """⭐ THE POSITIVE CONTROL FOR THE LEGITIMATE ROAD.

    Without this the invariant could be satisfied by forbidding all change, which
    is the shrink-only reading that `resource.impact_hitstop` already settled
    once for the wire-format ratchet next door.
    """
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, rows = contracts.peer_checksum_schema(root)
    dump = _fake_tree(
        tmp_path,
        "ggrs-rollback-schema-v999",
        rows + ["poison.new_canonical\tcomponent-canonical\tPoison"],
    )
    baseline = tmp_path / "baseline.json"
    baseline.write_text(
        json.dumps({"peer_checksum_schema": {"version": version, "rows": rows}})
    )
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_DUMP", dump.name)
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_BASELINE", baseline.name)
    kinds = contracts.checksum_feeding_kinds(root)
    monkeypatch.setattr(contracts, "checksum_feeding_kinds", lambda _root: kinds)
    assert contracts.peer_checksum_schema_violations(tmp_path) == []


def test_a_row_feeding_no_checksum_may_land_without_a_version_bump(
    tmp_path, monkeypatch
):
    """The deliberate case, which the history exercised twice.

    `derived.attacker_move_instance` and `smash.body_mark` both landed with the
    version held, and `ambition_mount`'s registration documents the reasoning:
    a registration nothing hashes changes what a rewind restores locally and
    nothing that crosses the wire. An invariant that reddened for those would
    train people to bump the version meaninglessly, which is how a fingerprint
    stops meaning anything.
    """
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, rows = contracts.peer_checksum_schema(root)
    dump = _fake_tree(
        tmp_path, version, rows + ["poison.new_clone\tcomponent-clone\tPoison"]
    )
    baseline = tmp_path / "baseline.json"
    baseline.write_text(
        json.dumps({"peer_checksum_schema": {"version": version, "rows": rows}})
    )
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_DUMP", dump.name)
    monkeypatch.setattr(contracts, "ROLLBACK_SCHEMA_BASELINE", baseline.name)
    kinds = contracts.checksum_feeding_kinds(root)
    monkeypatch.setattr(contracts, "checksum_feeding_kinds", lambda _root: kinds)
    assert contracts.peer_checksum_schema_violations(tmp_path) == []


def test_a_narrowed_projection_moves_the_slice_and_a_reworded_kind_does_not():
    """⛔⛤ THE FIRST VERSION OF THIS SLICE DROPPED `detail` AND WAS BLIND TO 48
    OF THE 144 ROWS.

    A `resource-clone-custom-checksum` row's `detail` records what its
    `fn(&T) -> u64` actually covers, and 22 of them say 22 different things — so
    narrowing a projection moves no name, no kind and no type. Keeping `detail`
    everywhere is the opposite error: 85 `component-canonical` rows share one
    sentence, and `Q122` measured one pluralised word moving 83 rows.

    ⭐ THE CONTROL AND THE POSITIVE DIFFER ONLY IN THEIR SUBJECT — both reword a
    sentence in the dump, and the only difference is whether that sentence
    distinguishes rows of its kind. Without the control this test would pass for
    a slice that simply hashed everything.
    """
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    _, rows = contracts.peer_checksum_schema(root)
    with_detail = [row for row in rows if row.count("\t") == 3]
    assert 20 < len(with_detail) < len(rows), (
        f"{len(with_detail)} of {len(rows)} rows carry their detail; there were "
        "48 of 144 when this was written. All of them means the prose tax is "
        "back, none means a narrowed projection is invisible."
    )
    kinds_with = {row.split("\t")[1] for row in with_detail}
    kinds_without = {row.split("\t")[1] for row in rows if row.count("\t") == 2}
    assert not (kinds_with & kinds_without), (
        "a kind carries its detail on some rows and not others; the rule is "
        "per-KIND, so this means the census read two different dumps"
    )
    # The load-bearing ones: every kind whose rows describe their own projection.
    assert "resource-clone-custom-checksum" in kinds_with, sorted(kinds_with)
    # And the boilerplate one, which must NOT be paying the prose tax.
    assert "component-canonical" in kinds_without, sorted(kinds_without)


# ── The peer INPUT payload may not move without its identity ────────────────
#
# The other half of the wire. `ggrs` documents `Config::Input` as "the only
# game-related data transmitted over the network", and `AmbitionGgrsConfig =
# GgrsConfig<ControlFrame>` — so `ControlFrame`'s declaration IS the peer input
# format. Until 2026-09-16 nothing versioned its shape: `INPUT_STREAM_VERSION`
# covers recorded replay files and exempts added fields BY DESIGN, the rollback
# dump carries one row naming the TYPE, and the codec-shape baseline has zero
# mentions because `ControlFrame` is `derived` rather than snapshotted.


def test_the_input_payload_ratchet_holds_against_the_live_tree():
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    assert contracts.input_payload_violations(root) == []


def test_the_input_payload_census_is_not_silently_empty():
    """⭐ THE VACUITY CONTROL. Every assertion next door is `not <difference>`,
    and a census that stopped finding fields makes all of them pass."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, shape = contracts.input_payload_shape(root)
    assert len(shape) >= 40, (
        f"only {len(shape)} rows in the peer input payload; there were 42 when "
        "this was written (39 `ControlFrame` fields and 3 `AttackStrengthHint` "
        "variants), so the parser has stopped finding them"
    )
    assert version.isdigit(), version
    # The field ORDER is part of the shape, because bincode encodes positionally
    # and carries no field names. A census returning a set would not notice a
    # reorder, which changes what every byte after it means.
    assert shape[0] == "axis_x: f32", shape[:3]


def test_a_field_added_without_bumping_the_identity_is_caught(tmp_path, monkeypatch):
    """⛔ The failure this ratchet exists for, and the one about to happen:
    SETTINGS-ROLLBACK's recommended repair adds a `ControlFrame` field."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, shape = contracts.input_payload_shape(root)
    baseline = tmp_path / "input.json"
    baseline.write_text(json.dumps({"version": version, "shape": shape}))
    monkeypatch.setattr(contracts, "INPUT_PAYLOAD_BASELINE", baseline.name)
    monkeypatch.setattr(
        contracts,
        "input_payload_shape",
        lambda _root: (version, shape + ["poison_new_field: bool"]),
    )
    assert contracts.input_payload_violations(tmp_path) == [
        f"ENTERED the peer input payload at identity {version}: poison_new_field: bool"
    ]


def test_the_same_field_with_the_identity_bumped_is_allowed(tmp_path, monkeypatch):
    """⭐ THE POSITIVE CONTROL FOR THE LEGITIMATE ROAD. Without it the ratchet
    could be satisfied by forbidding all change, which would block the very
    repair that made this guard worth building."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    version, shape = contracts.input_payload_shape(root)
    baseline = tmp_path / "input.json"
    baseline.write_text(json.dumps({"version": version, "shape": shape}))
    monkeypatch.setattr(contracts, "INPUT_PAYLOAD_BASELINE", baseline.name)
    monkeypatch.setattr(
        contracts,
        "input_payload_shape",
        lambda _root: (str(int(version) + 1), shape + ["poison_new_field: bool"]),
    )
    assert contracts.input_payload_violations(tmp_path) == []


def test_an_unfollowed_field_type_raises_instead_of_reading_green():
    """⛔⛤ THE TRANSITIVE BOUNDARY, ASSERTED RATHER THAN ASSUMED COMPLETE.

    A field whose type is not primitive can change the bincode shape without
    `ControlFrame`'s own text moving. Exactly one such type exists today and its
    variants are censused; a second appearing must FAIL, because the alternative
    is a guard that silently stops covering the thing it is named after.
    """
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    _, shape = contracts.input_payload_shape(root)
    followed = [row for row in shape if row.startswith("AttackStrengthHint::")]
    assert len(followed) >= 3, shape
    types = {row.split(": ", 1)[1] for row in shape if ": " in row}
    # ⭐ THIS SET GREW ON 2026-09-16, WHICH IS THE BOUNDARY WORKING RATHER THAN
    # LEAKING: SETTINGS-ROLLBACK moved the seat's frame policy onto
    # `ControlFrame`, the census refused until `ControlFrameModes` was followed,
    # and following it added `InputFrameMode` too. A type enters here only after
    # someone has taught the census its shape.
    assert types <= {
        "bool",
        "f32",
        "AttackStrengthHint",
        "crate::ControlFrameModes",
        "InputFrameMode",
    }, sorted(types)


def test_a_data_carrying_variant_is_recorded_rather_than_skipped():
    """⛔⛤ THE HOLE THE GPT ARCHITECTURE REVIEW OF 2026-09-16 POISONED OPEN.

    The variant scan was `^\\s*(\\w+),` — a bare identifier and a comma — so
    adding `Charged(u8)` to `AttackStrengthHint` changed the peer input encoding
    and this census reported the same three rows and NO violation. The byte pin
    next door does not necessarily catch it either: its corpus never constructs
    the new variant, and a developer adding one would repair the exhaustive Rust
    matches as a matter of course, leaving the tree green with the recorded input
    identity unbumped. ⇒ The closure claim *"a silent payload-shape change is
    impossible"* was false while this held.
    """
    import check_absence_contracts as contracts

    body = """
    /// A doc comment, which the scan must not read as a variant.
    #[default]
    Auto,
    Tilt,
    Charged(u8),
    Aimed { x: f32, y: f32 },
    Smash,
"""
    rows = contracts.serialized_variants("E", body)
    assert rows == [
        "E::Auto",
        "E::Tilt",
        "E::Charged(u8)",
        "E::Aimed { x: f32, y: f32 }",
        "E::Smash",
    ], rows


def test_a_data_carrying_variant_is_refused_because_the_encoding_must_be_fixed_width():
    """⭐⭐ A REFUSAL RATHER THAN A RECORDING, and the reason is the transport.

    Ambition packs every local player's frame into one payload and the receiving
    side divides the total byte count evenly among the player count. A variant
    whose payload differs from its siblings' makes one frame's width depend on
    what the player pressed, so the subdivision lands in the wrong places with no
    checksum anywhere to notice.
    """
    import check_absence_contracts as contracts

    assert contracts.variants_carrying_data(["E::Auto", "E::Smash"]) == []
    assert contracts.variants_carrying_data(
        ["E::Auto", "E::Charged(u8)", "E::Aimed { x: f32 }"]
    ) == ["E::Charged(u8)", "E::Aimed { x: f32 }"]


def test_the_live_input_enums_are_all_fieldless():
    """The refusal above, asked of the tree rather than of a fixture."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    _, shape = contracts.input_payload_shape(root)
    variants = [
        row
        for row in shape
        if "::" in row and ": " not in row.split("::", 1)[1]
    ]
    assert len(variants) >= 6, variants
    assert contracts.variants_carrying_data(variants) == []


def test_the_printed_total_equals_the_contracts_actually_printed():
    """⛔⛤ THE TOTAL IS HAND-COUNTED, AND THE HAND WAS WRONG FOR A DAY.

    `total` sums three collections plus a literal for the hand-emitted contracts.
    A ratchet landed 2026-09-16 without bumping that literal, so the lane printed
    "44 of 44" over 45 contracts — and a total nobody derives is a total nobody
    checks. This derives it the other way, from the lines the run actually
    emitted, so the two can only agree by being right.
    """
    import subprocess
    import sys

    root = Path(__file__).resolve().parents[2]
    run = subprocess.run(
        [sys.executable, str(root / "scripts" / "check_absence_contracts.py")],
        capture_output=True,
        text=True,
        cwd=root,
    )
    printed = [
        line for line in run.stdout.splitlines() if line.startswith(("  ok   ", "  RED  "))
    ]
    claimed = re.search(r"(\d+) of (\d+) absence contracts", run.stdout)
    assert claimed, run.stdout[-2000:]
    assert len(printed) == int(claimed.group(2)), (
        f"the lane claims {claimed.group(2)} contracts and printed "
        f"{len(printed)}. The hand-counted literal in `total` has drifted from "
        "the contracts actually emitted; name the new one there."
    )
    # ⭐ AND A FLOOR, because 0 == 0 satisfies the line above.
    assert len(printed) > 40, len(printed)


def test_the_checksum_feeding_kinds_are_read_from_the_source_not_a_list():
    """⛔⛤ A HAND-KEPT COPY OF THIS SET HID 25 OF 29 REGISTRATIONS ONCE.

    `id_peer_audit.rs` named six variants and omitted every `*CustomChecksum`
    kind, so the three checkpoint resources that write a raw `SessionScopeId`
    into their projection were never examined by the guard whose whole subject
    is host-local identity reaching a peer comparison. This asserts the set is
    still derived, and — the part that matters — that a kind whose label goes
    missing RAISES instead of silently dropping its rows and reading green.
    """
    import re

    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    feeding = contracts.checksum_feeding_kinds(root)
    text = (root / "crates/ambition_platformer2d_core/src/rollback_kind.rs").read_text()
    true_arm = text.split("pub fn feeds_peer_checksum")[1].split("=> true")[0]
    assert len(feeding) == len(set(re.findall(r"Self::(\w+)", true_arm))) >= 6, feeding
    # every kind in the frozen dump slice is one of them, and the reverse cannot
    # be asserted: `resource-clone-cursor` feeds the checksum and has no member
    # today, which is a legal state and not a broken filter.
    version, rows = contracts.peer_checksum_schema(root)
    assert {row.split("\t")[1] for row in rows} <= feeding
    assert len(rows) > 100, len(rows)


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
    """The capability-footprint graph census must remain non-vacuous."""
    import json

    root = Path(__file__).resolve().parents[2]
    baseline = json.loads((root / CAPABILITY_FOOTPRINT_BASELINE).read_text())
    assert baseline["closure_size"] > 20, baseline["closure_size"]
    assert baseline["never_asked_for_count"] > 0, baseline["never_asked_for_count"]


# ⛔⛔ THE TWO BASELINE-BACKED CONTRACTS HAD NO TEST IN THIS FILE, and that gap
# shipped a red main on 2026-09-06. `check_absence_contracts.py` reports 38
# contracts; the three families this file parametrises cover 36 of them, and the
# two that keep a JSON BASELINE instead of a pattern were covered by nothing here.
#
# ⇒ THE CONSEQUENCE, exactly as it happened: registering `ControlClaims` for
# rollback put a new type on the wire. `cargo test --workspace` was green, the
# `rollback_schema_baseline.txt` row was added, the schema version was bumped —
# and `rollback-wire-format-changes-are-declared` was RED for its own separate
# baseline, which only a lane nobody in that session ran would have said. A peer
# found it.
#
# ⚠ THE LESSON IS ABOUT LANES, NOT ABOUT ROLLBACK. A checker reachable only from
# a lane I assemble by hand is a checker with no skip-list to read past: nothing
# announces that it did not run. These two run wherever `pytest scripts/tests/`
# runs, which is the lane everything else here already lives in.
def test_the_rollback_wire_format_baseline_is_declared_and_current():
    """Every type on the rollback wire is in the baseline, and every baseline row
    still describes a live type."""
    from check_absence_contracts import rollback_schema_violations

    new, stale = rollback_schema_violations(REPO)
    assert not new, (
        "these entered the rollback wire format without being declared — add the "
        "baseline row AND bump the schema version, or keep the type off the "
        "wire:\n  " + "\n  ".join(new)
    )
    assert not stale, (
        "these are declared in the wire-format baseline and no longer on the "
        "wire; prune them in the same commit that removed them, or the baseline "
        "stops describing the format:\n  " + "\n  ".join(stale)
    )


def test_the_wire_format_baseline_is_not_silently_empty():
    """⭐ THE VACUITY CONTROL. Both assertions above are `not <difference>`, and a
    baseline that failed to load — or a collector that stopped finding encoded
    types — makes both differences empty and both assertions pass."""
    from check_absence_contracts import ROLLBACK_SCHEMA_BASELINE

    baseline = json.loads((REPO / ROLLBACK_SCHEMA_BASELINE).read_text())
    assert len(baseline["peer_checksum_schema"]["rows"]) >= 140, (
        f"only {len(baseline['peer_checksum_schema']['rows'])} rows feed the "
        "peer checksum in the baseline; there were 144 when this was written, so "
        "the file has been truncated rather than the format having shrunk"
    )
    assert len(baseline["encoded_types"]) >= 130, (
        f"only {len(baseline['encoded_types'])} encoded types; there were 137 "
        "when this was written — 129 of them before the census was widened past "
        "`crates/` to see `game/`'s nine boss-special impls"
    )


def test_the_capability_footprint_contract_holds_against_the_live_tree():
    """The other baseline-backed contract, covered here for the same reason."""
    from check_absence_contracts import capability_footprint_violations

    found = capability_footprint_violations(REPO)
    assert not found, "capability footprint grew:\n  " + "\n  ".join(found)


# ── The featureless-facade closure contract ─────────────────────────────────
#
# ⛔⛔ THE THIRD BASELINE-FREE CONTRACT, AND IT SHIPPED WITHOUT A RED PROBE. Every
# other family in this file has one; `the-featureless-facade-links-none-of-these`
# arrived on 2026-09-09 with only its own `cargo tree` behind it, which is the
# exact gap the comment above records shipping a red main once already.
#
# ⭐ AND IT IS D-BUILD-GRAPH-BLINDNESS'S ACCEPTANCE, in the one place the
# distinction is decidable: DECLARED-BUT-UNUSED, FEATURE-GATED and ACTUALLY
# LINKED are three different answers, and a measurement that conflates them is
# worse than none. `cargo metadata`'s resolve graph says 61 ambition crates for
# this facade; `cargo tree -e normal --no-default-features` says 49, and the
# 12-crate gap is entirely optional edges no feature enables. The contract must
# ask the second question, and these tests are what say it still does.


def test_the_featureless_closure_holds_against_the_live_tree():
    from check_absence_contracts import featureless_facade_report

    present, missing = featureless_facade_report(REPO)
    assert not missing, (
        "the featureless closure is missing crates the facade names "
        f"unconditionally, so the measurement itself is broken: {missing}"
    )
    assert not present, f"a capability this profile promises is absent is linked: {present}"


def test_the_featureless_closure_catches_a_forbidden_crate(monkeypatch):
    """The red probe: a name in the forbidden set that IS in the closure reports.

    Patched against the LIVE closure rather than a stub, so the test fails if the
    tree stops containing the crate it names — a probe against a fabricated
    closure would keep passing after the measurement stopped working.
    """
    import check_absence_contracts as contracts

    live = contracts.featureless_facade_closure(REPO)
    # A crate that IS linked with every feature off — the floor's own subject.
    linked = contracts.FEATURELESS_FACADE_FLOOR[0]
    assert linked in live, (
        f"`{linked}` is not in the featureless closure, so this probe would "
        "report nothing and prove nothing"
    )
    monkeypatch.setattr(contracts, "FEATURELESS_FACADE_FORBIDS", (linked,))
    present, missing = contracts.featureless_facade_report(REPO)
    assert not missing
    assert present == [linked]


def test_the_featureless_closure_reports_a_broken_instrument_rather_than_ok(monkeypatch):
    """⭐ THE ANTI-VACUITY CONTROL, and it is the failure mode this contract has by
    construction: it asserts an ABSENCE, so a `cargo tree` that fails — a renamed
    package, an output-format change, a stale lockfile — yields an empty set,
    which contains no forbidden crate and would print `ok` having measured
    nothing."""
    import check_absence_contracts as contracts

    monkeypatch.setattr(contracts, "featureless_facade_closure", lambda _root: set())
    present, missing = contracts.featureless_facade_report(REPO)
    assert not present
    assert missing == list(contracts.FEATURELESS_FACADE_FLOOR)


def test_the_featureless_closure_is_not_silently_empty():
    """The census must stay big enough to be the subject this contract names."""
    import check_absence_contracts as contracts

    live = contracts.featureless_facade_closure(REPO)
    assert len(live) >= 40, (
        f"only {len(live)} ambition crates in the featureless closure; there "
        "were 49 when this was written, so the walk has been truncated rather "
        "than the closure having shrunk that far"
    )


def test_a_variable_width_field_is_refused_even_with_the_identity_bumped():
    """⛔⛤ THE SECOND REVIEW'S POISON, AND THE DISTINCTION IT TURNS ON.

    The enum half was closed on 2026-09-16 and the same reviewer walked through
    the nested struct the next pass: `poison_optional: Option<bool>` on
    `ControlFrameModes`, WITH `CONTROL_FRAME_WIRE_IDENTITY` bumped, produced no
    violation. The census recorded the field and the ratchet accepted it, because
    a recorded change plus a bumped identity is the legitimate road.

    ⭐⭐ A WIRE IDENTITY BUMP BUYS A DIFFERENT FIXED-WIDTH PROTOCOL, NOT A
    VARIABLE-WIDTH ONE. Ambition concatenates every local player's frame into one
    payload and the receiving side divides the received total evenly by the player
    count, so `None` encoding shorter than `Some(false)` subdivides the packet in
    the wrong places however carefully the change was announced.
    """
    import check_absence_contracts as contracts

    followed = set(contracts._FOLLOWED_FIELD_TYPES)
    # The legitimate shapes pass.
    contracts.refuse_variable_width(
        "ControlFrameModes", [("movement", "InputFrameMode"), ("axis", "f32")], followed
    )
    for field in [
        ("poison_optional", "Option<bool>"),
        ("poison_list", "Vec<u8>"),
        ("poison_text", "String"),
        # ⛔ AND AN UNRECOGNISED TYPE, because a guard that accepts what it cannot
        # classify is the same hole in a politer costume.
        ("poison_unknown", "SomeTypeNobodyTaughtThisCensus"),
    ]:
        with pytest.raises(AssertionError) as caught:
            contracts.refuse_variable_width("ControlFrameModes", [field], followed)
        assert "FIXED-WIDTH" in str(caught.value)
        assert "BUMPING" in str(caught.value), "the message must name the bump it refuses"


def test_the_live_payload_is_fixed_width_at_every_level():
    """The refusal above, asked of the tree rather than of a fixture."""
    import check_absence_contracts as contracts

    root = Path(__file__).resolve().parents[2]
    _, shape = contracts.input_payload_shape(root)
    assert len(shape) >= 40, shape
