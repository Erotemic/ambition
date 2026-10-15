#!/usr/bin/env python3
"""Compare the systems that ran in one frame, crate by crate, across runs.

Each input is the list that `[census] systems_ran` writes when
`AMBITION_SYSTEM_LIST=PATH` is set (`ran|idle <schedule> <name>` per line):

    AMBITION_PROFILE_CENSUS=1 AMBITION_SYSTEM_LIST=hub_mary_o.txt \\
        target/debug/capture_scene --route mary_o_gameplay out.png 1280x720 --warmup 600
    AMBITION_PROFILE_CENSUS=1 AMBITION_SYSTEM_LIST=solo_mary_o.txt \\
        target/debug/capture_mary_o out.png 1280x720 --warmup 600
    python3 scripts/system_census_table.py hub_mary_o.txt solo_mary_o.txt

A standalone demo needs `--features bevy/debug`, or Bevy hides system names.
Each cell is `ran/registered`. A system counts for the crate it was made for:
a generic system counts for the crate of its innermost type argument. The
census's own systems are left out. `--all` adds the crates that are not ours.
"""

from __future__ import annotations

import collections
import pathlib
import sys


def crate(name: str) -> str:
    inner = name[name.rfind("<") + 1:] if "<" in name else name
    inner = inner.lstrip("&( ")
    if inner.startswith("Pipe("):
        inner = inner[len("Pipe("):]
    return inner.split("::")[0].rstrip(">), ")


def read(path: str) -> dict[str, tuple[int, int]]:
    ran: collections.Counter[str] = collections.Counter()
    registered: collections.Counter[str] = collections.Counter()
    for line in open(path):
        state, schedule, name = line.rstrip("\n").split(" ", 2)
        if "runtime_census" in name or schedule.startswith("FramePhaseMark"):
            continue
        key = crate(name)
        registered[key] += 1
        ran[key] += state == "ran"
    return {key: (ran[key], registered[key]) for key in registered}


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--all"]
    if not args:
        print(__doc__)
        return 2
    everything = "--all" in sys.argv
    tables = [read(path) for path in args]
    names = [pathlib.Path(path).stem[:18] for path in args]
    crates = {key for table in tables for key in table}
    if not everything:
        crates = {key for key in crates if key.startswith("ambition")}
    print(f"{'crate':42s}" + "".join(f"{n:>20s}" for n in names))
    totals = [sum(c[0] for c in t.values()) for t in tables], [sum(c[1] for c in t.values()) for t in tables]
    print(f"{'(all crates)':42s}" + "".join(f"{f'{r}/{g}':>20s}" for r, g in zip(*totals)))
    for key in sorted(crates, key=lambda k: (-max(t.get(k, (0, 0))[0] for t in tables), k)):
        cells = [f"{t[key][0]}/{t[key][1]}" if key in t else "-" for t in tables]
        print(f"{key:42s}" + "".join(f"{c:>20s}" for c in cells))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
