"""The steps after a scenario's build are timed one by one, and a failed step
stops the measurement (B7: measure the whole edit/build/test/load loop)."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compile_cost  # noqa: E402


def scenario(*then):
    return compile_cost.Scenario(name="t", edit="x", command=["true"], why="test", then=then)


def test_each_phase_has_its_own_seconds_and_peak() -> None:
    row = compile_cost.run_phases(
        scenario(("one", (sys.executable, "-c", "pass")), ("two", (sys.executable, "-c", "pass"))),
        {}, "after_edit", verbose=False,
    )
    assert set(row) == {
        "after_edit_one_seconds", "after_edit_one_peak_rss_bytes",
        "after_edit_two_seconds", "after_edit_two_peak_rss_bytes",
    }
    assert all(row[f"after_edit_{name}_seconds"] >= 0 for name in ("one", "two"))


def test_a_failed_phase_is_not_a_timing() -> None:
    with pytest.raises(SystemExit, match="failed, so its timing is meaningless"):
        compile_cost.run_phases(scenario(("bad", (sys.executable, "-c", "raise SystemExit(3)"))), {}, "warm_noop", verbose=False)


def test_the_edit_cycle_scenario_tests_and_loads_after_its_build() -> None:
    cycle = compile_cost.BY_NAME["edit-cycle"]
    assert [name for name, _ in cycle.then] == ["targeted_test", "first_frame"]
    assert "--no-run" in cycle.command, "the timed build is the build alone; the test is its own phase"
