#!/usr/bin/env python3
"""Drop the raw halves of profile bundles, keep everything the ledger reads.

⭐ **A BUNDLE WHOSE RAW HALF IS GONE CAN STILL BE RE-INGESTED. A DELETED BUNDLE
CANNOT.** On 2026-09-01, adding `sim_phases_ms` to the ledger backfilled 13 of 26
rows — the other 12 had no bundle left, so numbers that existed once are simply
unavailable now. The fix is not "keep everything": `profiles/` was 127 GB, of
which a single `tracy_zone_instances.csv` was **85 GB**, in a bundle that was
never in the ledger at all.

Keep the derived, readable, small artifacts the ingest and summary actually open
— the list below is taken FROM those two modules, not invented — and drop the
raw captures they were derived from.

⚠ `profiles/` is gitignored: this only ever touches local scratch. Nothing here
is recoverable from git, which is exactly why the keep-list is generous and the
default is a dry run.
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PROFILES = REPO / "dev/ambition_dev_measurements/profiles"

# Everything `profile_bundle_to_history.py` or `profile_bundle_summary.py`
# opens, plus the provenance files a reader needs to trust a row.
KEEP_SUFFIXES = {".csv", ".md", ".json", ".txt", ".status", ".command", ".stderr", ".stdout"}

# ⛔ THE EXCEPTIONS, AND THEY ARE THE WHOLE POINT. These match a kept suffix and
# are raw captures, not summaries. `tracy_zone_instances.csv` is one row per zone
# INSTANCE — 85 GB from a single run — while `tracy_zones.csv` is the aggregate
# the summary actually reads.
DROP_NAMES = {"tracy_zone_instances.csv"}
DROP_SUFFIXES = {".trace", ".data", ".gz", ".zst"}
DROP_DIRS = {"perf_windows"}


def classify(path: Path) -> bool:
    """True when `path` should be dropped."""
    if path.name in DROP_NAMES:
        return True
    if path.suffix in DROP_SUFFIXES:
        return True
    # perf.data.N rotations do not end in `.data`.
    return path.name.startswith("perf.data")


def prune(bundle: Path, apply: bool) -> tuple[int, int]:
    freed = 0
    count = 0
    for path in sorted(bundle.rglob("*")):
        if path.is_dir():
            if path.name in DROP_DIRS:
                size = sum(f.stat().st_size for f in path.rglob("*") if f.is_file())
                freed += size
                count += 1
                if apply:
                    shutil.rmtree(path, ignore_errors=True)
            continue
        if path.is_file() and classify(path):
            freed += path.stat().st_size
            count += 1
            if apply:
                path.unlink(missing_ok=True)
    return freed, count


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--apply", action="store_true", help="actually delete (default: dry run)")
    parser.add_argument("--profiles", type=Path, default=PROFILES)
    args = parser.parse_args()

    root: Path = args.profiles
    if not root.is_dir():
        print(f"no profiles directory at {root}")
        return 0

    total = 0
    # Loose archives beside the bundles, which are the same bytes twice.
    for archive in sorted(root.glob("*.tar.gz")):
        size = archive.stat().st_size
        total += size
        print(f"  {size / 1e9:8.2f} GB  {archive.name}")
        if args.apply:
            archive.unlink(missing_ok=True)

    for bundle in sorted(p for p in root.iterdir() if p.is_dir()):
        freed, count = prune(bundle, args.apply)
        if freed:
            total += freed
            print(f"  {freed / 1e9:8.2f} GB  {bundle.name}  ({count} item(s))")

    verb = "freed" if args.apply else "would free"
    print(f"\n{verb} {total / 1e9:.2f} GB")
    if not args.apply:
        print("dry run — pass --apply to delete. Only gitignored scratch is touched.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
