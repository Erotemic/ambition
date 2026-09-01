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


def label_for(**overrides) -> str:
    fields = {
        "scenario.id": "windowed:default",
        "scenario.version": 1,
        "scenario.headless": False,
        "build.cargo_profile": "profiling",
        "build.features": [],
        "build.package": "ambition_app",
        "build.binary": "ambition_game_bin",
        "host.machine_id": "m",
        "host.cpu_model": "cpu",
        "host.logical_cpus": 12,
        "gpu.rendering": "hardware",
        "gpu.adapter": "a",
        "display.resolution": "1600x900",
        "instruments.tracy": False,
        "instruments.perf": True,
        "instruments.census": True,
        "instruments.census_hz": 1.0,
    }
    fields.update(overrides)
    return history.comparable_label(fields)


def test_two_groups_that_differ_never_share_a_label():
    """⛔⛔ Three groups printed as ONE identical heading in the generated report.

    `windowed:default@v1/profiling/hardware/no-tracy/087907b3...` appeared three
    times. They differ by whether the Tracy instrumentation was COMPILED IN
    (`['profile']` vs `[]` — which `no-tracy` does not say, since that flag is
    about whether Tracy ATTACHED) and by 3200x1800 vs 1600x900, which is exactly
    the weak-GPU framebuffer-scale experiment.

    A label that cannot distinguish two groups is worse than a hash: the hash at
    least looks opaque, so nobody trusts it as a name.
    """
    assert label_for(**{"display.resolution": "3200x1800"}) != label_for()
    assert label_for(**{"build.features": ["profile"]}) != label_for()


def test_identical_groups_still_share_a_label():
    """Premise guard: distinguishing must not make every row unique.

    Folding something run-specific into the label would pass the arm above and
    make the report a list of one-row groups.
    """
    assert label_for() == label_for()


def key_with_quality(**overrides) -> str:
    fields = {
        "scenario.id": "windowed:default",
        "scenario.version": 1,
        "scenario.headless": False,
        "build.cargo_profile": "profiling",
        "build.features": [],
        "build.package": "ambition_app",
        "build.binary": "ambition_game_bin",
        "host.machine_id": "m",
        "host.cpu_model": "cpu",
        "host.logical_cpus": 12,
        "gpu.rendering": "hardware",
        "gpu.adapter": "a",
        "display.resolution": "1600x900",
        "quality.profile": "High",
        "quality.parallax_max_layers": "unbounded",
        "quality.msaa_samples": "4",
        "quality.max_scale_factor": "compositor",
        "instruments.tracy": False,
        "instruments.perf": True,
        "instruments.census": True,
        "instruments.census_hz": 1.0,
    }
    fields.update(overrides)
    record = {}
    for path, value in fields.items():
        head, tail = path.split(".", 1)
        record.setdefault(head, {})[tail] = value
    key, _ = history.comparability(record)
    return key


def test_the_quality_tier_splits_the_group():
    """⛔⛔ The tier changed measured drawn area 23x and was in NO record.

    `potato` draws 631,267 world-units² of sprite in `water_world`; `high` draws
    14,564,876. Nothing carried the tier — not the bundle metadata, not a census
    row, not the comparability key — so two captures of one room at two tiers
    were the same experiment as far as this ledger could tell.

    ⚠ And the tier is not one knob. It resolves parallax layer count, MSAA
    samples and the DPI cap together, which are the very variables `D-RASTER-3`
    is trying to separate.
    """
    high = key_with_quality()
    assert key_with_quality(**{"quality.profile": "Potato"}) != high
    assert key_with_quality(**{"quality.msaa_samples": "1"}) != high
    assert key_with_quality(**{"quality.parallax_max_layers": "2"}) != high
    assert key_with_quality(**{"quality.max_scale_factor": "1"}) != high


def test_an_unrecorded_tier_still_groups_with_itself():
    """Premise guard: every row taken before the tier was recorded has `None`.

    Those must stay comparable to each other, or adding this field would orphan
    the entire existing ledger instead of splitting it.
    """
    a = key_with_quality(**{"quality.profile": None, "quality.msaa_samples": None})
    b = key_with_quality(**{"quality.profile": None, "quality.msaa_samples": None})
    assert a == b
