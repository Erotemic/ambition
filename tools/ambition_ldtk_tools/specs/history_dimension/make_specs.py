#!/usr/bin/env python3
"""Write the rooms of the History dimension (`docs/planning/game/history-dimension.md`).

    python specs/history_dimension/make_specs.py          # writes the specs
    python specs/history_dimension/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

A grid. East is later: each column is an age. Each floor is a region of the
world, so the rooms of one column are contemporaneous. A way up and down (a
hole in the upper room's floor, over a tower of ledges in the lower room)
joins two floors of one column only where the two regions were in contact
then. There are no ladders: Jon (2026-11-04) asked for platforms and rooms
in levels, and a ladder is slow. The gate (DYNASTIC, `data/gate_network.ron`) is in the Cradle, at the bottom left:
everyone starts in Africa (`specs/check_flat_map.py history.ldtk`).

             C0       C1            C2              C3           C4          C5          C6     C7
    R0 Americas       beringia ┐    maize           olmecs       maya        taino       haiti
    R½ S. America     beringia ┘    andes           caral
    R1 East Asia      mammoth st.   yellow river    shang        silk road   steppe
    R2 Oceania        sahul         highlands       lapita       wayfinders  rapa nui
    R2½ SE Asia       sulawesi      austronesians   dong son
    R3 South Asia     south. route  mehrgarh        indus
    R4 Europe         ice age       first farmers   megaliths    celts       vikings     sail   industrial
    R5 Med./SW Asia   levant        crescent        uruk         classical
    R6 Africa cradle  south. coast  green sahara    nile         aksum       timbuktu    benin  the fort

The ways through, room by room, are in each room's function. Buildings are
under the grid, reached by doors. The art is the sprite tools'
`targets/props/history_*_props.py` and `targets/characters/` (the cast). A
prop sheet that is 2 frame px to a world unit is placed at half its frame.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "gate_network"))

from district_kit import WALL, WORLDS, edge, ensure_world, entity, door, ledge, make_rooms, pad, prop, resident, room, run, sign, solid, start, water  # noqa: E402

import gates  # noqa: E402
from blurbs import BLURB  # noqa: E402

WORLD = WORLDS / "history.ldtk"
GENERATOR = "specs/history_dimension/make_specs.py"
UID_OFFSET = 1200000
#: The size of one cell of the grid. A cell is one screen and a bit high, so
#: that a room is a level to cross, not a shaft to climb.
W, H = 2800, 640

# ---- The grid ----------------------------------------------------------------

#: The rooms of the grid, by (column, row). Column 0 is the oldest; row 0 is
#: the top floor.
GRID = {
    (0, 6): "history_cradle",
    (1, 0): "history_beringia",
    # Beringia takes two floors: the land bridge from Asia at the foot, the
    # way south into the Americas at the top. So it touches the mammoth
    # steppe under South America's row.
    (1, 0.5): "history_beringia",
    (1, 1): "history_mammoth_steppe",
    (1, 2): "history_sahul",
    # Southeast Asia, a floor between Oceania and South Asia: the dispersal
    # crossed Wallacea by sea, and the Austronesians sailed through it.
    (1, 2.5): "history_sulawesi",
    (2, 2.5): "history_austronesian_islands",
    (3, 2.5): "history_dong_son",
    (1, 3): "history_southern_route",
    (1, 4): "history_ice_age_europe",
    (1, 5): "history_levant",
    (1, 6): "history_southern_coast",
    (2, 0): "history_first_maize",
    (2, 0.5): "history_andean_highlands",
    (2, 1): "history_yellow_river",
    (2, 2): "history_new_guinea_highlands",
    (2, 3): "history_mehrgarh",
    (2, 4): "history_first_farmers_of_europe",
    (2, 5): "history_fertile_crescent",
    (2, 6): "history_green_sahara",
    (3, 0): "history_olmec_heartland",
    (3, 0.5): "history_caral",
    (3, 1): "history_shang_court",
    (3, 2): "history_lapita_voyagers",
    (3, 3): "history_indus_cities",
    (3, 4): "history_megalith_builders",
    (3, 5): "history_uruk",
    (3, 6): "history_nile_valley",
    (4, 0): "history_maya_city",
    (4, 1): "history_silk_road",
    (4, 2): "history_wayfinders",
    (4, 4): "history_celtic_frontier",
    (4, 5): "history_classical_shore",
    (4, 6): "history_aksum_and_kush",
    (5, 0): "history_taino_islands",
    (6, 0): "history_haitian_revolution",
    (5, 1): "history_mongol_steppe",
    (5, 2): "history_rapa_nui",
    (5, 4): "history_viking_north",
    (5, 6): "history_timbuktu_coast",
    (6, 6): "history_benin_city",
    (7, 6): "history_door_of_no_return",
    (6, 4): "history_age_of_sail",
    (7, 4): "history_industrial_age",
}

#: The grid is where the dimension starts, not a rule. A room may take
#: several cells of one row (a wide room: an age that lasted), a row may be
#: put in between two others (its key is any number that sorts there), and
#: `EXTRA_WAYS` joins two rooms that are not one above the other in the
#: grid. The rows are drawn in the order of their keys, one under the next.
ROW_ORDER = sorted({r for _, r in GRID})


def row_below(c, r):
    """The next row down that has another room in column `c`, or None. A
    row put in between (South America under the Americas) does not cut a
    way down of a column where it has no room."""
    for lower in ROW_ORDER[ROW_ORDER.index(r) + 1:]:
        if (c, lower) in GRID and GRID[(c, lower)] != GRID[(c, r)]:
            return lower
    return None


#: Each room's cells: its columns (one row).
CELLS = {}
for _cell, _rid in GRID.items():
    CELLS.setdefault(_rid, []).append(_cell)


def span(rid):
    """The first and last column of a room, and its rows, top first."""
    cols = sorted({c for c, _ in CELLS[rid]})
    rows = sorted({r for _, r in CELLS[rid]})
    i = ROW_ORDER.index(rows[0])
    assert cols == list(range(cols[0], cols[-1] + 1)) and rows == ROW_ORDER[i:i + len(rows)] \
        and len(CELLS[rid]) == len(cols) * len(rows), f"{rid} takes a rectangle of cells"
    return cols[0], cols[-1], rows

#: The ways between floors: (column, upper row) -> the left of the hole, in
#: room px. The hole is in the upper room's floor and the lower room's
#: ceiling, at the same x; a tower of ledges climbs to it in the lower room.
#: A way is only where the two regions met then.
LINKS = {
    # The great dispersal: out of Africa through the Levant, then to every
    # land mass.
    (1, 0): 600,   # Beringia and the mammoth steppe: across the land bridge
    (1, 1): 2100,  # the mammoth steppe and Sahul: down the east of Asia
    (1, 2): 600,   # Sahul and Sulawesi: by sea, island to island
    (1, 2.5): 2400,  # Sulawesi and the southern route: along the coasts
    (1, 3): 2100,  # the southern route and Ice Age Europe: north and west
    (1, 4): 600,   # Ice Age Europe and the Levant
    (1, 5): 2100,  # the Levant and the southern coast: out of Africa
    # The first farmers: farming goes from the Crescent to Europe and Africa.
    # The Austronesians reach New Guinea's coasts; with the people there they
    # become the Lapita.
    (2, 2): 2000,
    (2, 4): 400,
    (2, 5): 200,
    # Maize goes south from Mexico to the Andes, where the potato was first
    # farmed. (The way goes to the next floor down with a room in the
    # column: here, South America.)
    (2, 0): 2200,
    # The Austronesians: the Lapita's ancestors sailed from Taiwan.
    (3, 1): 1800,
    # The first cities: tin from the north to Uruk; Uruk and Egypt trade.
    (3, 4): 2100,
    (3, 5): 176,
    # Classical: Rome and the Celts; Rome and Aksum by the Red Sea.
    (4, 4): 2680,
    (4, 5): 1180,
}
HOLE = 96


def _local_x(rid, c, x):
    """`x` of cell column `c` in the coordinates of `rid` (a wide room's left)."""
    return x + (c - span(rid)[0]) * W


#: Every way up and down: (upper room, lower room, x of the hole in the
#: upper room, x of the hole in the lower room's ceiling). The grid's come
#: from `LINKS`; `EXTRA_WAYS` adds more in the same shape.
EXTRA_WAYS = []
WAYS = [(GRID[(c, r)], GRID[(c, row_below(c, r))], _local_x(GRID[(c, r)], c, x), _local_x(GRID[(c, row_below(c, r))], c, x))
           for (c, r), x in LINKS.items()] + EXTRA_WAYS


def _ups(rid):
    return [(upper, xl) for upper, lower, _xu, xl in WAYS if lower == rid]


def _downs(rid):
    return [(lower, xu) for upper, lower, xu, _xl in WAYS if upper == rid]


def up_zone(lower, upper):
    """The zone in `lower`'s ceiling, up to `upper`."""
    return f"{lower}_up" if len(_ups(lower)) == 1 else f"{lower}_up_to_{upper}"


def down_zone(upper, lower):
    """The zone at the foot of `upper`'s hole down to `lower`."""
    return f"{upper}_down" if len(_downs(upper)) == 1 else f"{upper}_down_to_{lower}"

#: The buildings, under the grid, each by a door in its room.
GRANARY = "history_temple_granary"
TOMB = "history_pyramid_tomb"
COLOSSEUM = "history_colosseum"
GER = "history_great_ger"
LIBRARY = "history_timbuktu_library"
MILL = "history_cotton_mill"
CAVE = "history_painted_cave"
BUILDINGS = (GRANARY, TOMB, COLOSSEUM, GER, LIBRARY, MILL, CAVE)

BG = "#3A2A1C"
#: A room of the grid (`W`, `H`, at the top of the file); the floor has 64 of
#: ground under it. In a room of two floors, `FLOOR` is the walk line of the
#: top one and `FLOOR + H` the ground.
FLOOR = H - 64
CEILING = 32
ROW_Y = {r: i * H for i, r in enumerate(ROW_ORDER)}

ROOMS_AT = {}
for _rid in CELLS:
    _c0, _c1, _rows = span(_rid)
    ROOMS_AT[_rid] = ((_c0 * W, ROW_Y[_rows[0]]), ((_c1 - _c0 + 1) * W, len(_rows) * H))
ROOMS_AT.update({rid: ((n * 1792, len(ROW_ORDER) * H + 400), (1600, 640)) for n, rid in enumerate(BUILDINGS)})

#: One cue for each floor (region), so the music stays with the player who
#: walks east through the ages. A building takes the cue of its region,
#: except where its own place has its own mood (the tomb, the arena, the
#: mill, the cave). The Age of Sail and the Industrial Age are not one
#: region's music any more; they keep the cues of their eras.
AFRICA = "history_africa_first_footsteps"
CRESCENT = "history_crescent_between_the_rivers"
EUROPE = "history_europe_solstice_stones"
SOUTH_ASIA = "history_south_asia_monsoon_road"
EAST_ASIA = "history_east_asia_yellow_earth"
AMERICAS = "history_americas_land_bridge"
OCEANIA = "history_oceania_sahul"
#: The fort of the Door of No Return has its own quiet cue: no region's
#: music is right there.
NO_RETURN = "history_atlantic_no_return"
SE_ASIA = "history_southeast_asia_bronze_and_rain"
MUSIC = {
    "history_cradle": AFRICA,
    "history_southern_coast": AFRICA,
    "history_green_sahara": AFRICA,
    "history_nile_valley": AFRICA,
    "history_aksum_and_kush": AFRICA,
    "history_timbuktu_coast": AFRICA,
    "history_benin_city": AFRICA,
    "history_door_of_no_return": NO_RETURN,
    "history_levant": CRESCENT,
    "history_fertile_crescent": CRESCENT,
    "history_uruk": CRESCENT,
    "history_classical_shore": CRESCENT,
    "history_ice_age_europe": EUROPE,
    "history_first_farmers_of_europe": EUROPE,
    "history_megalith_builders": EUROPE,
    "history_celtic_frontier": EUROPE,
    "history_viking_north": EUROPE,
    "history_age_of_sail": "pirates_guild_black_flag_jig",
    "history_industrial_age": "galvanic_march",
    "history_southern_route": SOUTH_ASIA,
    "history_mehrgarh": SOUTH_ASIA,
    "history_indus_cities": SOUTH_ASIA,
    "history_sahul": OCEANIA,
    "history_sulawesi": SE_ASIA,
    "history_austronesian_islands": SE_ASIA,
    "history_dong_son": SE_ASIA,
    "history_new_guinea_highlands": OCEANIA,
    "history_mammoth_steppe": EAST_ASIA,
    "history_yellow_river": EAST_ASIA,
    "history_shang_court": EAST_ASIA,
    "history_silk_road": EAST_ASIA,
    "history_mongol_steppe": EAST_ASIA,
    "history_beringia": AMERICAS,
    "history_first_maize": AMERICAS,
    "history_andean_highlands": AMERICAS,
    "history_caral": AMERICAS,
    "history_olmec_heartland": AMERICAS,
    "history_maya_city": AMERICAS,
    "history_taino_islands": AMERICAS,
    "history_haitian_revolution": AMERICAS,
    "history_lapita_voyagers": OCEANIA,
    "history_wayfinders": OCEANIA,
    "history_rapa_nui": OCEANIA,
    GRANARY: CRESCENT,
    TOMB: "vault_in_the_dark",
    COLOSSEUM: "fighters_guild_oath_of_steel",
    GER: EAST_ASIA,
    LIBRARY: AFRICA,
    MILL: "luddites_guild_loom_and_liberty",
    CAVE: "vault_in_the_dark",
}
BIOME = {
    "history_cradle": "history_savanna", "history_southern_coast": "history_savanna",
    "history_green_sahara": "history_green_sahara",
    "history_nile_valley": "history_nile", TOMB: "history_nile", "history_aksum_and_kush": "history_nile",
    "history_timbuktu_coast": "history_monsoon", LIBRARY: "history_monsoon",
    "history_benin_city": "history_benin",
    "history_door_of_no_return": "history_gold_coast",
    "history_levant": "history_crescent", "history_fertile_crescent": "history_crescent", GRANARY: "history_crescent",
    "history_uruk": "history_crescent",
    "history_classical_shore": "history_classical", COLOSSEUM: "history_classical",
    "history_ice_age_europe": "history_ice_age", "history_mammoth_steppe": "history_ice_age",
    "history_beringia": "history_ice_age",
    "history_first_farmers_of_europe": "history_megalith", "history_megalith_builders": "history_megalith",
    "history_celtic_frontier": "history_northwoods", "history_viking_north": "history_northwoods",
    "history_age_of_sail": "history_sail",
    "history_industrial_age": "history_industrial", MILL: "history_industrial",
    "history_southern_route": "history_indus", "history_mehrgarh": "history_indus", "history_indus_cities": "history_indus",
    "history_sahul": "history_sahul",
    "history_sulawesi": "history_wallacea", "history_austronesian_islands": "history_austronesia",
    "history_dong_son": "history_red_river",
    "history_new_guinea_highlands": "history_highlands",
    "history_yellow_river": "history_yellow_river", "history_shang_court": "history_yellow_river",
    "history_silk_road": "history_silk",
    "history_mongol_steppe": "history_steppe", GER: "history_steppe",
    "history_first_maize": "history_maize",
    "history_andean_highlands": "history_andes", "history_caral": "history_supe",
    "history_olmec_heartland": "history_olmec", "history_maya_city": "history_maya",
    "history_taino_islands": "history_caribbean",
    "history_haitian_revolution": "history_saint_domingue",
    "history_lapita_voyagers": "history_lapita", "history_wayfinders": "history_open_ocean",
    "history_rapa_nui": "history_rapa_nui",
    CAVE: "history_painted_cave",
}

#: The box of each kind of creature (its whole size).
BOX = {
    "locust_swarm": (44, 36), "aurochs": (96, 70), "scorpion_man": (52, 72),
    "mummy": (34, 62), "scarab_beetle": (40, 28), "asp": (30, 40),
    "hoplite": (36, 64), "immortal": (34, 64), "vandal": (34, 60),
    "terracotta_warrior": (40, 72), "desert_bandit": (30, 58), "silk_moth": (44, 34),
    "horse_archer": (64, 64), "golden_eagle": (48, 34), "steppe_wolf": (60, 36),
    "inflated_coin": (40, 40), "coral_crab": (48, 30), "monsoon_squall": (48, 36),
    "scurvy_ghost": (36, 50), "kraken_arm": (60, 96), "privateer": (32, 60),
    "steam_automaton": (60, 80), "soot_sprite": (30, 30), "robber_baron": (34, 62),
    # The second pass.
    "hyena": (56, 40), "cave_lion": (80, 50), "hippo": (100, 64), "sea_raider": (32, 60),
    "woolly_mammoth": (130, 110), "cave_bear": (80, 64), "viking_raider": (34, 62),
    "tiger": (84, 48), "indus_unicorn": (64, 52), "taotie_spirit": (48, 48),
    "smilodon": (80, 50), "jaguar": (72, 42), "llama": (52, 64),
    "diprotodon": (110, 70), "megalania": (120, 40), "cassowary_history": (44, 70),
    # The third pass.
    "howler_monkey": (40, 44), "green_iguana": (72, 32), "coconut_crab": (64, 40),
    "frigatebird": (64, 40), "pacific_rat": (32, 20),
    # Wave A: the Atlantic world.
    "driver_ants": (72, 24), "napoleonic_grenadier": (34, 64),
    # Wave B: South America.
    "andean_condor": (80, 40),
    # Wave C: Southeast Asia.
    "sulawesi_warty_pig": (64, 40), "saltwater_crocodile": (120, 32),
}

DOOR_W, DOOR_H = 48, 192
INSIDE_FLOOR = 576
BRIDGE_W = 224
#: The flags and levers of the ways through.
SLUICE = "history_sluice"
KNOWS_NOTHING = "history_knows_nothing"
CAN_ROAD, CANT_ROAD = "history_can_says_yes", "history_cant_says_maybe"
BOILER = "history_boiler"
FIRST_FIRE = "history_first_fire"
SOLSTICE = "history_solstice"
ORACLE = "history_oracle_bone"
UNICORN_SEAL = "history_unicorn_seal"
BALLCOURT_RING = "history_ballcourt_ring"
#: The Great Wall in the steppe (x0, x1), its top, and the gate in it.
WALL_X = (928, 1024)
WALL_TOP = FLOOR - 320
WALL_GATE = (952, 1000)
#: The two vigils.
TOMB_VIGIL = "history_tomb_vigil"
GAMES = "history_the_games"


# ---- Helpers -----------------------------------------------------------------


def enemy(rid, n, character, x, feet):
    """A creature whose feet (a flyer: the foot of its box) are at `feet`."""
    w, h = BOX[character]
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{rid}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def lock_wall(wall_id, name, x, y, w, h, gated_by, look):
    """A wall drawn as `look` that stands until `gated_by` is true."""
    return entity("LockWall", x, y, w, h, id=wall_id, name=name, gated_by=gated_by, look=look)


def gate(wall_id, name, x, gated_by, look, floor=FLOOR):
    """A door-sized LockWall on `floor` at `x`, with a lintel over it to the
    ceiling, so that the only way past is through it."""
    return [
        solid(f"{wall_id}_lintel", x - 16, CEILING, DOOR_W + 32, floor - DOOR_H - CEILING),
        lock_wall(wall_id, name, x, floor - DOOR_H, DOOR_W, DOOR_H, gated_by, look),
    ]


def lever(switch_id, name, prompt, x, floor, action="ToggleFlag", target=""):
    """A lever on `floor`; its state is `world.switch_on <switch_id>`."""
    fields = dict(id=switch_id, name=name, prompt=prompt, action=action)
    if target:
        fields["target_encounter"] = target
    return entity("Switch", x, floor - 32, 24, 32, **fields)


def prop_at(name, kind, x, floor, w, h):
    """A prop of `w` x `h` with its foot in the middle of its bottom, at `x` on `floor`."""
    return prop(name, kind, x - w // 2, floor - h, w, h)


def chest(name, x, floor, reward):
    return entity("ChestSpawn", x - 38, floor - 58, 76, 58, name=name, reward=reward)


def npc(spec, x, floor, h, catalog=None):
    """A resident of the dimension: catalog id `npc_<spec>`, Yarn node `<spec>`."""
    return resident(catalog or f"npc_{spec}", spec, x, floor, h=h)


def crossing(rid, name, span, look, *, w=160, speed=90.0, top=FLOOR - 8):
    """A hazard at the foot of a gap in the floor from `span[0]` to
    `span[1]`, and a platform drawn as `look` that sweeps to and fro over it
    at `top`. Returns the entities and the gap."""
    x0, x1 = span
    return [
        entity("HazardBlock", x0, H - 16, x1 - x0, 16, name=name),
        entity("MovingPlatform", x0 + 8, top, w, 16, id=f"{rid}_{look}", name=look.replace("history_", "the ").replace("_", " "),
               speed=speed, sweep_dx=float(x1 - x0 - w - 16), one_way=True, visual=look),
    ], span


def solids_beside(name, gaps, y, h, w=W):
    """Solids from 0 to `w` at `y`, `h` high, with `gaps` (x0, x1) left open."""
    out, x = [], 0
    for n, (g0, g1) in enumerate(sorted(gaps)):
        if g0 > x:
            out.append(solid(f"{name}_{n}", x, y, g0 - x, h))
        x = max(x, g1)
    if x < w:
        out.append(solid(f"{name}_end", x, y, w - x, h))
    return out


def vertical_edge(zone, name, x0, x1, y, to_room, to_zone):
    """An edge exit across the top (`y` 0) or the bottom of a room."""
    return entity("LoadingZone", x0, y, x1 - x0, 16, id=zone, name=name, activation="EdgeExit", target_room=to_room,
                  target_zone=to_zone, bidirectional=True)


#: The tower of ledges under a hole in a ceiling. A jump rises about 88, so
#: the ledges are `TOWER_RISE` apart. A jump from the top ledge puts the head
#: (48 over the feet) into the edge in the ceiling (0 to 16): 144 - 48 - 88 is
#: under 16. A player who comes down arrives with the feet at 115 (the edge
#: arrival is 92 under the top) and falls onto it.
TOWER_RISE = 72
TOWER_TOP = 144
TOWER_LEDGE = 128
TOWER_STEP = 56


def tower(name, hole_x, ground, w):
    """The ledges up from `ground` to a hole in the ceiling at `hole_x`: the
    top one under the hole, the ones under it to the right and the left in
    turn, so the way up is a zig-zag of jumps and not a ladder."""
    mid = hole_x + HOLE // 2
    lo, hi = WALL + TOWER_LEDGE // 2 + 8, w - WALL - TOWER_LEDGE // 2 - 8
    out = []
    for n, y in enumerate(range(TOWER_TOP, ground - TOWER_RISE + 1, TOWER_RISE)):
        x = mid if n == 0 else mid + (TOWER_STEP if n % 2 else -TOWER_STEP)
        x = min(max(x, lo), hi)
        out.append(ledge(f"{name}_{n}", x - TOWER_LEDGE // 2, y, TOWER_LEDGE))
    return out


def blurb(rid, x):
    """The development blurb (`blurbs.py`): two lines at the top of the room,
    over `x`, under the room's title (`district_kit.room`, at 40): where and
    when, and a fact."""
    out = []
    for n, text in enumerate(BLURB[rid]):
        w = len(text) * 9 + 24
        out.append(entity("DebugLabel", x - w // 2, 64 + n * 24, w, 24, name=f"{rid}_blurb_{n}", text=text,
                          category="Room"))
    return out


def grid_room(rid, title, entities, gaps=(), blurb_x=None):
    """A room of the grid: its ground (open at `gaps`, and at each way down
    under a one-way hatch), its ceiling (open at each way up, over a tower
    of ledges), its walls, the side exits to the ages before and after it on
    each of its rows, the ways to the rows above and below, its blurb, and
    `entities`. A room of two rows has a side exit at the walk line of each
    row that has a neighbour; the room's own terrain must reach it."""
    c0, c1, rows = span(rid)
    w, h = ROOMS_AT[rid][1]
    ground = h - 64
    floor_gaps = list(gaps)
    ceiling_gaps = []
    e = []
    for upper, x in _ups(rid):
        ceiling_gaps.append((x, x + HOLE))
        e.append(vertical_edge(up_zone(rid, upper), "up", x, x + HOLE, 0, upper, down_zone(upper, rid)))
        e += tower(f"tower_to_{upper}", x, ground, w)
    for lower, x in _downs(rid):
        floor_gaps.append((x, x + HOLE))
        # A hatch over the hole: walk over it, or drop through it.
        e.append(ledge("hatch", x, ground, HOLE))
        e.append(vertical_edge(down_zone(rid, lower), "down through the hatch", x, x + HOLE, h - 16, lower,
                               up_zone(lower, rid)))
    e += solids_beside("ground", floor_gaps, ground, h - ground, w)
    e += solids_beside("ceiling", ceiling_gaps, 0, CEILING, w)
    for side, col, x, after in (("west", c0 - 1, 0, "east"), ("east", c1 + 1, w - WALL, "west")):
        openings = []
        for i, r in enumerate(rows):
            other = GRID.get((col, r))
            if other is None or other == rid:
                continue
            walk = i * H + FLOOR
            openings.append((walk - 112, walk))
            word = "back in time" if side == "west" else "forward in time"
            e.append(edge(side_zone(rid, side, r), word, x, walk, other, side_zone(other, after, r)))
        e += [solid(f"{side}_wall_{n}", x, y0, WALL, y1 - y0)
              for n, (y0, y1) in enumerate(_beside(CEILING, ground, openings)) if y1 > y0]
    e += blurb(rid, w // 2 if blurb_x is None else blurb_x)
    at, size = ROOMS_AT[rid]
    return rid, room(rid, e + entities, at=at, size=size, biome=BIOME[rid], music=MUSIC[rid], bg=BG, title=title,
                     generator=GENERATOR, dangerous=True)


def side_zone(rid, side, r):
    """The id of `rid`'s side exit on `side` at row `r`: `<rid>_<side>`, or,
    when that side has an exit on more than one row, `<rid>_<side>_<row>`."""
    c0, c1, rows = span(rid)
    col = c0 - 1 if side == "west" else c1 + 1
    exits = [row for row in rows if GRID.get((col, row)) not in (None, rid)]
    return f"{rid}_{side}" if len(exits) <= 1 else f"{rid}_{side}_{rows.index(r)}"


def _beside(y0, y1, holes):
    """The spans from `y0` to `y1` that `holes` (top, bottom) leave."""
    out, y = [], y0
    for top, bottom in sorted(holes):
        out.append((y, top))
        y = max(y, bottom)
    out.append((y, y1))
    return out


def building(rid, back_to, back_zone, title, extra):
    """A building by a door: its floor, ceiling and walls, the door back out
    at its west end, its blurb, and `extra`."""
    w, h = ROOMS_AT[rid][1]
    g = INSIDE_FLOOR
    e = [
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, CEILING),
        solid("west_wall", 0, CEILING, WALL, g - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, g - CEILING),
        door(f"{rid}_door", "back out", 40, g, back_to, back_zone),
        start(rid, 140, g),
        *blurb(rid, w // 2),
        *extra,
    ]
    at, size = ROOMS_AT[rid]
    return rid, room(rid, e, at=at, size=size, biome=BIOME[rid], music=MUSIC[rid], bg=BG, title=title,
                     generator=GENERATOR)


def vigil(rid, back_to, back_zone, title, encounter, *, bell, store_name, chest_name, signs):
    """A building that a vigil shuts: the shutter, the strip that starts it,
    the bell that makes it ready again, and a store room whose grille opens
    when it is over."""
    g = INSIDE_FLOOR
    store = 1312
    extra = [
        entity("LockWall", 176, CEILING, 32, g - CEILING, id=f"{rid}_lock", name="the shutter", look="temple_gate_iron"),
        entity("EncounterTrigger", 320, CEILING, 64, g - CEILING, id=encounter, name=f"{encounter}_trigger",
               camera_zoom=1.2),
        lever(f"{rid}_again", bell, "Ring for it again", 140, g, action="ResetEncounter", target=encounter),
        solid("store_wall", store, CEILING, 32, g - 128 - CEILING),
        lock_wall(f"{rid}_store_grille", store_name, store, g - 128, 32, 128, f"encounter.cleared {encounter}",
                  "temple_gate_iron"),
        chest(chest_name, 1478, g, "currency:150"),
        *signs,
    ]
    return building(rid, back_to, back_zone, title, extra)


# ---- R6: Africa --------------------------------------------------------------


def cradle():
    """The Great Rift Valley, 300,000 years ago: the start of everyone. The
    ring and Clio; the footprints at Laetoli; the first fire. A thorn boma
    keeps the hyenas out until the fire is lit (`history_first_fire`)."""
    rid = "history_cradle"
    e = [
        start(rid, 520, FLOOR),
        *gates.as_ron(gates.ring("Cradle Gate", "history_cradle_gate", 112, FLOOR, "the gate")),
        *gates.as_ron(gates.terminal("Cradle Gate", "history_gate_terminal", 416, FLOOR)),
        sign("cradle_sign_gate", 256, FLOOR - 340, "DYNASTIC: the gate at the start of everyone", 380),
        npc("history_clio", 620, FLOOR, 66),
        prop_at("acacia_1", "history_acacia", 780, FLOOR, 192, 192),
        npc("history_lucy", 900, FLOOR, 43),
        prop_at("footprints", "history_laetoli_footprints", 1080, FLOOR, 128, 24),
        npc("history_mary_leaky", 1240, FLOOR, 62),
        sign("cradle_sign_footprints", 1080, FLOOR - 120, "LAETOLI: two people walked here, 3.6 million years ago", 480),
        prop_at("campfire", "history_campfire", 1400, FLOOR, 64, 64),
        lever(FIRST_FIRE, "the fire drill", "Make fire", 1460, FLOOR),
        sign("cradle_sign_fire", 1420, FLOOR - 160, "FIRE: people tended it long before this. Warm food, fewer hyenas.", 609),
        *gate("history_thorn_boma", "the thorn boma", 1640, f"world.switch_on {FIRST_FIRE}", "history_thorn_boma"),
        sign("cradle_sign", 1400, 140, "THE CRADLE: the Great Rift Valley, 300,000 years ago", 500),
        enemy(rid, 1, "hyena", 1900, FLOOR),
        ledge("escarpment_a", 2000, FLOOR - 80, 192),
        ledge("escarpment_b", 2220, FLOOR - 160, 192),
        enemy(rid, 2, "hyena", 2300, FLOOR),
        prop_at("acacia_2", "history_acacia", 2480, FLOOR, 192, 192),
        sign("cradle_sign_east", 2560, FLOOR - 220, "EAST: the rest of history", 300),
    ]
    return grid_room(rid, "THE CRADLE", e)


def southern_coast():
    """The southern coast, 75,000 years ago: shellfish, ochre and the first
    art. The way up is the way out of Africa, into the Levant."""
    rid = "history_southern_coast"
    up = LINKS[(1, 5)]
    e = [
        start(rid, 160, FLOOR),
        prop_at("acacia", "history_acacia", 320, FLOOR, 192, 192),
        npc("history_ochre_artist", 520, FLOOR, 63),
        prop_at("rock_art", "history_rock_art", 700, FLOOR, 128, 96),
        sign("coast_sign_ochre", 600, FLOOR - 200, "BLOMBOS CAVE: a pattern scratched in ochre, 75,000 years ago", 540),
        enemy(rid, 1, "hyena", 980, FLOOR),
        ledge("dune_a", 1120, FLOOR - 80, 192),
        ledge("dune_b", 1340, FLOOR - 160, 192),
        enemy(rid, 1, "cave_lion", 1500, FLOOR),
        prop_at("campfire", "history_campfire", 1800, FLOOR, 64, 64),
        sign("coast_sign", 1400, 140, "THE SOUTHERN COAST: shellfish, ochre and the first art", 480),
        sign("coast_sign_out", up + HOLE // 2, FLOOR - 260, "UP: OUT OF AFRICA. Everyone you know came this way.", 480),
        enemy(rid, 2, "hyena", 2400, FLOOR),
        prop_at("acacia_2", "history_acacia", 2600, FLOOR, 192, 192),
    ]
    return grid_room(rid, "THE SOUTHERN COAST", e)


def green_sahara():
    """The Sahara when it was green, 7,000 BCE: a lake where the desert is
    now, cattle and rock paintings. A reed boat carries you over the lake."""
    rid = "history_green_sahara"
    stuff, gap = crossing(rid, "the lake (hippos)", (1100, 1800), "history_reed_boat")
    e = [
        *stuff,
        start(rid, 480, FLOOR),
        sign("sahara_sign_up", 248, FLOOR - 260, "UP: the Fertile Crescent (cattle went both ways)", 420),
        npc("history_cattle_herder", 620, FLOOR, 68),
        prop_at("rock_art", "history_rock_art", 820, FLOOR, 128, 96),
        enemy(rid, 1, "hippo", 960, FLOOR),
        sign("sahara_sign_lake", 1450, FLOOR - 240, "LAKE MEGACHAD: bigger than the Caspian Sea, 7,000 BCE", 520),
        sign("sahara_sign_herding", 640, FLOOR - 220, "CATTLE FIRST: Africa's own grains, sorghum and millet, were farmed later, to the south.", 807),
        sign("sahara_sign", 1400, 140, "THE GREEN SAHARA: before the desert, there were lakes", 480),
        enemy(rid, 2, "hippo", 1960, FLOOR),
        ledge("rock_a", 2120, FLOOR - 80, 160),
        ledge("rock_b", 2300, FLOOR - 160, 160),
        enemy(rid, 1, "locust_swarm", 2400, FLOOR - 320),
        prop_at("rock_art_2", "history_rock_art", 2560, FLOOR, 128, 96),
    ]
    return grid_room(rid, "THE GREEN SAHARA", e, gaps=[gap])


def nile():
    rid = "history_nile_valley"
    stuff, gap = crossing(rid, "the crocodiles", (1000, 1800), "history_felucca")
    e = [
        *stuff,
        start(rid, 560, FLOOR),
        npc("history_hatshepsut_hats", 340, FLOOR, 64),
        prop_at("obelisk", "history_obelisk", 460, FLOOR, 48, 224),
        enemy(rid, 1, "scarab_beetle", 600, FLOOR),
        prop_at("papyrus_1", "history_papyrus", 720, FLOOR, 64, 96),
        enemy(rid, 1, "asp", 840, FLOOR),
        prop_at("papyrus_2", "history_papyrus", 940, FLOOR, 64, 96),
        sign("nile_sign_up", 224, FLOOR - 300, "UP: Uruk (they trade with us)", 300),
        sign("nile_sign_river", 1400, FLOOR - 240, "THE NILE: floods every year, on schedule, unlike the rest of history", 560),
        sign("nile_sign", 1400, 140, "THE NILE: 3,000 years of the same hats", 420),
        prop_at("papyrus_3", "history_papyrus", 1860, FLOOR, 64, 96),
        enemy(rid, 2, "scarab_beetle", 1960, FLOOR),
        prop_at("statue", "history_pharaoh_statue", 2080, FLOOR, 128, 224),
        npc("history_ramesses_pretty_good", 2220, FLOOR, 68),
        prop_at("jars", "history_canopic_jars", 2330, FLOOR, 80, 48),
        ledge("pyramid_step_a", 2300, FLOOR - 64, 240),
        ledge("pyramid_step_b", 2360, FLOOR - 128, 120),
        enemy(rid, 1, "mummy", 2500, FLOOR),
        enemy(rid, 2, "asp", 2400, FLOOR - 128),
        door("nile_tomb_door", "into the pyramid", 2640, FLOOR, TOMB, f"{TOMB}_door"),
        sign("nile_sign_tomb", 2560, FLOOR - 260, "PYRAMID: do not disturb (this means you)", 380),
    ]
    return grid_room(rid, "THE NILE", e, gaps=[gap])


def tomb():
    g = INSIDE_FLOOR
    return vigil(TOMB, "history_nile_valley", "nile_tomb_door", "THE PYRAMID TOMB", TOMB_VIGIL, bell="the priest's bell",
                 store_name="the grille of the treasure room", chest_name="the pharaoh's luggage for the afterlife",
                 signs=[
                     prop_at("jars", "history_canopic_jars", 640, g, 80, 48),
                     ledge("sarcophagus_ledge", 820, g - 96, 200),
                     prop_at("statue", "history_pharaoh_statue", 1120, g, 128, 224),
                     sign("tomb_sign", 800, 124, "THE PYRAMID TOMB: everything you need for the afterlife, packed", 560),
                     sign("tomb_sign_curse", 1440, g - 200, "THE CURSE: mostly a rumour", 300),
                 ])


def aksum():
    """Aksum and Kush, 300 BCE to 600 CE: the pyramids of Meroë, the
    kandakes who fought Rome to a draw, the stelae of Aksum. The way up is
    the Red Sea road to Rome."""
    rid = "history_aksum_and_kush"
    up = LINKS[(4, 5)]
    e = [
        start(rid, 160, FLOOR),
        prop_at("stele_1", "history_aksum_stele", 340, FLOOR, 48, 224),
        npc("history_kandake_can_do", 520, FLOOR, 67),
        enemy(rid, 1, "asp", 760, FLOOR),
        prop_at("obelisk", "history_obelisk", 900, FLOOR, 48, 224),
        sign("aksum_sign_up", up + HOLE // 2, FLOOR - 260, "UP: Rome, by the Red Sea", 300),
        enemy(rid, 1, "mummy", 1500, FLOOR),
        ledge("pyramid_step_a", 1600, FLOOR - 80, 240),
        ledge("pyramid_step_b", 1680, FLOOR - 160, 120),
        sign("aksum_sign_meroe", 1720, FLOOR - 280, "MEROË: more pyramids than Egypt, and smaller, and steeper", 520),
        sign("aksum_sign", 1400, 140, "AKSUM AND KUSH: the kingdoms up the Nile", 440),
        enemy(rid, 2, "scarab_beetle", 2100, FLOOR),
        prop_at("stele_2", "history_aksum_stele", 2300, FLOOR, 48, 224),
        enemy(rid, 2, "asp", 2480, FLOOR),
        prop_at("stele_3", "history_aksum_stele", 2620, FLOOR, 48, 224),
        sign("aksum_sign_east", 2480, FLOOR - 340, "FORWARD: Mali, by the atlas's road; Aksum and Mali never met.", 573),
    ]
    return grid_room(rid, "AKSUM AND KUSH", e)


def timbuktu():
    rid = "history_timbuktu_coast"
    stuff, gap = crossing(rid, "the sea", (1500, 2300), "history_dhow", speed=110.0)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("tower_1", "history_mud_tower", 260, FLOOR, 96, 224),
        npc("history_mansa_moolah", 420, FLOOR, 68),
        prop_at("gold", "history_gold_pile", 520, FLOOR, 64, 32),
        door("timbuktu_library_door", "into the library", 620, FLOOR, LIBRARY, f"{LIBRARY}_door"),
        sign("timbuktu_sign_library", 644, FLOOR - 150, "THE LIBRARIES OF TIMBUKTU", 300),
        prop_at("salt", "history_salt_slabs", 780, FLOOR, 64, 48),
        enemy(rid, 1, "inflated_coin", 900, FLOOR - 200),
        enemy(rid, 2, "inflated_coin", 1060, FLOOR - 300),
        prop_at("tower_2", "history_mud_tower", 1180, FLOOR, 96, 224),
        ledge("tower_beam", 1240, FLOOR - 96, 160),
        sign("timbuktu_sign_gold", 900, FLOOR - 400, "PRICE OF GOLD: falling (he is in town)", 380),
        sign("timbuktu_sign", 1400, 140, "TIMBUKTU TO THE MONSOON COAST: salt north, gold south, books everywhere", 640),
        enemy(rid, 1, "coral_crab", 1400, FLOOR),
        sign("timbuktu_sign_sea", 1900, FLOOR - 240, "THE MONSOON: east in summer, west in winter. Plan ahead.", 520),
        sign("timbuktu_sign_fold", 1900, FLOOR - 360, "THE SWAHILI COAST: 5,000 km east of Timbuktu. The atlas folds the map.", 654),
        enemy(rid, 1, "monsoon_squall", 1900, FLOOR - 320),
        enemy(rid, 2, "monsoon_squall", 2200, FLOOR - 420),
        enemy(rid, 2, "coral_crab", 2420, FLOOR),
        npc("history_zheng_hey", 2560, FLOOR, 68),
        prop_at("manuscripts", "history_manuscripts", 2660, FLOOR, 64, 48),
    ]
    return grid_room(rid, "TIMBUKTU AND THE MONSOON COAST", e, gaps=[gap])


def library():
    g = INSIDE_FLOOR
    extra = [
        prop_at("manuscripts_1", "history_manuscripts", 340, g, 64, 48),
        npc("history_ibn_batootah", 480, g, 68),
        ledge("shelf_a", 640, g - 96, 200),
        prop_at("manuscripts_2", "history_manuscripts", 740, g - 96, 64, 48),
        ledge("shelf_b", 900, g - 176, 200),
        prop_at("gold", "history_gold_pile", 1160, g, 64, 32),
        chest("a manuscript worth more than gold", 1420, g, "currency:150"),
        sign("library_sign", 800, 124, "THE LIBRARY OF TIMBUKTU: astronomy, law, medicine, poems", 520),
        sign("library_sign_gold", 1160, g - 120, "GOLD (worth less than the books)", 340),
    ]
    return building(LIBRARY, "history_timbuktu_coast", "timbuktu_library_door", "THE LIBRARY OF TIMBUKTU", extra)


def benin_city():
    """Benin City, the capital of the kingdom of Benin (the Edo people),
    about 1200-1897 CE: its walls of earth (the iya), the guild of brass
    casters, the Oba's palace with its brass plaques, and Iyoba Idia, the
    first queen mother. At the east end, a plain sign says what came in
    1897."""
    rid = "history_benin_city"
    e = [
        start(rid, 160, FLOOR),
        # The iya: a bank of earth. Up the step and over it.
        ledge("iya_step", 300, FLOOR - 80, 112),
        solid("iya_bank", 420, FLOOR - 144, 176, 144),
        sign("benin_sign_walls_a", 508, FLOOR - 340, "THE WALLS OF BENIN: banks and ditches dug by hand over centuries,", 600),
        sign("benin_sign_walls_b", 508, FLOOR - 312, "thousands of kilometres of them round the city and its villages", 580),
        npc("history_brass_caster", 720, FLOOR, 64),
        prop_at("furnace", "history_brass_furnace", 840, FLOOR, 80, 96),
        sign("benin_sign_wax", 860, FLOOR - 220, "LOST WAX: shape it in wax, coat it in clay, melt the wax out, pour in brass", 680),
        prop_at("manillas", "history_manillas", 1060, FLOOR, 64, 32),
        sign("benin_sign_manillas", 1040, FLOOR - 120, "MANILLAS: Portuguese brass, melted down for art", 440),
        enemy(rid, 1, "driver_ants", 1160, FLOOR),
        sign("benin_sign", 1400, 140, "BENIN CITY: the walls, the brass casters and the court of the Oba, 1200-1897 CE", 700),
        sign("benin_sign_palace", 1600, FLOOR - 340, "THE OBA'S PALACE: walls of red earth, a brass bird on every turret", 600),
        prop_at("palace", "history_benin_palace", 1560, FLOOR, 256, 192),
        ledge("palace_roof_a", 1460, FLOOR - 112, 192),
        ledge("palace_roof_b", 1560, FLOOR - 224, 128),
        chest("cowries (the money of Benin)", 1624, FLOOR - 224, "currency:100"),
        prop_at("plaque_1", "history_benin_plaque", 1760, FLOOR - 48, 48, 64),
        prop_at("plaque_2", "history_benin_plaque", 1830, FLOOR - 48, 48, 64),
        prop_at("plaque_3", "history_benin_plaque", 1900, FLOOR - 48, 48, 64),
        sign("benin_sign_plaques", 1830, FLOOR - 160, "THE PLAQUES: the court's history, in brass", 400),
        npc("history_iyoba_idia", 2010, FLOOR, 63),
        sign("benin_sign_idia", 2010, FLOOR - 260, "IYOBA IDIA: she raised an army and won her son's war. The first queen mother.", 700),
        enemy(rid, 2, "driver_ants", 2160, FLOOR),
        enemy(rid, 1, "asp", 2280, FLOOR),
        npc("history_lord_finders_keepers", 2480, FLOOR, 70),
        prop_at("case", "history_museum_case", 2600, FLOOR, 80, 112),
        sign("benin_sign_1897_a", 2380, FLOOR - 360, "1897: a British force burned Benin City and carried off its brass and ivory,", 680),
        sign("benin_sign_1897_b", 2380, FLOOR - 332,
             "thousands of works. They fill museums in Europe and America. A few have come home.", 750),
    ]
    return grid_room(rid, "BENIN CITY", e)


# ---- R5: the Mediterranean and Southwest Asia ------------------------------


def levant():
    """The Levant, 70,000 years ago: the door out of Africa, where people met
    the Neanderthals (and stayed a while). Ways down to Africa and up to
    Europe."""
    rid = "history_levant"
    up, down = LINKS[(1, 4)], LINKS[(1, 5)]
    e = [
        start(rid, 160, FLOOR),
        sign("levant_sign_up", up + HOLE // 2, FLOOR - 260, "UP: Europe (it is cold)", 300),
        npc("history_neville_neanderthal", 900, FLOOR, 64),
        prop_at("campfire", "history_campfire", 1020, FLOOR, 64, 64),
        enemy(rid, 1, "aurochs", 1300, FLOOR),
        ledge("cave_ledge_a", 1440, FLOOR - 80, 192),
        ledge("cave_ledge_b", 1640, FLOOR - 160, 192),
        enemy(rid, 1, "cave_lion", 1760, FLOOR),
        sign("levant_sign", 1400, 140, "THE LEVANT: the door out of Africa, 70,000 years ago", 500),
        sign("levant_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: Africa, where we all came from", 400),
        enemy(rid, 2, "aurochs", 2400, FLOOR),
    ]
    return grid_room(rid, "THE LEVANT", e)


def crescent():
    """The dawn of farming: Hammurabbi, Gilgamash, the fields and the canal.
    The sluice raises the reed bridge over it."""
    rid = "history_fertile_crescent"
    c0, c1 = 1296, 1744
    e = [
        entity("HazardBlock", c0, H - 16, c1 - c0, 16, name="the deep canal"),
        # The reed bridge stands while the sluice's lever is on.
        *[lock_wall(f"history_reed_bridge_{n}", "the reed bridge", x, FLOOR, BRIDGE_W, 24, f"not world.switch_on {SLUICE}",
                    "history_reed_bridge") for n, x in enumerate(range(c0, c1, BRIDGE_W))],
        start(rid, 620, FLOOR),
        sign("crescent_sign_up", 448, FLOOR - 300, "UP: Europe (farming is going there next)", 400),
        sign("crescent_sign_down", 248, FLOOR - 200, "DOWN: the Green Sahara", 260),
        prop_at("wheat_1", "history_wheat_sheaf", 720, FLOOR, 48, 64),
        prop_at("palm_1", "history_date_palm", 820, FLOOR, 96, 192),
        npc("history_hammurabbi", 960, FLOOR, 68),
        prop_at("stele", "history_code_stele", 1060, FLOOR, 64, 160),
        sign("crescent_sign_code", 1060, FLOOR - 220, "THE CODE: 282 laws. Read all of them.", 380),
        prop_at("wheat_2", "history_wheat_sheaf", 1160, FLOOR, 48, 64),
        lever(SLUICE, "the sluice", "Open the sluice", 1220, FLOOR),
        sign("crescent_sign_canal", (c0 + c1) // 2, FLOOR - 200, "IRRIGATION: the first public works project (over budget)", 520),
        sign("crescent_sign", 1400, 140, "THE FERTILE CRESCENT: c. 10,000 BCE, give or take", 460),
        prop_at("ziggurat", "history_ziggurat", 2100, FLOOR, 256, 192),
        enemy(rid, 1, "locust_swarm", 1900, FLOOR - 220),
        enemy(rid, 1, "aurochs", 1880, FLOOR),
        prop_at("wheat_3", "history_wheat_sheaf", 1820, FLOOR, 48, 64),
        ledge("mudbrick_step_a", 2240, FLOOR - 80, 160),
        ledge("mudbrick_step_b", 2060, FLOOR - 160, 160),
        enemy(rid, 2, "locust_swarm", 2300, FLOOR - 300),
        npc("history_gilgamash", 2130, FLOOR - 160, 70),
        enemy(rid, 1, "scorpion_man", 2420, FLOOR),
        prop_at("tablets", "history_clay_tablets", 2560, FLOOR, 64, 48),
        door("crescent_granary_door", "into the temple granary", 2620, FLOOR, GRANARY, f"{GRANARY}_door"),
        prop_at("palm_2", "history_date_palm", 2480, FLOOR, 96, 192),
    ]
    return grid_room(rid, "THE FERTILE CRESCENT", e, gaps=[(c0, c1)])


def granary():
    g = INSIDE_FLOOR
    extra = [
        npc("history_scribe_of_debts", 420, g, 64),
        prop_at("tablets_1", "history_clay_tablets", 540, g, 64, 48),
        prop_at("wheat_1", "history_wheat_sheaf", 700, g, 48, 64),
        prop_at("wheat_2", "history_wheat_sheaf", 760, g, 48, 64),
        ledge("granary_loft", 880, g - 112, 240),
        prop_at("tablets_2", "history_clay_tablets", 1000, g - 112, 64, 48),
        chest("the first receipt", 1400, g, "currency:120"),
        sign("granary_sign", 800, 124, "THE TEMPLE GRANARY: writing was made here, to count barley", 520),
        sign("granary_sign_debt", 1400, g - 150, "YOU OWE: 3 sheep (with interest)", 340),
    ]
    return building(GRANARY, "history_fertile_crescent", "crescent_granary_door", "THE TEMPLE GRANARY", extra)


def uruk():
    """Uruk and the Bronze Age, 3,000 to 1,100 BCE: the first city, the
    first author with a name, and the Sea Peoples who ended it all at once.
    Ways up to the tin road and down to Egypt."""
    rid = "history_uruk"
    up, down = LINKS[(3, 4)], LINKS[(3, 5)]
    e = [
        start(rid, 480, FLOOR),
        sign("uruk_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: Egypt (trade only, please)", 340),
        npc("history_enheduanna", 620, FLOOR, 62),
        prop_at("tablets", "history_clay_tablets", 740, FLOOR, 64, 48),
        prop_at("ziggurat", "history_ziggurat", 1000, FLOOR, 256, 192),
        ledge("ziggurat_step_a", 880, FLOOR - 96, 240),
        ledge("ziggurat_step_b", 940, FLOOR - 176, 120),
        enemy(rid, 1, "scorpion_man", 1300, FLOOR),
        sign("uruk_sign", 1400, 140, "URUK: the first city (population: too many)", 440),
        sign("uruk_sign_collapse", 1700, FLOOR - 300, "A LATER LAYER, 1177 BCE: the Bronze Age collapsed. Nobody agrees why.", 645),
        enemy(rid, 1, "sea_raider", 1600, FLOOR),
        enemy(rid, 2, "sea_raider", 1760, FLOOR),
        sign("uruk_sign_up", up + HOLE // 2, FLOOR - 260, "UP: the tin road north", 280),
        enemy(rid, 3, "sea_raider", 2400, FLOOR),
        prop_at("palm", "history_date_palm", 2600, FLOOR, 96, 192),
        sign("uruk_sign_east", 2440, FLOOR - 360, "FORWARD: the classical shore, 2,500 years on, by the atlas's road", 609),
    ]
    return grid_room(rid, "URUK AND THE BRONZE AGE", e)


def classical():
    rid = "history_classical_shore"
    gate_x = 1100
    e = [
        start(rid, 160, FLOOR),
        prop_at("column_1", "history_marble_column", 260, FLOOR, 48, 192),
        npc("history_herodotoss", 380, FLOOR, 66),
        prop_at("amphorae", "history_amphorae", 500, FLOOR, 64, 64),
        prop_at("column_2", "history_marble_column", 620, FLOOR, 48, 192),
        prop_at("bust_1", "history_bust", 760, FLOOR, 48, 96),
        npc("history_socratease", 960, FLOOR, 64),
        # The stoa gate: the answer to his question opens it.
        *gate("history_stoa_gate", "the stoa gate", gate_x, f"world.flag_set {KNOWS_NOTHING}", "history_stoa_gate"),
        sign("classical_sign_gate", gate_x + DOOR_W // 2, FLOOR - DOOR_H - 120, "THE STOA: what do you know?", 320),
        sign("classical_sign_down", 1228, FLOOR - 60, "DOWN: Aksum", 160),
        enemy(rid, 1, "hoplite", 1320, FLOOR),
        enemy(rid, 1, "immortal", 1460, FLOOR),
        # The aqueduct: its arches are a high road over the fight.
        prop_at("aqueduct", "history_aqueduct", 1680, FLOOR, 256, 192),
        ledge("aqueduct_step", 1440, FLOOR - 80, 112),
        ledge("aqueduct_top", 1552, FLOOR - 160, 256),
        sign("classical_sign_aqueduct", 1680, FLOOR - 260, "THE AQUEDUCT: water, uphill, for 500 years", 400),
        enemy(rid, 2, "immortal", 1620, FLOOR),
        sign("classical_sign", 1400, 140, "THE CLASSICAL SHORE: democracy, philosophy, plumbing", 500),
        prop_at("broken_column", "history_broken_column", 1960, FLOOR, 96, 96),
        enemy(rid, 1, "vandal", 2040, FLOOR),
        npc("history_julius_sneezer", 2200, FLOOR, 68),
        prop_at("bust_2", "history_bust", 2320, FLOOR, 48, 96),
        enemy(rid, 2, "hoplite", 2420, FLOOR),
        door("classical_colosseum_door", "into the colosseum", 2540, FLOOR, COLOSSEUM, f"{COLOSSEUM}_door"),
        sign("classical_sign_fall", 2320, FLOOR - 260, "ROME: did not fall in a day either", 340),
        sign("classical_sign_guests", 900, FLOOR - 380, "CLIO'S GUESTS: the famous here lived centuries apart. They meet only in the atlas.", 762),
        sign("classical_sign_up", 2680, FLOOR - 340, "UP: the frontier", 200),
    ]
    return grid_room(rid, "THE CLASSICAL SHORE", e)


def colosseum():
    g = INSIDE_FLOOR
    return vigil(COLOSSEUM, "history_classical_shore", "classical_colosseum_door", "THE COLOSSEUM", GAMES,
                 bell="the emperor's gong", store_name="the gate of the emperor's box", chest_name="a thumb (up)",
                 signs=[
                     prop_at("column_1", "history_marble_column", 560, g, 48, 192),
                     ledge("tier_a", 700, g - 96, 200),
                     ledge("tier_b", 960, g - 160, 200),
                     prop_at("column_2", "history_marble_column", 1200, g, 48, 192),
                     sign("colosseum_sign", 800, 124, "THE COLOSSEUM: bread and circuses, mostly circuses", 520),
                     sign("colosseum_sign_box", 1440, g - 200, "THE EMPEROR'S BOX", 260),
                 ])


# ---- R4: Europe --------------------------------------------------------------


def ice_age_europe():
    """Ice Age Europe, 40,000 years ago: mammoths, cave bears, and the
    painted cave by a door. Ways down to the Levant and up the long road
    to the southern route."""
    rid = "history_ice_age_europe"
    up, down = LINKS[(1, 3)], LINKS[(1, 4)]
    e = [
        start(rid, 200, FLOOR),
        sign("ice_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: the Levant (warmer)", 300),
        prop_at("hut", "history_mammoth_bone_hut", 900, FLOOR, 160, 128),
        door("ice_cave_door", "into the painted cave", 1040, FLOOR, CAVE, f"{CAVE}_door"),
        sign("ice_sign_cave", 1064, FLOOR - 150, "THE PAINTED CAVE (bring a lamp)", 320),
        enemy(rid, 1, "cave_bear", 1300, FLOOR),
        ledge("ice_a", 1420, FLOOR - 80, 192),
        ledge("ice_b", 1640, FLOOR - 160, 192),
        enemy(rid, 1, "woolly_mammoth", 1700, FLOOR),
        sign("ice_sign", 1400, 140, "THE ICE AGE: mammoths, caves and the first paintings, 40,000 years ago", 640),
        sign("ice_sign_up", up + HOLE // 2, FLOOR - 260, "UP: India, by the atlas's road (the real one went by the Levant)", 600),
        enemy(rid, 1, "cave_lion", 2400, FLOOR),
        prop_at("hut_2", "history_mammoth_bone_hut", 2600, FLOOR, 160, 128),
    ]
    return grid_room(rid, "THE ICE AGE", e)


def painted_cave():
    g = INSIDE_FLOOR
    extra = [
        prop_at("stencils_1", "history_hand_stencils", 420, g, 128, 128),
        npc("history_cave_painter", 600, g, 62),
        ledge("cave_shelf_a", 760, g - 96, 200),
        prop_at("stencils_2", "history_hand_stencils", 900, g - 96, 128, 128),
        ledge("cave_shelf_b", 1000, g - 176, 200),
        prop_at("stencils_3", "history_hand_stencils", 1220, g, 128, 128),
        chest("the first signature (a handprint)", 1420, g, "currency:150"),
        sign("cave_sign", 800, 124, "THE PAINTED CAVE: lions, horses, and somebody's hand", 520),
    ]
    return building(CAVE, "history_ice_age_europe", "ice_cave_door", "THE PAINTED CAVE", extra)


def first_farmers_of_europe():
    """The first farmers of Europe, 6,000 to 3,000 BCE: wheat came up from
    the Crescent (the way down), and Ötzi walked into a glacier."""
    rid = "history_first_farmers_of_europe"
    down = LINKS[(2, 4)]
    e = [
        start(rid, 160, FLOOR),
        sign("farmers_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: where the wheat came from", 340),
        prop_at("longhouse", "history_longhouse", 760, FLOOR, 192, 128),
        npc("history_otzi_nice_man", 960, FLOOR, 62),
        enemy(rid, 1, "aurochs", 1300, FLOOR),
        ledge("hill_a", 1480, FLOOR - 80, 192),
        ledge("hill_b", 1700, FLOOR - 160, 192),
        enemy(rid, 1, "steppe_wolf", 1600, FLOOR),
        sign("farmers_sign", 1400, 140, "THE FIRST FARMERS OF EUROPE: the wheat walked here from the east", 600),
        prop_at("stone", "history_standing_stone", 2100, FLOOR, 48, 128),
        enemy(rid, 2, "steppe_wolf", 2300, FLOOR),
        prop_at("longhouse_2", "history_longhouse", 2560, FLOOR, 192, 128),
    ]
    return grid_room(rid, "THE FIRST FARMERS OF EUROPE", e)


def megalith_builders():
    """The megalith builders, 2,500 BCE: the stones line up with the sun.
    Pull the solstice lever and the trilithon lets you through
    (`history_solstice`). The way down is the tin road to Uruk."""
    rid = "history_megalith_builders"
    down = LINKS[(3, 4)]
    gate_x = 1500
    e = [
        start(rid, 160, FLOOR),
        prop_at("stone_1", "history_standing_stone", 360, FLOOR, 48, 128),
        npc("history_not_a_druid", 560, FLOOR, 68),
        prop_at("stone_2", "history_standing_stone", 760, FLOOR, 48, 128),
        enemy(rid, 1, "aurochs", 1000, FLOOR),
        prop_at("stone_3", "history_standing_stone", 1200, FLOOR, 48, 128),
        lever(SOLSTICE, "the sun stone", "Wait for midsummer", 1360, FLOOR),
        sign("megalith_sign_sun", 1400, FLOOR - 260, "MIDSUMMER: the sun rises over the heel stone", 420),
        *gate("history_trilithon_gate", "the trilithon", gate_x, f"world.switch_on {SOLSTICE}", "history_trilithon_gate"),
        sign("megalith_sign", 1400, 140, "THE MEGALITH BUILDERS: big stones, careful sums, a lot of beer", 600),
        enemy(rid, 1, "steppe_wolf", 1800, FLOOR),
        sign("megalith_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: the tin road to Uruk", 320),
        enemy(rid, 2, "aurochs", 2400, FLOOR),
        prop_at("stone_4", "history_standing_stone", 2600, FLOOR, 48, 128),
    ]
    return grid_room(rid, "THE MEGALITH BUILDERS", e)


def celtic_frontier():
    """The Celtic frontier, 100 CE: hillforts and the edge of Rome. The
    way down at the east end goes to Rome."""
    rid = "history_celtic_frontier"
    e = [
        start(rid, 160, FLOOR),
        prop_at("palisade_1", "history_palisade", 380, FLOOR, 128, 96),
        npc("history_boudic_car", 600, FLOOR, 70),
        enemy(rid, 1, "vandal", 900, FLOOR),
        ledge("hillfort_a", 1080, FLOOR - 80, 192),
        ledge("hillfort_b", 1300, FLOOR - 160, 192),
        ledge("hillfort_c", 1520, FLOOR - 240, 192),
        prop_at("palisade_2", "history_palisade", 1400, FLOOR, 128, 96),
        enemy(rid, 1, "steppe_wolf", 1700, FLOOR),
        sign("celtic_sign", 1400, 140, "THE CELTIC FRONTIER: hillforts, torcs and a very long wall", 560),
        enemy(rid, 2, "vandal", 2100, FLOOR),
        prop_at("palisade_3", "history_palisade", 2400, FLOOR, 128, 96),
        sign("celtic_sign_down", 2680, FLOOR - 260, "DOWN: Rome", 160),
    ]
    return grid_room(rid, "THE CELTIC FRONTIER", e)


def viking_north():
    """The Viking north, 900 CE: a fjord that a longship crosses, rune
    stones, and Leif Errandson, who got to America first and went home."""
    rid = "history_viking_north"
    stuff, gap = crossing(rid, "the fjord", (1100, 1900), "history_longship", w=192, speed=100.0)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("rune_1", "history_rune_stone", 340, FLOOR, 48, 96),
        npc("history_leif_errandson", 520, FLOOR, 72),
        prop_at("longhouse", "history_longhouse", 760, FLOOR, 192, 128),
        enemy(rid, 1, "viking_raider", 980, FLOOR),
        sign("viking_sign_fjord", 1500, FLOOR - 240, "THE FJORD: the longship goes both ways, like the Vikings", 520),
        sign("viking_sign", 1400, 140, "THE VIKING NORTH: raiders, traders, settlers, poets", 500),
        enemy(rid, 2, "viking_raider", 2040, FLOOR),
        sign("viking_sign_helmet", 2200, FLOOR - 200, "NOTE: no horns on the helmets. Ever.", 360),
        prop_at("rune_2", "history_rune_stone", 2400, FLOOR, 48, 96),
        enemy(rid, 3, "viking_raider", 2560, FLOOR),
    ]
    return grid_room(rid, "THE VIKING NORTH", e, gaps=[gap])


def sail():
    rid = "history_age_of_sail"
    h0, h1 = 960, 1440
    e = [
        entity("HazardBlock", h0, H - 16, h1 - h0, 16, name="the harbour"),
        # The ships' booms: spars that rock on their middle over the water.
        *[entity("MovingPlatform", x, FLOOR - 64, 192, 16, id=f"history_sail_boom_{n}", name="a ship's boom", speed=0.0,
                 line=True, swing=16.0, swing_period=3.4, visual="history_ship_boom") for n, x in enumerate((1000, 1240))],
        start(rid, 160, FLOOR),
        npc("history_vasco_da_gamer", 300, FLOOR, 66),
        prop_at("astrolabe", "history_astrolabe", 420, FLOOR, 64, 96),
        npc("history_christopher_columbust", 560, FLOOR, 68),
        prop_at("mast_1", "history_mast", 700, FLOOR, 96, 256),
        prop_at("crates_1", "history_cargo_crates", 840, FLOOR, 96, 64),
        sign("sail_sign_exchange", 840, FLOOR - 140, "THE COLUMBIAN EXCHANGE: potatoes east, horses west", 460),
        sign("sail_sign_harbour", (h0 + h1) // 2, FLOOR - 220, "MIND THE BOOM", 240),
        sign("sail_sign", 1400, 140, "THE AGE OF SAIL: the world, joined up (not gently)", 480),
        enemy(rid, 1, "privateer", 1560, FLOOR),
        # A plain sign; this part of the story is not a joke.
        sign("sail_sign_middle_passage", 1720, FLOOR - 300,
             "THE MIDDLE PASSAGE: more than twelve million people were taken across this sea in chains. Remember them.", 640),
        enemy(rid, 1, "scurvy_ghost", 1800, FLOOR - 220),
        enemy(rid, 1, "kraken_arm", 1960, FLOOR),
        ledge("yard_a", 2040, FLOOR - 80, 160),
        ledge("yard_b", 2200, FLOOR - 160, 160),
        enemy(rid, 2, "scurvy_ghost", 2240, FLOOR - 340),
        npc("history_captain_cookbook", 2300, FLOOR, 68),
        enemy(rid, 2, "privateer", 2440, FLOOR),
        prop_at("lighthouse", "history_lighthouse", 2600, FLOOR, 96, 256),
    ]
    return grid_room(rid, "THE AGE OF SAIL", e, gaps=[(h0, h1)])


def industrial():
    rid = "history_industrial_age"
    stuff, gap = crossing(rid, "the railway cutting", (1700, 2300), "history_rail_wagon", speed=120.0)
    gate_x = 1300
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("lamp_1", "history_gas_lamp", 240, FLOOR, 32, 160),
        npc("history_adam_smithy", 360, FLOOR, 68),
        prop_at("chimney", "history_chimney", 480, FLOOR, 64, 256),
        npc("history_karl_marks", 620, FLOOR, 68),
        npc("history_general_ludd", 800, FLOOR, 66),
        prop_at("lamp_2", "history_gas_lamp", 900, FLOOR, 32, 160),
        door("industrial_mill_door", "into the cotton mill", 1000, FLOOR, MILL, f"{MILL}_door"),
        sign("industrial_sign_mill", 1024, FLOOR - 150, "THE COTTON MILL", 260),
        # The factory gate: the engine in the mill opens it.
        *gate("history_factory_gate", "the factory gate", gate_x, f"world.switch_on {BOILER}", "history_factory_gate"),
        sign("industrial_sign_gate", gate_x + DOOR_W // 2, FLOOR - DOOR_H - 120, "GATE OPENS WHEN THE ENGINE RUNS", 380),
        enemy(rid, 1, "steam_automaton", 1480, FLOOR),
        enemy(rid, 1, "soot_sprite", 1560, FLOOR - 240),
        enemy(rid, 1, "robber_baron", 1620, FLOOR),
        sign("industrial_sign_rail", 2000, FLOOR - 240, "THE RAILWAY: time itself was set by it", 400),
        sign("industrial_sign", 1400, 140, "THE INDUSTRIAL AGE: more of everything, especially smoke", 520),
        enemy(rid, 2, "soot_sprite", 2000, FLOOR - 320),
        enemy(rid, 2, "robber_baron", 2400, FLOOR),
        prop_at("engine", "history_steam_engine", 2480, FLOOR, 160, 160),
        chest("the twentieth century (some assembly required)", 2660, FLOOR, "currency:200"),
        sign("industrial_sign_end", 2480, FLOOR - 260, "TO BE CONTINUED: the twentieth century is under construction", 560),
    ]
    return grid_room(rid, "THE INDUSTRIAL AGE", e, gaps=[gap])


def mill():
    g = INSIDE_FLOOR
    extra = [
        prop_at("engine", "history_steam_engine", 520, g, 160, 160),
        lever(BOILER, "the boiler", "Stoke the boiler", 660, g),
        sign("mill_sign_boiler", 660, g - 200, "THE ENGINE: it runs the mill, and the gate", 400),
        ledge("catwalk_a", 800, g - 96, 200),
        ledge("catwalk_b", 1040, g - 176, 200),
        prop_at("chimney", "history_chimney", 1220, g, 64, 256),
        chest("a share in the mill", 1440, g, "currency:150"),
        sign("mill_sign", 800, 124, "THE COTTON MILL: 14 hours a day, 6 days a week", 480),
    ]
    return building(MILL, "history_industrial_age", "industrial_mill_door", "THE COTTON MILL", extra)


# ---- R2½: Southeast Asia --------------------------------------------------------


def sulawesi():
    """Sulawesi, about 51,000 years ago: the limestone karst of Maros-Pangkep,
    where the oldest known painted story is (a pig and three hunters, at
    least 51,200 years old). Wallacea was never joined to Asia or to Sahul,
    so everyone here came by sea. The way up from the southern route comes
    in at the east; the way on to Sahul is up a tower at the west."""
    rid = "history_sulawesi"
    up, down = LINKS[(1, 2)], LINKS[(1, 2.5)]
    e = [
        start(rid, 2600, FLOOR),
        sign("sulawesi_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: the southern route", 240),
        sign("sulawesi_sign_up", up + HOLE // 2, FLOOR - 300, "UP: Sahul, across more sea", 258),
        enemy(rid, 1, "sulawesi_warty_pig", 2000, FLOOR),
        # The karst: a tower of limestone, climbed by its shelves.
        ledge("karst_shelf_a", 1400, FLOOR - 72, 140),
        ledge("karst_shelf_b", 1470, FLOOR - 144, 120),
        solid("karst", 1580, FLOOR - 216, 260, 216),
        chest("a lump of red ochre", 1710, FLOOR - 216, "currency:60"),
        sign("sulawesi_sign_wallacea", 1710, FLOOR - 400, "WALLACEA: never joined to Asia or to Sahul. Everyone came by sea.", 609),
        # The cave: its roof hangs over the painted wall.
        solid("cave_roof", 820, CEILING, 520, FLOOR - 200 - CEILING),
        prop_at("pig_painting", "history_pig_painting", 1080, FLOOR - 40, 128, 64),
        prop_at("hand_stencils", "history_sulawesi_hand_stencils", 1260, FLOOR - 48, 64, 48),
        npc("history_pig_painter", 940, FLOOR, 61),
        sign("sulawesi_sign_painting", 1080, FLOOR - 160, "A PIG AND THREE HUNTERS: the oldest story in paint, 51,200 years old. No ending yet.", 780),
        enemy(rid, 2, "sulawesi_warty_pig", 420, FLOOR),
        sign("sulawesi_sign", 1400, 140, "SULAWESI: the oldest painted story, 51,200 years ago", 500),
    ]
    return grid_room(rid, "SULAWESI", e)


def austronesian_islands():
    """The Austronesian islands, about 2000 BCE: the people who sailed out of
    Taiwan by outrigger canoe reach the Philippines and Indonesia. Their
    languages now run from Madagascar to Rapa Nui. An outrigger canoe
    crosses the lagoon; the way up is New Guinea's coast, where their
    grandchildren will be the Lapita."""
    rid = "history_austronesian_islands"
    stuff, gap = crossing(rid, "the lagoon (crocodiles)", (900, 1500), "history_outrigger_canoe", w=176)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("stilt_house_1", "history_bamboo_stilt_house", 300, FLOOR, 96, 112),
        npc("history_word_keeper", 520, FLOOR, 63),
        prop_at("pots", "history_red_slip_pots", 640, FLOOR, 48, 32),
        sign("austronesia_sign_mata", 500, FLOOR - 220, "MATA: 'eye' in Malay, Tagalog, Fijian and Maori; maso in Malagasy, maka in Hawaiian. One word.", 864),
        sign("austronesia_sign_lagoon", 1200, FLOOR - 240, "OUT OF TAIWAN: island to island, by outrigger canoe", 483),
        enemy(rid, 1, "saltwater_crocodile", 1620, FLOOR),
        prop_at("stilt_house_2", "history_bamboo_stilt_house", 1820, FLOOR, 96, 112),
        ledge("house_floor", 1772, FLOOR - 64, 96),
        sign("austronesia_sign_up", LINKS[(2, 2)] + HOLE // 2, FLOOR - 300, "UP: New Guinea. Their grandchildren will be the Lapita.", 519),
        enemy(rid, 1, "coral_crab", 2300, FLOOR),
        prop_at("pots_2", "history_red_slip_pots", 2560, FLOOR, 48, 32),
        sign("austronesia_sign", 1400, 140, "THE AUSTRONESIANS: out of Taiwan, to half the world, 2000 BCE", 600),
    ]
    return grid_room(rid, "THE AUSTRONESIAN ISLANDS", e, gaps=[gap])


def paddy(rid, n, x0, x1):
    """A flooded rice paddy: a cut in the ground with a bed of mud under
    shallow water, to wade. Returns the entities and the gap."""
    return [
        solid(f"paddy_bed_{n}", x0, FLOOR + 40, x1 - x0, H - FLOOR - 40),
        water(f"{rid}_paddy_{n}", x0, FLOOR - 8, x1 - x0, 48),
    ], (x0, x1)


def dong_son():
    """Dong Son, about 600 BCE to 43 CE: the Red River delta of northern
    Vietnam: flooded paddies and dikes, stilt houses and the great bronze
    drums, found from China to Indonesia. In 40 CE the Trung sisters led a
    rising against Han rule and held the land for three years."""
    rid = "history_dong_son"
    p1, g1 = paddy(rid, 1, 700, 1100)
    p2, g2 = paddy(rid, 2, 1300, 1720)
    e = [
        *p1, *p2,
        start(rid, 160, FLOOR),
        npc("history_drum_caster", 400, FLOOR, 66),
        prop_at("drum", "history_bronze_drum", 520, FLOOR, 48, 40),
        sign("dongson_sign_drum", 460, FLOOR - 200, "THE DRUMS: cast in bronze, found from China to Indonesia. Loud diplomacy.", 681),
        prop_at("stilt_house", "history_bamboo_stilt_house", 1200, FLOOR, 96, 112),
        ledge("stilt_floor", 1152, FLOOR - 64, 96),
        enemy(rid, 1, "saltwater_crocodile", 1460, FLOOR + 40),
        sign("dongson_sign_rice", 1200, FLOOR - 260, "THE PADDIES: two harvests a year, if the river agrees", 501),
        prop_at("sheaves", "history_rice_sheaves", 1880, FLOOR, 64, 32),
        enemy(rid, 1, "tiger", 1960, FLOOR),
        npc("history_trung_trac", 2200, FLOOR, 64),
        npc("history_trung_nhi", 2320, FLOOR, 62),
        sign("dongson_sign_trung_a", 2260, FLOOR - 300, "40 CE: the Trưng sisters raised an army against Han rule and held the land three years.", 807),
        sign("dongson_sign_trung_b", 2260, FLOOR - 272, "The legend says they rode elephants. The temples to them still stand.", 645),
        ledge("hill_a", 2440, FLOOR - 80, 160),
        ledge("hill_b", 2600, FLOOR - 160, 140),
        chest("a drum's bronze star", 2670, FLOOR - 160, "currency:100"),
        enemy(rid, 2, "tiger", 2560, FLOOR),
        sign("dongson_sign", 1400, 140, "DONG SON: rice, rivers and bronze drums, 600 BCE to 43 CE", 560),
    ]
    return grid_room(rid, "DONG SON", e, gaps=[g1, g2])


# ---- R3: South Asia ----------------------------------------------------------


def southern_route():
    """The southern route, 65,000 years ago: along the coast of India, the
    fastest migration in prehistory. Ways up to Sulawesi (by the coasts and
    the sea) and down to Europe (the atlas's road)."""
    rid = "history_southern_route"
    up, down = LINKS[(1, 2.5)], LINKS[(1, 3)]
    e = [
        start(rid, 200, FLOOR),
        sign("route_sign_up", up + 160, FLOOR - 260, "UP: Sulawesi and the islands of Wallacea", 384),
        npc("history_coast_walker", 900, FLOOR, 64),
        enemy(rid, 1, "coral_crab", 1100, FLOOR),
        ledge("rocks_a", 1240, FLOOR - 80, 192),
        ledge("rocks_b", 1460, FLOOR - 160, 192),
        enemy(rid, 1, "tiger", 1600, FLOOR),
        sign("route_sign", 1400, 140, "THE SOUTHERN ROUTE: along the coast, one beach at a time", 520),
        sign("route_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: Europe, by the atlas's road", 321),
        enemy(rid, 2, "coral_crab", 2160, FLOOR),
        prop_at("hut", "history_mud_hut", 2690, FLOOR, 128, 96),
    ]
    return grid_room(rid, "THE SOUTHERN ROUTE", e)


def mehrgarh():
    """Mehrgarh, 7,000 BCE: barley, cotton, and the first dentist."""
    rid = "history_mehrgarh"
    e = [
        start(rid, 160, FLOOR),
        prop_at("hut_1", "history_mud_hut", 400, FLOOR, 128, 96),
        npc("history_dentist_of_mehrgarh", 600, FLOOR, 65),
        prop_at("rice", "history_rice_sheaf", 740, FLOOR, 48, 64),
        enemy(rid, 1, "locust_swarm", 1000, FLOOR - 260),
        ledge("terrace_a", 1100, FLOOR - 80, 224),
        ledge("terrace_b", 1360, FLOOR - 160, 224),
        enemy(rid, 1, "tiger", 1500, FLOOR),
        sign("mehrgarh_sign", 1400, 140, "MEHRGARH: barley, cotton and the first dentist, 7,000 BCE", 560),
        sign("mehrgarh_sign_teeth", 600, FLOOR - 200, "DRILLED TEETH: 9,000 years old. Open wide.", 420),
        prop_at("hut_2", "history_mud_hut", 2000, FLOOR, 128, 96),
        enemy(rid, 2, "tiger", 2300, FLOOR),
        prop_at("rice_2", "history_rice_sheaf", 2560, FLOOR, 48, 64),
    ]
    return grid_room(rid, "MEHRGARH", e)


def indus_cities():
    """The Indus cities, 2,500 BCE: drains, bricks, and a script that nobody
    can read. The citadel's gate opens for the unicorn seal
    (`history_unicorn_seal`), which is up on the granary's roof."""
    rid = "history_indus_cities"
    gate_x = 1500
    e = [
        start(rid, 160, FLOOR),
        npc("history_priest_king_probably", 420, FLOOR, 66),
        prop_at("bath", "history_great_bath", 700, FLOOR, 192, 96),
        sign("indus_sign_bath", 700, FLOOR - 160, "THE GREAT BATH: waterproof bricks, 4,500 years old", 460),
        enemy(rid, 1, "indus_unicorn", 1000, FLOOR),
        ledge("granary_a", 1080, FLOOR - 80, 160),
        ledge("granary_b", 1240, FLOOR - 160, 160),
        ledge("granary_roof", 1080, FLOOR - 240, 192),
        entity("PickupSpawn", 1176 - 17, FLOOR - 240 - 40 - 17, 34, 34, name="the unicorn seal",
               kind=f"flag:{UNICORN_SEAL}", respawn="Never", sprite="history_unicorn_seal"),
        *gate("history_seal_gate", "the citadel gate", gate_x, f"world.flag_set {UNICORN_SEAL}", "history_seal_gate"),
        sign("indus_sign_gate", gate_x + DOOR_W // 2, FLOOR - DOOR_H - 120, "THE CITADEL: show your seal", 320),
        sign("indus_sign", 1400, 140, "THE INDUS CITIES: the best drains of the ancient world", 520),
        enemy(rid, 2, "indus_unicorn", 1900, FLOOR),
        enemy(rid, 1, "tiger", 2200, FLOOR),
        prop_at("hut", "history_mud_hut", 2560, FLOOR, 128, 96),
    ]
    return grid_room(rid, "THE INDUS CITIES", e)


# ---- R2: Oceania and the islands ---------------------------------------------


def sahul():
    """Sahul, 65,000 years ago: the first great sea crossing, to the land
    that is now Australia and New Guinea. A bamboo raft carries you over the
    strait."""
    rid = "history_sahul"
    up, down = LINKS[(1, 1)], LINKS[(1, 2)]
    stuff, gap = crossing(rid, "the strait", (1000, 1700), "history_bamboo_raft")
    e = [
        *stuff,
        start(rid, 200, FLOOR),
        sign("sahul_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: Sulawesi, the island before this one", 402),
        npc("history_raft_builder", 860, FLOOR, 66),
        sign("sahul_sign_strait", 1350, FLOOR - 240, "THE STRAIT: out of sight of land, 65,000 years ago", 500),
        sign("sahul_sign", 1400, 140, "SAHUL: the first great sea crossing", 400),
        prop_at("boab", "history_boab_tree", 1860, FLOOR, 160, 192),
        enemy(rid, 1, "diprotodon", 1960, FLOOR),
        sign("sahul_sign_up", up + HOLE // 2, FLOOR - 260, "UP: the mammoth steppe, by the atlas's road", 411),
        prop_at("shelter", "history_rock_shelter", 2400, FLOOR, 160, 128),
        enemy(rid, 1, "megalania", 2300, FLOOR),
        prop_at("boab_2", "history_boab_tree", 2640, FLOOR, 160, 192),
    ]
    return grid_room(rid, "SAHUL", e, gaps=[gap])


def new_guinea_highlands():
    """The New Guinea highlands, 7,000 BCE: one of the places farming was
    invented on its own: taro, bananas and sugar cane in the Kuk swamp."""
    rid = "history_new_guinea_highlands"
    e = [
        start(rid, 160, FLOOR),
        prop_at("fern_1", "history_tree_fern", 360, FLOOR, 96, 192),
        npc("history_kuk_gardener", 560, FLOOR, 62),
        enemy(rid, 1, "cassowary_history", 900, FLOOR),
        ledge("garden_a", 1080, FLOOR - 80, 192),
        ledge("garden_b", 1300, FLOOR - 160, 192),
        ledge("garden_c", 1520, FLOOR - 240, 192),
        prop_at("fern_2", "history_tree_fern", 1400, FLOOR, 96, 192),
        enemy(rid, 2, "cassowary_history", 1800, FLOOR),
        sign("guinea_sign", 1400, 140, "THE NEW GUINEA HIGHLANDS: farming, invented again, by 8000 BCE", 620),
        sign("guinea_sign_kuk", 560, FLOOR - 200, "KUK SWAMP: drained by hand, gardened ever since", 440),
        prop_at("fern_3", "history_tree_fern", 2300, FLOOR, 96, 192),
        enemy(rid, 3, "cassowary_history", 2500, FLOOR),
        sign("guinea_sign_down", LINKS[(2, 2)] + HOLE // 2, FLOOR - 260, "DOWN: the Austronesians, who came by sea from Taiwan", 492),
    ]
    return grid_room(rid, "THE NEW GUINEA HIGHLANDS", e)


def lapita_voyagers():
    """The Lapita voyagers, 1,500 BCE: stamped pots, stilt houses over the
    lagoon, and the first people to sail out of sight of land into the
    remote Pacific. Their ancestors came from Taiwan: the way up is the
    way back to East Asia. An outrigger canoe crosses the lagoon."""
    rid = "history_lapita_voyagers"
    up = LINKS[(3, 1)]
    stuff, gap = crossing(rid, "the lagoon", (1000, 1600), "history_outrigger_canoe", w=176)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("house", "history_stilt_house", 400, FLOOR, 128, 160),
        npc("history_lapita_potter", 620, FLOOR, 63),
        prop_at("pot", "history_lapita_pot", 740, FLOOR, 48, 48),
        sign("lapita_sign_pot", 700, FLOOR - 200, "LAPITA POTS: stamped with a toothed comb. Find one, and you know who came by.", 700),
        prop_at("palm_1", "history_coconut_palm", 880, FLOOR, 96, 192),
        sign("lapita_sign_lagoon", 1300, FLOOR - 260, "THE LAGOON: houses on stilts, canoes to the reef", 440),
        sign("lapita_sign_up", up + HOLE // 2, FLOOR - 300, "UP: the Shang court, by the atlas's road (the ancestors came from Taiwan)", 681),
        sign("lapita_sign", 1400, 140, "THE LAPITA VOYAGERS: out of sight of land, into the open Pacific, 1,500 BCE", 720),
        enemy(rid, 1, "coconut_crab", 2050, FLOOR),
        prop_at("palm_2", "history_coconut_palm", 2200, FLOOR, 96, 192),
        enemy(rid, 1, "coral_crab", 2350, FLOOR),
        enemy(rid, 2, "coconut_crab", 2560, FLOOR),
    ]
    return grid_room(rid, "THE LAPITA VOYAGERS", e, gaps=[gap])


def wayfinders():
    """The wayfinders: the Polynesian voyagers who found every island of the
    Pacific by stars, swells and birds. A double-hulled voyaging canoe
    crosses the open sea; frigatebirds attack on the way."""
    rid = "history_wayfinders"
    stuff, gap = crossing(rid, "the open sea", (1000, 2300), "history_voyaging_canoe", w=208, speed=110.0)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("compass", "history_star_compass", 460, FLOOR, 128, 32),
        sign("wayfinders_sign_compass", 460, FLOOR - 160, "THE STAR COMPASS: where each star rises and sets (in the navigator's head)", 680),
        npc("history_wayfinder", 680, FLOOR, 67),
        prop_at("breadfruit", "history_breadfruit_tree", 860, FLOOR, 128, 160),
        sign("wayfinders_sign_birds", 1500, FLOOR - 340, "AT DUSK THE TERNS FLY HOME TO LAND: follow them", 480),
        enemy(rid, 1, "frigatebird", 1350, FLOOR - 300),
        enemy(rid, 2, "frigatebird", 1800, FLOOR - 360),
        enemy(rid, 3, "frigatebird", 2150, FLOOR - 280),
        sign("wayfinders_sign", 1400, 140, "THE WAYFINDERS: across the Pacific with no instruments but the sky, the swells and the birds", 820),
        prop_at("palm", "history_coconut_palm", 2480, FLOOR, 96, 192),
        enemy(rid, 1, "coconut_crab", 2640, FLOOR),
    ]
    return grid_room(rid, "THE WAYFINDERS", e, gaps=[gap])


def rapa_nui():
    """Rapa Nui, 1200 CE on: the moai, carved in the crater and walked
    upright with ropes; rongorongo, which nobody can read; and the sweet
    potato from South America. Ride a walking moai over the quarry ditch.
    At the east end, a plain sign says what happened in 1862."""
    rid = "history_rapa_nui"
    top = FLOOR - 176
    stuff, gap = crossing(rid, "the quarry ditch", (1400, 1900), "history_walking_moai", w=40, speed=40.0, top=top)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("moai_1", "history_moai", 400, FLOOR, 96, 192),
        prop_at("moai_2", "history_moai", 540, FLOOR, 96, 192),
        npc("history_moai_carver", 740, FLOOR, 66),
        prop_at("tablet", "history_rongorongo_tablet", 880, FLOOR, 48, 32),
        sign("rapa_nui_sign_tablet", 880, FLOOR - 160, "RONGORONGO: a script nobody can read (yet)", 400),
        prop_at("kumara", "history_kumara", 1020, FLOOR, 64, 48),
        sign("rapa_nui_sign_kumara", 1020, FLOOR - 260, "KUMARA: the sweet potato, from South America. Somebody went there and came back.", 720),
        npc("history_thor_hey_im_right_dahl", 1160, FLOOR, 73),
        # Up to the moai's head, over the ditch on it, and down.
        ledge("ditch_step_a", 1240, FLOOR - 88, 96),
        ledge("ditch_step_b", 1304, top, 96),
        sign("rapa_nui_sign_walk", 1650, top - 160, "THE MOAI WALKED: rocked upright with ropes, as the island's stories say", 600),
        ledge("ditch_far", 1900, top, 96),
        ledge("ditch_far_step", 1996, FLOOR - 88, 96),
        sign("rapa_nui_sign", 1400, 140, "RAPA NUI: the most remote inhabited island, settled about 1200 CE", 620),
        enemy(rid, 1, "pacific_rat", 2160, FLOOR),
        enemy(rid, 2, "pacific_rat", 2260, FLOOR),
        enemy(rid, 3, "pacific_rat", 2360, FLOOR),
        prop_at("moai_3", "history_moai", 2560, FLOOR, 96, 192),
        sign("rapa_nui_sign_1862_a", 2400, FLOOR - 340, "1862: slave ships took about 1,500 people; disease took most of the rest.", 680),
        sign("rapa_nui_sign_1862_b", 2400, FLOOR - 312, "The story that the islanders ruined their own island is mostly wrong.", 640),
    ]
    return grid_room(rid, "RAPA NUI", e, gaps=[gap])


# ---- R1: East Asia and the steppe ---------------------------------------------


def mammoth_steppe():
    """The mammoth steppe, 40,000 years ago: Denisovans, mammoths and the
    road north-east. Ways up to Beringia and down to Sahul."""
    rid = "history_mammoth_steppe"
    up, down = LINKS[(1, 0)], LINKS[(1, 1)]
    e = [
        start(rid, 200, FLOOR),
        sign("mammoth_sign_up", up + HOLE // 2, FLOOR - 260, "UP: Beringia, the land bridge", 340),
        npc("history_denny_denisovan", 900, FLOOR, 65),
        prop_at("hut", "history_mammoth_bone_hut", 1080, FLOOR, 160, 128),
        enemy(rid, 1, "woolly_mammoth", 1400, FLOOR),
        ledge("drift_a", 1560, FLOOR - 80, 192),
        ledge("drift_b", 1780, FLOOR - 160, 192),
        enemy(rid, 1, "cave_bear", 1800, FLOOR),
        sign("mammoth_sign", 1400, 140, "THE MAMMOTH STEPPE: grass and ice from Spain to Alaska", 540),
        sign("mammoth_sign_down", down + HOLE // 2, FLOOR - 260, "DOWN: Sahul, by the atlas's road", 312),
        enemy(rid, 1, "steppe_wolf", 2400, FLOOR),
    ]
    return grid_room(rid, "THE MAMMOTH STEPPE", e)


def yellow_river():
    """The Yellow River, 6,000 BCE: millet in the north, rice in the south,
    silk from a teacup. A reed boat crosses the river."""
    rid = "history_yellow_river"
    stuff, gap = crossing(rid, "the Yellow River", (1100, 1800), "history_reed_boat")
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("rice_1", "history_rice_sheaf", 340, FLOOR, 48, 64),
        npc("history_leizu", 560, FLOOR, 62),
        enemy(rid, 1, "silk_moth", 800, FLOOR - 260),
        prop_at("hut", "history_mud_hut", 940, FLOOR, 128, 96),
        sign("river_sign_river", 1450, FLOOR - 240, "THE YELLOW RIVER: China's sorrow, and its cradle", 480),
        sign("river_sign", 1400, 140, "THE YELLOW RIVER: millet, rice and silk", 420),
        enemy(rid, 1, "coral_crab", 1960, FLOOR),
        enemy(rid, 2, "silk_moth", 2200, FLOOR - 320),
        prop_at("rice_2", "history_rice_sheaf", 2400, FLOOR, 48, 64),
        enemy(rid, 2, "coral_crab", 2560, FLOOR),
    ]
    return grid_room(rid, "THE YELLOW RIVER", e, gaps=[gap])


def shang_court():
    """The Shang court, 1,200 BCE: bronze, and the first Chinese writing on
    oracle bones. Crack a bone over the fire and the bronze gate opens
    (`history_oracle_bone`)."""
    rid = "history_shang_court"
    gate_x = 1500
    e = [
        start(rid, 160, FLOOR),
        prop_at("ding_1", "history_bronze_ding", 380, FLOOR, 96, 96),
        npc("history_fu_hao", 600, FLOOR, 64),
        prop_at("bones", "history_oracle_bones", 760, FLOOR, 64, 48),
        enemy(rid, 1, "taotie_spirit", 1000, FLOOR - 240),
        lever(ORACLE, "the oracle bone", "Crack the oracle bone", 1300, FLOOR),
        sign("shang_sign_oracle", 1300, FLOOR - 200, "ASK THE ANCESTORS: will the gate open? (crack and see)", 520),
        *gate("history_bronze_gate", "the bronze gate", gate_x, f"world.switch_on {ORACLE}", "history_bronze_gate"),
        sign("shang_sign", 1400, 140, "THE SHANG COURT: bronze, bones and the first Chinese writing", 560),
        sign("shang_sign_down", LINKS[(3, 1)] + HOLE // 2, FLOOR - 260, "DOWN: the Lapita, by the atlas's road. Their ancestors sailed from Taiwan.", 690),
        enemy(rid, 2, "taotie_spirit", 1900, FLOOR - 300),
        ledge("hall_a", 2000, FLOOR - 80, 192),
        ledge("hall_b", 2220, FLOOR - 160, 192),
        enemy(rid, 1, "steppe_wolf", 2300, FLOOR),
        prop_at("ding_2", "history_bronze_ding", 2560, FLOOR, 96, 96),
    ]
    return grid_room(rid, "THE SHANG COURT", e)


def silk():
    rid = "history_silk_road"
    stuff, gap = crossing(rid, "the quicksand", (1100, 1900), "history_camel_caravan", speed=70.0)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("lantern_1", "history_paper_lantern", 260, FLOOR, 32, 128),
        npc("history_marco_polo_neck", 380, FLOOR, 66),
        prop_at("silk", "history_silk_bolts", 500, FLOOR, 80, 64),
        enemy(rid, 1, "desert_bandit", 640, FLOOR),
        npc("history_confusius", 760, FLOOR, 64),
        prop_at("spice", "history_spice_sacks", 880, FLOOR, 80, 48),
        enemy(rid, 1, "silk_moth", 980, FLOOR - 260),
        sign("silk_sign_sand", 1500, FLOOR - 260, "QUICKSAND: ride the camels (they know the way)", 460),
        sign("silk_sign", 1400, 140, "THE SILK ROAD: 4,000 miles of middlemen", 420),
        sign("silk_sign_guests", 700, FLOOR - 380, "CLIO'S GUESTS: Confucius and Marco Polo lived 1,700 years apart.", 600),
        enemy(rid, 2, "silk_moth", 1500, FLOOR - 380),
        enemy(rid, 1, "terracotta_warrior", 2040, FLOOR),
        prop_at("pagoda", "history_pagoda", 2240, FLOOR, 128, 256),
        ledge("pagoda_eave_a", 2140, FLOOR - 80, 200),
        ledge("pagoda_eave_b", 2180, FLOOR - 160, 120),
        enemy(rid, 2, "desert_bandit", 2400, FLOOR),
        enemy(rid, 2, "terracotta_warrior", 2540, FLOOR),
        prop_at("lantern_2", "history_paper_lantern", 2660, FLOOR, 32, 128),
    ]
    return grid_room(rid, "THE SILK ROAD", e, gaps=[gap])


def steppe():
    """The Great Wall at the west end, and two roads through it: Genghis
    Can's gate (short, through the garrison) and Genghis Can't's scaffold
    (long, over it)."""
    rid = "history_mongol_steppe"
    w0, w1 = WALL_X
    g0, g1 = WALL_GATE
    gate_top = FLOOR - DOOR_H
    e = [
        start(rid, 160, FLOOR),
        prop_at("ger_1", "history_ger", 300, FLOOR, 160, 128),
        npc("history_genghis_can", 440, FLOOR, 67, catalog="npc_genghis_can"),
        npc("history_genghis_cant", 560, FLOOR, 66, catalog="npc_genghis_cant"),
        prop_at("standard", "history_horse_standard", 640, FLOOR, 32, 160),
        # The Great Wall: its top, and the gate in its foot.
        prop_at("great_wall", "history_great_wall", (w0 + w1) // 2, FLOOR, 256, 192),
        solid("wall_top", w0, WALL_TOP, w1 - w0, gate_top - WALL_TOP),
        solid("wall_west", w0, gate_top, g0 - w0, DOOR_H),
        lock_wall("history_wall_gate", "the gate in the Great Wall", g0, gate_top, g1 - g0, DOOR_H,
                  f"world.flag_set {CAN_ROAD}", "history_great_wall_gate"),
        solid("wall_east", g1, gate_top, w1 - g1, DOOR_H),
        # Genghis Can't's scaffold up the west face: it stands while his flag
        # is set.
        *[lock_wall(f"history_wall_scaffold_{n}", "the scaffold", x, y, 160, 24, f"not world.flag_set {CANT_ROAD}",
                    "history_scaffold_plank") for n, (x, y) in enumerate(((744, FLOOR - 80), (568, FLOOR - 160), (744, FLOOR - 240)))],
        sign("steppe_sign_wall", (w0 + w1) // 2, WALL_TOP - 120, "THE GREAT WALL: through it, or over it?", 400),
        # Over it: the rampart walk east, and the steps down.
        ledge("rampart", w1, WALL_TOP, 480),
        ledge("rampart_step_a", 1520, FLOOR - 240, 160),
        ledge("rampart_step_b", 1720, FLOOR - 160, 160),
        ledge("rampart_step_c", 1920, FLOOR - 80, 160),
        # Through it: the garrison waits at the foot.
        enemy(rid, 1, "horse_archer", 1160, FLOOR),
        enemy(rid, 1, "steppe_wolf", 1300, FLOOR),
        enemy(rid, 2, "steppe_wolf", 1380, FLOOR),
        enemy(rid, 2, "horse_archer", 1600, FLOOR),
        enemy(rid, 1, "golden_eagle", 1800, FLOOR - 400),
        sign("steppe_sign", 1560, 140, "THE MONGOL STEPPE: the largest land empire, on horseback", 500),
        prop_at("ovoo", "history_ovoo", 2160, FLOOR, 64, 96),
        enemy(rid, 2, "golden_eagle", 2300, FLOOR - 300),
        prop_at("ger_2", "history_ger", 2420, FLOOR, 160, 128),
        door("steppe_ger_door", "into the great ger", 2620, FLOOR, GER, f"{GER}_door"),
        sign("steppe_sign_ger", 2560, FLOOR - 200, "THE GREAT GER: the capital (it moves)", 380),
    ]
    return grid_room(rid, "THE MONGOL STEPPE", e)


def great_ger():
    g = INSIDE_FLOOR
    extra = [
        prop_at("standard", "history_horse_standard", 360, g, 32, 160),
        npc("history_kublai_khant_wait", 520, g, 66),
        sign("ger_sign_yam", 800, 150, "THE YAM: a fresh horse every 25 miles, a letter across Asia in a month", 600),
        ledge("ger_bench", 900, g - 96, 200),
        prop_at("ovoo", "history_ovoo", 1180, g, 64, 96),
        chest("a paiza (a passport of gold)", 1420, g, "currency:150"),
        sign("ger_sign", 800, 124, "THE GREAT GER: the empire's office, portable", 460),
    ]
    return building(GER, "history_mongol_steppe", "steppe_ger_door", "THE GREAT GER", extra)


# ---- R0: the Americas ----------------------------------------------------------


def beringia():
    """Beringia, 25,000 to 15,000 years ago, in two floors. The way down at
    the west is the land bridge back to Asia, where people may have lived for
    thousands of years, cut off by the ice. Then two ways south, both real,
    and which came first is still argued: along the floor, the Pacific coast
    under the ice, east to South America (Monte Verde, older than Clovis);
    up the ice, the inland way, east at the top to the first maize."""
    rid = "history_beringia"
    down = LINKS[(1, 0)]
    g = FLOOR + H
    shelf = 2140
    e = [
        start(rid, 200, g),
        sign("beringia_sign_down", down + HOLE // 2, g - 260, "DOWN: Asia, across the land bridge", 360),
        enemy(rid, 1, "woolly_mammoth", 1000, g),
        sign("beringia_sign_standstill_a", 1160, g - 300, "THE BERINGIAN STANDSTILL: their DNA suggests people lived here", 600),
        sign("beringia_sign_standstill_b", 1160, g - 272, "for thousands of years, cut off from Asia and the south by ice.", 600),
        enemy(rid, 1, "smilodon", 1400, g),
        # Up the face of the ice: shelves, back and forth, a jump apart; the
        # top one beside the top of the ice.
        *[ledge(f"ice_{n}", 1960 if n % 2 else 1760, g - 72 * (n + 1), 176) for n in range(8)],
        # The top of the ice: the inland way south. Under it, the coast.
        solid("ice_top", shelf, FLOOR, W - shelf, 48),
        sign("beringia_sign_inland", 2440, FLOOR - 280, "INLAND: south between two sheets of ice, to Mexico", 480),
        npc("history_dr_clovis_first", 2300, FLOOR, 69),
        prop_at("points", "history_clovis_points", 2440, FLOOR, 64, 32),
        sign("beringia_sign_points", 2440, FLOOR - 160, "CLOVIS POINTS: 13,000 years old. Not the first. Sorry.", 520),
        sign("beringia_sign_coast_a", 2440, g - 300, "THE COAST: south along the shore, under the ice, to South America.", 620),
        sign("beringia_sign_coast_b", 2440, g - 272, "At Monte Verde, Chile, people were living 14,500 years ago: before Clovis.", 660),
        enemy(rid, 2, "smilodon", 2500, g),
        sign("beringia_sign", 1000, 300, "BERINGIA: a land bridge where the sea is now, 20,000 years ago", 600),
    ]
    return grid_room(rid, "BERINGIA", e)


def first_maize():
    """The first maize, 7,000 BCE, in the Balsas valley of Mexico: a grass
    called teosinte, bred over thousands of years into corn. The way down
    is maize going south to the Andes."""
    rid = "history_first_maize"
    e = [
        start(rid, 160, FLOOR),
        prop_at("maize_1", "history_maize_plants", 380, FLOOR, 96, 128),
        npc("history_teosinte_breeder", 560, FLOOR, 61),
        sign("maize_sign_cob", 560, FLOOR - 200, "TEOSINTE TO MAIZE: an ear the size of your thumb, then this", 520),
        enemy(rid, 1, "green_iguana", 900, FLOOR),
        enemy(rid, 1, "howler_monkey", 1300, FLOOR),
        enemy(rid, 1, "jaguar", 1700, FLOOR),
        sign("maize_sign", 1400, 140, "THE FIRST MAIZE: farming, invented again, in the Americas", 560),
        prop_at("maize_2", "history_maize_plants", 2000, FLOOR, 96, 128),
        sign("maize_sign_down", 2248, FLOOR - 220, "DOWN: maize goes south, to the Andes", 340),
        enemy(rid, 2, "jaguar", 2500, FLOOR),
    ]
    return grid_room(rid, "THE FIRST MAIZE", e)


# ---- R½: South America -------------------------------------------------------


def andean_highlands():
    """The Andean highlands near Lake Titicaca, 8,000-2,000 BCE: where people
    first farmed the potato and quinoa, and later tamed the llama and the
    alpaca. Up the rock steps to the cold high plain, where potatoes are
    freeze-dried into chuno. The way up is maize, which came south. Antoine
    Parmen-tater is a guest from France, 1785, who takes the credit."""
    rid = "history_andean_highlands"
    e = [
        start(rid, 160, FLOOR),
        npc("history_potato_farmer", 300, FLOOR, 60),
        prop_at("potatoes_1", "history_potatoes", 420, FLOOR, 64, 32),
        sign("andes_sign_kinds", 380, FLOOR - 200, "THE POTATO: first farmed here. Peru grows about 4,000 kinds of it", 600),
        # Up to the high plain.
        solid("slope_1", 560, FLOOR - 80, 1140, 80),
        solid("slope_2", 680, FLOOR - 160, 900, 80),
        solid("slope_3", 800, FLOOR - 240, 660, 80),
        solid("puna", 920, FLOOR - 320, 420, 80),
        prop_at("chuno", "history_chuno", 1080, FLOOR - 320, 96, 32),
        sign("andes_sign_chuno", 1130, FLOOR - 360, "CHUNO: freeze them at night, dry them in the sun, stamp out the water. They keep for years.", 800),
        enemy(rid, 1, "andean_condor", 1500, FLOOR - 400),
        sign("andes_sign", 1400, 140, "THE ANDES: the potato, quinoa and the llama, 8,000-2,000 BCE", 560),
        enemy(rid, 1, "llama", 1800, FLOOR),
        enemy(rid, 2, "llama", 1980, FLOOR),
        sign("andes_sign_up", 2248, FLOOR - 220, "UP: the first maize", 220),
        npc("history_antoine_parmen_tater", 2440, FLOOR, 66),
        prop_at("potatoes_2", "history_potatoes", 2560, FLOOR, 64, 32),
        sign("andes_sign_world", 1860, FLOOR - 160, "THE POTATO went round the world and fed Europe's growing towns.", 580),
        sign("andes_sign_ireland_a", 2470, FLOOR - 440, "1845-1852, IRELAND: blight killed the potato crop.", 480),
        sign("andes_sign_ireland_b", 2470, FLOOR - 412, "About a million people died and a million more left,", 500),
        sign("andes_sign_ireland_c", 2470, FLOOR - 384, "while grain and cattle were still shipped out to Britain.", 540),
    ]
    return grid_room(rid, "THE ANDES", e)


def caral():
    """Caral in the Supe valley of Peru, about 2600 BCE: the oldest city of
    the Americas, as old as the pyramids of Egypt. Cotton from the valley
    went to the coast as fishing nets, and anchovies came back. Up the
    stepped platform pyramid; no enemies, as no signs of war have been found
    here."""
    rid = "history_caral"
    e = [
        start(rid, 160, FLOOR),
        npc("history_net_maker", 320, FLOOR, 60),
        prop_at("nets", "history_cotton_nets", 460, FLOOR, 96, 80),
        sign("caral_sign_nets", 420, FLOOR - 220, "ASPERO, ON THE COAST: cotton nets for anchovies, anchovies for cotton", 640),
        # The pyramid: up its steps to the top.
        solid("pyramid_1", 800, FLOOR - 80, 1040, 80),
        solid("pyramid_2", 900, FLOOR - 160, 840, 80),
        solid("pyramid_3", 1000, FLOOR - 240, 640, 80),
        solid("pyramid_4", 1100, FLOOR - 320, 440, 80),
        solid("pyramid_top", 1200, FLOOR - 400, 240, 80),
        prop_at("quipu", "history_quipu", 1250, FLOOR - 400, 48, 64),
        npc("history_caral_flautist", 1380, FLOOR - 400, 64),
        sign("caral_sign_quipu", 1320, FLOOR - 360, "A KNOTTED CORD found here may be the oldest quipu. Archaeologists are still untying that one.", 820),
        sign("caral_sign", 1400, 140, "CARAL, 2600 BCE: a city of the Americas as old as the pyramids of Egypt", 640),
        prop_at("shicra", "history_shicra_bags", 2000, FLOOR, 64, 48),
        sign("caral_sign_shicra", 2140, FLOOR - 150, "SHICRA: bags of stones in the walls, so the pyramids shake in an earthquake and stand", 760),
        sign("caral_sign_war", 2400, FLOOR - 300, "NO WALLS, NO WEAPONS: no signs of war have been found at Caral.", 580),
    ]
    return grid_room(rid, "CARAL", e)


def olmec_heartland():
    """The Olmec heartland, 1,200 BCE, on the Gulf coast of Mexico: colossal
    basalt heads, the first rubber and the first ballgame balls, and cacao. A
    pad of rubber throws you up to the terrace in the trees."""
    rid = "history_olmec_heartland"
    e = [
        start(rid, 160, FLOOR),
        prop_at("head_1", "history_colossal_head", 520, FLOOR, 128, 160),
        sign("olmec_sign_head", 520, FLOOR - 260, "A COLOSSAL HEAD: tonnes of basalt, moved without wheels or oxen", 580),
        npc("history_head_sculptor", 760, FLOOR, 63),
        prop_at("balls", "history_rubber_balls", 900, FLOOR, 64, 32),
        entity("ReboundPad", 1036, FLOOR - 16, 128, 16, name="the rubber", impulseX=0, impulseY=-900),
        sign("olmec_sign_rubber", 1100, FLOOR - 160, "RUBBER: latex and morning-glory juice, by 1600 BCE. Bounce.", 580),
        ledge("rubber_terrace", 1240, FLOOR - 240, 192),
        chest("a jade earspool", 1336, FLOOR - 240, "currency:100"),
        prop_at("cacao_1", "history_cacao_tree", 1560, FLOOR, 96, 128),
        enemy(rid, 1, "jaguar", 1800, FLOOR),
        enemy(rid, 1, "howler_monkey", 2000, FLOOR),
        sign("olmec_sign", 1400, 140, "THE OLMEC HEARTLAND: rubber, chocolate and heads the size of a car, 1,200 BCE", 700),
        prop_at("head_2", "history_colossal_head", 2240, FLOOR, 128, 160),
        enemy(rid, 2, "jaguar", 2450, FLOOR),
        prop_at("cacao_2", "history_cacao_tree", 2640, FLOOR, 96, 128),
    ]
    return grid_room(rid, "THE OLMEC HEARTLAND", e)


def maya_city():
    """A Maya city, 250-900 CE: temple pyramids over the forest, zero, the
    calendar and the ballcourt. The ballcourt gate opens when the ball goes
    through the stone ring (`history_ballcourt_ring`), up on the court's wall."""
    rid = "history_maya_city"
    gate_x = 1500
    e = [
        start(rid, 160, FLOOR),
        npc("history_zero_hero", 360, FLOOR, 62),
        prop_at("pyramid", "history_maya_pyramid", 620, FLOOR, 192, 192),
        ledge("pyramid_step_a", 524, FLOOR - 64, 192),
        ledge("pyramid_step_b", 556, FLOOR - 128, 128),
        ledge("pyramid_temple", 588, FLOOR - 192, 64),
        npc("history_lady_six_sky_high", 860, FLOOR, 64),
        prop_at("stela", "history_maya_stela", 1000, FLOOR, 48, 144),
        sign("maya_sign_stela", 1000, FLOOR - 220, "BAR AND DOT: a dot is one, a bar is five, a shell is ZERO", 520),
        # The ballcourt: up the court's sloped wall to the ring.
        ledge("court_a", 1080, FLOOR - 80, 160),
        ledge("court_b", 1180, FLOOR - 160, 160),
        ledge("court_ring", 1280, FLOOR - 240, 128),
        prop_at("ring", "history_ballcourt_ring", 1380, FLOOR - 240, 64, 96),
        lever(BALLCOURT_RING, "the stone ring", "Put the ball through the ring", 1310, FLOOR - 240),
        sign("maya_sign_ring", 1300, FLOOR - 360, "THE BALLCOURT: through the ring, hips only", 400),
        *gate("history_ballcourt_gate", "the ballcourt gate", gate_x, f"world.switch_on {BALLCOURT_RING}",
              "history_ballcourt_gate"),
        sign("maya_sign", 1400, 140, "THE MAYA CITY: zero, the calendar and the tallest buildings in the Americas, 250-900 CE", 760),
        enemy(rid, 1, "howler_monkey", 1800, FLOOR),
        enemy(rid, 1, "jaguar", 2050, FLOOR),
        prop_at("cacao", "history_cacao_tree", 2300, FLOOR, 96, 128),
        # Green iguanas live in the Maya lowlands; on Boriken they came only
        # in the 20th century, so the Taino islands have none.
        enemy(rid, 1, "green_iguana", 2500, FLOOR),
        prop_at("stela_2", "history_maya_stela", 2660, FLOOR, 48, 144),
    ]
    return grid_room(rid, "THE MAYA CITY", e)


def taino_islands():
    """The Taino islands, 1000-1492 CE: Boriken, Ayiti and Cuba. A hammock
    throws you up into the ceiba; a dugout canoe carries you over the sea
    between the islands. At the east end, a plain sign says what came in
    1492, and that the Taino are still here."""
    rid = "history_taino_islands"
    stuff, gap = crossing(rid, "the sea between the islands", (1500, 2200), "history_dugout_canoe", w=176)
    e = [
        *stuff,
        start(rid, 160, FLOOR),
        prop_at("bohio_1", "history_bohio", 360, FLOOR, 128, 128),
        npc("history_cacica_of_boriken", 560, FLOOR, 62),
        prop_at("buren", "history_buren", 700, FLOOR, 64, 32),
        prop_at("barbacoa", "history_barbacoa", 820, FLOOR, 80, 64),
        sign("taino_sign_food", 760, FLOOR - 160, "CASABE: cassava bread, baked on the buren. It keeps for months.", 580),
        prop_at("hammock", "history_hammock", 1000, FLOOR, 96, 64),
        entity("ReboundPad", 956, FLOOR - 28, 88, 16, name="the hammock", impulseX=0, impulseY=-900),
        sign("taino_sign_words", 1000, FLOOR - 300, "WORDS FROM TAINO: canoe, hammock, hurricane, barbecue, savanna, maize, tobacco", 700),
        prop_at("ceiba", "history_ceiba_tree", 1180, FLOOR, 160, 192),
        ledge("ceiba_branch", 1100, FLOOR - 220, 192),
        chest("a guanin (gold and copper)", 1196, FLOOR - 220, "currency:100"),
        npc("history_canoe_maker", 1380, FLOOR, 65),
        sign("taino_sign_sea", 1850, FLOOR - 260, "CANOES: one tree each, up to a hundred paddlers", 460),
        sign("taino_sign", 1400, 140, "THE TAINO ISLANDS: Boriken, Ayiti and Cuba, 1000-1492 CE", 580),
        # The Taino names beside the names a player knows, so that the
        # names in the rest of the room (and in Haiti) are easy to place.
        sign("taino_sign_names_a", 420, FLOOR - 260, "BORIKEN is Puerto Rico (Puerto Ricans still call themselves boricua).", 660),
        sign("taino_sign_names_b", 420, FLOOR - 232, "AYITI is the island of Haiti and the Dominican Republic. CUBA kept its name.", 700),
        enemy(rid, 1, "coral_crab", 2320, FLOOR),
        enemy(rid, 2, "coral_crab", 2450, FLOOR),
        prop_at("bohio_2", "history_bohio", 2560, FLOOR, 128, 128),
        enemy(rid, 3, "coral_crab", 2680, FLOOR),
        sign("taino_sign_1492_a", 2340, FLOOR - 340, "1492: strangers landed here. Within a generation most Taino had died", 640),
        sign("taino_sign_1492_b", 2340, FLOOR - 312,
             "of disease, war and forced labour. Not all: their descendants and words are still here.", 800),
    ]
    return grid_room(rid, "THE TAINO ISLANDS", e, gaps=[gap])


def haitian_revolution():
    """Saint-Domingue becomes Haiti, 1791-1804, on Ayiti, the island of the
    Taino room to the west. The plain of the north, its cane and its sugar
    works, where the enslaved rose in 1791; the mornes, where Leclerc's army
    was beaten in 1802-1803 (up and over them); the first flag (1803) and
    independence (1 January 1804). Toussaint Louverture and Catherine Flon
    are real people; the cook is original. The Door of No Return, in the
    fort on the Gold Coast, comes out on the plain (`haiti_landing`) and
    does not go back; the ring at the east end (ANTILLES) does."""
    rid = "history_haitian_revolution"
    e = [
        start(rid, 160, FLOOR),
        pad("haiti_landing", "off the ship", 420, FLOOR),
        prop_at("cane_1", "history_sugarcane", 340, FLOOR, 96, 128),
        prop_at("sugar_works", "history_sugar_works", 620, FLOOR, 256, 192),
        prop_at("cane_2", "history_sugarcane", 860, FLOOR, 96, 128),
        sign("haiti_sign_1789_a", 560, FLOOR - 340, "SAINT-DOMINGUE, 1789: France's richest colony. Its sugar and coffee", 620),
        sign("haiti_sign_1789_b", 560, FLOOR - 312, "were grown by about half a million enslaved people.", 480),
        sign("haiti_sign_1791", 620, FLOOR - 240, "AUGUST 1791: the enslaved of the northern plain rose up", 500),
        npc("history_toussaint_louverture", 980, FLOOR, 64),
        sign("haiti_sign_1794", 940, FLOOR - 160, "1793-1794: slavery ended here, then in every French colony", 540),
        # The mornes: up and over.
        solid("morne_1", 1100, FLOOR - 80, 800, 80),
        solid("morne_2", 1240, FLOOR - 160, 520, 80),
        solid("morne_3", 1380, FLOOR - 240, 240, 80),
        enemy(rid, 1, "napoleonic_grenadier", 1160, FLOOR - 80),
        enemy(rid, 2, "napoleonic_grenadier", 1500, FLOOR - 240),
        sign("haiti_sign_mornes", 1500, FLOOR - 340, "DEYE MON, GEN MON: beyond the mountains, more mountains", 520),
        sign("haiti_sign_1802_a", 1500, FLOOR - 400, "1802: Napoleon sent an army to take the colony back. In Guadeloupe, slavery came back.", 780),
        sign("haiti_sign_1802_b", 1500, FLOOR - 372, "Here the army lost, to the war in the mountains and to yellow fever.", 620),
        sign("haiti_sign", 1400, 140, "HAITI: the revolution of the enslaved, 1791-1804 CE", 460),
        enemy(rid, 3, "napoleonic_grenadier", 1940, FLOOR),
        npc("history_catherine_flon", 2030, FLOOR, 62),
        prop_at("flag", "history_first_flag", 2120, FLOOR, 64, 160),
        sign("haiti_sign_flag", 2080, FLOOR - 250, "18 MAY 1803: the white torn out of the French flag, the blue and the red sewn together", 780),
        npc("history_soup_joumou_cook", 2230, FLOOR, 68),
        prop_at("soup_pot", "history_soup_pot", 2330, FLOOR, 80, 64),
        sign("haiti_sign_1804_a", 2400, FLOOR - 460, "1 JANUARY 1804: HAITI. The first nation founded by people who freed themselves,", 720),
        sign("haiti_sign_1804_b", 2400, FLOOR - 432, "named with the Taino name of its island, Ayiti.", 440),
        sign("haiti_sign_1825_a", 2240, FLOOR - 370, "1825: France made Haiti pay 150 million francs for its lost 'property':", 640),
        sign("haiti_sign_1825_b", 2240, FLOOR - 342, "the people who had freed themselves. With the loans, Haiti paid until 1947.", 680),
        # The ring (ANTILLES, `data/gate_network.ron`): the way back from the
        # far side of the Atlantic.
        *gates.as_ron(gates.terminal("Ayiti Gate", "history_ayiti_terminal", 2396, FLOOR)),
        *gates.as_ron(gates.ring("Ayiti Gate", "history_ayiti_gate", 2470, FLOOR, "the gate")),
    ]
    return grid_room(rid, "HAITI", e)


def door_of_no_return():
    """A castle of the European trading companies on the Gold Coast, 1700s
    (a composite, not one real fort). The beach and the fishing town; the
    courtyard, its cannon facing the sea; the low dungeon; and the narrow
    door to the sea, which goes one way, to Saint-Domingue. A sober room:
    no enemies and no joke. Ottobah Cugoano, who was taken from this coast,
    is real; the guide is original."""
    rid = "history_door_of_no_return"
    dungeon = (1500, 2300)
    e = [
        start(rid, 160, FLOOR),
        prop_at("canoe_1", "history_fante_canoe", 320, FLOOR, 192, 64),
        prop_at("canoe_2", "history_fante_canoe", 560, FLOOR, 192, 64),
        sign("fort_sign_coast", 400, FLOOR - 200, "THE GOLD COAST: forts built to trade for gold. Then for people.", 580),
        # The castle gate: under its wall into the courtyard.
        solid("castle_gate", 760, CEILING, 80, FLOOR - 160 - CEILING),
        ledge("battlement_stair", 900, FLOOR - 100, 120),
        ledge("battlement", 1000, FLOOR - 200, 400),
        prop_at("cannon_1", "history_fort_cannon", 1120, FLOOR - 200, 80, 48),
        prop_at("cannon_2", "history_fort_cannon", 1300, FLOOR - 200, 80, 48),
        npc("history_ottobah_cugoano", 1360, FLOOR, 66),
        sign("fort_sign", 1400, 140, "A FORT ON THE GOLD COAST: the Atlantic slave trade, 1501-1866 CE", 580),
        # The dungeon: a low, dark passage.
        solid("dungeon_roof", dungeon[0], CEILING, dungeon[1] - dungeon[0], FLOOR - 120 - CEILING),
        sign("fort_sign_dungeon_a", 1700, FLOOR - 104, "THE DUNGEON: hundreds of people at a time,", 380),
        sign("fort_sign_dungeon_b", 2100, FLOOR - 104, "for weeks or months, waiting for a ship.", 360),
        npc("history_castle_guide", 2400, FLOOR, 64),
        sign("fort_sign_numbers_a", 2440, FLOOR - 360, "1501-1866: about 12.5 million people were forced onto ships to the Americas.", 680),
        sign("fort_sign_numbers_b", 2440, FLOOR - 332, "About 10.7 million lived to land. About one voyage in ten saw a revolt.", 640),
        # The door: one way only. Nothing brings you back through it; the
        # ring in Haiti (ANTILLES) dials home.
        door("fort_door_of_no_return", "the Door of No Return", 2650, FLOOR, "history_haitian_revolution", "haiti_landing",
             bidirectional=False),
        sign("fort_sign_door", 2674, FLOOR - 150, "THE DOOR OF NO RETURN", 220),
    ]
    return grid_room(rid, "THE DOOR OF NO RETURN", e)


#: The rooms, by floor from the bottom (Africa) to the top.
ROOMS = (sulawesi, austronesian_islands, dong_son, cradle, southern_coast, green_sahara, nile, tomb, aksum, timbuktu, library, benin_city, door_of_no_return,
         levant, crescent, granary, uruk, classical, colosseum,
         ice_age_europe, painted_cave, first_farmers_of_europe, megalith_builders, celtic_frontier, viking_north, sail,
         industrial, mill,
         southern_route, mehrgarh, indus_cities,
         sahul, new_guinea_highlands, lapita_voyagers, wayfinders, rapa_nui,
         mammoth_steppe, yellow_river, shang_court, silk, steppe, great_ger,
         beringia, first_maize, olmec_heartland, maya_city, taino_islands, haitian_revolution,
         andean_highlands, caral)


def main(argv):
    made = [make() for make in ROOMS]
    assert sorted(rid for rid, _ in made) == sorted(list(CELLS) + list(BUILDINGS)), "every room is made once"
    paths = []
    for rid, text in made:
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    if code := ensure_world(WORLD, UID_OFFSET):
        return code
    # A world made from the sandbox's defs has no `dangerous` level field.
    code, _ = run("level", "add-field-def", "dangerous", "--type", "Bool", "--default", "false", "--doc",
                  "Out of town: an NPC on the player's side joins the player's fights here (RoomMetadata::dangerous).",
                  str(WORLD), "--in-place")
    if code:
        return code
    if "--relayout" in argv:
        # A room may not land on one that has not moved yet. The rooms that
        # go up move first, the top one first; then the rooms that go down,
        # the bottom one first; then the new rooms, into the space the others
        # left.
        was = {level["identifier"]: level["worldY"] for level in json.loads(WORLD.read_text())["levels"]}

        def order(path):
            y = ROOMS_AT[path.stem][0][1]
            if path.stem not in was:
                return (2, y)
            return (0, y) if y <= was[path.stem] else (1, -y)
        paths.sort(key=order)
    if code := make_rooms(WORLD, paths, keep_position="--relayout" not in argv,
                           overwrite_hand_edits="--overwrite-hand-edits" in argv):
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
