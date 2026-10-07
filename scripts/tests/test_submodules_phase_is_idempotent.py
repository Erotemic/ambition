"""`scripts/setup/submodules.sh` puts submodules on `main` and never loses work.

The policy (docs/submodules.md): every submodule is on `main`, and a pin that
disagrees with a submodule's `main` is a pin to update. The phase moves a clean
checkout onto `origin/main` only when that cannot orphan a commit; unpushed,
divergent and dirty checkouts are left alone and reported.

⛔⛔ THE FAILURE THIS PINS IS NOT HYPOTHETICAL. A bare
`git submodule update --init --recursive` moves every submodule to the recorded
gitlink and detaches whatever branch was there. On 2026-09-02 a setup run
reverted an in-progress fix in `ambition_music_renderer`, the next asset regen
failed with the exact error that fix removes, and a commit made afterwards
landed on the detached HEAD where no branch could reach it — recovered only from
the reflog.

⭐ These are BEHAVIOURAL fixtures over real git repositories, not assertions
about the script's text: each builds a superproject plus a submodule, runs the
phase, and asks what happened to the checkout.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent.parent
PHASE = REPO / "scripts/setup/submodules.sh"


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, check=True
    ).stdout.strip()


def _init(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-q", "-b", "main")
    git(path, "config", "user.email", "t@t")
    git(path, "config", "user.name", "t")
    # Local file:// submodules need this under recent git.
    git(path, "config", "protocol.file.allow", "always")


def _commit(path: Path, name: str, body: str = "x") -> str:
    (path / name).write_text(body)
    git(path, "add", "-A")
    git(path, "commit", "-qm", f"add {name}")
    return git(path, "rev-parse", "HEAD")


@pytest.fixture
def super_and_sub(tmp_path: Path):
    """A superproject with one submodule, plus the phase script copied in."""
    sub = tmp_path / "sub"
    _init(sub)
    _commit(sub, "a.txt")

    top = tmp_path / "top"
    _init(top)
    _commit(top, "root.txt")
    git(top, "-c", "protocol.file.allow=always", "submodule", "add", "-q",
        str(sub), "vendor/sub")
    git(top, "commit", "-qm", "add submodule")

    (top / "scripts/setup").mkdir(parents=True)
    (top / "scripts/lib").mkdir(parents=True)
    shutil.copy(PHASE, top / "scripts/setup/submodules.sh")
    shutil.copy(REPO / "scripts/lib/setup_common.sh", top / "scripts/lib/setup_common.sh")
    return top, sub


def run_phase(top: Path):
    env = dict(os.environ)
    env["GIT_ALLOW_PROTOCOL"] = "file"
    return subprocess.run(
        ["bash", "scripts/setup/submodules.sh"],
        cwd=top, capture_output=True, text=True, env=env,
    )


def test_an_uninitialized_submodule_is_initialized_at_its_gitlink(super_and_sub):
    """The one case the phase exists for."""
    top, _ = super_and_sub
    shutil.rmtree(top / "vendor/sub")
    (top / "vendor/sub").mkdir()
    assert not (top / "vendor/sub/.git").exists()

    result = run_phase(top)
    assert result.returncode == 0, result.stderr

    recorded = git(top, "ls-files", "--stage", "vendor/sub").split()[1]
    assert (top / "vendor/sub/.git").exists()
    assert git(top / "vendor/sub", "rev-parse", "HEAD") == recorded


def test_an_initialized_submodule_ahead_of_its_gitlink_is_left_exactly_alone(super_and_sub):
    """⛔ The regression. A branch with work on it is not the phase's to move."""
    top, _ = super_and_sub
    sub_wt = top / "vendor/sub"
    git(sub_wt, "config", "user.email", "t@t")
    git(sub_wt, "config", "user.name", "t")
    git(sub_wt, "checkout", "-q", "-b", "agent/work")
    ahead = _commit(sub_wt, "b.txt")

    recorded = git(top, "ls-files", "--stage", "vendor/sub").split()[1]
    assert ahead != recorded, "fixture must actually be ahead of the gitlink"

    result = run_phase(top)
    assert result.returncode == 0, result.stderr

    assert git(sub_wt, "rev-parse", "HEAD") == ahead, "the checkout was moved"
    assert git(sub_wt, "branch", "--show-current") == "agent/work", "the branch was detached"
    # and it says so rather than silently leaving a mismatch
    assert "LEFT ALONE" in result.stderr


def test_a_dirty_submodule_is_never_touched(super_and_sub):
    """Uncommitted work is the case with no recovery at all — not even a reflog."""
    top, _ = super_and_sub
    sub_wt = top / "vendor/sub"
    (sub_wt / "a.txt").write_text("uncommitted edit that exists nowhere else")
    head_before = git(sub_wt, "rev-parse", "HEAD")

    result = run_phase(top)
    assert result.returncode == 0, result.stderr

    assert git(sub_wt, "rev-parse", "HEAD") == head_before
    assert (sub_wt / "a.txt").read_text() == "uncommitted edit that exists nowhere else"


