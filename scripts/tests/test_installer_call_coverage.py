"""The installer mutation runner cannot open a door around the build policy.

⛔ `--run` is fourteen full `app_it` builds. The first version invoked cargo
directly, with none of the target-binding or disk-headroom checks every other
build door in this repository goes through. A GPT review found it 2026-09-07.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_disk_headroom as guard  # noqa: E402


class _Done:
    returncode = 0
    stdout = ""
    stderr = ""


def _no_cargo(argv, *args, **kwargs):
    """The runner's `git diff` dirty-check is allowed; a cargo build is the failure.

    ⚠ The first draft stubbed EVERY subprocess and failed on the git call that
    precedes the policy -- the test was measuring its own fixture.
    """
    if argv and argv[0] == "cargo":
        pytest.fail("cargo was invoked before the build policy answered")
    return _Done()


def _no_mutation(*args, **kwargs):
    """⛔⛔ THE TRIP-WIRE, and it was added after the first poison of these tests
    MUTATED THE REAL TREE: with the preflight removed and cargo stubbed, the
    runner poisoned two installer lines in `combat_schedule.rs` and the stub's
    `pytest.fail` fired before the restore. A test that lets the subject reach
    the working tree is one that can leave it dirty when it fails."""
    pytest.fail("a source line was mutated before the build policy answered")


def _runner():
    spec = importlib.util.spec_from_file_location(
        "measure_installer_call_coverage",
        REPO / "scripts" / "measure_installer_call_coverage.py",
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_list_is_source_only_and_needs_no_bound_target(monkeypatch, capsys):
    """`--list` must work on a checkout whose target is not bound at all."""
    monkeypatch.setattr(guard, "_bindmount_check_exit_code", lambda: None)
    monkeypatch.setattr(sys, "argv", ["measure_installer_call_coverage.py", "--list"])
    assert _runner().main() == 0
    assert "subject(s)" in capsys.readouterr().out


def test_run_refuses_on_an_unbound_target_before_mutating_anything(monkeypatch, tmp_path):
    """⛔ EXIT 2 FROM THE POLICY, BEFORE THE FIRST EDIT. The sidecar is the record
    of a mutation in flight; it must not exist, because nothing was mutated."""
    monkeypatch.setattr(guard, "_bindmount_check_exit_code", lambda: 2)
    runner = _runner()
    sidecar = tmp_path / "sidecar.json"
    monkeypatch.setattr(runner, "SIDECAR", sidecar)
    monkeypatch.setattr(runner.subprocess, "run", _no_cargo)
    monkeypatch.setattr(runner, "_write_line", _no_mutation)
    monkeypatch.setattr(sys, "argv", ["measure_installer_call_coverage.py", "--run"])
    with pytest.raises(SystemExit) as exit_info:
        runner.main()
    assert exit_info.value.code == 2
    assert not sidecar.exists(), "a mutation was staged before the policy answered"


def test_run_refuses_below_the_disk_floor_before_mutating_anything(monkeypatch, tmp_path):
    """A bound target that is nearly full is refused the same way."""
    monkeypatch.setattr(guard, "_bindmount_check_exit_code", lambda: 0)
    monkeypatch.setattr(guard, "free_gb_on_target", lambda: guard.MIN_FREE_GB - 1.0)
    runner = _runner()
    sidecar = tmp_path / "sidecar.json"
    monkeypatch.setattr(runner, "SIDECAR", sidecar)
    monkeypatch.setattr(runner.subprocess, "run", _no_cargo)
    monkeypatch.setattr(runner, "_write_line", _no_mutation)
    monkeypatch.setattr(sys, "argv", ["measure_installer_call_coverage.py", "--run"])
    with pytest.raises(SystemExit) as exit_info:
        runner.main()
    assert exit_info.value.code == 2
    assert not sidecar.exists()
