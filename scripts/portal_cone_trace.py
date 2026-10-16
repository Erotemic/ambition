#!/usr/bin/env python3
"""Measure how smoothly portal windows open, close and change shape.

Reads the `ambition_portal::cone_trace` lines a run logs (run with
`RUST_LOG=ambition_portal::cone_trace=debug`, e.g. a `capture_scene` film) and
reports, for each window, the largest change of its drawn area from one frame
to the next, as a fraction of the view. A window that eases changes a little
each frame; a pop changes a lot in one.

    python3 scripts/portal_cone_trace.py RUN.log [--pop 0.05]
"""
import argparse
import re
import sys
from collections import defaultdict

LINE = re.compile(
    r"cone t=(?P<t>[\d.]+) ch=(?P<ch>\S+) blend=(?P<blend>[\d.]+) "
    r"target=(?P<target>[\d.]+) area=(?P<area>[\d.]+) x=(?P<x>-?[\d.]+) y=(?P<y>-?[\d.]+)"
    r"(?: cut=(?P<cut>[01]))?(?: view=(?P<vx>-?[\d.]+),(?P<vy>-?[\d.]+))?"
)


def windows(lines):
    by = defaultdict(list)
    for line in lines:
        m = LINE.search(line)
        if m:
            row = {k: (m[k] if k == "ch" else float(m[k] or 0)) for k in LINE.groupindex}
            by[m["ch"]].append(row)
    return by


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("log")
    ap.add_argument("--pop", type=float, default=0.05, help="area change per frame that counts as a pop")
    ap.add_argument("--camera-jump", type=float, default=40.0, help="view move in one frame (px) that is a crossing")
    args = ap.parse_args()
    with open(args.log, errors="replace") as f:
        by = windows(f)
    if not by:
        print("no cone_trace lines: run with RUST_LOG=ambition_portal::cone_trace=debug", file=sys.stderr)
        return 2
    # A handover: a window that was open is cut this frame (its viewer came
    # through it), and its partner takes the picture on the same frame.
    # A camera that jumps (it follows its body through a portal) moves every
    # window across the view in one frame; those frames are crossings too.
    cut_at = set()
    for rows in by.values():
        rows.sort(key=lambda r: r["t"])
        cut_at |= {b["t"] for a, b in zip(rows, rows[1:]) if b["cut"] and a["area"] > 0}
        cut_at |= {
            b["t"]
            for a, b in zip(rows, rows[1:])
            if a["area"] > 0 and abs(b["vx"] - a["vx"]) + abs(b["vy"] - a["vy"]) > args.camera_jump
        }
    worst_all = 0.0
    print(f"{'window':>8} {'frames':>6} {'open':>5} {'max d(area)':>12} {'pops':>5} {'handovers':>9}  worst at t")
    for ch, rows in sorted(by.items()):
        rows.sort(key=lambda r: r["t"])
        pairs = list(zip(rows, rows[1:]))
        handovers = sum(1 for _, b in pairs if b["t"] in cut_at)
        deltas = [(abs(b["area"] - a["area"]), b["t"]) for a, b in pairs if b["t"] not in cut_at]
        worst, at = max(deltas, default=(0.0, 0.0))
        pops = sum(1 for d, _ in deltas if d > args.pop)
        opened = sum(1 for r in rows if r["area"] > 0)
        worst_all = max(worst_all, worst)
        if opened == 0:
            continue
        print(f"{ch:>8} {len(rows):>6} {opened:>5} {worst:>12.4f} {pops:>5} {handovers:>9}  {at:.3f}")
    print(f"worst change in one frame, handovers apart: {worst_all:.4f} of the view")
    return 0


if __name__ == "__main__":
    sys.exit(main())
