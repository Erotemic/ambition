"""`-j` has to reach the CHILD process, and cap both consumers.

⭐ The cap is set ONCE, in the environment, rather than on the ~20 `[CARGO, ...]`
argv sites -- so a job added later is capped without anyone remembering to. That
is only true if the environment actually arrives, which is what these run.

⛔⛔ And it must cap BOTH consumers. `CARGO_BUILD_JOBS` bounds rustc processes;
a test BINARY then spawns its own threads, so capping only the compile still
lets one binary saturate a shared machine.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import run_tests  # noqa: E402

#: Print the three variables the cap writes, so the assertions read the CHILD's
#: view rather than this process's.
PROBE = (
    "import os; print('CAP', "
    "os.environ.get('CARGO_BUILD_JOBS'), "
    "os.environ.get('RUST_TEST_THREADS'), "
    "os.environ.get('NEXTEST_TEST_THREADS'))"
)


def _probe_job():
    return run_tests.Job("cap probe", [sys.executable, "-c", PROBE])


def _run(capsys, **kwargs):
    code = run_tests.run([_probe_job()], False, **kwargs)
    return code, capsys.readouterr().out


def test_a_cap_reaches_the_child_for_both_compile_and_test_threads(capsys):
    code, out = _run(capsys, job_limit=5)
    assert code == 0, out
    assert "CAP 5 5 5" in out, (
        "the cap did not reach the child; an argv-only cap would look like this"
    )


def test_without_the_flag_nothing_is_capped(capsys):
    """⚠ The default must stay 'every core'. A cap that leaked in unasked would
    silently halve everyone's suite and read as a machine getting slower."""
    code, out = _run(capsys)
    assert code == 0, out
    assert "CAP None None None" in out, out


def test_the_run_announces_the_cap(capsys):
    """A capped run's wall clock is not comparable with an uncapped one, so the
    run says so where the timings are read, not only in `--help`."""
    _, out = _run(capsys, job_limit=3)
    assert "-j3" in out
    assert "NOT comparable" in out


def test_an_explicit_cap_beats_an_ambient_one(capsys, monkeypatch):
    """⛔ `-j` is the caller telling THIS machine what it may use. A
    `setdefault` would let a stale exported `CARGO_BUILD_JOBS` win over the flag
    the human just typed."""
    monkeypatch.setenv("CARGO_BUILD_JOBS", "64")
    _, out = _run(capsys, job_limit=2)
    assert "CAP 2 2 2" in out, out


@pytest.mark.parametrize("bad", ["0", "-1"])
def test_a_nonsense_cap_is_refused(bad):
    """`-j0` is not 'unlimited' to cargo, it is an error -- and refusing it here
    names the flag rather than surfacing a cargo diagnostic three jobs later."""
    import subprocess

    proc = subprocess.run(
        [sys.executable, str(REPO / "scripts" / "run_tests.py"), "-j", bad, "--list"],
        capture_output=True, text=True,
    )
    assert proc.returncode != 0
    assert "at least 1" in proc.stderr, proc.stderr
