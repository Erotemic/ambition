#!/usr/bin/env python3
"""Write the area specs of the primeval trail: the level of the dinosaurs.

    python specs/primeval/make_specs.py          # writes the .ron specs
    python specs/primeval/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

A linear level of four rooms in a row, in the `primeval` biome. The enemies
are the T-rex's kin (the birds, and the raptors) and the dinosaurs that are
not birds yet; the T-rex is at its end.

    central hub -> trailhead -> run -> canopy -> the tyrant's hall

A coconut palm (a `Prop`, scenery) stands by each coconut on the ground.

- `primeval_trailhead`: the small birds (kiwis, a cockerel and his hen), a
  pack of three compsognathus, and a coconut to learn to throw.
- `primeval_run`: a long flat room for the runners (emus, a pair of
  ostriches, a pachycephalosaurus) with swallows over it, and a
  dilophosaurus that spits from the last ledge.
- `primeval_canopy`: a tall room of ledges. Raptors on the floor, a
  cassowary, a shoebill and a dilophosaurus on ledges, an owl and a vulture
  over it, and a triceratops in front of the way out.
- `primeval_tyrant_hall`: the T-rex. The hall is the one of `trex_arena` (the
  fight needs its width, and its two ledges with a step to each); this
  placement has its own encounter id, so its progress is its own.

Each room after the first has its door back, and puts the door forward in the
room before it (`connect_to`), so the rooms are made in order. A room that is
in the map already is replaced, with the doors that other rooms put in it:
make all four again, in order (`--apply` does). The central hub is not
replaced, so its door stays: a second `--apply` makes the trailhead without
the link that placed it.

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

from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk  # noqa: E402

LDTK = default_sandbox_ldtk()

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
    "triceratops": (96, 92),
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
    return (f'( type: "GroundItem", px: [{x}, {floor_y - 15}], size: [18, 15], fields: ( id: "{room}_coconut_{n}", '
            f'name: "coconut", held_item: "coconut" ) ),')


def palm(room, n, x, floor_y):
    """A coconut palm that stands on the floor at `x` (scenery: no collision)."""
    return (f'( type: "Prop", px: [{x}, {floor_y - 200}], size: [176, 200], fields: ( name: "{room}_palm_{n}", '
            f'kind: "coconut_palm" ) ),')


def door(zone, name, x, floor_y, to_room, to_zone):
    return (f'( type: "LoadingZone", px: [{x}, {floor_y - 96}], size: [48, 96], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def forward_door(in_level, zone, name, x, floor_y, to_zone):
    """The door forward, put in the room before this one. `name` is what
    the player reads at the door: prose, not the id of the zone."""
    return (f'( target_room: "{in_level}", px: [{x}, {floor_y - 96}], size: [48, 96], id: "{zone}", name: "{name}", '
            f'target_zone: "{to_zone}", activation: "Door", bidirectional: true ),')


def shell(w, h):
    floor = h - 32
    return floor, [
        solid("floor", 0, floor, w, 32),
        solid("ceiling", 0, 0, w, WALL),
        solid("left_wall", 0, WALL, WALL, floor - WALL),
        solid("right_wall", w - WALL, WALL, WALL, floor - WALL),
    ]


def room(rid, world_x, w, h, entities, connect, label):
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
    music_track: "{MUSIC}",
    entities: [
        {body}
    ],
    connect_to: [
        {links}
    ],
)
"""


def trailhead():
    rid, w, h = "primeval_trailhead", 1600, 512
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_to_hub", "to the central hub", WALL, floor, "central_hub_complex", "primeval_hub_door"),
        palm(rid, 1, 210, floor),
        coconut(rid, 1, 300, floor),
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
    connect = [(f'( target_room: "central_hub_main", px: [{HUB_DOOR[0]}, {HUB_DOOR[1]}], size: [48, 96], id: "primeval_hub_door", name: "{HUB_DOOR_NAME}", '
                f'target_zone: "{rid}_to_hub", activation: "Door", bidirectional: true ),')]
    return rid, room(rid, 20000, w, h, e, connect, "The primeval trail. A coconut: Attack picks it up, Attack throws it.")


def run():
    rid, w, h = "primeval_run", 2400, 512
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
        # It spits down the last stretch from the top of the ledge.
        enemy(rid, 1, "dilophosaurus", 2110, floor - 112),
    ]
    connect = [forward_door("primeval_trailhead", "primeval_trailhead_east", "to the run", 1600 - WALL - 48, 480, f"{rid}_west")]
    return rid, room(rid, 22000, w, h, e, connect, "The run. The tall birds are fast: a ledge is out of the reach of a kick.")


def canopy():
    rid, w, h = "primeval_canopy", 2000, 704
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [90, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "back to the run", WALL, floor, "primeval_run", "primeval_run_east"),
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
    connect = [forward_door("primeval_run", "primeval_run_east", "to the canopy", 2400 - WALL - 48, 480, f"{rid}_west")]
    return rid, room(rid, 25000, w, h, e, connect, "The canopy. Something on that ledge has not moved. It is looking at you.")


def tyrant_hall():
    rid, w, h = "primeval_tyrant_hall", 1792, 704
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
    connect = [forward_door("primeval_canopy", "primeval_canopy_east", "to the tyrant's hall", 2000 - WALL - 48, 672, f"{rid}_entry")]
    return rid, room(rid, 28000, w, h, e, connect, "The Tyrant King.")


#: Where the door of the level is in the central hub (`door free-spots central_hub_main`).
HUB_DOOR = (1200, 880)
#: What the player reads at that door.
HUB_DOOR_NAME = "to the primeval trail"

ROOMS = (trailhead, run, canopy, tyrant_hall)


def hub_door_is_placed():
    """The central hub has the door of this level already."""
    world = json.loads(Path(LDTK).read_text())
    hub = next(level for level in world["levels"] if level["identifier"] == "central_hub_main")
    return "primeval_hub_door" in json.dumps(hub)


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}_area.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    if "--apply" in argv:
        if hub_door_is_placed():
            # The name of a door that is there already (`HUB_DOOR_NAME`).
            subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", "entity", "set-field", str(HERE / "hub_door_name.yaml"),
                            "--ldtk", str(LDTK), "--in-place"], cwd=TOOLS, check=True)
        for path in made:
            spec = path
            if path == made[0] and hub_door_is_placed():
                # The hub is not replaced, so its door is there already: make
                # the trailhead again without the link that would place it.
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
