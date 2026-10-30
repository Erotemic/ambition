#!/usr/bin/env python3
"""Write the Tar Pits, out past the end of the primeval trail, east of the
canopy.

    python specs/primeval/tar_pits.py          # writes the spec
    python specs/primeval/tar_pits.py --apply  # and makes the room

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "out of town zones with enemies", joined by side exits on
a flat map. The pits touch the canopy's east edge with the same floor (world
y 480). The canopy is edited in place (`side_exits.py`: the primeval
generator is older than its levels), so its east exit is ADDED to it and its
wall opened.

- Two pools of tar (murky water, its picture in the art) to wade, slowly.
- A mammoth half sunk in the first, its spine walked on (y 480).
- The paleontologists' dig camp at the east end, its scaffold (y 480, 384).
- Raptors, a triceratops, a dilophosaurus, compys, a vulture waiting; Dr
  Feld the paleontologist, who will not leave.
- East, by a side exit: the Smoking Mountain, its shelves up to a crater.
- At the trailhead, the ranger's hut gets a deck and a door, and an inside:
  the ranger station, Ranger Dlamini on the radio.
- Under the ridge road, down a sinkhole in it (a vertical exit and a
  ladder): the fossil beds, Dr Stratton the geologist and her strata.

The art is the sprite tools' `targets/props/primeval_tar_props.py`, drawn on
these numbers.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WALL, WORLDS, add_entities, breakable_wall, edge, entity, make_rooms, paint, resident, room, run, sign, solid, start  # noqa: E402
from district_scenery import body_box, piece  # noqa: E402

WORLD = WORLDS / "dinosaur.ldtk"
GENERATOR = "specs/primeval/tar_pits.py"
PITS = "primeval_tar_pits"
#: Past the pits, east by a side exit: the Smoking Mountain.
MOUNTAIN = "primeval_smoking_mountain"
CANOPY = "primeval_canopy"
#: The canopy's east edge in its room (x), its floor (y), and where it is in
#: the world: the pits start where it ends, floors level.
CANOPY_EAST, CANOPY_FLOOR, CANOPY_AT = 1984, 672, (30816, -192)
W, H, FLOOR, BED = 2400, 640, 576, 624
POOLS = ((520, 900), (1300, 1700))


def walk(name, x, y, w):
    """A one-way walk the room's art draws: a still platform, no picture."""
    return entity("MovingPlatform", x, y, w, 16, id=name, name=name, speed=0.0, visual="none", one_way=True)


def enemy(n, character, x, feet, w, h, room=PITS):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{room}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def pits():
    (a0, a1), (b0, b1) = POOLS
    e = [
        piece("tar_pits_bones", 120, 500, name="tar_bones"),
        piece("tar_pits_pools", 500, 440, name="tar_pools"),
        piece("tar_pits_mammoth", 380, 410, name="tar_mammoth"),
        piece("tar_pits_dig_camp", 1730, 360, name="tar_dig_camp"),
        solid("ground_west", 0, FLOOR, a0, H - FLOOR),
        solid("bed_west", a0, BED, a1 - a0, H - BED),
        solid("ground_mid", a1, FLOOR, b0 - a1, H - FLOOR),
        solid("bed_east", b0, BED, b1 - b0, H - BED),
        solid("ground_east", b1, FLOOR, W - b1, H - FLOOR),
        solid("west_wall", 0, 0, WALL, FLOOR - 112),
        solid("east_wall", W - WALL, 0, WALL, FLOOR - 112),
        edge("tar_east", "on up the Smoking Mountain", W - WALL, FLOOR, MOUNTAIN, "mountain_west"),
        edge("tar_west", "back to the canopy", 0, FLOOR, CANOPY, "primeval_canopy_tar_pits"),
        start(PITS, 120, FLOOR),
        walk("mammoth_back", 600, 480, 240),
        walk("scaffold_low", 1980, 480, 160),
        walk("scaffold_high", 2040, 384, 160),
        # A chest for whoever climbs to the top (Jon, 2026-10-30: "interactive elements").
        entity("ChestSpawn", 2100, 384 - 58, 76, 58, name="the dig's best find", reward="currency:75"),
        sign("tar_sign", 300, FLOOR - 200, "THE TAR PITS: everything here was once in a hurry", 480),
        sign("tar_sign_wet", 1500, FLOOR - 150, "WET TAR (since the Pleistocene)", 340),
        sign("dig_sign", 2120, 300, "DIG SITE: please do not fossilize", 380),
        # Dr Feld, at his sieve (`dialogue/sandbox/primeval.yarn`).
        resident("npc_primeval_feld", "primeval_feld", 2190, FLOOR),
        enemy(1, "npc_raptor_stalker", 1000, FLOOR, 48, 80),
        enemy(2, "npc_raptor_stalker", 1180, FLOOR, 48, 80),
        # The dig camp (x 1730..) is the one quiet place: the animals keep
        # to the west of it, the triceratops on the camp's doorstep.
        enemy(3, "triceratops", 1720, FLOOR, 96, 92),
        enemy(4, "dilophosaurus", 340, FLOOR, 44, 74),
        enemy(5, "compsognathus", 960, FLOOR, 24, 26),
        enemy(6, "compsognathus", 1240, FLOOR, 24, 26),
        enemy(7, "vulture", 700, 420, 44, 66),
    ]
    at = (CANOPY_AT[0] + CANOPY_EAST + WALL, CANOPY_AT[1] + CANOPY_FLOOR - FLOOR)
    return room(PITS, e, at=at, size=(W, H), biome="primeval", music="dinosaur_liberators", bg="#2E3B22",
                title="THE TAR PITS", generator=GENERATOR)


