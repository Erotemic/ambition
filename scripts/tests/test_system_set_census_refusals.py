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
