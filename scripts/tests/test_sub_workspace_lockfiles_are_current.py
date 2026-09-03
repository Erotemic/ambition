"""Tracked lockfiles in independent sub-workspaces must resolve without updates.

The check runs `cargo tree --locked` in each tracked sub-workspace. Machine-local
ignored lockfiles are not repository state and do not affect the checked set."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "lib"))
from cargo_bin import cargo_binary  # noqa: E402

REPO = Path(__file__).resolve().parents[2]

def _tracked_sub_workspaces() -> list[Path]:
    """Independent workspaces whose lockfiles are repository state.

    Filesystem recursion is the wrong discovery authority here: this repository
    commonly stores complete sibling checkouts under `.worktrees/`, and `rglob`
    therefore made the test population depend on which worktrees happened to be
    mounted on one developer's machine. Ignored/generated lockfiles have the same
    problem. Git's tracked-file view states exactly which lockfiles this checkout
    promises to keep current while still discovering a newly committed fourth
    sub-workspace automatically.
    """
    result = subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "-z", "--", "**/Cargo.lock"],
        capture_output=True,
        check=True,
    )
    lockfiles = [
        REPO / raw.decode("utf8", errors="surrogateescape")
        for raw in result.stdout.split(b"\0")
        if raw
    ]
    return sorted({path.parent for path in lockfiles if path.parent != REPO})


# Discovered rather than listed: a newly committed sub-workspace cannot appear
# uncovered, while ignored worktrees and local generated locks are invisible.
SUB_WORKSPACES = _tracked_sub_workspaces()


def test_the_repo_still_has_sub_workspaces_to_check():
    assert SUB_WORKSPACES, (
        "no sub-workspace lockfiles found, so this guard is watching nothing. "
        "They are the only proof that an outside consumer can resolve against "
        "the umbrella crate; if they moved, point this at wherever they went."
    )


@pytest.mark.parametrize(
    "workspace", SUB_WORKSPACES, ids=lambda path: str(path.relative_to(REPO))
)
def test_the_lockfile_resolves_without_updating(workspace: Path):
    result = subprocess.run(
        [cargo_binary(), "tree", "--locked", "--prefix", "none", "--edges", "normal"],
        cwd=workspace,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        f"{workspace.relative_to(REPO)}/Cargo.lock is STALE — a dependency "
        "change in the main workspace invalidated it:\n"
        f"{result.stderr}\n"
        f"Fix: `cd {workspace.relative_to(REPO)} && cargo update --workspace "
        "--offline`, then commit the lockfile with the change that caused it."
    )
