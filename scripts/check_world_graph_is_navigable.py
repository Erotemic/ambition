#!/usr/bin/env python3
"""Every authored loading zone leads somewhere, and no area is a one-way trap.

The room graph is built from `LoadingZone` entities: each one with a
`target_room` + `target_zone` becomes ONE DIRECTED edge, plus the reverse when
it authors `bidirectional`. `RoomSet::from_parts` drops an edge whose target
names no room and prints `room graph warning: unknown target room '<id>'` to
stderr -- a warning nobody reads during a build, on a door that then goes
nowhere.

MEASURED 2026-09-05 across the four shipped worlds: 72 areas, 150 directed
edges, 122 zones authoring `bidirectional`, zero dangling targets and zero
trapped areas. This check exists to keep that true.

⛔⛔ A ROOM IS AN `activeArea`, NOT A LEVEL, and reading the wrong key is how I
produced two confident false findings before this script existed. `LdtkLevel::
active_area` reads the level field **`activeArea`** (camelCase) and falls back to
the level identifier when it is absent or blank. Keying on the level identifier
invents areas that do not exist and reports their cross-area doors as dangling;
keying on `active_area` (snake_case) matches NOTHING and silently falls back to
identifiers everywhere, which looks identical. ⇒ The key is asserted below rather
than assumed.

⚠ FAILS ONLY ON A PRESENCE. Worlds are SYMLINKS into `game/ambition_map_assets`;
a checkout without that submodule has no worlds to judge, and this exits 3
("cannot check") rather than passing. A world that is present and broken is the
only thing that reddens it.

Exit 0 navigable, 1 a dangling target or a trapped area, 3 no worlds readable."""

from __future__ import annotations

import collections
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
WORLDS = REPO / "game/ambition_content/assets/worlds"

#: The level field naming the composed area a level belongs to.
#: ⛔ camelCase, and asserted against the source of truth below.
AREA_FIELD = "activeArea"


#: Where the Rust reads it, so the claim above can be CHECKED rather than trusted.
AREA_FIELD_SOURCE = REPO / "crates/ambition_platformer2d_ldtk/src/project.rs"


def assert_area_field_matches_the_engine() -> None:
    """The engine and this script must name the SAME level field.

    ⛔⛔ WITHOUT THIS, THE DOC ABOVE IS EXACTLY THE KIND OF SENTENCE THIS REPO
    KEEPS CATCHING: a comment asserting a fact with nothing checking it. If
    `raw_active_area` is ever repointed, this script keeps reading `activeArea`,
    finds nothing, falls back to level identifiers for every level, and reports
    confident nonsense -- which is precisely the failure it was written after.
    """
    try:
        source = AREA_FIELD_SOURCE.read_text(encoding="utf-8")
    except OSError:
        return  # the crate is not here; the caller's exit-3 path covers it
    if f'field_string("{AREA_FIELD}")' not in source:
        raise SystemExit(
            f'⛔ {AREA_FIELD_SOURCE.relative_to(REPO)} no longer reads '
            f'`field_string("{AREA_FIELD}")`. This script would silently fall '
            "back to level identifiers and invent areas. Re-derive the key from "
            "`LdtkLevel::raw_active_area` and update AREA_FIELD."
        )


def area_of(level: dict) -> str:
    """The room a level belongs to — `LdtkLevel::active_area`'s rule, exactly."""
    for field in level.get("fieldInstances") or []:
        if field.get("__identifier") == AREA_FIELD:
            value = field.get("__value")
            if isinstance(value, str) and value.strip():
                return value.strip()
    return level.get("identifier") or "<unnamed level>"


def main() -> int:
    assert_area_field_matches_the_engine()
    worlds = sorted(WORLDS.glob("*.ldtk")) if WORLDS.is_dir() else []
    readable = []
    for world in worlds:
        try:
            readable.append((world, json.loads(world.read_text(encoding="utf-8", errors="replace"))))
        except (OSError, ValueError) as error:
            # ⚠ REPORTED, never skipped: a scanner that swallows a read error
            # reports its own failure as a clean result.
            print(f"⛔ {world.name} did not read as LDtk JSON: {error}", file=sys.stderr)
            return 1
    if not readable:
        print(
            "cannot check: no worlds under "
            f"{WORLDS.relative_to(REPO)} (they are symlinks into the "
            "`game/ambition_map_assets` submodule).\n"
            "  This is NOT a pass -- no door was examined.\n"
            "  fix: git submodule update --init game/ambition_map_assets",
            file=sys.stderr,
        )
        return 3

    areas: set[str] = set()
    for _, document in readable:
        for level in document.get("levels") or []:
            areas.add(area_of(level))

    edges: set[tuple[str, str]] = set()
    zones = both_ways = 0
    for _, document in readable:
        for level in document.get("levels") or []:
            room = area_of(level)
            for layer in level.get("layerInstances") or []:
                for entity in layer.get("entityInstances") or []:
                    if entity.get("__identifier") != "LoadingZone":
                        continue
                    fields = {
                        f.get("__identifier"): f.get("__value")
                        for f in entity.get("fieldInstances") or []
                    }
                    target = fields.get("target_room")
                    if not target or not fields.get("target_zone"):
                        continue
                    zones += 1
                    edges.add((room, target))
                    if fields.get("bidirectional") is True:
                        both_ways += 1
                        edges.add((target, room))

    # ⛔ ANTI-VACUITY: a corpus that parsed but yielded no doors would print a
    # clean bill for a world with nothing in it.
    if not edges:
        print("⛔ no loading zones with a target were found; the check is vacuous", file=sys.stderr)
        return 1

    outgoing: dict[str, set[str]] = collections.defaultdict(set)
    for source, target in edges:
        outgoing[source].add(target)
    entered = {target for _, target in edges}

    dangling = sorted(t for t in entered if t not in areas)
    trapped = sorted(r for r in entered if r in areas and not outgoing.get(r))

    print(
        f"world graph: {len(areas)} areas, {len(edges)} directed edges from "
        f"{zones} zones ({both_ways} authoring `bidirectional`)"
    )
    if dangling:
        print(
            "FAIL: loading zones target rooms that are not an area in any world: "
            + ", ".join(dangling)
            + "\n  `RoomSet::from_parts` DROPS these edges with an stderr warning, "
            "so the door simply does nothing.",
            file=sys.stderr,
        )
    if trapped:
        print(
            "FAIL: areas you can enter and not leave: "
            + ", ".join(trapped)
            + "\n  Every door out of them is one-way. Author the return zone, or "
            "`bidirectional` on the way in.",
            file=sys.stderr,
        )
    if dangling or trapped:
        return 1
    print("ok: every authored door leads to a real area, and no area is a trap")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
