"""The SECONDS weighting behind `scripts/compile_ratchet.py` (D27).

The graph walk was already guarded by the gate's own existence — it runs in the
suite and reddens when it is wrong. What was not guarded is the WEIGHT each node
in that walk contributes, which is new and which has four ways to be plausibly
wrong that produce numbers nobody would question.

Each test names the naive draft it rejects, and each was watched red against
that draft before it was kept.
"""

from __future__ import annotations

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


APP = "ambition_app"


@pytest.fixture
def model(monkeypatch):
    """A tiny resolved graph for cost-model tests.

    The production gate separately observes the real repository. These tests ask
    whether projections over an already-observed graph behave correctly, so
    giving every arithmetic poison test its own Cargo walk was testing the wrong
    seam and made a pure model suite take seconds.

        app -> monolith -> floor

    The rates deliberately make the monolith cheap and a carved dense crate
    expensive, preserving the counterexample that motivated the seconds guard.
    """
    lines = {APP: 5_000, MONOLITH: 100_000, FLOOR: 10_000}
    dirs = {name: Path("/synthetic") / name for name in lines}
    edges = {APP: {MONOLITH}, MONOLITH: {FLOOR}, FLOOR: set()}
    weights = {
        "weight_unit": "synthetic milliseconds per physical line",
        "profile": "release",
        "cache_class": "rebuild",
        "opt_levels": ["3"],
        "ledger": "synthetic",
        "builds": [],
        "crates_measured": len(lines),
        "median_ms_per_line": 1.0,
        "ms_per_line": {APP: 2.0, MONOLITH: 0.6, FLOOR: 1.2},
    }

    monkeypatch.setattr(ratchet, "workspace_dirs", lambda: dirs)
    monkeypatch.setattr(
        ratchet,
        "crate_lines",
        lambda directory: {
            "lines": lines[directory.name],
            "files": 1,
            "test_file_lines": 0,
        },
    )
    monkeypatch.setattr(
        ratchet,
        "resolved_edges",
        lambda consumer: ({name: set(deps) for name, deps in edges.items()}, APP),
    )

    before = ratchet.snapshot(weights=weights)
    frozen = {**before, "headroom_fraction": ratchet.HEADROOM_FRACTION}
    return weights, before, frozen


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


def test_the_weight_is_a_rate_so_growth_since_the_measurement_costs_seconds(model):
    """⛔ the naive draft freezes each crate's measured DURATION.

    A frozen duration cannot answer the question a carve asks — 10,000 lines
    leaving a crate would change nothing — and a crate that doubled since it was
    measured would still be priced at its old size.

    This is a projection property, so it runs on the tiny observed graph above;
    the production gate is what proves the real repository can be observed.
    """
    weights, before, _frozen = model
    rate = weights["ms_per_line"][MONOLITH]
    now = before["crates"][MONOLITH]

    assert now["seconds"] == round(rate * now["lines"] / 1000.0, 3)
    grown = ratchet.snapshot(
        override_lines={MONOLITH: now["lines"] + 10_000}, weights=weights
    )
    assert grown["crates"][MONOLITH]["seconds"] > now["seconds"]


def test_an_unmeasured_crate_is_priced_at_the_median_and_reported_as_a_guess(model):
    """⛔ the naive draft gives an unmeasured crate a weight of zero.

    A new crate must receive the median placeholder and, more importantly, an
    UNPRICED finding. This is model behavior and does not require Cargo to
    rediscover Ambition's live graph.
    """
    weights, _before, frozen = model
    invented = ratchet.snapshot(
        override_lines={"ambition_invented": 10_000},
        extra_edges={APP: {"ambition_invented"}},
        weights=weights,
    )
    entry = invented["crates"]["ambition_invented"]

    assert entry["seconds"] > 0
    assert entry["seconds"] == round(weights["median_ms_per_line"] * 10_000 / 1000.0, 3)
    assert entry["seconds_source"] == "estimated"
    assert "ambition_invented" in invented["unpriced_crates"]

    severities = [severity for severity, _ in ratchet.evaluate(invented, frozen)]
    assert "UNPRICED" in severities


