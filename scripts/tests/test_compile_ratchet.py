"""The SECONDS weighting behind `scripts/compile_ratchet.py` (D27).

The graph walk was already guarded by the gate's own existence — it runs in the
suite and reddens when it is wrong. What was not guarded is the WEIGHT each node
in that walk contributes, which is new and which has four ways to be plausibly
wrong that produce numbers nobody would question.

Each test names the naive draft it rejects, and each was watched red against
that draft before it was kept.
"""

from __future__ import annotations

import functools
import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import compile_ratchet as ratchet  # noqa: E402

MONOLITH = "ambition_platformer2d_actor_monolith"
FLOOR = "ambition_platformer2d_core"


def unit(**over):
    """One `kind: unit` row from a first-party lib in the weighted config."""
    row = {
        "unit": "ambition_thing",
        "target": "",
        "seconds": 10.0,
        "lines": 1000,
        "first_party": True,
        "backfilled": False,
        "opt_level": "3",
        "commit": "abc123456789",
        "build_source": "/t/release-rebuild.html",
        "build_profile": "release",
        "build_label": "collector: release/first-party",
        "build_fresh_units": 632,
        "build_dirty_units": 57,
        "build_total_seconds": "360.0s",
        "build_build_started_at": "2026-08-08T05:54:02Z",
    }
    row.update(over)
    return row


@functools.cache
def baseline() -> dict:
    return json.loads(ratchet.BASELINE.read_text(encoding="utf-8"))


@functools.cache
def live() -> dict:
    """The current tree, priced with the FROZEN weights — one cargo walk, reused."""
    return ratchet.snapshot(weights=baseline()["unit_weights"])


# ---------------------------------------------------------------------------
# picking the measurements
# ---------------------------------------------------------------------------


def test_cache_state_comes_from_the_counters_never_from_the_build_label():
    """⛔ the naive draft selects rebuilds by `build_label`.

    `dev/ambition_dev_measurements/compile_units.jsonl` holds a build labelled `collector: dev/first-party`
    with `build_fresh_units: 0` — it recompiled all 688 units and took 540s
    against two honest first-party rebuilds at 188s and 210s. A label-based
    filter admits it and every weight comes out multiples high, with nothing in
    the output saying so.
    """
    honest = unit(seconds=10.0, build_fresh_units=632, build_dirty_units=57)
    liar = unit(
        seconds=40.0,
        build_source="/t/liar.html",
        build_fresh_units=0,
        build_dirty_units=689,
    )
    assert ratchet.cache_class(honest) == "rebuild"
    assert ratchet.cache_class(liar) == "cold"
    # Same label on both, so a label filter cannot tell them apart.
    assert honest["build_label"] == liar["build_label"]

    weights = ratchet.unit_weights([honest, liar])
    assert weights["ms_per_line"] == {"ambition_thing": 10.0}  # not 25.0, not 40.0


def test_configurations_are_not_pooled_into_one_weight():
    """⛔ the naive draft takes every first-party row it can find.

    The ledger mixes `release/opt-3`, `test/opt-1` and `test/opt-0`. Two crates
    read out of a pooled table need not even share an opt level, so their
    relative weights — which is all a sum over a closure uses — are meaningless.
    """
    rows = [
        unit(seconds=10.0),
        unit(seconds=2.0, build_profile="test", opt_level="1",
             build_source="/t/dev-rebuild.html"),
    ]
    weights = ratchet.unit_weights(rows)

    assert weights["profile"] == "release"
    assert weights["ms_per_line"] == {"ambition_thing": 10.0}  # not 6.0
    assert weights["opt_levels"] == ["3"]


def test_a_backfilled_row_is_dropped_because_its_lines_describe_another_tree():
    """⛔ the naive draft divides `seconds` by whatever `lines` says.

    A backfilled row's LOC was read at ingest, not at build — the ledger's own
    schema says so. Dividing a real duration by an unrelated line count produces
    a rate that looks exactly like a measurement.
    """
    rows = [
        unit(seconds=10.0, lines=1000),
        unit(seconds=10.0, lines=100, backfilled=True, build_source="/t/backfill.html"),
    ]
    assert ratchet.unit_weights(rows)["ms_per_line"] == {"ambition_thing": 10.0}


