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
        # Created by the apply, which locks the profile even when cargo never had.
        ".cargo-lock",
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


def test_the_lock_is_held_through_every_delete_even_when_no_lock_file_existed(tmp_path):
    """⛔ THE RACE THIS REPOSITORY ALREADY FIXED ONCE (clean_workspace_crates.sh,
    2026-09-17) AND THIS SCRIPT REINTRODUCED: a profile with no `.cargo-lock`
    took no lock, so a cargo starting meanwhile could create it, acquire it and
    build underneath the deletion. Every delete must happen while a competitor
    CANNOT take the lock."""
    lru = _module()
    now = time.time()
    profile = _tree(tmp_path, now)
    lock = profile / ".cargo-lock"
    assert not lock.exists(), "premise: the profile starts with no lock file"

    competitor_got_it: list[bool] = []
    real_remove = lru.remove

    def watched_remove(path):
        with open(lock, "a") as competitor:
            try:
                fcntl.flock(competitor, fcntl.LOCK_EX | fcntl.LOCK_NB)
                competitor_got_it.append(True)
                fcntl.flock(competitor, fcntl.LOCK_UN)
            except BlockingIOError:
                competitor_got_it.append(False)
        real_remove(path)

    lru.remove = watched_remove
    units, _, _ = lru.sweep(tmp_path, days=7, apply=True, now=now)
    assert units == 1 and competitor_got_it, "premise: something was deleted"
    assert not any(competitor_got_it), "a delete ran while another process could take cargo's lock"


def test_a_dry_run_writes_nothing(tmp_path):
    lru = _module()
    profile = _tree(tmp_path, time.time())
    lru.sweep(tmp_path, days=7, apply=False)
    assert not (profile / ".cargo-lock").exists(), "a report created the lock file"


def test_an_unbound_repo_target_refuses_to_apply(monkeypatch):
    """⛔ On virtiofs an unbound `target/` exposes the shadowed duplicate under the
    mount point; reclaiming it is the maintainer's call. The refusal is the
    canonical `target_bindmount.sh --check`, consulted for the repo's own target
    and only there."""
    lru = _module()
    calls = []

    class Result:
        returncode = 2
        stderr = "not bound"

    def fake_run(argv, **_):
        calls.append(argv)
        return Result()

    monkeypatch.setattr(lru.subprocess, "run", fake_run)
    assert lru.bind_refusal(lru.REPO / "target") is not None
    assert calls and calls[0][-1] == "--check"
    calls.clear()
    assert lru.bind_refusal(pathlib.Path("/somewhere/else/target")) is None
    assert not calls, "a CARGO_TARGET_DIR elsewhere is not a bind point"


def test_main_refuses_before_touching_anything(tmp_path, monkeypatch):
    lru = _module()
    now = time.time()
    profile = _tree(tmp_path, now)
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path))
    monkeypatch.setattr(lru, "bind_refusal", lambda target: "REFUSING")
    monkeypatch.setattr(sys, "argv", ["sweep", "--apply", "--ensure-free", "1e9"])
    assert lru.main() == 2
    assert (profile / "deps" / f"libglam-{OLD}.rlib").exists()
    assert (profile / "ambition_game_bin").exists(), "--ensure-free emptied a refused target"


def test_a_rustc_incremental_session_is_its_own_unit(tmp_path):
    """rustc names sessions `<crate>-<suffix>` with a suffix that is NOT a cargo
    unit hash (e.g. `aipg_client-cn2ostj37iq8`), so the artifact regex cannot see
    them — and `incremental/` is routinely the largest part of a profile."""
    lru = _module()
    now = time.time()
    profile = tmp_path / "debug"
    _touch(profile / "deps" / f"libx-{LIVE}.rlib", 0, now)
    stale = profile / "incremental" / "aipg_client-cn2ostj37iq8"
    fresh = profile / "incremental" / "ambition_core-1a2b3c4d5e6f7"
    for session, age in ((stale, 30), (fresh, 0)):
        _touch(session / "s-h0abc-1xyz" / "dep-graph.bin", age, now)
        stamp = now - age * 86400
        os.utime(session / "s-h0abc-1xyz", (stamp, stamp))
        os.utime(session, (stamp, stamp))
    units, size, _ = lru.sweep(tmp_path, days=7, apply=True, now=now)
    assert (units, size) == (1, 10)
    assert not stale.exists() and fresh.exists()
