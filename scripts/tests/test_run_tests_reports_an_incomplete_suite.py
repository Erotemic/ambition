"""⛔⛔ A SUITE THAT STOPPED EARLY MUST NOT BE READABLE AS A PASSING ONE.

`run_tests.py` refuses to START the next job when the target volume drops below
a hard floor — a real 49-job run exhausted a 290 GB volume partway through and
reported the wreckage as `error: linking with clang failed`. Stopping is the
right call. **Reporting it is a separate problem, and the first version got it
wrong in exactly one place.**

⚠ THE DEFECT THIS FILE PINS (found in review 2026-09-03, in code written the
same day): the abort returned `1` to its shell, and the shell was the only
reader that learned anything. `write_status` ran FIRST and wrote
`state="done", exit_code=0`, because every job that actually ran had passed. A
long autonomous run polls that file rather than the exit code — that is what it
is FOR — so an abandoned suite left a green record behind it.

⭐ Hence three assertions, not one: the shell rc, the serialized state, AND the
serialized exit code. Any one of them alone was satisfied by the broken version.

⭐⭐ AND `scripts/last_test_run.py` IS THE READER THAT MATTERS, because it is the
one agents ask "did the suite pass?". It believed the file: with all completed
jobs `ok`, a fresh mtime and an unmoved tree, it printed `all N jobs passed` and
returned 0 for a plan that never finished. Fixing only the writer would have
left the consumer answering the wrong question the moment anything else wrote a
non-`done` state.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402


def _two_trivial_jobs() -> list:
    """Two jobs that cost nothing, so the abort — not the work — is under test."""
    return [
        run_tests.Job("first job", [sys.executable, "-c", "pass"]),
        run_tests.Job("second job", [sys.executable, "-c", "pass"]),
    ]


def _run_with_free_space(monkeypatch, tmp_path, free_gb_sequence):
    """Drive the runner with a scripted disk reading, one per call.

    ⚠ The readings are consumed in this order: the up-front headroom refusal,
    then one per job before it starts, then one for the summary's disk delta.
    An off-by-one here silently tests the WRONG job, so the abort cases below
    assert which job was refused by name rather than trusting the position.
    """
    readings = list(free_gb_sequence)

    def fake_free() -> float:
        # The last reading repeats: the summary asks again after the loop.
        return readings.pop(0) if len(readings) > 1 else readings[0]

    monkeypatch.setattr(run_tests, "free_gb_on_target", fake_free)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    status = tmp_path / "status.json"
    rc = run_tests.run(_two_trivial_jobs(), False, status_json=str(status))
    return rc, json.loads(status.read_text())


def test_a_disk_abort_is_serialized_as_incomplete_not_merely_returned(
    monkeypatch, tmp_path
):
    """Plenty of room for job one, under the hard floor before job two."""
    rc, status = _run_with_free_space(
        monkeypatch, tmp_path, [500.0, 500.0, run_tests.ABORT_FREE_GB - 0.5]
    )

    assert rc == 1, "the shell must see a failure"
    # ⛔ THE THREE THAT THE BROKEN VERSION SPLIT APART.
    assert status["state"] == "aborted", (
        "the status file said `done` for a suite that stopped early — a waiter "
        "reading it would record a green run that never finished"
    )
    assert status["exit_code"] == 1, (
        "the serialized exit code disagreed with the one the shell got"
    )
    # It must say WHERE it stopped and how much never happened; a bare
    # `aborted` leaves a reader unable to judge what the run does cover.
    assert status["aborted_on_disk"] == "second job"
    assert status["never_ran"] == 1
    assert status["finished_jobs"] == 1
    assert status["failed"] == [], (
        "the job that ran really did pass — the incompleteness is the whole "
        "finding, and inventing a failed job would misreport it"
    )


def test_a_complete_run_is_still_plainly_done(monkeypatch, tmp_path):
    """⭐ THE CONTROL ARM. A guard that reddens the normal path is worse.

    Both jobs run with room to spare, so nothing here is aborted and the file
    must carry the ordinary green vocabulary.
    """
    rc, status = _run_with_free_space(monkeypatch, tmp_path, [500.0, 500.0, 480.0, 460.0])

    assert rc == 0
    assert status["state"] == "done"
    assert status["exit_code"] == 0
    assert status["aborted_on_disk"] is None
    assert status["never_ran"] == 0
    assert status["finished_jobs"] == 2


def test_a_maintenance_lane_that_builds_is_not_exempt_from_the_floor(
    monkeypatch, tmp_path
):
    """⛔⛔ THE EXEMPTION IS FOR LANES THAT DO NOT BUILD, AND ONE OF THEM DOES.

    `--tool-tests` and `--maintenance` skip the disk floor because hygiene
    audits are pure Python. ⚠ `--maintenance`'s intra-doc-link ratchet runs
    `cargo doc -p <crate> --no-deps` for every crate — so the lane the guard
    waves through is one that can still fill the volume. Found 2026-09-03 by
    running it on a box at 12 GB, hours after writing the exemption.

    An argv check cannot catch this: the argv is
    `python3 scripts/check_doc_link_ratchet.py`. The `builds` flag on the job
    is what the guard reads, and this pins that it is read at all.
    """
    jobs = [
        run_tests.Job("pure python audit", [sys.executable, "-c", "pass"]),
        run_tests.Job("cold cargo doc", [sys.executable, "-c", "pass"], builds=True),
    ]
    readings = [500.0, 500.0, run_tests.ABORT_FREE_GB - 0.5]

    def fake_free() -> float:
        return readings.pop(0) if len(readings) > 1 else readings[0]

    monkeypatch.setattr(run_tests, "free_gb_on_target", fake_free)
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    status = tmp_path / "status.json"
    rc = run_tests.run(jobs, False, status_json=str(status), maintenance_only=True)
    written = json.loads(status.read_text())

    assert rc == 1, "the building job was started on a volume under the floor"
    assert written["aborted_on_disk"] == "cold cargo doc"


def test_a_maintenance_lane_that_only_reads_keeps_its_exemption(
    monkeypatch, tmp_path
):
    """⭐ THE CONTROL ARM, and it is the reason the flag exists at all.

    Guarding the whole lane would make a full disk block the pure-Python
    audits — the checks most worth running when the volume is full, since they
    are the only ones that still can.
    """
    jobs = [
        run_tests.Job("pure python audit", [sys.executable, "-c", "pass"]),
        run_tests.Job("another pure audit", [sys.executable, "-c", "pass"]),
    ]
    readings = [1.0, 1.0, 1.0]
    monkeypatch.setattr(run_tests, "free_gb_on_target", lambda: readings[0])
    monkeypatch.setattr(run_tests, "append_cost_ledger", lambda *a, **k: None)
    status = tmp_path / "status.json"
    rc = run_tests.run(jobs, False, status_json=str(status), maintenance_only=True)

    assert rc == 0, "a pure-Python lane must still run on a full volume"
    assert json.loads(status.read_text())["state"] == "done"


def _last_test_run(status_path: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(REPO / "scripts" / "last_test_run.py"),
         "--status-json", str(status_path), "--max-age", "600"],
        capture_output=True, text=True,
    )


def test_the_reader_agents_actually_use_refuses_an_aborted_run(
    monkeypatch, tmp_path
):
    """⭐⭐ THE END THAT GETS READ.

    `last_test_run.py` exists to answer "did the suite pass?" for someone who
    was not watching. Every check it makes was satisfied by the aborted run —
    fresh, unmoved tree, all recorded jobs `ok` — so it printed `all 1 jobs
    passed`. Refusal, not a verdict, is the only correct answer for a plan that
    stopped two thirds of the way through.
    """
    _, status = _run_with_free_space(
        monkeypatch, tmp_path, [500.0, 500.0, run_tests.ABORT_FREE_GB - 0.5]
    )
    path = tmp_path / "for_reader.json"
    path.write_text(json.dumps(status))

    proc = _last_test_run(path)

    assert proc.returncode == 2, (
        f"an incomplete suite was reported as a verdict:\n{proc.stdout}"
    )
    assert "REFUSED" in proc.stdout
    assert "second job" in proc.stdout, "it must name where the run stopped"
    assert "passed" not in proc.stdout.split("REFUSED")[0].splitlines()[-1]
