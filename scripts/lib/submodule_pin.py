"""Is a tool submodule's checkout behind the commit the superproject pins?

`git pull` and `git merge` move a submodule's pointer, not its files. A tool
checkout left behind its pin still runs, on the old sources: a music
regeneration finds "no missing products", the asset inspector lists no new
score, and a module the superproject now calls (2026-11-23:
`ambition_music_renderer.score_inventory`) is simply absent. This names that
state so the tools that read the tree can say it instead of reporting a quiet
success, and `ensure` repairs it when that is safe.

A checkout AHEAD of its pin, or diverged from it, is a developer editing the
submodule: allowed, and not reported by `behind_pin`.

    python3 scripts/lib/submodule_pin.py ensure <path> [<path> ...]

checks out the pinned commit of every listed submodule that is not checked out
or is behind its pin and has no local changes to tracked files. One that has
such changes is left alone, with the command to run once they are committed or
stashed, and the exit status is 2 (0 with `AMBITION_ALLOW_STALE_TOOL=1`).
"""

from __future__ import annotations

import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


def _git(cwd: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)


@dataclass
class PinState:
    path: str
    pinned: str
    checkout: str | None  # None: not checked out
    kind: str  # "at-pin" | "missing" | "behind" | "unfetched" | "own-commits"
    behind: int = 0
    missing_subjects: tuple[str, ...] = ()
    local_changes: bool = False

    @property
    def fix(self) -> str:
        return f"git submodule update --init {self.path}"

    def sentence(self) -> str:
        """What is stale, in one sentence, ending in the command that fixes it."""
        if self.kind == "missing":
            return f"{self.path} is not checked out; the superproject pins {self.pinned[:9]}: run `{self.fix}`"
        if self.kind == "unfetched":
            head = (f"{self.path} is at {self.checkout[:9]}, and the commit the superproject pins "
                    f"({self.pinned[:9]}) has not been fetched into it")
        else:
            head = (f"{self.path} is checked out at {self.checkout[:9]}, {self.behind} commit(s) behind the "
                    f"{self.pinned[:9]} the superproject pins, so it runs old sources")
            if self.missing_subjects:
                shown = "; ".join(self.missing_subjects[:3])
                more = f"; and {self.behind - 3} more" if self.behind > 3 else ""
                head += f" (missing: {shown}{more})"
        if self.local_changes:
            return (f"{head}. It has uncommitted changes to tracked files: commit or stash them "
                    f"in {self.path}, then run `{self.fix}`")
        return f"{head}: run `{self.fix}` (or pull with `--recurse-submodules`)"


def pin_state(root: Path, path: str) -> PinState | None:
    """How `path` (a submodule of `root`) stands against its pin; None when it is not a submodule."""
    listed = _git(root, "ls-tree", "HEAD", path)
    fields = listed.stdout.split()
    if listed.returncode != 0 or len(fields) < 3 or fields[0] != "160000":
        return None  # not a submodule here (a vendored copy, or no git)
    pinned = fields[2]
    sub = root / path
    # ⚠ An uninitialized submodule is an empty directory INSIDE the superproject,
    # so `git rev-parse HEAD` there answers with the SUPERPROJECT's commit.
    # Ask for the toplevel first and require it to be the submodule itself.
    top = _git(sub, "rev-parse", "--show-toplevel") if sub.is_dir() else None
    if top is None or top.returncode != 0 or Path(top.stdout.strip()).resolve() != sub.resolve():
        return PinState(path, pinned, None, "missing")
    checkout = _git(sub, "rev-parse", "HEAD").stdout.strip()
    changed = bool(_git(sub, "status", "--porcelain", "--untracked-files=no").stdout.strip())
    if checkout == pinned:
        return PinState(path, pinned, checkout, "at-pin", local_changes=changed)
    if _git(sub, "cat-file", "-e", f"{pinned}^{{commit}}").returncode != 0:
        return PinState(path, pinned, checkout, "unfetched", local_changes=changed)
    if _git(sub, "merge-base", "--is-ancestor", checkout, pinned).returncode == 0:
        count = int(_git(sub, "rev-list", "--count", f"{checkout}..{pinned}").stdout.strip() or 0)
        subjects = _git(sub, "log", "--format=%h %s", "-n", "3", f"{checkout}..{pinned}").stdout.splitlines()
        return PinState(path, pinned, checkout, "behind", count, tuple(subjects), changed)
    return PinState(path, pinned, checkout, "own-commits", local_changes=changed)


def behind_pin(root: Path, path: str) -> str | None:
    """A sentence naming how `path` (a submodule of `root`) is behind its pin, or None."""
    state = pin_state(root, path)
    if state is None or state.kind in {"at-pin", "own-commits"}:
        return None
    return state.sentence()


def ensure(root: Path, paths: list[str], label: str) -> int:
    """Check out the pin of each listed submodule that is missing or behind and safe to move."""
    allow_stale = bool(os.environ.get("AMBITION_ALLOW_STALE_TOOL"))
    status = 0
    for path in paths:
        state = pin_state(root, path)
        if state is None or state.kind == "at-pin":
            continue
        if state.kind == "own-commits":
            print(f"==> {path}: at {state.checkout[:9]}, not the pinned {state.pinned[:9]} "
                  "(it has commits the pin does not); using it as it is", file=sys.stderr)
            continue
        if state.local_changes:
            if allow_stale:
                continue  # the producer names the stale tree it runs on
            print(f"{label}: refused: {state.sentence()}", file=sys.stderr)
            status = 2
            continue
        before = ("not checked out" if state.kind == "missing" else
                  f"at {state.checkout[:9]}, " + (f"{state.behind} commit(s) behind its pin" if state.kind == "behind"
                                                  else "without its pinned commit fetched"))
        print(f"==> {path}: {before}; checking out the pinned {state.pinned[:9]}", file=sys.stderr)
        done = subprocess.run(["git", "submodule", "update", "--init", "--", path], cwd=root,
                              stdout=sys.stderr, stderr=sys.stderr)
        after = pin_state(root, path)
        if done.returncode != 0 or after is None or after.kind != "at-pin":
            print(f"{label}: could not check out {path} at its pin {state.pinned[:9]}; "
                  f"run `{state.fix}` yourself and read its error", file=sys.stderr)
            status = 2
    return status


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[0] != "ensure":
        print("usage: submodule_pin.py ensure [--label NAME] <submodule path> [...]", file=sys.stderr)
        return 2
    rest = argv[1:]
    label = "submodules"
    if rest[:1] == ["--label"] and len(rest) >= 2:
        label, rest = rest[1], rest[2:]
    return ensure(Path(__file__).resolve().parents[2], rest, label)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
