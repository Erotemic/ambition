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
- The Void, east of the bridge, black and white as the void's own things
  are (Jon, 2026-10-30: "a really cool void zone / dungeon set of rooms in
  the magical world. With the black / white theming of the void
  characters"): `magical_void_rift`, where the colour stops; under its west
  end `magical_void_gallery`, THINGS THE VOID HAS TAKEN; under its east end
  `magical_void_falls`, a shaft the void pours down; and at the bottom
  `magical_void_heart`, the first star that went out. Every room joins two
  others: rift-gallery, gallery-falls, falls-rift, falls-heart,
  heart-gallery.

- The Unlit, east of the friendly void, where the void is hostile (Jon,
  2026-10-30: "have some areas where the void creatures and void fighters
  are hostile"): `magical_void_ledges` east of the rift, broken ledges over
  nothing; down its hole `magical_void_coliseum`, a ring the void built out
  of the shape it remembers of a fight (two platforms over a floor); west of
  it at its floor `magical_void_wastes`, back to the falls' foot. A loop:
  rift, ledges, coliseum, wastes, falls, rift. Void stars here are enemies;
  so are the void fighters, the polygon fighters gone dark.

Mary-O meets the robot in the gate garden (Jon: "make sure maryo herself is
in the world"). The music is hers (Jon: "use more maryo songs here too ...
You can use billions and billions too").

The rooms sit where their edges meet on a flat map: the hollow's top is the
square's and the garden's ground, its shafts under their holes.
"""

from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "gate_network"))

from district_kit import WALL, WORLDS, breakable_wall, edge, ensure_world, entity, ledge, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
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
RIFT = "magical_void_rift"
GALLERY = "magical_void_gallery"
FALLS = "magical_void_falls"
HEART = "magical_void_heart"
NURSERY = "magical_void_nursery"
LEDGES = "magical_void_ledges"
COLISEUM = "magical_void_coliseum"
WASTES = "magical_void_wastes"

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
#: The Void: the rift east of the bridge; the gallery under its west end and
#: the heart under the gallery; the falls, a tall shaft, under its east end.
VOID_FLOOR, VOID_ROOF, VOID_H, VOID_BG = 576, 64, 640, "#050308"
VOID_BIOME = "void"
RIFT_AT, RIFT_W = (BRIDGE_AT[0] + KEEP_W, BRIDGE_AT[1]), 2560
RIFT_WEST_HOLE = (400, 464)
GALLERY_AT, GALLERY_W = (RIFT_AT[0], RIFT_AT[1] + VOID_H), 1792
GALLERY_HOLE = (1200, 1264)
HEART_AT = (RIFT_AT[0], GALLERY_AT[1] + VOID_H)
FALLS_AT, FALLS_W, FALLS_H, FALLS_FLOOR = (GALLERY_AT[0] + GALLERY_W, GALLERY_AT[1]), 768, 1280, 1216
#: The falls' top: under the rift's east hole (the falls' x).
FALLS_TOP = (408, 472)
RIFT_EAST_HOLE = (FALLS_TOP[0] + FALLS_AT[0] - RIFT_AT[0], FALLS_TOP[1] + FALLS_AT[0] - RIFT_AT[0])
#: The rift's pool: a cut in its floor where the void stands (touch it and
#: the room starts again), crossed on shards that crumble under a body and
#: come back.
POOL = (1880, 2160)
SHARDS = (1890, 1990, 2090)
#: The nursery: behind the heart's west wall (a wall that breaks), west of it.
NURSERY_W = 640
NURSERY_AT = (HEART_AT[0] - NURSERY_W, HEART_AT[1])
#: The Unlit: the ledges east of the rift (its floor the rift's), the
#: coliseum under the ledges' east end, the wastes west of the coliseum at
#: its floor (the falls' foot's floor).
LEDGES_AT, LEDGES_W = (RIFT_AT[0] + RIFT_W, RIFT_AT[1]), 4480
COLISEUM_W, COLISEUM_H, COLISEUM_FLOOR = 1920, 1280, 1216
COLISEUM_AT = (LEDGES_AT[0] + LEDGES_W - COLISEUM_W, LEDGES_AT[1] + VOID_H)
WASTES_AT, WASTES_W = (FALLS_AT[0] + FALLS_W, COLISEUM_AT[1] + COLISEUM_FLOOR - VOID_FLOOR), COLISEUM_AT[0] - (FALLS_AT[0] + FALLS_W)
#: Whether the void fighters' catalog rows have landed (07f85ece0: they
#: have).
VOID_FIGHTERS_READY = True
#: The ledges' hole down into the coliseum's roof (the coliseum's x).
COLISEUM_TOP = (928, 992)
LEDGES_HOLE = (COLISEUM_TOP[0] + COLISEUM_AT[0] - LEDGES_AT[0], COLISEUM_TOP[1] + COLISEUM_AT[0] - LEDGES_AT[0])
#: The falls' ledge from the gallery, on its west wall at the gallery's floor.
FALLS_SHELF_Y = GALLERY_AT[1] + VOID_FLOOR - FALLS_AT[1]


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


def ground_at(floor: int, h: int, x_from: int, x_to: int, holes: tuple[tuple[int, int], ...] = ()) -> list[str]:
    """The floor of a void room at `floor`, cut by `holes`."""
    out, x = [], x_from
    for i, (h0, h1) in enumerate(sorted(holes)):
        out.append(solid(f"ground_{i}", x, floor, h0 - x, h - floor))
        x = h1
    out.append(solid(f"ground_{len(holes)}", x, floor, x_to - x, h - floor))
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
        # Mary-O (her tall form: the cast of this world is taller than hers), by
        # the gate, to meet whoever comes through it.
        resident("mary_o_tall", "magical_mary_o", 860, FLOOR, h=64),
        sign("garden_welcome", 1000, 300, "THE STARLIGHT KINGDOM: please wipe your feet of void", 520),
        ledge("star_step_a", 960, 512, 160),
        ledge("star_step_b", 1180, 416, 128),
        *hole("garden", *HATCH, HOLLOW, "hollow_hatch_shaft", "garden_hatch", "down the hatch"),
        sign("garden_hatch_sign", HATCH[0] + 32, 560, "HATCH (stars below)", 220),
        edge("garden_east", "on to Starlight Square", w - 16, FLOOR, SQUARE, "square_west"),
    ]
    return rid, room(rid, e, at=GARDEN_AT, size=(w, H), biome="starlight", music="support_theme",
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
        # Mary-O's own Solid Snake, off duty in town (it wanders, as it does
        # in her levels; nobody stomps it here).
        resident("solid_snake", "magical_solid_snake", 1400, FLOOR, brain="", w=64, h=32),
        # Over the square, the snakes on a paper plane, going round.
        resident("npc_snakes_on_a_paper_plane", "magical_paper_plane_snakes", 2380, FLOOR - 24, h=32, w=48),
        # And her snake plant, rooted between the wand shop and the bakery.
        resident("snake_plant", "magical_snake_plant", 850, FLOOR, h=48),
        edge("square_east", "out of town, along the dune road", w - 16, FLOOR, ROAD, "road_west"),
    ]
    return rid, room(rid, e, at=SQUARE_AT, size=(w, H), biome="starlight", music="billions_and_billions",
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
    return rid, room(rid, e, at=HOLLOW_AT, size=(w, h), biome="starless", music="embedded_theme", bg="#1E1430",
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
        # Over the spilled cart, a swallow with a coconut.
        resident("laden_swallow", "magical_laden_swallow", 600, FLOOR - 24, h=40, w=54),
        sign("road_sign", 700, 360, "<- STARLIGHT SQUARE      THE CASTLE ->", 460),
        sign("road_castle", w - 520, 320, "THE CASTLE: taken by the void. Mary-O is on it. (She has a plan.)", 620),
        ledge("dune_a", 1200, 528, 192),
        ledge("dune_b", 1500, 448, 192),
        ledge("dune_c", 2200, 512, 256),
        slop(rid, 1, 1300, FLOOR),
        slop(rid, 2, 1900, FLOOR),
        slop(rid, 3, 2600, FLOOR),
    ]
    return rid, room(rid, e, at=ROAD_AT, size=(w, H), biome="stardune", music="sparse_theme", bg="#3A2850",
                     title="THE DUNE ROAD", generator=GENERATOR, dangerous=True)


def void_star(n: int, x: int, floor: int) -> str:
    """A void star: it wanders (its own Wanderer profile: no override), it
    talks (rudely), it is nobody's enemy until it is struck."""
    return resident("void_star", f"magical_void_star_{'ab'[n % 2]}", x, floor, brain="", h=48)


def keep():
    """The castle the void took: void stars loitering, the
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
        # The robot she rescues at the end of every castle, waiting to be
        # rescued from this one.
        resident("mary_o_robot", "magical_rescued_robot", 380, FLOOR, h=48),
        sign("keep_sign", 1310, 150, "THE VOID CASTLE: under new management", 480),
        ledge("keep_gallery_west", 860, 480, 280),
        ledge("keep_gallery_east", 1480, 480, 280),
        ledge("keep_gallery_top", 1160, 352, 300),
        *hole("keep_west", *STAIR_WEST, BRIDGE, "bridge_west_shaft", "keep_west_stair", "down the west stair"),
        *hole("keep_east", *STAIR_EAST, BRIDGE, "bridge_east_shaft", "keep_east_stair", "down the east stair"),
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
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        edge("bridge_east", "on, into the void", w - 16, g, RIFT, "rift_west"),
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
    return rid, room(rid, e, at=BRIDGE_AT, size=(w, h), biome="void_castle", music="castle_crash_override", bg="#140A20",
                     title="THE VOID LIZARD'S BRIDGE", generator=GENERATOR)


def shaft_up(zone, x0, x1, to_room, to_zone, name):
    """The way up out of a void room's roof: a gap 16 deep at its top."""
    return entity("LoadingZone", x0, 0, x1 - x0, 16, id=zone, name=name, activation="EdgeExit",
                  target_room=to_room, target_zone=to_zone, bidirectional=True)


def floor_hole(zone, x0, x1, floor, h, to_room, to_zone, name):
    """A hole in a void room's floor: a hatch to stand on, the way down at
    the room's bottom."""
    return [ledge(f"{zone}_hatch", x0, floor, x1 - x0),
            entity("LoadingZone", x0, h - 16, x1 - x0, 16, id=zone, name=name, activation="EdgeExit",
                   target_room=to_room, target_zone=to_zone, bidirectional=True)]


def void_room(rid, e, at, size, title, music, dangerous=False):
    return rid, room(rid, e, at=at, size=size, biome=VOID_BIOME, music=music, bg=VOID_BG, title=title, generator=GENERATOR,
                     dangerous=dangerous)


def void_fighter(rid: str, character: str, x: int, floor: int, brain: str = "") -> str:
    """A void fighter (the polygon fighters gone dark: `void_pointed_polygon`,
    `void_pugnacious_polygon`, `void_projectile_polygon`): an enemy that
    fights with its fighter's kit (the row's own Fighter policy drives it;
    `brain` only says it is hostile)."""
    return entity("EnemySpawn", x - 16, floor - 68, 32, 68, name=f"{rid}_{character}", brain=brain or "combatant",
                  character_id=character, respawn="OnRoomReenter")


def hostile_star(rid: str, n: int, x: int, floor: int) -> str:
    """A void star of the Unlit: an enemy, as in Mary-O's own levels (this
    game's row, `npc_void_star`, on her sheet)."""
    return entity("EnemySpawn", x - 23, floor - 46, 46, 46, name=f"{rid}_void_star_{n}", brain="combatant",
                  character_id="npc_void_star", respawn="OnRoomReenter")


def rift():
    """Where the colour stops: the castle's stone breaks into black shards;
    the town's things, half rubbed out, in white line. A hole down at each
    end."""
    rid, w, h, g, top = RIFT, RIFT_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [
        piece("rift_tear", 0, top),
        piece("rift_outlines", 760, 220),
        piece("rift_pool", POOL[0] - 4, g),
        *ground_at(g, h, 0, w, (RIFT_WEST_HOLE, POOL, RIFT_EAST_HOLE)),
        entity("HazardBlock", POOL[0], g + 24, POOL[1] - POOL[0], h - g - 24, name="the void, standing"),
        *[entity("BreakablePlatform", x, g, 60, 16, name=f"rift_shard_{i}", max_hp=1.0, collision="Solid", trigger="OnStand",
                 respawn="AfterSeconds", respawn_seconds=2.5, look="rift_shard") for i, x in enumerate(SHARDS)],
        # The high way over it, for those who would rather not stand on
        # anything the void can take.
        ledge("rift_pool_ledge", 1940, 470, 160),
        sign("rift_pool_sign", 2020, 380, "MIND THE VOID (it minds you)", 320),
        solid("roof", 0, 0, w, top),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        edge("rift_east", "on, into the Unlit", w - 16, g, LEDGES, "ledges_west"),
        edge("rift_west", "back to the lizard's bridge", 0, g, BRIDGE, "bridge_east"),
        start(rid, 200, g),
        sign("rift_sign", 1240, 120, "THE VOID: colour stops here. Please leave yours at the door.", 640),
        ledge("rift_shard_a", 900, 448, 160),
        ledge("rift_shard_b", 1300, 368, 200),
        ledge("rift_shard_c", 1760, 448, 160),
        *floor_hole("rift_down_west", *RIFT_WEST_HOLE, g, h, GALLERY, "gallery_up", "down into the gallery"),
        *floor_hole("rift_down_east", *RIFT_EAST_HOLE, g, h, FALLS, "falls_top", "down the falls"),
        void_star(5, 1000, g),
        void_star(6, 1600, g),
    ]
    return void_room(rid, e, RIFT_AT, (w, h), "THE VOID", "embedded_theme")


def gallery():
    """THINGS THE VOID HAS TAKEN, on plinths, and a void star who minds them.
    Up to the rift; down to the heart; east to the falls."""
    rid, w, h, g, top = GALLERY, GALLERY_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [
        piece("gallery_exhibits", 500, 200),
        *ground_at(g, h, 0, w, (GALLERY_HOLE,)),
        solid("roof_west", 0, 0, RIFT_WEST_HOLE[0], top),
        solid("roof_east", RIFT_WEST_HOLE[1], 0, w - RIFT_WEST_HOLE[1], top),
        solid("west_wall", 0, top, WALL, g - top),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        shaft_up("gallery_up", *RIFT_WEST_HOLE, RIFT, "rift_down_west", "up to the rift"),
        *floor_hole("gallery_down", *GALLERY_HOLE, g, h, HEART, "heart_up", "down to the heart"),
        edge("gallery_east", "on to the falls", w - 16, g, FALLS, "falls_west"),
        start(rid, RIFT_WEST_HOLE[0] + 120, g),
        sign("gallery_sign", 1100, 110, "THINGS THE VOID HAS TAKEN (please do not touch: they are not there)", 700),
        sign("gallery_lamp", 620, 300, "A STAR LAMP. Switched off.", 240),
        sign("gallery_wheel", 900, 340, "A CART'S WHEEL. Rolled off.", 260),
        sign("gallery_moon", 1440, 340, "A CRESCENT. Bitten.", 220),
        sign("gallery_wand", 1640, 300, "A WAND. Bent.", 180),
        resident("void_star", "magical_void_curator", 760, g, brain="stand_still", h=48),
        void_star(7, 1500, g),
    ]
    return void_room(rid, e, GALLERY_AT, (w, h), "THE GALLERY OF THE VOID", "sparse_theme")


#: The falls' ledges, zigzag down (x, y, w): one under the ladder from the
#: rift, none more than a jump from the next.
FALLS_LEDGES = ((360, 200, 180), (180, 300, 180), (300, 400, 180), (160, 490, 180), (220, 680, 180),
                (380, 790, 180), (200, 900, 180), (360, 1010, 180), (180, 1110, 180))


def falls():
    """A shaft the void pours down: in at its top from the rift, a ledge on
    its west wall from the gallery, out at its foot to the heart."""
    rid, w, h, g, top = FALLS, FALLS_W, FALLS_H, FALLS_FLOOR, VOID_ROOF
    shelf = FALLS_SHELF_Y
    e = [
        piece("falls_cascade", 540, top),
        solid("floor", 0, g, w, h - g),
        solid("roof_west", 0, 0, FALLS_TOP[0], top),
        solid("roof_east", FALLS_TOP[1], 0, w - FALLS_TOP[1], top),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        edge("falls_east", "into the wastes", w - 16, g, WASTES, "wastes_west"),
        solid("west_wall_top", 0, top, WALL, shelf - 96 - top),
        solid("west_shelf", 0, shelf, 200, 32),
        solid("west_wall_bottom", 0, shelf + 32, WALL, g - 96 - shelf - 32),
        shaft_up("falls_top", *FALLS_TOP, RIFT, "rift_down_east", "up to the rift"),
        edge("falls_west", "back to the gallery", 0, shelf, GALLERY, "gallery_east"),
        edge("falls_foot", "on to the heart", 0, g, HEART, "heart_east"),
        *[ledge(f"falls_ledge_{i}", x, y, lw) for i, (x, y, lw) in enumerate(FALLS_LEDGES)],
        start(rid, 300, g),
        sign("falls_sign", 384, 140, "THE FALLS: the void goes down. So can you.", 420),
        entity("ChestSpawn", 420, g - 58, 76, 58, name="a thing the void dropped", reward="currency:90"),
    ]
    return void_room(rid, e, FALLS_AT, (w, h), "THE FALLS", "embedded_theme")


def heart():
    """The bottom of the void: the first star that went out, asleep, as big
    as the room; the eldest of the void stars beside it, who talks."""
    rid, w, h, g, top = HEART, GALLERY_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [
        piece("heart_black_star", 420, top),
        solid("floor", 0, g, w, h - g),
        solid("roof_west", 0, 0, GALLERY_HOLE[0], top),
        solid("roof_east", GALLERY_HOLE[1], 0, w - GALLERY_HOLE[1], top),
        # The west wall is a wall above, and below a wall that breaks (a
        # secret: the nursery is behind it).
        solid("west_wall", 0, top, WALL, g - 96 - top),
        breakable_wall("heart_nursery_wall", WALL, g - 96, 32, 96, look="void_crack_wall", hp=3, name="a crack in the dark"),
        edge("heart_west", "through the crack", 0, g, NURSERY, "nursery_east"),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        shaft_up("heart_up", *GALLERY_HOLE, GALLERY, "gallery_down", "up to the gallery"),
        edge("heart_east", "back to the falls", w - 16, g, FALLS, "falls_foot"),
        start(rid, w - 200, g),
        sign("heart_sign", 700, 40 + top, "THE HEART OF THE VOID: the first star that went out. Shh.", 600),
        ledge("heart_point_west", 200, 448, 192),
        ledge("heart_point_east", 1000, 400, 160),
        resident("void_star", "magical_void_elder", 1000, g, brain="stand_still", h=48),
        entity("ChestSpawn", 1500, g - 58, 76, 58, name="a light the void could not take", reward="currency:150"),
    ]
    return void_room(rid, e, HEART_AT, (w, h), "THE HEART OF THE VOID", "eternal_cosmic")


def nursery():
    """A secret, behind the heart's west wall: where a star goes the first
    time it goes out. Cradles; a void star who rocks them; a chest."""
    rid, w, h, g, top = NURSERY, NURSERY_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [
        piece("nursery_cradles", 60, top),
        solid("floor", 0, g, w, h - g),
        solid("roof", 0, 0, w, top),
        solid("west_wall", 0, top, WALL, g - top),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        edge("nursery_east", "back out to the heart", w - 16, g, HEART, "heart_west"),
        start(rid, w - 120, g),
        sign("nursery_sign", 320, 400, "THE NURSERY. Lights off, please.", 360),
        resident("void_star", "magical_void_nurse", 230, g, brain="stand_still", h=48),
        entity("ChestSpawn", 400, g - 58, 76, 58, name="a night light, never used", reward="currency:200"),
    ]
    return void_room(rid, e, NURSERY_AT, (w, h), "THE NURSERY", "eternal_cosmic")


#: The ledges' broken walk: (x, w) pieces of floor at the room's floor,
#: nothing between them but the void (it starts the room again).
LEDGES_FLOOR = ((0, 520), (680, 300), (1140, 260), (1560, 420), (2140, 420), (2720, 380), (3260, 1220))


def ledges():
    """East of the rift: a broken walk over nothing, hostile. Void stars on
    the pieces; one high ledge over each gap; the hole down to the coliseum."""
    rid, w, h, g, top = LEDGES, LEDGES_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [solid("roof", 0, 0, w, top), solid("east_wall", w - WALL, top, WALL, g - top)]
    hole = LEDGES_HOLE
    for i, (x, pw) in enumerate(LEDGES_FLOOR):
        # Each piece of floor, cut by the coliseum's hole where it crosses.
        if x < hole[0] < x + pw:
            e += [solid(f"piece_{i}a", x, g, hole[0] - x, h - g), solid(f"piece_{i}b", hole[1], g, x + pw - hole[1], h - g)]
        else:
            e.append(solid(f"piece_{i}", x, g, pw, h - g))
    for i, ((x0, w0), (x1, _)) in enumerate(zip(LEDGES_FLOOR, LEDGES_FLOOR[1:])):
        gap0, gap1 = x0 + w0, x1
        e.append(entity("HazardBlock", gap0, g + 24, gap1 - gap0, h - g - 24, name=f"the void, gap {i}"))
        # A post at each lip, as Mary-O's levels keep a walker on its walk:
        # the void stars turn at it instead of walking off into the gap.
        e += [solid(f"post_{i}_west", gap0 - 12, g - 20, 12, 20), solid(f"post_{i}_east", gap1, g - 20, 12, 20)]
        e.append(ledge(f"over_gap_{i}", gap0 - 40, g - 110, gap1 - gap0 + 80))
        # Its chains to the roof, and the castle falling away under it
        # (`magical_unlit_props.py` `ledges_gap`, drawn at the first gap).
        e.insert(0, piece("ledges_gap", gap0 - 44, top, name=f"ledges_gap_{i}"))
    e += [
        *floor_hole("ledges_down", *hole, g, h, COLISEUM, "coliseum_top", "down into the ring"),
        edge("ledges_west", "back to the rift", 0, g, RIFT, "rift_east"),
        start(rid, 200, g),
        sign("ledges_sign", 1300, 120, "THE UNLIT: here the void does not ask your name", 560),
        hostile_star(rid, 1, 800, g),
        hostile_star(rid, 2, 1650, g),
        hostile_star(rid, 3, 2300, g),
        hostile_star(rid, 4, 2900, g),
        hostile_star(rid, 5, 3900, g),
        # A zoner on the long last piece, where it has room to shoot.
        *([void_fighter(rid, "void_projectile_polygon", 4100, g)] if VOID_FIGHTERS_READY else []),
    ]
    return void_room(rid, e, LEDGES_AT, (w, h), "THE UNLIT", "castle_crash_override", dangerous=True)


def coliseum():
    """Under the ledges: a ring the void built out of the shape it remembers
    of a fight. A floor, two platforms over it and one over them. In from the
    roof; out at the floor, west, to the wastes."""
    rid, w, h, g, top = COLISEUM, COLISEUM_W, COLISEUM_H, COLISEUM_FLOOR, VOID_ROOF
    e = [
        # The stands and their crowd (`targets/props/magical_unlit_props.py`).
        piece("coliseum_stand_west", 30, 720),
        piece("coliseum_stand_east", 1880 - 440 - 10, 720),
        piece("ring_seat", 960 - 70, g - 290 - 212),
        solid("floor", 0, g, w, h - g),
        solid("roof_west", 0, 0, COLISEUM_TOP[0], top),
        solid("roof_east", COLISEUM_TOP[1], 0, w - COLISEUM_TOP[1], top),
        solid("east_wall", w - WALL, top, WALL, g - top),
        solid("west_wall", 0, top, WALL, g - 96 - top),
        shaft_up("coliseum_top", *COLISEUM_TOP, LEDGES, "ledges_down", "up to the Unlit"),
        edge("coliseum_west", "out to the wastes", 0, g, WASTES, "wastes_east"),
        start(rid, 200, g),
        # Its stage: the floor, two side platforms, a top one (one-way).
        ledge("ring_left", 560, g - 150, 240),
        ledge("ring_right", 1120, g - 150, 240),
        ledge("ring_top", 840, g - 290, 240),
        # The ring's purse, on the top platform, and who holds it.
        entity("ChestSpawn", 922, g - 290 - 58, 76, 58, name="the ring's purse", reward="currency:250"),
        sign("coliseum_champion", 960, g - 420, "CHAMPION OF THE UNLIT RING: (vacant)", 420),
        # The stair down from the roof's hole: steps of one-way.
        *[ledge(f"ring_step_{i}", 860 - (i % 2) * 220, 220 + i * 150, 200) for i in range(5)],
        sign("coliseum_sign", 960, 140, "THE UNLIT RING: the void learns to fight from the shapes that fought it", 760),
        hostile_star(rid, 1, 300, g),
        hostile_star(rid, 2, 1600, g),
        # The ring's three: the sword, the fists, the shooter.
        *([void_fighter(rid, "void_pointed_polygon", 700, g), void_fighter(rid, "void_pugnacious_polygon", 1240, g),
           void_fighter(rid, "void_projectile_polygon", 680, g - 150)] if VOID_FIGHTERS_READY else []),
    ]
    return void_room(rid, e, COLISEUM_AT, (w, h), "THE UNLIT RING", "invincible_maryo", dangerous=True)


def wastes():
    """West of the ring, at its floor: open dark to the falls' foot. Void
    stars hunt here."""
    rid, w, h, g, top = WASTES, WASTES_W, VOID_H, VOID_FLOOR, VOID_ROOF
    e = [
        piece("wastes_dead_things", 300, g - 340, px=1.5),
        solid("floor", 0, g, w, h - g),
        solid("roof", 0, 0, w, top),
        edge("wastes_west", "back to the falls", 0, g, FALLS, "falls_east"),
        edge("wastes_east", "on to the ring", w - 16, g, COLISEUM, "coliseum_west"),
        solid("west_wall", 0, top, WALL, g - 96 - top),
        solid("east_wall", w - WALL, top, WALL, g - 96 - top),
        start(rid, 200, g),
        sign("wastes_sign", w // 2, 120, "THE WASTES: nothing for miles, and it is all looking at you", 620),
        ledge("waste_rise_a", w // 4, g - 120, 200),
        ledge("waste_rise_b", w // 2, g - 200, 200),
        ledge("waste_rise_c", 3 * w // 4, g - 120, 200),
        hostile_star(rid, 1, w // 4 + 100, g),
        hostile_star(rid, 2, w // 2 + 100, g),
        hostile_star(rid, 3, 3 * w // 4 + 100, g),
        *([void_fighter(rid, "void_pugnacious_polygon", w // 2 + 400, g)] if VOID_FIGHTERS_READY else []),
        entity("ChestSpawn", w // 2 + 60, g - 200 - 58, 76, 58, name="what the void kept for itself", reward="currency:180"),
    ]
    return void_room(rid, e, WASTES_AT, (w, h), "THE WASTES", "sparse_theme", dangerous=True)


ROOMS = (garden, square, hollow, road, keep, bridge, rift, gallery, falls, heart, nursery, ledges, coliseum, wastes)


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
                 *((BRIDGE, "Climbable", 1, x + 16, 0, 32, BRIDGE_FLOOR) for x, _ in (STAIR_WEST, STAIR_EAST)),
                 *((RIFT, "Climbable", 1, x + 16, VOID_FLOOR - 64, 32, VOID_H - VOID_FLOOR + 64) for x, _ in (RIFT_WEST_HOLE, RIFT_EAST_HOLE)),
                 (GALLERY, "Climbable", 1, RIFT_WEST_HOLE[0] + 16, 0, 32, VOID_FLOOR),
                 (GALLERY, "Climbable", 1, GALLERY_HOLE[0] + 16, VOID_FLOOR - 64, 32, VOID_H - VOID_FLOOR + 64),
                 (HEART, "Climbable", 1, GALLERY_HOLE[0] + 16, 0, 32, VOID_FLOOR),
                 (FALLS, "Climbable", 1, FALLS_TOP[0] + 16, 0, 32, 216),
                 (LEDGES, "Climbable", 1, LEDGES_HOLE[0] + 16, VOID_FLOOR - 64, 32, VOID_H - VOID_FLOOR + 64),
                 (COLISEUM, "Climbable", 1, COLISEUM_TOP[0] + 16, 0, 32, 236)):
        if code := paint(WORLD, *args):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
