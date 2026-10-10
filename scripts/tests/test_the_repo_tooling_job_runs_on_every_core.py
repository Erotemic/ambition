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
