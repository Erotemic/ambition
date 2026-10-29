#!/usr/bin/env python3
"""Frame data of a moveset's moves, in 60 Hz frames, from the authored RON.

One line per move: the duration, and the startup, active, recovery, cancel
and armor windows, plus the landing lag. The scan reads the authored windows,
not the running game. For what a fighter DOES, use `fighter_motion` (motion)
and `moveset_takes` (combat).

    python3 scripts/smash_frame_data.py game/ambition_content/assets/data/movesets/player_robot.ron
    python3 scripts/smash_frame_data.py game/ambition_content/assets/data/movesets/goblin.ron jab,tilt_forward,air_neutral

With no move list, every move in the file is printed. Written for the audit in
`docs/planning/demos/smash-feel-audit-2026-10-29.md`.
"""
from __future__ import annotations

import re
import sys

HZ = 60.0
MOVE_HEAD = re.compile(r'\(\s*id:\s*"([a-z_0-9]+)",\s*display_name')
WINDOW = re.compile(r"start_s:\s*([0-9.]+),\s*end_s:\s*([0-9.]+),\s*tag:\s*([A-Za-z]+)")


def frames(seconds: float) -> int:
    return int(round(seconds * HZ))


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    src = open(sys.argv[1]).read()
    wanted = set(sys.argv[2].split(",")) if len(sys.argv) > 2 else None
    heads = list(MOVE_HEAD.finditer(src))
    for i, head in enumerate(heads):
        move = head.group(1)
        if wanted and move not in wanted:
            continue
        end = heads[i + 1].start() if i + 1 < len(heads) else len(src)
        body = src[head.start() : end]
        dur = re.search(r"duration_s:\s*([0-9.]+)", body)
        lag = re.search(r"landing_lag_s:\s*(Some\(([0-9.]+)\)|None)", body)
        tags: dict[str, list[tuple[float, float]]] = {}
        for a, b, tag in WINDOW.findall(body):
            tags.setdefault(tag, []).append((float(a), float(b)))

        def rng(tag: str) -> str:
            return " ".join(f"{frames(a)}-{frames(b)}" for a, b in tags.get(tag, []))

        lag_txt = "-"
        if lag:
            lag_txt = "none" if lag.group(1) == "None" else f"{frames(float(lag.group(2)))}f"
        print(
            f"{move:28s} dur={frames(float(dur.group(1))) if dur else '?':>3}f"
            f"  startup[{rng('Startup')}] active[{rng('Active')}] recov[{rng('Recovery')}]"
            f" cancel[{rng('Cancelable')}] armor[{rng('Armor')}] landlag={lag_txt}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
