#!/usr/bin/env python3
"""Put the public gate in the central hub, and the old door routes behind it.

    python specs/gate_network/make_gates.py          # writes the specs
    python specs/gate_network/make_gates.py --apply  # and changes the worlds

Run it from `tools/ambition_ldtk_tools`. Change the script, not the specs or
the worlds. Each step looks at the world first, so a second `--apply` changes
nothing.

What it does (`docs/planning/game/gate-network.md`):

- The central hub (`central_hub_main`, `sandbox.ldtk`) gets ONE public gate:
  a dialed ring (`hub_public_gate_zone`, address LEGAL) and its terminal
  (`hub_public_terminal`), on the main floor beside the door to the gate
  stack. The hub's old gate on the shelf (to the Evil dimension) goes: the
  public gate dials CITADEL.
- The doors that went from the hub's main floor to test rooms and to other
  dimensions move into the DEVELOPER WING (`central_hub_dev_wing`), a new
  level of the same room (`central_hub_complex`), east of the main floor
  through the old east exit. Each door keeps its id, name and target, so each
  far door still leads back, and a test that walks to a door in the hub still
  finds it in the hub. The east exit to the scroll lab moves to the wing's
  east edge.
- The wing is east and low, so the room's top left corner (the origin of the
  room's coordinates) does not move: every hub position stays where it was.
- `overfit_annex_entry` (another room) moves its place in the world file out
  of the wing's way. Its rooms and doors do not change.
- The intro's gate (`intro_portal_zone`, `gate_stack_lower`) comes out of the
  hub's door to the gate stack, beside the public gate, so the first walk
  from the intro leads to the gate.
- Ring 7 of the gate stack (`math_gate_home_zone`) becomes a dialed ring with
  its own terminal; its old switch is now its power breaker.
- The military gate annex (`military_gate_annex`, its own room and facility)
  opens off the ground floor of the General's tower. A checkpoint wall stands
  between its door and its ring (SENTINEL) until the save holds the military
  clearance. The fair early way past is the maintenance crawlway under the
  checkpoint: the wall stops short of the floor, and the gap is low enough
  that only a crouching body goes through.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
sys.path.insert(0, str(HERE))

import gates  # noqa: E402
from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

SANDBOX = default_worlds_dir() / "sandbox.ldtk"
INTRO = default_worlds_dir() / "intro.ldtk"

HUB = "central_hub_main"
WING = "central_hub_dev_wing"
AREA = "central_hub_complex"
FLOOR = 976

#: The wing: east of the hub's main floor (whose collision ends at 1904), its
#: floor level with the main floor. The main floor's east wall is open from
#: y 768 to its floor, and that opening is the way in.
WING_W, WING_H = 1152, 512
WING_X, WING_Y = 1904, FLOOR + 48 - WING_H
WING_FLOOR = FLOOR - WING_Y

#: Where `overfit_annex_entry` moves, out of the wing's place.
OVERFIT_ENTRY_TO = (3328, 1280)
#: `pulse_voyager_outpost` stays where it was authored (a first draft of the
#: wing, west of the hub, moved it).
PULSE_VOYAGER_AT = (-1024, 768)

#: The doors that leave the main floor for the wing, as they are authored
#: (id, name, target_room, target_zone, bidirectional). Test rooms, and the
#: doors to other dimensions that the public gate replaces.
DEV_DOORS = [
    ("hall_of_bosses_door", "to hall of bosses", "hall_of_bosses", "hall_entry", True),
    ("hall_of_characters_door", "to hall of characters", "hall_of_characters", "hall_of_characters_entry", True),
    ("hall_of_biomes_door", "to hall of biomes", "hall_of_biomes", "hall_of_biomes_entry", True),
    ("gravity_lab_door", "to gravity lab", "gravity_lab", "gravity_lab_entry", True),
    ("gradient_ascent_door", "to gradient ascent", "gradient_ascent", "gradient_ascent_entry", True),
    ("skirmish_heights_door", "to skirmish heights", "skirmish_heights", "skirmish_heights_entry", True),
    ("proving_grounds_hub_door", "to proving grounds", "proving_grounds", "proving_grounds_to_hub", True),
    ("overfit_annex_hub_door", "to overfit annex", "overfit_annex_entry", "overfit_annex_entry_to_hub", True),
    ("volatile_cache_door", "to volatile cache", "volatile_cache", "volatile_cache_entry", True),
    ("falling_sand_main_door", "to falling sand", "falling_sand_room", "falling_sand_entry", True),
    ("pirate_cove_door", "to pirate cove", "pirate_cove", "pirate_cove_entry", True),
    ("ninja_dojo_door", "to ninja dojo", "ninja_dojo", "ninja_dojo_entry", True),
    ("primeval_hub_door", "to the primeval trail", "primeval_trailhead", "primeval_trailhead_to_hub", True),
    ("noodly_hub_door", "to the Noodly Temple", "noodly_narthex", "noodly_narthex_to_hub", True),
    ("gnu_ton_handle_hub_door", "to GNU-ton gravity handle", "gnu_ton_orchard", "orchard_hub_door", True),
]
DOOR_X0, DOOR_STEP = 64, 68


def wing_door_x(index: int) -> int:
    return DOOR_X0 + index * DOOR_STEP


#: The public gate on the main floor.
PUBLIC_RING_X = 1296
PUBLIC_DIALER_X = 1500
#: The door to the gate stack stands on the low platform in the middle of the
#: hall, east of the hub's start: the intro's gate comes out of it, and the
#: public gate is a short walk east. It is the nearest door to the start (the
#: wake room's door is on the balcony above), and it stands clear of the start
#: (x 936..964): a press of Interact where a body stands at the start, or where
#: something dropped there lies, is not a press at the door.
GATE_STACK_DOOR = (1100, 832)
#: The bags for organic travellers, on the wall east of the dialer.
BAGS_X = 1600
#: The hub guide moves east of the bags, so a press at the ring or the dialer
#: is theirs. Their patrol (96 each way) stays on the main floor: west of the
#: ring the floor opening to the basement begins at 1152, and a guide placed
#: there walked into it.
GUIDE_X = 1700

#: Ring 7 of the gate stack: its dialer east of the ring, by the breaker.
RING7_DIALER = (1400, 736)


#: The military gate annex: its own room, off the General's tower.
ANNEX = "military_gate_annex"
ANNEX_W, ANNEX_H = 1024, 448
#: Out of every other level's way (the tower's neighbours are packed tight).
ANNEX_AT = (83200, 3168)
ANNEX_FLOOR = 384
#: The checkpoint: a wall from the ceiling, open with the clearance. It stops
#: `CRAWLWAY` above the floor: the maintenance crawlway, above a crouching
#: robot (24) and below a standing one (48). The wall is 64 thick, so a blink
#: does not cross it, and it reaches the ceiling, so nothing flies over.
CHECKPOINT = (448, 64)
CRAWLWAY = 32
CLEARANCE = "gate.clearance.military"
ANNEX_RING_X = 640
ANNEX_DIALER_X = 848
#: The tower's door to the annex, on the tower's ground floor (y 1248), between
#: its start and its door to the hub.
TOWER = "military_tower"
TOWER_FLOOR = 1248
TOWER_ANNEX_DOOR = (300, TOWER_FLOOR - 96)


def annex_entities() -> list[tuple]:
    floor = ANNEX_FLOOR
    wall_x, wall_w = CHECKPOINT
    return [
        ("Solid", 0, 0, ANNEX_W, 16, dict(name="annex_ceiling")),
        ("Solid", 0, 16, 16, floor - 16, dict(name="annex_west_wall")),
        ("Solid", ANNEX_W - 16, 16, 16, floor - 16, dict(name="annex_east_wall")),
        ("Solid", 0, floor, ANNEX_W, ANNEX_H - floor, dict(name="annex_floor")),
        ("LockWall", wall_x, 16, wall_w, floor - CRAWLWAY - 16,
         dict(id="annex_checkpoint", name="the military checkpoint", gated_by=f"world.flag_set {CLEARANCE}", look="temple_gate_iron")),
        ("LoadingZone", 48, floor - 96, 48, 96,
         dict(id="military_gate_annex_door", name="back to the tower", activation="Door", target_room=TOWER,
              target_zone="military_tower_annex_door", bidirectional=True)),
        ("PlayerStart", 130, floor - 46, 28, 46, dict(name=f"{ANNEX}_spawn")),
        ("DebugLabel", 160, floor - 200, 300, 24,
         dict(name="annex_checkpoint_sign", category="Room", text="CHECKPOINT: military clearance only")),
        ("DebugLabel", wall_x - 88, floor - CRAWLWAY - 28, 240, 24,
         dict(name="annex_crawlway_sign", category="Custom", text="maintenance crawlway")),
        *gates.ring("Military Gate (Annex)", "military_gate_zone", ANNEX_RING_X, floor, "the military gate"),
        *gates.terminal("Military Gate (Annex)", "military_gate_terminal", ANNEX_DIALER_X, floor),
        ("Prop", wall_x + wall_w + 48, floor - 96, 64, 96, dict(name="annex_gate_power_cabinet", kind="gate_power_cabinet")),
        ("DebugLabel", ANNEX_RING_X - 24, floor - 260, 360, 24,
         dict(name="annex_gate_sign", category="Room", text="MILITARY GATE: priority band, use is logged")),
        ("CameraZone", 0, 0, ANNEX_W, ANNEX_H, dict(id=f"{ANNEX}_camera", name=f"{ANNEX}_camera", mode="Default")),
    ]


def annex_spec() -> str:
    """The annex's `area create` spec."""
    body = "\n        ".join(gates.as_ron(annex_entities()))
    return f"""// Written by specs/gate_network/make_gates.py. Change the script, not this file.
(
    id: "{ANNEX}",
    level_id: "{ANNEX}",
    world_x: {ANNEX_AT[0]},
    world_y: {ANNEX_AT[1]},
    px_wid: {ANNEX_W},
    px_hei: {ANNEX_H},
    fill_collision: "empty",
    biome: "tower",
    music_track: "fighters_guild_oath_of_steel",
    ambient_profile: "barracks",
    visual_theme: "red",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


def tower_door_entities() -> list[tuple]:
    x, y = TOWER_ANNEX_DOOR
    return [
        ("LoadingZone", x, y, 48, 96, dict(id="military_tower_annex_door", name="to the gate annex", activation="Door",
                                           target_room=ANNEX, target_zone="military_gate_annex_door", bidirectional=True)),
        ("DebugLabel", x - 30, y - 28, 140, 24, dict(name="military_tower_annex_sign", category="Custom", text="GATE ANNEX")),
    ]


def run(*args) -> tuple[int, str]:
    """Run one tool command. A step that wrote its file is done even when the
    world's check after it fails: the world is whole only after the last
    step, and the last step checks it."""
    done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", *args], cwd=TOOLS, capture_output=True, text=True)
    out = done.stdout + done.stderr
    print(out)
    wrote = any(word in out for word in ("wrote ", "wrote new level", "replaced level"))
    return (0 if wrote else done.returncode), out


def wing_spec() -> str:
    """The developer wing's `area create` spec."""
    e = [
        gates.ron_entity("Solid", 0, 0, WING_W, 32, name="dev_wing_ceiling"),
        gates.ron_entity("Solid", 0, WING_FLOOR, WING_W, WING_H - WING_FLOOR, name="dev_wing_floor"),
        gates.ron_entity("Solid", WING_W - 48, 32, 48, WING_FLOOR - 208 - 32, name="dev_wing_east_wall"),
        gates.ron_entity("LoadingZone", WING_W - 30, WING_FLOOR - 200, 30, 176, id="east_exit", name="to scroll lab",
                         activation="EdgeExit", target_room="scroll_lab", target_zone="west_exit", bidirectional=False),
        gates.ron_entity("DebugLabel", 96, 72, 960, 24, name="dev_wing_title", category="Room",
                         text="DEVELOPER WING: test rooms and old door routes. Not public travel: the public gate is in the hall."),
    ]
    for index, (zone, name, room, far, both) in enumerate(DEV_DOORS):
        e.append(gates.ron_entity("LoadingZone", wing_door_x(index), WING_FLOOR - 96, 48, 96, id=zone, name=name,
                                  activation="Door", target_room=room, target_zone=far, bidirectional=both))
    body = "\n        ".join(e)
    return f"""// Written by specs/gate_network/make_gates.py. Change the script, not this file.
(
    id: "{AREA}",
    level_id: "{WING}",
    world_x: {WING_X},
    world_y: {WING_Y},
    px_wid: {WING_W},
    px_hei: {WING_H},
    fill_collision: "empty",
    biome: "hub",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


def holds(level: dict, identifier: str, field: str, value: str) -> bool:
    """Whether `level` has an `identifier` entity whose `field` is `value`."""
    for layer in level.get("layerInstances") or []:
        for entity in layer.get("entityInstances") or []:
            if entity["__identifier"] != identifier:
                continue
            for instance in entity.get("fieldInstances") or []:
                if instance["__identifier"] == field and instance["__value"] == value:
                    return True
    return False


def hub_delete_yaml(hub: dict) -> str:
    """The deletes of what the hub still holds of `targets` below."""
    targets = [("LoadingZone", "id", zone) for zone, *_ in DEV_DOORS]
    targets += [
        ("LoadingZone", "id", "east_exit"),
        ("LoadingZone", "id", "evil_gate_home_zone"),
        ("Switch", "id", "evil_gate_switch"),
        ("Prop", "name", "Evil Gate Ring (Home)"),
        ("Prop", "name", "Evil Gate Portal (Home)"),
        ("Solid", "name", "evil_gate_shelf"),
    ]
    lines = ["# The hub's doors that move to the developer wing, and its old Evil gate.", f"level_id: {HUB}", "deletes:"]
    for identifier, field, value in targets:
        if not holds(hub, identifier, field, value):
            continue
        lines.append(f"  - target: {{identifier: {identifier}, match: {{{field}: {json.dumps(value)}}}}}")
    return "\n".join(lines) + "\n"


def public_gate_entities() -> list[tuple]:
    return [
        *gates.ring("Public Gate (Hub)", "hub_public_gate_zone", PUBLIC_RING_X, FLOOR, "the public gate"),
        *gates.terminal("Public Gate (Hub)", "hub_public_terminal", PUBLIC_DIALER_X, FLOOR),
        ("DebugLabel", PUBLIC_RING_X - 40, FLOOR - 420, 420, 24,
         dict(name="hub_public_gate_sign", category="Room", text="PUBLIC GATE: dial at the terminal, then walk in")),
        ("DebugLabel", 1500, FLOOR - 260, 360, 24,
         dict(name="hub_dev_wing_sign", category="Room", text="developer wing (test routes) ->")),
    ]


def hub_upkeep_entities() -> list[tuple]:
    """What the public gate costs to run, where a traveller sees it: its
    power, its traffic board, and the bags for organic travellers."""
    return [
        ("Prop", PUBLIC_RING_X - 100, FLOOR - 96, 64, 96, dict(name="hub_gate_power_cabinet", kind="gate_power_cabinet")),
        ("Prop", PUBLIC_RING_X + 16, FLOOR - 376, 160, 96, dict(name="hub_gate_traffic_board", kind="gate_traffic_board")),
        ("Prop", BAGS_X, FLOOR - 48, 32, 48, dict(name="hub_gate_transit_bags", kind="gate_transit_bags")),
        ("DebugLabel", PUBLIC_RING_X - 120, FLOOR - 128, 220, 24,
         dict(name="hub_gate_power_label", category="Custom", text="RING POWER: 4 MW while open")),
        ("DebugLabel", BAGS_X - 110, FLOOR - 76, 260, 24,
         dict(name="hub_gate_bags_label", category="Custom", text="Transit sickness bags (organics)")),
    ]


def hub_move_yaml() -> str:
    return f"""# The door to the gate stack on the middle platform by the start, the bags
