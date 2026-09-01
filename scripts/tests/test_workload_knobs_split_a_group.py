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


# ── where the knobs are READ from ────────────────────────────────────────────
#
# ⛔⛔ **THE FIELDS ABOVE WERE DERIVED FROM THE CENSUS, AND THE PROFILER HAS A
# `--no-census` FLAG.** So this was reachable:
#
#     AMBITION_ACTOR_POPULATION_CAP=16 scripts/profile_desktop.sh --no-census ...
#
# No `census_sim_phases.csv` exists, every knob reads `None`, and the modified
# workload hashes straight back into the ordinary uncapped group — exactly the
# history error these fields were added to prevent. An experiment's identity must
# not depend on the instrument used to measure it.

import tempfile  # noqa: E402
from pathlib import Path as _Path  # noqa: E402

import pytest  # noqa: E402


def bundle_with(census_row: dict | None):
    """A bundle directory that either has a sim-phase census or does not."""
    tmp = tempfile.mkdtemp()
    if census_row is not None:
        cols = ["wall_s", "t", "ticks", *census_row.keys()]
        vals = ["1.0", "0.5", "100", *census_row.values()]
        _Path(tmp, "census_sim_phases.csv").write_text(
            ",".join(cols) + "\n" + ",".join(str(v) for v in vals) + "\n"
        )
    return hist.Bundle(tmp)


def test_a_no_census_capture_still_carries_the_knobs():
    facts = hist.workload_facts(
        bundle_with(None),
        {"actor_population_cap": "16", "actor_brain_override": "ambition::melee_brute_striker"},
    )
    assert facts["actor_cap"] == "16", (
        "--no-census must not return a capped run to the uncapped group"
    )
    assert facts["brain_override"] == "ambition::melee_brute_striker"


def test_a_bundle_older_than_the_metadata_falls_back_to_the_census_row():
    """Premise guard: the census road must keep working for existing bundles."""
    facts = hist.workload_facts(bundle_with({"actor_cap": "64"}), {})
    assert facts["actor_cap"] == "64"


def test_an_ordinary_capture_records_no_knobs_at_all():
    """Premise guard: absent must stay absent, or every row re-keys."""
    facts = hist.workload_facts(bundle_with(None), {})
    assert facts == {"actor_cap": None, "brain_override": None, "brain_profile": None}


def test_a_launcher_census_disagreement_refuses_to_ingest():
    """The configured process is not the process that ran. That is not a preference."""
    with pytest.raises(ValueError, match="does not describe the run"):
        hist.workload_facts(
            bundle_with({"actor_cap": "64"}), {"actor_population_cap": "16"}
        )


# ── the frame cap ────────────────────────────────────────────────────────────
#
# ⛔⛔ **A PACED FRAME IS NOT A FREE ONE, AND NOTHING RECORDED WHICH IT WAS.**
# `FramePaceCap::Auto` is the DEFAULT — "caps to the display refresh (battery
# saver); `Off` renders unthrottled" — so every capture taken before 2026-09-01
# was throttled, and no bundle said so. Tracy put
# `bevy_framepace::framerate_limiter` at 4.61% of the frame, which is how it was
# finally noticed.
#
# Comparing a capped run against an uncapped one measures the SETTING, not the
# code, and the ledger's whole job is to refuse that.


def with_cap(cap):
    rec = record()
    rec["quality"] = {"profile": "Ultra", "frame_cap": cap}
    return rec


def test_a_capped_run_and_an_uncapped_one_are_different_experiments():
    assert key(with_cap("Auto")) != key(with_cap("Off")), (
        "a paced frame and an unthrottled one must not share a group"
    )


def test_the_label_names_the_cap_so_two_groups_do_not_print_alike():
    assert "cap:Auto" in label(with_cap("Auto"))
    assert "cap:Off" in label(with_cap("Off"))


def test_a_capture_that_recorded_no_cap_keeps_its_old_label():
    """⛔ PREMISE GUARD. Every row written before the census carried a cap has
    `None` here; stamping a default onto them would rewrite history and split
    them from each other."""
    assert "cap:" not in label(record())