#: The mountain (the art, `targets/props/primeval_volcano_props.py`, is
#: drawn on these): its height, the shelves walked on (x0, x1, top), the
#: lava pits between them (x0, x1, the lava's top, the bed), the crater.
MOUNTAIN_W, MOUNTAIN_H = 2400, 960
SHELVES = ((0, 520, 896), (660, 1000, 800), (1000, 1300, 704), (1440, 1760, 608), (1760, 2000, 512), (2000, 2200, 416))
LAVA_PITS = ((520, 660, 912, 944), (1300, 1440, 752, 784))
CRATER = (2200, 2384, 448, 480)


def smoking_mountain():
    """Past the tar pits: the Smoking Mountain. Basalt shelves climb east
    to the crater's rim, lava in the pits between them and in the crater (a
    DamageVolume each); on the rim, the meteor watch (a chest). Raptors, a
    dilophosaurus, a pachycephalosaurus, compys, a vulture over the rim."""
    w, h = MOUNTAIN_W, MOUNTAIN_H
    g = SHELVES[0][2]
    e = [
        piece("volcano_west", 0, 300, name="volcano_west"),
        piece("volcano_east", 1192, 0, name="volcano_east"),
        *[solid(f"shelf_{i}", x0, top, x1 - x0, h - top) for i, (x0, x1, top) in enumerate(SHELVES)],
        *[solid(f"pit_bed_{i}", x0, bed, x1 - x0, h - bed) for i, (x0, x1, _top, bed) in enumerate(LAVA_PITS + (CRATER,))],
        *[entity("DamageVolume", x0, top, x1 - x0, bed - top, name=f"lava_{i}", damage=1)
          for i, (x0, x1, top, bed) in enumerate(LAVA_PITS + (CRATER,))],
        solid("west_wall", 0, 0, WALL, g - 112),
        solid("east_wall", w - WALL, 0, WALL, h),
        edge("mountain_west", "back down to the tar pits", 0, g, PITS, "tar_east"),
        start(MOUNTAIN, 120, g),
        entity("ChestSpawn", 2030, SHELVES[-1][2] - 58, 76, 58, name="the meteor watch's thermos", reward="health:2"),
        # Professor Tock, keeping the meteor watch (`primeval.yarn`).
        resident("npc_primeval_tock", "primeval_tock", 2160, SHELVES[-1][2]),
        sign("mountain_sign", 300, g - 220, "THE SMOKING MOUNTAIN: not a volcano (a volcano would need a permit)", 560),
        sign("lava_sign", 590, g - 90, "HOT", 80),
        sign("meteor_sign", 2100, SHELVES[-1][2] - 140, "METEOR WATCH: any day now (66,000,000 years and counting)", 520),
        enemy(1, "npc_raptor_stalker", 800, SHELVES[1][2], 48, 80, room=MOUNTAIN),
        enemy(2, "dilophosaurus", 1560, SHELVES[3][2], 44, 74, room=MOUNTAIN),
        enemy(3, "pachycephalosaurus", 1860, SHELVES[4][2], 44, 72, room=MOUNTAIN),
        enemy(4, "compsognathus", 300, g, 24, 26, room=MOUNTAIN),
        enemy(5, "compsognathus", 1100, SHELVES[2][2], 24, 26, room=MOUNTAIN),
        enemy(6, "vulture", 2120, SHELVES[-1][2] - 200, 44, 66, room=MOUNTAIN),
    ]
    at = (CANOPY_AT[0] + CANOPY_EAST + WALL + W, CANOPY_AT[1] + CANOPY_FLOOR - g)
    return room(MOUNTAIN, e, at=at, size=(w, h), biome="primeval", music="liberated_dinosaur_libertarians", bg="#3A2A22",
                title="THE SMOKING MOUNTAIN", generator=GENERATOR)


