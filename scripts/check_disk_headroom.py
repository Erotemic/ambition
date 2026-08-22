#!/usr/bin/env python3
"""Check free space on the filesystem Cargo actually writes to.

The target directory follows `CARGO_TARGET_DIR`, then `.cargo/config.toml`, then
the repository-local `target/`. The default threshold is sized for a full suite;
callers performing smaller work may request a lower floor.

Usage::

    python3 scripts/check_disk_headroom.py
    python3 scripts/check_disk_headroom.py --min-gb 5
    python3 scripts/check_disk_headroom.py --quiet"""

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
    """Resolve the Cargo target directory from environment, config, or repo default."""
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
