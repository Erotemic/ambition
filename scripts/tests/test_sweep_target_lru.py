"""The LRU sweep removes a unit only when every file it owns has gone unused."""

from __future__ import annotations

import fcntl
import importlib.util
import os
import pathlib
import re
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts/sweep_target_lru.py"

OLD = "0123456789abcdef"
LIVE = "fedcba9876543210"
SPLIT = "00112233aabbccdd"
HASHED_DIR = re.compile(r"-[0-9a-f]{16}$")


def _module():
    spec = importlib.util.spec_from_file_location("sweep_lru", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    # A dataclass resolves its module through `sys.modules` while it is built.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _touch(path: pathlib.Path, age_days: float, now: float) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * 10)
    stamp = now - age_days * 86400
    os.utime(path, (stamp, stamp))
    # A unit DIRECTORY (`.fingerprint/<pkg>-<hash>/`) was last written when its
    # files were; creating the fixture must not make it look used today.
    if HASHED_DIR.search(path.parent.name):
        os.utime(path.parent, (stamp, stamp))


def _tree(root: pathlib.Path, now: float) -> pathlib.Path:
    profile = root / "debug"
    # A unit nothing has used for 30 days, across all three unit directories.
    _touch(profile / "deps" / f"libglam-{OLD}.rlib", 30, now)
    _touch(profile / "deps" / f"glam-{OLD}.d", 30, now)
    _touch(profile / ".fingerprint" / f"glam-{OLD}" / "lib-glam", 30, now)
    # A unit used an hour ago.
    _touch(profile / "deps" / f"libbevy-{LIVE}.rlib", 1 / 24, now)
    # ⛔ A unit whose rlib is old but whose fingerprint was just written: it is
    # ONE unit, so the fresh half keeps the stale half. Deleting by file age
    # alone would leave a fingerprint claiming an artifact that is gone.
    _touch(profile / "deps" / f"libsplit-{SPLIT}.rlib", 30, now)
    _touch(profile / ".fingerprint" / f"split-{SPLIT}" / "lib-split", 0, now)
    # Not hash-keyed: a top-level binary and a capture are never grouped.
    _touch(profile / "ambition_game_bin", 30, now)
    return profile


def test_only_a_wholly_unused_unit_is_removed(tmp_path):
    lru = _module()
    now = time.time()
    profile = _tree(tmp_path, now)

    units, size, skipped = lru.sweep(tmp_path, days=7, apply=False, now=now)
    assert (units, skipped) == (1, [])
    assert size == 30  # three 10-byte files
    assert (profile / "deps" / f"libglam-{OLD}.rlib").exists(), "a dry run deleted"

    lru.sweep(tmp_path, days=7, apply=True, now=now)
    remaining = sorted(p.relative_to(profile).as_posix() for p in profile.rglob("*") if p.is_file())
    assert remaining == [
        f".fingerprint/split-{SPLIT}/lib-split",
        "ambition_game_bin",
        f"deps/libbevy-{LIVE}.rlib",
        f"deps/libsplit-{SPLIT}.rlib",
    ]


def test_a_profile_cargo_is_building_is_not_touched(tmp_path):
    lru = _module()
    now = time.time()
    profile = _tree(tmp_path, now)
    lock = profile / ".cargo-lock"
    lock.touch()
    with open(lock, "a") as held:
        fcntl.flock(held, fcntl.LOCK_EX)
        units, _, skipped = lru.sweep(tmp_path, days=7, apply=True, now=now)
    assert skipped == [str(profile)]
    assert units == 0
    assert (profile / "deps" / f"libglam-{OLD}.rlib").exists()


def test_emptying_a_profile_keeps_the_directory(tmp_path):
    """`target/debug` is a bind mount here: remove its contents, never itself."""
    lru = _module()
    profile = _tree(tmp_path, time.time())
    lru.empty_profile(profile)
    assert profile.is_dir()
    assert list(profile.iterdir()) == []
