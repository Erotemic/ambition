#!/usr/bin/env python3
"""Write the Searchlight Moor, out of town in the Evil dimension, west of the
gate forest.

    python specs/evil_moor/make_specs.py          # writes the spec
    python specs/evil_moor/make_specs.py --apply  # and makes the room

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "out of town zones with enemies", joined by side exits on a
flat map. The moor touches the gate forest's west edge with the same floor
(y 656). The forest is Castle Godwin's generator's room, edited in place
since: its west exit is ADDED to it (and its wall opened), not regenerated.

- A murky bog (x 600..1100) to wade, a drowned cart in it.
- Castle Godwin's watchtower, ruined, its landings to climb (y 560..272), a
  searchlight on top.
- The founder's statue, toppled on its face: its back is walked on (y 600).
- Barbed wire. Attack hounds, failed prototypes and zombies.

The art is the sprite tools' `targets/props/evil_moor_props.py`, drawn on
these numbers.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, edge, entity, ledge, make_rooms, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "evil.ldtk"
GENERATOR = "specs/evil_moor/make_specs.py"
MOOR = "evil_searchlight_moor"
FOREST = "evil_gate_forest"
W, H, FLOOR = 2400, 784, 656
BOG = (600, 1100, 640)


def enemy(n, character, x, feet, w, h):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{MOOR}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def moor():
    bx0, bx1, water = BOG
    e = [
        piece("moor_wire", 200, 600, name="moor_wire"),
        piece("moor_bog", 560, 580, name="moor_bog"),
        piece("moor_watchtower", 1180, 0, name="moor_watchtower"),
        piece("moor_statue", 1740, 500, name="moor_statue"),
        solid("ground_west", 0, FLOOR, bx0, H - FLOOR),
        solid("bog_bed", bx0, 720, bx1 - bx0, H - 720),
        solid("ground_east", bx1, FLOOR, W - bx1, H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR),
        solid("east_wall", W - WALL, 0, WALL, FLOOR - 112),
        edge("moor_east", "back to the forest", W - WALL, FLOOR, FOREST, "forest_west"),
        start(MOOR, W - 200, FLOOR),
        *[ledge(f"tower_{i}", x0, y, x1 - x0) for i, (x0, x1, y) in enumerate(((1500, 1620, 560), (1540, 1660, 464), (1500, 1620, 368), (1520, 1680, 272)))],
        # His back, walked on: a still one-way platform with no picture (the
        # statue's art draws it), as the math town's cradle plinth.
        entity("MovingPlatform", 1890, 600, 300, 16, id="statue_back", name="statue_back", speed=0.0, visual="none", one_way=True),
        # A chest for whoever climbs to the top (Jon, 2026-10-30: "interactive elements").
        entity("ChestSpawn", 1600, 272 - 58, 76, 58, name="the watchman's strongbox", reward="currency:50"),
        sign("moor_sign", W - 200, FLOOR - 260, "THE MOOR: curfew is always", 360),
        sign("tower_sign", 1590, 110, "WATCHTOWER 7 (the other six are worse)", 380),
        sign("bog_sign", 850, FLOOR - 140, "BOG: do not drink, do not swim, do not look", 440),
        sign("statue_sign", 2050, 450, "OUR FOUNDER (temporarily horizontal)", 380),
        enemy(1, "npc_evil_attack_hound", 420, FLOOR, 52, 50),
        enemy(2, "npc_evil_attack_hound", 1760, FLOOR, 52, 50),
        enemy(3, "npc_evil_failed_prototype", 1260, FLOOR, 32, 40),
        enemy(4, "npc_evil_failed_prototype", 1420, FLOOR, 32, 40),
        enemy(5, "npc_evil_zombie", 220, FLOOR, 30, 67),
        enemy(6, "npc_evil_zombie", 1960, 580, 30, 67),
    ]
    return MOOR, room(MOOR, e, at=(-W, 0), size=(W, H), biome="evil_forest", music="oppressive_crush",
                      bg="#3A0E0E", title="THE SEARCHLIGHT MOOR", generator=GENERATOR)


FOREST_EDGE_YAML = f"""# The gate forest's way west to the moor. Written by {GENERATOR}.
level_id: {FOREST}
entities:
  - type: LoadingZone
    px: [0, {FLOOR - 96}]
    size: [{WALL}, 96]
    fields:
      id: forest_west
      name: out to the moor
      activation: EdgeExit
      target_room: {MOOR}
      target_zone: moor_east
      bidirectional: true
"""


def main(argv):
    rid, text = moor()
    path = HERE / f"{rid}.ron"
    path.write_text(text)
    print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    # The moor's east edge and the forest's west edge name each other, so the
    # check after the first of the two writes refuses the half-joined world;
    # only the end state is checked (the last `validate`).
    make_rooms(WORLD, [path])
    if code := add_entities(FOREST_EDGE_YAML, WORLD, HERE / "forest_edge.yaml", unless='"out to the moor"'):
        return code
    code, _ = run("intgrid", "paint", "--ldtk", str(WORLD), "--level", FOREST, "--layer", "Collision", "--value", "0",
                  "--px", f"0,{FLOOR - 112}", "--size", f"{WALL},112")
    if code:
        return code
    # The bog: murky water (the IntGrid Water layer's kind 2; a WaterVolume
    # entity is only ever clear).
    bx0, bx1, water = BOG
    code, _ = run("intgrid", "paint", "--ldtk", str(WORLD), "--level", MOOR, "--layer", "Water", "--value", "2",
                  "--px", f"{bx0},{water}", "--size", f"{bx1 - bx0},{720 - water}")
    if code:
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
