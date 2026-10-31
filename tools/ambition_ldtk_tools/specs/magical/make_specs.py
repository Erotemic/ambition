#!/usr/bin/env python3
"""The Magical dimension (Mary-O's), its first rooms: `magical.ldtk`.

    python specs/magical/make_specs.py          # writes the specs
    python specs/magical/make_specs.py --apply  # and makes the world and rooms

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "I want you to start authoring the maryo-dimension for the
ambition game. I.e. the magical dimension." It is Mary-O's land as the robot
finds it by the gate (address DESTINY): the colours of a magical girl and the
shapes of an old platform game, in Mary-O's own biomes (`starlight` above,
`starless` under the ground, `stardune` out of town; the castle the void took,
`void_castle`, on the horizon).

One region, not a string of rooms (`docs/planning/game/world-composition.md`):

- `magical_gate_garden`: the arrival, the ring among the round hills.
- `magical_starlight_square`: east of the garden by a side exit, the town's
  square; its old well goes down.
- `magical_starless_hollow`: under the garden and the square, where the stars
  go when nobody is looking. Down the well from the square, along under the
  ground (AI Slop live here), up a ladder through a hatch into the garden: a
  loop.
- `magical_stardune_road`: east of the square by a side exit, out of town,
  toward the castle; more AI Slop.
- `magical_void_keep`: east of the road by a side exit, the castle the void
  took. Mary-O is here, at a truce. Void stars loiter; not friends, not foes
  (Jon, 2026-10-30: "he and the void stars are not quite peaceful, but at
  least not hostile npcs. You can talk to them."). Two stairs down.
- `magical_void_bridge`: under the keep, the Void Lizard's bridge over the
  lava, its switch at the east end. Down one stair, along, up the other: a
  loop. The lizard is not an enemy here; it talks, and minds its switch.

The rooms sit where their edges meet on a flat map: the hollow's top is the
square's and the garden's ground, its shafts under their holes.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "gate_network"))

from district_kit import WALL, WORLDS, edge, ensure_world, entity, ledge, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
import gates  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "magical.ldtk"
GENERATOR = "specs/magical/make_specs.py"
UID_OFFSET = 1000000

GARDEN = "magical_gate_garden"
SQUARE = "magical_starlight_square"
HOLLOW = "magical_starless_hollow"
ROAD = "magical_stardune_road"
KEEP = "magical_void_keep"
BRIDGE = "magical_void_bridge"

FLOOR = 640
H = 768
GARDEN_AT, GARDEN_W = (0, 0), 2048
SQUARE_AT, SQUARE_W = (GARDEN_W, 0), 2560
ROAD_AT, ROAD_W = (GARDEN_W + SQUARE_W, 0), 3200
HOLLOW_AT, HOLLOW_W, HOLLOW_H = (1024, H), 3072, 640
HOLLOW_FLOOR, HOLLOW_ROOF = 576, 64
#: The garden's hatch and the square's well, each a 64-wide hole in its
#: ground over a shaft of the hollow (room x).
HATCH = (1376, 1440)
WELL = (1600, 1664)
#: The Void Castle: the keep east of the road, its bridge under it; the two
#: stairs down from the keep's ground (both rooms' x: they share a left edge).
KEEP_AT, KEEP_W = (ROAD_AT[0] + ROAD_W, 0), 2560
BRIDGE_AT, BRIDGE_H = (KEEP_AT[0], H), 640
BRIDGE_FLOOR, BRIDGE_ROOF, PIT_FLOOR = 576, 64, 632
SPAN = (760, 1800)
STAIR_WEST = (480, 544)
STAIR_EAST = (2000, 2064)


def in_hollow(x: int, room_at: tuple[int, int]) -> int:
    """A room's x, as the hollow's."""
    return x + room_at[0] - HOLLOW_AT[0]


def hole(name: str, x0: int, x1: int, to_room: str, to_zone: str, zone: str, label: str) -> list[str]:
    """A hole down through a room's ground: a hatch over it to stand on (drop
    through with down), and the way down at its bottom."""
    return [
        ledge(f"{name}_hatch", x0, FLOOR, x1 - x0),
        entity("LoadingZone", x0, H - 16, x1 - x0, 16, id=zone, name=label, activation="EdgeExit",
               target_room=to_room, target_zone=to_zone, bidirectional=True),
    ]


def ground(x_from: int, x_to: int, holes: tuple[tuple[int, int], ...] = ()) -> list[str]:
    """The ground from `x_from` to `x_to`, cut by `holes`."""
    out, x = [], x_from
    for i, (h0, h1) in enumerate(sorted(holes)):
        out.append(solid(f"ground_{i}", x, FLOOR, h0 - x, H - FLOOR))
        x = h1
    out.append(solid(f"ground_{len(holes)}", x, FLOOR, x_to - x, H - FLOOR))
    return out


def slop(rid: str, n: int, x: int, floor: int) -> str:
    return entity("EnemySpawn", x - 18, floor - 36, 36, 36, name=f"{rid}_npc_ai_slop_{n}", brain="combatant",
                  character_id="npc_ai_slop", respawn="OnRoomReenter")


def garden():
    """The arrival: the ring among the round hills, its terminal, the hatch
    down to the hollow."""
    rid, w = GARDEN, GARDEN_W
    e = [
        # The garden's things (`targets/props/magical_props.py`), first.
        piece("garden_beds", 56, FLOOR - 60),
        piece("garden_hill_low", 880, 508),
        piece("garden_hill_high", 1100, 412),
        piece("garden_hatch", HATCH[0] - 64, FLOOR - 92),
        piece("garden_star_tree", 1530, FLOOR - 400),
        *ground(0, w, (HATCH,)),
        solid("west_wall", 0, 0, WALL, FLOOR),
        *gates.as_ron(gates.ring("Starlight Gate", "magical_gate_zone", 360, FLOOR, "the gate")),
        *gates.as_ron(gates.terminal("Starlight Gate", "magical_gate_terminal", 690, FLOOR)),
        start(rid, 220, FLOOR),
        sign("garden_welcome", 1000, 300, "THE STARLIGHT KINGDOM: please wipe your feet of void", 520),
        ledge("star_step_a", 960, 512, 160),
        ledge("star_step_b", 1180, 416, 128),
        *hole("garden", *HATCH, HOLLOW, "hollow_hatch_shaft", "garden_hatch", "down the hatch"),
        sign("garden_hatch_sign", HATCH[0] + 32, 560, "HATCH (stars below)", 220),
        edge("garden_east", "on to Starlight Square", w - 16, FLOOR, SQUARE, "square_west"),
    ]
    return rid, room(rid, e, at=GARDEN_AT, size=(w, H), biome="starlight", music="it_was_revealed_to_me_in_a_dream",
                     bg="#F4C8E0", title="THE GATE GARDEN", generator=GENERATOR)


def square():
    """The town's square, and its old well, which goes down."""
    rid, w = SQUARE, SQUARE_W
    e = [
        # The fronts (`targets/props/magical_square_props.py`), first: they
        # draw behind everything the room lists after them.
        piece("square_wand_shop", 410, 180),
        piece("square_bakery", 910, 170),
        piece("square_well", 1540, 410),
        piece("square_house", 1880, 110),
        *ground(0, w, (WELL,)),
        edge("square_west", "back to the gate garden", 0, FLOOR, GARDEN, "garden_east"),
        start(rid, 200, FLOOR),
        sign("square_wands", 600, 150, "WAND REPAIR: closed (out fighting the void)", 440),
        sign("square_bakery", 1100, 130, "STAR BAKERY: crescents, half moons, full moons on Sundays", 520),
        ledge("square_awning_a", 480, 480, 240),
        ledge("square_awning_b", 980, 480, 240),
        ledge("square_balcony", 1900, 448, 288),
        *hole("square_well", *WELL, HOLLOW, "hollow_well_shaft", "square_well", "down the old well"),
        sign("square_well_sign", WELL[0] + 32, 540, "THE OLD WELL: wishes not refunded", 340),
        resident("npc_magical_celeste", "magical_celeste", 1160, FLOOR),
        resident("npc_magical_lumen", "magical_lumen", 2060, FLOOR),
        edge("square_east", "out of town, along the dune road", w - 16, FLOOR, ROAD, "road_west"),
    ]
    return rid, room(rid, e, at=SQUARE_AT, size=(w, H), biome="starlight", music="it_was_revealed_to_me_in_a_dream",
                     bg="#F4C8E0", title="STARLIGHT SQUARE", generator=GENERATOR)


