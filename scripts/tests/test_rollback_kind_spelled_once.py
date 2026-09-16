"""The spelling guard must run in a lane, and must be able to FAIL.

⛔⛤ A GUARD WITH NO TEST BESIDE IT RUNS IN NO LANE. That happened to
`check_consolidation_ledger_still_resolves.py` on 2026-09-16 -- committed,
correct, and executed by nothing. This file is what makes `pytest scripts/tests`
run the guard, and it also poisons it, because a source-text check that reports
clean over a corpus it can no longer see is indistinguishable from a clean tree.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_rollback_kind_spelled_once as guard  # noqa: E402


def test_the_tree_is_collapsed_today():
    assert guard.main() == 0


def test_the_pattern_still_recognises_the_duplication():
    """⛔ The KNOWN-ANSWER control: the exact shape both roads used to carry."""
    found = guard.offenders(
        "            RollbackEntryKind::ComponentClone,\n"
        "            detail::CLONE_UNHASHED,\n"
    )
    assert found == [("ComponentClone", "CLONE_UNHASHED")], found


def test_a_caller_supplied_detail_is_not_flagged():
    """⚠ The `*_custom_checksum` family has no literal sentence to collapse."""
    assert guard.offenders(
        "            RollbackEntryKind::ComponentCloneCustomChecksum,\n"
        "            detail,\n"
    ) == []
