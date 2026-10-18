#!/usr/bin/env python3
"""Derive a Smash fighter's `body:` patch from an Ultimate archetype.

A fighter that plays like Bowser should not copy Bowser's numbers: it should
differ from Smash's default fighter the way Bowser differs from Ultimate's
roster median. So each number of the default fighter (`base_body` in
`game/ambition_demo_smash/assets/data/match_rules.ron`) is scaled by the
anchor's ratio to the median, read from
`docs/related-work/data/smash_ultimate_fighter_stats.csv`:

    run, initial dash, air speed, air acceleration, gravity, fall and fast fall,
    traction (run_accel + ground_friction, split as the base splits it),
    roll distance (the speed; the duration is the base's),
    jump launch from height AND gravity:   v ~ sqrt(g * h),
    and the knockback weight:              weight / 100.

It prints a facet body to paste into the fighter's `smash_fighter` facet. The
facet is the authority; this is how its numbers were derived, so a reviewer can
re-derive or change the anchor.

Usage::

    python3 scripts/smash_fighter_shape.py Bowser
    python3 scripts/smash_fighter_shape.py "Captain Falcon" --strength 0.5
"""
import argparse
import csv
import math
import re
import statistics
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CSV = REPO / "docs/related-work/data/smash_ultimate_fighter_stats.csv"
RULES = REPO / "game/ambition_demo_smash/assets/data/match_rules.ron"


def number(cell):
    m = re.fullmatch(r"\s*([0-9.]+)\s*", cell or "")
    return float(m.group(1)) if m else None


def base_body():
    text = RULES.read_text()
    block = text[text.index("base_body: Some((") :]
    block = block[: block.index("\n    )),")]
    out = {}
    for name, value in re.findall(r"^\s+([a-z_]+): Some\(([0-9.]+)\)", block, re.M):
        out[name] = float(value)
    frac = re.search(r"short_hop_speed_frac: ([0-9.]+)", block)
    out["short_hop_speed_frac"] = float(frac.group(1))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("anchor", help="an Ultimate fighter, as the CSV names it")
    ap.add_argument(
        "--strength",
        type=float,
        default=1.0,
        help="how much of the anchor's difference to take (0 = the base, 1 = all of it)",
    )
    args = ap.parse_args()

    rows = {r["character"]: r for r in csv.DictReader(CSV.open())}
    if args.anchor not in rows:
        raise SystemExit(f"{args.anchor!r} is not in the table; try one of: {', '.join(sorted(rows))}")
    anchor = rows[args.anchor]

    def ratio(col):
        a = number(anchor[col])
        med = statistics.median(v for v in (number(r[col]) for r in rows.values()) if v is not None)
        if a is None:
            raise SystemExit(f"{args.anchor} has no `{col}` in the table")
        # Take `strength` of the difference, geometrically.
        return (a / med) ** args.strength

    base = base_body()
    g = ratio("gravity")
    jump = math.sqrt(g * ratio("full_hop"))
    air_jump = math.sqrt(g * ratio("air_jump"))
    traction = ratio("traction")
    body = {
        "max_run_speed": base["max_run_speed"] * ratio("run_speed"),
        "initial_dash_speed": base["initial_dash_speed"] * ratio("initial_dash"),
        "run_accel": base["run_accel"] * traction,
        "ground_friction": base["ground_friction"] * traction,
        "max_air_speed": base["max_air_speed"] * ratio("air_speed"),
        "air_accel": base["air_accel"] * ratio("air_accel_max"),
        "gravity": base["gravity"] * g,
        "max_fall_speed": base["max_fall_speed"] * ratio("fall_speed"),
        "fast_fall_speed": base["fast_fall_speed"] * ratio("fast_fall_speed"),
        "jump_speed": base["jump_speed"] * jump,
        "double_jump_speed": base["double_jump_speed"] * air_jump,
        "dodge_roll_speed": base["dodge_roll_speed"] * ratio("froll_distance"),
    }
    weight = (number(anchor["weight"]) / 100.0) ** args.strength
    strength = "" if args.strength == 1.0 else f" at {args.strength:.2f} strength"
    print(f"    // Shaped like Ultimate's {args.anchor}{strength}: each number is Smash's")
    print("    // default fighter's, scaled by how that fighter differs from Ultimate's")
    flag = f" --strength {args.strength}" if strength else ""
    print(f"    // roster median (`scripts/smash_fighter_shape.py \"{args.anchor}\"{flag}`).")
    print(f"    knockback_weight: Some({weight:.2f}),")
    print("    body: Some((")
    for name, value in body.items():
        print(f"        {name}: Some({value:.1f}),")
    print("    )),")


if __name__ == "__main__":
    main()
