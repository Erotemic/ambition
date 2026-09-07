#!/usr/bin/env python3
"""Which authored placements actually persist their death?

⭐ THE QUESTION THIS EXISTS TO ANSWER. `sync_ecs_actors_with_save` zeroes a body's
health on load when the save carries `enemy_<id>_dead` (`DeadStaysDead`) or
`enemy_<id>_dead_until_rest` (`OnRest`). Everything else writes NO flag and reads
none. So "a persisted enemy death survives a room reload" is only a claim about
placements that AUTHOR one of those two policies -- and a test that picks a room
without one is not measuring the mirror, it is measuring a body that was never
persistent.

⛔⛔ AND THE ENUM'S `#[default]` IS NOT THE PLACEMENT DEFAULT. `RespawnPolicy`
derives `Default = DeadStaysDead`, and its own doc says so -- but a placement that
authors nothing takes `UNDESCRIBED_BODY_RESPAWN`, which is `OnRoomReenter`
(`actor_spawn/mod.rs`). Reading the enum and concluding "unspecified placements
persist" is the error this sweep exists to make un-makeable: it counts the
AUTHORED field, in the shipped worlds, and says which rooms hold one.

⚠ It reads the LDtk exports under `game/**/assets/worlds` and
`game/ambition_map_assets/**`, which is where the shipped worlds live. Duplicate
copies of the same world (the web bundle, the map-assets mirror) are reported
per FILE on purpose: a room that persists in one copy and not another is a real
divergence, and collapsing them would hide it.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]

# The two policies that make a death reach the save. `OnRoomReenter` and
# `InPlace` deliberately write nothing.
PERSISTING = ("DeadStaysDead", "OnRest")


def worlds() -> list[Path]:
    return sorted(p for p in (REPO / "game").rglob("*.ldtk"))


def placements(path: Path):
    """Yield (room, entity identifier, policy) for every authored respawn field."""
    data = json.loads(path.read_text(errors="replace"))
    for level in data.get("levels", []):
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances", []):
                for field in entity.get("fieldInstances", []):
                    if field.get("__identifier") != "respawn":
                        continue
                    value = field.get("__value")
                    if value:
                        yield level.get("identifier", "<unnamed>"), entity.get(
                            "__identifier", "<unnamed>"
                        ), value


def main() -> int:
    files = worlds()
    if not files:
        print(
            "FAIL: no .ldtk worlds found under game/ — the sweep is broken, not "
            "the tree.",
            file=sys.stderr,
        )
        return 1

    authored = 0
    by_room: dict[tuple[str, str], list[str]] = defaultdict(list)
    for path in files:
        for room, entity, policy in placements(path):
            authored += 1
            if policy in PERSISTING:
                by_room[(str(path.relative_to(REPO)), room)].append(f"{entity}:{policy}")

    print(f"shipped .ldtk worlds: {len(files)}")
    print(f"placements authoring a `respawn` field: {authored}")
    print(f"  of those, PERSISTING ({' or '.join(PERSISTING)}):")
    if not by_room:
        print("    none — no room in the shipped world can witness a persisted death")
    for (path, room), rows in sorted(by_room.items()):
        print(f"    {path}  room={room}  {len(rows)}  {', '.join(sorted(rows))}")

    # ⚠ ANTI-VACUITY: a world file the parser silently failed to walk would
    # report zero authored fields and every conclusion below would be about
    # nothing. The shipped tree authors the field in several worlds; zero means
    # the reader broke.
    if authored == 0:
        print(
            "\nFAIL: not one placement in any shipped world authors a `respawn` "
            "field.\n  The field exists in the LDtk entity contract, so this is a "
            "broken reader,\n  not an unauthored tree.",
            file=sys.stderr,
        )
        return 1

    spellings = {p for rows in by_room.values() for p in (r.split(":")[1] for r in rows)}
    print(
        f"\n⇒ FLAG SPELLINGS a test in these rooms must use: "
        f"{sorted(spellings) if spellings else 'n/a'}"
    )
    print(
        "  `DeadStaysDead` -> `enemy_<id>_dead`; `OnRest` -> "
        "`enemy_<id>_dead_until_rest`.\n"
        "  ⚠ They are different flags. A test written against the wrong one fails "
        "for a\n  reason that has nothing to do with the save mirror."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
