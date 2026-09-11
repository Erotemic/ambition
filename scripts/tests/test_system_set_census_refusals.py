"""The SystemSet census must refuse rather than report a confident zero.

⛔⛔ **THE FAILURE THIS PINS IS A CLEAN BILL FROM AN EMPTY CORPUS.** In a checkout
where `git ls-files` refused — a missing `safe.directory` entry — the script
printed *"0 SystemSet types declared, 0 with ZERO members"* and exited 0. Every
bucket collapsing to zero is the empty-corpus tell, and the one reader who cannot
apply that judgement is the script itself.

⚠ Each refusal is exercised by BREAKING the input, not by reading the branch. A
guard that is only read has not been shown to fire.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
SCRIPT = REPO / "scripts" / "measure_system_sets_without_members.py"

import measure_system_sets_without_members as census  # noqa: E402


def _fake_git(returncode: int, stdout: str = "", stderr: str = ""):
    def run(*_args, **_kwargs):
        return subprocess.CompletedProcess([], returncode, stdout, stderr)

    return run


def test_a_failing_git_refuses_instead_of_reporting_zero(monkeypatch, capsys):
    monkeypatch.setattr(
        census.subprocess, "run", _fake_git(128, stderr="dubious ownership")
    )
    with pytest.raises(SystemExit) as exc:
        census.main()
    message = str(exc.value)
    assert "git ls-files failed" in message
    assert "dubious ownership" in message, "the refusal must carry git's own reason"
    assert "0 SystemSet types declared" not in capsys.readouterr().out


def test_an_empty_file_list_refuses(monkeypatch):
    monkeypatch.setattr(census.subprocess, "run", _fake_git(0, stdout=""))
    with pytest.raises(SystemExit) as exc:
        census.main()
    assert "listed no .rs files" in str(exc.value)


def test_a_corpus_with_no_declarations_refuses(monkeypatch, tmp_path):
    """⭐ THE ANTI-VACUITY FLOOR. "No unwired set" over zero declarations is a true
    statement about nothing, and it reads exactly like a pass."""
    (tmp_path / "plain.rs").write_text("fn main() {}\n")
    monkeypatch.setattr(census, "REPO", tmp_path)
    monkeypatch.setattr(census.subprocess, "run", _fake_git(0, stdout="plain.rs\n"))
    with pytest.raises(SystemExit) as exc:
        census.main()
    assert "NOT ONE declares" in str(exc.value)


def test_a_repeated_name_is_printed_because_its_memberships_pool(
    monkeypatch, tmp_path, capsys
):
    """⛔ A repeated name hides an EMPTY set behind a populated sibling.

    Membership is matched as `in_set(...Name)` across the corpus, so two sets
    sharing a name cannot be told apart. `dead.rs` declares an `Umbrella` that
    nothing joins; `live.rs` declares another that two systems join. The pooled
    count is 2, so the empty one never reaches the ZERO bucket — which is the
    whole point of the script. It must at least SAY so.
    """
    (tmp_path / "dead.rs").write_text("#[derive(SystemSet)]\nstruct Umbrella;\n")
    (tmp_path / "live.rs").write_text(
        "#[derive(SystemSet)]\npub struct Umbrella;\n"
        "fn wire() { a.in_set(Umbrella); b.in_set(Umbrella); }\n"
    )
    monkeypatch.setattr(census, "REPO", tmp_path)
    monkeypatch.setattr(
        census.subprocess, "run", _fake_git(0, stdout="dead.rs\nlive.rs\n")
    )
    census.main()
    out = capsys.readouterr().out
    assert "2 declarations collapsed to 1 names" in out
    assert "Umbrella" in out
    assert "dead.rs" in out and "live.rs" in out
    assert "1 with ZERO" not in out, (
        "the pooled count hides the empty one; this test documents that, and the "
        "collapse notice is the only warning the reader gets"
    )


def test_this_repository_declares_no_system_set_with_zero_members():
    """⛔⛤ THE CENSUS IS ALSO THE GATE, because the finding was a single instance.

    `PlatformerRuntimeSet` was the ONE empty set of 134 and it is deleted
    (2026-09-11). A ratchet over a population of one is a floor at zero, so this
    asks the instrument for the repository's own answer rather than pinning a
    number that would only ever be re-derived.

    ⚠ THE ANTI-VACUITY ARM IS THE DECLARATION COUNT, not the emptiness. The
    script's own docstring records it printing `0 SystemSet types declared, 0 with
    ZERO members` from a corpus `git ls-files` had refused to produce — a clean
    bill from nothing. It exits 1 on that now, and this floors the population too,
    so a corpus that lost its subject cannot pass by having none.
    """
    proc = subprocess.run(
        [sys.executable, str(SCRIPT)], cwd=str(REPO), capture_output=True, text=True
    )
    assert proc.returncode == 0, proc.stderr
    declared = int(proc.stdout.splitlines()[0].split()[0])
    assert declared >= 100, (
        f"only {declared} SystemSet declarations found. This gate is meaningless "
        "over a corpus that lost its subject — check the file list before reading "
        "the verdict below it."
    )
    assert "0 with ZERO `in_set(` members" in proc.stdout, (
        "a declared `SystemSet` has no members. `.after(<that set>)` is a SILENT "
        "no-op: not a compile error, not a warning, nothing at the call site, and "
        "the only symptom is a measurement that looks too tidy.\n" + proc.stdout
    )
