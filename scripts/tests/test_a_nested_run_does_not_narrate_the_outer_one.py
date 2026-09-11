"""A `run_tests.py` started INSIDE a job must not overwrite the shared status file.

⛔⛔ THE FAILURE IT PREVENTS, observed by the fighter lane 2026-09-06 while polling
during its own `--rust` run: `target/run_tests_status.json` read
`{"jobs": 1, "state": "done"}` while a five-job lane was mid-flight. `scripts/tests`
shells out to `run_tests.py`, the repo-tooling job runs those tests, so an inner
run overwrote the outer run's status DURING the outer run's first job.

⚠ THE WINDOW IS WHAT MAKES IT DANGEROUS. The bad value is transient — the outer run
corrects it on its next write — so a reader either side of the window sees nothing
wrong, and it lies in the GREEN direction: a waiter fires early and reports a lane
that never finished. That is the one live signal an agent has while a lane's stdout
sits buffered for fifteen minutes.

⭐ FIXED AT THE DEFAULT, NOT THE CALL SITES. One test already passed
`--status-json` to a tmp path, so the hazard was known and handled ONCE while every
other caller inherited the default — a constraint filed on the first case that
suffered it. A new test cannot reintroduce it by simply not knowing.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402

#: ⛔⛤ NARROWED TO ONE JOB, AND THE SUBJECT SURVIVES IT. These two tests each shell
#: out to a real nested run, and they used to take the whole `--maintenance` lane:
#: 70s apiece, 141s of the repo-tooling job's 292s — 48% of a job that runs in
#: EVERY lane, spent twice on the same question.
#:
#: ⛔ WHAT THEY NEED IS A RUN THAT ACTUALLY RUNS A JOB, which is the recorded
#: poison: the first version used `--list`, which plans and exits and writes no
#: status file at all, so it passed with the nesting guard deliberately disabled.
#: ONE real job writes the status exactly as five do, so the narration question is
#: unchanged and the price is not.
#:
#: ⚠ `zone names` is chosen for being pure Python and fastest, NOT for anything
#: about zones. A cargo-backed job (`-p <crate>`) was measured too and rejected:
#: 16s or 104s depending on build state, in a job that is deliberately
#: Python-only and runs FIRST.
ONE_JOB = ("--only-job", "zone names")



def _ran_far_enough(proc) -> None:
    """Fail only for reasons this file is ABOUT.

    ⛔⛔ THESE TESTS USED TO ASSERT `returncode == 0`, WHICH MADE THEM FAIL FOR
    REASONS THAT HAVE NOTHING TO DO WITH NESTING. `--maintenance` exits non-zero
    when the disk is under the suite's free-space floor — it refuses a job and
    says so — and on 2026-09-06 that reported itself as a NESTING-GUARD failure
    while the real cause was 40 GB free. The message a red test prints is the
    first thing its reader believes, and this one pointed at the wrong subsystem.

    ⇒ What these tests actually need is that the run got far enough to decide
    WHERE to narrate. A run refused for the environment never reached that
    decision and has nothing to say about it, so it skips with the real reason
    instead of failing with a false one.
    """
    import pytest

    if proc.returncode != 0:
        tail = (proc.stdout or "")[-1500:] + (proc.stderr or "")[-1500:]
        for reason in ("below the", "floor", "INCOMPLETE"):
            if reason in tail:
                pytest.skip(
                    "the suite refused for an environmental reason, not a nesting "
                    f"one, so this test has no subject: ...{tail[-300:]}"
                )
        raise AssertionError(tail)

def test_the_marker_names_the_outer_pid_so_a_run_knows_its_own() -> None:
    """The value is the OUTER pid: a run must not mistake its own marker for a
    parent's, or the top-level invocation would classify itself as nested."""
    assert run_tests.NESTED_ENV == "AMBITION_RUN_TESTS_OUTER_PID"


def test_a_nested_invocation_leaves_the_shared_status_file_alone(tmp_path) -> None:
    shared = REPO / "target" / run_tests.STATUS_NAME
    before = shared.read_text(encoding="utf-8") if shared.exists() else None

    # ⛔⛔ IT MUST ACTUALLY RUN JOBS. The first version of this test used
    # `--list`, which PLANS AND EXITS and never writes a status file at all — so
    # it passed with the nesting guard deliberately disabled. A test that
    # exercises the right path can still assert the wrong thing; the poison is
    # what said so.
    env = {**os.environ, run_tests.NESTED_ENV: "99999"}
    proc = subprocess.run(
        [
            sys.executable, str(REPO / "scripts" / "run_tests.py"),
            "--maintenance", *ONE_JOB,
        ],
        cwd=REPO, env=env, capture_output=True, text=True,
    )
    _ran_far_enough(proc)

    after = shared.read_text(encoding="utf-8") if shared.exists() else None
    assert after == before, (
        "a nested run overwrote the shared status file; a poller reading inside "
        "that window sees the inner run's job count and 'done'"
    )


def test_an_explicit_status_json_is_still_honoured_when_nested(tmp_path) -> None:
    """⚠ A caller that ASKS for a path has said where it wants the narration —
    the nesting guard must not silently redirect it."""
    target = tmp_path / "explicit.json"
    env = {**os.environ, run_tests.NESTED_ENV: "99999"}
    proc = subprocess.run(
        [
            sys.executable, str(REPO / "scripts" / "run_tests.py"),
            "--maintenance", *ONE_JOB, "--status-json", str(target),
        ],
        cwd=REPO, env=env, capture_output=True, text=True,
    )
    _ran_far_enough(proc)
    assert target.exists(), "an explicit --status-json was not written"
    assert json.loads(target.read_text())["state"] == "done"


def test_only_job_narrows_the_lane_to_the_named_job() -> None:
    """⭐ THE FLAG THESE TESTS NOW DEPEND ON, guarded where they use it."""
    proc = subprocess.run(
        [
            sys.executable, str(REPO / "scripts" / "run_tests.py"),
            "--maintenance", "--only-job", "zone names", "--list",
        ],
        cwd=REPO, capture_output=True, text=True,
    )
    assert proc.returncode == 0, proc.stderr
    assert "Planned 1 job(s)" in proc.stdout, proc.stdout


def test_only_job_refuses_rather_than_running_an_empty_plan() -> None:
    """⛔⛔ A FILTER THAT SELECTS NOTHING PRINTS `0/0 jobs passed`, which reads as
    success — this repository's most repeated instrument failure. The refusal also
    lists the lane's job names, because a reader who mistyped one needs them."""
    proc = subprocess.run(
        [
            sys.executable, str(REPO / "scripts" / "run_tests.py"),
            "--maintenance", "--only-job", "no-job-is-called-this",
        ],
        cwd=REPO, capture_output=True, text=True,
    )
    assert proc.returncode == 1, proc.stdout
    assert "REFUSING" in proc.stderr
    assert "zone names" in proc.stderr, "the refusal must name what it could have run"
