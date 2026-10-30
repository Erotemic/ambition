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
- At its far west end, the Crooked Lantern: an inn (a door), Gudrun and
  Tobias inside.
- Beside the founder's plinth, a gate down to his crypt: a stair down a
  hole under the gate's porch (no door: the crypt is right under the moor).
- Up the moor's watchtower and out of the top of the room, the Searchlight
  Ridge over the moor and the allotments: its searchlights, a sentry hut,
  and a gap at its west end to drop onto the allotments' greenhouse roof.
  The allotments, the moor, the tower and the ridge are a loop.
- Under the moor and the allotments, the catacombs: from the crypt's west
  end to a shaft up under Crabbe's plot (13) in the allotments. The moor,
  the crypt, the catacombs and the allotments are a loop (Jon, 2026-10-30:
  "large interconnected regions ... not a series of disconnected doors").
- West of the moor by a side exit (floor y 656), the Castle Allotments:
  where the castle grows its vegetables (pumpkins that scowl, cabbages that
  bite, mandrakes not to be pulled), the greenhouse's roof to climb to a
  chest, Hester Crabbe the head gardener behind her fence, and what has got
  into the beds.
- West of the allotments by a side exit, Lower Godwin: the hamlet under the
  castle, its cottages, bakery, well, stocks and decree board; Barnaby
  Bellows the town crier on his platform, Prudence Crumb the baker; her
  bakery inside (a door), and behind its flour, the Gazette's press.

The art is the sprite tools' `targets/props/evil_moor_props.py`, drawn on
these numbers.

Where a room is in the world is its LDtk editor's once it exists: `--apply`
keeps each room where it stands; `--apply --relayout` puts each back where
this script says (`specs/check_flat_map.py evil.ldtk` checks the joins).
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, door, edge, entity, ledge, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "evil.ldtk"
GENERATOR = "specs/evil_moor/make_specs.py"
MOOR = "evil_searchlight_moor"
#: The inn at the moor's far west end, and where its room is in the world.
INN = "evil_crooked_lantern"
#: A building: anywhere in the world, out of the way of the catacombs.
INN_AT, INN_W, INN_H, INN_FLOOR = (-2400, 2304), 1024, 512, 448
#: The founder's crypt, down a gate beside his plinth, and where it is.
CRYPT = "evil_founders_crypt"
#: Right under the moor's east half: its shaft under the gate's porch.
CRYPT_AT, CRYPT_W, CRYPT_H, CRYPT_FLOOR = (-1600, 784), 1600, 576, 512
#: The stair down at the crypt gate, in the moor (x0, x1), and its shaft in
#: the crypt.
CRYPT_STAIR = (2282, 2346)
CRYPT_SHAFT = (CRYPT_STAIR[0] - 800, CRYPT_STAIR[1] - 800)
#: Under the allotments and the moor's west half, from the crypt west to the
#: shaft up under Crabbe's plot (13): the allotments' x 280..344.
CATACOMBS = "evil_catacombs"
CATACOMBS_AT, CATACOMBS_W = (-4160, 784), 2560
PLOT_13 = (280, 344)
#: Over the moor and the allotments, up the watchtower: the Searchlight
#: Ridge. Its ground (G 480 to its foot, 560); the hole the watchtower comes
#: up through (over the moor's x 1500..1620); the gap at its west end over
#: the allotments' greenhouse (their x 470..850), a drop down onto its roof.
RIDGE = "evil_searchlight_ridge"
#: 64 of ground under its walk: a body that comes up from the moor arrives
#: (92 over the room's foot) standing over the floor, on the hatch.
RIDGE_AT, RIDGE_W, RIDGE_H, RIDGE_G = (-4000, -544), 3600, 544, 480
RIDGE_TOWER = (3100, 3220)
RIDGE_GAP = (70, 450)
MOOR_TOWER_TOP = (1500, 1620)
#: The sarcophagus (x0, x1, its lid, walked on).
SARCOPHAGUS = (1100, 1300, 440)
FOREST = "evil_gate_forest"
#: The allotments, west of the moor: the same floor, the rooms touching.
ALLOT = "evil_castle_allotments"
ALLOT_W = 2000
#: The raised beds (x0, x1, top), the water butt and the greenhouse's roof,
#: all walked on (`evil_allotment_props.py` draws on these).
ALLOT_BEDS = ((960, 1160, 624), (1240, 1440, 624), (1520, 1720, 624))
ALLOT_BUTT = (860, 900, 592)
ALLOT_ROOF = (470, 850, 512)
#: The hamlet, west of the allotments: the same floor, the rooms touching.
HAMLET = "evil_lower_godwin"
HAMLET_W = 2400
#: The crier's platform (x0, x1, y), walked on (`evil_hamlet_props.py`).
HAMLET_PLATFORM = (1270, 1410, 592)
W, H, FLOOR = 2400, 784, 656
BOG = (600, 1100, 640)


