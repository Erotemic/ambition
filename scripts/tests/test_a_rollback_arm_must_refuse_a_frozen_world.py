"""The sync-test arm accounting, against corpora with known answers."""

from __future__ import annotations

import importlib.util
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]


def _load():
    name = "a_rollback_arm_must_refuse_a_frozen_world"
    spec = importlib.util.spec_from_file_location(name, REPO / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


CHECK = _load()


def test_the_population_is_found_by_the_fixture_call():
    arms = CHECK.sync_test_arms()
    assert arms, "no sync-test fixture found at all"
    # The one the original census could not see: it lives outside `crates/` and
    # `game/`, which is where that sweep looked.
    assert "examples/capability_demo/tests/rollback_round_trip.rs" in arms


def test_every_arm_is_checked_or_accounted_for():
    arms = CHECK.sync_test_arms()
    loose = sorted(
        rel
        for rel, health in arms.items()
        if not health and rel not in CHECK.ADJUDICATED and rel not in CHECK.NOT_AN_ARM
    )
    assert not loose, (
        "sync-test rollback arm(s) that neither read the session's health nor "
        f"carry a reading of what a frozen world breaks in them: {loose}"
    )


def test_no_adjudication_or_exemption_outlives_its_file():
    """⚠ A row for a file that no longer builds a sync-test session excuses nothing.

    Worse, it hides the next one: a reader seeing a long list assumes it was
    derived from the tree.
    """
    arms = set(CHECK.sync_test_arms())
    stale = sorted((set(CHECK.ADJUDICATED) | set(CHECK.NOT_AN_ARM)) - arms)
    assert not stale, f"rows naming files that no longer build one: {stale}"


def test_a_health_call_satisfies_the_accounting():
    assert CHECK.sync_test_arms(["scripts/tests/fixtures_do_not_exist.rs"]) == {}


def test_the_adjudications_all_name_a_mechanism():
    """⛔ A row whose reason is blank is a waiver wearing a decision's clothes."""
    for rel, reason in CHECK.ADJUDICATED.items():
        assert len(reason) > 20, f"{rel} carries no mechanism: {reason!r}"