def hollow():
    """Under the garden and the square: where the stars go when nobody is
    looking. A shaft up to each hole."""
    rid, w, h, g, top = HOLLOW, HOLLOW_W, HOLLOW_H, HOLLOW_FLOOR, HOLLOW_ROOF
    hatch = (in_hollow(HATCH[0], GARDEN_AT), in_hollow(HATCH[1], GARDEN_AT))
    well = (in_hollow(WELL[0], SQUARE_AT), in_hollow(WELL[1], SQUARE_AT))
    e = [
        # Where the stars sleep (`targets/props/magical_props.py`), first.
        piece("hollow_shaft_light_west", hatch[0] - 100, 0),
        piece("hollow_shaft_light_east", well[0] - 100, 0),
        piece("hollow_star_roots", 700, top),
        piece("hollow_star_heap", 1100, g - 140),
        piece("hollow_star_heap_east", 2150, g - 140),
        solid("floor", 0, g, w, h - g),
        solid("roof_west", 0, 0, hatch[0], top),
        solid("roof_middle", hatch[1], 0, well[0] - hatch[1], top),
        solid("roof_east", well[1], 0, w - well[1], top),
        solid("west_wall", 0, top, WALL, g - top),
        solid("east_wall", w - WALL, top, WALL, g - top),
        entity("LoadingZone", hatch[0], 0, hatch[1] - hatch[0], 16, id="hollow_hatch_shaft", name="up to the gate garden",
               activation="EdgeExit", target_room=GARDEN, target_zone="garden_hatch", bidirectional=True),
        entity("LoadingZone", well[0], 0, well[1] - well[0], 16, id="hollow_well_shaft", name="up the old well",
               activation="EdgeExit", target_room=SQUARE, target_zone="square_well", bidirectional=True),
        start(rid, well[0] - 120, g),
        sign("hollow_sign", 1500, 120, "THE STARLESS HOLLOW: where the stars go when nobody is looking", 600),
        ledge("hollow_ledge_a", 900, 448, 192),
        ledge("hollow_ledge_b", 1400, 384, 224),
        ledge("hollow_ledge_c", 1900, 448, 192),
        entity("ChestSpawn", 1474, 384 - 58, 76, 58, name="a fallen star, in a box", reward="currency:80"),
        slop(rid, 1, 800, g),
        slop(rid, 2, 1600, g),
        slop(rid, 3, 2300, g),
    ]
    return rid, room(rid, e, at=HOLLOW_AT, size=(w, h), biome="starless", music="vault_in_the_dark", bg="#1E1430",
                     title="THE STARLESS HOLLOW", generator=GENERATOR, dangerous=True)


