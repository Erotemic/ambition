"""D-LANE-UNRUNNABLE: a lane that COULD NOT RUN is not a lane that failed.

⛔⛔ THE RUN THAT PRODUCED THIS. On 2026-09-09 a `--rust` lane reported
`5/6 jobs passed` with `workspace doctests` FAILED, and the entire content of
that failure was

    error: extern location for bevy does not exist:
           target/debug/deps/libbevy-f87968c7af3ad766.rlib

— a stale build artifact left by the same commit's own manifest feature changes.
`cargo test --workspace --doc` immediately afterwards was clean. The job never
compiled a doctest, let alone ran one, so "FAILED" was a claim about the
repository that nothing had measured, and a reader could not tell it apart from a
real red. That is this row's sentence: report INCOMPLETE, not pass — and not
"failed" either.

⭐ THE THREE ARMS BELOW ARE EACH OTHER'S CONTROL. A classifier that called
everything unrunnable would pass the first and fail the second; one that called
nothing unrunnable would pass the second and fail the first; one that reported
incompleteness as SUCCESS would pass both and fail the third.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402

STALE_RLIB = (
    "error: extern location for bevy does not exist: "
    "/repo/target/debug/deps/libbevy-f87968c7af3ad766.rlib"
)


def _run_one(monkeypatch, tmp_path, script: str) -> tuple[int, dict]:
    """One job whose output and exit code are exactly what the script prints."""
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: 500.0)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    status = tmp_path / "status.json"
    rc = run_tests.run(
        [run_tests.Job("the only job", [sys.executable, "-c", script])],
        False,
        status_json=str(status),
    )
    return rc, json.loads(status.read_text())


def test_a_stale_artifact_is_reported_as_incomplete_rather_than_failed(
    monkeypatch, tmp_path
):
    rc, status = _run_one(
        monkeypatch,
        tmp_path,
        f"print({STALE_RLIB!r}); raise SystemExit(101)",
    )

    assert rc == 1, "an incomplete lane must still be non-zero — incomplete is not pass"
    assert status["exit_code"] == 1
    assert status["failed"] == [], (
        "a job that could not run was counted as a failing job, so the run "
        "reads as evidence about the code that nothing measured"
    )
    assert [row["job"] for row in status["unrunnable"]] == ["the only job"]
    assert "stale build artifact" in status["unrunnable"][0]["remedy"]
    # The per-job row carries it too, for a reader that walks `completed`.
    assert status["completed"][0]["unrunnable"]


def test_an_ordinary_test_failure_is_still_a_failure(monkeypatch, tmp_path):
    """⭐ THE CONTROL ARM, and the one that matters most: a signature broad enough
    to swallow a genuine red would convert every real failure into a shrug."""
    rc, status = _run_one(
        monkeypatch,
        tmp_path,
        "print('test result: FAILED. 1 passed; 1 failed'); raise SystemExit(101)",
    )

    assert rc == 1
    assert status["failed"] == ["the only job"]
    assert status["unrunnable"] == []
    assert "unrunnable" not in status["completed"][0]


def test_a_clean_run_says_nothing_about_either(monkeypatch, tmp_path):
    """⭐ THE GREEN CONTROL. A guard that reddens or annotates the normal path is
    worse than the gap it closes."""
    rc, status = _run_one(monkeypatch, tmp_path, "pass")

    assert rc == 0
    assert status["exit_code"] == 0
    assert status["failed"] == []
    assert status["unrunnable"] == []
    assert status["state"] == "done"


def test_the_signature_table_is_not_silently_empty():
    """⛔ THE ANTI-VACUITY FLOOR. `unrunnable_reason` returns `None` for
    everything if the table is empty, and both the first arm above and the whole
    feature would be satisfied by a table nobody filled in."""
    assert run_tests.UNRUNNABLE_SIGNATURES
    assert run_tests.unrunnable_reason(STALE_RLIB)
