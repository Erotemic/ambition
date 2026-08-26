#!/usr/bin/env python3
"""Reclaim target/ space by keeping exactly the graphs you name.

⭐ THE LIVE SET COMES FROM CARGO, NOT FROM A CLOCK. `cargo build
--message-format=json` emits a `compiler-artifact` record per unit with the
files it resolved — including for `"fresh": true` units, so marking a warm graph
compiles nothing and still names every artifact. That is the whole design:

⛔⛔ TIMESTAMPS CANNOT WORK HERE, and both of them were measured. A genuine no-op
rebuild advances NEITHER mtime NOR atime — a rrlib stayed at 19:11:28 while the
clock read 19:11:41, and both filesystems mount `relatime` so an unread file's
atime never moves either. So "delete anything older than the stamp" deletes LIVE
artifacts, which is the flaw in `cargo sweep --stamp/--file` and in
`cargo mark-sweep reap --max-age-days` for this repo.

⛔ TWO MODES, BECAUSE THERE ARE TWO POPULATIONS. `CARGO_INCREMENTAL=1` and `=0`
produce DIFFERENT artifact hashes that coexist — measured, the same crate built
both ways yields `6eeced958ebf3728` and `93ca37900671d6c1` side by side, and
switching back rebuilds nothing. `run_game.sh` builds under the repo default
(incremental=true); `run_tests.py` sets `CARGO_INCREMENTAL=0`. A marker that
ignores the environment protects the wrong half — which is exactly what
`sweep_cargo_target.sh` does through `cargo mark-sweep`, whose `--cmd` list
shares one environment and therefore cannot express this.

Usage::

    scripts/sweep_target.py                       # both modes, dry run
    scripts/sweep_target.py --apply
    scripts/sweep_target.py --runs-only --apply   # keep the edit loop only
    scripts/sweep_target.py --cmd 'build -p ambition_app --bin ambition_game_bin' --apply
    scripts/sweep_target.py --drop-incremental --apply
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# The graphs each mode protects. `--cmd` REPLACES the run set, which is what
# `run_game.sh --sweep` uses to name the one shape it was asked for.
RUN_CMDS = [
    "build -p ambition_app --bin ambition_game_bin",
]
TEST_CMDS = [
    "test --no-run --workspace",
]

# Cargo's own subdirectories under a profile. Anything else under `target/` is
# somebody's output — captures, probe runs, logs — and is never touched.
CARGO_DIRS = ("deps", ".fingerprint", "build")


def target_dir() -> Path:
    if env := os.environ.get("CARGO_TARGET_DIR"):
        return Path(env)
    out = subprocess.run(
        ["cargo", "metadata", "--format-version=1", "--no-deps"],
        cwd=REPO, capture_output=True, text=True, check=True,
    )
    return Path(json.loads(out.stdout)["target_directory"])


def mark(cmd: str, incremental: bool) -> set[str]:
    """Artifact paths cargo resolves for one command, under one environment.

    Returns basenames: the same artifact is reachable through several paths and
    cargo hardlinks heavily, so identity here is the FILENAME cargo printed.
    """
    env = dict(os.environ, CARGO_INCREMENTAL="1" if incremental else "0")
    proc = subprocess.run(
        ["cargo", *cmd.split(), "--message-format=json"],
        cwd=REPO, capture_output=True, text=True, env=env,
    )
    if proc.returncode != 0:
        tail = "\n".join(proc.stderr.strip().splitlines()[-8:])
        raise SystemExit(
            f"⛔ marking failed, so NOTHING was swept:\n  cargo {cmd}\n"
            f"  CARGO_INCREMENTAL={'1' if incremental else '0'}\n{tail}"
        )
    live: set[str] = set()
    for line in proc.stdout.splitlines():
        try:
            msg = json.loads(line)
        except ValueError:
            continue
        if msg.get("reason") != "compiler-artifact":
            continue
        for path in msg.get("filenames") or []:
            live.add(Path(path).name)
        if executable := msg.get("executable"):
            live.add(Path(executable).name)
    return live


def hashes_of(names: set[str]) -> set[str]:
    """The 16-hex suffixes in a set of artifact names.

    `.fingerprint/<crate>-<hash>/` and `build/<crate>-<hash>/` are keyed by the
    same hash the artifact filename carries, so keeping a `deps` file keeps its
    bookkeeping with it. Anything whose hash no artifact mentions is dead.
    """
    out = set()
    for name in names:
        stem = name.split(".")[0]
        if "-" in stem:
            tail = stem.rsplit("-", 1)[1]
            if len(tail) == 16 and all(c in "0123456789abcdef" for c in tail):
                out.add(tail)
    return out


def sweep(target: Path, live: set[str], live_hashes: set[str], apply: bool) -> tuple[int, int]:
    freed = removed = 0
    for profile_dir in target.iterdir():
        if not profile_dir.is_dir():
            continue
        for sub in CARGO_DIRS:
            d = profile_dir / sub
            if not d.is_dir():
                continue
            for entry in d.iterdir():
                if sub == "deps":
                    keep = entry.name in live
                else:
                    keep = any(h in entry.name for h in live_hashes)
                if keep:
                    continue
                try:
                    size = (
                        sum(f.stat().st_size for f in entry.rglob("*") if f.is_file())
                        if entry.is_dir()
                        else entry.stat().st_size
                    )
                except OSError:
                    size = 0
                freed += size
                removed += 1
                if apply:
                    shutil.rmtree(entry, ignore_errors=True) if entry.is_dir() else entry.unlink(missing_ok=True)
    return removed, freed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--apply", action="store_true", help="actually delete; without it, report only")
    ap.add_argument("--runs-only", action="store_true", help="protect only the run_game graph")
    ap.add_argument("--tests-only", action="store_true", help="protect only the run_tests graph")
    ap.add_argument("--cmd", action="append", default=[],
                    help="replace the RUN graph with this cargo command (repeatable)")
    ap.add_argument("--drop-incremental", action="store_true",
                    help="also delete incremental/ — pure edit-loop cache, and usually the largest thing here")
    args = ap.parse_args()

    target = target_dir()
    if not target.is_dir():
        print(f"nothing to sweep: {target} does not exist")
        return 0

    live: set[str] = set()
    if not args.tests_only:
        for cmd in (args.cmd or RUN_CMDS):
            print(f"marking (incremental) cargo {cmd}")
            live |= mark(cmd, incremental=True)
    if not args.runs_only:
        for cmd in TEST_CMDS:
            print(f"marking (CARGO_INCREMENTAL=0) cargo {cmd}")
            live |= mark(cmd, incremental=False)

    # ⛔ A MARK THAT FOUND NOTHING MUST NOT SWEEP EVERYTHING.
    if not live:
        raise SystemExit("⛔ the mark produced no artifacts — refusing to sweep")
    print(f"live set: {len(live)} artifacts")

    removed, freed = sweep(target, live, hashes_of(live), args.apply)
    verb = "removed" if args.apply else "would remove"
    print(f"{verb} {removed} entries, {freed / 1024**3:.2f} GB")

    incr_total = 0
    for profile_dir in target.iterdir():
        incr = profile_dir / "incremental"
        if incr.is_dir():
            incr_total += sum(f.stat().st_size for f in incr.rglob("*") if f.is_file())
            if args.drop_incremental and args.apply:
                shutil.rmtree(incr, ignore_errors=True)
    if incr_total:
        if args.drop_incremental:
            print(f"{verb} incremental/: {incr_total / 1024**3:.2f} GB")
        else:
            print(f"incremental/ holds {incr_total / 1024**3:.2f} GB "
                  f"— pass --drop-incremental to reclaim it (edit-loop cache only; "
                  f"run_tests.py sets CARGO_INCREMENTAL=0 and never reads it)")
    if not args.apply:
        print("(dry run — pass --apply)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
