"""The aggregation math behind `scripts/compile_report.py`.

⛔ **this tests the arithmetic, not the HTML.** Asserting on rendered bytes
would pin the layout and catch nothing; every case below is a way the reader
could quietly compute the wrong number and still produce a page that looks fine.

Each test names the naive first draft it rejects, because that draft is what a
reader of this file will otherwise reinvent.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

pytestmark = pytest.mark.detached_tool

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import compile_report as report  # noqa: E402


def unit(**over):
    """A minimal `kind: unit` row; every test overrides only what it means."""
    row = {
        "schema": 1,
        "kind": "unit",
        "unit": "ambition_thing",
        "seconds": 1.0,
        "lines": 1000,
        "first_party": True,
        "frontend_seconds": 0.25,
        "codegen_seconds": 0.75,
        "build_source": "/t/a.html",
        "build_profile": "test",
        "build_fresh_units": 600,
        "build_dirty_units": 57,
        "build_total_units": 657,
        "config": "dev",
        "phase": "first-party",
        "opt_level": "1",
        "incremental": False,
        "backfilled": False,
        "commit": "abc123456789",
        "build_build_started_at": "2026-08-08T05:00:00Z",
    }
    row.update(over)
    return row


def test_cost_per_line_is_computed_per_build_never_pooled_across_builds():
    """⛔ the naive draft sums seconds and lines over the whole ledger.

    Two builds of the SAME crate at different speeds pool into a single
    meaningless average. `dev/ambition_dev_measurements/compile_units.jsonl` holds seven builds across
    three configurations, so pooling is not a hypothetical.
    """
    rows = [
        unit(build_source="/t/fast.html", seconds=2.0, lines=1000),
        unit(build_source="/t/slow.html", seconds=8.0, lines=1000),
    ]
    by_build = report.crate_costs_by_build(rows)

    assert set(by_build) == {"/t/fast.html", "/t/slow.html"}
    assert by_build["/t/fast.html"][0].ms_per_line == 2.0
    assert by_build["/t/slow.html"][0].ms_per_line == 8.0
    # And the pooled answer — 5.0 ms/line — describes neither build.
    assert all(c.ms_per_line != 5.0 for costs in by_build.values() for c in costs)


def test_several_units_of_one_crate_in_one_build_sum_before_dividing():
    """A crate compiles as lib + test + build-script. One crate, one cost."""
    rows = [
        unit(unit="ambition_thing", seconds=3.0, lines=2000, target=""),
        unit(unit="ambition_thing", seconds=1.0, lines=2000, target='… "test"'),
    ]
    (cost,) = report.crate_costs_by_build(rows)["/t/a.html"]
    assert cost.units == 2
    assert cost.seconds == 4.0
    assert cost.lines == 2000  # NOT 4000 — the crate has one line count
    assert cost.ms_per_line == 2.0


def test_a_first_party_build_with_nothing_cached_is_reported_as_cold():
    """⛔ the naive draft trusts the `phase` column.

    `cargo-timing-20260808T111707964Z` is labelled `dev/first-party` and has
    `build_fresh_units: 0` — nothing was cached, so it recompiled all 688 units
    including third-party. Its 540s wall clock is not comparable to the 188s and
    210s of the two honest first-party rebuilds, and a reader that groups by the
    label averages them together.
    """
    honest = report.summarise_builds([unit(build_fresh_units=631)])[0]
    mislabelled = report.summarise_builds(
        [unit(build_fresh_units=0, build_dirty_units=688, build_total_units=688)]
    )[0]

    assert honest.cache_state == "warm"
    assert honest.label_disputed is False

    assert mislabelled.phase == "first-party"  # what the row claims
    assert mislabelled.cache_state == "cold"  # what the row shows
    assert mislabelled.label_disputed is True


def test_codegen_share_carries_both_denominators():
    """⛔ the naive draft divides codegen by one denominator and picks a side.

    Units that emit no metadata — build scripts, bins, the app's cdylib — have
    null `frontend_seconds`/`codegen_seconds` but real `seconds`. So codegen is
    ~80% of the time that IS split and ~73% of all unit-seconds, and both
    numbers are true. Reporting one without naming its denominator is how the
    same measurement gets quoted as two different findings.
    """
    rows = [
        unit(seconds=8.0, frontend_seconds=2.0, codegen_seconds=6.0),
        unit(seconds=2.0, frontend_seconds=None, codegen_seconds=None),
    ]
    split = report.split_totals(rows)

    assert split.frontend_seconds == 2.0
    assert split.codegen_seconds == 6.0
    assert split.attributed_seconds == 8.0
    assert split.all_seconds == 10.0
    assert split.unattributed_seconds == 2.0
    assert split.units_with_split == 1
    assert split.units == 2
    # 6/8 against the split time; 6/10 against every unit-second.
    assert split.codegen_share_of_split == 0.75
    assert split.codegen_share_of_all == 0.6


def test_schema_zero_scenario_rows_normalise_by_the_documented_table():
    """⛔ the naive draft reads `machine_cargo_incremental` as a boolean.

    `dev/compile_telemetry_schema.md` §4: `"(config default)"` means OFF before
    `.cargo/config.toml` turned incremental on and ON after, so the same string
    maps to both values. The ledger is append-only and is NOT rewritten; the
    mapping lives at read time, here.
    """
    on_by_env = report.normalise_scenario(
        {"env": {"CARGO_INCREMENTAL": "1"}, "label": "incremental", "scenario": "test-build"}
    )
    assert (on_by_env["incremental"], on_by_env["profile"]) == (True, "test")

    off_by_config = report.normalise_scenario(
        {"env": {}, "label": "baseline-config-default", "scenario": "test-build"}
    )
    on_by_config = report.normalise_scenario(
        {"env": {}, "label": "config-incremental-on", "scenario": "check"}
    )
    # Same `machine_cargo_incremental` string, opposite meanings.
    assert off_by_config["incremental"] is False
    assert on_by_config["incremental"] is True
    assert (off_by_config["profile"], on_by_config["profile"]) == ("test", "dev")
    assert off_by_config["dimension_source"] == "normalised (schema 0)"

    # A schema-1 row states its own dimensions and must not be re-derived.
    recorded = report.normalise_scenario(
        {"schema": 1, "env": {}, "label": "", "incremental": True, "profile": "dev", "opt_level": "3"}
    )
    assert (recorded["incremental"], recorded["opt_level"]) == (True, "3")
    assert recorded["dimension_source"] == "recorded"

    # An unmatched schema-0 row is UNKNOWN, never guessed.
    assert report.normalise_scenario({"env": {}, "label": "who knows"})["incremental"] is None


def test_a_warm_pass_that_cost_more_than_the_edit_is_flagged_not_averaged():
    """The first scenario of a session pays whatever the tree owed.

    Two of the four recorded rows have `warm_noop_seconds` at or above their
    `after_edit_seconds` (102.66 vs 104.86, and 32.30 vs 8.22). Subtracting
    those gives a negative or near-zero "cost of an edit", which is not a
    measurement of anything.
    """
    good = report.normalise_scenario(
        {"env": {}, "label": "x", "warm_noop_seconds": 0.5, "after_edit_seconds": 21.26}
    )
    assert good["edit_cost_seconds"] == 20.76
    assert good["warm_pass_suspect"] is False

    inverted = report.normalise_scenario(
        {"env": {}, "label": "x", "warm_noop_seconds": 32.30, "after_edit_seconds": 8.22}
    )
    assert inverted["warm_pass_suspect"] is True
    assert inverted["edit_cost_seconds"] is None


def test_test_time_splits_the_build_graph_from_the_running():
    """`seconds - executed_seconds` is the build; `executed_seconds` is libtest.

    ⚠ a pytest job reports `executed_seconds: 0.0` because libtest is the only
    thing that prints "finished in Xs" — so its whole wall clock lands in the
    build column and that is a known overstatement, not a bug.
    """
    runs = [
        {
            "finished": 1785732658.0,
            "seconds": 100.0,
            "executed_seconds": 30.0,
            "per_job": [
                {"job": "workspace", "ok": True, "seconds": 90.0, "executed_seconds": 30.0},
                {"job": "repo tooling", "ok": True, "seconds": 10.0, "executed_seconds": 0.0},
            ],
        },
        {
            "finished": 1785732758.0,
            "seconds": 50.0,
            "executed_seconds": 20.0,
            "per_job": [
                {"job": "workspace", "ok": False, "seconds": 50.0, "executed_seconds": 20.0}
            ],
        },
    ]
    by_job = {j.job: j for j in report.job_costs(runs)}

    assert by_job["workspace"].runs == 2
    assert by_job["workspace"].seconds == 140.0
    assert by_job["workspace"].executed_seconds == 50.0
    assert by_job["workspace"].build_seconds == 90.0
    assert by_job["workspace"].failures == 1

    tooling = by_job["repo tooling"]
    assert tooling.executed_seconds == 0.0
    assert tooling.build_seconds == 10.0  # the whole job, by libtest's blindness

    suite = report.suite_totals(runs)
    assert suite.runs == 2
    assert suite.seconds == 150.0
    assert suite.build_seconds == 100.0
    assert suite.build_share == 100.0 / 150.0


def test_the_page_renders_from_any_subset_of_the_five_ledgers():
    """The report must render for every subset of available ledgers.

    Empty and missing ledgers are both valid inputs, including non-recursive
    checkouts without the measurements submodule.
    """
    real = {name: report.load_jsonl(path) for name, path in report.LEDGERS.items()}
    names = list(real)
    empty = report.LedgerLoad(path=Path("x.jsonl"), rows=[])

    for present in [set(), {"compile_units"}, {"carve_lineage"}, {"run_tests_cost"}, set(names)]:
        loads = {name: (real[name] if name in present else empty) for name in names}
        page = report.render(loads)
        assert page.startswith("<!doctype html>")
        assert "</html>" in page


def test_a_missing_or_empty_ledger_loads_as_no_rows(tmp_path):
    """A fresh clone has none of these files and must still get a page."""
    missing = report.load_jsonl(tmp_path / "nope.jsonl")
    assert missing.rows == [] and missing.missing is True

    empty = tmp_path / "empty.jsonl"
    empty.write_text("", encoding="utf-8")
    assert report.load_jsonl(empty).rows == []

    ragged = tmp_path / "ragged.jsonl"
    ragged.write_text('{"a": 1}\n\n{not json}\n{"a": 2}\n', encoding="utf-8")
    loaded = report.load_jsonl(ragged)
    assert [r["a"] for r in loaded.rows] == [1, 2]
    assert loaded.malformed == 1