def road():
    """Out of town on the dunes, toward the castle the void took."""
    rid, w = ROAD, ROAD_W
    e = [
        # The dunes and what was left on the road (`targets/props/magical_props.py`), first.
        piece("road_dunes", 1080, 444),
        piece("road_dune_far", 2080, 508),
        piece("road_cart", 380, FLOOR - 90),
        piece("road_lamp", 2900, FLOOR - 190),
        *ground(0, w),
        edge("road_west", "back to Starlight Square", 0, FLOOR, SQUARE, "square_east"),
        edge("road_east", "into the Void Castle", w - 16, FLOOR, KEEP, "keep_west"),
        start(rid, 200, FLOOR),
        sign("road_sign", 700, 360, "<- STARLIGHT SQUARE      THE CASTLE ->", 460),
        sign("road_castle", w - 520, 320, "THE CASTLE: taken by the void. Mary-O is on it. (She has a plan.)", 620),
        ledge("dune_a", 1200, 528, 192),
        ledge("dune_b", 1500, 448, 192),
        ledge("dune_c", 2200, 512, 256),
        slop(rid, 1, 1300, FLOOR),
        slop(rid, 2, 1900, FLOOR),
        slop(rid, 3, 2600, FLOOR),
    ]
    return rid, room(rid, e, at=ROAD_AT, size=(w, H), biome="stardune", music="pulse_drift_voyage", bg="#3A2850",
                     title="THE DUNE ROAD", generator=GENERATOR, dangerous=True)


def void_star(n: int, x: int, floor: int) -> str:
    """A void star: it wanders (its own Wanderer profile: no override), it
    talks (rudely), it is nobody's enemy until it is struck."""
    return resident("void_star", f"magical_void_star_{'ab'[n % 2]}", x, floor, brain="", h=48)


def keep():
    """The castle the void took: Mary-O at a truce, void stars loitering, the
    lizard's banners; a stair down at each end of the hall."""
    rid, w = KEEP, KEEP_W
    e = [
        # The keep's things (`targets/props/magical_void_props.py`), first.
        piece("keep_gate", 0, 150),
        piece("keep_banner", 640, 82, name="keep_banner_west"),
        piece("keep_banner", 1640, 82, name="keep_banner_middle"),
        piece("keep_banner", 2240, 82, name="keep_banner_east"),
        piece("keep_star_statue", 1220, FLOOR - 240),
        piece("keep_stair", STAIR_WEST[0] - 32, FLOOR - 76, name="keep_stair_west"),
        piece("keep_stair", STAIR_EAST[0] - 32, FLOOR - 76, name="keep_stair_east"),
        *ground(0, w, (STAIR_WEST, STAIR_EAST)),
        solid("east_wall", w - WALL, 0, WALL, FLOOR),
        edge("keep_west", "back out to the dune road", 0, FLOOR, ROAD, "road_east"),
        start(rid, 260, FLOOR),
        sign("keep_sign", 1310, 150, "THE VOID CASTLE: under new management", 480),
        ledge("keep_gallery_west", 860, 480, 280),
        ledge("keep_gallery_east", 1480, 480, 280),
        ledge("keep_gallery_top", 1160, 352, 300),
        *hole("keep_west", *STAIR_WEST, BRIDGE, "bridge_west_shaft", "keep_west_stair", "down the west stair"),
        *hole("keep_east", *STAIR_EAST, BRIDGE, "bridge_east_shaft", "keep_east_stair", "down the east stair"),
        resident("mary_o", "magical_mary_o", 760, FLOOR, h=48),
        void_star(0, 1000, FLOOR),
        void_star(1, 1700, FLOOR),
        void_star(2, 2300, FLOOR),
    ]
    return rid, room(rid, e, at=KEEP_AT, size=(w, H), biome="void_castle", music="castle_crash_override",
                     bg="#140A20", title="THE VOID CASTLE", generator=GENERATOR)


