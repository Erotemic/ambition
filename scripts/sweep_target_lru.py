#!/usr/bin/env python3
"""Reclaim target/ space by age of LAST USE, without running cargo.

Every cargo unit leaves files keyed by one 16-hex hash — ``deps/lib<crate>-<hash>.rlib``
(and ``.rmeta``/``.d``/``.so``/test executables), ``.fingerprint/<pkg>-<hash>/``,
``build/<pkg>-<hash>/``, ``examples/<name>-<hash>`` — and a stale unit is one no build
has touched in a while. This groups a profile's files by that hash and removes a
group only when EVERY file in it is older than ``--days`` by the later of its
access and modification time.

⭐ WHY THIS NEEDS NO LIVE-SET MARKING, unlike ``sweep_target.py``. Marking asks
cargo which artifacts a command resolves, which means running the command — and
a "dry run" that runs ``cargo test --no-run --workspace`` under two incremental
modes COMPILES both graphs, so on a nearly full disk the dry run is what fills
it. Here nothing compiles: the price of deleting a unit that was still live is
that cargo rebuilds it. Measured 2026-09-23: with a warm graph (0 units to
compile), moving glam's rlibs aside made the next build recompile exactly glam
and its dependents and succeed — cargo treats a missing output as dirty.

⚠ WHAT "USE" MEANS, AND ITS LIMIT. The root filesystem mounts ``relatime``: a
file's atime moves when it is read and its atime is not newer than its mtime,
or is more than a day old. Compiling or linking a dependent reads an rlib;
running a test binary reads it; a no-op build reads the unit's fingerprint
files. Under ``noatime`` this degrades to build age, which is still correct,
only less precise.

Usage::

    scripts/sweep_target_lru.py                      # report only
    scripts/sweep_target_lru.py --apply              # remove units unused 7+ days
    scripts/sweep_target_lru.py --days 3 --apply
    scripts/sweep_target_lru.py --apply --ensure-free 40
        # ...and if the disk still has under 40 GiB free, empty every profile

⛔ SAFETY, each a rule this repository learned before this script existed
(`clean_workspace_crates.sh` carries the history):

* ``--apply`` REFUSES when ``target/`` is on a virtiofs worktree and not bound
  onto local disk (``scripts/setup/target_bindmount.sh --check``). Unbound,
  ``target/`` exposes the shadowed duplicate underneath the mount point, and
  reclaiming that is a maintainer's decision, not a sweep's.
* The profile's ``.cargo-lock`` is OPENED FOR APPEND — created when absent —
  and exclusively locked before the first delete, and held through the last.
  Probing it, or skipping a profile that has no lock file yet, lets a cargo
  that starts in between build underneath the deletion.
* ``incremental/`` sessions are rustc's, named ``<crate>-<suffix>`` with a
  suffix that is NOT a cargo unit hash, so each is its own reclaim unit, aged
  like any other.
"""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# Cargo's own per-unit directories under a profile, grouped by unit hash.
# Anything else there — top-level hardlinked binaries, captures, logs — is not
# grouped and is kept.
UNIT_DIRS = ("deps", ".fingerprint", "build", "examples")
# rustc's incremental cache: one entry per crate session directory, keyed by
# its own path because its suffix is not a cargo unit hash.
INCREMENTAL_DIR = "incremental"

# `<name>-<16 hex>` optionally followed by an extension chain.
HASHED = re.compile(r"^(?P<stem>.+)-(?P<hash>[0-9a-f]{16})(?P<ext>\..*)?$")


@dataclass
class Unit:
    hash: str
    paths: list[Path] = field(default_factory=list)
    last_used: float = 0.0
    size: int = 0


def _stat_tree(path: Path) -> tuple[float, int]:
    """Latest use and total size over a file or directory tree.

    ⛔ A DIRECTORY'S ATIME IS NOT A USE. Listing a directory reads it, so `ls`,
    `du` and this very walk move it — the first version of this script counted
    it, and one dry run made every unit look used within the minute. A
    directory contributes its mtime only (an entry was written); a file
    contributes max(atime, mtime). Cargo's freshness check READS a unit's
    fingerprint files, so even a no-op build marks the unit used.
    """
    latest = 0.0
    size = 0
    stack = [path]
    while stack:
        current = stack.pop()
        try:
            st = current.lstat()
        except OSError:
            continue
        if current.is_dir() and not current.is_symlink():
            latest = max(latest, st.st_mtime)
            with contextlib.suppress(OSError):
                stack.extend(current.iterdir())
        else:
            latest = max(latest, st.st_atime, st.st_mtime)
            size += st.st_size
    return latest, size


def profile_units(profile: Path) -> dict[str, Unit]:
    """Every hash-keyed unit in one profile directory."""
    units: dict[str, Unit] = {}
    for sub in UNIT_DIRS:
        directory = profile / sub
        if not directory.is_dir():
            continue
        for entry in directory.iterdir():
            match = HASHED.match(entry.name)
            if not match:
                continue
            unit = units.setdefault(match["hash"], Unit(match["hash"]))
            latest, size = _stat_tree(entry)
            unit.paths.append(entry)
            unit.last_used = max(unit.last_used, latest)
            unit.size += size
    incremental = profile / INCREMENTAL_DIR
    if incremental.is_dir():
        for entry in incremental.iterdir():
            key = f"{INCREMENTAL_DIR}/{entry.name}"
            latest, size = _stat_tree(entry)
            units[key] = Unit(key, [entry], latest, size)
    return units