def enemy(n, character, x, feet, w, h):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{MOOR}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def moor():
    bx0, bx1, water = BOG
    e = [
        piece("moor_inn_front", 0, 400, name="moor_inn_front"),
        piece("moor_wire", 200, 600, name="moor_wire"),
        door("moor_inn_door", "into the Crooked Lantern", 120, FLOOR, INN, "inn_door"),
        sign("inn_sign", 250, FLOOR - 260, "THE CROOKED LANTERN: open after curfew, which is always", 460),
        piece("moor_bog", 560, 580, name="moor_bog"),
        piece("moor_watchtower", 1180, 0, name="moor_watchtower"),
        piece("moor_statue", 1740, 500, name="moor_statue"),
        piece("moor_crypt_gate", 2250, 500, name="moor_crypt_gate"),
        # The stair down under the gate's porch to the crypt: a hole in the
        # ground, a ladder down it (`main`), the way down at its bottom.
        entity("LoadingZone", CRYPT_STAIR[0], H - 16, CRYPT_STAIR[1] - CRYPT_STAIR[0], 16, id="moor_crypt_stair",
               name="down the steps to the founder's crypt", activation="EdgeExit", target_room=CRYPT,
               target_zone="crypt_up", bidirectional=True),
        solid("ground_west", 0, FLOOR, bx0, H - FLOOR),
        solid("bog_bed", bx0, 720, bx1 - bx0, H - 720),
        solid("ground_east", bx1, FLOOR, CRYPT_STAIR[0] - bx1, H - FLOOR),
        solid("ground_gate", CRYPT_STAIR[1], FLOOR, W - CRYPT_STAIR[1], H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR - 112),
        edge("moor_west", "to the allotments", 0, FLOOR, ALLOT, "allot_east"),
        solid("east_wall", W - WALL, 0, WALL, FLOOR - 112),
        edge("moor_east", "back to the forest", W - WALL, FLOOR, FOREST, "forest_west"),
        start(MOOR, W - 200, FLOOR),
        *[ledge(f"tower_{i}", x0, y, x1 - x0) for i, (x0, x1, y) in enumerate(((1500, 1620, 560), (1540, 1660, 464), (1500, 1620, 368), (1520, 1680, 272)))],
        # Over the top of the watchtower, out of the room: the ridge.
        entity("LoadingZone", MOOR_TOWER_TOP[0], 0, MOOR_TOWER_TOP[1] - MOOR_TOWER_TOP[0], 16, id="moor_ridge",
               name="up onto the Searchlight Ridge", activation="EdgeExit", target_room=RIDGE, target_zone="ridge_tower",
               bidirectional=True),
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
        enemy(5, "npc_evil_zombie", 520, FLOOR, 30, 67),
        enemy(6, "npc_evil_zombie", 1960, 580, 30, 67),
    ]
    return MOOR, room(MOOR, e, at=(-W, 0), size=(W, H), biome="evil_forest", music="oppressive_crush",
                      bg="#3A0E0E", title="THE SEARCHLIGHT MOOR", generator=GENERATOR)


