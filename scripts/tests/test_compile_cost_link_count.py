"""`compile_cost`'s two controls: the host-link counter and the baseline check.

Both exist because a row of durations cannot police itself.

M0 wants to tell "a content edit rebuilt one small crate" apart from "a content
edit rebuilt one small crate AND relinked the host". Those two can have the same
wall clock on a warm cache, so the count is not a decoration on the timing — it
is the only column that separates them.

These arms cover the two ways the count goes silently wrong — counting cached
units, and mistaking a library for an executable — and the baseline control that
refuses a row whose "warm" no-op did real work.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import compile_cost  # noqa: E402
from compile_cost import (  # noqa: E402
    LINK_COUNT_FLAG,
    count_host_links,
    count_rebuilt_units,
    instrumented_for_link_count,
)


def artifact(name: str, *, fresh: bool, executable: str | None, kind: str = "lib") -> str:
    return json.dumps(
        {
            "reason": "compiler-artifact",
            "fresh": fresh,
            "executable": executable,
            "target": {"name": name, "kind": [kind]},
        }
    )


def test_a_cached_unit_is_not_a_link() -> None:
    """The defect this is really guarding: a warm build re-reports EVERY unit.

    Counting artifacts rather than non-fresh artifacts turns "this build did
    nothing" into a large constant, and the constant looks plausible because it
    is the size of the dependency graph.
    """
    stream = "\n".join(
        artifact(f"dep{index}", fresh=True, executable="/t/debug/dep") for index in range(50)
    )
    assert count_host_links(stream) == 0


def test_a_freshly_linked_executable_counts() -> None:
    stream = "\n".join(
        [
            artifact("serde", fresh=True, executable=None),
            artifact("ambition_app", fresh=False, executable=None),
            artifact("app_it", fresh=False, executable="/t/debug/deps/app_it-abc"),
        ]
    )
    assert count_host_links(stream) == 1


def test_a_library_is_not_told_from_a_test_binary_by_its_kind() -> None:
    """MEASURED on `ambition_entity_catalog`: cargo reported its linked test
    binary as `kind: ["rlib"]`, identical to the library built beside it.

    A counter keyed on `kind` therefore either counts both or neither. Only
    `executable` separates them, and this arm fails the moment someone
    "simplifies" the check back to a kind test.
    """
    stream = "\n".join(
        [
            artifact("entity_catalog", fresh=False, executable=None, kind="rlib"),
            artifact("entity_catalog", fresh=False, executable="/t/debug/deps/ec-1", kind="rlib"),
        ]
    )
    assert count_host_links(stream) == 1


def test_a_check_build_links_nothing() -> None:
    """`cargo check` emits artifacts with a null `executable` throughout, so the
    lane's count is 0 — which is a RESULT (the gate never links), not a gap."""
    stream = "\n".join(
        artifact(f"unit{index}", fresh=False, executable=None) for index in range(12)
    )
    assert count_host_links(stream) == 0


def test_progress_lines_and_a_truncated_message_do_not_become_a_count() -> None:
    """Cargo's human output and a half-written line must both be skipped rather
    than crashing or counting. A build killed mid-write is already being
    reported as a failure; it must not ALSO corrupt the number."""
    stream = "\n".join(
        [
            "   Compiling ambition_app v0.1.0",
            artifact("app_it", fresh=False, executable="/t/debug/deps/app_it-abc"),
            '{"reason": "compiler-artifact", "fresh": false, "exec',
            "    Finished `test` profile",
        ]
    )
    assert count_host_links(stream) == 1


def test_the_flag_is_appended_and_the_command_is_otherwise_untouched() -> None:
    command = ["cargo", "check", "-p", "ambition_app"]
    instrumented, countable = instrumented_for_link_count(command)
    assert countable is True
    assert instrumented == [*command, LINK_COUNT_FLAG]
    assert command == ["cargo", "check", "-p", "ambition_app"], "must not mutate the caller's list"


def test_a_scenario_that_chose_its_own_format_keeps_it_and_reports_unmeasured() -> None:
    """Overriding it would measure a different command than the scenario asked
    for. `None` means unmeasured, which is true; a number would be a lie about
    which command produced it."""
    command = ["cargo", "check", "--message-format=short"]
    instrumented, countable = instrumented_for_link_count(command)
    assert countable is False
    assert instrumented == command


# ── the baseline control (added 2026-09-16) ────────────────────────────────
#
# Three rows on one day recorded a "warm" no-op that had done real work — 346 s,
# 47 s, 7.25 s against a 0.73 s floor — always because a merge landed between
# building the subject and measuring it. Every duration in such a row times a
# different build than the row claims, and nothing in the durations says so.


def test_a_warm_no_op_rebuilds_nothing_and_that_is_the_test() -> None:
    """⭐ ZERO REBUILT UNITS, NOT A DURATION THRESHOLD.

    A threshold is per-machine and per-lane; "compiled nothing" is exact on any
    host. It also covers `cargo check`, which links nothing whether the baseline
    was warm or stone cold — so `host_link_invocations` cannot serve as the
    control there.
    """
    warm = "\n".join(artifact(f"dep{i}", fresh=True, executable=None) for i in range(40))
    assert count_rebuilt_units(warm) == 0
    assert count_host_links(warm) == 0

    did_work = warm + "\n" + artifact("ambition_app", fresh=False, executable=None)
    assert count_rebuilt_units(did_work) == 1
    assert count_host_links(did_work) == 0, (
        "a check lane links nothing, so only the unit count can see that it rebuilt"
    )


def test_the_refusal_stops_the_run_rather_than_warning() -> None:
    """⛔ A RULE THAT ASKS A READER TO CHECK THE CONTROL FIRST DOES NOT HOLD.

    Once `after_edit_seconds` has been read, an odd baseline gets EXPLAINED
    rather than discarded — the explanation is always available and always
    plausible. So the instrument must refuse to produce the interesting term.
    """
    scenario = compile_cost.SCENARIOS[0]
    contaminated = compile_cost.BuildCost(
        seconds=47.33, peak_rss_bytes=None, host_link_invocations=0, units_rebuilt=44
    )
    with pytest.raises(SystemExit) as raised:
        compile_cost.refuse_a_baseline_that_is_not_one(scenario, contaminated)
    message = str(raised.value)
    assert "44" in message, "the refusal must say how much work the baseline did"
    assert "not a baseline" in message


def test_a_genuine_baseline_is_not_refused() -> None:
    warm = compile_cost.BuildCost(
        seconds=0.78, peak_rss_bytes=None, host_link_invocations=0, units_rebuilt=0
    )
    compile_cost.refuse_a_baseline_that_is_not_one(compile_cost.SCENARIOS[0], warm)


def test_an_unmeasurable_lane_is_not_refused_for_being_unmeasurable() -> None:
    """`None` means UNMEASURED, never zero — a scenario that chose its own
    `--message-format` must not be refused for a control it could not report."""
    unknown = compile_cost.BuildCost(
        seconds=0.78, peak_rss_bytes=None, host_link_invocations=None, units_rebuilt=None
    )
    compile_cost.refuse_a_baseline_that_is_not_one(compile_cost.SCENARIOS[0], unknown)