# east of the dialer, the hub guide west of the ring, and the signs.
level_id: {HUB}
moves:
  - target: {{identifier: LoadingZone, match: {{id: gate_stack_portal}}}}
    px: [{GATE_STACK_DOOR[0]}, {GATE_STACK_DOOR[1]}]
  - target: {{identifier: Prop, match: {{name: hub_gate_transit_bags}}}}
    px: [{BAGS_X}, {FLOOR - 48}]
  - target: {{identifier: DebugLabel, match: {{name: hub_gate_bags_label}}}}
    px: [{BAGS_X - 110}, {FLOOR - 76}]
  - target: {{identifier: NpcSpawn, match: {{dialogue_id: hub_guide}}}}
    px: [{GUIDE_X}, 904]
  - target: {{identifier: DebugLabel, match: {{name: hub_public_gate_sign}}}}
    px: [{PUBLIC_RING_X - 40}, {FLOOR - 420}]
  - target: {{identifier: DebugLabel, match: {{name: hub_dev_wing_sign}}}}
    px: [1500, {FLOOR - 260}]
"""


def hub_edit_yaml() -> str:
    return f"""# The sign to the developer wing points east, where the wing is.
level_id: {HUB}
edits:
  - target: {{identifier: DebugLabel, match: {{name: hub_dev_wing_sign}}}}
    fields:
      text: "developer wing (test routes) ->"
