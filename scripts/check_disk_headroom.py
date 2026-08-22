#!/usr/bin/env python3
"""Is there room on the volume cargo writes to?

⛔ **this volume has filled to 100% three times.** 2026-07-31 twice (S14 and
again four hours later), and 2026-08-02 during a long autonomous run. Every time
the symptom was the same and the cause appeared nowhere in it: a mid-build ENOSPC
surfaces as **a wall of unrelated compile errors**, so the reader debugs a
phantom regression in whichever crate was unlucky.

`run_tests.py` has refused below a floor since S14, and that refusal works. The
hole it does not cover is the one that filled the disk on 2026-08-02: **a bare
`cargo test --workspace` typed directly.** The guard lived inside the suite
runner, and the command an agent or a human actually reaches for when they want
"just run the tests" goes straight past it.

So the check moves here, where anything can call it, and `run_tests.py` imports
it rather than keeping a second copy.

## Why a floor of tens of gigabytes and not "some free space"

A full suite is ~28 feature jobs, and **every feature combination is a separate
variant of the dependency graph that cargo never prunes** — measured at ~295 G of
`debug/deps` in one day. `CARGO_INCREMENTAL=0` fixed the incremental half of the
2026-07-31 fill and not this half. So the floor is a floor for a SUITE; a single
`cargo check` needs far less, which is why the threshold is an argument.

Usage:
    python3 scripts/check_disk_headroom.py            # suite floor (40 GB)
    python3 scripts/check_disk_headroom.py --min-gb 5 # enough for one build
    python3 scripts/check_disk_headroom.py --quiet    # exit code only
"""

from __future__ import annotations

import argparse
import os
import shutil
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# A floor with a little room, not a precise budget — the point is to refuse BEFORE a job dies of
# ENOSPC and reports it as a compile error.
MIN_FREE_GB = 40.0


def target_dir() -> Path:
    """Where cargo actually writes — which is NOT always under the repo.

    ⚠ **this function exists because the first version of the disk guard read
    the wrong filesystem.** It measured the REPO's volume, and this checkout has
    the repo on a 1.8 TB disk while `.cargo/config.toml` points `target-dir` at
    `/home/joncrall/ambition-target` on a 387 GB one. The guard would have
    reported 380 GB free while the volume that actually fills had two — a green
    instrument answering a question nobody asked.
    """
    if env := os.environ.get("CARGO_TARGET_DIR"):
        return Path(env)
    config = REPO / ".cargo" / "config.toml"
    if config.is_file():
        for line in config.read_text().splitlines():
            line = line.strip()
            if line.startswith("target-dir"):
                _, _, value = line.partition("=")
                value = value.strip().strip('"').strip("'")
                if value:
                    return Path(value)
    return REPO / "target"


def free_gb_on_target() -> float:
    """Free space on the volume cargo writes to.

    Falls back to the repo's own volume only when the target directory's parent
    does not exist yet.
    """
    path = target_dir()
    while not path.exists() and path != path.parent:
        path = path.parent
    return shutil.disk_usage(path).free / 1024**3


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--min-gb",
        type=float,
        default=MIN_FREE_GB,
        help=f"required free GB (default {MIN_FREE_GB}, the full-suite floor)",
    )
    parser.add_argument("--quiet", action="store_true", help="exit code only")
    args = parser.parse_args()

    free = free_gb_on_target()
    where = target_dir()
    if free < args.min_gb:
        print(
            f"REFUSING: {free:.1f} GB free on {where}, need {args.min_gb:.0f} GB.\n\n"
            "⚠ do not start a build. A mid-build ENOSPC does not say 'disk full' — "
            "it surfaces as unrelated compile errors in whichever crate was "
            "unlucky, and the real cause appears nowhere.\n\n"
            f"  Look first: du -sh {where}/* | sort -h\n"
            f"  Free it:    cargo clean            (then expect one full rebuild)",
            file=sys.stderr,
        )
        return 1

    if not args.quiet:
        print(f"OK: {free:.1f} GB free on {where} (floor {args.min_gb:.0f}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
