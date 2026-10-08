"""A run that planned no web job must not report that the wasm CHECK ran.

`run_tests.py` appends the default web build CHECK only when
`wasm_target_installed()`. On a machine without `wasm32-unknown-unknown` the
plan silently contains no web job, every planned job passes, the return code is
zero — and the coverage footer said, unconditionally:

    - the wasm/web build LINK (the wasm CHECK ran)

The LINK branch has always warned when the target is missing. The CHECK branch
did not, and the footer spoke for both. Found by review 2026-09-02. Since
2026-10-08 a missing target plans both jobs as unrunnable (`Job.missing`), so
the status file and its reader say NOT RUN as well (Q59).

⛔ THIS IS THE FOURTH CONDITIONALLY-BLIND CHECK IN THIS REPO IN A DAY, and the
family is the point: a check that is CORRECT and does not run, reported as
though it did. The others were `--rust` skipping the whole Python lane, the web
job living only in the exhaustive plan, and `check_no_warnings.py` reusing a
build cache. Each was found by accident. This one is guarded.

⚠ The machine that runs this test usually HAS the target installed, so the
defective branch is unreachable here without forcing it — which is exactly why
it survived. `wasm_target_installed` is monkeypatched.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def test_the_footer_admits_the_web_build_was_never_checked():
    absent = run_tests.coverage_notice(
        exhaustive=False, filtered=False, web_check_planned=False
    )
    assert "the wasm CHECK ran" not in absent, (
        "the run planned no web job; claiming the CHECK ran is the defect"
    )
    assert "wasm32-unknown-unknown is not installed" in absent, (
        "and it must say WHY, so the reader can fix it"
    )


def test_the_footer_still_credits_a_check_that_was_planned():
    present = run_tests.coverage_notice(
        exhaustive=False, filtered=False, web_check_planned=True
    )
    assert "the wasm CHECK ran" in present, (
        "premise: with the job planned the original sentence is correct, and "
        "this test must not pass merely by the footer going silent"
    )


def test_a_missing_target_plans_the_check_as_not_run(monkeypatch, tmp_path):
    """⭐ Q59: THE SKIPPED CHECK IS IN THE RECEIPT, not only on the console.

    The plan printed `SKIPPING the web build CHECK` and left the job out, so
    the status file said `done` and `last_test_run.py` said every job passed.
    Now the job is planned with its remedy, is not executed, and the run is
    `incomplete` with the job in `unrunnable`.
    """
    monkeypatch.setattr(run_tests, "wasm_target_installed", lambda: False)
    jobs = run_tests.build_jobs(only=[], heavy=False, libtest_args=[])
    web = [j for j in jobs if "web build check" in j.name]
    assert len(web) == 1 and web[0].missing and "rustup target add" in web[0].missing, web

    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    status = tmp_path / "status.json"
    rc = run_tests.run(
        [run_tests.Job("a cheap job", [sys.executable, "-c", "pass"]), web[0]],
        False, status_json=str(status),
    )
    written = json.loads(status.read_text())
    assert rc != 0 and written["state"] == "incomplete", written
    assert [u["job"] for u in written["unrunnable"]] == [web[0].name], written


def test_the_job_is_planned_when_the_target_is_present(monkeypatch, capsys):
    monkeypatch.setattr(run_tests, "wasm_target_installed", lambda: True)
    jobs = run_tests.build_jobs(only=[], heavy=False, libtest_args=[])
    out = capsys.readouterr().out
    assert any("web build check" in j.name for j in jobs), (
        "positive control: the job IS planned when the target exists, so the "
        "test above is measuring the branch and not a permanently empty plan"
    )
    assert all(not j.missing for j in jobs if "web build check" in j.name)


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
