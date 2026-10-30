#!/usr/bin/env python3
"""Can a CPU punish a whiff at all? A census from frame data and reaction delay.

For each attacker move (on the Smash stage: the fighter facet's `move_timing`,
which retimes the authored windows) thrown at its own maximum reach and
missing, the attacker cannot act for `total - last active` seconds after its
active window ends. A defender standing just outside that reach answers:

    sees the whiff end        + the rung's reaction delay (`reaction_ms`)
    decides                   + up to one decision interval (5 ticks)
    closes the gap            + (attacker reach - its own reach) / run speed
    starts its fastest move   + its `hit_s`

The whiff is punishable at a rung when that total fits inside the attacker's
recovery. The margin printed is recovery minus the answer, in 60 Hz frames:
positive is a window the brain could convert, negative is one no policy can.

REASONED simplifications, stated so a reader can weigh them: reach is the
farthest forward edge of the move's ACTIVE volumes in the authored RON (no
facet scaling; projectile launchers are skipped); the defender's best answer
is any of its grounded moves (not aerials); travel is at max run speed from
rest (no dash acceleration); aerials are treated as landing at once (their
landing lag is not added). It decides FEASIBILITY, not policy.

    python3 scripts/smash_punish_windows.py
    python3 scripts/smash_punish_windows.py --attacker pugnacious_polygon --defender pointed_polygon
"""
from __future__ import annotations

import argparse
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "game/ambition_content/assets/data"
HZ = 60.0
POLYGONS = ["pointed_polygon", "pugnacious_polygon", "projectile_polygon"]
DECISION_INTERVAL_S = 5 / HZ

TIMING = re.compile(
    r'"([a-z_0-9]+)":\s*\(hit_s:\s*([0-9.]+),\s*last_s:\s*([0-9.]+),\s*total_s:\s*([0-9.]+)'
)
MOVE_HEAD = re.compile(r'\(\s*id:\s*"([a-z_0-9]+)",\s*display_name')
ACTIVE = re.compile(r"tag:\s*Active,\s*volumes:\s*\[(.*?)\],\s*motion_scale", re.S)
RECT = re.compile(r"offset:\s*\(\s*(-?[0-9.]+),\s*(-?[0-9.]+)\s*\),\s*half_extents:\s*\(\s*([0-9.]+),\s*([0-9.]+)\s*\)")
GROUNDED_AIR = ("air_", "_air", "aerial")


def ladder() -> dict[int, float]:
    text = (DATA / "fighter_brain_ladder.ron").read_text()
    return {int(l): float(ms) / 1000.0 for l, ms in re.findall(r"level:\s*(\d+),\s*reaction_ms:\s*([0-9.]+)", text)}


def fighter(name: str) -> dict:
    facet = (DATA / f"fighters/{name}.ron").read_text()
    run = float(re.search(r"max_run_speed:\s*Some\(([0-9.]+)\)", facet).group(1))
    timing = {m: (float(h), float(l), float(t)) for m, h, l, t in TIMING.findall(facet)}
    moves = (DATA / f"movesets/{name}.ron").read_text()
    heads = list(MOVE_HEAD.finditer(moves))
    reach: dict[str, float] = {}
    for i, head in enumerate(heads):
        body = moves[head.start() : heads[i + 1].start() if i + 1 < len(heads) else len(moves)]
        edges = [
            float(ox) + float(hx)
            for volumes in ACTIVE.findall(body)
            for ox, _oy, hx, _hy in RECT.findall(volumes)
        ]
        if edges:
            reach[head.group(1)] = max(edges)
    return {"run": run, "timing": timing, "reach": reach}


def grounded(move: str) -> bool:
    return not any(tag in move for tag in GROUNDED_AIR)


def answer_s(defender: dict, gap: float) -> float | None:
    """The defender's fastest hit on a body `gap` px past its own front."""
    best = None
    for move, (hit_s, _last, _total) in defender["timing"].items():
        if not grounded(move) or move not in defender["reach"]:
            continue
        travel = max(0.0, gap - defender["reach"][move]) / defender["run"]
        t = travel + hit_s
        best = t if best is None or t < best else best
    return best


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--attacker", choices=POLYGONS)
    parser.add_argument("--defender", choices=POLYGONS)
    parser.add_argument("--levels", default="5,9")
    args = parser.parse_args()
    delays = ladder()
    levels = [int(x) for x in args.levels.split(",")]
    fighters = {name: fighter(name) for name in POLYGONS}
    pairs = [
        (a, d)
        for a in ([args.attacker] if args.attacker else POLYGONS)
        for d in ([args.defender] if args.defender else POLYGONS)
    ]
    totals = {lv: [0, 0] for lv in levels}
    for attacker, defender in pairs:
        att, dfn = fighters[attacker], fighters[defender]
        print(f"\n{attacker} whiffs at max reach, {defender} answers (margin in frames; + = punishable)")
        print(f"  {'move':<36} {'reach':>6} {'recov':>6} " + " ".join(f"{'r' + str(lv):>6}" for lv in levels))
        for move, (_hit, last, total) in sorted(att["timing"].items(), key=lambda kv: -(kv[1][2] - kv[1][1])):
            if move not in att["reach"]:
                continue
            recovery = total - last
            fastest = answer_s(dfn, att["reach"][move])
            if fastest is None:
                continue
            cells = []
            for lv in levels:
                answer = delays[lv] + DECISION_INTERVAL_S + fastest
                margin = (recovery - answer) * HZ
                totals[lv][1] += 1
                totals[lv][0] += margin > 0
                cells.append(f"{margin:>+6.0f}")
            print(f"  {move:<36} {att['reach'][move]:>6.0f} {recovery * HZ:>6.0f} " + " ".join(cells))
    print("\npunishable whiffs (attacker move x defender pairs):")
    for lv in levels:
        hit, n = totals[lv]
        print(f"  rung {lv} (reaction {delays[lv] * 1000:.0f} ms): {hit} of {n} ({100.0 * hit / max(n, 1):.0f}%)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
