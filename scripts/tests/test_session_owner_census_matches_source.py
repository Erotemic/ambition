"""The C03 census guard must run in a lane, and must be able to FAIL.

⛔ A guard with no test beside it runs in no lane — that happened in this
repository on 2026-09-16 to a checker that was committed, correct, and executed
by nothing.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_session_owner_census_matches_source as guard  # noqa: E402


def test_the_census_matches_source_today():
    assert guard.main() == 0


def test_the_marker_is_the_authority_the_guard_reads():
    """⚠ The prose can be reworded; the marker is what must not drift silently."""
    stated = guard.declared()
    assert set(stated) == {
        "SessionScopedResources",
        "SessionOwnedCheckpointState",
        "SessionMechanics",
    }, stated
    assert sum(stated.values()) >= 30, stated


def test_the_bundle_count_is_read_from_source_not_the_marker():
    """⛔ THE CONTROL. If this returned the marker's number the guard could never

    disagree with anything — the failure mode where a comparison compares a value
    with itself.
    """
    real = guard.bundle_members(
        guard.BUNDLES["SessionScopedResources"], "SessionScopedResources"
    )
    assert real == guard.declared()["SessionScopedResources"]
    assert real > 3, "the ResMut field pattern stopped matching; the scan is broken"


def test_session_mechanics_is_counted_as_one_resource_not_six_fields():
    """⚠ THE MISTAKE THIS GUARD RECORDS. It is ONE resource with six fields, and

    counting fields reads the total as 41 instead of 36.
    """
    assert guard.declared()["SessionMechanics"] == 1
