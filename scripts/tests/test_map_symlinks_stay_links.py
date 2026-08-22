"""Every tracked `.ldtk` must stay a SYMLINK into `game/ambition_map_assets`.

The six LDtk worlds (~5 MB, the densest content this repo tracks) live in the
`ambition_map_assets` submodule and reach the game through tracked symlinks.
Git stores those natively (mode 120000), so a fresh clone gets them for free.

⛔ **The failure this defends is silent and expensive**: a tool that writes a
world by creating a temp file and renaming it over the destination *destroys the
symlink* and leaves a real multi-megabyte file, which then gets committed
straight back into the main repo. Nothing errors. The next person notices when
the checkout is 5 MB bigger for no reason anyone can attribute.

Every writer in `tools/ambition_ldtk_tools` was checked (2026-08-08) and all go
through `path.write_text(dump_editor_style(project))`, which writes *through* a
link and preserves it. The LDtk **editor** is outside that guarantee, which is
why this is a test and not a comment.

⚠ **A DANGLING link is not a failure here.** Jon, 2026-08-08: *"A symlink lets
us know when the submodule isn't checked out."* That is the intended signal, so
resolution is checked separately and skipped when the submodule is absent — the
mode assertion below is what must hold unconditionally, and it holds whether or
not anyone ran `git submodule update`.
"""

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
    """⛔ **The index alone cannot see this**, which is why it needs its own test.

    A generator that writes a real file over a tracked symlink produces a
    TYPECHANGE: `git status` says `T`, the index still reports mode 120000, and
    every assertion above keeps passing because they all read the index. The
    breakage is entirely in the working tree.

    Caught live on 2026-08-08 when regenerating `sanic_speedway.ldtk` replaced
    its link. ⚠ before this test, the only symptom was
    `test_every_link_points_into_the_map_submodule` dying with a raw
    `OSError: [Errno 22] Invalid argument` out of `readlink` — a real failure
    wearing a stack trace nobody can act on. Now that test skips the entry and
    this one names it.
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
