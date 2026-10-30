#!/usr/bin/env python3
"""Write the rooms of the Ninja Village, and the door to it from the Shadow Dojo.

    python specs/ninja_village/make_specs.py          # writes the specs
    python specs/ninja_village/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The Ninja Village is a mountain village of the ninja dimension, its own world
file (`pvn.ldtk`, the PvN dimension of pirates and ninjas). Gate address SILENT arrives at its gate shrine.
The design is in `README.md` here. In short:

- `ninja_village_gate_shrine`: the shrine where the ring stands (Agent GATE
  places the ring and its terminal on the clear floor at the west end). Its
  keeper sweeps; Nagisa the Gatekeeper keeps the ring; a courier is sick
  from the crossing. A stone stair goes up to the torii terrace.
- `ninja_village_street`: the main street, the tea house, and the houses
  whose roofs are a road of their own. The dojo's door.
- `ninja_village_dojo`: the village dojo and its rank exam (an encounter).
- `ninja_village_forest`: the cedar forest. Its branches step down from the
  west to the floor; at its east end, the door to the Shadow Dojo (the
  `ninja_dojo`, the clan of the Shadow Oni).

THE LOOP: the low road (shrine floor, street, forest floor) and the high
road (the forest's branches, the roofs of the street, the torii terrace and
the shrine's stair).

The problem: the dojo's rank exam (`ninja_village_rank_exam` in
`quests.ron`, the encounter `ninja_village.rank_exam`, the talk
`ninja_village.yarn`).
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE))

from district_kit import (  # noqa: E402
    HUB_ROOM, SANDBOX_LDTK, WALL, WORLDS, add_entities, door, edge, ensure_world, entity, ledge, make_rooms, paint, resident, room, run, sign, solid, start,
)

import props  # noqa: E402  (this directory's scenery)

#: The PvN dimension's world file (Jon, 2026-10-29: the pirate and ninja
#: dimensions are one): the port and the village are both in it.
WORLD_LDTK = WORLDS / "pvn.ldtk"
UID_OFFSET = 700000
GENERATOR = "specs/ninja_village/make_specs.py"

SHRINE = "ninja_village_gate_shrine"
STREET = "ninja_village_street"
DOJO = "ninja_village_dojo"
FOREST = "ninja_village_forest"
#: Out of the village (Jon, 2026-10-30: "out of town zones with enemies"):
#: the Bamboo Pass, east of the forest by a side exit.
PASS = "ninja_bamboo_pass"

#: The contract with Agent GATE: the arrival point a dialled SILENT names.
ARRIVAL = "ninja_village_gate_arrival"
#: Until GATE's ring and return terminal stand, the arrival zone is a
#: one-way door home for testing: GATE makes it the ring's arrival pad.

#: The Shadow Dojo (in this dimension's world file since 2026-10-29), and its door to the forest.
CLAN = "ninja_dojo"
CLAN_DOOR = "ninja_dojo_village_door"

#: The rank exam: its encounter, and the room's lever that sets it again.
EXAM = "ninja_village.rank_exam"

BIOME = "ninja_village"
BG = "#6F8FA8"
VILLAGE_MUSIC = "morning_on_the_eaves"
FOREST_MUSIC = "the_last_lantern"
DOJO_MUSIC = "nine_rooftops"

#: The village is 10000 px under the port in the PvN world file (`pvn.ldtk`):
#: the two were laid out each in its own file, from the same corner.
VILLAGE_Y = 10000
#: The shrine, the street and the forest touch where their side exits join,
#: with the low road level (`specs/check_flat_map.py`). The high road from the
#: street's last roof (y 384) arrives on the forest's first branch 64 higher:
#: both are drawn on their heights. The dojo is a building.
ROOMS_AT = {
    SHRINE: ((0, VILLAGE_Y), (1600, 832)),
    STREET: ((1600, VILLAGE_Y - 64), (2816, 896)),
    FOREST: ((4416, VILLAGE_Y - 128), (2816, 960)),
    DOJO: ((1856, VILLAGE_Y + 1152), (1600, 576)),
    PASS: ((4416 + 2816, VILLAGE_Y - 128), (2400, 960)),
}


def build(rid, entities, title, music=VILLAGE_MUSIC, **kw):
    at, size = ROOMS_AT[rid]
    # The room's scenery (`props.py`) is listed first: it draws behind.
    entities = [*props.scenery(rid), *entities]
    return rid, room(rid, entities, at=at, size=size, biome=BIOME, music=music, bg=BG, title=title, generator=GENERATOR, **kw)


def shrine():
    """The gate shrine. Floor 704. The ring stands on the clear floor of the
    west end (x 160..512 is kept clear to 224 over the floor). A stone stair
    of three steps goes up to the torii terrace (y 448); the floor and the
    terrace both go on east to the street."""
    rid = SHRINE
    _, (w, h) = ROOMS_AT[rid]
    floor, terrace = 704, 448
    e = [
        solid("ground", 0, floor, w, h - floor),
        solid("west_wall", 0, 0, WALL, floor),
        solid("east_wall_high", w - WALL, 0, WALL, terrace - 96),
        solid("east_wall_mid", w - WALL, terrace + 16, WALL, floor - 96 - terrace - 16),
        # The arrival is the zone of the gate ring (`gate_interface.yaml`, written
        # by specs/gate_network/make_gates.py), added after the rooms are made.
        start(rid, 336, floor),
        # Nagisa the Gatekeeper keeps the gate itself (Jon, 2026-10-25); Sato
        # keeps the shrine. She stands east of the ring's clear floor.
        resident("npc_village_nagisa", "ninja_village_nagisa", 548, floor),
        resident("npc_village_sato", "ninja_village_sato", 640, floor),
        resident("npc_village_kenji", "ninja_village_kenji", 800, floor),
        sign("shrine_sign_gate", 336, floor - 300, "SILENT: the gate shrine. Bow if you like.", 380),
        sign("shrine_sign_bench", 800, floor - 140, "Bench for the gate-sick", 240),
        ledge("stone_stair_1", 960, floor - 64, 96),
        ledge("stone_stair_2", 1040, floor - 128, 96),
        ledge("stone_stair_3", 1120, floor - 192, 96),
        ledge("torii_terrace", 1192, terrace, w - 1192),
        edge("shrine_east_low", "down to the village", w - WALL, floor, STREET, "street_west_low"),
        edge("shrine_east_high", "over the roofs", w - WALL, terrace, STREET, "street_west_high"),
    ]
    return build(rid, e, "THE NINJA VILLAGE: THE GATE SHRINE")


#: The roofs of the street, west to east: (name, x0, x1, ridge y). Each
#: step up is 64 or less and each gap 80, a run and a jump.
ROOFS = [
    ("roof_baker", 16, 480, 512),
    ("roof_weaver", 560, 1000, 480),
    ("roof_tea_house", 1080, 1480, 544),
    ("roof_dojo", 1560, 1960, 448),
    ("roof_granary", 2040, 2400, 416),
    ("roof_watch_house", 2480, 2800, 384),
]


def street():
    """The main street (y 768), the length of the room. Over it the roofs of
    the houses are a road of their own, from the shrine's terrace in the west
    to the forest's branches in the east. A ladder goes up the baker's wall
    from the street to the first roof."""
    rid = STREET
    _, (w, h) = ROOMS_AT[rid]
    ground = 768
    west_roof, east_roof = ROOFS[0][3], ROOFS[-1][3]
    e = [
        solid("street", 0, ground, w, h - ground),
        solid("west_wall_high", 0, 0, WALL, west_roof - 96),
        solid("west_wall_mid", 0, west_roof + 16, WALL, ground - 96 - west_roof - 16),
        solid("east_wall_high", w - WALL, 0, WALL, east_roof - 96),
        solid("east_wall_mid", w - WALL, east_roof + 16, WALL, ground - 96 - east_roof - 16),
        edge("street_west_low", "to the gate shrine", 0, ground, SHRINE, "shrine_east_low"),
        edge("street_west_high", "to the torii terrace", 0, west_roof, SHRINE, "shrine_east_high"),
        start(rid, 120, ground),
        *[ledge(name, x0, y, x1 - x0) for name, x0, x1, y in ROOFS],
        resident("npc_village_daichi", "ninja_village_daichi", 760, ground),
        resident("npc_village_ume", "ninja_village_ume", 1280, ground),
        sign("tea_house_sign", 1280, ground - 200, "UME'S TEA HOUSE: gate-sick half price", 360),
        door("street_dojo_door", "into the dojo", 1736, ground, DOJO, "dojo_door"),
        sign("dojo_sign", 1760, ground - 180, "VILLAGE DOJO: rank exams on the hour", 340),
        sign("notice_board", 2240, ground - 140, "NOTICE: rooftop running permitted after the rice is in", 440),
        edge("street_east_low", "to the forest", w - WALL, ground, FOREST, "forest_west_low"),
        edge("street_east_high", "into the branches", w - WALL, east_roof, FOREST, "forest_west_high"),
    ]
    return build(rid, e, "THE VILLAGE STREET")


#: The forest's branches, west to east: each 64 lower than the one before,
#: the last one 64 over the floor.
BRANCH_W, BRANCH_STEP = 320, 384
FOREST_FLOOR = 832
BRANCHES = [(f"branch_{i}", i * BRANCH_STEP + (WALL if i == 0 else 0), 384 + 64 * i) for i in range(7)]


def forest():
    """The cedar forest. Floor 832. The branches step down east from the
    west edge (y 384) to 64 over the floor: from the floor, a robot climbs
    them west, back to the roofs of the street. The Shadow Dojo's door is
    at the east end."""
    rid = FOREST
    _, (w, h) = ROOMS_AT[rid]
    top = BRANCHES[0][2]
    e = [
        solid("forest_floor", 0, FOREST_FLOOR, w, h - FOREST_FLOOR),
        solid("west_wall_high", 0, 0, WALL, top - 96),
        solid("west_wall_mid", 0, top + 16, WALL, FOREST_FLOOR - 96 - top - 16),
        solid("east_wall", w - WALL, 0, WALL, FOREST_FLOOR - 112),
        edge("forest_east", "on to the bamboo pass", w - WALL, FOREST_FLOOR, PASS, "pass_west"),
        edge("forest_west_low", "to the village", 0, FOREST_FLOOR, STREET, "street_east_low"),
        edge("forest_west_high", "onto the roofs", 0, top, STREET, "street_east_high"),
        start(rid, 120, FOREST_FLOOR),
        *[ledge(name, x, y, BRANCH_W) for name, x, y in BRANCHES],
        door("forest_clan_door", "to the hall of the Shadow Clan", 2700, FOREST_FLOOR, CLAN, CLAN_DOOR),
        sign("clan_sign", 2480, FOREST_FLOOR - 200, "HALL OF THE SHADOW CLAN. Visitors by appointment. Appointments by stealth.", 520),
        sign("forest_sign", 900, FOREST_FLOOR - 120, "Cedar path. Mind the roots.", 260),
    ]
    return build(rid, e, "THE CEDAR FOREST", music=FOREST_MUSIC)


def dojo():
    """The village dojo. Floor 512. The teacher and his top student stand by
    the door; past them, the exam floor: the wall that shuts it while the
    exam is on, the strip that starts it, the lever that sets it again, and
    the training dummies."""
    rid = DOJO
    _, (w, h) = ROOMS_AT[rid]
    floor, ceiling = h - 64, 32
    e = [
        solid("floor", 0, floor, w, 64),
        solid("ceiling", 0, 0, w, ceiling),
        solid("west_wall", 0, ceiling, WALL, floor - ceiling),
        solid("east_wall", w - WALL, ceiling, WALL, floor - ceiling),
        door("dojo_door", "out to the street", 40, floor, STREET, "street_dojo_door"),
        start(rid, 140, floor),
        resident("npc_village_hayashi", "ninja_village_hayashi", 240, floor),
        resident("npc_village_ren", "ninja_village_ren", 360, floor),
        entity("Switch", 440, floor - 32, 24, 32, id="dojo_exam_bell", name="the exam bell", prompt="Ring for the exam again",
               action="ResetEncounter", target_encounter=EXAM),
        entity("LockWall", 496, ceiling, 32, floor - ceiling, id="dojo_exam_wall", name="the exam floor's screen"),
        entity("EncounterTrigger", 640, ceiling, 64, floor - ceiling, id=EXAM, name="the rank exam", camera_zoom=1.2),
        sign("exam_sign", 1000, ceiling + 40, "RANK EXAM, PART ONE: THE DUMMIES. PART TWO: THE EXAMINER.", 560),
        ledge("exam_beam", 1040, floor - 112, 240),
        # Part one: the dummies. Part two: the examiner, at the far end. The
        # encounter has no waves of its own, so these are its one wave.
        *[entity("EnemySpawn", x, floor - 58, 34, 58, name=f"exam dummy {i + 1}", brain="sandbag_finite", character_id="sandbag",
                 respawn="OnRoomReenter") for i, x in enumerate((800, 920, 1040))],
        entity("EnemySpawn", 1420, floor - 66, 32, 66, name="the examiner", brain="combatant",
               character_id="npc_ninja_shadow_duelist_eye_band", respawn="OnRoomReenter"),
    ]
    return build(rid, e, "THE VILLAGE DOJO", music=DOJO_MUSIC)


#: The Bamboo Pass: its gorge (x), the stream in it and its bed (y), the
#: stepping stone out (x0, x1, y), the lookout's platforms (x0, x1, (y...)).
#: The art (`targets/props/ninja_pass_props.py`) is drawn on these numbers.
PASS_GORGE = (900, 1400)
PASS_STREAM, PASS_BED = 880, 944
PASS_STEP = (1340, 1390, 880)
PASS_LOOKOUT = (1540, 1700, (704, 576))


def walk(name, x, y, w):
    """A one-way walk the room's art draws (a bridge, a platform): a still
    platform with no picture of its own."""
    return entity("MovingPlatform", x, y, w, 16, id=name, name=name, speed=0.0, visual="none", one_way=True)


def pass_enemy(n, character, x, feet, w, h):
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{PASS}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def bamboo_pass():
    """Out of the village: the Bamboo Pass, the Shadow Clan's sparring
    ground, where everyone who passes is an opponent. Floor 832, level with
    the forest's. A gorge with a stream (a rope bridge over it, a stone to
    climb out by), a bamboo lookout (its ladder), a waterfall and jizo by
    the path at the east wall."""
    rid = PASS
    _, (w, h) = ROOMS_AT[rid]
    g, (g0, g1) = FOREST_FLOOR, PASS_GORGE
    s0, s1, sy = PASS_STEP
    l0, l1, (low, high) = PASS_LOOKOUT
    e = [
        solid("ground_west", 0, g, g0, h - g),
        solid("gorge_bed", g0, PASS_BED, g1 - g0, h - PASS_BED),
        solid("ground_east", g1, g, w - g1, h - g),
        solid("west_wall", 0, 0, WALL, g - 112),
        solid("east_wall", w - WALL, 0, WALL, g),
        edge("pass_west", "back to the cedar forest", 0, g, FOREST, "forest_east"),
        start(rid, 120, g),
        walk("rope_bridge", g0, g, g1 - g0),
        walk("stepping_stone", s0, sy, s1 - s0),
        walk("lookout_low", l0, low, l1 - l0),
        walk("lookout_high", l0, high, l1 - l0),
        sign("pass_sign", 420, g - 200, "SHADOW CLAN SPARRING GROUND: all who pass are opponents. Nothing personal.", 560),
        sign("bridge_sign", 1150, g - 110, "ROPE BRIDGE: one at a time. Opponents count.", 380),
        sign("falls_sign", 2120, g - 140, "Please do not fight the jizo.", 300),
        # The ninjas who can fight where they are placed (locomotion
        # authored): the swordsmen of the rank exam and their oni leader.
        pass_enemy(1, "npc_ninja_shadow_duelist_eye_band", 640, g, 32, 66),
        pass_enemy(2, "npc_ninja_shadow_duelist_eye_band", 1520, g, 32, 66),
        pass_enemy(3, "npc_ninja_shadow_oni_leader", 1600, high, 34, 71),
        pass_enemy(4, "npc_ninja_shadow_duelist_eye_band", 2060, g, 32, 66),
    ]
    return build(rid, e, "THE BAMBOO PASS", music=FOREST_MUSIC)


ROOMS = (shrine, street, forest, dojo, bamboo_pass)

#: The ladder up the baker's wall: (room, x, top, bottom).
LADDERS = [(STREET, 400, ROOFS[0][3] - 16, 768), (PASS, PASS_LOOKOUT[1] + 4, PASS_LOOKOUT[2][1] - 16, FOREST_FLOOR)]

CLAN_DOOR_YAML = f"""# The Shadow Dojo's door to the Ninja Village's forest. Written by {GENERATOR}.
level_id: {CLAN}
entities:
  - type: LoadingZone
    px: [720, 512]
    size: [48, 96]
    fields:
      id: {CLAN_DOOR}
      name: to the village forest
      activation: Door
      target_room: {FOREST}
      target_zone: forest_clan_door
      bidirectional: true
"""

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
    if code := add_entities(CLAN_DOOR_YAML, WORLD_LDTK, HERE / "clan_door.yaml", unless=f'"{CLAN_DOOR}"'):
        return code
    if GATE_INTERFACE.exists():
        code, _ = run("entity", "add", str(GATE_INTERFACE), "--ldtk", str(WORLD_LDTK), "--in-place")
        if code:
            return code
    for rid, x, top, bottom in LADDERS:
        if code := paint(WORLD_LDTK, rid, "Climbable", 1, x, top, 32, bottom - top):
            return code
    # The pass's stream: murky water (the art draws it).
    if code := paint(WORLD_LDTK, PASS, "Water", 2, PASS_GORGE[0] + 16, PASS_STREAM, PASS_GORGE[1] - PASS_GORGE[0] - 16, PASS_BED - PASS_STREAM):
        return code
    for checked in (WORLD_LDTK, SANDBOX_LDTK):
        code, _ = run("validate", str(checked))
        if code:
            return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
