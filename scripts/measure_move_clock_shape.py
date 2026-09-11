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

    python3 scripts/measure_move_clock_shape.py bundle.json

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
    "special": ("special", "special_forward", "special_up", "special_down"),
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
