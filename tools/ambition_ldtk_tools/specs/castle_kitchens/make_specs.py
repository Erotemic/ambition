#!/usr/bin/env python3
"""Write the kitchens under Castle Godwin's gatehouse (the Evil dimension).

    python specs/castle_kitchens/make_specs.py          # writes the spec
    python specs/castle_kitchens/make_specs.py --apply  # and makes the room

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "build out the world detail ... adding art, flavor,
character". The castle had soldiers and monsters and nobody who lives there:
the kitchens are where its staff eat, gossip and organise. Mrs Hargreave the
cook at the hearth, Osgood the night porter at his desk, Brigid Dunmore of
the Henchpersons' Guild at the long table (`dialogue/sandbox/castle_godwin.yarn`).

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
GATEHOUSE = "castle_godwin_gatehouse"
GATEHOUSE_DOOR_X, GATEHOUSE_FLOOR = 1240, 704
#: Under the gatehouse, clear of the throne's halls below.
AT = (2400, 896)
W, H, FLOOR, CEILING = 1280, 576, 512, 48


def kitchens():
    e = [
        piece("castle_kitchens_room", 16, CEILING, name="kitchens_room"),
        solid("floor", 0, FLOOR, W, H - FLOOR),
        solid("ceiling", 0, 0, W, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", W - WALL, CEILING, WALL, FLOOR - CEILING),
        door("kitchens_door", "up to the gatehouse", 40, FLOOR, GATEHOUSE, "castle_godwin_gatehouse_kitchens"),
        start(KITCHENS, 140, FLOOR),
        resident("npc_castle_osgood", "castle_osgood", 280, FLOOR),
        resident("npc_castle_hargreave", "castle_hargreave", 860 - 20, FLOOR),
        resident("npc_castle_dunmore", "castle_dunmore", 1040, FLOOR),
        sign("kitchens_sign_menu", 660, 250, "TODAY: gruel. TOMORROW: gruel (evil)", 400),
        sign("kitchens_sign_guild", 1010, 350, "HENCHPERSONS' GUILD, LOCAL 666: meeting at the kettle", 460),
    ]
    return room(KITCHENS, e, at=AT, size=(W, H), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE CASTLE KITCHENS", generator=GENERATOR)


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
    path = HERE / f"{KITCHENS}.ron"
    path.write_text(kitchens())
    print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    # The two doors name each other: only the end state is checked.
    make_rooms(WORLD, [path])
    if code := add_entities(GATEHOUSE_DOOR_YAML, WORLD, HERE / ".gatehouse_door.yaml", unless='"down to the kitchens"'):
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
