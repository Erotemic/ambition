#!/usr/bin/env python3
"""Write the rooms of the Math dimension's town, and the gate that leads to it.

    python specs/math_town/make_specs.py          # writes the specs
    python specs/math_town/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The Math dimension is its own world file, `math.ldtk`: one world file is one
dimension. Its town is the hub of the open world (the crowned jewel); it lives
on a torus glued to the rest of the dimension, and the wrap is a later
discovery (docs/storylines/cannon_expansions.md, D5-D7). These are its first
rooms:

- `math_gate_hall`: the gate square. Travellers to and from the dimension
  arrive here, by the gate on the square; colonnade galleries on each side
  and a walk over the gate. The town's street is through its east edge.
- `math_town_street`: the street at the foot of the terraces. Ledges climb
  the cliff to the first terrace and on to the second.

And in the Home dimension, in `gate_stack_lower` (`intro.ldtk`, the gate
station by the creator's lab), Gate 7: a second gate ring whose portal goes
to the gate square. Both gates follow one switch, `math_gate_switch`, by the
Home gate: switched on there, both are open (the gate pair is one link).
The engine side of the pair (`GatePortalRegistry` rows) is in
`game/ambition_content/src/math_town.rs`; its ids are the ones here.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

MATH_LDTK = default_worlds_dir() / "math.ldtk"
INTRO_LDTK = default_worlds_dir() / "intro.ldtk"

BIOME = "math_town"
MUSIC = "standing_on_shoulders"
BG = "#BFD7EE"
WALL = 16

#: The ids the engine side names (`ambition_content::math_town`).
SWITCH = "math_gate_switch"
HOME_ZONE = "math_gate_home_zone"
MATH_ZONE = "math_gate_return_zone"
HOME_RING, HOME_PORTAL = "Math Gate Ring (Home)", "Math Gate Portal (Home)"
MATH_RING, MATH_PORTAL = "Math Gate Ring (Math)", "Math Gate Portal (Math)"
HALL, STREET = "math_gate_hall", "math_town_street"

#: Each room: where it is in the world, and its size.
ROOMS_AT = {
    HALL: (0, 1600, 896),
    STREET: (1712, 2400, 1104),
}


def solid(name, x, y, w, h):
    return f'( type: "Solid", px: [{x}, {y}], size: [{w}, {h}], fields: ( name: "{name}" ) ),'


def ledge(name, x, y, w):
    return f'( type: "OneWayPlatform", px: [{x}, {y}], size: [{w}, 16], fields: ( name: "{name}" ) ),'


def label(name, x, y, text, w=420):
    return f'( type: "DebugLabel", px: [{x}, {y}], size: [{w}, 24], fields: ( name: "{name}", text: "{text}", category: "Custom" ) ),'


def edge(zone, name, x, y, h, to_room, to_zone):
    return (f'( type: "LoadingZone", px: [{x}, {y}], size: [16, {h}], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "EdgeExit", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def gate(ring, portal, zone, x, floor, to_room, to_zone, name):
    """A gate ring standing on the floor at `x` (its left edge), its portal
    in it, and the door of the portal: a Door the size of the opening."""
    top = floor - 192
    return [
        f'( type: "Prop", px: [{x}, {top}], size: [192, 192], fields: ( name: "{ring}", kind: "gate_ring" ) ),',
        f'( type: "Prop", px: [{x}, {top}], size: [192, 192], fields: ( name: "{portal}", kind: "gate_portal" ) ),',
        (f'( type: "LoadingZone", px: [{x + 48}, {top}], size: [96, 192], fields: ( id: "{zone}", name: "{name}", '
         f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: false ) ),'),
    ]


def room(rid, entities, label_text):
    world_x, w, h = ROOMS_AT[rid]
    body = "\n        ".join(entities + [
        f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),',
        label(f"{rid}_label", 70, 40, label_text, 900),
    ])
    return f"""// Written by specs/math_town/make_specs.py. Change the script, not this file.
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
    music_track: "{MUSIC}",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


def gate_hall():
    """The gate square: the gate in the middle on the floor, a colonnade
    gallery each side, steps up to them, and a walk over the gate."""
    rid = HALL
    _, w, h = ROOMS_AT[rid]
    floor = h - 64
    gx = w // 2 - 96
    e = [
        solid("floor", 0, floor, w, 64),
        solid("west_wall", 0, 0, WALL, floor),
        solid("east_wall_upper", w - WALL, 0, WALL, floor - 240),
        f'( type: "PlayerStart", px: [{gx - 120}, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        *gate(MATH_RING, MATH_PORTAL, MATH_ZONE, gx, floor, "gate_stack_lower", HOME_ZONE, "to the Home dimension"),
        # Steps up to the galleries, and the galleries.
        ledge("west_step", 96, floor - 96, 128),
        ledge("west_gallery", 176, floor - 192, 400),
        ledge("east_step", w - 224, floor - 96, 128),
        ledge("east_gallery", w - 576, floor - 192, 400),
        # The walk over the gate, from gallery to gallery.
        ledge("gate_walk", gx - 160, floor - 336, 512),
        ledge("west_upper", 320, floor - 448, 192),
        ledge("east_upper", w - 512, floor - 448, 192),
        edge(f"{rid}_east", "to the town", w - WALL, floor - 240, 240, STREET, f"{STREET}_west"),
        label("hall_sign_gate", gx - 40, floor - 236, "GATE OF THE MATH DIMENSION", 300),
        label("hall_sign_axioms", 200, floor - 260, "Arrivals: please declare all axioms", 380),
        label("hall_sign_infinity", w - 580, floor - 260, "Infinity is not a valid destination", 380),
        label("hall_sign_manifest", 40, floor - 40, "<- Manifest Office (closed for recounting)", 420),
    ]
    return rid, room(rid, e, "The gate square of the Math dimension.")


