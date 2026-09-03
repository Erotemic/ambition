"""Publish a generated file into a tree that may be MIRRORED BY SYMLINK.

`scripts/mirror_assets_for_worktree.py` gives a worktree its generated assets by
symlinking them, file by file, at the main checkout's copies. Its whole design
rests on one sentence:

    mirror the files individually and a regenerated sprite lands as a REAL file
    in the worktree, replacing that one symlink, while every other asset still
    points at the shared copy.

⛔⛔ **THAT IS ONLY TRUE IF THE WRITER UNLINKS FIRST.** `open(dst, "wb")`,
`Path.write_text()`, `Path.write_bytes()`, `shutil.copy2()` and `Image.save()`
all OPEN THE DESTINATION FOR WRITING, and an open-for-write FOLLOWS a symlink —
so the bytes land in the MAIN CHECKOUT that every other session builds and gates
from, and the link stays in place so nothing looks wrong. Measured 2026-09-02
rather than argued: `open(link, "wb").write(...)` changed the target's bytes.

⇒ **The invariant every publisher into a mirrored tree owes: break a symlinked
destination before writing it.** That is one line, it is easy to forget, and
forgetting it is silent — which is why it lives here once instead of at each
call site.

    from publish_safely import break_mirrored_destination

    break_mirrored_destination(out_path)
    with out_path.open("wb") as fh:          # now writes a REAL file
        ...

⚠ **Not `os.O_NOFOLLOW`, and not a check-then-write race guard.** This is not a
security boundary — the writer and the link are both ours. It exists to stop an
ACCIDENT (a regen in a worktree editing the shared checkout), so the honest
shape is "unlink the link, then write normally".

⚠ **Directories are never unlinked**, only file symlinks: a symlinked directory
in a mirrored tree is a bug in the mirror, not something a publisher should
silently rewrite. It raises instead.
"""

from __future__ import annotations

import os
import shutil
from pathlib import Path

__all__ = [
    "break_mirrored_destination",
    "publish_file_without_following_symlink",
    "open_for_publishing",
]


def break_mirrored_destination(dst: str | os.PathLike[str]) -> bool:
    """Unlink ``dst`` if it is a symlink, so the next write creates a real file.

    Returns True if a link was broken, False if there was nothing to do (the
    common case: a real file, or nothing there at all).

    ⛔ Uses ``Path.is_symlink()``, which does NOT follow the link — ``exists()``
    would report False for a symlink whose target is missing and let the write
    through as "nothing here", which is the same bug in a different costume.
    """
    path = Path(dst)
    if not path.is_symlink():
        return False
    if path.is_dir():
        raise IsADirectoryError(
            f"{path} is a symlinked DIRECTORY inside a mirrored tree. That is a "
            "mirror bug (it links files individually, precisely so a write "
            "cannot reach the shared copy) and a publisher must not resolve it "
            "by deleting the link."
        )
    path.unlink()
    return True


def publish_file_without_following_symlink(
    src: str | os.PathLike[str], dst: str | os.PathLike[str]
) -> Path:
    """`shutil.copy2(src, dst)` that lands in the worktree, not through a mirror.

    ``copy2`` opens the destination for writing, so on a mirrored file it edits
    the main checkout. This breaks the link first and preserves copy2's metadata
    behaviour otherwise.
    """
    destination = Path(dst)
    destination.parent.mkdir(parents=True, exist_ok=True)
    break_mirrored_destination(destination)
    shutil.copy2(src, destination)
    return destination


def open_for_publishing(dst: str | os.PathLike[str], mode: str = "wb"):
    """`open(dst, mode)` for a publisher, with the mirrored link broken first.

    Only for WRITING modes — opening a mirrored file for READING should follow
    the link, which is the entire point of the mirror.
    """
    if not any(flag in mode for flag in ("w", "a", "x", "+")):
        raise ValueError(
            f"open_for_publishing is for writing; {mode!r} reads, and a read "
            "SHOULD follow the mirror's symlink"
        )
    destination = Path(dst)
    destination.parent.mkdir(parents=True, exist_ok=True)
    break_mirrored_destination(destination)
    return destination.open(mode)
