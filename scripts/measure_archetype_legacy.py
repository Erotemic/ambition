#!/usr/bin/env python3
"""Measure how much of the retired enemy-archetype implementation remains.

The report tracks the named legacy components using production-code lines rather
than test fixtures. It distinguishes physical lines from code lines so large
comments do not masquerade as remaining model implementation. The component
list and historical baselines below define the migration surface being measured."""

from __future__ import annotations

import glob
import os
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

COMPONENTS = [
    ("ArchetypeSpec", "crates/ambition_combat/src/archetype_spec.rs", 319),
    (
        "features/enemies + CharacterRoster",
        "crates/ambition_platformer2d_actor_monolith/src/features/enemies/*.rs",
        1198,
    ),
    ("character_archetypes.ron", "game/ambition_content/assets/data/character_archetypes.ron", 843),
    ("enemy_roster.rs", "game/ambition_content/src/enemy_roster.rs", 75),
    (
        "ActorTuning",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/actor_tuning.rs",
        275,
    ),
    (
        "autonomous_reconcile",
        "crates/ambition_platformer2d_actor_monolith/src/features/ecs/autonomous_reconcile.rs",
        1045,
    ),
]


def classify(path: str) -> tuple[int, int, int]:
    """`(code, comment, test)` line counts for one file."""
    lines = open(path, encoding="utf-8").read().split("\n")
    if path.endswith(".ron"):
        code = sum(1 for line in lines if line.strip() and not line.strip().startswith("//"))
        comment = sum(1 for line in lines if line.strip().startswith("//"))
        return code, comment, 0

    test: set[int] = set()
    if os.path.basename(path).endswith("_tests.rs"):
        test = set(range(len(lines)))
    else:
        i = 0
        while i < len(lines):
            if lines[i].strip().startswith("#[cfg(test)]"):
                j = i + 1
                while j < len(lines) and lines[j].strip().startswith("#["):
                    j += 1
                if j >= len(lines):
                    break
                if "{" in lines[j]:
                    depth = 0
                    k = j
                    while k < len(lines):
                        depth += lines[k].count("{") - lines[k].count("}")
                        if depth == 0 and k >= j:
                            break
                        k += 1
                    test.update(range(i, k + 1))
                    i = k + 1
                    continue
                test.update(range(i, j + 1))
                i = j + 1
                continue
            i += 1

    code = comment = 0
    for n, line in enumerate(lines):
        if n in test:
            continue
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("//"):
            comment += 1
        else:
            code += 1
    return code, comment, len(test)


def main() -> int:
    print(f"{'component':38s} {'baseline':>8s} {'code':>6s} {'+cmnt':>7s} {'test':>6s}")
    total_base = total_code = total_prod = 0
    for label, pattern, baseline in COMPONENTS:
        paths = sorted(glob.glob(os.path.join(REPO, pattern)))
        if not paths:
            print(f"{label:38s} {baseline:8d}   GONE — component deleted")
            total_base += baseline
            continue
        code = comment = test = 0
        for path in paths:
            c, m, t = classify(path)
            code += c
            comment += m
            test += t
        print(f"{label:38s} {baseline:8d} {code:6d} {code + comment:7d} {test:6d}")
        total_base += baseline
        total_code += code
        total_prod += code + comment
    print(f"{'TOTAL':38s} {total_base:8d} {total_code:6d} {total_prod:7d}")
    print()
    print(f"production lines (comments included): {total_prod}  ({total_prod - total_base:+d})")
    print(f"CODE lines (comments excluded):       {total_code}")
    print()
    print("⚠ the baseline's own unit is unrecorded — almost certainly `wc -l` at the")
    print("  time, which is the middle column plus whatever tests those files then")
    print("  held. Compare the trend, not the difference.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