CANOPY_EDGE_YAML = f"""# The canopy's way east to the tar pits. Written by {GENERATOR}.
level_id: {CANOPY}
entities:
  - type: LoadingZone
    px: [{CANOPY_EAST}, {CANOPY_FLOOR - 96}]
    size: [{WALL}, 96]
    fields:
      id: primeval_canopy_tar_pits
      name: on to the tar pits
      activation: EdgeExit
      target_room: {PITS}
      target_zone: tar_west
      bidirectional: true
"""


#: The ranger's hut at the trailhead (`jungle_props.py` `jungle_ranger_hut`,
#: placed at (470, 330) by `scenery_overlay.py`): its deck (x 480..680, y
#: 434) and its doorway (x 516..556); and the station inside, a building, so
#: anywhere in the world (by the tyrant's hall, out of the way).
TRAILHEAD = "primeval_trailhead"
STATION = "primeval_ranger_station"
STATION_AT, STATION_W, STATION_H, STATION_FLOOR = (44000, 800), 1024, 512, 448
HUT_DECK = (480, 680, 434)
HUT_DOOR_X = 516


def ranger_station():
    """The ranger station inside: the trail's big map, the radio, specimen
    jars, a cot, the lost-property box. Ranger Dlamini on the radio
    (`primeval.yarn`)."""
    w, h, g = STATION_W, STATION_H, STATION_FLOOR
    e = [
        piece("ranger_station_room", 16, 32, name="ranger_station_room"),
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, 32),
        solid("west_wall", 0, 32, WALL, g - 32),
        solid("east_wall", w - WALL, 32, WALL, g - 32),
        entity("LoadingZone", 40, g - 96, 48, 96, id="station_door", name="out to the trailhead", activation="Door",
               target_room=TRAILHEAD, target_zone="trailhead_ranger_door", bidirectional=True),
        start(STATION, 140, g),
        resident("npc_primeval_dlamini", "primeval_dlamini", 660, g),
        sign("station_sign_map", 520, 250, "THE TRAIL: you are here (please stay here)", 380),
        sign("station_sign_lost", 930, g - 70, "LOST PROPERTY", 140),
    ]
    return room(STATION, e, at=STATION_AT, size=(w, h), biome="primeval", music="liberated_dinosaur_libertarians", bg="#2A2018",
                title="THE RANGER STATION", generator=GENERATOR)


#: The ridge road (edited in place) and its sinkhole (x0, x1), its road (y
#: 480), where it is in the world; the fossil beds under it, their shaft
#: under the sinkhole (`primeval_fossil_props.py` draws on these).
RIDGE = "primeval_ridge"
RIDGE_AT, RIDGE_ROAD, RIDGE_H = (27408, 0), 480, 512
SINKHOLE = (1400, 1464)
FOSSILS = "primeval_fossil_beds"
FOSSILS_X0 = 1000
FOSSILS_W, FOSSILS_H, FOSSILS_FLOOR, FOSSILS_ROOF = 1600, 640, 576, 64
FOSSILS_SHAFT = (SINKHOLE[0] - FOSSILS_X0, SINKHOLE[1] - FOSSILS_X0)
FOSSILS_SCAFFOLD = ((900, 1220, 432), (980, 1140, 320))
#: West of the beds, behind the cracked wall over their way west (a
#: breakable wall: hit it; broken for good in the save): the amber vault.
AMBER = "primeval_amber_vault"
AMBER_W = 640


