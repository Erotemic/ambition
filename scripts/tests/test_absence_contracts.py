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
    "registration-does-not-demand-art": "    CharacterLoadDemand::request(&mut demand, id);",
    "no-string-keyed-sheet-row-lookup": "    let row = sheet.row_index_of(name)?;",
    "rollback-exit-oracle-is-not-quarantined": "#[ignore]",
    "fight-tests-do-not-hand-roll-damage": "    mary_hp -= 12;",
    "one-reader-of-the-catalog-default-action-set":
        "    let authored = catalog.build_default_action_set(id);",
    "one-caller-of-the-provider-resolver":
        "    let p = provider_of_character(registry, owners, id);",
    "one-caller-of-the-motion-model-resolver":
        "    let m = motion_model_spec_for_character(registry, catalog, id);",
    "one-reader-of-the-catalog-axis-tuning":
        "    match catalog.axis_tuning(id) {",
    "one-place-builds-the-worlds-path":
        '    DEFAULT_LDTK = REPO_ROOT / "assets" / "worlds" / "sandbox.ldtk"',
}

# The language each contract's subject is written in, which decides what a
# COMMENT looks like when the prose check strips one. Rust unless stated: the
# harness assumed Rust everywhere until the first Python contract arrived
# (2026-07-28), and a prose check that only ever tested `//` would have proved
# nothing about a `#`.
CONTRACT_LANGUAGE = {
    "one-place-builds-the-worlds-path": "py",
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
        "ambition_platformer_primitives": {"innocent_helper"},
        "innocent_helper": {"ambition_actors"},
        "ambition_actors": set(),
    }
    contract = {
        "id": "test",
        "crate": "ambition_platformer_primitives",
        "forbidden": ["ambition_actors"],
        "reason": "",
    }
    found = dependency_violations(contract, graph)
    assert found == [
        "ambition_platformer_primitives -> innocent_helper -> ambition_actors"
    ], f"the two-hop inversion was not reported: {found}"


def test_the_floor_contract_rejects_any_workspace_edge():
    graph = {"ambition_engine_core": {"anything_at_all"}, "anything_at_all": set()}
    contract = {
        "id": "test",
        "crate": "ambition_engine_core",
        "forbidden": "*",
        "reason": "",
    }
    assert dependency_violations(contract, graph) == [
        "ambition_engine_core -> anything_at_all"
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
