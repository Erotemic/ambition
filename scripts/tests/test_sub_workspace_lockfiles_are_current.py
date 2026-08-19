"""Every sub-workspace's `Cargo.lock` still resolves.

Independent workspaces may carry their own TRACKED lockfiles. Their independent
resolution is what makes them proof that a third party can build against the
umbrella crate. An ignored machine-local lockfile is not repository state and
must not change the population this guard checks.

The cost is that changing a dependency anywhere in the main workspace can
invalidate those lockfiles, and nothing in the fast loop says so. `cargo test` in each
does eventually — but those are non-fast jobs at the end of the suite, so the
feedback arrives minutes later attached to a job that looks unrelated.

⚠ **this has now bitten three times**, most recently on 2026-08-01 when removing
`ambition_platformer2d_core` from two crates left all three stale. The footprint
ratchet caught `minimal_game` only, because that is the one workspace it happens
to run in; the other two were found by hand afterwards. One of them was already
committed by then.

`cargo tree --locked` is the whole check: it refuses to update the lockfile and
exits non-zero if resolution would need to. No network, no build.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

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
        ["cargo", "tree", "--locked", "--prefix", "none", "--edges", "normal"],
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