def fossil_enemy(n, character, x, feet, w, h):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{FOSSILS}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def fossil_beds():
    """Under the ridge road, down its sinkhole: the rock the trail stands
    on, cut open. Its strata, the fossils in them, a great skull under a
    survey scaffold; Dr Stratton by her board of the layers
    (`primeval.yarn`); compys at the east end, eating her core samples."""
    w, h, g, top = FOSSILS_W, FOSSILS_H, FOSSILS_FLOOR, FOSSILS_ROOF
    s0, s1 = FOSSILS_SHAFT
    e = [
        piece("fossil_beds", 16, 16, name="fossil_beds"),
        solid("floor", 0, g, w, h - g),
        solid("roof_west", 0, 0, s0, top),
        solid("roof_east", s1, 0, w - s1, top),
        solid("west_wall", 0, top, WALL, g - 112 - top),
        # The way west, cracked over (`secret_`: the map shows the vault only
        # once it is found).
        breakable_wall("primeval_fossil_cracked_wall", WALL, g - 112, 32, 112, look="fossil_cracked_wall", hp=3,
                       name="a cracked wall, amber in the crack"),
        edge("secret_fossils_west", "through the cracked wall", 0, g, AMBER, "secret_amber_east"),
        solid("east_wall", w - WALL, top, WALL, g - top),
        entity("LoadingZone", s0, 0, s1 - s0, 16, id="fossils_shaft", name="up the sinkhole to the ridge road",
               activation="EdgeExit", target_room=RIDGE, target_zone="ridge_sinkhole", bidirectional=True),
        start(FOSSILS, 300, g),
        *[entity("OneWayPlatform", x0, y, x1 - x0, 16, name=f"survey_scaffold_{i}") for i, (x0, x1, y) in enumerate(FOSSILS_SCAFFOLD)],
        resident("npc_primeval_stratton", "primeval_stratton", 680, g),
        sign("fossils_sign", 1300, 110, "THE FOSSIL BEDS: everything in its layer", 380),
        sign("fossils_sign_skull", 1000, 476, "DO NOT LICK THE SKULL (it is not salt)", 340),
        entity("ChestSpawn", 1130, 432 - 58, 76, 58, name="the type specimen's tin", reward="currency:60"),
        fossil_enemy(1, "compsognathus", 1300, g, 24, 26),
        fossil_enemy(2, "compsognathus", 1360, g, 24, 26),
        fossil_enemy(3, "compsognathus", 1420, g, 24, 26),
        fossil_enemy(4, "npc_raptor_stalker", 1480, g, 48, 80),
    ]
    at = (RIDGE_AT[0] + FOSSILS_X0, RIDGE_AT[1] + RIDGE_H)
    return room(FOSSILS, e, at=at, size=(w, h), biome="primeval", music="liberated_dinosaur_libertarians", bg="#2A2018",
                title="THE FOSSIL BEDS", generator=GENERATOR)


def amber_vault():
    """Behind the fossil beds' cracked wall: a pocket of amber the survey
    missed. A mosquito in it (very full), a beetle, a fern, a tiny foot, and
    a ranger's sandwich nobody can explain."""
    w, h, g, top = AMBER_W, FOSSILS_H, FOSSILS_FLOOR, FOSSILS_ROOF
    e = [
        piece("amber_vault_room", 16, 16, name="amber_vault_room"),
        solid("floor", 0, g, w, h - g),
        solid("roof", 0, 0, w, top),
        solid("west_wall", 0, top, WALL, g - top),
        solid("east_wall", w - WALL, top, WALL, g - 112 - top),
        edge("secret_amber_east", "back through the cracked wall", w - WALL, g, FOSSILS, "secret_fossils_west"),
        start(AMBER, w - 120, g),
        entity("ChestSpawn", 120, g - 58, 76, 58, name="the survey's missing samples", reward="currency:120"),
        sign("amber_sign", 320, 100, "AMBER: do not extract anything from anything", 420),
    ]
    at = (RIDGE_AT[0] + FOSSILS_X0 - w, RIDGE_AT[1] + RIDGE_H)
    return room(AMBER, e, at=at, size=(w, h), biome="primeval", music="liberated_dinosaur_libertarians", bg="#2A2018",
                title="THE AMBER VAULT", generator=GENERATOR)