def profiles(target: Path) -> list[Path]:
    """Profile directories: any directory holding `deps/`, one or two levels down
    (`target/debug`, `target/<triple>/release`)."""
    found = []
    for candidate in [*target.glob("*"), *target.glob("*/*")]:
        if candidate.is_dir() and (candidate / "deps").is_dir():
            found.append(candidate)
    return sorted(found)


@contextlib.contextmanager
def build_lock(profile: Path):
    """Hold cargo's build lock for the profile, or yield False if it is busy.

    Opened for APPEND so an absent lock file is created: cargo's own `flock` on
    the same path then waits for us, whichever of the two arrived first.
    """
    lock_path = profile / ".cargo-lock"
    with open(lock_path, "a") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def bind_refusal(target: Path) -> str | None:
    """Why an apply must not touch `target`, or None.

    Only the repository's own `target/` can be a shadowed bind point; a
    `CARGO_TARGET_DIR` elsewhere is the caller's own directory.
    """
    if target.resolve() != (REPO / "target").resolve():
        return None
    check = REPO / "scripts" / "setup" / "target_bindmount.sh"
    if not check.exists():
        return None
    result = subprocess.run(["bash", str(check), "--check"], capture_output=True, text=True)
    if result.returncode == 0:
        return None
    return (
        f"{result.stderr.strip()}\n\nREFUSING: target/ is not bound onto local disk "
        "(target_bindmount.sh --check exited "
        f"{result.returncode}). An unbound target exposes the shadowed duplicate, "
        "which is not a sweep's to reclaim. Run: scripts/setup/target_bindmount.sh"
    )


def remove(path: Path) -> None:
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path, ignore_errors=True)
    else:
        with contextlib.suppress(FileNotFoundError):
            path.unlink()


def empty_profile(profile: Path) -> None:
    """Everything under the profile, keeping the directory itself and its lock
    file — the caller holds that lock while this runs."""
    for entry in profile.iterdir():
        if entry.name == ".cargo-lock":
            continue
        remove(entry)


def free_gib(path: Path) -> float:
    return shutil.disk_usage(path).free / 2**30


def target_dir() -> Path:
    if env := os.environ.get("CARGO_TARGET_DIR"):
        return Path(env)
    return REPO / "target"


def sweep(target: Path, days: float, apply: bool, now: float | None = None) -> tuple[int, int, list[str]]:
    """Remove (or report) stale units. Returns (units, bytes, skipped profiles)."""
    now = time.time() if now is None else now
    cutoff = now - days * 86400
    removed_units = removed_bytes = 0
    skipped: list[str] = []
    for profile in profiles(target):
        # A report deletes nothing, so it takes no lock and writes no file.
        lock = build_lock(profile) if apply else contextlib.nullcontext(True)
        with lock as held:
            if not held:
                skipped.append(str(profile))
                continue
            for unit in profile_units(profile).values():
                if unit.last_used >= cutoff:
                    continue
                removed_units += 1
                removed_bytes += unit.size
                if apply:
                    for path in unit.paths:
                        remove(path)
    return removed_units, removed_bytes, skipped


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--days", type=float, default=7.0, help="unused for this long is stale (default 7)")
    ap.add_argument("--apply", action="store_true", help="actually delete; without it, report only")
    ap.add_argument(
        "--ensure-free",
        type=float,
        metavar="GIB",
        help="after the sweep, empty every profile if the disk has less than this free",
    )
    args = ap.parse_args()

    target = target_dir()
    if not target.is_dir():
        print(f"no target directory at {target}")
        return 0
    mount = subprocess.run(
        ["findmnt", "-no", "OPTIONS", "-T", str(target)], capture_output=True, text=True
    ).stdout.strip()
    if "noatime" in mount.split(","):
        print("⚠ target/ is on a noatime mount: last use degrades to build age")

    if args.apply and (refusal := bind_refusal(target)):
        print(refusal, file=sys.stderr)
        return 2

    before = free_gib(target)
    units, size, skipped = sweep(target, args.days, args.apply)
    verb = "removed" if args.apply else "would remove"
    print(f"{verb} {units} unit(s) unused for {args.days:g}+ days, {size / 2**30:.1f} GiB")
    for profile in skipped:
        print(f"⚠ skipped {profile}: a cargo process holds its build lock")

    if args.ensure_free is not None and args.apply and free_gib(target) < args.ensure_free:
        for profile in profiles(target):
            with build_lock(profile) as held:
                if held:
                    empty_profile(profile)
                    print(f"emptied {profile} (free space was under {args.ensure_free:g} GiB)")
                else:
                    print(f"⚠ could not empty {profile}: a cargo process holds its build lock")
    elif args.ensure_free is not None and not args.apply:
        print("(--ensure-free acts only with --apply)")

    print(f"free: {before:.1f} GiB -> {free_gib(target):.1f} GiB")
    if not args.apply:
        print("(dry run — pass --apply)")
    return 1 if skipped and args.apply else 0


if __name__ == "__main__":
    sys.exit(main())
