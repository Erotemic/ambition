#!/usr/bin/env python3
"""Write the lower floor of Castle Godwin (the Evil dimension): the kitchens
under the gatehouse, the Complaints Department, the cells, and the Prototype
Workshop off the lab tower's foot.

    python specs/castle_kitchens/make_specs.py                     # writes the specs
    python specs/castle_kitchens/make_specs.py --apply             # and makes the rooms
    python specs/castle_kitchens/make_specs.py --apply --relayout  # back to WHERE

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "build out the world detail ... adding art, flavor,
character"; then: "focus on compositions of levels ... large interconnected
regions ... It needs to feel like a connected world, not a series of
disconnected doors." The castle's staff live on its lower floor, and the
floor joins the castle by its edges on a faithful flat map: a stair down from
the gatehouse (a hole in its floor, `castle_godwin/make_specs.py`) into the
kitchens; east along the floor to the Complaints Department, the cells, and
into the lab tower partway up; the workshop off the tower's foot. With the
barracks into the tower above, the castle is a loop.

Mrs Hargreave the cook, Osgood the night porter, Brigid Dunmore of the
Henchpersons' Guild (`dialogue/sandbox/castle_godwin.yarn`); Agnes Mudge
holding ticket 4,117; Ambrose Quill, the cells' trusty; Dr. Gideon Sprocket.

Where a room is in the world is its LDtk editor's once it exists: `--apply`
keeps each room where it stands; `WHERE` places a new room, or every room
with `--relayout`. `specs/check_flat_map.py evil.ldtk` checks the joins.

The art is the sprite tools' `targets/props/castle_kitchens_props.py`,
`castle_cells_props.py` and `evil_workshop_props.py`.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, edge, entity, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "evil.ldtk"
GENERATOR = "specs/castle_kitchens/make_specs.py"
KITCHENS = "castle_godwin_kitchens"
COMPLAINTS = "castle_godwin_complaints"
CELLS = "castle_godwin_cells"
WORKSHOP = "castle_godwin_workshop"
GATEHOUSE = "castle_godwin_gatehouse"
TOWER = "castle_godwin_tower"
W, H, FLOOR, CEILING = 1280, 576, 512, 48
CW, CH, CFLOOR = 960, 448, 384
#: The cells: as wide as the gap to the tower, and 64 taller than the rooms
#: by them so their vault meets the barracks' floor (the trapdoor's shaft,
#: x 160..224 under the barracks' 2000..2064).
LW, LH, LFLOOR, LCEILING = 720, 640, 576, 112
TRAPDOOR = (160, 224)
TRAPDOOR_LEVER = "castle_godwin_trapdoor_lever"
#: Where each room goes when it is new (or with `--relayout`): the floor of
#: each at world y 1344, the tower's join to the cells. The kitchens under
#: the gatehouse's stair (their x 40..104 under its x 1240..1304); the
#: complaints and the cells east of them, the cells' east edge on the
#: tower's west; the workshop's floor level with the tower's foot (1728),
#: its west edge on the tower's east.
WHERE = {
    KITCHENS: (3200, 832),
    COMPLAINTS: (3200 + W, 1344 - CFLOOR),
    CELLS: (3200 + W + CW, 1344 - LFLOOR),
    WORKSHOP: (7184, 1728 - FLOOR),
}
#: The stair up to the gatehouse: the shaft in the kitchens' ceiling (x0, x1).
STAIR = (40, 104)


def kitchens():
    s0, s1 = STAIR
    e = [
        piece("castle_kitchens_room", 16, CEILING, name="kitchens_room"),
        solid("floor", 0, FLOOR, W, H - FLOOR),
        solid("ceiling_west", 0, 0, s0, CEILING),
        solid("ceiling_east", s1, 0, W - s1, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", W - WALL, CEILING, WALL, FLOOR - 112 - CEILING),
        entity("LoadingZone", s0, 0, s1 - s0, 16, id="kitchens_stair", name="up the stair to the gatehouse",
               activation="EdgeExit", target_room=GATEHOUSE, target_zone="castle_godwin_gatehouse_stair", bidirectional=True),
        edge("kitchens_east", "on to the Complaints Department", W - WALL, FLOOR, COMPLAINTS, "complaints_west"),
        start(KITCHENS, 160, FLOOR),
        resident("npc_castle_osgood", "castle_osgood", 280, FLOOR),
        resident("npc_castle_hargreave", "castle_hargreave", 860 - 20, FLOOR),
        resident("npc_castle_dunmore", "castle_dunmore", 1040, FLOOR),
        sign("kitchens_sign_menu", 660, 250, "TODAY: gruel. TOMORROW: gruel (evil)", 400),
        sign("kitchens_sign_guild", 1010, 350, "HENCHPERSONS' GUILD, LOCAL 666: meeting at the kettle", 460),
    ]
    return room(KITCHENS, e, at=WHERE[KITCHENS], size=(W, H), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE CASTLE KITCHENS", generator=GENERATOR)


def complaints():
    e = [
        piece("complaints_room", 16, CEILING, name="complaints_room"),
        solid("floor", 0, CFLOOR, CW, CH - CFLOOR),
        solid("ceiling", 0, 0, CW, CEILING),
        solid("west_wall", 0, CEILING, WALL, CFLOOR - 112 - CEILING),
        solid("east_wall", CW - WALL, CEILING, WALL, CFLOOR - 112 - CEILING),
        edge("complaints_west", "back to the kitchens", 0, CFLOOR, KITCHENS, "kitchens_east"),
        edge("complaints_east", "on to the cells", CW - WALL, CFLOOR, CELLS, "cells_west"),
        start(COMPLAINTS, 120, CFLOOR),
        resident("npc_evil_mudge", "evil_mudge", 200, CFLOOR),
        sign("complaints_sign_ticket", 316, 290, "TAKE A NUMBER", 200),
        sign("complaints_sign_hours", 660, 340, "OPEN: by appointment. APPOINTMENTS: by complaint", 470),
        sign("complaints_sign_serving", 615, 70, "NOW SERVING", 160),
        sign("complaints_sign_box", 400, 250, "SUGGESTIONS", 160),
    ]
    return room(COMPLAINTS, e, at=WHERE[COMPLAINTS], size=(CW, CH), biome="castle_godwin", music="doves_of_the_commons",
                bg="#1A1416", title="THE COMPLAINTS DEPARTMENT", generator=GENERATOR)


def workshop():
    e = [
        piece("workshop_room", 16, CEILING, name="workshop_room"),
        solid("floor", 0, FLOOR, W, H - FLOOR),
        solid("ceiling", 0, 0, W, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - 112 - CEILING),
        solid("east_wall", W - WALL, CEILING, WALL, FLOOR - CEILING),
        edge("workshop_west", "back into the lab tower", 0, FLOOR, TOWER, "castle_godwin_tower_foot"),
        start(WORKSHOP, 140, FLOOR),
        resident("npc_castle_sprocket", "castle_sprocket", 790, FLOOR),
        entity("ChestSpawn", 1110, FLOOR - 58, 76, 58, name="a crate of spare parts", reward="currency:40"),
        sign("workshop_sign_kettle", 1040, FLOOR - 150, "MARK 47: almost", 220),
        sign("workshop_sign_button", 1180, FLOOR - 110, "DO NOT", 120),
        sign("workshop_sign_tools", 255, 220, "RETURN THE BIG SPANNER", 300),
    ]
    return room(WORKSHOP, e, at=WHERE[WORKSHOP], size=(W, H), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE PROTOTYPE WORKSHOP", generator=GENERATOR)


def cells():
    """The cells: where the complaints end up, down the chute from the
    suggestions box, and the people who made them. Ambrose Quill, the
    trusty, out of the open cell (`castle_godwin.yarn`)."""
    t0, t1 = TRAPDOOR
    e = [
        piece("castle_cells_room", 16, LCEILING, name="cells_room"),
        solid("floor", 0, LFLOOR, LW, LH - LFLOOR),
        solid("vault_west", 0, 0, t0, LCEILING),
        solid("vault_east", t1, 0, LW - t1, LCEILING),
        solid("west_wall", 0, LCEILING, WALL, LFLOOR - 112 - LCEILING),
        solid("east_wall", LW - WALL, LCEILING, WALL, LFLOOR - 112 - LCEILING),
        edge("cells_west", "back to the Complaints Department", 0, LFLOOR, COMPLAINTS, "complaints_east"),
        edge("cells_east", "into the lab tower", LW - WALL, LFLOOR, TOWER, "castle_godwin_tower_cells"),
        # The shortcut: a ladder up the shaft to the barracks' trapdoor, shut
        # until this lever is pulled.
        entity("LoadingZone", t0, 0, t1 - t0, 16, id="cells_trapdoor", name="up through the trapdoor to the barracks",
               activation="EdgeExit", target_room="castle_godwin_barracks", target_zone="castle_godwin_barracks_trapdoor",
               bidirectional=True),
        entity("Switch", 600, LFLOOR - 32, 24, 32, id=TRAPDOOR_LEVER, name="the trapdoor lever", prompt="Pull the lever",
               action="ToggleFlag", target_encounter=""),
        start(CELLS, 300, LFLOOR),
        resident("npc_castle_quill", "castle_quill", 420, LFLOOR),
        sign("cells_sign_chute", 140, 150, "SUGGESTIONS: received", 260),
        sign("cells_sign_rules", 520, 180, "NO COMPLAINING", 220),
        sign("cells_sign_trapdoor", 250, 70, "TRAPDOOR: to the barracks (lever by the desk)", 380),
    ]
    return room(CELLS, e, at=WHERE[CELLS], size=(LW, LH), biome="castle_godwin", music="doves_of_the_commons", bg="#1A1416",
                title="THE CELLS", generator=GENERATOR)


#: Each room's spec, the workshop first: a room may only stand where no other
#: does, and the old places overlap the new.
ROOMS = ((WORKSHOP, workshop), (COMPLAINTS, complaints), (KITCHENS, kitchens), (CELLS, cells))


def main(argv):
    paths = []
    for rid, make in ROOMS:
        path = HERE / f"{rid}.ron"
        path.write_text(make())
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    # The rooms name each other: only the end state is checked.
    make_rooms(WORLD, paths, keep_position="--relayout" not in argv)
    # The stair's ladder, up the kitchens' shaft; the trapdoor's, up the
    # cells'.
    if code := paint(WORLD, KITCHENS, "Climbable", 1, STAIR[0] + 16, 0, 32, FLOOR):
        return code
    if code := paint(WORLD, CELLS, "Climbable", 1, TRAPDOOR[0] + 16, 0, 32, LFLOOR):
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