def street():
    """The street at the foot of the terraces. Ledges climb the cliff: a
    switchback to the first terrace, then on to the second."""
    rid = STREET
    _, w, h = ROOMS_AT[rid]
    floor = h - 64
    t1, t2 = floor - 240, floor - 480
    c1, c2 = 1504, 1904
    e = [
        solid("street", 0, floor, c1, 64),
        solid("terrace_1", c1, t1, w - c1, h - t1),
        solid("terrace_2", c2, t2, w - c2, t1 - t2),
        solid("east_wall", w - WALL, 0, WALL, t2),
        f'( type: "PlayerStart", px: [160, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        edge(f"{rid}_west", "to the gate square", 0, floor - 240, 240, HALL, f"{HALL}_east"),
        # The switchback up the face of the first cliff.
        ledge("stair_1a", c1 - 128, floor - 80, 128),
        ledge("stair_1b", c1 - 288, floor - 160, 144),
        ledge("stair_1c", c1 - 128, t1, 128),
        # And up the second.
        ledge("stair_2a", c2 - 128, t1 - 80, 128),
        ledge("stair_2b", c2 - 288, t1 - 160, 144),
        ledge("stair_2c", c2 - 128, t2, 128),
        # Balconies and a roof line over the street: a way along above it.
        ledge("balcony_a", 304, floor - 144, 128),
        ledge("balcony_b", 560, floor - 224, 144),
        ledge("roofline", 768, floor - 304, 256),
        label("street_sign", 180, floor - 300, "Euclid Street (the long way round)", 380),
    ]
    return rid, room(rid, e, "The town of the Math dimension: the street under the terraces.")


ROOMS = (gate_hall, street)


def home_gate_yaml() -> str:
    """Gate 7 in `gate_stack_lower`: the ring, its portal, its door and the
    switch, on the floor east of the old gate."""
    floor = 736
    x = 1150
    top = floor - 192
    return f"""# Written by specs/math_town/make_specs.py. Change the script, not this file.
# Gate 7 of the gate station: the gate to the Math dimension.
level_id: gate_stack_lower
entities:
  - type: Prop
    px: [{x}, {top}]
    size: [192, 192]
    fields: {{name: "{HOME_RING}", kind: gate_ring}}
  - type: Prop
    px: [{x}, {top}]
    size: [192, 192]
    fields: {{name: "{HOME_PORTAL}", kind: gate_portal}}
  - type: LoadingZone
    px: [{x + 48}, {top}]
    size: [96, 192]
    fields:
      id: {HOME_ZONE}
      name: to the Math dimension
      activation: Door
      target_room: {HALL}
      target_zone: {MATH_ZONE}
      bidirectional: false
  - type: Switch
    px: [{x + 210}, {floor - 32}]
    size: [24, 32]
    fields:
      id: {SWITCH}
      name: {SWITCH}
      action: ResetEncounter
      target_encounter: ""
  - type: DebugLabel
    px: [{x - 20}, {top - 32}]
    size: [260, 24]
    fields:
      name: gate_stack_sign_gate_seven
      text: "GATE 7: MATH (see the switch)"
      category: Custom
"""


MANIFEST_LABEL_DELETE = """# Written by specs/math_town/make_specs.py. Change the script, not this file.
# The old "Manifest Office (TBD)" sign stood where Gate 7 stands; the Manifest
# Office is in the Math dimension now, on the gate square.
level_id: gate_stack_lower
deletes:
  - target: {identifier: DebugLabel, match: {name: gate_stack_manifest_label}}
"""


def home_gate_is_placed() -> bool:
    return HOME_ZONE in INTRO_LDTK.read_text()


def run(*args) -> tuple[int, str]:
    done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", *args], cwd=TOOLS, capture_output=True, text=True)
    print(done.stdout + done.stderr)
    return done.returncode, done.stdout + done.stderr


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}_area.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    gate_path = HERE / "home_gate.yaml"
    gate_path.write_text(home_gate_yaml())
    print(gate_path.relative_to(TOOLS))
    if "--apply" in argv:
        # Each room's doors name the other rooms and the Home gate, so the
        # worlds are not whole until all are made: until then it is enough
        # that each room was written. The last command's check is the one
        # that must pass.
        for path in made:
            code, out = run("area", "create", str(path), "--ldtk", str(MATH_LDTK), "--replace-existing")
            if code != 0 and "wrote new level" not in out and "replaced level" not in out:
                return code
        if not home_gate_is_placed():
            if "gate_stack_manifest_label" in INTRO_LDTK.read_text():
                delete = HERE / "home_gate_manifest_sign_delete.yaml"
                delete.write_text(MANIFEST_LABEL_DELETE)
                code, _ = run("entity", "delete", str(delete), "--ldtk", str(INTRO_LDTK), "--in-place")
                if code != 0:
                    return code
            code, _ = run("entity", "add", str(gate_path), "--ldtk", str(INTRO_LDTK), "--in-place")
            if code != 0:
                return code
        for world in (MATH_LDTK, INTRO_LDTK):
            code, _ = run("validate", str(world))
            if code != 0:
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
