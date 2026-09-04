#!/usr/bin/env python3
"""Every authored route gate in every shipped world, and the condition it asks.

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

Usage:  python3 scripts/authored_route_gates.py
"""

from __future__ import annotations

import json
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
