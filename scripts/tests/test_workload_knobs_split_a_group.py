"""A capped or re-brained room must not share a comparability group with the real one.

⛔⛔ **THE CENSUS ROW SAID IT AND THE LEDGER DID NOT.** `runtime_census.rs`
appends `actor_cap=` to the sim-phase row specifically so *"a reader quoting a
number will not go looking for the environment it was taken in"*. That warning
reached the prose summary and never reached `COMPARABILITY_FIELDS`, so a 16-body
scaling arm and the shipped 130-body hall hashed to the same
`comparable_key` — the ledger's own answer to "is this the same experiment" was
yes.

`AMBITION_ACTOR_BRAIN_OVERRIDE` is worse, because it changes what every body in
the room *does*: a hall of stand-still statues and a hall of fighters are the
same scenario id, the same host, the same build.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import profile_bundle_to_history as hist  # noqa: E402


def record(**workload) -> dict:
    """A minimal record; only the workload block varies between arms."""
    base = {
        "scenario": {"id": "headless:--start-room+hall_of_characters", "version": 1},
        "build": {"cargo_profile": "profiling", "features": []},
        "gpu": {"rendering": "headless"},
        "host": {"machine_id": "ec9af5ee"},
    }
    if workload:
        base["workload"] = workload
    return base


def key(rec: dict) -> str:
    return hist.comparability(rec)[0]


def label(rec: dict) -> str:
    return hist.comparable_label(hist.comparability(rec)[1])


def test_a_capped_run_is_not_the_shipped_room():
    full = record(actor_cap=None, brain_override=None)
    capped = record(actor_cap="16", brain_override=None)
    assert key(full) != key(capped), (
        "a 16-body scaling arm and the 130-body hall must not hash to one group"
    )


def test_a_rebrained_room_is_not_the_authored_one():
    statues = record(actor_cap=None, brain_override=None)
    fighters = record(actor_cap=None, brain_override="fighter")
    assert key(statues) != key(fighters), (
        "a hall of stand-still statues and a hall of fighters are not one experiment"
    )


def test_two_arms_at_the_same_cap_still_group():
    """Premise guard: without this, hashing the run id would 'pass' every test above."""
    assert key(record(actor_cap="16")) == key(record(actor_cap="16"))


def test_a_record_written_before_the_knobs_existed_keys_LIKE_AN_UNCAPPED_RUN():
    """⛔⛔ **THIS IS THE ONE THAT LETS THE FIELD BE ADDED AT ALL.**

    `comparability` hashes every field including the `None`s, so a new field
    normally re-keys the whole ledger and orphans every series — the smash run
    had six rows that would stop grouping with their own successors.

    They do not orphan only because `dig` reads a missing path as `None` and the
    knobs are absent from an ordinary capture: an old record recomputes to
    exactly the key a new uncapped run gets. The existing rows are re-keyed in
    place on that basis, so if this ever stops holding, the migration was wrong.
    """
    ancient = record()  # no "workload" block whatsoever
    modern = record(actor_cap=None, brain_override=None)
    assert key(ancient) == key(modern)


def test_the_label_names_the_knob_when_set_and_is_unchanged_when_not():
    assert "/cast:" not in label(record()), (
        "an ordinary capture keeps the label it has always had"
    )
    assert "/cast:cap16" in label(record(actor_cap="16"))
    assert "/cast:cap16+fighter" in label(record(actor_cap="16", brain_override="fighter"))
    assert "/cast:fighter" in label(record(brain_override="fighter"))