def test_a_baseline_without_seconds_goes_red_instead_of_checking_nothing(model):
    """⛔ a stale baseline must fail instead of silently checking nothing."""
    _weights, before, frozen = model
    stale = {k: v for k, v in frozen.items() if k != "worst_edit_cost_seconds"}
    severities = [severity for severity, _ in ratchet.evaluate(before, stale)]
    assert "STALE" in severities


def test_appending_to_the_ledger_cannot_move_a_guarded_number(
    model, monkeypatch, tmp_path, capsys
):
    """⛔ appending measurements cannot move a number priced by frozen weights.

    The old version paid for two real repository observations and rewrote the
    entire real ledger. Neither is part of the property. A one-row synthetic
    ledger proves the same authority boundary directly: `main` must use weights
    frozen in the baseline while an explicit `unit_weights()` read sees the new
    ledger.
    """
    weights, _before, frozen = model
    baseline_path = tmp_path / "compile-ratchet-baseline.json"
    baseline_path.write_text(json.dumps(frozen), encoding="utf-8")
    monkeypatch.setattr(ratchet, "BASELINE", baseline_path)

    ledger = tmp_path / "compile_units.jsonl"
    rate = weights["ms_per_line"][MONOLITH]

    def write_ledger(multiplier: float) -> None:
        row = unit(unit=MONOLITH, seconds=rate * multiplier, lines=1000)
        ledger.write_text(json.dumps(row) + "\n", encoding="utf-8")

    monkeypatch.setattr(ratchet, "UNIT_LEDGER", ledger)
    write_ledger(1.0)

    def reported_worst_seconds() -> str:
        ratchet.main(["--report-only"])
        printed = capsys.readouterr().out
        return next(
            line for line in printed.splitlines()
            if line.strip().startswith("worst_edit_cost_seconds")
        )

    before = reported_worst_seconds()
    write_ledger(9.0)
    assert ratchet.unit_weights()["ms_per_line"][MONOLITH] == pytest.approx(rate * 9)
    assert reported_worst_seconds() == before


# ---------------------------------------------------------------------------
# the case the row exists for
# ---------------------------------------------------------------------------


def test_lines_moved_into_a_dense_crate_reads_as_a_win_on_every_line_number(model):
    """A line-only model must reject a carve that makes compile seconds worse."""
    weights, before, _frozen = model
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

    assert after["largest_unit"]["lines"] == before["largest_unit"]["lines"] - moved
    assert after["worst_edit_cost"]["lines"] == before["worst_edit_cost"]["lines"]
    assert (
        after["watched_edit_cost"][MONOLITH]["lines"]
        == before["watched_edit_cost"][MONOLITH]["lines"] - moved
    )
    assert after["critical_path_crates"] == before["critical_path_crates"]

    grew = (
        after["worst_edit_cost_seconds"]["seconds"]
        - before["worst_edit_cost_seconds"]["seconds"]
    )
    assert grew > 100.0
    assert grew > before["worst_edit_cost_seconds"]["seconds"] * ratchet.HEADROOM_FRACTION

    findings = ratchet.evaluate(after, before)
    assert any(
        severity == "REGRESSED" and label.startswith("worst_edit_cost_seconds")
        for severity, label in findings
    )
    assert any(severity == "CARVED" and "largest_unit_lines" in label
               for severity, label in findings)


