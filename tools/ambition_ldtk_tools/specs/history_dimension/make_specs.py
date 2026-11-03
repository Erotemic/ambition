#!/usr/bin/env python3
"""Write the rooms of the History dimension (`docs/planning/game/history-dimension.md`).

    python specs/history_dimension/make_specs.py          # writes the specs
    python specs/history_dimension/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

A timeline: walk east and you walk forward in time. Its gate is DYNASTIC
(`data/gate_network.ron`), at the far west, at the dawn of farming. The eight
eras stand side by side on one road (`specs/check_flat_map.py history.ldtk`):

    [crescent][nile][classical][silk road][steppe][timbuktu][sail][industrial]
       door     door   door                 door     door           door

- `history_fertile_crescent`: the ring and its terminal; Clio the Muse,
  Hammurabbi, Gilgamash. A canal crosses the fields: the sluice lever raises
  the reed bridge over it. The Temple Granary by a door.
- `history_nile_valley`: the Nile crosses the room; a felucca carries you
  over it. Hat-shepsut, Ramesses the Pretty Good. The Pyramid Tomb by a door:
  a vigil of mummies opens its treasure room.
- `history_classical_shore`: Socratease asks a question at the stoa gate; the
  answer opens it. Herodotoss, Julius Sneezer. The aqueduct is a high road.
  The Colosseum by a door: the games open the emperor's box.
- `history_silk_road`: a camel caravan carries you over the quicksand. Marco
  Polo-neck, Confusius.
- `history_mongol_steppe`: the Great Wall, and two roads through it. Genghis
  Can opens the gate (the short road, through the garrison); Genghis Can't
  finds a scaffold over the Wall (the long road, over the garrison). The
  Great Ger by a door, with Kublai Khan't-Wait.
- `history_timbuktu_coast`: Mansa Moolah's gold; the Library of Timbuktu by a
  door; a dhow over the sea to Zheng Hey's pier.
- `history_age_of_sail`: the harbour; ships' booms rock over the water.
  Vasco da Gamer, Christopher Columbust, Captain Cookbook.
- `history_industrial_age`: Adam Smithy, Karl Marks, General Ludd. The boiler
  lever in the Cotton Mill opens the factory gate on the railway; a rail
  wagon carries you to the end of the first pass.

The art is the sprite tools' `targets/props/history_*_props.py` and
`targets/characters/` (the cast). A prop sheet that is 2 frame px to a world
unit is placed at half its frame.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "gate_network"))

from district_kit import WALL, WORLDS, edge, ensure_world, entity, door, ledge, make_rooms, prop, resident, room, run, sign, solid, start  # noqa: E402

import gates  # noqa: E402

WORLD = WORLDS / "history.ldtk"
GENERATOR = "specs/history_dimension/make_specs.py"
UID_OFFSET = 1200000

CRESCENT = "history_fertile_crescent"
NILE = "history_nile_valley"
CLASSICAL = "history_classical_shore"
SILK = "history_silk_road"
STEPPE = "history_mongol_steppe"
TIMBUKTU = "history_timbuktu_coast"
SAIL = "history_age_of_sail"
INDUSTRIAL = "history_industrial_age"
#: The road from west to east: earliest first.
ROAD = (CRESCENT, NILE, CLASSICAL, SILK, STEPPE, TIMBUKTU, SAIL, INDUSTRIAL)

GRANARY = "history_temple_granary"
TOMB = "history_pyramid_tomb"
COLOSSEUM = "history_colosseum"
GER = "history_great_ger"
LIBRARY = "history_timbuktu_library"
MILL = "history_cotton_mill"

BG = "#3A2A1C"
#: An outdoor room's size; the floor has 64 of ground under it.
W, H = 2800, 1024
FLOOR = 960
CEILING = 32
ROW_Y = 1200

#: Each room: where it is in the world, and its size. The road is one row;
#: the buildings are under it, out of the way of the flat map.
ROOMS_AT = {rid: ((n * W, ROW_Y), (W, H)) for n, rid in enumerate(ROAD)}
ROOMS_AT.update({rid: ((n * 1792, 2400), (1600, 640)) for n, rid in enumerate((GRANARY, TOMB, COLOSSEUM, GER, LIBRARY, MILL))})

MUSIC = {
    CRESCENT: "the_sun_is_a_group_project",
    NILE: "salt_steel_and_starlight",
    CLASSICAL: "standing_on_shoulders",
    SILK: "spaghetti_western",
    STEPPE: "hawks_of_the_high_aerie",
    TIMBUKTU: "pulse_drift_voyage",
    SAIL: "pirates_guild_black_flag_jig",
    INDUSTRIAL: "galvanic_march",
    GRANARY: "carry_the_one",
    TOMB: "vault_in_the_dark",
    COLOSSEUM: "fighters_guild_oath_of_steel",
    GER: "hawks_of_the_high_aerie",
    LIBRARY: "the_last_lantern",
    MILL: "luddites_guild_loom_and_liberty",
}
BIOME = {
    CRESCENT: "history_crescent", GRANARY: "history_crescent",
    NILE: "history_nile", TOMB: "history_nile",
    CLASSICAL: "history_classical", COLOSSEUM: "history_classical",
    SILK: "history_silk",
    STEPPE: "history_steppe", GER: "history_steppe",
    TIMBUKTU: "history_monsoon", LIBRARY: "history_monsoon",
    SAIL: "history_sail",
    INDUSTRIAL: "history_industrial", MILL: "history_industrial",
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
}

DOOR_W, DOOR_H = 48, 192
INSIDE_FLOOR = 576
#: The crescent's canal (x0, x1), its reed bridge and the sluice's lever.
CANAL = (1296, 1744)
BRIDGE_W = 224
SLUICE = "history_sluice"
#: The Nile (x0, x1); the silk road's quicksand; the harbour's water; the
#: railway cutting. A boat, a caravan or a wagon carries you over each.
NILE_RIVER = (1000, 1800)
QUICKSAND = (1100, 1900)
SEA = (1500, 2300)
HARBOUR = (960, 1440)
CUTTING = (1700, 2300)
#: The stoa gate's answer, the Wall's two roads, the mill's boiler.
KNOWS_NOTHING = "history_knows_nothing"
CAN_ROAD, CANT_ROAD = "history_can_says_yes", "history_cant_says_maybe"
BOILER = "history_boiler"
#: The Great Wall in the steppe (x0, x1), its top, and the gate in it.
WALL_X = (928, 1024)
WALL_TOP = 640
WALL_GATE = (952, 1000)
#: The two vigils.
TOMB_VIGIL = "history_tomb_vigil"
GAMES = "history_the_games"


def enemy(rid, n, character, x, feet):
    """A creature whose feet (a flyer: the foot of its box) are at `feet`."""
    w, h = BOX[character]
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{rid}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def lock_wall(wall_id, name, x, y, w, h, gated_by, look):
    """A wall drawn as `look` that stands until `gated_by` is true."""
    return entity("LockWall", x, y, w, h, id=wall_id, name=name, gated_by=gated_by, look=look)


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
    """A gap in the floor from `span[0]` to `span[1]`, a hazard at its foot,
    and a platform drawn as `look` that sweeps to and fro over it at `top`."""
    x0, x1 = span
    return [
        solid("ground_west", 0, FLOOR, x0, H - FLOOR),
        solid("ground_east", x1, FLOOR, W - x1, H - FLOOR),
        entity("HazardBlock", x0, H - 16, x1 - x0, 16, name=name),
        entity("MovingPlatform", x0 + 8, top, w, 16, id=f"{rid}_{look}", name=look.replace("history_", "the ").replace("_", " "),
               speed=speed, sweep_dx=float(x1 - x0 - w - 16), one_way=True, visual=look),
    ]


def road_room(rid, title, entities, *, ground=True, first=False, last=False):
    """An outdoor room of the road: its ceiling, its walls, the side exits to
    the eras before and after it, and `entities`."""
    n = ROAD.index(rid)
    e = [solid("ceiling", 0, 0, W, CEILING)]
    if ground:
        e.append(solid("ground", 0, FLOOR, W, H - FLOOR))
    if first:
        e.append(solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING))
    else:
        e += [solid("west_wall", 0, CEILING, WALL, FLOOR - 112 - CEILING),
              edge(f"{rid}_west", "back in time", 0, FLOOR, ROAD[n - 1], f"{ROAD[n - 1]}_east")]
    if last:
        e.append(solid("east_wall", W - WALL, CEILING, WALL, FLOOR - CEILING))
    else:
        e += [solid("east_wall", W - WALL, CEILING, WALL, FLOOR - 112 - CEILING),
              edge(f"{rid}_east", "forward in time", W - WALL, FLOOR, ROAD[n + 1], f"{ROAD[n + 1]}_west")]
    at, size = ROOMS_AT[rid]
    return rid, room(rid, e + entities, at=at, size=size, biome=BIOME[rid], music=MUSIC[rid], bg=BG, title=title,
                     generator=GENERATOR, dangerous=True)


def building(rid, back_to, back_zone, title, extra):
    """A building by a door: its floor, ceiling and walls, the door back out
    at its west end, and `extra`."""
    w, h = ROOMS_AT[rid][1]
    g = INSIDE_FLOOR
    e = [
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, CEILING),
        solid("west_wall", 0, CEILING, WALL, g - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, g - CEILING),
        door(f"{rid}_door", "back out", 40, g, back_to, back_zone),
        start(rid, 140, g),
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


# ---- 1. The Fertile Crescent ------------------------------------------------


def crescent():
    """The dawn of farming: the ring, Clio, the fields and the canal."""
    rid = CRESCENT
    c0, c1 = CANAL
    e = [
        solid("ground_west", 0, FLOOR, c0, H - FLOOR),
        solid("ground_east", c1, FLOOR, W - c1, H - FLOOR),
        entity("HazardBlock", c0, H - 16, c1 - c0, 16, name="the deep canal"),
        # The reed bridge stands while the sluice's lever is on.
        *[lock_wall(f"history_reed_bridge_{n}", "the reed bridge", x, FLOOR, BRIDGE_W, 24, f"not world.switch_on {SLUICE}",
                    "history_reed_bridge") for n, x in enumerate(range(c0, c1, BRIDGE_W))],
        start(rid, 520, FLOOR),
        # The ring (DYNASTIC) and its terminal: the way in and the way home.
        *gates.as_ron(gates.ring("Crescent Gate", "history_crescent_gate", 112, FLOOR, "the gate")),
        *gates.as_ron(gates.terminal("Crescent Gate", "history_gate_terminal", 416, FLOOR)),
        sign("crescent_sign_gate", 256, FLOOR - 340, "DYNASTIC: the gate at the start of everything", 380),
        npc("history_clio", 620, FLOOR, 66),
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
    return road_room(rid, "THE FERTILE CRESCENT", e, ground=False, first=True)


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
        sign("granary_sign", 800, 90, "THE TEMPLE GRANARY: writing was made here, to count barley", 520),
        sign("granary_sign_debt", 1400, g - 150, "YOU OWE: 3 sheep (with interest)", 340),
    ]
    return building(GRANARY, CRESCENT, "crescent_granary_door", "THE TEMPLE GRANARY", extra)


# ---- 2. The Nile -------------------------------------------------------------


def nile():
    rid = NILE
    r0, r1 = NILE_RIVER
    e = [
        *crossing(rid, "the crocodiles", NILE_RIVER, "history_felucca"),
        start(rid, 160, FLOOR),
        npc("history_hatshepsut_hats", 320, FLOOR, 64),
        prop_at("obelisk", "history_obelisk", 460, FLOOR, 48, 224),
        enemy(rid, 1, "scarab_beetle", 600, FLOOR),
        prop_at("papyrus_1", "history_papyrus", 720, FLOOR, 64, 96),
        enemy(rid, 1, "asp", 840, FLOOR),
        prop_at("papyrus_2", "history_papyrus", 940, FLOOR, 64, 96),
        sign("nile_sign_river", (r0 + r1) // 2, FLOOR - 240, "THE NILE: floods every year, on schedule, unlike the rest of history", 560),
        sign("nile_sign", 1400, 140, "THE NILE: 3,000 years of the same hats", 420),
        prop_at("papyrus_3", "history_papyrus", 1860, FLOOR, 64, 96),
        enemy(rid, 2, "scarab_beetle", 1960, FLOOR),
        prop_at("statue", "history_pharaoh_statue", 2080, FLOOR, 128, 224),
        npc("history_ramesses_pretty_good", 2220, FLOOR, 68),
        prop_at("jars", "history_canopic_jars", 2330, FLOOR, 80, 48),
        # The pyramid's steps up to the tomb's door.
        ledge("pyramid_step_a", 2300, FLOOR - 64, 240),
        ledge("pyramid_step_b", 2360, FLOOR - 128, 120),
        enemy(rid, 1, "mummy", 2500, FLOOR),
        enemy(rid, 2, "asp", 2400, FLOOR - 128),
        door("nile_tomb_door", "into the pyramid", 2640, FLOOR, TOMB, f"{TOMB}_door"),
        sign("nile_sign_tomb", 2560, FLOOR - 260, "PYRAMID: do not disturb (this means you)", 380),
    ]
    return road_room(rid, "THE NILE", e, ground=False)


def tomb():
    g = INSIDE_FLOOR
    return vigil(TOMB, NILE, "nile_tomb_door", "THE PYRAMID TOMB", TOMB_VIGIL, bell="the priest's bell",
                 store_name="the grille of the treasure room", chest_name="the pharaoh's luggage for the afterlife",
                 signs=[
                     prop_at("jars", "history_canopic_jars", 640, g, 80, 48),
                     ledge("sarcophagus_ledge", 820, g - 96, 200),
                     prop_at("statue", "history_pharaoh_statue", 1120, g, 128, 224),
                     sign("tomb_sign", 800, 90, "THE PYRAMID TOMB: everything you need for the afterlife, packed", 560),
                     sign("tomb_sign_curse", 1440, g - 200, "THE CURSE: mostly a rumour", 300),
                 ])


# ---- 3. The Classical Shore -----------------------------------------------


def classical():
    rid = CLASSICAL
    gate = 1100
    e = [
        start(rid, 160, FLOOR),
        prop_at("column_1", "history_marble_column", 260, FLOOR, 48, 192),
        npc("history_herodotoss", 380, FLOOR, 66),
        prop_at("amphorae", "history_amphorae", 500, FLOOR, 64, 64),
        prop_at("column_2", "history_marble_column", 620, FLOOR, 48, 192),
        prop_at("bust_1", "history_bust", 760, FLOOR, 48, 96),
        npc("history_socratease", 960, FLOOR, 64),
        # The stoa gate: the answer to his question opens it.
        solid("stoa_lintel", gate - 16, CEILING, DOOR_W + 32, FLOOR - DOOR_H - CEILING),
        lock_wall("history_stoa_gate", "the stoa gate", gate, FLOOR - DOOR_H, DOOR_W, DOOR_H,
                  f"world.flag_set {KNOWS_NOTHING}", "history_stoa_gate"),
        sign("classical_sign_gate", gate + DOOR_W // 2, FLOOR - DOOR_H - 120, "THE STOA: what do you know?", 320),
        enemy(rid, 1, "hoplite", 1300, FLOOR),
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
        prop_at("column_3", "history_marble_column", 2540, FLOOR, 48, 192),
        door("classical_colosseum_door", "into the colosseum", 2620, FLOOR, COLOSSEUM, f"{COLOSSEUM}_door"),
        sign("classical_sign_fall", 2320, FLOOR - 260, "ROME: did not fall in a day either", 340),
    ]
    return road_room(rid, "THE CLASSICAL SHORE", e)


def colosseum():
    g = INSIDE_FLOOR
    return vigil(COLOSSEUM, CLASSICAL, "classical_colosseum_door", "THE COLOSSEUM", GAMES, bell="the emperor's gong",
                 store_name="the gate of the emperor's box", chest_name="a thumb (up)",
                 signs=[
                     prop_at("column_1", "history_marble_column", 560, g, 48, 192),
                     ledge("tier_a", 700, g - 96, 200),
                     ledge("tier_b", 960, g - 160, 200),
                     prop_at("column_2", "history_marble_column", 1200, g, 48, 192),
                     sign("colosseum_sign", 800, 90, "THE COLOSSEUM: bread and circuses, mostly circuses", 520),
                     sign("colosseum_sign_box", 1440, g - 200, "THE EMPEROR'S BOX", 260),
                 ])


# ---- 4. The Silk Road --------------------------------------------------------


def silk():
    rid = SILK
    q0, q1 = QUICKSAND
    e = [
        *crossing(rid, "the quicksand", QUICKSAND, "history_camel_caravan", speed=70.0),
        start(rid, 160, FLOOR),
        prop_at("lantern_1", "history_paper_lantern", 260, FLOOR, 32, 128),
        npc("history_marco_polo_neck", 380, FLOOR, 66),
        prop_at("silk", "history_silk_bolts", 500, FLOOR, 80, 64),
        enemy(rid, 1, "desert_bandit", 640, FLOOR),
        npc("history_confusius", 760, FLOOR, 64),
        prop_at("spice", "history_spice_sacks", 880, FLOOR, 80, 48),
        enemy(rid, 1, "silk_moth", 980, FLOOR - 260),
        sign("silk_sign_sand", (q0 + q1) // 2, FLOOR - 260, "QUICKSAND: ride the camels (they know the way)", 460),
        sign("silk_sign", 1400, 140, "THE SILK ROAD: 4,000 miles of middlemen", 420),
        enemy(rid, 2, "silk_moth", 1500, FLOOR - 380),
        enemy(rid, 1, "terracotta_warrior", 2040, FLOOR),
        prop_at("pagoda", "history_pagoda", 2240, FLOOR, 128, 256),
        ledge("pagoda_eave_a", 2140, FLOOR - 80, 200),
        ledge("pagoda_eave_b", 2180, FLOOR - 160, 120),
        enemy(rid, 2, "desert_bandit", 2400, FLOOR),
        enemy(rid, 2, "terracotta_warrior", 2540, FLOOR),
        prop_at("lantern_2", "history_paper_lantern", 2660, FLOOR, 32, 128),
    ]
    return road_room(rid, "THE SILK ROAD", e, ground=False)


# ---- 5. The Mongol Steppe ----------------------------------------------------


def steppe():
    """The Great Wall at the west end, and two roads through it: Genghis
    Can's gate (short, through the garrison) and Genghis Can't's scaffold
    (long, over it)."""
    rid = STEPPE
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
                    "history_scaffold_plank") for n, (x, y) in enumerate(((744, 880), (568, 800), (744, 720)))],
        sign("steppe_sign_wall", (w0 + w1) // 2, WALL_TOP - 120, "THE GREAT WALL: through it, or over it?", 400),
        # Over it: the rampart walk east, and the steps down.
        ledge("rampart", w1, WALL_TOP, 480),
        ledge("rampart_step_a", 1520, 720, 160),
        ledge("rampart_step_b", 1720, 800, 160),
        ledge("rampart_step_c", 1920, 880, 160),
        # Through it: the garrison waits at the foot.
        enemy(rid, 1, "horse_archer", 1160, FLOOR),
        enemy(rid, 1, "steppe_wolf", 1300, FLOOR),
        enemy(rid, 2, "steppe_wolf", 1380, FLOOR),
        enemy(rid, 2, "horse_archer", 1600, FLOOR),
        enemy(rid, 1, "golden_eagle", 1800, FLOOR - 400),
        sign("steppe_sign", 1400, 140, "THE MONGOL STEPPE: the largest land empire, on horseback", 500),
        prop_at("ovoo", "history_ovoo", 2160, FLOOR, 64, 96),
        enemy(rid, 2, "golden_eagle", 2300, FLOOR - 300),
        prop_at("ger_2", "history_ger", 2420, FLOOR, 160, 128),
        door("steppe_ger_door", "into the great ger", 2620, FLOOR, GER, f"{GER}_door"),
        sign("steppe_sign_ger", 2560, FLOOR - 200, "THE GREAT GER: the capital (it moves)", 380),
    ]
    return road_room(rid, "THE MONGOL STEPPE", e)


def great_ger():
    g = INSIDE_FLOOR
    extra = [
        prop_at("standard", "history_horse_standard", 360, g, 32, 160),
        npc("history_kublai_khant_wait", 520, g, 66),
        sign("ger_sign_yam", 800, 150, "THE YAM: a fresh horse every 25 miles, a letter across Asia in a month", 600),
        ledge("ger_bench", 900, g - 96, 200),
        prop_at("ovoo", "history_ovoo", 1180, g, 64, 96),
        chest("a paiza (a passport of gold)", 1420, g, "currency:150"),
        sign("ger_sign", 800, 90, "THE GREAT GER: the empire's office, portable", 460),
    ]
    return building(GER, STEPPE, "steppe_ger_door", "THE GREAT GER", extra)


# ---- 6. Timbuktu and the Monsoon Coast --------------------------------------


def timbuktu():
    rid = TIMBUKTU
    s0, s1 = SEA
    e = [
        *crossing(rid, "the sea", SEA, "history_dhow", speed=110.0),
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
        sign("timbuktu_sign_sea", (s0 + s1) // 2, FLOOR - 240, "THE MONSOON: east in summer, west in winter. Plan ahead.", 520),
        enemy(rid, 1, "monsoon_squall", 1900, FLOOR - 320),
        enemy(rid, 2, "monsoon_squall", 2200, FLOOR - 420),
        enemy(rid, 2, "coral_crab", 2420, FLOOR),
        npc("history_zheng_hey", 2560, FLOOR, 68),
        prop_at("manuscripts", "history_manuscripts", 2660, FLOOR, 64, 48),
    ]
    return road_room(rid, "TIMBUKTU AND THE MONSOON COAST", e, ground=False)


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
        sign("library_sign", 800, 90, "THE LIBRARY OF TIMBUKTU: astronomy, law, medicine, poems", 520),
        sign("library_sign_gold", 1160, g - 120, "GOLD (worth less than the books)", 340),
    ]
    return building(LIBRARY, TIMBUKTU, "timbuktu_library_door", "THE LIBRARY OF TIMBUKTU", extra)


# ---- 7. The Age of Sail -------------------------------------------------------


def sail():
    rid = SAIL
    h0, h1 = HARBOUR
    e = [
        solid("ground_west", 0, FLOOR, h0, H - FLOOR),
        solid("ground_east", h1, FLOOR, W - h1, H - FLOOR),
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
    return road_room(rid, "THE AGE OF SAIL", e, ground=False)


# ---- 8. The Industrial Age ---------------------------------------------------


def industrial():
    rid = INDUSTRIAL
    k0, k1 = CUTTING
    gate = 1300
    e = [
        *crossing(rid, "the railway cutting", CUTTING, "history_rail_wagon", speed=120.0),
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
        solid("gate_lintel", gate - 16, CEILING, DOOR_W + 32, FLOOR - DOOR_H - CEILING),
        lock_wall("history_factory_gate", "the factory gate", gate, FLOOR - DOOR_H, DOOR_W, DOOR_H,
                  f"world.switch_on {BOILER}", "history_factory_gate"),
        sign("industrial_sign_gate", gate + DOOR_W // 2, FLOOR - DOOR_H - 120, "GATE OPENS WHEN THE ENGINE RUNS", 380),
        enemy(rid, 1, "steam_automaton", 1480, FLOOR),
        enemy(rid, 1, "soot_sprite", 1560, FLOOR - 240),
        enemy(rid, 1, "robber_baron", 1620, FLOOR),
        sign("industrial_sign_rail", (k0 + k1) // 2, FLOOR - 240, "THE RAILWAY: time itself was set by it", 400),
        sign("industrial_sign", 1400, 140, "THE INDUSTRIAL AGE: more of everything, especially smoke", 520),
        enemy(rid, 2, "soot_sprite", 2000, FLOOR - 320),
        enemy(rid, 2, "robber_baron", 2400, FLOOR),
        prop_at("engine", "history_steam_engine", 2480, FLOOR, 160, 160),
        chest("the twentieth century (some assembly required)", 2660, FLOOR, "currency:200"),
        sign("industrial_sign_end", 2480, FLOOR - 260, "TO BE CONTINUED: the twentieth century is under construction", 560),
    ]
    return road_room(rid, "THE INDUSTRIAL AGE", e, ground=False, last=True)


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
        sign("mill_sign", 800, 90, "THE COTTON MILL: 14 hours a day, 6 days a week", 480),
    ]
    return building(MILL, INDUSTRIAL, "industrial_mill_door", "THE COTTON MILL", extra)


ROOMS = (crescent, granary, nile, tomb, classical, colosseum, silk, steppe, great_ger, timbuktu, library, sail, industrial,
         mill)


def main(argv):
    paths = []
    for make in ROOMS:
        rid, text = make()
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
    if code := make_rooms(WORLD, paths, keep_position="--relayout" not in argv):
        return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