def test_every_unit_of_one_crate_sums_against_one_line_count():
    """⛔ the naive draft averages the per-unit rates.

    A crate compiles as lib + test + bin + build script. It has one line count
    however many units it produces, so the rates must be summed before the
    division, not after it.
    """
    rows = [
        unit(seconds=8.0, lines=1000),
        unit(seconds=2.0, lines=1000, target='app_it "test" (test)'),
    ]
    assert ratchet.unit_weights(rows)["ms_per_line"] == {"ambition_thing": 10.0}


# ---------------------------------------------------------------------------
# applying them
# ---------------------------------------------------------------------------


def test_the_weight_is_a_rate_so_growth_since_the_measurement_costs_seconds():
    """⛔ the naive draft freezes each crate's measured DURATION.

    A frozen duration cannot answer the question a carve asks — 10,000 lines
    leaving a crate would change nothing — and a crate that doubled since it was
    measured would still be priced at its old size.
    """
    weights = baseline()["unit_weights"]
    rate = weights["ms_per_line"][MONOLITH]
    now = live()["crates"][MONOLITH]

    assert now["seconds"] == round(rate * now["lines"] / 1000.0, 3)
    grown = ratchet.snapshot(
        override_lines={MONOLITH: now["lines"] + 10_000}, weights=weights
    )
    assert grown["crates"][MONOLITH]["seconds"] > now["seconds"]


def test_an_unmeasured_crate_is_priced_at_the_median_and_reported_as_a_guess():
    """⛔ the naive draft gives an unmeasured crate a weight of zero.

    That is the failure mode where a carve looks free: the lines leave a priced
    crate and land somewhere that costs nothing. The median is a placeholder —
    a least-squares fit of seconds on lines reads R^2 = 0.12 over these 55
    crates — so it has to arrive with a finding attached, or the next reader
    takes it for a measurement.
    """
    weights = baseline()["unit_weights"]
    invented = ratchet.snapshot(
        override_lines={"ambition_invented": 10_000},
        extra_edges={"ambition_app": {"ambition_invented"}},
        weights=weights,
    )
    entry = invented["crates"]["ambition_invented"]

    assert entry["seconds"] > 0
    assert entry["seconds"] == round(weights["median_ms_per_line"] * 10_000 / 1000.0, 3)
    assert entry["seconds_source"] == "estimated"
    # ⚠ MEMBERSHIP, not equality. Equality here also asserted "and the working
    # tree contains no other unpriced crate", which is a different claim and not
    # this test's subject — a real carve landing before its `compile_collect.py`
    # run makes it false, and the D33 carve (`ambition_character_sprites`,
    # 2026-08-09) did exactly that. The property under test is that an UNMEASURED
    # crate is priced from the median and REPORTED; the ratchet's own UNPRICED
    # finding is what guards the tree, and it fires on the live tree already.
    assert "ambition_invented" in invented["unpriced_crates"]

    severities = [severity for severity, _ in ratchet.evaluate(invented, baseline())]
    assert "UNPRICED" in severities


def test_a_baseline_without_seconds_goes_red_instead_of_checking_nothing():
    """⛔ the naive draft reads the frozen seconds with `.get(..., 0)`.

    A baseline frozen before this guard existed then compares 1,249s against 0,
    which is either a silent pass or a nonsense regression depending on which
    way the `.get` falls. Neither is a guard.
    """
    stale = {k: v for k, v in baseline().items() if k != "worst_edit_cost_seconds"}
    severities = [severity for severity, _ in ratchet.evaluate(live(), stale)]
    assert "STALE" in severities


