#!/usr/bin/env python3
"""Write the area specs of the primeval trail: the level of the dinosaurs.

    python specs/primeval/make_specs.py          # writes the .ron specs
    python specs/primeval/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

A linear level of six rooms in a row, in the `primeval` biome. The enemies
are the T-rex's kin (the birds, and the raptors) and the dinosaurs that are
not birds yet; the T-rex is at its end.

    central hub -> trailhead -> run -> rookery -> ridge road -> canopy -> the tyrant's hall

A coconut palm (a `Prop`, scenery) stands by each coconut on the ground.

- `primeval_trailhead`: the small birds (kiwis, a cockerel and his hen), a
  pack of three compsognathus, and a coconut to learn to throw.
- `primeval_run`: a long flat room for the runners (emus, a pair of
  ostriches, a pachycephalosaurus) with swallows over it, a dilophosaurus
  that spits from a ledge, and at its end the first Willson: a pelican on a
  bicycle, who rides at what he sees.
- `primeval_rookery`: the penguins' rocks. Two African swallows by the door
  with a coconut under them: they pick it up, carry it over the player and
  let it go (`ambition_content::primeval`). Penguins in a pair and in a
  huddle, a shoebill on a rock, and a third swallow with its own coconut.
- `primeval_ridge`: a long road. Two Willsons ride it, with a pack of
  compsognathus, a raptor, an emu and a pachycephalosaurus, and a swallow
  over the middle. Ledges over the road are out of a bicycle's reach.
- `primeval_canopy`: a tall room of ledges. Raptors on the floor, a
  cassowary, a shoebill and a dilophosaurus on ledges, an owl and a vulture
  over it, and a triceratops in front of the way out.
- `primeval_tyrant_hall`: the T-rex. The hall is the one of `trex_arena` (the
  fight needs its width, and its two ledges with a step to each); this
  placement has its own encounter id, so its progress is its own.

The rooms are the dinosaur dimension's world file (`dinosaur.ldtk`); the door
to them is in the central hub's developer wing (`sandbox.ldtk`). Each room
after the first has its door back, and puts the door forward in the room
before it (`connect_to`), so the rooms are made in order. A room that is in
the map already is replaced, with the doors that other rooms put in it: make
all six again, in order (`--apply` does). A link places a door only in its own
file, so the trailhead is made without its link to the hub, whose door
(`gate_network/make_gates.py`) stays where it is.

An enemy is an `EnemySpawn` with the `character_id` of a row of the character
catalog; the row's own profile is what makes it hunt. The size of its box is
the size the T-rex's call summons that kind with.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk, default_worlds_dir  # noqa: E402

sys.path.insert(0, str(HERE.parent / "gate_network"))
import gates  # noqa: E402

#: The dinosaur dimension's world file: the rooms of the trail.
LDTK = default_worlds_dir() / "dinosaur.ldtk"
#: The central hub's: the door to the trail.
HUB_LDTK = default_sandbox_ldtk()

BIOME = "primeval"
MUSIC = "dinosaur_liberators"
BG = "#16200F"
WALL = 16

#: The box of each kind of enemy (its whole size): what its body is built in.
BOX = {
    "kiwi": (24, 26), "chicken": (32, 44), "chicken_hen": (32, 44), "emu": (36, 84), "ostrich": (40, 104),
    "ostrich_hen": (40, 104), "cassowary": (40, 82), "shoebill": (36, 80), "npc_raptor_stalker": (48, 80),
    "stochastic_parrot": (36, 32), "owl": (36, 48), "european_swallow": (24, 20), "african_swallow": (24, 20),
    "vulture": (44, 66), "compsognathus": (24, 26), "dilophosaurus": (44, 74), "pachycephalosaurus": (44, 72),
    "triceratops": (96, 92), "penguin": (28, 40), "willson": (76, 88),
}

#: Each room: where it is in the world (a multiple of 16), and its size. The
#: rooms are a row in this order; a room's door forward is in its east wall.
#: A room that is made again where another room of the map still is, is
#: refused: the rooms after the trailhead are past where the first four were
#: (20000 to 29792), so a map with the old rooms can be made into this one.
ROOMS_AT = {
    "primeval_trailhead": (20000, 1600, 512),
    "primeval_run": (30000, 3200, 512),
    "primeval_rookery": (34000, 2608, 512),
    "primeval_ridge": (37008, 3408, 512),
    "primeval_canopy": (41008, 2000, 704),
    "primeval_tyrant_hall": (44000, 1792, 704),
}


def solid(name, x, y, w, h):
    return f'( type: "Solid", px: [{x}, {y}], size: [{w}, {h}], fields: ( name: "{name}" ) ),'


def ledge(name, x, y, w):
    return f'( type: "OneWayPlatform", px: [{x}, {y}], size: [{w}, 16], fields: ( name: "{name}" ) ),'


def enemy(room, n, character, x, feet_y):
    """An enemy whose feet are at `feet_y` (a flyer: where the foot of its box is)."""
    w, h = BOX[character]
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{room}_{character}_{n}", '
            f'brain: "combatant", character_id: "{character}", respawn: "OnRoomReenter" ) ),')


def coconut(room, n, x, floor_y):
    """A coconut on the floor: its box is the drawn nut (14 x 11), centred
    where an 18-wide box at `x` would be."""
    return (f'( type: "GroundItem", px: [{x + 2}, {floor_y - 11}], size: [14, 11], fields: ( id: "{room}_coconut_{n}", '
            f'name: "coconut", held_item: "coconut" ) ),')


def palm(room, n, x, floor_y):
    """A coconut palm that stands on the floor at `x` (scenery: no collision)."""
    return (f'( type: "Prop", px: [{x}, {floor_y - 200}], size: [176, 200], fields: ( name: "{room}_palm_{n}", '
            f'kind: "coconut_palm" ) ),')


def door(zone, name, x, floor_y, to_room, to_zone):
    return (f'( type: "LoadingZone", px: [{x}, {floor_y - 96}], size: [48, 96], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def forward_door(in_level, name, to_zone):
    """The door forward, put in the east wall of the room before this one
    (`in_level`). `name` is what the player reads at the door: prose, not the
    id of the zone."""
    _, w, h = ROOMS_AT[in_level]
    return (f'( target_room: "{in_level}", px: [{w - WALL - 48}, {h - 32 - 96}], size: [48, 96], id: "{in_level}_east", name: "{name}", '
            f'target_zone: "{to_zone}", activation: "Door", bidirectional: true ),')


def shell(w, h):
    floor = h - 32
    return floor, [
        solid("floor", 0, floor, w, 32),
        solid("ceiling", 0, 0, w, WALL),
        solid("left_wall", 0, WALL, WALL, floor - WALL),
        solid("right_wall", w - WALL, WALL, WALL, floor - WALL),
    ]


#: The trail rooms that are out of town: an NPC on the player's side joins the
#: player's fights there (`RoomMetadata::dangerous`). The Tyrant King's hall
#: is a boss arena.
DANGEROUS = {"primeval_trailhead", "primeval_run", "primeval_rookery", "primeval_ridge", "primeval_canopy"}


def room(rid, entities, connect, label):
    world_x, w, h = ROOMS_AT[rid]
    danger = "\n    dangerous: true," if rid in DANGEROUS else ""
    body = "\n        ".join(entities + [
        f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),',
        f'( type: "DebugLabel", px: [70, 40], size: [900, 24], fields: ( name: "{rid}_label", text: "{label}", category: "Custom" ) ),',
    ])
    links = "\n        ".join(connect)
    return f"""// Written by specs/primeval/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: 0,
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{BG}",
    biome: "{BIOME}",
    parallax_theme: "{BIOME}",
    music_track: "{MUSIC}",{danger}
    entities: [
        {body}
    ],
    connect_to: [
        {links}
    ],
)
"""


def trailhead():
    rid = "primeval_trailhead"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_to_hub", "to the central hub", WALL, floor, "central_hub_complex", "primeval_hub_door"),
        # The trailhead's ring (GENESIS, `data/gate_network.ron`; round, the
        # Jungle's shape, `gates.RING_SHAPES`) and its
        # terminal: the public way in and the way home.
        *gates.as_ron(gates.ring("Primeval Gate (Trailhead)", "primeval_trailhead_gate", 80, floor, "the gate", shape="round")),
        *gates.as_ron(gates.terminal("Primeval Gate (Trailhead)", "primeval_gate_terminal", 380, floor)),
        palm(rid, 1, 400, floor),
        coconut(rid, 1, 470, floor),
        ledge("perch_a", 620, floor - 80, 160),
        palm(rid, 2, 1300, floor),
        enemy(rid, 1, "kiwi", 560, floor),
        enemy(rid, 2, "kiwi", 820, floor),
        coconut(rid, 2, 690, floor - 80),
        ledge("perch_b", 1040, floor - 96, 192),
        enemy(rid, 1, "chicken", 1100, floor),
        enemy(rid, 1, "chicken_hen", 1260, floor),
        # A pack: one is a nuisance, three are a problem.
        enemy(rid, 1, "compsognathus", 1390, floor),
        enemy(rid, 2, "compsognathus", 1440, floor),
        enemy(rid, 3, "compsognathus", 1490, floor),
    ]
    connect = [(f'( target_room: "{HUB_LEVEL}", px: [{HUB_DOOR[0]}, {HUB_DOOR[1]}], size: [48, 96], id: "primeval_hub_door", name: "{HUB_DOOR_NAME}", '
                f'target_zone: "{rid}_to_hub", activation: "Door", bidirectional: true ),')]
    return rid, room(rid, e, connect, "The primeval trail. A coconut: Attack picks it up, Attack throws it.")


def run():
    rid = "primeval_run"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "back to the trailhead", WALL, floor, "primeval_trailhead", "primeval_trailhead_east"),
        palm(rid, 1, 170, floor),
        coconut(rid, 1, 260, floor),
        palm(rid, 2, 1180, floor),
        palm(rid, 3, 2120, floor),
        enemy(rid, 1, "emu", 640, floor),
        enemy(rid, 2, "emu", 860, floor),
        ledge("rest_a", 1000, floor - 112, 176),
        enemy(rid, 1, "european_swallow", 1040, floor - 250),
        enemy(rid, 2, "european_swallow", 1240, floor - 290),
        coconut(rid, 2, 1080, floor - 112),
        ledge("rest_b", 1420, floor - 144, 160),
        enemy(rid, 1, "pachycephalosaurus", 1380, floor),
        enemy(rid, 1, "ostrich", 1640, floor),
        enemy(rid, 1, "ostrich_hen", 1900, floor),
        ledge("rest_c", 2040, floor - 112, 176),
        # It spits down the stretch from the top of the ledge.
        enemy(rid, 1, "dilophosaurus", 2110, floor - 112),
        # The last stretch is his: a pelican on a bicycle, and a ledge to be
        # on when he comes.
        ledge("rest_d", 2560, floor - 112, 208),
        enemy(rid, 1, "willson", 2900, floor),
        palm(rid, 4, 2700, floor),
    ]
    connect = [forward_door("primeval_trailhead", "to the run", f"{rid}_west")]
    return rid, room(rid, e, connect, "The run. The tall birds are fast: a ledge is out of the reach of a kick.")


def rookery():
    rid = "primeval_rookery"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "back to the run", WALL, floor, "primeval_run", "primeval_run_east"),
        # An old survey ring by the door: a hidden address (NEST) that only
        # receives. The way out is the trail, down to the trailhead's ring.
        *gates.as_ron(gates.ring("Rookery Survey Gate", "primeval_rookery_gate", 80, floor, "the old survey ring", shape="round")),
        # Two swallows and one coconut by the door: the first thing the room
        # shows is a bird that picks it up.
        palm(rid, 1, 420, floor),
        coconut(rid, 1, 520, floor),
        enemy(rid, 1, "african_swallow", 430, floor - 210),
        enemy(rid, 2, "african_swallow", 640, floor - 250),
        ledge("shelf_a", 560, floor - 104, 176),
        # The rocks: steps of stone that a penguin does not climb.
        solid("rock_a", 820, floor - 48, 224, 48),
        enemy(rid, 1, "penguin", 1080, floor),
        enemy(rid, 2, "penguin", 1150, floor),
        ledge("shelf_b", 1220, floor - 120, 192),
        enemy(rid, 1, "shoebill", 1290, floor - 120),
        solid("rock_b", 1480, floor - 64, 192, 64),
        palm(rid, 2, 1700, floor),
        coconut(rid, 2, 1800, floor),
        enemy(rid, 3, "african_swallow", 1860, floor - 240),
        ledge("shelf_c", 1900, floor - 112, 176),
        # The huddle, in front of the way out.
        enemy(rid, 3, "penguin", 2140, floor),
        enemy(rid, 4, "penguin", 2210, floor),
        enemy(rid, 5, "penguin", 2280, floor),
    ]
    connect = [forward_door("primeval_run", "to the rookery", f"{rid}_west")]
    return rid, room(rid, e, connect, "The rookery. A swallow with a coconut is slow. Look up.")


def ridge():
    rid = "primeval_ridge"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "back to the rookery", WALL, floor, "primeval_rookery", "primeval_rookery_east"),
        palm(rid, 1, 190, floor),
        coconut(rid, 1, 290, floor),
        ledge("over_a", 520, floor - 112, 208),
        enemy(rid, 1, "compsognathus", 700, floor),
        enemy(rid, 2, "compsognathus", 750, floor),
        enemy(rid, 3, "compsognathus", 800, floor),
        # The road is flat from end to end: it is his.
        enemy(rid, 1, "willson", 1120, floor),
        ledge("over_b", 1260, floor - 128, 224),
        coconut(rid, 2, 1340, floor - 128),
        enemy(rid, 1, "african_swallow", 1560, floor - 260),
        palm(rid, 2, 1620, floor),
        enemy(rid, 1, "emu", 1840, floor),
        ledge("over_c", 2020, floor - 112, 208),
        enemy(rid, 1, "npc_raptor_stalker", 2260, floor),
        ledge("over_d", 2520, floor - 136, 224),
        enemy(rid, 2, "willson", 2760, floor),
        palm(rid, 3, 2980, floor),
        enemy(rid, 1, "pachycephalosaurus", 3140, floor),
    ]
    connect = [forward_door("primeval_rookery", "to the ridge road", f"{rid}_west")]
    return rid, room(rid, e, connect, "The ridge road. It is flat from end to end, and it is his.")


def canopy():
    rid = "primeval_canopy"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "back to the ridge road", WALL, floor, "primeval_ridge", "primeval_ridge_east"),
        palm(rid, 1, 150, floor),
        coconut(rid, 1, 240, floor),
        palm(rid, 2, 1090, floor),
        ledge("branch_a", 320, floor - 112, 176),
        ledge("branch_b", 560, floor - 224, 192),
        enemy(rid, 1, "owl", 600, floor - 400),
        enemy(rid, 1, "npc_raptor_stalker", 760, floor),
        ledge("branch_c", 880, floor - 144, 240),
        enemy(rid, 1, "shoebill", 960, floor - 144),
        enemy(rid, 1, "african_swallow", 1120, floor - 330),
        enemy(rid, 2, "african_swallow", 1280, floor - 360),
        coconut(rid, 2, 1180, floor),
        ledge("branch_d", 1240, floor - 256, 192),
        enemy(rid, 1, "dilophosaurus", 1330, floor - 256),
        enemy(rid, 2, "npc_raptor_stalker", 1200, floor),
        enemy(rid, 1, "cassowary", 1480, floor),
        ledge("branch_e", 1560, floor - 128, 176),
        enemy(rid, 1, "vulture", 1620, floor - 420),
        # The heavy stands in front of the way out.
        enemy(rid, 1, "triceratops", 1790, floor),
    ]
    connect = [forward_door("primeval_ridge", "to the canopy", f"{rid}_west")]
    return rid, room(rid, e, connect, "The canopy. Something on that ledge has not moved. It is looking at you.")


def tyrant_hall():
    rid = "primeval_tyrant_hall"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        # The hall of `trex_arena`: a ledge by each wall out of the reach of
        # his lowered head, and a step up to each.
        ledge("ledge_left", 16, 464, 192),
        ledge("step_left", 240, 560, 128),
        ledge("step_right", 1424, 560, 128),
        ledge("ledge_right", 1584, 464, 192),
        f'( type: "PlayerStart", px: [112, {floor - 48}], size: [24, 48], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_entry", "back to the canopy", 40, floor, "primeval_canopy", "primeval_canopy_east"),
        (f'( type: "BossSpawn", px: [1120, {floor - 112}], size: [140, 112], fields: ( name: "T-Rex", '
         f'brain: "PhaseScript:trex_boss", encounter_id: "primeval.trex" ) ),'),
    ]
    connect = [forward_door("primeval_canopy", "to the tyrant's hall", f"{rid}_entry")]
    return rid, room(rid, e, connect, "The Tyrant King.")


#: Where the door of the level is in the central hub: the hub's developer
#: wing (`specs/gate_network/make_gates.py`); the public way in is the
#: trailhead's ring.
HUB_LEVEL = "central_hub_dev_wing"
HUB_DOOR = (880, 368)
#: What the player reads at that door.
HUB_DOOR_NAME = "to the primeval trail"

ROOMS = (trailhead, run, rookery, ridge, canopy, tyrant_hall)


def hub_door_is_placed():
    """The central hub has the door of this level already."""
    world = json.loads(Path(HUB_LDTK).read_text())
    hub = next((level for level in world["levels"] if level["identifier"] == HUB_LEVEL), None)
    return hub is not None and "primeval_hub_door" in json.dumps(hub)


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}_area.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    if "--apply" in argv:
        if not hub_door_is_placed():
            print(f"the hub has no door to the trail ({HUB_LEVEL} in {HUB_LDTK}): make it first (gate_network/make_gates.py)")
            return 2
        # The name of the door (`HUB_DOOR_NAME`).
        subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", "entity", "set-field", str(HERE / "hub_door_name.yaml"),
                        "--ldtk", str(HUB_LDTK), "--in-place"], cwd=TOOLS, check=True)
        for path in made:
            spec = path
            if path == made[0]:
                # The hub is in another file, and its door is there already:
                # make the trailhead without the link that would place it.
                spec = Path(tempfile.mkdtemp()) / path.name
                spec.write_text(re.sub(r"connect_to: \[.*?\n    \],", "connect_to: [],", path.read_text(), flags=re.S))
            done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", "area", "create", str(spec), "--ldtk",
                                   str(LDTK), "--replace-existing"], cwd=TOOLS, capture_output=True, text=True)
            print(done.stdout + done.stderr)
            # A room made again loses the door that the next room puts in it,
            # so the map is not whole until the last room is made: before
            # that, it is enough that the room was written.
            if done.returncode != 0 and (path == made[-1] or "wrote new level" not in done.stdout + done.stderr):
                return done.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