def test_the_same_carve_at_the_median_rate_is_inside_budget_and_still_flagged(model):
    """The median placeholder can clear magnitude while UNPRICED still gates it.

    The previous version sized the carve against today's live-tree drift. That
    made a model poison test depend on whatever unrelated edits happened to have
    landed before pytest started. Here the observed graph itself is the frozen
    reference, so only the behavior named by the test can move the result.
    """
    weights, before, frozen = model
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

    probe_moved = 10_000
    per_line = grew_by(carve(probe_moved)) / probe_moved
    assert per_line > 0, "moving lines to the median rate must cost something"
    moved = int(budget * 0.5 / per_line)

    after = carve(moved)
    grew = grew_by(after)
    assert 0 < grew < budget * 0.75

    severities = [severity for severity, _ in ratchet.evaluate(after, frozen)]
    assert "REGRESSED" not in severities
    assert "UNPRICED" in severities


class TestLargestUnitAgainstBaseline:
    """**A win can be given back in SILENCE, and the report is what says so.**

    ⛔⛔ the monolith went 111,429 (frozen) → 110,932 after a carve, was
    celebrated as *"under baseline for the first time"*, and stood at 112,357 one
    day later — **+928 OVER**. The gate said nothing, correctly: the number
    carries a 2% growth budget and 112,357 is inside it.

    ⭐ so the GATE is right and the REPORT was incomplete. A budget answers *"is
    this worth failing on"*; it does not answer *"are we where we thought we
    were"*. ⛔ this annotates the second question and gates NOTHING — deliberately
    not a tightened budget, because *"the compile ratchet is an INSTRUMENT, NOT A
    TARGET"* and a tighter budget would make it more of one.
    """

    @staticmethod
    def _pair(was: int, now: int, was_crate: str = "mono", now_crate: str = "mono"):
        frozen = {"largest_unit": {"lines": was, "crate": was_crate}, "headroom_fraction": 0.02}
        current = {"largest_unit": {"lines": now, "crate": now_crate}}
        return current, frozen

    def test_a_regression_inside_the_budget_is_still_reported(self):
        # The exact case that was invisible: over the frozen line, under the budget.
        text = ratchet._vs_baseline(*self._pair(111_429, 112_357))
        assert "111,429" in text and "+928" in text
        assert "within budget" in text, "the annotation must say the gate is right"

    def test_an_improvement_reads_as_one(self):
        text = ratchet._vs_baseline(*self._pair(111_429, 110_932))
        assert "-497" in text and "within budget" in text

    def test_outside_the_budget_says_so(self):
        text = ratchet._vs_baseline(*self._pair(111_429, 120_000))
        assert "OUTSIDE budget" in text

    def test_a_different_crate_taking_the_title_is_flagged(self):
        # Comparing two crates' line counts as if they were one number is how a
        # carve looks like an improvement while the work merely moved.
        text = ratchet._vs_baseline(*self._pair(111_429, 90_000, "mono", "somebody_else"))
        assert "different crate" in text

    def test_a_missing_or_zero_baseline_annotates_nothing(self):
        assert ratchet._vs_baseline({"largest_unit": {"lines": 1}}, {}) == ""
        assert ratchet._vs_baseline(*self._pair(0, 10)) == ""


def _snapshot(commit: str, critical_path: int, largest: int) -> dict:
    """The two fields these arms move, in the shape the real baseline carries.

    ⚠ `largest_unit` / `worst_edit_cost` are DICTS keyed by `crate`, not names:
    `adopt_wins` compares subject crates before adopting, and a string fixture
    fails inside the function rather than at the assertion, which reads as a
    broken test instead of a wrong fixture.
    """
    return {
        "commit": commit,
        "unit_weights": {},
        "unpriced_crates": [],
        "critical_path_crates": critical_path,
        "largest_unit": {"crate": "ambition_x", "lines": largest},
        "largest_unit_seconds": {"crate": "ambition_x", "seconds": float(largest)},
        "watched_edit_cost": {},
        "worst_edit_cost": {"crate": "ambition_y", "crates": 1, "lines": 10},
        "worst_edit_cost_seconds": {"crate": "ambition_y", "crates": 1, "seconds": 10.0},
    }


