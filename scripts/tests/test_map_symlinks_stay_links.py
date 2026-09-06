"""Tracked LDtk worlds must remain symlinks into `game/ambition_map_assets`.

Writers may update the target through the link but must not replace the symlink
with a copied world file. Dangling links are allowed when the submodule is not
initialized; link resolution is checked separately."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]

#: Where the worlds really live. The symlink text must point inside this.
SUBMODULE = "game/ambition_map_assets"

#: Git's mode for a symlink blob. A regular file is 100644.
SYMLINK_MODE = "120000"


def _tracked_ldtk() -> list[tuple[str, str]]:
    """`(mode, path)` for every tracked `.ldtk`, straight from the index."""
    out = subprocess.run(
        ["git", "-C", str(REPO), "ls-files", "--stage", "*.ldtk"],
        capture_output=True, text=True, timeout=60, check=True,
    ).stdout
    rows = []
    for line in out.splitlines():
        if not line.strip():
            continue
        meta, path = line.split("\t", 1)
        rows.append((meta.split()[0], path))
    return rows


def test_there_are_tracked_worlds_at_all():
    """⛔ kills the vacuous pass. Every assertion below iterates this list, so an
    empty one would make the whole file green while proving nothing — the exact
    shape of a check that cannot fail."""
    assert _tracked_ldtk(), "no tracked .ldtk files; this whole file is vacuous"


def test_every_tracked_world_is_a_symlink():
    """The assertion that matters, and it needs no submodule checkout."""
    regular = [path for mode, path in _tracked_ldtk() if mode != SYMLINK_MODE]
    assert not regular, (
        f"these worlds are tracked as REAL FILES, not symlinks: {regular}. "
        f"Something replaced the link instead of writing through it — see this "
        f"file's docstring. Restore with:\n"
        f"  ln -sf <relative path into {SUBMODULE}> <path>"
    )


def test_the_working_tree_agrees_with_the_index():
    """Tracked map symlinks must also remain symlinks in the working tree.

    The index still reports mode 120000 after a generator replaces a link with a
    regular file, so working-tree type changes require a separate check.
    """
    broken = [
        path
        for mode, path in _tracked_ldtk()
        if mode == SYMLINK_MODE and not (REPO / path).is_symlink()
    ]
    assert not broken, (
        f"these worlds are TRACKED as symlinks but are REAL FILES on disk: "
        f"{broken}\n"
        f"A writer replaced the link instead of writing through it, so the "
        f"world's bytes now live in the main repo again and diverge from "
        f"{SUBMODULE}. Write the regenerated world to its real path inside the "
        f"submodule, then restore the link:\n"
        f"  ln -sf <relative path into {SUBMODULE}> <path>"
    )


def test_every_link_points_into_the_map_submodule():
    """A symlink pointing somewhere else is as wrong as a real file — it would
    make the world's real home ambiguous, and `git mv` on either end would break
    it silently.

    ⚠ **Only examines entries that ARE links.** The test above owns the "it
    stopped being a link" failure; without this filter a single replaced file
    fails both tests, and this one fails with a raw `OSError` from `readlink`
    rather than a sentence anyone can act on. One defect, one readable failure.
    """
    for mode, path in _tracked_ldtk():
        # BOTH conditions, and the second is not redundant: a TYPECHANGE
        # leaves the index saying 120000 while the worktree holds a regular
        # file, so an index-only filter reaches `readlink` on a real file and
        # raises `OSError: Invalid argument` instead of failing readably.
        # `test_the_working_tree_agrees_with_the_index` owns that failure.
        if mode != SYMLINK_MODE or not (REPO / path).is_symlink():
            continue
        target = (REPO / path).readlink()
        resolved = ((REPO / path).parent / target).resolve()
        assert resolved.is_relative_to((REPO / SUBMODULE).resolve()), (
            f"{path} links to {target}, which is outside {SUBMODULE}"
        )


def test_the_links_resolve_when_the_submodule_is_checked_out():
    """⚠ SKIPPED rather than failed when the submodule is absent.

    A dangling link is the DESIGNED signal for "you did not run
    `git submodule update --init`", not a defect in the tree. Failing here would
    turn a deliberate, legible state into a red test on every non-recursive
    clone.
    """
    if not (REPO / SUBMODULE / ".git").exists():
        pytest.skip(f"{SUBMODULE} is not checked out; dangling links are expected")
    missing = [path for _, path in _tracked_ldtk() if not (REPO / path).is_file()]
    assert not missing, (
        f"{SUBMODULE} is checked out but these links do not resolve: {missing}. "
        f"The submodule's layout probably drifted from the link targets."
    )
