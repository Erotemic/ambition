"""The single-runner ratchet must run in a lane, and must be able to FAIL.

⛔⛤ A GUARD WITH NO TEST BESIDE IT RUNS IN NO LANE. This file is what makes
`pytest scripts/tests` execute the guard, and it poisons the pattern directly,
because a source-text check whose regex has stopped matching reports exactly
what a clean tree reports.

⚠ The tree-is-clean arm and the pattern arms answer DIFFERENT questions. The
first can go red for a reason that is a finding about the repository; the rest
stay true whatever the repository does, which is what makes them a control on
the instrument rather than on the tree.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_commit_only_schedules_have_one_runner as guard  # noqa: E402


def test_every_commit_only_schedule_has_one_runner_today():
    assert guard.main() == 0


def test_the_pattern_matches_the_bare_and_the_qualified_call():
    """⚠ Both spellings are the same call, and only one of them is obvious."""
    pattern = guard.runner_pattern("CheckpointDomainApply")
    assert pattern.search("world.try_run_schedule(CheckpointDomainApply);")
    assert pattern.search(
        "world.try_run_schedule(ambition_platformer2d_shared_tangle::lifecycle::CheckpointDomainApply)"
    )
    assert pattern.search("world.run_schedule(lifecycle::CheckpointDomainApply);")


def test_a_longer_label_with_the_same_prefix_is_not_this_schedule():
    """⛔ THE ONE THAT A `\\b`-LESS PATTERN GETS WRONG, and it fails OPEN.

    `CheckpointDomainApplyLater` matching as `CheckpointDomainApply` would report
    a stray runner that does not exist — noisy, but safe. The dangerous
    direction is the reverse, so the suffix boundary is asserted here rather
    than assumed.
    """
    pattern = guard.runner_pattern("CheckpointDomainApply")
    assert not pattern.search("world.try_run_schedule(CheckpointDomainApplyLater);")


def test_running_a_different_schedule_is_not_a_hit():
    pattern = guard.runner_pattern("CheckpointDomainApply")
    assert not pattern.search("world.try_run_schedule(CheckpointCapture);")


def test_the_sanctioned_runner_is_a_path_this_repository_still_has():
    """⛔ A contract naming a file that no longer exists is a guard that cannot
    fail for the right reason: every real call site becomes a 'stray'."""
    for label, (sanctioned, _why) in guard.COMMIT_ONLY.items():
        assert (guard.REPO / sanctioned).is_file(), f"{label} names a missing {sanctioned}"