def allotments():
    w = ALLOT_W
    e = [
        piece("allotment_west", 40, 480, name="allotment_west"),
        piece("allotment_beds", 940, 420, name="allotment_beds"),
        # Plot 13's hole down to the catacombs: a ladder (`main`), the way
        # down at its bottom.
        solid("ground_west", 0, FLOOR, PLOT_13[0], H - FLOOR),
        solid("ground_east", PLOT_13[1], FLOOR, w - PLOT_13[1], H - FLOOR),
        # Over the greenhouse, the gap in the ridge: a drop down from it.
        entity("LoadingZone", ALLOT_ROOF[0], 0, ALLOT_ROOF[1] - ALLOT_ROOF[0], 16, id="allot_ridge_drop",
               name="under the ridge's gap", activation="EdgeExit", target_room=RIDGE, target_zone="ridge_gap",
               bidirectional=True),
        entity("LoadingZone", PLOT_13[0], H - 16, PLOT_13[1] - PLOT_13[0], 16, id="allot_plot_13",
               name="down the hole in plot 13", activation="EdgeExit", target_room=CATACOMBS, target_zone="catacombs_up",
               bidirectional=True),
        solid("west_wall", 0, 0, WALL, FLOOR - 112),
        edge("allot_west", "to Lower Godwin", 0, FLOOR, HAMLET, "hamlet_east"),
        solid("east_wall", w - WALL, 0, WALL, FLOOR - 112),
        edge("allot_east", "back to the moor", w - WALL, FLOOR, MOOR, "moor_west"),
        start(ALLOT, w - 200, FLOOR),
        *[ledge(f"bed_{i}", x0, top, x1 - x0) for i, (x0, x1, top) in enumerate(ALLOT_BEDS)],
        ledge("water_butt", ALLOT_BUTT[0], ALLOT_BUTT[2], ALLOT_BUTT[1] - ALLOT_BUTT[0]),
        ledge("greenhouse_roof", ALLOT_ROOF[0], ALLOT_ROOF[2], ALLOT_ROOF[1] - ALLOT_ROOF[0]),
        entity("ChestSpawn", 620, ALLOT_ROOF[2] - 58, 76, 58, name="the prize-money tin", reward="currency:60"),
        resident("npc_evil_crabbe", "evil_crabbe", 230, FLOOR),
        sign("allot_sign_plot", 340, FLOOR - 110, "PLOT 13: H. CRABBE. KEEP OUT. YES, YOU.", 380),
        sign("allot_sign_mandrake", 1640, FLOOR - 150, "DO NOT PULL THE MANDRAKES", 300),
        sign("allot_sign_show", 1480, FLOOR - 200, "BEST IN SHOW: the Malevolent Marrow", 360),
        sign("allot_sign_cabbage", 1340, FLOOR - 100, "CABBAGES: feed from the side", 300),
        enemy(20, "npc_evil_zombie", 1200, FLOOR, 30, 67),
        enemy(21, "npc_evil_attack_hound", 1880, FLOOR, 52, 50),
        enemy(22, "npc_evil_failed_prototype", 1340, ALLOT_BEDS[1][2], 32, 40),
        enemy(23, "npc_evil_zombie", 1060, ALLOT_BEDS[0][2], 30, 67),
    ]
    return ALLOT, room(ALLOT, e, at=(-W - w, 0), size=(w, H), biome="evil_forest", music="oppressive_crush",
                       bg="#3A0E0E", title="THE CASTLE ALLOTMENTS", generator=GENERATOR)