def sinkhole_yaml() -> str:
    """The ridge road's sinkhole: its picture over the hole (the hole is cut
    in the road's collision by `main`), the way down at its bottom, a sign."""
    bx, by, bw, bh = body_box("ridge_sinkhole")
    fx, fy = SINKHOLE[0] - 20, RIDGE_ROAD - 70
    s0, s1 = SINKHOLE
    return f"""# The ridge road's sinkhole down to the fossil beds. Written by {GENERATOR}.
level_id: {RIDGE}
entities:
  - type: Prop
    px: [{round(fx + bx / 2)}, {round(fy + by / 2)}]
    size: [{max(1, round(bw / 2))}, {max(1, round(bh / 2))}]
    fields:
      name: ridge_sinkhole
      kind: ridge_sinkhole
  - type: LoadingZone
    px: [{s0}, {RIDGE_H - 16}]
    size: [{s1 - s0}, 16]
    fields:
      id: ridge_sinkhole
      name: down the sinkhole to the fossil beds
      activation: EdgeExit
      target_room: {FOSSILS}
      target_zone: fossils_shaft
      bidirectional: true
  - type: DebugLabel
    px: [{s0 - 90}, {RIDGE_ROAD - 190}]
    size: [240, 24]
    fields:
      name: ridge_sign_sinkhole
      text: "SINKHOLE: fossils, do not feed"
      category: Custom
"""


HUT_DOOR_YAML = f"""# The ranger hut's deck and its door, at the trailhead. Written by {GENERATOR}.
level_id: {TRAILHEAD}
entities:
  - type: OneWayPlatform
    px: [{HUT_DECK[0]}, {HUT_DECK[2]}]
    size: [{HUT_DECK[1] - HUT_DECK[0]}, 16]
    fields:
      name: ranger_hut_deck
  - type: LoadingZone
    px: [{HUT_DOOR_X}, {HUT_DECK[2] - 96}]
    size: [48, 96]
    fields:
      id: trailhead_ranger_door
      name: into the ranger station
      activation: Door
      target_room: {STATION}
      target_zone: station_door
      bidirectional: true
"""


def main(argv):
    paths = []
    for rid, text in ((PITS, pits()), (MOUNTAIN, smoking_mountain()), (STATION, ranger_station()), (FOSSILS, fossil_beds()), (AMBER, amber_vault())):
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    # The pits' west edge and the canopy's east edge name each other, so the
    # check after the first of the two writes refuses the half-joined world;
    # only the end state is checked (the last `validate`).
    make_rooms(WORLD, paths)
    if code := add_entities(CANOPY_EDGE_YAML, WORLD, HERE / ".canopy_edge.yaml", unless='"on to the tar pits"'):
        return code
    if code := add_entities(HUT_DOOR_YAML, WORLD, HERE / ".hut_door.yaml", unless='"into the ranger station"'):
        return code
    if code := add_entities(sinkhole_yaml(), WORLD, HERE / ".sinkhole.yaml", unless='"down the sinkhole to the fossil beds"'):
        return code
    s0, s1 = SINKHOLE
    for args in ((CANOPY, "Collision", 0, CANOPY_EAST, CANOPY_FLOOR - 112, WALL, 112),
                 # The sinkhole through the ridge road, its ladder from over
                 # the road (as the woods' hole to the Mobius Mine), and the
                 # beds' shaft.
                 (RIDGE, "Collision", 0, s0, RIDGE_ROAD, s1 - s0, RIDGE_H - RIDGE_ROAD),
                 (RIDGE, "Climbable", 1, s0 + 16, RIDGE_ROAD - 64, 32, RIDGE_H - RIDGE_ROAD + 64),
                 (FOSSILS, "Climbable", 1, FOSSILS_SHAFT[0] + 16, 0, 32, FOSSILS_FLOOR),
                 # The tar: murky water (the IntGrid Water layer's kind 2).
                 *((PITS, "Water", 2, x0, FLOOR, x1 - x0, BED - FLOOR) for x0, x1 in POOLS)):
        if code := paint(WORLD, *args):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
