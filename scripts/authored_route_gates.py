#!/usr/bin/env python3
"""Every authored use of the condition catalog: route gates AND dialogue lines.

⛔⛔ THE POINT IS THE DENOMINATOR. `capability-progression-and-world-gating.md`
records that five of seven gate families are now reachable from a route
(`world.flag_set`, `inventory.holds`, `held.is_held`, `body.can`, `body.fits`,
`world.switch_on`), and that no shipped level authors any of the new ones. That
is easy to read as "a migration is owed". It is not: the corpus of authored
route gates is TINY, and until you have counted it you cannot tell a vocabulary
that is unused from a vocabulary that has nothing to be used on.

⭐ A `LockWall` with no `gated_by` is not a defect — it belongs to the encounter
lock, which is a different writer (`contribute_encounter_lock_walls`). Both
classes are printed so the split is visible rather than assumed.

⛔⛔ TWO CONSUMERS, NOT ONE, AND THE SECOND IS THE BUSIER. This script counted
only walls on 2026-09-04 and reported a corpus of three. `ConditionCatalog` has a
second authored road: `ambition_conversation/src/dialog/authored_conditions.rs`
installs a Yarn verb `condition(id, arg)`, so every `.yarn` line calling it is
an authored use of the same vocabulary. Counting one consumer and calling the
result "the authored corpus" is the denominator error this script exists to
prevent, committed inside the instrument itself.

Usage:  python3 scripts/authored_route_gates.py
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from collections import Counter


def worlds() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "*.ldtk"], capture_output=True, text=True, check=True
    ).stdout.split()
    return sorted(out)


def main() -> int:
    rows: list[tuple[str, str, str | None, str | None]] = []
    unreadable: list[str] = []
    for path in worlds():
        try:
            data = json.load(open(path, encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            # ⛔ NOT silently skipped: this script's whole output is a COUNT, and
            # a dropped file lowers it without saying so.
            unreadable.append(f"{path}: {error}")
            continue
        for level in data.get("levels", []):
            for layer in level.get("layerInstances", []) or []:
                for entity in layer.get("entityInstances", []) or []:
                    if entity.get("__identifier") != "LockWall":
                        continue
                    fields = {
                        f["__identifier"]: f.get("__value")
                        for f in entity.get("fieldInstances", [])
                    }
                    rows.append(
                        (path, level.get("identifier", "?"), fields.get("id"), fields.get("gated_by"))
                    )

    gated = [r for r in rows if r[3]]
    ungated = [r for r in rows if not r[3]]

    print("AUTHORED ROUTE GATES\n")
    for path, level, wall_id, gate in rows:
        kind = f"gated_by={gate!r}" if gate else "no gated_by (encounter lock)"
        print(f"  {path}\n     level={level!r} id={wall_id!r} {kind}")

    print(f"\nworlds scanned: {len(worlds())}")
    print(f"LockWall instances: {len(rows)}  ({len(gated)} gated, {len(ungated)} encounter)")

    conditions = Counter(
        (g[3].split()[0] if "." in g[3].split()[0] else "world.flag_set") for g in gated
    )
    print("\nconditions actually authored:")
    for name, count in sorted(conditions.items()):
        print(f"  {count:>3}  {name}")

    # ── the second road: authored dialogue ──────────────────────────────────
    yarn = subprocess.run(
        ["git", "ls-files", "*.yarn"], capture_output=True, text=True, check=True
    ).stdout.split()
    calls: list[tuple[str, str]] = []
    for path in sorted(yarn):
        try:
            text = open(path, encoding="utf-8").read()
        except OSError as error:
            unreadable.append(f"{path}: {error}")
            continue
        for match in re.finditer(r'condition\(\s*"([^"]+)"', text):
            calls.append((path, match.group(1)))

    print(f"\ndialogue files: {len(yarn)}  (using condition(): "
          f"{len({p for p, _ in calls})})")
    print(f"condition() calls: {len(calls)}")
    by_id = Counter(cid for _, cid in calls)
    for name, count in sorted(by_id.items()):
        print(f"  {count:>3}  {name}")

    print(
        f"\nTOTAL authored uses of the condition vocabulary: {len(gated) + len(calls)}"
        f"  ({len(gated)} route gates + {len(calls)} dialogue lines)"
    )
    published = [
        "world.flag_set",
        "world.switch_on",
        "inventory.holds",
        "held.is_held",
        "body.can",
        "body.fits",
        "encounter.cleared",
    ]
    used = set(conditions) | set(by_id)
    unused = [c for c in published if c not in used]
    if unused:
        print(f"published but authored NOWHERE ({len(unused)} of {len(published)}):")
        for name in unused:
            print(f"       {name}")

    if unreadable:
        print(
            f"\n⛔ {len(unreadable)} world(s) could not be READ, so the counts above "
            "are incomplete and describe only what answered:"
        )
        for line in unreadable:
            print(f"  {line}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
