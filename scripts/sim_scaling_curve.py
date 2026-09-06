#!/usr/bin/env python3
"""Per-phase sim cost vs actor count, on the PRODUCTION rollback host, headless, no Tracy.

Runs `examples/hall_bench` (the shipped local session in one room) at each
population cap, interleaved across repetitions, and prints the median-of-windows
per-phase table from the `[census] sim_phases` rows. Different rooms are not a
scaling experiment; this is the same room with `AMBITION_ACTOR_POPULATION_CAP`.

    python3 scripts/sim_scaling_curve.py                 # builds hall_bench (profiling, no `profile` feature)
    python3 scripts/sim_scaling_curve.py --caps 2,64,130 --reps 3 --ticks 3000

The first repetition is discarded: the binary is 1.5 GB and its first run pays
the page cache. `--bin` points at an already-built example to skip the build.
"""

from __future__ import annotations

import argparse
import collections
import os
import re
import statistics
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
SIM_PHASES = re.compile(r"\[census\] sim_phases t=\S+ ticks=(\d+)(.*)")
BODIES = re.compile(r"\[census\] populations .* AbilityBase=(\d+)")
NOT_PHASES = {"views", "offered", "kept", "kept_max", "actor_cap"}


def build(target_dir: Path) -> Path:
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(target_dir))
    subprocess.run(
        ["cargo", "build", "--example", "hall_bench", "--profile", "profiling", "-p", "ambition_app"],
        cwd=REPO, env=env, check=True,
    )
    return target_dir / "profiling" / "examples" / "hall_bench"


def run_one(binary: Path, cap: int, ticks: int, room: str) -> tuple[float, dict[str, list[float]], int, int]:
    env = dict(os.environ, AMBITION_PROFILE_CENSUS="1", AMBITION_ACTOR_POPULATION_CAP=str(cap))
    proc = subprocess.run(
        [str(binary), "--ticks", str(ticks), "--room", room],
        cwd=REPO, env=env, capture_output=True, text=True, errors="replace",
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr[-2000:])
        raise SystemExit(f"hall_bench failed at cap={cap} (exit {proc.returncode})")
    m = re.search(r"ms_per_tick median=([0-9.]+)", proc.stdout)
    tick_ms = float(m.group(1)) if m else float("nan")
    phases: dict[str, list[float]] = collections.defaultdict(list)
    bodies = 0
    never_closed = 0
    for line in proc.stderr.splitlines():
        if "NEVER CLOSED" in line:
            never_closed += 1
        if (m := SIM_PHASES.search(line)):
            for key, value in re.findall(r"([A-Za-z_.]+)=([0-9.]+)", m.group(2)):
                phases[key].append(float(value))
        if (m := BODIES.search(line)):
            bodies = int(m.group(1))
    return tick_ms, phases, bodies, never_closed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--caps", default="2,16,64,130,200")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--ticks", type=int, default=3000)
    ap.add_argument("--room", default="hall_of_characters")
    ap.add_argument("--bin", type=Path, help="an already-built hall_bench example")
    ap.add_argument("--target-dir", type=Path, default=REPO / "target" / "notrace",
                    help="cargo target dir for the no-`profile`-feature build (default target/notrace)")
    args = ap.parse_args()
    caps = [int(c) for c in args.caps.split(",")]
    binary = args.bin or build(args.target_dir)

    tick: dict[int, list[float]] = collections.defaultdict(list)
    phase: dict[str, dict[int, list[float]]] = collections.defaultdict(lambda: collections.defaultdict(list))
    bodies: dict[int, int] = {}
    never = 0
    for rep in range(args.reps):
        for cap in caps:
            tick_ms, phases, n, nc = run_one(binary, cap, args.ticks, args.room)
            never += nc
            print(f"rep{rep + 1} cap={cap} actors={n} ms_per_tick={tick_ms:.3f}", file=sys.stderr, flush=True)
            if rep == 0 and args.reps > 1:
                continue  # first pair pays the page cache
            tick[cap].append(tick_ms)
            bodies[cap] = n
            for key, values in phases.items():
                phase[key][cap].extend(values[1:] if len(values) > 2 else values)

    def med(values: list[float]) -> float:
        return statistics.median(values) if values else float("nan")

    names = sorted((k for k in phase if k not in NOT_PHASES), key=lambda k: -med(phase[k][caps[-1]]))
    w = 40
    print(f"\nroom={args.room} ticks={args.ticks} reps={args.reps} (first discarded); ms/tick, median of 1 s windows")
    print(f"{'cap':{w}}" + "".join(f"{c:>8}" for c in caps))
    print(f"{'actors (AbilityBase)':{w}}" + "".join(f"{bodies.get(c, 0):>8}" for c in caps))
    print(f"{'WHOLE TICK (wall, harness)':{w}}" + "".join(f"{med(tick[c]):8.3f}" for c in caps))
    print(f"{'sum of measured sim phases':{w}}" + "".join(f"{sum(med(phase[k][c]) for k in names):8.3f}" for c in caps))
    for k in names:
        print(f"{k:{w}}" + "".join(f"{med(phase[k][c]):8.3f}" for c in caps))
    print(f"\n'!! NEVER CLOSED' warnings: {never}")
    return 1 if never else 0


if __name__ == "__main__":
    raise SystemExit(main())
