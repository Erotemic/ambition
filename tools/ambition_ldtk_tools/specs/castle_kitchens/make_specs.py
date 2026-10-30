#!/usr/bin/env python3
"""Write the kitchens under Castle Godwin's gatehouse (the Evil dimension).

    python specs/castle_kitchens/make_specs.py          # writes the spec
    python specs/castle_kitchens/make_specs.py --apply  # and makes the room

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "build out the world detail ... adding art, flavor,
character". The castle had soldiers and monsters and nobody who lives there:
the kitchens are where its staff eat, gossip and organise. Mrs Hargreave the
cook at the hearth, Osgood the night porter at his desk, Brigid Dunmore of
the Henchpersons' Guild at the long table
(`dialogue/sandbox/castle_godwin.yarn`). Through a door at the kitchens'
east end, the castle's Complaints Department: two windows, both CLOSED,
NOW SERVING 3, and Agnes Mudge holding ticket 4,117. And by a door ADDED to
the barracks (x 2160, by its failed prototypes), the Prototype Workshop
where they come from, and Dr. Gideon Sprocket.

A building inside the castle, so a door: from the gatehouse floor (y 704,
x 1240), which is ADDED to the gatehouse (its generator is older than its
level, as the moor's forest).

The art is the sprite tools' `targets/props/castle_kitchens_props.py`.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, door, entity, make_rooms, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "evil.ldtk"
GENERATOR = "specs/castle_kitchens/make_specs.py"
KITCHENS = "castle_godwin_kitchens"
COMPLAINTS = "castle_godwin_complaints"
WORKSHOP = "castle_godwin_workshop"
BARRACKS = "castle_godwin_barracks"
BARRACKS_DOOR_X, BARRACKS_FLOOR = 2160, 704
#: East of the Complaints Department, clear of the throne's halls below.
WORKSHOP_AT = (4704, 896)
GATEHOUSE = "castle_godwin_gatehouse"
GATEHOUSE_DOOR_X, GATEHOUSE_FLOOR = 1240, 704
#: Under the gatehouse, clear of the throne's halls below.
AT = (2400, 896)
W, H, FLOOR, CEILING = 1280, 576, 512, 48
#: East of the kitchens; its door at the kitchens' x 1130 (drawn in the art).
COMPLAINTS_AT = (3680, 896)
CW, CH, CFLOOR = 960, 448, 384
COMPLAINTS_DOOR_X = 1130


def kitchens():
    e = [
        piece("castle_kitchens_room", 16, CEILING, name="kitchens_room"),
        solid("floor", 0, FLOOR, W, H - FLOOR),
        solid("ceiling", 0, 0, W, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", W - WALL, CEILING, WALL, FLOOR - CEILING),
        door("kitchens_door", "up to the gatehouse", 40, FLOOR, GATEHOUSE, "castle_godwin_gatehouse_kitchens"),
        door("kitchens_complaints_door", "the Complaints Department", COMPLAINTS_DOOR_X, FLOOR, COMPLAINTS, "complaints_door"),
        start(KITCHENS, 140, FLOOR),
        resident("npc_castle_osgood", "castle_osgood", 280, FLOOR),
        resident("npc_castle_hargreave", "castle_hargreave", 860 - 20, FLOOR),
        resident("npc_castle_dunmore", "castle_dunmore", 1040, FLOOR),
        sign("kitchens_sign_menu", 660, 250, "TODAY: gruel. TOMORROW: gruel (evil)", 400),
        sign("kitchens_sign_guild", 1010, 350, "HENCHPERSONS' GUILD, LOCAL 666: meeting at the kettle", 460),
    ]
    return room(KITCHENS, e, at=AT, size=(W, H), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE CASTLE KITCHENS", generator=GENERATOR)



def complaints():
    e = [
        piece("complaints_room", 16, CEILING, name="complaints_room"),
        solid("floor", 0, CFLOOR, CW, CH - CFLOOR),
        solid("ceiling", 0, 0, CW, CEILING),
        solid("west_wall", 0, CEILING, WALL, CFLOOR - CEILING),
        solid("east_wall", CW - WALL, CEILING, WALL, CFLOOR - CEILING),
        door("complaints_door", "back to the kitchens", 32, CFLOOR, KITCHENS, "kitchens_complaints_door"),
        start(COMPLAINTS, 120, CFLOOR),
        resident("npc_evil_mudge", "evil_mudge", 200, CFLOOR),
        sign("complaints_sign_ticket", 316, 290, "TAKE A NUMBER", 200),
        sign("complaints_sign_hours", 660, 340, "OPEN: by appointment. APPOINTMENTS: by complaint", 470),
        sign("complaints_sign_serving", 615, 70, "NOW SERVING", 160),
        sign("complaints_sign_box", 400, 250, "SUGGESTIONS", 160),
    ]
    return room(COMPLAINTS, e, at=COMPLAINTS_AT, size=(CW, CH), biome="castle_godwin", music="doves_of_the_commons",
                bg="#1A1416", title="THE COMPLAINTS DEPARTMENT", generator=GENERATOR)


def workshop():
    e = [
        piece("workshop_room", 16, CEILING, name="workshop_room"),
        solid("floor", 0, FLOOR, W, H - FLOOR),
        solid("ceiling", 0, 0, W, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", W - WALL, CEILING, WALL, FLOOR - CEILING),
        door("workshop_door", "up to the barracks", 40, FLOOR, BARRACKS, "castle_godwin_barracks_workshop"),
        start(WORKSHOP, 140, FLOOR),
        resident("npc_castle_sprocket", "castle_sprocket", 790, FLOOR),
        entity("ChestSpawn", 1110, FLOOR - 58, 76, 58, name="a crate of spare parts", reward="currency:40"),
        sign("workshop_sign_kettle", 1040, FLOOR - 150, "MARK 47: almost", 220),
        sign("workshop_sign_button", 1180, FLOOR - 110, "DO NOT", 120),
        sign("workshop_sign_tools", 255, 220, "RETURN THE BIG SPANNER", 300),
    ]
    return room(WORKSHOP, e, at=WORKSHOP_AT, size=(W, H), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE PROTOTYPE WORKSHOP", generator=GENERATOR)


BARRACKS_DOOR_YAML = f"""# The barracks' door down to the workshop. Written by {GENERATOR}.
level_id: {BARRACKS}
entities:
  - type: LoadingZone
    px: [{BARRACKS_DOOR_X}, {BARRACKS_FLOOR - 96}]
    size: [48, 96]
    fields:
      id: castle_godwin_barracks_workshop
      name: down to the workshop
      activation: Door
      target_room: {WORKSHOP}
      target_zone: workshop_door
      bidirectional: true
"""


GATEHOUSE_DOOR_YAML = f"""# The gatehouse's door down to the kitchens. Written by {GENERATOR}.
level_id: {GATEHOUSE}
entities:
  - type: LoadingZone
    px: [{GATEHOUSE_DOOR_X}, {GATEHOUSE_FLOOR - 96}]
    size: [48, 96]
    fields:
      id: castle_godwin_gatehouse_kitchens
      name: down to the kitchens
      activation: Door
      target_room: {KITCHENS}
      target_zone: kitchens_door
      bidirectional: true
"""


def main(argv):
    paths = []
    for rid, make in ((KITCHENS, kitchens), (COMPLAINTS, complaints), (WORKSHOP, workshop)):
        path = HERE / f"{rid}.ron"
        path.write_text(make())
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    # The two doors name each other: only the end state is checked.
    make_rooms(WORLD, paths)
    if code := add_entities(GATEHOUSE_DOOR_YAML, WORLD, HERE / ".gatehouse_door.yaml", unless='"down to the kitchens"'):
        return code
    if code := add_entities(BARRACKS_DOOR_YAML, WORLD, HERE / ".barracks_door.yaml", unless='"down to the workshop"'):
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
