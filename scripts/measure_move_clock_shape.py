#!/usr/bin/env python3
"""What SHAPE is an authored move's clock — startup, active, endlag?

⛔⛤ THE ONE AXIS THAT IS NOT SMASH-SHAPED IS **ACTIVE**, and this is how to see
it. A move that is generous in space AND long-lived in time AND short in total is
very safe to throw, and each of those three is authored in a different place, so
no single file shows the combination.

Reads a `moveset_export` bundle (`moveset_export --out bundle.json`), which
already derives `startup_f` / `active_f` / `endlag_f` per move at the sim's own
Hz. This adds no instrument; it groups what the bundle carries by MOVE CLASS and
prints the distribution.

    python3 scripts/measure_move_clock_shape.py bundle.json [--all] [--rows]

`--rows` prints every classified move instead of only the bands, for pasting into
a review; `--all` widens past the smash grid (see below).

⛤ THIS INSTRUMENT SEES MOVES `measure_authored_strike_extents.py --clock` CANNOT,
AND THAT IS THE REASON TO KEEP BOTH. That one reads
`assets/data/movesets/*.ron` — 17 files, 18 entities — while the smash grid seats
21: `mary_o_tall`, `player_robot_v3`, `sanic` and `smash_george_booul` have no RON
table at all, and two of them author moves in the long Active band
(`bubble_shield`, and `reductio`/`reductio_ad_absurdum` at 14.4f each). Measured
2026-09-12: across the 17 long moves BOTH instruments see they agree to 0.1f, and
nothing is seen only by the RON reader — so the disagreement is entirely scope,
and the roster-wide census is the one that is short.

⚠ THE REFERENCE BANDS ARE RESEARCH, NOT MEASUREMENT, and they are labelled as
such in the output. They are the ranges commonly published for Smash Ultimate and
they go stale; the MEASURED half is this repository's own numbers beside them.
Nothing here decides anything — a rebalance is Jon's call and wants these numbers
in front of it.

⛔ THE SMASH GRID ONLY, unless `--all`. A bundle carries 48 fighters and 21 are
seatable in a match; a median over the other 27 describes movesets nobody fights.
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

#: Move class -> the verbs that reach it, in the vocabulary the bundle writes.
#:
#: ⛔ KEYED ON THE VERB, NOT ON THE MOVE ID. Ids are per-character
#: (`performer_tilt_forward`, `run_up_kick`, `cipher_sweep`), and a census keyed
#: on a naming convention measures the convention.
CLASSES: dict[str, tuple[str, ...]] = {
    "jab": ("attack",),
    "tilt": ("attack_forward", "attack_up", "attack_down"),
    "smash": ("smash_forward", "smash_up", "smash_down"),
    "aerial": (
        "attack_air",
        "attack_air_forward",
        "attack_air_back",
        "attack_air_up",
        "attack_air_down",
        "air_forward",
        "air_back",
        "air_up",
        "air_down",
    ),
    "special": (
        "special",
        "special_forward",
        "special_up",
        "special_down",
        "special_air_down",
    ),
    # ⛔ GRAB AND DASH ARE CLASSES, AND LEAVING THEM OUT DROPPED FOUR OF THE
    # TWENTY AUTHORED VERBS SILENTLY. `class_of` returns None for an unmapped
    # verb and `rows` skips it without a word, so the omission read as "the
    # roster has no grab moves" rather than as "this map has no grab row".
    # Worse, `grab`, `grab_dash`, `attack_dash` and `special_air_down` are four
    # of the NINE verbs that contain no move at 10 active frames — the exact
    # population the census is about — so the gap fell on the evidence.
    "grab": ("grab", "grab_dash"),
    "dash": ("attack_dash",),
}

#: ⚠ RESEARCH, NOT MEASURED HERE. Commonly published Smash Ultimate ranges, as
#: (startup, active) frame bands. Printed for comparison and labelled every time.
REFERENCE: dict[str, tuple[tuple[int, int], tuple[int, int]]] = {
    "jab": ((2, 5), (2, 4)),
    "tilt": ((5, 12), (2, 5)),
    "smash": ((10, 20), (2, 5)),
    "aerial": ((5, 12), (2, 5)),
}


def class_of(verbs: list[str]) -> str | None:
    for name, members in CLASSES.items():
        if any(v in members for v in verbs):
            return name
    return None


def rows(bundle: dict, grid_only: bool = True) -> dict[str, list[dict]]:
    """Every move that has a live window, grouped by class."""
    out: dict[str, list[dict]] = {name: [] for name in CLASSES}
    for character in bundle.get("characters", []):
        if grid_only and not character.get("on_smash_grid"):
            continue
        for move in character.get("moves", []):
            name = class_of(move.get("verbs") or [])
            if name is None:
                continue
            derived = move.get("derived") or {}
            # ⛔ A MOVE WITH NO ACTIVE WINDOW IS NOT A SLOW MOVE, IT IS A
            # DIFFERENT KIND OF MOVE. A hitless special's "startup" is its whole
            # duration, and folding those into a startup median describes the
            # utility moves rather than the attacks.
            if not derived.get("active_f"):
                continue
            out[name].append(
                {
                    "character": character.get("id"),
                    "id": move.get("id"),
                    "startup_f": derived.get("startup_f"),
                    "active_f": derived["active_f"],
                    "endlag_f": derived.get("endlag_f") or 0.0,
                    "duration_f": move.get("duration_f"),
                }
            )
    return out


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__)
        return 2
    grid_only = "--all" not in sys.argv[1:]
    bundle = json.loads(Path(args[0]).read_text())
    grouped = rows(bundle, grid_only)
    total = sum(len(v) for v in grouped.values())
    # ⛔ THE FLOOR. An empty or renamed bundle groups nothing and every median is
    # absent, which prints as a tidy table of dashes rather than as "no data".
    if total < 20:
        raise SystemExit(
            f"only {total} classified move(s) with a live window across "
            f"{len(bundle.get('characters', []))} character(s). ⇒ REFUSING TO "
            "REPORT: a median over a handful describes whichever movesets "
            "happened to classify, not the roster. Check `verbs` in the bundle "
            "against this script's CLASSES map."
        )
    scope = "the smash grid" if grid_only else "every fighter in the bundle"
    print(
        f"{total} moves with a live window across {scope}, "
        f"at {bundle.get('sim_hz', 60.0):g} Hz\n"
    )
    print(f"{'class':<9}{'n':>4}  {'startup_f':>22}  {'active_f':>22}  {'endlag_f':>16}")
    for name, moves in grouped.items():
        if not moves:
            continue
        def band(key: str) -> str:
            values = [m[key] for m in moves if m[key] is not None]
            if not values:
                return "—"
            return (
                f"{min(values):5.1f} {statistics.median(values):5.1f} "
                f"{max(values):5.1f}"
            )
        print(
            f"{name:<9}{len(moves):>4}  {band('startup_f'):>22}  "
            f"{band('active_f'):>22}  {band('endlag_f'):>16}"
        )
    print("\n(min  median  max)\n")

    # ⛔⛤ THE DISTRIBUTION, NOT THE MEDIAN — a median inside 2-5 is compatible
    # with a third of the roster sitting at 10+, and item 5's claim is about
    # whether ANY meaningful population runs long. The bands are counted per
    # class so "jabs are fine and smashes are not" cannot hide inside a total.
    print(f"{'class':<9}{'n':>4}  {'2-5f':>9}  {'6-9f':>9}  {'10f+':>9}")
    for name, moves in grouped.items():
        if not moves:
            continue
        # ⛔ THE TOP BAND IS `>= 10`, NOT `> 9`, AND THE DIFFERENCE IS SIX MOVES.
        # A 9.6f move counted as "10f+" is simply mislabelled, and this roster
        # has six of them at exactly 9.6 — five of the performer's. `> 9` prints
        # 24 where the committed census says 18.
        lo = sum(1 for m in moves if m["active_f"] <= 5)
        mid = sum(1 for m in moves if 5 < m["active_f"] < 10)
        hi = sum(1 for m in moves if m["active_f"] >= 10)
        def cell(k: int) -> str:
            return f"{k:3d} {k / len(moves) * 100:4.0f}%"
        print(
            f"{name:<9}{len(moves):>4}  {cell(lo):>9}  {cell(mid):>9}  {cell(hi):>9}"
        )
    every = [m for moves in grouped.values() for m in moves]
    long_moves = sorted(
        (m for m in every if m["active_f"] >= 10),
        key=lambda m: m["active_f"],
        reverse=True,
    )
    print(
        f"\n{len(long_moves)} of {len(every)} moves run ACTIVE at 10 frames or"
        f" more ({len(long_moves) / len(every) * 100:.0f}%)."
    )
    # ⛔ NAMED, NOT COUNTED. A count that holds while its members change has
    # burned this repository before; the row is what a reader can check.
    if long_moves:
        print("\nevery move past 9 active frames, named:")
        print(f"  {'character':<33}{'move':<37}{'act':>6}{'start':>7}{'end':>7}")
        for m in long_moves:
            print(
                f"  {str(m['character']):<33}{str(m['id']):<37}"
                f"{m['active_f']:>6.1f}{(m['startup_f'] or 0.0):>7.1f}"
                f"{m['endlag_f']:>7.1f}"
            )
    if "--rows" in sys.argv[1:]:
        print("\nevery classified move (frames):")
        print(
            f"  {'class':<9}{'character':<33}{'move':<37}"
            f"{'start':>7}{'act':>6}{'end':>7}{'total':>7}"
        )
        for name, moves in grouped.items():
            for m in sorted(moves, key=lambda m: (-m["active_f"], str(m["id"]))):
                total = m["duration_f"]
                print(
                    f"  {name:<9}{str(m['character']):<33}{str(m['id']):<37}"
                    f"{(m['startup_f'] or 0.0):>7.1f}{m['active_f']:>6.1f}"
                    f"{m['endlag_f']:>7.1f}"
                    f"{(total if total is not None else 0.0):>7.1f}"
                )
    print()
    print("⚠ REFERENCE BANDS BELOW ARE RESEARCH, NOT MEASURED HERE — commonly")
    print("  published Smash Ultimate ranges, which go stale. The numbers above")
    print("  are this repository's own.\n")
    for name, ((s_lo, s_hi), (a_lo, a_hi)) in REFERENCE.items():
        moves = grouped.get(name) or []
        if not moves:
            continue
        actives = [m["active_f"] for m in moves]
        med = statistics.median(actives)
        verdict = "inside" if a_lo <= med <= a_hi else f"{med / a_hi:.1f}x the top"
        print(
            f"  {name:<8} reference startup {s_lo}-{s_hi}f, active {a_lo}-{a_hi}f"
            f"   -> our active median {med:.1f}f, {verdict}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