def hamlet():
    w = HAMLET_W
    p0, p1, py = HAMLET_PLATFORM
    e = [
        piece("hamlet_west", 90, FLOOR - 210, name="hamlet_west"),
        piece("hamlet_east", 1250, FLOOR - 180, name="hamlet_east"),
        solid("ground", 0, FLOOR, w, H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR),
        solid("east_wall", w - WALL, 0, WALL, FLOOR - 112),
        edge("hamlet_east", "to the allotments", w - WALL, FLOOR, ALLOT, "allot_west"),
        start(HAMLET, w - 200, FLOOR),
        ledge("crier_platform", p0, py, p1 - p0),
        resident("npc_evil_bellows", "evil_bellows", p0 + 50, py),
        resident("npc_evil_crumb", "evil_crumb", 780, FLOOR),
        door("hamlet_bakery_door", "into Crumb's bakery", BAKERY_DOOR_X, FLOOR, BAKERY, "bakery_door"),
        sign("hamlet_sign", 600, 140, "LOWER GODWIN: twinned with nowhere, by order", 460),
        sign("hamlet_sign_stocks", 1640, FLOOR - 100, "STOCKS: for complaining", 260),
        sign("hamlet_sign_decree", 1800, FLOOR - 190, "DECREE 47: smiling is compulsory on Tuesdays", 440),
        sign("hamlet_sign_bakery", 810, FLOOR - 240, "CRUMB'S: bread for the castle, bread for us", 420),
    ]
    return HAMLET, room(HAMLET, e, at=(-W - ALLOT_W - w, 0), size=(w, H), biome="evil_forest", music="argand_waltz",
                        bg="#3A0E0E", title="LOWER GODWIN", generator=GENERATOR)


#: Crumb's bakery inside, by its door in the hamlet (the art's doorway,
#: x 836..862); a building, so anywhere in the world (west of the inn).
BAKERY = "evil_crumbs_bakery"
BAKERY_AT, BAKERY_W, BAKERY_H, BAKERY_FLOOR = (-6800, 1024), 1024, 448, 384
BAKERY_DOOR_X = 826


def bakery():
    """Crumb's bakery inside: the castle's oven and the hamlet's, shelves of
    both breads, the counter; behind the flour sacks, the press of the Lower
    Godwin Gazette and Silas Prentice, who prints it."""
    w, h, g = BAKERY_W, BAKERY_H, BAKERY_FLOOR
    e = [
        piece("bakery_room", 16, 32, name="bakery_room"),
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, 32),
        solid("west_wall", 0, 32, WALL, g - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 32),
        door("bakery_door", "out to Lower Godwin", 40, g, HAMLET, "hamlet_bakery_door"),
        start(BAKERY, 140, g),
        resident("npc_evil_prentice", "evil_prentice", 830, g),
    ]
    return BAKERY, room(BAKERY, e, at=BAKERY_AT, size=(w, h), biome="castle_godwin", music="argand_waltz", bg="#2A1A14",
                        title="CRUMB'S BAKERY", generator=GENERATOR)


def inn():
    """The Crooked Lantern inside: the fire, the bar, the castle's WANTED
    posters and the locals' map behind the picture. Gudrun behind the bar,
    Tobias by the fire (`dialogue/sandbox/castle_godwin.yarn`)."""
    w, h, g = INN_W, INN_H, INN_FLOOR
    e = [
        piece("inn_room", 16, 32, name="inn_room"),
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, 32),
        solid("west_wall", 0, 32, WALL, g - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 32),
        door("inn_door", "out onto the moor", 40, g, MOOR, "moor_inn_door"),
        start(INN, 140, g),
        resident("npc_evil_tobias", "evil_tobias", 430, g),
        resident("npc_evil_gudrun", "evil_gudrun", 700, g),
        sign("inn_sign_rules", 512, 110, "NO SINGING THE OLD ANTHEM. HUM IT.", 420),
    ]
    return INN, room(INN, e, at=INN_AT, size=(w, h), biome="castle_godwin", music="argand_waltz", bg="#2A1A14",
                     title="THE CROOKED LANTERN", generator=GENERATOR)


