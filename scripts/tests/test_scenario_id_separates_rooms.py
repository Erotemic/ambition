"""Two headless captures of DIFFERENT rooms must not share a comparability key.

⛔⛔ **THEY DID.** `scripts/profile_desktop.sh` set `headless_scenario` to the
literal string `caller-specified` for every headless run with custom arguments,
and `scenario.id` is one of `COMPARABILITY_FIELDS`. So a
`--start-room hall_of_characters` capture and a `--start-room goblin_encounter`
capture produced the SAME key, and `perf_history.py compare` would subtract one
frame time from the other and report the difference as a regression or a win.

The refusal that tool is designed around — *"two records whose comparability
fields differ must not be subtracted, and the refusal must NAME the field"* —
cannot fire on a field that never recorded the difference.

⭐ The windowed path was always correct: it derives `windowed:<passthrough>`. The
headless path now mirrors it.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_to_history as history  # noqa: E402


def key_for(scenario_id: str) -> str:
    """The comparability key a record with this scenario id would carry."""
    record = {
        "scenario": {"id": scenario_id, "version": 1, "headless": True},
        "build": {
            "cargo_profile": "profiling",
            "features": [],
            "package": "ambition_app",
            "binary": "ambition_game_bin",
        },
        "host": {"machine_id": "m", "cpu_model": "cpu", "logical_cpus": 12},
        "gpu": {"rendering": "headless", "adapter": None},
        "display": {"resolution": "headless"},
        "instruments": {"tracy": False, "perf": True, "census": True, "census_hz": 1.0},
    }
    key, _fields = history.comparability(record)
    return key


def test_two_rooms_do_not_share_a_comparability_key():
    hall = key_for("headless:--start-room+hall_of_characters")
    goblin = key_for("headless:--start-room+goblin_encounter")
    assert hall != goblin, (
        "two different rooms produced one comparability group; the ledger will "
        "subtract one room's frame time from another's and call it a change"
    )


def test_the_same_room_still_groups_with_itself():
    """Premise guard: separating rooms must not separate a room from ITSELF.

    A key derived from something run-specific — a timestamp, a path, the tick
    count — would pass the arm above while making every capture its own group,
    so nothing could ever be compared to anything.
    """
    a = key_for("headless:--start-room+hall_of_characters")
    b = key_for("headless:--start-room+hall_of_characters")
    assert a == b, "the same scenario must land in the same group across runs"


def test_scenario_id_is_still_a_comparability_field():
    """The whole defect was a field that did not record what it named.

    If `scenario.id` were dropped from `COMPARABILITY_FIELDS`, the two arms above
    would both pass — the first by accident of some other field, the second
    trivially — and rooms would silently merge again.
    """
    assert "scenario.id" in history.COMPARABILITY_FIELDS
