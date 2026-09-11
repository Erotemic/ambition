#!/usr/bin/env python3
"""How big is a fighter's strike volume against its own body?

⛔⛤ THE NUMBER THAT MATTERS IS A RATIO, NOT AN AREA. A 3.8 px-tall box is only
"stingy" against the 48 px body it belongs to, and the roster's bodies differ by
more than 3x — so an absolute area ranks the big characters first and says
nothing about how any of them FEELS.

Reads a `moveset_takes` recording. The take already carries, per frame, every
live hitbox's half-extents and every body's, so this adds no instrument: it is
arithmetic over a recording somebody already has to make.

    moveset_takes --characters grid --verbs attack_forward --out takes.json
    python3 scripts/measure_strike_area_over_body.py takes.json

⚠ THE PEAK, NOT THE MEAN. A move whose box is live for two frames and enormous
reads as small under a mean over the take, and what a player feels is whether the
box that IS live can reach them. The peak frame is the one the ratio is about.

⛔ SUBJECT-OWNED ONLY. A take records the target's boxes too, and counting those
measures the sandbag.

⛔⛤ IT IS THE BOUNDS, NOT THE SHAPE, AND THE DIFFERENCE IS NOT SMALL FOR A BLADE.
`volume_json` writes `half` from `volume.bounds().half_size()`, so a convex
sword-arc poly is measured by the rectangle around it. The ratio therefore
OVERSTATES a bladed move and is exact for a rect. ⇒ Read it as "how much of the
body's own area could this swing possibly cover", which is the right question for
"does it feel like it connects" and the wrong one for a damage budget.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path


def peaks(take: dict) -> dict[str, tuple[float, tuple[float, float], tuple[float, float]]]:
    """Per character: the peak ratio and the FULL EXTENTS it was read from.

    ⭐ THE EXTENTS TRAVEL WITH THE RATIO. "0.09" is unactionable to whoever has
    to author the repair; "14.2 x 3.8 px against a 30 x 48 body" names the axis
    that is thin. A row quoting extents beside a ratio it did not derive them
    from is two measurements pretending to be one.
    """
    out: dict[str, tuple[float, tuple[float, float], tuple[float, float]]] = {}
    for row in take.get("takes", []):
        character = row.get("character")
        if character is None:
            continue
        best = 0.0
        best_strike = (0.0, 0.0)
        best_body = (0.0, 0.0)
        for frame in row.get("frames", []):
            subject = next(
                (
                    b
                    for b in frame.get("bodies", [])
                    if b.get("role") == "subject"
                ),
                None,
            )
            if subject is None:
                continue
            bh = subject.get("half")
            if not bh or bh[0] <= 0 or bh[1] <= 0:
                continue
            body_area = 4.0 * bh[0] * bh[1]
            for hit in frame.get("hitboxes", []):
                if not hit.get("subject_owned"):
                    continue
                hh = hit.get("half")
                if not hh:
                    continue
                ratio = 4.0 * hh[0] * hh[1] / body_area
                if ratio > best:
                    best = ratio
                    best_strike = (2.0 * hh[0], 2.0 * hh[1])
                    best_body = (2.0 * bh[0], 2.0 * bh[1])
        # ⛔ A CHARACTER WITH NO LIVE BOX IS 0.0 AND IS REPORTED, not dropped: a
        # missing row reads as "not measured" and a dropped one reads as nothing
        # at all, and "this verb produced no hitbox" is the louder finding of the
        # two this file can make.
        out[character] = (best, best_strike, best_body)
    return out


def ratios(take: dict) -> dict[str, float]:
    """Peak (strike area / body area) per character — the ratio alone."""
    return {name: row[0] for name, row in peaks(take).items()}


def main() -> int:
    args = [a for a in sys.argv[1:] if a != "--detail"]
    detail_flag = "--detail" in sys.argv[1:]
    if len(args) != 1:
        print(__doc__)
        return 2
    sys.argv = [sys.argv[0], args[0]]
    path = Path(sys.argv[1])
    take = json.loads(path.read_text())
    detail = detail_flag
    rows_full = peaks(take)
    rows = {name: row[0] for name, row in rows_full.items()}
    if not rows:
        raise SystemExit(
            f"{path} holds no takes with a subject body. ⇒ REFUSING TO REPORT: "
            "an empty recording gives every ratio 0.0, which reads as 'every "
            "fighter swings at nothing' rather than 'nothing was recorded'."
        )
    # ⛔⛔ AND "NO TAKES" IS THE EASY HALF. A file WITH takes and no hitboxes in
    # any of them gives every character 0.00 and prints a table that reads as a
    # devastating finding about the roster. A total collapse to zero across every
    # row is the empty-corpus tell, and the one reader who cannot apply that
    # judgement is this script. ⚠ ONE zero among many is a real finding and is
    # printed — it is the ALL-zero case that cannot be distinguished from a
    # recording that never reached a move.
    if not any(rows.values()):
        raise SystemExit(
            f"{path} holds {len(rows)} take(s) and NOT ONE live subject-owned "
            "hitbox. ⇒ REFUSING TO REPORT: every ratio would be 0.00, which reads "
            "as 'the whole roster swings at nothing' rather than 'this recording "
            "never reached a move'. Check the takes' own `outcome` and "
            "`max_live_hitboxes` first."
        )
    width = max(len(name) for name in rows)
    print(f"{len(rows)} characters, peak strike area over own body area\n")
    for name, value in sorted(rows.items(), key=lambda kv: kv[1]):
        mark = "  <- under its own body" if value < 1.0 else ""
        if detail:
            _, (sw, sh), (bw, bh) = rows_full[name]
            mark = f"  {sw:6.1f} x {sh:5.1f} px against a {bw:.0f} x {bh:.0f} body{mark}"
        print(f"  {name:<{width}}  {value:5.2f}{mark}")
    under = sum(1 for v in rows.values() if v < 1.0)
    print(f"\n{under} of {len(rows)} swing a box smaller than the body that swings it.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
