#!/usr/bin/env python3
"""Write the rooms of the Pirate Port, and the door to it from the pirate cove.

    python specs/pirate_port/make_specs.py          # writes the specs
    python specs/pirate_port/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The Pirate Port is a seafaring town of the pirate dimension, its own world
file (`pvn.ldtk`, the PvN dimension of pirates and ninjas). Gate address SAILING arrives in its gate plaza.
The design is in `README.md` here. In short:

- `pirate_port_gate_plaza`: the gate plaza. Travellers arrive at the ring
  (Agent GATE places the ring and its terminal on the clear floor at the
  west end). The dock clerk logs the ring's freight; a traveller is sick
  from the crossing. A stair goes up to the harbour balcony.
- `pirate_port_waterfront`: the market street, and over it the galleries
  and roofs of the houses. The chandler's stall; the tavern's door.
- `pirate_port_tavern`: the tavern, The Drowned Compass.
- `pirate_port_docks`: the pier over the harbour, a brig at her moorings,
  the warehouse, and the gantry. The harbour master; the way to the cove.

THE LOOP: the low road (plaza floor, market street, pier) and the high road
(the brig's rigging, the rope to the warehouse roof, the gantry, the roofs
and galleries of the waterfront, the harbour balcony, the plaza's stair).
Each room joins the next at both heights.

The problem: a page of the clerk's freight log blew away to the crow's nest
of the brig (`flag:pirate_port_manifest_found`). The quest is
`pirate_port_lost_manifest` (`quests.ron`), the talk `pirate_port.yarn`.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

from district_kit import (  # noqa: E402
    HUB_ROOM, SANDBOX_LDTK, WALL, WORLDS, add_entities, door, edge, ensure_world, entity, ledge, make_rooms, paint, resident, room, run, sign, solid, start, water,
)

import props  # noqa: E402  (this directory's scenery)

#: The PvN dimension's world file (Jon, 2026-10-29: the pirate and ninja
#: dimensions are one): the port and the village are both in it.
WORLD_LDTK = WORLDS / "pvn.ldtk"
UID_OFFSET = 600000
GENERATOR = "specs/pirate_port/make_specs.py"

PLAZA = "pirate_port_gate_plaza"
WATERFRONT = "pirate_port_waterfront"
TAVERN = "pirate_port_tavern"
DOCKS = "pirate_port_docks"

#: The contract with Agent GATE: the arrival point a dialled SAILING names.
ARRIVAL = "pirate_port_gate_arrival"
#: Until GATE's ring and return terminal stand, the arrival zone is a
#: one-way door home for testing: GATE makes it the ring's arrival pad.

#: The pirate cove (in this dimension's world file since 2026-10-29), and its door to the docks.
COVE = "pirate_cove"
COVE_DOOR = "pirate_cove_port_door"

BIOME = "pirate_port"
BG = "#9CC4D8"
TOWN_MUSIC = "pirates_guild_black_flag_jig"
DOCK_MUSIC = "ten_fathoms_and_a_grudge"

#: Each room: (world_x, world_y), (width, height).
ROOMS_AT = {
    PLAZA: ((0, 0), (1600, 832)),
    WATERFRONT: ((1856, 0), (2560, 896)),
    DOCKS: ((4672, 0), (2816, 960)),
    TAVERN: ((1856, 1152), (1024, 448)),
}

#: The flag of the page in the crow's nest.
PAGE_FLAG = "pirate_port_manifest_found"


def build(rid, entities, title, music=TOWN_MUSIC, **kw):
    at, size = ROOMS_AT[rid]
    # The room's scenery (`props.py`) is listed first: it draws behind.
    entities = [*props.scenery(rid), *entities]
    return rid, room(rid, entities, at=at, size=size, biome=BIOME, music=music, bg=BG, title=title, generator=GENERATOR, **kw)


def plaza():
    """The gate plaza. Floor 704. The ring stands on the clear floor of the
    west end (x 160..512 is kept clear to 224 over the floor); the clerk's
    booth and the freight crates are east of it. A stair of three steps goes
    up to the harbour balcony (y 448), and both the floor and the balcony go
    on east to the waterfront."""
    rid = PLAZA
    _, (w, h) = ROOMS_AT[rid]
    floor, balcony = 704, 448
    e = [
        solid("ground", 0, floor, w, h - floor),
        solid("west_wall", 0, 0, WALL, floor),
        solid("east_wall_high", w - WALL, 0, WALL, balcony - 96),
        solid("east_wall_mid", w - WALL, balcony + 16, WALL, floor - 96 - balcony - 16),
        # The arrival is the zone of the gate ring (`gate_interface.yaml`, written
        # by specs/gate_network/make_gates.py), added after the rooms are made.
        start(rid, 336, floor),
        # The clerk's booth, and the ring's freight stacked by it.
        ledge("freight_crate_low", 560, floor - 48, 96),
        ledge("freight_crate_high", 584, floor - 96, 48),
        resident("npc_port_ines", "pirate_port_ines", 704, floor),
        resident("npc_port_tomas", "pirate_port_tomas", 880, floor),
        sign("plaza_sign_freight", 700, floor - 200, "RING FREIGHT: declare all cargo, livestock and curses", 420),
        sign("plaza_sign_arrivals", 336, floor - 300, "SAILING: arrivals, mind the step and the nausea", 380),
        # The stair to the balcony.
        ledge("stair_1", 1000, floor - 64, 96),
        ledge("stair_2", 1080, floor - 128, 96),
        ledge("stair_3", 1160, floor - 192, 96),
        ledge("harbour_balcony", 1232, balcony, w - 1232),
        sign("plaza_sign_balcony", 1400, balcony - 80, "Harbour balcony: no fishing from it", 300),
        edge("plaza_east_low", "to the waterfront", w - WALL, floor, WATERFRONT, "waterfront_west_low"),
        edge("plaza_east_high", "along the galleries", w - WALL, balcony, WATERFRONT, "waterfront_west_high"),
    ]
    return build(rid, e, "THE PIRATE PORT: THE GATE PLAZA")


def waterfront():
    """The waterfront. The market street (y 768) runs the whole room; over it
    the galleries of the houses (y 512), then the roofs (y 448, 384) step up
    to the east. A stair by the chandler goes from the street to the
    galleries, and a ladder by the tavern."""
    rid = WATERFRONT
    _, (w, h) = ROOMS_AT[rid]
    street, gallery, roof_a, roof_b = 768, 512, 448, 384
    e = [
        solid("street", 0, street, w, h - street),
        solid("west_wall_high", 0, 0, WALL, gallery - 96),
        solid("west_wall_mid", 0, gallery + 16, WALL, street - 96 - gallery - 16),
        solid("east_wall_high", w - WALL, 0, WALL, roof_b - 96),
        solid("east_wall_mid", w - WALL, roof_b + 16, WALL, street - 96 - roof_b - 16),
        edge("waterfront_west_low", "to the gate plaza", 0, street, PLAZA, "plaza_east_low"),
        edge("waterfront_west_high", "to the harbour balcony", 0, gallery, PLAZA, "plaza_east_high"),
        start(rid, 120, street),
        # The galleries and the roofs.
        ledge("gallery", 16, gallery, 1600 - 16),
        ledge("roof_chandlery", 1664, roof_a, 352),
        ledge("roof_net_loft", 2080, roof_b, w - 2080),
        # From the street to the galleries: the chandler's stair.
        ledge("chandler_stair_1", 400, street - 64, 96),
        ledge("chandler_stair_2", 480, street - 128, 96),
        ledge("chandler_stair_3", 560, street - 192, 96),
        # The market.
        ledge("chandler_stall_roof", 760, street - 112, 160),
        resident("npc_port_agnes", "pirate_port_agnes", 840, street),
        sign("chandler_sign", 840, street - 180, "AGNES: CHANDLER. Rope, tar, biscuits, remedies", 380),
        door("waterfront_tavern_door", "into The Drowned Compass", 1320, street, TAVERN, "tavern_door"),
        sign("tavern_sign", 1344, street - 150, "THE DROWNED COMPASS", 260),
        sign("market_sign", 1900, street - 120, "Fish from three seas. Ask which.", 300),
        edge("waterfront_east_low", "to the docks", w - WALL, street, DOCKS, "docks_west_low"),
        edge("waterfront_east_high", "along the roofs to the gantry", w - WALL, roof_b, DOCKS, "docks_west_high"),
    ]
    return build(rid, e, "THE WATERFRONT")


def tavern():
    """The Drowned Compass: one room, the counter at the east end. Wren keeps
    it; Broadside Bess of the cove drinks in it."""
    rid = TAVERN
    _, (w, h) = ROOMS_AT[rid]
    floor, ceiling = h - 64, 32
    e = [
        solid("floor", 0, floor, w, 64),
        solid("ceiling", 0, 0, w, ceiling),
        solid("west_wall", 0, ceiling, WALL, floor - ceiling),
        solid("east_wall", w - WALL, ceiling, WALL, floor - ceiling),
        door("tavern_door", "out to the waterfront", 40, floor, WATERFRONT, "waterfront_tavern_door"),
        start(rid, 140, floor),
        ledge("counter", 720, floor - 40, 240),
        resident("npc_port_wren", "pirate_port_wren", 680, floor),
        resident("npc_pirate_heavy_broadside_bess", "pirate_port_bess", 400, floor, w=64, h=96),
        sign("tavern_sign_rules", 520, ceiling + 40, "No duels. No gate talk before noon.", 360),
    ]
    return build(rid, e, None)


#: The docks, from the west: the pier's three spans and the two gaps of
#: open water between them, the brig on the east span.
DECK, SEA_FLOOR = 768, 928
SPANS = [(0, 720), (832, 1504), (1616, 2816)]
HULL_X0, HULL_X1, SHIP_DECK = 1760, 2560, 640
MAINMAST = 2256
YARDS = [  # (name, x0, x1, y)
    ("fore_yard", 1880, 2040, 528),
    ("main_lower_yard", 2160, 2352, 528),
    ("main_upper_yard", 2176, 2336, 416),
    ("crows_nest", 2216, 2296, 304),
]
WAREHOUSE_ROOF, GANTRY = 448, 384


def docks():
    """The pier over the harbour. The low road is the pier (y 768), with two
    gaps of open water (ladders out of each). The brig lies on the east span:
    a gangway to her deck (y 640), and her yards up the masts to the crow's
    nest (y 304), where the clerk's page is. The high road goes back west:
    a rope from the main upper yard to the warehouse roof (y 448), then the
    gantry (y 384) to the waterfront's roofs. The door at the east end is the
    way to the pirate cove (and the skyship's lookout over it)."""
    rid = DOCKS
    _, (w, h) = ROOMS_AT[rid]
    e = [
        solid("sea_floor", 0, SEA_FLOOR, w, h - SEA_FLOOR),
        water("harbour", 0, DECK + 16, w, SEA_FLOOR - DECK - 16),
        solid("west_wall_high", 0, 0, WALL, GANTRY - 96),
        solid("west_wall_mid", 0, GANTRY + 16, WALL, DECK - 96 - GANTRY - 16),
        solid("east_wall", w - WALL, 0, WALL, DECK),
        edge("docks_west_low", "to the waterfront", 0, DECK, WATERFRONT, "waterfront_east_low"),
        edge("docks_west_high", "along the roofs to the waterfront", 0, GANTRY, WATERFRONT, "waterfront_east_high"),
        start(rid, 120, DECK),
    ]
    for i, (x0, x1) in enumerate(SPANS):
        e.append(solid(f"pier_span_{i}", x0, DECK, x1 - x0, 32))
    # The brig: her hull stands on the pier; a gangway to her deck.
    e += [
        solid("brig_hull", HULL_X0, SHIP_DECK, HULL_X1 - HULL_X0, DECK - SHIP_DECK),
        ledge("gangway", HULL_X0 - 80, DECK - 64, 80),
        *[ledge(name, x0, y, x1 - x0) for name, x0, x1, y in YARDS],
        entity("PickupSpawn", 2244, 304 - 32, 24, 24, name="the clerk's page", kind=f"flag:{PAGE_FLAG}", respawn="Never",
               sprite="port_manifest_page"),
        # The high road back west.
        ledge("signal_rope", 1504, 416, 2176 - 1504),
        ledge("warehouse_roof", 864, WAREHOUSE_ROOF, 1504 - 864),
        ledge("gantry", WALL, GANTRY, 864 - WALL),
        # The people of the pier.
        resident("npc_port_jory", "pirate_port_jory", 420, DECK),
        resident("npc_port_bram", "pirate_port_bram", 2640, DECK),
        door("docks_cove_door", "to the pirate cove", 2720, DECK, COVE, COVE_DOOR),
        sign("docks_sign_cove", 2580, DECK - 200, "PIRATE COVE: crews only. Skyship moorings above it.", 420),
        sign("docks_sign_brig", 2160, SHIP_DECK - 420, "THE BRIG PERSISTENCE: do not climb the rigging", 380),
        sign("docks_sign_warehouse", 1180, WAREHOUSE_ROOF + 120, "BONDED WAREHOUSE: ring freight awaiting a clerk", 420),
    ]
    return build(rid, e, "THE DOCKS", music=DOCK_MUSIC)


ROOMS = (plaza, waterfront, tavern, docks)

#: The ladders out of the two gaps of open water, and up the mainmast
#: (painted on the `Climbable` layer): (room, x, top, bottom).
LADDERS = [
    (DOCKS, SPANS[0][1] + 2, DECK - 16, SEA_FLOOR),
    (DOCKS, SPANS[1][1] + 2, DECK - 16, SEA_FLOOR),
    (DOCKS, MAINMAST - 16, 304 - 16, SHIP_DECK),
]

COVE_DOOR_YAML = f"""# The pirate cove's door to the Pirate Port's docks. Written by {GENERATOR}.
level_id: {COVE}
entities:
  - type: LoadingZone
    px: [808, 512]
    size: [48, 96]
    fields:
      id: {COVE_DOOR}
      name: to the port's docks
      activation: Door
      target_room: {DOCKS}
      target_zone: docks_cove_door
      bidirectional: true
"""

#: Agent GATE's ring and terminal in the plaza, if it has written them: a
#: regenerate of the rooms puts them back.
GATE_INTERFACE = HERE / "gate_interface.yaml"


def main(argv):
    made = []
    for make in ROOMS:
        rid, text = make()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(HERE.parents[1]))
    if "--apply" not in argv:
        return 0
    if code := ensure_world(WORLD_LDTK, UID_OFFSET):
        return code
    if code := make_rooms(WORLD_LDTK, made):
        return code
    if code := add_entities(COVE_DOOR_YAML, WORLD_LDTK, HERE / "cove_door.yaml", unless=f'"{COVE_DOOR}"'):
        return code
    if code := add_entities(props.cove_scenery_yaml(GENERATOR), WORLD_LDTK, HERE / "cove_scenery.yaml", unless='"cove_cave_0"'):
        return code
    if GATE_INTERFACE.exists():
        code, _ = run("entity", "add", str(GATE_INTERFACE), "--ldtk", str(WORLD_LDTK), "--in-place")
        if code:
            return code
    for rid, x, top, bottom in LADDERS:
        if code := paint(WORLD_LDTK, rid, "Climbable", 1, x, top, 32, bottom - top):
            return code
    for checked in (WORLD_LDTK, SANDBOX_LDTK):
        code, _ = run("validate", str(checked))
        if code:
            return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