def test_adopt_wins_records_where_the_unadopted_numbers_came_from():
    """⛔⛔ ONE PROVENANCE FIELD CANNOT STAND FOR TWO PROVENANCES.

    `--adopt-wins` banks improvements and leaves every regression at its older,
    tighter value — but `commit` comes from the CURRENT snapshot, so it advanced
    for numbers that did not. MEASURED 2026-09-05: `f507fcb91` advanced `commit`
    (11ef33c5b5a5 -> c4a51a1b76a9) while leaving `critical_path_crates: 14` as
    measured at the older one.

    ⚠ THIS DOCSTRING USED TO SAY THAT COMMIT "changed ONLY `commit`". It did not
    — it also re-recorded the whole per-crate table, 538 insertions — and
    `compile_ratchet.py` retracts the sentence in its own comment: *"I had
    grepped the diff for two field names and read the filtered view as the
    whole."* The retraction lived in the source and the retracted claim lived
    here, so the test went on teaching the wrong fact. ⇒ A correction has to land
    everywhere the claim does. The report then claimed the newer commit and
    `--diff` offered a range in which no manifest had changed at all, while the
    PATH finding told the reader to "say which carve did it".
    """
    frozen = _snapshot("oldersha", critical_path=14, largest=100)
    # A win on one metric, a regression on the other: exactly the mixed case.
    current = _snapshot("newersha", critical_path=16, largest=90)
    merged, adopted, held = ratchet.adopt_wins(current, frozen)

    assert merged["commit"] == "newersha", "the adopting commit is still recorded"
    assert merged["carried_from"] == "oldersha", (
        "the held numbers' provenance was lost, which is the whole defect"
    )
    assert merged["critical_path_crates"] == 14, "a regression must stay frozen"
    assert adopted and held, f"expected a mixed adopt: adopted={adopted} held={held}"


def test_a_fully_adopted_baseline_still_records_its_predecessor():
    """⚠ Recorded even when nothing was held, because the reader cannot tell the
    two cases apart from the file — and a field that only sometimes exists is one
    more thing to forget. The report compares it with `commit` and stays quiet
    when they agree."""
    frozen = _snapshot("oldersha", critical_path=14, largest=100)
    current = _snapshot("newersha", critical_path=10, largest=90)
    merged, _adopted, held = ratchet.adopt_wins(current, frozen)
    assert not held, f"nothing should be held here: {held}"
    assert merged["carried_from"] == "oldersha"


def test_accept_refreezes_only_the_named_metric():
    """⭐ THE MIDDLE VERB THE RATCHET WAS MISSING.

    `--update` accepts everything including regressions nobody looked at;
    `--adopt-wins` accepts none of them. Neither can say "this ONE regression is
    a deliberate landing" — which is exactly what the regression message asks the
    reader to say. `critical_path_crates: 14 -> 16` stayed red for a week after
    being triaged, attributed to two named D33 carve outputs, and accepted.
    """
    frozen = _snapshot("oldersha", critical_path=14, largest=100)
    current = _snapshot("newersha", critical_path=16, largest=140)
    merged, accepted, unknown = ratchet.accept_regressions(
        current, frozen, ["critical_path_crates"], "the carve programme"
    )
    assert not unknown
    assert merged["critical_path_crates"] == 16, "the named metric moves to today"
    assert merged["largest_unit"]["lines"] == 100, (
        "an UNNAMED regression must stay frozen — accepting one number is not "
        "accepting the run"
    )
    assert accepted == ["critical_path_crates: 14 -> 16"]
    assert merged["accepted_reasons"]["critical_path_crates"] == "the carve programme"


def test_accept_refuses_a_metric_that_is_not_a_judgement_call():
    """⛔ A baseline KEY is not automatically a thing a human can accept.
    `unit_weights` and `unpriced_crates` are inputs to the pricing, and accepting
    them at "today's value" is the laundering `--adopt-wins` had to be taught not
    to do — it silently prices eight crates at a placeholder median."""
    frozen = _snapshot("oldersha", critical_path=14, largest=100)
    current = _snapshot("newersha", critical_path=16, largest=140)
    _merged, _accepted, unknown = ratchet.accept_regressions(
        current, frozen, ["unpriced_crates"], "because"
    )
    assert unknown == ["unpriced_crates"]