def crypt():
    """The founder's crypt: niches of bones, his busts (still pointing),
    candles, the sarcophagus (its lid walked on, his medals on it), and
    what guards him."""
    w, h, g = CRYPT_W, CRYPT_H, CRYPT_FLOOR
    s0, s1, lid = SARCOPHAGUS
    e = [
        piece("crypt_room", 16, 32, name="crypt_room"),
        solid("floor", 0, g, w, h - g),
        solid("ceiling_west", 0, 0, CRYPT_SHAFT[0], 32),
        solid("ceiling_east", CRYPT_SHAFT[1], 0, w - CRYPT_SHAFT[1], 32),
        solid("west_wall", 0, 32, WALL, g - 112 - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 32),
        entity("LoadingZone", CRYPT_SHAFT[0], 0, CRYPT_SHAFT[1] - CRYPT_SHAFT[0], 16, id="crypt_up",
               name="up the steps to the moor", activation="EdgeExit", target_room=MOOR, target_zone="moor_crypt_stair",
               bidirectional=True),
        edge("crypt_west", "into the catacombs", 0, g, CATACOMBS, "catacombs_east"),
        start(CRYPT, CRYPT_SHAFT[0] - 120, g),
        ledge("sarcophagus_lid", s0, lid, s1 - s0),
        entity("ChestSpawn", s0 + 60, lid - 58, 76, 58, name="the founder's medals", reward="currency:100"),
        sign("crypt_sign", 800, 110, "HERE LIES OUR FOUNDER (mostly)", 380),
        sign("crypt_sign_busts", 760, g - 160, "He pointed the way. West. At nothing.", 380),
        enemy(10, "npc_evil_spectral_wolf", 620, g, 56, 56),
        enemy(11, "npc_evil_zombie", 900, g, 30, 67),
        # West, toward the catacombs: not by the steps down.
        enemy(12, "npc_evil_zombie", 260, g, 30, 67),
    ]
    return CRYPT, room(CRYPT, e, at=CRYPT_AT, size=(w, h), biome="castle_godwin", music="vault_in_the_dark", bg="#141012",
                       title="THE FOUNDER'S CRYPT", generator=GENERATOR)


def ridge():
    """High over the moor and the allotments: the Searchlight Ridge, up the
    moor's watchtower. Its searchlights sweep the red sky; a sentry hut and
    its sentries; a chest before the gap at the west end, where the ridge
    has fallen away over the allotments' greenhouse: drop down onto its
    roof (no way back up: a shortcut down)."""
    w, h, g = RIDGE_W, RIDGE_H, RIDGE_G
    t0, t1 = RIDGE_TOWER
    g0, g1 = RIDGE_GAP
    e = [
        piece("ridge_west", 16, 0, name="ridge_west"),
        piece("ridge_east", 1800, 0, name="ridge_east"),
        solid("ground_west_end", 0, g + 40, g0, h - g - 40),
        solid("ground", g1, g, t0 - g1, h - g),
        solid("ground_east", t1, g, w - t1, h - g),
        solid("west_wall", 0, 0, WALL, g + 40),
        solid("east_wall", w - WALL, 0, WALL, g),
        entity("LoadingZone", t0, h - 16, t1 - t0, 16, id="ridge_tower", name="down the watchtower to the moor",
               activation="EdgeExit", target_room=MOOR, target_zone="moor_ridge", bidirectional=True),
        entity("LoadingZone", g0, h - 16, g1 - g0, 16, id="ridge_gap", name="down through the gap to the allotments",
               activation="EdgeExit", target_room=ALLOT, target_zone="allot_ridge_drop", bidirectional=True),
        # The hatch at the watchtower's top: come up through it and stand on
        # it; drop through it to go down.
        ledge("tower_hatch", t0, g, t1 - t0),
        start(RIDGE, t0 - 120, g),
        entity("ChestSpawn", g1 + 160, g - 58, 76, 58, name="the sentry's thermos money", reward="currency:60"),
        sign("ridge_sign", 2380, 240, "THE SEARCHLIGHT RIDGE: smile, you are always lit", 460),
        sign("ridge_sign_gap", g1 + 40, 300, "MIND THE GAP (the gap minds you)", 320),
        enemy(40, "npc_fascist_enforcer", 2500, g, 32, 72),
        enemy(41, "npc_evil_attack_hound", 1900, g, 52, 50),
        enemy(42, "npc_fascist_enforcer", 1300, g, 32, 72),
    ]
    return RIDGE, room(RIDGE, e, at=RIDGE_AT, size=(w, h), biome="evil_forest", music="oppressive_crush", bg="#3A0E0E",
                       title="THE SEARCHLIGHT RIDGE", generator=GENERATOR)


