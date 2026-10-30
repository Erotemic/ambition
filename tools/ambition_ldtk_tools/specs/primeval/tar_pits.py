#!/usr/bin/env python3
"""Write the Tar Pits, out past the end of the primeval trail, east of the
canopy.

    python specs/primeval/tar_pits.py          # writes the spec
    python specs/primeval/tar_pits.py --apply  # and makes the room

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "out of town zones with enemies", joined by side exits on
a flat map. The pits touch the canopy's east edge with the same floor (world
y 480). The canopy is edited in place (`side_exits.py`: the primeval
generator is older than its levels), so its east exit is ADDED to it and its
wall opened.

- Two pools of tar (murky water, its picture in the art) to wade, slowly.
- A mammoth half sunk in the first, its spine walked on (y 480).
- The paleontologists' dig camp at the east end, its scaffold (y 480, 384).
- Raptors, a triceratops, a dilophosaurus, compys, a vulture waiting; Dr
  Feld the paleontologist, who will not leave.

The art is the sprite tools' `targets/props/primeval_tar_props.py`, drawn on
these numbers.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, edge, entity, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "dinosaur.ldtk"
GENERATOR = "specs/primeval/tar_pits.py"
PITS = "primeval_tar_pits"
CANOPY = "primeval_canopy"
#: The canopy's east edge in its room (x), its floor (y), and where it is in
#: the world: the pits start where it ends, floors level.
CANOPY_EAST, CANOPY_FLOOR, CANOPY_AT = 1984, 672, (30816, -192)
W, H, FLOOR, BED = 2400, 640, 576, 624
POOLS = ((520, 900), (1300, 1700))


def walk(name, x, y, w):
    """A one-way walk the room's art draws: a still platform, no picture."""
    return entity("MovingPlatform", x, y, w, 16, id=name, name=name, speed=0.0, visual="none", one_way=True)


def enemy(n, character, x, feet, w, h):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{PITS}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def pits():
    (a0, a1), (b0, b1) = POOLS
    e = [
        piece("tar_pits_bones", 120, 500, name="tar_bones"),
        piece("tar_pits_pools", 500, 440, name="tar_pools"),
        piece("tar_pits_mammoth", 440, 410, name="tar_mammoth"),
        piece("tar_pits_dig_camp", 1730, 360, name="tar_dig_camp"),
        solid("ground_west", 0, FLOOR, a0, H - FLOOR),
        solid("bed_west", a0, BED, a1 - a0, H - BED),
        solid("ground_mid", a1, FLOOR, b0 - a1, H - FLOOR),
        solid("bed_east", b0, BED, b1 - b0, H - BED),
        solid("ground_east", b1, FLOOR, W - b1, H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR - 112),
        solid("east_wall", W - WALL, 0, WALL, FLOOR),
        edge("tar_west", "back to the canopy", 0, FLOOR, CANOPY, "primeval_canopy_tar_pits"),
        start(PITS, 120, FLOOR),
        walk("mammoth_back", 600, 480, 240),
        walk("scaffold_low", 1980, 480, 160),
        walk("scaffold_high", 2040, 384, 160),
        sign("tar_sign", 300, FLOOR - 200, "THE TAR PITS: everything here was once in a hurry", 480),
        sign("tar_sign_wet", 1500, FLOOR - 150, "WET TAR (since the Pleistocene)", 340),
        sign("dig_sign", 2120, 300, "DIG SITE: please do not fossilize", 380),
        # Dr Feld, at his sieve (`dialogue/sandbox/primeval.yarn`).
        resident("npc_primeval_feld", "primeval_feld", 2190, FLOOR),
        enemy(1, "npc_raptor_stalker", 1000, FLOOR, 48, 80),
        enemy(2, "npc_raptor_stalker", 1180, FLOOR, 48, 80),
        # The dig camp (x 1730..) is the one quiet place: the animals keep
        # to the west of it, the triceratops on the camp's doorstep.
        enemy(3, "triceratops", 1720, FLOOR, 96, 92),
        enemy(4, "dilophosaurus", 340, FLOOR, 44, 74),
        enemy(5, "compsognathus", 960, FLOOR, 24, 26),
        enemy(6, "compsognathus", 1240, FLOOR, 24, 26),
        enemy(7, "vulture", 700, 420, 44, 66),
    ]
    at = (CANOPY_AT[0] + CANOPY_EAST + WALL, CANOPY_AT[1] + CANOPY_FLOOR - FLOOR)
    return room(PITS, e, at=at, size=(W, H), biome="primeval", music="dinosaur_liberators", bg="#2E3B22",
                title="THE TAR PITS", generator=GENERATOR)


CANOPY_EDGE_YAML = f"""# The canopy's way east to the tar pits. Written by {GENERATOR}.
level_id: {CANOPY}
entities:
  - type: LoadingZone
    px: [{CANOPY_EAST}, {CANOPY_FLOOR - 96}]
    size: [{WALL}, 96]
    fields:
      id: primeval_canopy_tar_pits
      name: on to the tar pits
      activation: EdgeExit
      target_room: {PITS}
      target_zone: tar_west
      bidirectional: true
"""


def main(argv):
    path = HERE / f"{PITS}.ron"
    path.write_text(pits())
    print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    # The pits' west edge and the canopy's east edge name each other, so the
    # check after the first of the two writes refuses the half-joined world;
    # only the end state is checked (the last `validate`).
    make_rooms(WORLD, [path])
    if code := add_entities(CANOPY_EDGE_YAML, WORLD, HERE / ".canopy_edge.yaml", unless='"on to the tar pits"'):
        return code
    for args in ((CANOPY, "Collision", 0, CANOPY_EAST, CANOPY_FLOOR - 112, WALL, 112),
                 # The tar: murky water (the IntGrid Water layer's kind 2).
                 *((PITS, "Water", 2, x0, FLOOR, x1 - x0, BED - FLOOR) for x0, x1 in POOLS)):
        if code := paint(WORLD, *args):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