def test_accept_keeps_the_split_provenance():
    """⚠ An accept moves SOME numbers to today, so the rest are still carried —
    the same rule `--adopt-wins` learned. Losing it here would reintroduce the
    defect one verb over."""
    frozen = _snapshot("oldersha", critical_path=14, largest=100)
    current = _snapshot("newersha", critical_path=16, largest=140)
    merged, _accepted, _unknown = ratchet.accept_regressions(
        current, frozen, ["critical_path_crates"], "the carve programme"
    )
    # ⛔ NOT `merged["commit"]`. `freeze()` stamps that from `envelope()`, which
    # reads git; a value set here OVERRIDES the real sha, and setting it from a
    # snapshot that carries no `commit` wrote `null` into the live baseline. The
    # earlier version of this assertion PINNED that bug and made it look checked.
    assert "commit" not in merged or merged["commit"] == "oldersha", (
        "accept must not stamp provenance; the envelope owns it"
    )
    assert merged["carried_from"] == "oldersha"


# ---------------------------------------------------------------------------
# a finding must be readable ALONE
# ---------------------------------------------------------------------------


def test_a_regression_says_how_much_of_it_predates_the_baseline(model):
    """⛔⛔ A CORRECT NUMBER THAT IS UNREADABLE ALONE.

    `--adopt-wins` refreshes the `crates` table from the CURRENT snapshot and
    writes the derived scalars back from the OLD one, so a baseline holds one fact
    twice with different values. The findings compare against the STALE copy --
    deliberately, since changing that is a change to what the gate MEANS and
    belongs to the carve owner. But the disclosure printed twenty lines above, in
    different words, so a reader met `+68,648, budget +10,804` with nothing
    attached and read a catastrophe where the real movement was roughly one budget
    over. Same number, opposite actions.
    """
    _weights, before, frozen = model
    crate = frozen["worst_edit_cost"]["crate"]
    stored = frozen["worst_edit_cost"]["lines"]

    # The baseline disagreeing with itself: the table says MORE than the scalar.
    frozen = {
        **frozen,
        "crates": {**frozen["crates"], crate: {**frozen["crates"][crate],
                                               "edit_cost_lines": stored + 40_000}},
    }
    current = {
        **before,
        "worst_edit_cost": {**before["worst_edit_cost"], "lines": stored + 60_000},
    }

    findings = dict(
        (message.split(":")[0], message) for _sev, message in ratchet.evaluate(current, frozen)
    )
    worst = next(m for k, m in findings.items() if k.startswith("worst_edit_cost_lines"))
    assert "PREDATES THE BASELINE" in worst
    assert ratchet._lines(40_000) in worst, "it must name the amount, not just warn"


def test_a_metric_whose_baseline_agrees_with_itself_gets_no_such_note(model):
    """⚠ ANTI-FALSE-POSITIVE, and the arm that makes the one above mean something.

    If the note were appended unconditionally, the test above would pass on a
    baseline that is perfectly consistent — and every reader would learn to skip
    the sentence.
    """
    _weights, before, frozen = model
    stored = frozen["worst_edit_cost"]["lines"]
    current = {
        **before,
        "worst_edit_cost": {**before["worst_edit_cost"], "lines": stored + 60_000},
    }
    messages = [m for _s, m in ratchet.evaluate(current, frozen)
                if m.startswith("worst_edit_cost_lines")]
    assert messages, "the regression itself must still be reported"
    assert "PREDATES THE BASELINE" not in messages[0]