def catacombs():
    """Under the allotments and the moor: the old burial tunnels from the
    crypt's west end to a shaft up under Crabbe's plot, 13, in the
    allotments (the roots of her prize marrows hang through the vault).
    Skulls in rows, a fall of rubble, a forgotten shrine; what walks here."""
    w, h, g = CATACOMBS_W, CRYPT_H, CRYPT_FLOOR
    s0, s1 = 40, 104
    e = [
        piece("catacombs_west", 16, 0, name="catacombs_west"),
        piece("catacombs_east", 1280, 32, name="catacombs_east"),
        solid("floor", 0, g, w, h - g),
        solid("ceiling_west", 0, 0, s0, 32),
        solid("ceiling_east", s1, 0, w - s1, 32),
        solid("west_wall", 0, 32, WALL, g - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 112 - 32),
        entity("LoadingZone", s0, 0, s1 - s0, 16, id="catacombs_up", name="up the shaft to the allotments",
               activation="EdgeExit", target_room=ALLOT, target_zone="allot_plot_13", bidirectional=True),
        edge("catacombs_east", "on to the founder's crypt", w - WALL, g, CRYPT, "crypt_west"),
        start(CATACOMBS, 200, g),
        sign("catacombs_sign", 420, 120, "PLOT 13 (from underneath)", 300),
        sign("catacombs_sign_shrine", 1840, 220, "TO WHOEVER THIS WAS", 260),
        enemy(30, "npc_evil_zombie", 760, g, 30, 67),
        enemy(31, "npc_evil_failed_prototype", 1180, g, 32, 40),
        enemy(32, "npc_evil_zombie", 1700, g, 30, 67),
        enemy(33, "npc_evil_spectral_wolf", 2100, g, 56, 56),
    ]
    return CATACOMBS, room(CATACOMBS, e, at=CATACOMBS_AT, size=(w, h), biome="castle_godwin", music="vault_in_the_dark",
                           bg="#141012", title="THE CATACOMBS", generator=GENERATOR)


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
    paths = []
    # The inn first (it moves out of the catacombs' way), then the crypt.
    for rid, text in (inn(), crypt(), catacombs(), ridge(), bakery(), hamlet(), allotments(), moor()):
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    # The moor's east edge and the forest's west edge name each other, so the
    # check after the first of the two writes refuses the half-joined world;
    # only the end state is checked (the last `validate`).
    if code := make_rooms(WORLD, paths, keep_position="--relayout" not in argv):
        return code
    # The ladders: down the crypt gate's steps, up the crypt's and the
    # catacombs' shafts, down the hole in plot 13.
    for args in ((MOOR, "Climbable", 1, CRYPT_STAIR[0] + 16, FLOOR - 64, 32, H - FLOOR + 64),
                 (CRYPT, "Climbable", 1, CRYPT_SHAFT[0] + 16, 0, 32, CRYPT_FLOOR),
                 (CATACOMBS, "Climbable", 1, 56, 0, 32, CRYPT_FLOOR),
                 (ALLOT, "Climbable", 1, PLOT_13[0] + 16, FLOOR - 64, 32, H - FLOOR + 64),
                 # The watchtower's top ladder, from its last landing (272) up
                 # out of the moor: a climb, so the body comes up into the
                 # ridge on its ladder, not falling back through the hole.
                 (MOOR, "Climbable", 1, MOOR_TOWER_TOP[0] + 44, 0, 32, 272),
                 # Up out of the hole the watchtower comes up through.
                 (RIDGE, "Climbable", 1, RIDGE_TOWER[0] + 44, RIDGE_G - 80, 32, RIDGE_H - RIDGE_G + 80)):
        if code := paint(WORLD, *args):
            return code
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
