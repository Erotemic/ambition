"""`--only-job` narrows the PLAN, and every reporter must say so.

⛔⛔ **IT NARROWED ONE THING AND FOUR REPORTERS KEPT ANSWERING ABOUT THE LANE.**
The summary line, the coverage footer, the cost ledger and the status JSON each
re-derived "which lane is this" from the lane booleans, so a run of one selected
job printed `1/1 jobs passed  [lane: --rust]`, appended a one-job row to a ledger
of whole-lane costs, and closed with a paragraph explaining what `--rust` omits —
a sentence about a lane that had not run. The interpreter preflight was a fifth:
it read the plan BEFORE the filter, so selecting a pure-Rust job still aborted
the process over Python jobs the plan no longer contained.

⭐ THE FIX IS ONE VALUE, NOT FIVE EDITS. `RunScope` is built beside the filter
and the reporters read it. These tests exercise the REPORTERS rather than the
dataclass, because the dataclass was never the thing that was wrong.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402

LANE = run_tests.RunScope(lane="--rust", planned=7, selected=1,
                          only_job="repo tooling")
WHOLE = run_tests.RunScope(lane="--rust", planned=7, selected=7)


def _run(monkeypatch, tmp_path, **kwargs) -> tuple[int, dict, str]:
    """One trivially-passing job, run through the real `run()`."""
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    status = tmp_path / "status.json"
    rc = run_tests.run(
        [run_tests.Job("repo tooling (scripts/tests)", [sys.executable, "-c", "pass"]),
         run_tests.Job("workspace (default features)", [sys.executable, "-c", "pass"])],
        False,
        status_json=str(status),
        **kwargs,
    )
    return rc, json.loads(status.read_text()), status


def test_the_summary_line_carries_the_narrowing_not_just_the_lane(
    monkeypatch, tmp_path, capsys
):
    """⛔ THE SUMMARY LINE IS THE ONE THAT TRAVELS. It gets quoted into a commit
    message and a planning row on its own, so `[lane: --rust]` on a one-job run
    is the claim that does the damage."""
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    _run(monkeypatch, tmp_path, rust_only=True, only_job="repo tooling")
    out = capsys.readouterr().out
    assert "1/1 jobs passed" in out, out
    assert "--only-job 'repo tooling'" in out, (
        "the summary line named the lane and not the selection:\n" + out
    )
    assert "1 of 2 jobs" in out, out


def test_an_unnarrowed_run_names_the_lane_alone(monkeypatch, tmp_path, capsys):
    """⭐ THE CONTROL ARM. A label that always mentioned a selection would pass
    the test above while making every ordinary run's summary unreadable."""
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    _run(monkeypatch, tmp_path, rust_only=True)
    out = capsys.readouterr().out
    assert "[lane: --rust]" in out, out
    assert "--only-job" not in out.split("jobs passed")[1][:200], out


def test_the_coverage_footer_leads_with_the_narrowing(monkeypatch, tmp_path, capsys):
    """The footer's whole job is stating what was NOT checked, and "the lane did
    not run" is a larger omission than anything else on the page."""
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    _run(monkeypatch, tmp_path, rust_only=True, only_job="repo tooling")
    out = capsys.readouterr().out
    assert "NOT THE `--rust` LANE" in out, out
    # Before the paragraph that describes what the whole lane omits.
    assert out.index("NOT THE `--rust` LANE") < out.index("Get them with"), (
        "the narrowing notice printed after the lane's own omission list, so a "
        "reader meets the lane's caveats before learning the lane did not run"
    )


def test_the_notice_is_absent_when_nothing_was_narrowed():
    """⭐ POISON FOR THE NOTICE ITSELF."""
    assert WHOLE.narrowing_notice() == ""
    assert LANE.narrowing_notice() != ""
    notice = run_tests.coverage_notice(False, False, rust_only=True, run_scope=WHOLE)
    assert "SELECTED JOB" not in notice, notice


def test_the_cost_ledger_refuses_a_narrowed_run(monkeypatch, tmp_path, capsys):
    """⛔⛔ EVERY ROW IN THAT LEDGER IS READ AS "WHAT THIS LANE COSTS". A
    `--only-job` row carries the lane's own flags and one job's seconds, so a
    reader averaging `rust_only` rows would take a deliberate narrowing as
    evidence that the lane got cheaper."""
    monkeypatch.setenv(run_tests.COST_LEDGER_TEST_WRITES_ENV, "1")
    ledger = tmp_path / "cost.jsonl"
    monkeypatch.setenv("RUN_TESTS_COST_LEDGER", str(ledger))
    results = [run_tests.JobResult("a job", ["x"], True, 1.0, None, None)]

    assert run_tests.append_cost_ledger(results, False, False, rust_only=True,
                                        scope=LANE) is None
    assert not ledger.exists(), "a narrowed run wrote a lane-cost row"
    assert "cost NOT recorded" in capsys.readouterr().out

    # The control: the same call without narrowing DOES record, so the refusal
    # above is about the scope and not about the fixture.
    assert run_tests.append_cost_ledger(results, False, False, rust_only=True,
                                        scope=WHOLE) == ledger
    assert ledger.exists()


def test_the_status_file_carries_the_narrowing_for_a_poller(monkeypatch, tmp_path):
    """⛔ THE POLLERS DO NOT READ THE TERMINAL. `{"rust_only": true, "jobs": 1}`
    with nothing else reads as a lane that collapsed to one job, not as a lane
    that was deliberately filtered."""
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    _, status, _ = _run(monkeypatch, tmp_path, rust_only=True,
                        only_job="repo tooling")
    assert status["lane"] == "--rust"
    assert status["only_job"] == "repo tooling"
    assert status["planned_jobs"] == 2
    assert status["jobs"] == 1


def test_the_interpreter_preflight_reads_the_selected_plan_not_the_lane(
    monkeypatch, tmp_path
):
    """⛔⛔ THE PREFLIGHT'S OWN DOCSTRING PROMISES IT REFUSES ONLY WHEN THE PLAN
    HOLDS PYTHON JOBS. Asked before the filter, "the plan" was the lane — so
    selecting a pure-Rust job on a machine whose interpreter lacks the Python
    lane's modules aborted the process at exit 2 over jobs that had already been
    dropped.
    """
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    monkeypatch.setattr(run_tests, "SCRIPTS_ENV_MODULES",
                        ["a_module_no_interpreter_has"])
    jobs = [
        run_tests.Job("python lane", [sys.executable, "-m", "pytest", "--version"]),
        run_tests.Job("workspace (default features)", ["/bin/true"]),
    ]

    # Selecting the CARGO job leaves no Python job in the plan, so nothing is
    # refused — and the job actually runs.
    rc = run_tests.run(jobs, False, status_json=str(tmp_path / "a.json"),
                       only_job="workspace")
    assert rc == 0, "the preflight refused over a job the selection had dropped"

    # The control arm: with the Python job in the plan it still refuses.
    with pytest.raises(SystemExit) as raised:
        run_tests.run(jobs, False, status_json=str(tmp_path / "b.json"))
    assert raised.value.code == 2