"""


def intro_edit_yaml() -> str:
    return """# The intro's gate comes out of the hub's door to the gate stack, beside the
# public gate. Ring 7 is dialed: its zone names no target, and its old switch
# is its power breaker.
level_id: gate_stack_lower
edits:
  - target: {identifier: LoadingZone, match: {id: intro_portal_zone}}
    fields:
      target_zone: gate_stack_portal
  - target: {identifier: LoadingZone, match: {id: math_gate_home_zone}}
    fields:
      name: ring 7
      target_room: ""
      target_zone: ""
      bidirectional: false
  - target: {identifier: Switch, match: {id: math_gate_switch}}
    fields:
      name: Ring 7 breaker
      prompt: Throw the breaker
  - target: {identifier: DebugLabel, match: {name: gate_stack_sign_gate_seven}}
    fields:
      text: "RING 7: the Math line (power: the breaker)"
"""


def ring7_entities() -> list[tuple]:
    x, floor = RING7_DIALER
    return gates.terminal("Ring 7 (Home)", "ring7_terminal", x, floor)


#: The districts' arrival rooms (`specs/pirate_port`, `specs/ninja_village`):
#: each generator adds its `gate_interface.yaml` after it makes its rooms.
#: The ring's zone is the arrival; the floor is clear from x 160 to 512.
DISTRICT_RINGS = [
    ("pirate_port", "pirate_port_gate_plaza", "Port Gate (Plaza)", "pirate_port_gate_arrival", "pirate_port_gate_terminal"),
    ("ninja_village", "ninja_village_gate_shrine", "Village Gate (Shrine)", "ninja_village_gate_arrival", "ninja_village_gate_terminal"),
]
DISTRICT_FLOOR = 704


def district_interfaces() -> dict[Path, str]:
    """Each district's `gate_interface.yaml`: its ring and its terminal."""
    out = {}
    for spec_dir, room, label, zone, terminal in DISTRICT_RINGS:
        entities = gates.ring(label, zone, 160, DISTRICT_FLOOR, "the gate") + gates.terminal(label, terminal, 364, DISTRICT_FLOOR)
        out[HERE.parent / spec_dir / "gate_interface.yaml"] = gates.as_yaml(
            room, entities, "Written by specs/gate_network/make_gates.py. Change that script, not this file.\n"
            "The district's gate ring (its zone is the arrival) and its terminal; the district's\n"
            "make_specs.py adds this file after it makes its rooms.")
    return out


