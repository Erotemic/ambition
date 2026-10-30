#!/usr/bin/env python3
"""Add scenery to rooms whose LDtk level is edited in place.

    python specs/scenery_overlay.py           # prints what it would add
    python specs/scenery_overlay.py --apply   # adds what is missing

Run it from `tools/ambition_ldtk_tools`.

Some rooms' generators are older than their levels: the level was edited in
the LDtk after (gate terminals, water, enemies tuned), so regenerating the
room would undo that (2026-10-30: `specs/primeval/make_specs.py --apply`
moved and dropped enemies, a gate terminal and the water). Their scenery is
ADDED instead: one `Prop` per piece of art, over the art's body box, and
signs and residents, each added once (by its name).
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from district_kit import WORLDS, add_entities  # noqa: E402
from district_scenery import body_box  # noqa: E402

GENERATOR = "specs/scenery_overlay.py"

#: World file -> level -> [(kind, frame x, frame y)], furthest back first. The
#: numbers are the art's (its `PIECES` frames); change them with it.
OVERLAYS = {
    "dinosaur.ldtk": {
        "primeval_tyrant_hall": [("primeval_tyrant_hall_room", 0, 0)],
        # Along the trail (`targets/props/jungle_props.py`): each piece stands
        # on the floor at the foot of its frame (y 480; the canopy's 672).
        "primeval_trailhead": [("jungle_fern_clump", 700, 360), ("jungle_fallen_log", 900, 420), ("jungle_ranger_hut", 470, 330)],
        "primeval_run": [("jungle_idol_head", 600, 370), ("jungle_fern_clump", 1500, 360), ("jungle_ribcage", 2350, 360)],
        "primeval_rookery": [("jungle_nest", 980, 410), ("jungle_nest", 1320, 410), ("jungle_fern_clump", 2100, 360), ("jungle_fallen_log", 2300, 420)],
        "primeval_ridge": [("jungle_ribcage", 600, 360), ("jungle_idol_head", 2200, 370), ("jungle_fern_clump", 2650, 360)],
        "primeval_canopy": [("jungle_fallen_log", 700, 612), ("jungle_fern_clump", 450, 552), ("jungle_nest", 1500, 602)],
    },
    "pvn.ldtk": {
        "ninja_dojo": [("shadow_dojo_hall", 0, 0)],
        "pirate_sky_lookout": [("sky_lookout_deck", 0, 190)],
    },
    "evil.ldtk": {
        "evil_gate_forest": [("evil_forest_roadside", 430, 260)],
    },
    "science.ldtk": {
        # One drawing in two frames (`targets/props/orbital_spur_props.py`).
        "orbital_station_damaged_spur": [("spur_west", 16, 64), ("spur_east", 1280, 64)],
    },
}


#: World file -> level -> [(name, x, y, text, width)]: lines of writing
#: (`DebugLabel`) with their middle over x, each added once (by its name).
SIGNS = {
    "pvn.ldtk": {
        "pirate_sky_lookout": [
            ("sky_lookout_sign_parking", 830, 640, "SHARK PARKING: extinguish before boarding", 420),
            ("sky_lookout_sign_nest", 1056, 300, "CROW'S NEST: no crows. Some parrots.", 380),
            ("sky_lookout_sign_perch", 210, 390, "THE CAPTAIN'S PERCH (the captain is out)", 400),
            ("sky_lookout_sign_cannon", 1100, 590, "ANTI-SHARK GUN: aim up, mostly", 340),
        ],
    },
}


#: World file -> level -> [(character, dialogue node, x of the middle,
#: floor)]: residents who stand and talk (`NpcSpawn`), each added once (by
#: its dialogue node).
RESIDENTS = {
    "dinosaur.ldtk": {
        "primeval_trailhead": [("npc_primeval_brandvold", "primeval_brandvold", 660, 480)],
    },
}


def resident_yaml(level_id: str, residents: list) -> str:
    rows = [f"""  - type: NpcSpawn
    px: [{x - 16}, {floor - 64}]
    size: [32, 64]
    fields:
      character_id: {character}
      dialogue_id: {dialogue}
      prompt: Talk
      brain_override: stand_still""" for character, dialogue, x, floor in residents]
    return f"# Residents of {level_id}. Written by {GENERATOR}.\nlevel_id: {level_id}\nentities:\n" + "\n".join(rows) + "\n"


def sign_yaml(level_id: str, signs: list) -> str:
    rows = [f"""  - type: DebugLabel
    px: [{x - w // 2}, {y}]
    size: [{w}, 24]
    fields:
      name: {name}
      text: "{text}"
      category: Custom""" for name, x, y, text, w in signs]
    return f"# Signs of {level_id}. Written by {GENERATOR}.\nlevel_id: {level_id}\nentities:\n" + "\n".join(rows) + "\n"


def overlay_name(level_id: str, kind: str, i: int) -> str:
    """A placed piece's name: the same piece is placed in many rooms, and
    twice in one, and the name is what says it is placed already. The first
    pieces placed (2026-10-30) were named `<kind>_overlay`: those keep it."""
    legacy = {("primeval_tyrant_hall", "primeval_tyrant_hall_room"), ("evil_gate_forest", "evil_forest_roadside"), ("ninja_dojo", "shadow_dojo_hall")}
    return f"{kind}_overlay" if (level_id, kind) in legacy else f"{level_id}_{kind}_{i}"


def overlay_yaml(level_id: str, pieces: list[tuple[int, str, int, int]], px: float = 2.0) -> str:
    """The Props of `pieces`, each (its index in the room's list, kind,
    frame x, frame y): the index names it."""
    rows = []
    for i, kind, fx, fy in pieces:
        bx, by, bw, bh = body_box(kind)
        rows.append(f"""  - type: Prop
    px: [{round(fx + bx / px)}, {round(fy + by / px)}]
    size: [{max(1, round(bw / px))}, {max(1, round(bh / px))}]
    fields:
      name: {overlay_name(level_id, kind, i)}
      kind: {kind}""")
    return f"# Scenery of {level_id}. Written by {GENERATOR}.\nlevel_id: {level_id}\nentities:\n" + "\n".join(rows) + "\n"


def main(argv) -> int:
    for world, levels in OVERLAYS.items():
        ldtk = WORLDS / world
        for level_id, pieces in levels.items():
            # Each piece is added once, by its own name: a piece appended to
            # a room's list later is added on its own.
            placed = ldtk.read_text()
            missing = [(i, *piece) for i, piece in enumerate(pieces) if f'"{overlay_name(level_id, piece[0], i)}"' not in placed]
            if "--apply" not in argv:
                print(f"{world} {level_id}: {len(pieces) - len(missing)} piece(s) present, would add {len(missing)}")
                continue
            if missing and (code := add_entities(overlay_yaml(level_id, missing), ldtk, HERE / f".{level_id}_scenery.yaml")):
                return code
    for world, levels in RESIDENTS.items():
        ldtk = WORLDS / world
        for level_id, residents in levels.items():
            first = f'"{residents[0][1]}"'
            if "--apply" not in argv:
                print(f"{world} {level_id}: {'present' if first in ldtk.read_text() else 'would add'} {len(residents)} resident(s)")
            elif code := add_entities(resident_yaml(level_id, residents), ldtk, HERE / f".{level_id}_residents.yaml", unless=first):
                return code
    for world, levels in SIGNS.items():
        ldtk = WORLDS / world
        for level_id, signs in levels.items():
            first = f'"{signs[0][0]}"'
            if "--apply" not in argv:
                print(f"{world} {level_id}: {'present' if first in ldtk.read_text() else 'would add'} {len(signs)} sign(s)")
            elif code := add_entities(sign_yaml(level_id, signs), ldtk, HERE / f".{level_id}_signs.yaml", unless=first):
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
