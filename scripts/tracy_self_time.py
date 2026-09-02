#!/usr/bin/env python3
"""Self time vs total for the schedule zones of a Tracy trace.

"Self" is time inside a zone but inside none of its children. For a Bevy
schedule that is executor work plus the tracing layer's own per-span cost; a
schedule that is half self time is not half real work. Measured 2026-09-01 on
the Ultra hall capture: GgrsSchedule 54% self, Update 52%, Render 47% — see
performance-and-iteration.md ("the Tracy build is a different program").

    python3 scripts/tracy_self_time.py <bundle-dir-or-trace> [--top N] [--match SUBSTR]

Needs tracy-csvexport matching the trace's Tracy version
(scripts/setup/install_profiling_tools.sh); the export is session-wide, not per window.
"""

from __future__ import annotations

import argparse
import csv
import io
import subprocess
import sys
from pathlib import Path


def export(trace: Path, self_time: bool) -> dict[str, dict]:
    cmd = ["tracy-csvexport"] + (["-e"] if self_time else []) + [str(trace)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit(f"tracy-csvexport failed ({proc.returncode}); is its version the trace's?")
    return {row["name"]: row for row in csv.DictReader(io.StringIO(proc.stdout))}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("path", type=Path, help="a profile bundle directory (uses its tracy.trace) or a .trace file")
    ap.add_argument("--top", type=int, default=25)
    ap.add_argument("--match", default="schedule{", help="zone-name substring to report (default: schedules)")
    args = ap.parse_args()
    trace = args.path if args.path.is_file() else args.path / "tracy.trace"
    if not trace.is_file():
        raise SystemExit(f"no trace at {trace}")
    totals = export(trace, self_time=False)
    selfs = export(trace, self_time=True)
    rows = []
    for name, total in totals.items():
        if args.match not in name:
            continue
        own = selfs.get(name)
        total_ns = int(total["total_ns"])
        if not own or total_ns == 0:
            continue
        self_ns = int(own["total_ns"])
        rows.append((total_ns, self_ns, int(total["counts"]), name))
    rows.sort(reverse=True)
    print(f"{'zone':48s} {'total ms':>10} {'self ms':>10} {'self%':>6} {'count':>8}")
    for total_ns, self_ns, count, name in rows[: args.top]:
        print(f"{name[:48]:48s} {total_ns/1e6:10.1f} {self_ns/1e6:10.1f} {100*self_ns/total_ns:6.1f} {count:8d}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