def level_of(ldtk: Path, level_id: str) -> dict | None:
    world = json.loads(ldtk.read_text())
    return next((level for level in world["levels"] if level["identifier"] == level_id), None)


def apply_yaml(text: str, name: str, command: list[str], ldtk: Path) -> int:
    path = HERE / name
    path.write_text(text)
    code, _ = run(*command, str(path), "--ldtk", str(ldtk), "--in-place")
    path.unlink()
    return code


def main(argv) -> int:
    spec = HERE / "central_hub_dev_wing_area.ron"
    spec.write_text(wing_spec())
    print(spec.relative_to(TOOLS))
    for path, text in district_interfaces().items():
        path.write_text(text)
        print(path.relative_to(TOOLS))
    annex = HERE / f"{ANNEX}_area.ron"
    annex.write_text(annex_spec())
    print(annex.relative_to(TOOLS))
    if "--apply" not in argv:
        return 0
    hub = level_of(SANDBOX, HUB)
    delete = hub_delete_yaml(hub)
    if "target:" in delete:
        if apply_yaml(delete, "hub_delete.yaml", ["entity", "delete"], SANDBOX):
            return 1
    if not holds(level_of(SANDBOX, HUB), "LoadingZone", "id", "west_exit"):
        west = [("LoadingZone", 0, 776, 30, 176, dict(id="west_exit", name="to square arena", activation="EdgeExit",
                 target_room="square_arena", target_zone="east_exit", bidirectional=False))]
        if apply_yaml(gates.as_yaml(HUB, west, "The hub's west exit, back where it was."), "hub_west.yaml", ["entity", "add"], SANDBOX):
            return 1
    for level_id, place in (("pulse_voyager_outpost", PULSE_VOYAGER_AT), ("overfit_annex_entry", OVERFIT_ENTRY_TO)):
        level = level_of(SANDBOX, level_id)
        if level and (level["worldX"], level["worldY"]) != place:
            code, _ = run("world", "repack", str(SANDBOX), "--order", level_id,
                          "--start-x", str(place[0]), "--start-y", str(place[1]), "--in-place")
            if code:
                return code
    wing = level_of(SANDBOX, WING)
    if wing and (wing["worldX"], wing["worldY"]) != (WING_X, WING_Y):
        code, _ = run("level", "delete", WING, "--ldtk", str(SANDBOX), "--in-place", "--no-repair")
        if code:
            return code
    code, out = run("area", "create", str(spec), "--ldtk", str(SANDBOX), "--replace-existing")
    if code and "wrote new level" not in out and "replaced level" not in out:
        return code
    hub_text = json.dumps(level_of(SANDBOX, HUB))
    if "hub_public_gate_zone" not in hub_text:
        if apply_yaml(gates.as_yaml(HUB, public_gate_entities(), "The public gate of the central hub."),
                      "hub_public_gate.yaml", ["entity", "add"], SANDBOX):
            return 1
    if "hub_gate_power_cabinet" not in json.dumps(level_of(SANDBOX, HUB)):
        if apply_yaml(gates.as_yaml(HUB, hub_upkeep_entities(), "The public gate's upkeep, in view."),
                      "hub_upkeep.yaml", ["entity", "add"], SANDBOX):
            return 1
    if apply_yaml(hub_move_yaml(), "hub_move.yaml", ["entity", "move"], SANDBOX):
        return 1
    if apply_yaml(hub_edit_yaml(), "hub_edit.yaml", ["entity", "set-field"], SANDBOX):
        return 1
    if apply_yaml(intro_edit_yaml(), "intro_edit.yaml", ["entity", "set-field"], INTRO):
        return 1
    if "ring7_terminal" not in json.dumps(level_of(INTRO, "gate_stack_lower")):
        if apply_yaml(gates.as_yaml("gate_stack_lower", ring7_entities(), "Ring 7's terminal."),
                      "ring7_terminal.yaml", ["entity", "add"], INTRO):
            return 1
    code, out = run("area", "create", str(annex), "--ldtk", str(SANDBOX), "--replace-existing")
    if code and "wrote new level" not in out and "replaced level" not in out:
        return code
    if "military_tower_annex_door" not in json.dumps(level_of(SANDBOX, TOWER)):
        if apply_yaml(gates.as_yaml(TOWER, tower_door_entities(), "The tower's door to its gate annex."),
                      "tower_annex_door.yaml", ["entity", "add"], SANDBOX):
            return 1
    for checked in (SANDBOX, INTRO):
        code, _ = run("validate", str(checked))
        if code:
            return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
