"""Is a tool submodule's checkout behind the commit the superproject pins?

`git pull` moves a submodule's pointer, not its files. A tool checkout left
behind its pin still runs, on the old sources: a music regeneration finds
"no missing products" and the asset inspector lists no new score, because
both discover their inputs in the tool's own tree. This names that state so
the tools that read the tree can say it instead of reporting a quiet success.

A checkout AHEAD of its pin, or diverged from it, is a developer editing the
submodule: allowed, and not reported here.
"""

from __future__ import annotations

import subprocess
from pathlib import Path


def _git(cwd: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)


def behind_pin(root: Path, path: str) -> str | None:
    """A sentence naming how `path` (a submodule of `root`) is behind its pin, or None."""
    listed = _git(root, "ls-tree", "HEAD", path)
    fields = listed.stdout.split()
    if listed.returncode != 0 or len(fields) < 3 or fields[0] != "160000":
        return None  # not a submodule here (a vendored copy, or no git)
    pinned = fields[2]
    head = _git(root / path, "rev-parse", "HEAD")
    fix = f"run `git submodule update --init {path}` (or pull with `--recurse-submodules`)"
    if head.returncode != 0:
        return f"{path} is not checked out; the superproject pins {pinned[:9]}: {fix}"
    checkout = head.stdout.strip()
    if checkout == pinned:
        return None
    if _git(root / path, "cat-file", "-e", f"{pinned}^{{commit}}").returncode != 0:
        return (f"{path} is at {checkout[:9]}, and the commit the superproject pins ({pinned[:9]}) "
                f"has not been fetched: {fix}")
    if _git(root / path, "merge-base", "--is-ancestor", checkout, pinned).returncode == 0:
        count = _git(root / path, "rev-list", "--count", f"{checkout}..{pinned}").stdout.strip()
        return (f"{path} is at {checkout[:9]}, {count} commit(s) behind the {pinned[:9]} the superproject "
                f"pins, so its newest sources are not here: {fix}")
    return None
