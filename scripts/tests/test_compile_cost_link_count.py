"""`compile_cost`'s host-link counter, which is the half of a row that no
duration can state.

M0 wants to tell "a content edit rebuilt one small crate" apart from "a content
edit rebuilt one small crate AND relinked the host". Those two can have the same
wall clock on a warm cache, so the count is not a decoration on the timing — it
is the only column that separates them.

These arms cover the two ways the count goes silently wrong: counting cached
units, and mistaking a library for an executable.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from compile_cost import (  # noqa: E402
    LINK_COUNT_FLAG,
    count_host_links,
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
