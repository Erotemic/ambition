"""The repo tooling job runs `scripts/tests` on every core when it can.

It ran serially: measured 2026-10-10 on 14 cores, 1160 s against 232 s and
236 s with 8 workers, the same 1776 passed and 19 skipped. Each push waits on
this job, so the serial run was most of the wait.

Without pytest-xdist the job runs serially and says so on stderr: the same
tests run, so nothing is skipped.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def _tooling_argv() -> list[str]:
    return run_tests.repo_coupled_python_job().argv


def test_with_xdist_the_job_runs_on_every_core(monkeypatch):
    real = importlib.util.find_spec
    monkeypatch.setattr(
        importlib.util,
        "find_spec",
        lambda name, *rest: object() if name == "xdist" else real(name, *rest),
    )
    argv = _tooling_argv()
    assert "-n" in argv and argv[argv.index("-n") + 1] == "auto", f"the job runs serially: {argv}"


def test_without_xdist_the_job_runs_serially_and_says_so(monkeypatch, capsys):
    real = importlib.util.find_spec
    monkeypatch.setattr(
        importlib.util,
        "find_spec",
        lambda name, *rest: None if name == "xdist" else real(name, *rest),
    )
    argv = _tooling_argv()
    assert "-n" not in argv, argv
    assert "runs serially" in capsys.readouterr().err
    # The same suite and the same marker: only the workers differ.
    assert "scripts/tests" in argv
    assert argv[argv.index("-m", argv.index("pytest")) + 1] == "not detached_tool"


def _workers_seen(monkeypatch, tmp_path, job_limit=None) -> str:
    """The `PYTEST_XDIST_AUTO_NUM_WORKERS` a job gets from the real `run()`:
    what `-n auto` turns into a count of workers."""
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    monkeypatch.delenv("PYTEST_XDIST_AUTO_NUM_WORKERS", raising=False)
    seen = tmp_path / "workers"
    probe = f"import os; open({str(seen)!r}, 'w').write(os.environ.get('PYTEST_XDIST_AUTO_NUM_WORKERS', 'unset'))"
    run_tests.run(
        [run_tests.Job("repo tooling (scripts/tests)", [sys.executable, "-c", probe])],
        False,
        status_json=str(tmp_path / "status.json"),
        job_limit=job_limit,
    )
    return seen.read_text()


def test_the_workers_obey_the_cap_and_take_at_most_six_with_none(monkeypatch, tmp_path):
    """⭐ A worker pool on a shared machine does not take every CPU by default
    (at most 6 parallel jobs there, Jon 2026-10-03). `-j` caps the workers as
    it caps cargo's jobs and test threads; with no `-j`, at most
    `PYTEST_WORKERS_UNCAPPED`."""
    assert _workers_seen(monkeypatch, tmp_path, job_limit=3) == "3"
    uncapped = _workers_seen(monkeypatch, tmp_path)
    assert uncapped.isdigit() and 1 <= int(uncapped) <= run_tests.PYTEST_WORKERS_UNCAPPED == 6, (
        f"with no -j the pool is not capped: {uncapped}"
    )
