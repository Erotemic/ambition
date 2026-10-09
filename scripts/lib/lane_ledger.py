"""Which check ran on which tree, and what it said (TEST-LANES).

`run_tests.py` appends one row for each job it finishes. `required_checks.py`
reads the rows to decide whether a push carries the checks its change needs.

A row names the TREE the job tested, not a commit. Tests run on a working tree
that is usually not committed yet, and a commit names one tree only after it
exists. So the tree is written from the working tree: tracked files as they
are on disk, plus untracked source files that are not ignored.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path

LEDGER_NAME = "lane_ledger.jsonl"

#: Untracked files with these suffixes are part of the tested tree. A new
#: source file is compiled before it is committed; without it, the tree of the
#: run differs from the commit at exactly the file the change added.
SOURCE_SUFFIXES = (".rs", ".toml", ".py", ".ron", ".ldtk", ".yaml", ".json")

#: Untracked directories that hold no source of this repository.
NOT_SOURCE_PARTS = ("/.venv/", ".egg-info/", "/__pycache__/", "/node_modules/")


def ledger_path(repo: Path) -> Path:
    return repo / "target" / LEDGER_NAME


def _git(repo: Path, *args: str, env: dict | None = None, data: bytes | None = None) -> bytes:
    return subprocess.run(
        ["git", *args], cwd=repo, env=env, input=data, capture_output=True, check=True
    ).stdout


def gitlinks(repo: Path) -> list[str]:
    """The submodule paths of HEAD. A worktree can hold one as a symlink, and a
    symlink is not the content the commit records."""
    out = _git(repo, "ls-files", "-s", "-z")
    return [
        entry.split(b"\t", 1)[1].decode()
        for entry in out.split(b"\0")
        if entry.startswith(b"160000 ")
    ]


def tested_tree(repo: Path) -> str | None:
    """The git tree of the working tree as a job sees it, or None outside git.

    The tree is written through a copy of the index, so the real index and the
    staged changes do not move. The copy keeps the stat cache, so only changed
    files are hashed again.
    """
    try:
        index = Path(_git(repo, "rev-parse", "--git-path", "index").decode().strip())
        if not index.is_absolute():
            index = repo / index
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch) / "index"
            if index.exists():
                shutil.copy2(index, copy)
            env = dict(os.environ, GIT_INDEX_FILE=str(copy))
            if not index.exists():
                _git(repo, "read-tree", "HEAD", env=env)
            # Each submodule keeps the commit HEAD records.
            keep = [f":(exclude){path}" for path in gitlinks(repo)]
            _git(repo, "add", "-u", "--", ".", *keep, env=env)
            untracked = [
                name
                for name in _git(repo, "ls-files", "--others", "--exclude-standard", "-z")
                .decode("utf-8", "replace")
                .split("\0")
                if name.endswith(SOURCE_SUFFIXES)
                and not any(part in f"/{name}" for part in NOT_SOURCE_PARTS)
            ]
            if untracked:
                _git(repo, "add", "--pathspec-from-file=-", "--pathspec-file-nul", env=env,
                     data="\0".join(untracked).encode())
            return _git(repo, "write-tree", env=env).decode().strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def record(repo: Path, tree: str, job: str, argv: list[str], ok: bool,
           unrunnable: str | None) -> None:
    """Append one finished job. A row is evidence only for the tree it names."""
    path = ledger_path(repo)
    path.parent.mkdir(parents=True, exist_ok=True)
    row = {
        "tree": tree,
        "job": job,
        # The program by its name, not its path: the cargo or python binary
        # differs between machines, and the coverage reads only the arguments.
        "argv": [Path(argv[0]).name, *argv[1:]] if argv else [],
        "ok": ok,
        "unrunnable": unrunnable,
        "finished": time.time(),
    }
    with path.open("a") as handle:
        handle.write(json.dumps(row) + "\n")


def rows(repo: Path) -> list[dict]:
    """Every recorded row, oldest first. A line that does not parse is skipped:
    it is not evidence of a pass."""
    path = ledger_path(repo)
    if not path.exists():
        return []
    out = []
    for line in path.read_text().splitlines():
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out
