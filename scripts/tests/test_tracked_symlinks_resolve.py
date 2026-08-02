"""Every symlink git tracks points at something that exists.

⛔ `game/ambition_content/assets/sprites` dangled through a completely clean
`cargo check --workspace --all-targets`, and would have through the test suite
too. A `git mv` during the crate rename moved the symlink's TARGET and left the
link text pointing at the old path; nothing in the build reads that directory, so
nothing failed. The first symptom available was a game with no sprites, at
runtime, in whichever composition happened to load them first.

That is the worst shape a defect can have here: the repository is internally
inconsistent, every automated check is green, and the only detector is a person
launching the right game.

⚠ **the check must follow the link, not merely find it.** `Path.exists()` on a
symlink already resolves the target (it is `stat`, not `lstat`), which is exactly
the behaviour wanted — but `Path.is_symlink()` does NOT, so an
`is_symlink() and ...` guard that reads naturally can end up asserting nothing.
The mode bits from `git ls-files -s` are the authority for "this is a symlink";
the filesystem answers whether it resolves.

Cheap enough to sit in the fast loop: one `git ls-files`, one `stat` per link.
"""

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