def test_appending_to_the_ledger_cannot_move_a_guarded_number(monkeypatch, tmp_path, capsys):
    """⛔ the naive draft recomputes the weights from the ledger every run.

    Then `scripts/compile_collect.py` appending 57 rows turns the gate red on a
    tree nobody edited — the false red this whole instrument was designed to
    avoid. Weights are frozen in the baseline and move only on `--update`.
    """
    def reported_worst_seconds() -> str:
        """The `worst_edit_cost_seconds` REPORT row, as printed.

        ⚠ the row, not the whole output. An earlier draft searched all of stdout
        and passed against the very draft it rejects: a `REGRESSED` finding
        quotes the frozen value in its own message, so the string was present
        precisely when the guard had gone wrong.
        """
        ratchet.main(["--report-only"])
        printed = capsys.readouterr().out
        return next(
            line for line in printed.splitlines()
            if line.strip().startswith("worst_edit_cost_seconds")
        )

    # Baseline reading FIRST, against the real ledger and this working tree.
    before = reported_worst_seconds()

    doctored = tmp_path / "compile_units.jsonl"
    doctored.write_text(
        "\n".join(
            json.dumps({**json.loads(line), "seconds": json.loads(line)["seconds"] * 9})
            for line in ratchet.UNIT_LEDGER.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ratchet, "UNIT_LEDGER", doctored)
    # The ledger really did change: read directly, the weights are 9x.
    assert ratchet.unit_weights()["ms_per_line"][MONOLITH] == pytest.approx(
        baseline()["unit_weights"]["ms_per_line"][MONOLITH] * 9, rel=1e-3
    )

    # ⛔ compare the two READINGS, never a reading against a frozen constant.
    # The first draft of this test asserted the row contained the baseline's
    # `worst_edit_cost_seconds` verbatim — and that number is
    # `frozen weight × the tree's CURRENT line count`, so it moved the moment
    # another agent added 380 lines to the monolith. It went red on ordinary
    # work while the property it names was never violated. A test whose subject
    # is "X cannot change Y" must hold everything but X still, and the tree is
    # not X.
    assert reported_worst_seconds() == before


# ---------------------------------------------------------------------------
# the case the row exists for
# ---------------------------------------------------------------------------


def test_lines_moved_into_a_dense_crate_reads_as_a_win_on_every_line_number():
    """⛔ the naive draft is the gate as it stood before D27 — four line numbers.

    10,000 lines leave the monolith (0.61 ms/line measured) for a sibling crate
    shaped like `ambition_platformer2d_runtime` (14.77 ms/line measured), which
    still depends on what the monolith depended on. The build gets ~140s slower.
    Every line number says the carve worked.
    """
    weights = baseline()["unit_weights"]
    before = live()
    new = "ambition_carved"
    moved = 10_000
    edges = {p: {new} for p in before["crates"][MONOLITH]["direct_dependents"]}
    edges[new] = {
        name
        for name, entry in before["crates"].items()
        if MONOLITH in entry.get("direct_dependents", ())
    }
    dense = {**weights, "ms_per_line": {**weights["ms_per_line"], new: 14.77}}
    after = ratchet.snapshot(
        override_lines={MONOLITH: before["crates"][MONOLITH]["lines"] - moved, new: moved},
        extra_edges=edges,
        weights=dense,
    )

    # Every line number improves or holds still.
    assert after["largest_unit"]["lines"] == before["largest_unit"]["lines"] - moved
    assert after["worst_edit_cost"]["lines"] == before["worst_edit_cost"]["lines"]
    assert (
        after["watched_edit_cost"][MONOLITH]["lines"]
        == before["watched_edit_cost"][MONOLITH]["lines"] - moved
    )
    assert after["critical_path_crates"] == before["critical_path_crates"]

    # And the build is slower, by more than the whole worst-case budget.
    grew = (
        after["worst_edit_cost_seconds"]["seconds"]
        - before["worst_edit_cost_seconds"]["seconds"]
    )
    assert grew > 100.0
    assert grew > before["worst_edit_cost_seconds"]["seconds"] * ratchet.HEADROOM_FRACTION

    # ⛔ evaluated against `before` -- the live snapshot this carve was applied
    # to -- and NOT against the frozen baseline file. This test is about the
    # MODEL's reasoning: that the naive line view calls a dense carve a win
    # while the cost model calls it a regression. Comparing against the frozen
    # file made the verdict depend on how far the tree had drifted from it, and
    # in 2026-08 the monolith drifted +10,393 lines past its baseline, so
    # `live - 10,000` was still above the frozen number and the CARVED finding
    # stopped appearing. The test went red for a reason that had nothing to do
    # with the reasoning it exists to pin.
    findings = ratchet.evaluate(after, before)
    assert any(
        severity == "REGRESSED" and label.startswith("worst_edit_cost_seconds")
        for severity, label in findings
    )
    assert any(severity == "CARVED" and "largest_unit_lines" in label
               for severity, label in findings)


def test_the_same_carve_at_the_median_rate_is_inside_budget_and_still_flagged():
    """⛔ the naive draft treats the median fallback as good enough to gate on.

    A carve into an UNMEASURED crate is priced at the population median, and at
    that rate a carve can sit comfortably inside the magnitude budget while still
    being a real cost. So the magnitude arm does NOT gate this class, and the
    UNPRICED finding is the only thing standing there. Recorded as a test so the
    limit is a known one.

    ⛔ **the carve size is DERIVED, and it took three tries.** Kept in full,
    because each draft failed for a different and instructive reason:

    1. `moved = 10_000`, with a docstring saying *"the same 10,000 lines cost
       +19.5s, inside a 25s budget"*. Both numbers went stale as the tree grew;
       by 2026-08-09 that carve cost **+26.4s against +25.2s** and went red,
       **clearing by 1.2s — 4.7%.** ⇒ the assertion had been pinning a
       COINCIDENCE: *"no magnitude finding"* was never a property of the guard,
       only a fact about where the tree sat relative to one budget.
    2. Sized to **half the budget** from a probe — and red again within hours.
       ⛔ **because it measured against the LIVE tree and asserted against the
       FROZEN baseline.** Two denominators. The live tree had already spent
       +13.1s of the +25.2s budget (an unpriced carve, priced at the median), so
       half-the-budget from `live` still landed at +25.8s from `baseline`.
       ⚠ its comment claimed *"a margin no plausible tree growth erases"*. One
       ordinary 369-line commit erased it.
    3. **This version sizes against the headroom that is actually LEFT** —
       `budget - (live - baseline)` — which is the same quantity a real carve
       faces, so the claim under test got more honest rather than more forgiving.
       It `skip`s, loudly, when the tree has spent everything.

    ⇒ the durable lesson is #2's: **name both denominators before dividing.** A
    number sized against one reference and judged against another is wrong by
    exactly the drift between them, and it fails intermittently as that drift
    moves — which reads like flakiness rather than like a bug.
    """
    weights = baseline()["unit_weights"]
    before = live()
    new = "ambition_carved"
    budget = before["worst_edit_cost_seconds"]["seconds"] * ratchet.HEADROOM_FRACTION

    def carve(moved):
        edges = {p: {new} for p in before["crates"][MONOLITH]["direct_dependents"]}
        edges[new] = {
            name
            for name, entry in before["crates"].items()
            if MONOLITH in entry.get("direct_dependents", ())
        }
        return ratchet.snapshot(
            override_lines={MONOLITH: before["crates"][MONOLITH]["lines"] - moved, new: moved},
            extra_edges=edges,
            weights=weights,
        )

    def grew_by(after):
        return (
            after["worst_edit_cost_seconds"]["seconds"]
            - before["worst_edit_cost_seconds"]["seconds"]
        )

    # ⛔ TWO DENOMINATORS, and getting them confused is what broke the previous
    # two versions of this test. `grew_by` measures against the LIVE tree;
    # `ratchet.evaluate` compares against the FROZEN baseline. The live tree has
    # usually already spent part of the budget, so "half the budget" measured
    # from `live` can still land over the budget measured from `baseline`.
    #
    # Size against the headroom that is actually LEFT — which is also what a real
    # carve faces, so the claim under test gets more honest rather than less.
    spent = (
        before["worst_edit_cost_seconds"]["seconds"]
        - baseline()["worst_edit_cost_seconds"]["seconds"]
    )
    remaining = budget - spent
    if remaining <= 0:
        pytest.skip(
            f"the live tree has already spent the whole budget "
            f"({spent:.1f}s of {budget:.1f}s); there is no headroom to size a "
            f"carve into, and that is a fact about the tree, not this test"
        )

    # Probe once to learn this tree's median-rate premium per moved line, then
    # size the real carve at half the REMAINING headroom. Linear in `moved`: the
    # moved lines are repriced from the owner's measured rate to the median.
    probe_moved = 10_000
    per_line = grew_by(carve(probe_moved)) / probe_moved
    assert per_line > 0, "moving lines to the median rate must cost something"
    moved = int(remaining * 0.5 / per_line)

    after = carve(moved)
    grew = grew_by(after)
    assert 0 < grew < remaining * 0.75

    severities = [severity for severity, _ in ratchet.evaluate(after, baseline())]
    assert "REGRESSED" not in severities, (
        f"a {moved:,}-line carve costing +{grew:.1f}s should clear the magnitude "
        f"arm: the live tree has spent {spent:.1f}s of its {budget:.1f}s budget, "
        f"leaving {remaining:.1f}s, and this carve uses half of that. The UNPRICED "
        f"finding is the gate here, not magnitude"
    )
    assert "UNPRICED" in severities
