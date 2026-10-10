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
        "after_edit_one_seconds", "after_edit_one_peak_rss_bytes", "after_edit_one_units_rebuilt",
        "after_edit_two_seconds", "after_edit_two_peak_rss_bytes", "after_edit_two_units_rebuilt",
    }
    assert row["after_edit_one_units_rebuilt"] is None, "a phase that is not cargo has no unit count"
    assert all(row[f"after_edit_{name}_seconds"] >= 0 for name in ("one", "two"))


def test_a_failed_phase_is_not_a_timing() -> None:
    with pytest.raises(SystemExit, match="failed, so its timing is meaningless"):
        compile_cost.run_phases(scenario(("bad", (sys.executable, "-c", "raise SystemExit(3)"))), {}, "warm_noop", verbose=False)


def test_the_edit_cycle_scenario_tests_and_loads_after_its_build() -> None:
    cycle = compile_cost.BY_NAME["edit-cycle"]
    assert [name for name, _ in cycle.then] == ["targeted_test", "first_frame_build", "first_frame"]
    assert "--no-run" in cycle.command, "the timed build is the build alone; the test is its own phase"
    phases = dict(cycle.then)
    assert phases["first_frame"][0] != "cargo", "the drawing is timed without the build of its binary"


def test_a_warm_phase_that_compiled_is_not_a_control() -> None:
    cycle = scenario(("targeted_test", ("cargo",)), ("first_frame", ("draw",)))
    compile_cost.refuse_a_phase_control_that_rebuilt(
        cycle, {"warm_noop_targeted_test_units_rebuilt": 0, "warm_noop_first_frame_units_rebuilt": None}
    )
    with pytest.raises(SystemExit, match="warm `targeted_test` phase REBUILT 3 unit"):
        compile_cost.refuse_a_phase_control_that_rebuilt(cycle, {"warm_noop_targeted_test_units_rebuilt": 3})


def test_a_cargo_phase_reports_the_units_it_compiled(monkeypatch) -> None:
    seen = []

    def fake_run_timed(command, env):
        seen.append(command)
        return compile_cost.BuildCost(seconds=1.0, peak_rss_bytes=2, host_link_invocations=0, units_rebuilt=5)

    monkeypatch.setattr(compile_cost, "run_timed", fake_run_timed)
    row = compile_cost.run_phases(scenario(("t", ("cargo", "test", "--", "filter"))), {}, "warm_noop", verbose=False)
    assert row["warm_noop_t_units_rebuilt"] == 5
    assert seen == [["cargo", "test", "--", "filter"]]


def test_the_count_flag_goes_before_the_arguments_of_the_test_binary() -> None:
    instrumented, countable = compile_cost.instrumented_for_link_count(["cargo", "test", "--", "filter"])
    assert countable
    assert instrumented == ["cargo", "test", compile_cost.LINK_COUNT_FLAG, "--", "filter"]