def test_largest_unit_is_not_annotated_because_it_is_not_a_derived_scalar(model):
    """⭐ `_derived_scalars` is the ONE list saying which numbers are derivations.

    `largest_unit` is primary — a max over per-crate line counts with no stored
    duplicate — so it can never disagree with the table and must never carry the
    note. A note there would be a false claim about the baseline.
    """
    _weights, before, frozen = model
    labels = [label for label, _entry in ratchet._derived_scalars(frozen)]
    assert "worst_edit_cost" in labels
    assert not any(label.startswith("largest_unit") for label in labels)


def _with_table(snapshot: dict, **rows: int) -> dict:
    """Give a snapshot the per-crate table its scalars are derived from."""
    snapshot["crates"] = {
        name: {"edit_cost_lines": lines, "lines": lines, "direct_dependents": []}
        for name, lines in rows.items()
    }
    return snapshot


def test_a_held_scalar_holds_the_table_row_it_was_derived_from():
    """⛔⛔ ONE FACT, TWO VALUES: the defect that made the ratchet's own report
    print `⚠ baseline disagrees with itself`.

    `worst_edit_cost` and `watched_edit_cost` are DERIVED from the `crates` table.
    `adopt_wins` starts from the CURRENT snapshot and writes held scalars back
    from the OLD one, so before this the table advanced while the scalar did not
    and the file stored one crate's line count twice with two values. Measured on
    the real baseline: `ambition_geometry` read 540,227 in the scalar and 592,091
    in the same file's table, and `+51,864` of a reported regression was that gap
    rather than any code anyone wrote.

    ⚠ This is NOT the change the module defers to the carve owner. Findings still
    compare against the HELD scalar, so a regression keeps its older, tighter
    value and nothing is laundered — the table simply stops contradicting it.
    """
    frozen = _with_table(_snapshot("oldersha", critical_path=14, largest=100),
                         ambition_mono=100, ambition_other=50)
    frozen["watched_edit_cost"] = {"ambition_mono": {"lines": 100}}
    # `ambition_mono` REGRESSED (100 -> 130), so its scalar is held.
    current = _with_table(_snapshot("newersha", critical_path=14, largest=100),
                          ambition_mono=130, ambition_other=50)
    current["watched_edit_cost"] = {"ambition_mono": {"lines": 130}}

    merged, _adopted, held = ratchet.adopt_wins(current, frozen)

    assert any("ambition_mono" in row for row in held), (
        f"premise: the regression must be HELD, not adopted; held={held}"
    )
    assert merged["watched_edit_cost"]["ambition_mono"]["lines"] == 100, (
        "premise: a held scalar keeps its older, tighter value"
    )
    assert merged["crates"]["ambition_mono"]["edit_cost_lines"] == 100, (
        "the table row a held scalar was derived from must be held with it; "
        "otherwise the baseline stores one fact twice with two values, and the "
        "gap is reported as a regression nobody caused"
    )


def test_an_adopted_crate_keeps_the_current_table_row():
    """⚠ THE OTHER ARM, and the one that makes the rule above falsifiable.

    Holding every row would be a different bug — it would freeze the table
    against a snapshot nobody measured. Only a crate whose SCALAR was held keeps
    its old row; a crate that IMPROVED has its scalar adopted, so its row must
    move with it or the two disagree in the opposite direction.
    """
    frozen = _with_table(_snapshot("oldersha", critical_path=14, largest=100),
                         ambition_mono=100)
    frozen["watched_edit_cost"] = {"ambition_mono": {"lines": 100}}
    # A real carve: 100 -> 70.
    current = _with_table(_snapshot("newersha", critical_path=14, largest=100),
                          ambition_mono=70)
    current["watched_edit_cost"] = {"ambition_mono": {"lines": 70}}

    merged, adopted, _held = ratchet.adopt_wins(current, frozen)

    assert any("ambition_mono" in row for row in adopted), (
        f"premise: the improvement must be ADOPTED; adopted={adopted}"
    )
    assert merged["watched_edit_cost"]["ambition_mono"]["lines"] == 70
    assert merged["crates"]["ambition_mono"]["edit_cost_lines"] == 70, (
        "an adopted scalar must not be left beside a stale table row"
    )
