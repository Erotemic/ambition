#!/usr/bin/env python3
"""The AUTHORED move clock, in 60fps-equivalent frames, across every shipped moveset.

⛔⛤ THE QUESTION IS THE MOVESET LANE'S ITEM 5, WHICH ASKS FOR NUMBERS BEFORE A
REBALANCE: *"startup is right (tilts ~5 frames, smashes ~12) but ACTIVE runs
10-17 frames against Ultimate's usual 2-5, and totals are SHORTER than
Ultimate's."* That was written from ONE character's kit. This reads every baked
moveset and reports the distribution, which is what turns an impression into a
decision Jon can make.

⚠ IT READS THE AUTHORED WINDOWS, NOT A RECORDING. A window is authored in SECONDS
(`start_s`/`end_s`); frames are `seconds * 60` — a presentation of the same
number, not a second measurement. ⇒ It needs no engine, no take and no build.

⚠ ACTIVE IS THE UNION OF `Active` WINDOWS, NOT THEIR SUM. A move with two
overlapping active windows is live once over that span; summing would report a
multi-hit move as twice as long as it is live.

⛔⛔ IT KEYS BY `(entity, move)` AND ASSERTS THE KEY IS 1:1, and that assertion is
load-bearing rather than decorative. `cellular_automaton.ron` publishes TWO
entities — `perfect_cellular_automaton` and `imperfect_cellular_automaton` — so a
key of `(file, move)` reports 20 moves twice and every quantile shifts. The first
version of this script keyed by file, and the repeat looked exactly like a parser
bug rather than like a second character.

Usage:
    scripts/measure_authored_move_clock.py            # distribution + the longest-live
    scripts/measure_authored_move_clock.py --top 20
"""

from __future__ import annotations

import argparse
import glob
import re
import sys

ENTITY = re.compile(r'^            id: "([a-z_0-9]+)",', re.M)
MOVE_ID = re.compile(r'id:\s*"([a-z_0-9]+)"')
WINDOW = re.compile(r"start_s:\s*([0-9.]+),\s*\n\s*end_s:\s*([0-9.]+),\s*\n\s*tag:\s*(\w+)")
MOVESETS = "game/ambition_content/assets/data/movesets/*.ron"
FPS = 60.0
# Ultimate's usual ACTIVE window, the comparison item 5 names.
ULTIMATE_ACTIVE_MAX = 5.0


def union(spans: list[tuple[float, float]]) -> float:
    """Total time covered, counting overlap once."""
    spans = sorted(spans)
    total, cur_s, cur_e = 0.0, None, None
    for s, e in spans:
        if cur_e is None or s > cur_e:
            if cur_e is not None:
                total += cur_e - cur_s
            cur_s, cur_e = s, e
        else:
            cur_e = max(cur_e, e)
    return total + (cur_e - cur_s if cur_e is not None else 0.0)


def rows() -> list[tuple[str, str, float, float, float]]:
    found: list[tuple[str, str, float, float, float]] = []
    seen: set[tuple[str, str]] = set()
    for path in sorted(glob.glob(MOVESETS)):
        text = open(path, errors="ignore").read()
        entities = [(m.group(1), m.start()) for m in ENTITY.finditer(text)]
        for k, (entity, start) in enumerate(entities):
            end = entities[k + 1][1] if k + 1 < len(entities) else len(text)
            parts = MOVE_ID.split(text[start:end])
            for i in range(1, len(parts) - 1, 2):
                move, body = parts[i], parts[i + 1]
                windows = [(float(a), float(b), tag) for a, b, tag in WINDOW.findall(body)]
                active = [(a, b) for a, b, tag in windows if tag == "Active"]
                if not active:
                    continue
                key = (entity, move)
                # ⛔ THE PREMISE, ASSERTED. See the module doc.
                assert key not in seen, f"key is not 1:1: {key} appears twice"
                seen.add(key)
                found.append(
                    (
                        entity,
                        move,
                        min(a for a, _ in active) * FPS,
                        union(active) * FPS,
                        max(b for _, b, _ in windows) * FPS,
                    )
                )
    return found


def dist(values: list[float], label: str, ultimate: str) -> None:
    values = sorted(values)
    n = len(values)
    q = lambda p: values[min(n - 1, int(p * n))]  # noqa: E731
    print(
        f"  {label:<8} min {values[0]:5.1f}  p25 {q(.25):5.1f}  median {q(.5):5.1f}"
        f"  p75 {q(.75):5.1f}  max {values[-1]:5.1f}   (Ultimate: {ultimate})"
    )


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--top", type=int, default=12)
    args = ap.parse_args(argv)

    found = rows()
    if not found:
        # ⛔ AN EMPTY CORPUS PRINTS `ok` UNLESS IT REFUSES.
        print(f"NO MOVES PARSED from {MOVESETS} — that is the instrument, not the content")
        return 1
    entities = {r[0] for r in found}
    print(
        f"{len(found)} moves with an Active window, across {len(entities)} entities "
        f"in {len(glob.glob(MOVESETS))} shipped moveset file(s)\n"
    )
    dist([r[2] for r in found], "startup", "tilts ~5, smashes ~12")
    dist([r[3] for r in found], "ACTIVE", "usually 2-5")
    dist([r[4] for r in found], "total", "—")
    over = [r for r in found if r[3] > ULTIMATE_ACTIVE_MAX]
    over10 = [r for r in found if r[3] > 2 * ULTIMATE_ACTIVE_MAX]
    print(
        f"\n  live > {ULTIMATE_ACTIVE_MAX:.0f} frames: {len(over)} of {len(found)} "
        f"({100 * len(over) // len(found)}%)"
    )
    print(
        f"  live > {2 * ULTIMATE_ACTIVE_MAX:.0f} frames: {len(over10)} of {len(found)} "
        f"({100 * len(over10) // len(found)}%)\n"
    )
    print(f"  the {args.top} longest-live:")
    for entity, move, startup, active, total in sorted(found, key=lambda r: -r[3])[: args.top]:
        print(
            f"    {entity:<30} {move:<32} active {active:5.1f}f  "
            f"startup {startup:4.1f}f  total {total:5.1f}f"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
