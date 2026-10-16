#!/usr/bin/env python3
"""Report of `scripts/executor_ab.sh`: per arm, the median over repetitions of
the simulation wall time per tick (`[census] sim_phases`), the main-world
schedule time per second (`[census] phases` x frames), frames per second, and
process CPU. Census windows before SKIP seconds are left out (loading).

Usage: scripts/executor_ab_report.py OUTDIR [SKIP=8]
"""
import glob
import os
import re
import statistics
import sys
from collections import defaultdict


def main() -> int:
    out = sys.argv[1]
    skip = float(sys.argv[2]) if len(sys.argv) > 2 else 8.0
    runs = defaultdict(list)
    for err in sorted(glob.glob(os.path.join(out, "*.err"))):
        arm = os.path.basename(err).rsplit(".", 2)[0]
        sim, ticks, seconds, main, frames = [], 0, 0, [], []
        cpu = None
        for line in open(err, errors="replace"):
            if line.startswith("time wall="):
                wall, user, sys_ = map(float, re.findall(r"=([\d.]+)", line))
                cpu = (wall, user + sys_)
            at = re.search(r" t=([\d.]+)", line)
            if not at or float(at.group(1)) < skip:
                continue
            if line.startswith("[census] sim_phases "):
                values = re.findall(r"[\w.]+=([\d.]+)", line.split("unmeasured=", 1)[1])
                sim.append(sum(float(v) for v in values))
                ticks += int(re.search(r"ticks=(\d+)", line).group(1))
                seconds += 1
            elif line.startswith("[census] phases "):
                count = int(re.search(r"frames=(\d+)", line).group(1))
                values = dict(re.findall(r"(\w+)=([\d.]+)", line.split("frames=", 1)[1]))
                main.append(count * sum(float(v) for k, v in values.items() if k not in ("outside", "frames")))
                frames.append(count)
        if not sim or not main or cpu is None:
            print(f"skip {err}: no census windows after {skip} s or no time line")
            continue
        runs[arm].append(dict(sim=statistics.mean(sim), tps=ticks / seconds, main=sum(main) / len(main),
                              fps=sum(frames) / len(frames), wall=cpu[0], cpu=cpu[1]))
    print(f"{'arm':13} {'reps':>4} {'sim ms/tick':>11} {'ticks/s':>7} {'main ms/s':>9} {'frames/s':>8} {'cpu s':>6} {'wall s':>6}")
    for arm in ("parallel", "frame-serial", "serial"):
        rows = runs.get(arm)
        if not rows:
            continue
        med = lambda key: statistics.median(row[key] for row in rows)
        print(f"{arm:13} {len(rows):4} {med('sim'):11.2f} {med('tps'):7.1f} {med('main'):9.0f} {med('fps'):8.1f} "
              f"{med('cpu'):6.1f} {med('wall'):6.1f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
