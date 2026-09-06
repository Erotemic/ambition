"""`sync_status.sh` must look at worktrees whose path contains a space.

The listing it parsed is `<path> <sha> [branch]`, and it took `${line%% *}` —
so a path with a space in it became its first word, `[[ -d ]]` failed, and the
whole worktree was skipped in SILENCE. That is the one failure a status tool
cannot have: the reason to run it is to trust "nothing to report", and it was
reporting nothing about a directory it never opened.

Also pins the exit-code contract the header states: 0 clean, 1 needs attention.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
SYNC_STATUS = REPO / "scripts/sync_status.sh"


def _git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


def _repo_with_a_worktree(root: Path, worktree_name: str) -> Path:
    work = root / "main"
    work.mkdir()
    _git(work, "init", "-q", "-b", "main", ".")
    _git(work, "config", "user.email", "t@example.com")
    _git(work, "config", "user.name", "t")
    (work / "seed.txt").write_text("seed\n", encoding="utf8")
    _git(work, "add", "seed.txt")
    _git(work, "commit", "-qm", "seed")

    side = root / worktree_name
    _git(work, "worktree", "add", "-q", "-b", "side", str(side))
    return work


def _run(work: Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(SYNC_STATUS), "--no-fetch", "--quiet"],
        cwd=work,
        capture_output=True,
        text=True,
    )


@pytest.mark.skipif(shutil.which("git") is None, reason="git is not installed")
def test_a_clean_repo_and_worktree_exit_zero(tmp_path: Path) -> None:
    work = _repo_with_a_worktree(tmp_path, "plain")
    done = _run(work)
    assert done.returncode == 0, done.stdout + done.stderr
    assert "✔" in done.stdout


@pytest.mark.skipif(shutil.which("git") is None, reason="git is not installed")
def test_uncommitted_work_in_a_worktree_whose_path_has_a_space_is_reported(
    tmp_path: Path,
) -> None:
    work = _repo_with_a_worktree(tmp_path, "a side tree")
    (tmp_path / "a side tree" / "unsaved.txt").write_text("work\n", encoding="utf8")

    done = _run(work)

    assert "a side tree" in done.stdout, (
        "the worktree whose path contains a space was never inspected:\n"
        + done.stdout
        + done.stderr
    )
    assert "1 uncommitted file(s)" in done.stdout, (
        "the worktree was named but its uncommitted work was not counted "
        f"(--quiet suppresses the file list, not the count):\n{done.stdout}"
    )
    # ⛔ THE EXIT CODE IS A VERDICT, NOT A COUNT — the header once promised the
    # number of problems, which collides with the 2 this script uses for "could
    # not answer" and wraps at 256 besides.
    assert done.returncode == 1, done.stdout + done.stderr


@pytest.mark.skipif(shutil.which("git") is None, reason="git is not installed")
def test_a_directory_that_is_not_a_repo_is_an_error_not_a_clean_bill(
    tmp_path: Path,
) -> None:
    outside = tmp_path / "not-a-repo"
    outside.mkdir()
    done = subprocess.run(
        [str(SYNC_STATUS), "--no-fetch", "--quiet"],
        cwd=outside,
        capture_output=True,
        text=True,
        env={"PATH": "/usr/bin:/bin", "HOME": str(tmp_path), "GIT_CEILING_DIRECTORIES": str(tmp_path)},
    )
    assert done.returncode == 2, done.stdout + done.stderr