# --- the `main` policy -------------------------------------------------------


def _advance_remote(sub: Path, name: str = "b.txt") -> str:
    """The submodule's own origin moves on, as main does between pin bumps."""
    return _commit(sub, name)


def _wt(top: Path) -> Path:
    wt = top / "vendor/sub"
    git(wt, "config", "user.email", "t@t")
    git(wt, "config", "user.name", "t")
    return wt


def test_a_detached_checkout_at_main_is_attached_to_main_without_moving(super_and_sub):
    top, _ = super_and_sub
    wt = _wt(top)
    git(wt, "checkout", "-q", "--detach")
    head = git(wt, "rev-parse", "HEAD")
    result = run_phase(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "branch", "--show-current") == "main"
    assert git(wt, "rev-parse", "HEAD") == head


def test_a_clean_checkout_behind_main_is_fast_forwarded_and_the_stale_pin_is_reported(super_and_sub):
    """DEVELOPMENT MODE: the superproject is on `main`, so a stale gitlink is
    bookkeeping and the submodule follows its own origin/main."""
    top, sub = super_and_sub
    assert git(top, "branch", "--show-current") == "main", "the premise: active development"
    wt = _wt(top)
    new = _advance_remote(sub)
    result = run_phase(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == new
    assert git(wt, "branch", "--show-current") == "main"
    # the checkout moved; the SUPERPROJECT pin did not, and the phase says so
    assert git(top, "ls-files", "--stage", "vendor/sub").split()[1] != new
    assert "the PIN needs updating" in result.stderr
    assert "--bump-pins" in result.stdout


def test_bump_pins_stages_the_gitlink_and_commits_nothing(super_and_sub):
    top, sub = super_and_sub
    head_before = git(top, "rev-parse", "HEAD")
    new = _advance_remote(sub)
    env = dict(os.environ, GIT_ALLOW_PROTOCOL="file")
    result = subprocess.run(
        ["bash", "scripts/setup/submodules.sh", "--bump-pins"],
        cwd=top, capture_output=True, text=True, env=env,
    )
    assert result.returncode == 0, result.stderr
    assert git(top, "diff", "--cached", "--raw", "vendor/sub").split()[3].startswith(new[:7])
    assert git(top, "rev-parse", "HEAD") == head_before, "the phase committed"


def test_a_checkout_on_the_pre_split_history_is_kept_under_a_ref_and_moved_to_main(super_and_sub):
    """The split was intentional, so an old lineage is an expected state."""
    top, sub = super_and_sub
    wt = _wt(top)
    old = git(wt, "rev-parse", "HEAD")
    # origin's main is rewritten onto a new root, as at the split
    git(sub, "checkout", "-q", "--orphan", "split")
    _commit(sub, "fresh.txt")
    git(sub, "branch", "-M", "main")
    new = git(sub, "rev-parse", "HEAD")
    assert git(wt, "fetch", "-q", "origin") == ""
    assert subprocess.run(["git", "merge-base", old, new], cwd=wt, capture_output=True, text=True).stdout.strip() == ""

    result = run_phase(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == new
    assert git(wt, "branch", "--show-current") == "main"
    assert git(wt, "rev-parse", f"refs/backup/pre-split/{old[:12]}") == old, "the old position was not kept"
    assert "pre-split" in result.stdout


def test_a_dirty_checkout_on_the_pre_split_history_is_left_alone(super_and_sub):
    top, sub = super_and_sub
    wt = _wt(top)
    old = git(wt, "rev-parse", "HEAD")
    (wt / "a.txt").write_text("uncommitted")
    git(sub, "checkout", "-q", "--orphan", "split")
    _commit(sub, "fresh.txt")
    git(sub, "branch", "-M", "main")
    result = run_phase(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == old
    assert (wt / "a.txt").read_text() == "uncommitted"
    assert "LEFT ALONE" in result.stderr


def test_a_diverged_checkout_is_left_alone_and_both_sides_are_counted(super_and_sub):
    top, sub = super_and_sub
    wt = _wt(top)
    git(wt, "checkout", "-q", "-b", "agent/work")
    mine = _commit(wt, "mine.txt")
    _advance_remote(sub, "theirs.txt")
    result = run_phase(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == mine
    assert git(wt, "branch", "--show-current") == "agent/work"
    assert "diverged" in result.stderr and "LEFT ALONE" in result.stderr


# --- review mode: a historical or deliberately non-main superproject ----------
#
# `git checkout <sha>`, `git bisect` and a rebase leave the superproject on a
# detached HEAD; a deliberate branch is anything but `main`. Running a general
# setup command there must not turn historical Ambition code into a mixture with
# today's submodule tips, so present submodules are not moved. Dirty, ahead and
# diverged work is protected in BOTH modes (nothing moves in review mode at all).


def _run(top: Path, *args: str):
    env = dict(os.environ, GIT_ALLOW_PROTOCOL="file")
    return subprocess.run(
        ["bash", "scripts/setup/submodules.sh", *args],
        cwd=top, capture_output=True, text=True, env=env,
    )


def _pin(top: Path) -> str:
    return git(top, "ls-files", "--stage", "vendor/sub").split()[1]


def _detached_at_the_commit_that_records_the_old_pin(top: Path) -> str:
    """The superproject is moved to a historical commit; returns that commit."""
    here = git(top, "rev-parse", "HEAD")
    git(top, "checkout", "-q", "--detach", here)
    return here


def test_review_mode_does_not_move_a_clean_present_submodule_to_todays_main(super_and_sub):
    """⛔ THE POINT OF REVIEW MODE. origin/main has moved on; this detached checkout
    records the older pin, and the submodule is clean and present: it stays."""
    top, sub = super_and_sub
    wt = _wt(top)
    recorded = _pin(top)
    _advance_remote(sub)  # today's main is now ahead of the pin
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == recorded, "review mode moved a present submodule to origin/main"
    assert "REVIEW mode" in result.stderr
    assert "not main" in result.stderr
    # and it is not told that the historical pin is "stale"
    assert "the PIN needs updating" not in result.stderr


def test_a_non_main_superproject_branch_is_review_mode_too(super_and_sub):
    top, sub = super_and_sub
    wt = _wt(top)
    recorded = _pin(top)
    _advance_remote(sub)
    git(top, "checkout", "-q", "-b", "review/some-branch")

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == recorded
    assert "REVIEW mode" in result.stderr and "review/some-branch" in result.stderr


def test_review_mode_reports_a_mixture_instead_of_hiding_it(super_and_sub):
    """A submodule left at a different commit than this checkout records is named,
    with the command that restores the recorded state, and still not moved."""
    top, sub = super_and_sub
    wt = _wt(top)
    old_pin = _pin(top)
    new = _advance_remote(sub)
    git(wt, "fetch", "-q", "origin")
    git(wt, "checkout", "-q", "--detach", new)  # the checkout is ahead of the pin, clean
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == new, "review mode moved the checkout"
    assert "records " + old_pin[:9] in result.stderr
    assert "AHEAD of the recorded pin" in result.stderr
    assert "git submodule update -- vendor/sub" in result.stdout


def test_review_mode_initializes_a_missing_submodule_at_the_recorded_commit_not_main(super_and_sub):
    top, sub = super_and_sub
    recorded = _pin(top)
    _advance_remote(sub)
    shutil.rmtree(top / "vendor/sub")
    (top / "vendor/sub").mkdir()
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(top / "vendor/sub", "rev-parse", "HEAD") == recorded, "a historical checkout was given today's submodule"


def test_review_mode_preserves_dirty_and_ahead_work_and_says_so(super_and_sub):
    top, sub = super_and_sub
    wt = _wt(top)
    (wt / "a.txt").write_text("uncommitted edit that exists nowhere else")
    head = git(wt, "rev-parse", "HEAD")
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == head
    assert (wt / "a.txt").read_text() == "uncommitted edit that exists nowhere else"
    assert "uncommitted changes" in result.stderr


def test_review_mode_refuses_to_bump_pins(super_and_sub):
    """Restaging gitlinks from a historical checkout would rewrite history's pins."""
    top, sub = super_and_sub
    _advance_remote(sub)
    _detached_at_the_commit_that_records_the_old_pin(top)
    result = _run(top, "--bump-pins")
    assert result.returncode != 0
    assert "review mode" in result.stderr
    assert git(top, "diff", "--cached", "--raw", "vendor/sub") == "", "a pin was staged"


def test_follow_main_selects_development_behaviour_on_a_non_main_checkout(super_and_sub):
    """The explicit override: a deliberate branch or worktree that wants it."""
    top, sub = super_and_sub
    wt = _wt(top)
    new = _advance_remote(sub)
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top, "--follow-main")
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == new
    assert "REVIEW mode" not in result.stderr
    assert "the PIN needs updating" in result.stderr


def test_review_mode_preserves_local_commits_on_a_branch_and_names_them(super_and_sub):
    top, _ = super_and_sub
    wt = _wt(top)
    git(wt, "checkout", "-q", "-b", "agent/work")
    ahead = _commit(wt, "local.txt")
    _detached_at_the_commit_that_records_the_old_pin(top)

    result = _run(top)
    assert result.returncode == 0, result.stderr
    assert git(wt, "rev-parse", "HEAD") == ahead
    assert git(wt, "branch", "--show-current") == "agent/work", "the branch was detached or switched"
    assert "AHEAD of the recorded pin" in result.stderr