def bridge():
    """Under the keep: the Void Lizard's bridge over the lava, its tea table,
    its switch. A stair up at each end."""
    rid, w, h, g, top = BRIDGE, KEEP_W, BRIDGE_H, BRIDGE_FLOOR, BRIDGE_ROOF
    x0, x1 = SPAN
    e = [
        # The bridge's things (`targets/props/magical_void_props.py`), first.
        piece("bridge_span", x0 - 10, g - 70),
        piece("bridge_tea_table", 1012, g - 70),
        piece("bridge_switch", 1790, g - 64),
        solid("floor_west", 0, g, x0, h - g),
        solid("floor_east", x1, g, w - x1, h - g),
        solid("bridge_deck", x0, g, x1 - x0, 16),
        solid("pit_floor", x0, PIT_FLOOR, x1 - x0, h - PIT_FLOOR),
        solid("roof_west", 0, 0, STAIR_WEST[0], top),
        solid("roof_middle", STAIR_WEST[1], 0, STAIR_EAST[0] - STAIR_WEST[1], top),
        solid("roof_east", STAIR_EAST[1], 0, w - STAIR_EAST[1], top),
        solid("west_wall", 0, top, WALL, g - top),
        solid("east_wall", w - WALL, top, WALL, g - top),
        entity("LoadingZone", STAIR_WEST[0], 0, STAIR_WEST[1] - STAIR_WEST[0], 16, id="bridge_west_shaft", name="up the west stair",
               activation="EdgeExit", target_room=KEEP, target_zone="keep_west_stair", bidirectional=True),
        entity("LoadingZone", STAIR_EAST[0], 0, STAIR_EAST[1] - STAIR_EAST[0], 16, id="bridge_east_shaft", name="up the east stair",
               activation="EdgeExit", target_room=KEEP, target_zone="keep_east_stair", bidirectional=True),
        start(rid, STAIR_WEST[0] - 120, g),
        sign("bridge_sign", 1280, 130, "THE VOID LIZARD'S BRIDGE: wipe your feet, mind the switch", 600),
        sign("bridge_switch_sign", 1900, 470, "DO NOT LAND ON", 200),
        resident("void_lizard", "magical_void_lizard", 1300, g, w=92, h=64),
        void_star(3, 400, g),
        void_star(4, 2300, g),
    ]
    return rid, room(rid, e, at=BRIDGE_AT, size=(w, h), biome="void_castle", music="phrygian_furnace", bg="#140A20",
                     title="THE VOID LIZARD'S BRIDGE", generator=GENERATOR)


ROOMS = (garden, square, hollow, road, keep, bridge)


def main(argv):
    paths = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    if code := ensure_world(WORLD, UID_OFFSET):
        return code
    if code := make_rooms(WORLD, paths):
        return code
    # The ladders: up each shaft of the hollow and on up through its hole.
    hatch0 = in_hollow(HATCH[0], GARDEN_AT)
    well0 = in_hollow(WELL[0], SQUARE_AT)
    for args in ((GARDEN, "Climbable", 1, HATCH[0] + 16, FLOOR - 64, 32, H - FLOOR + 64),
                 (SQUARE, "Climbable", 1, WELL[0] + 16, FLOOR - 64, 32, H - FLOOR + 64),
                 (HOLLOW, "Climbable", 1, hatch0 + 16, 0, 32, HOLLOW_FLOOR),
                 (HOLLOW, "Climbable", 1, well0 + 16, 0, 32, HOLLOW_FLOOR),
                 *((KEEP, "Climbable", 1, x + 16, FLOOR - 64, 32, H - FLOOR + 64) for x, _ in (STAIR_WEST, STAIR_EAST)),
                 *((BRIDGE, "Climbable", 1, x + 16, 0, 32, BRIDGE_FLOOR) for x, _ in (STAIR_WEST, STAIR_EAST))):
        if code := paint(WORLD, *args):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
