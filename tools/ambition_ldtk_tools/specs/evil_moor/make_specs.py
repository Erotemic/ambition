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
- Beside the founder's plinth, a gate down to his crypt.
- West of the moor by a side exit (floor y 656), the Castle Allotments:
  where the castle grows its vegetables (pumpkins that scowl, cabbages that
  bite, mandrakes not to be pulled), the greenhouse's roof to climb to a
  chest, Hester Crabbe the head gardener behind her fence, and what has got
  into the beds.
- West of the allotments by a side exit, Lower Godwin: the hamlet under the
  castle, its cottages, bakery, well, stocks and decree board; Barnaby
  Bellows the town crier on his platform, Prudence Crumb the baker.

The art is the sprite tools' `targets/props/evil_moor_props.py`, drawn on
these numbers.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, door, edge, entity, ledge, make_rooms, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import piece  # noqa: E402

WORLD = WORLDS / "evil.ldtk"
GENERATOR = "specs/evil_moor/make_specs.py"
MOOR = "evil_searchlight_moor"
#: The inn at the moor's far west end, and where its room is in the world.
INN = "evil_crooked_lantern"
INN_AT, INN_W, INN_H, INN_FLOOR = (-2400, 1024), 1024, 512, 448
#: The founder's crypt, down a gate beside his plinth, and where it is.
CRYPT = "evil_founders_crypt"
CRYPT_AT, CRYPT_W, CRYPT_H, CRYPT_FLOOR = (-2400, 1600), 1600, 576, 512
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
        door("moor_crypt_door", "down into the founder's crypt", 2290, FLOOR, CRYPT, "crypt_door"),
        solid("ground_west", 0, FLOOR, bx0, H - FLOOR),
        solid("bog_bed", bx0, 720, bx1 - bx0, H - 720),
        solid("ground_east", bx1, FLOOR, W - bx1, H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR - 112),
        edge("moor_west", "to the allotments", 0, FLOOR, ALLOT, "allot_east"),
        solid("east_wall", W - WALL, 0, WALL, FLOOR - 112),
        edge("moor_east", "back to the forest", W - WALL, FLOOR, FOREST, "forest_west"),
        start(MOOR, W - 200, FLOOR),
        *[ledge(f"tower_{i}", x0, y, x1 - x0) for i, (x0, x1, y) in enumerate(((1500, 1620, 560), (1540, 1660, 464), (1500, 1620, 368), (1520, 1680, 272)))],
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
        solid("ground", 0, FLOOR, w, H - FLOOR),
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
        sign("hamlet_sign", 600, 140, "LOWER GODWIN: twinned with nowhere, by order", 460),
        sign("hamlet_sign_stocks", 1640, FLOOR - 100, "STOCKS: for complaining", 260),
        sign("hamlet_sign_decree", 1800, FLOOR - 190, "DECREE 47: smiling is compulsory on Tuesdays", 440),
        sign("hamlet_sign_bakery", 810, FLOOR - 240, "CRUMB'S: bread for the castle, bread for us", 420),
    ]
    return HAMLET, room(HAMLET, e, at=(-W - ALLOT_W - w, 0), size=(w, H), biome="evil_forest", music="argand_waltz",
                        bg="#3A0E0E", title="LOWER GODWIN", generator=GENERATOR)


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
        solid("ceiling", 0, 0, w, 32),
        solid("west_wall", 0, 32, WALL, g - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 32),
        door("crypt_door", "back up to the moor", 40, g, MOOR, "moor_crypt_door"),
        start(CRYPT, 140, g),
        ledge("sarcophagus_lid", s0, lid, s1 - s0),
        entity("ChestSpawn", s0 + 60, lid - 58, 76, 58, name="the founder's medals", reward="currency:100"),
        sign("crypt_sign", 800, 110, "HERE LIES OUR FOUNDER (mostly)", 380),
        sign("crypt_sign_busts", 760, g - 160, "He pointed the way. West. At nothing.", 380),
        enemy(10, "npc_evil_spectral_wolf", 620, g, 56, 56),
        enemy(11, "npc_evil_zombie", 900, g, 30, 67),
        enemy(12, "npc_evil_zombie", 1380, g, 30, 67),
    ]
    return CRYPT, room(CRYPT, e, at=CRYPT_AT, size=(w, h), biome="castle_godwin", music="vault_in_the_dark", bg="#141012",
                       title="THE FOUNDER'S CRYPT", generator=GENERATOR)


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
    for rid, text in (inn(), crypt(), hamlet(), allotments(), moor()):
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    # The moor's east edge and the forest's west edge name each other, so the
    # check after the first of the two writes refuses the half-joined world;
    # only the end state is checked (the last `validate`).
    make_rooms(WORLD, paths)
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
