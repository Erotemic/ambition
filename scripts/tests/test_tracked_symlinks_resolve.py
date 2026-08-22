"""Every symlink tracked by Git must resolve to an existing target.

Git mode bits determine which paths are symlinks; filesystem resolution then
checks their targets. This catches repository paths that remain tracked and
syntactically valid but point at moved or missing content."""

from __future__ import annotations

import subprocess
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# git's mode for a symlink blob.
SYMLINK_MODE = "120000"


def _tracked_symlinks() -> list[Path]:
    listing = subprocess.run(
        ["git", "ls-files", "-s", "-z"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split("\0")
    out: list[Path] = []
    for entry in listing:
        if not entry.startswith(SYMLINK_MODE):
            continue
        # `<mode> <sha> <stage>\t<path>`
        _, _, path = entry.partition("\t")
        if path:
            out.append(REPO / path)
    return sorted(out)


def test_every_tracked_symlink_points_at_something_that_exists():
    links = _tracked_symlinks()
    assert links, (
        "git reports no tracked symlinks at all. There was one "
        "(`game/ambition_content/assets/sprites`); if it became a real "
        "directory that is fine, but this guard is now watching nothing and "
        "should be removed rather than left looking green."
    )

    dangling = [
        f"{link.relative_to(REPO)} -> {link.readlink()}"
        for link in links
        if not link.exists()
    ]
    assert not dangling, (
        "these tracked symlinks point at nothing, and no build step reads them "
        "so every other check stays green:\n  "
        + "\n  ".join(dangling)
        + "\n\nRepoint the link (`ln -sfn <target> <link>`) and commit it. A "
        "moved target is the usual cause — a `git mv` rewrites paths but not "
        "the text inside a symlink."
    )
