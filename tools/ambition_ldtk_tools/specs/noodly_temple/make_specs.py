#!/usr/bin/env python3
"""Write the area specs of the Noodly Temple: a level of keys and doors.

    python specs/noodly_temple/make_specs.py          # writes the .ron specs
    python specs/noodly_temple/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The temple of the Flying Spaghetti Monster: seven rooms round one hall, in
the `noodly` biome. The monster is in the last room. The way to it is shut by
doors, and each door has a key.

    central hub -> narthex -> nave -+-> scullery   (open)        the Marinara Key
                                    +-> cistern    (red door)    the Pesto Key
                                    +-> belfry     (red door)    the Parmesan Key
                                    +-> reliquary  (green door, gold door)  the Colander Key
                                    +-> sanctum    (colander door)   the monster

A key is a `PickupSpawn` that sets a flag for good (`kind: "flag:..."`,
`respawn: "Never"`). A door is a `LockWall` that stands until its flag is set
(`gated_by`), and its `look` is the sheet that draws it. A lever is a
`Switch`; a gate that answers a lever asks `world.switch_on`, and a gate
turned round asks `not world.switch_on`.

What each room asks of the player:

- `noodly_narthex`: the way in. Ramen-nujan says what the temple is. A green
  door shuts the alms: the first lock the player sees has no key yet.
- `noodly_nave`: the hall, with a shrine. Each other room is behind a chapel
  in its walls, and each chapel but one is behind a door.
- `noodly_scullery`: a fight that shuts the room (an encounter of three
  waves, `data/encounters/goblin_encounter.ron`). Its end opens the pantry:
  the Marinara Key. A gold door shuts the larder until late in the level.
- `noodly_cistern`: three wheels and five gates. Wheel I opens the low gate
  and shuts two others, so the way to the key is found with the wheel turned
  back and the long way round. Its pools are pits in a dry deck, and no gate
  is at a pool: the game has no water that flows.
- `noodly_belfry`: a tall room with three bells, each in a side room: one
  behind a wall that breaks, one past the meatballs, one behind a green door
  (so the belfry is not finished before the cistern). Three bells open the
  three seals of the Parmesan Key.
- `noodly_reliquary`: a second fight, of the monster's own. Its end opens
  the vault: the Colander Key.
- `noodly_sanctum`: the hall of `flying_spaghetti_monster_arena`, with its
  own encounter id, so its progress is its own.

The player flies, so no height shuts a way: each way that is shut is shut by
a solid wall with one opening, and a door in the opening.

A room that is in the map already is replaced. A room names doors in rooms
that are made after it, so the map is whole only after the last room: run
`--apply` to make all seven, in order. The central hub is not replaced, so
its door stays.
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

sys.path.insert(0, str(HERE.parent / "gate_network"))
import gates  # noqa: E402

LDTK = default_sandbox_ldtk()

BIOME = "noodly"
MUSIC = "flying_spaghetti_monster_roots_stage_choir_backing"
BOSS_MUSIC = "flying_spaghetti_monster_roots_boss_choir_backing"
BG = "#15131F"
WALL = 16
#: A door (a `LockWall`) is this wide and this tall, and so is its opening.
GATE_W, GATE_H = 32, 128

#: Each room: where it is in the world (a multiple of 16), and its size.
#: The tests of the level tool make levels at 22000 to 25000, 60000, 100000
#: and 106000 of the same map: keep clear of them.
ROOMS_AT = {
    "noodly_narthex": (70000, 960, 512),
    "noodly_nave": (72000, 1600, 1024),
    "noodly_scullery": (74000, 1408, 640),
    "noodly_cistern": (76000, 1600, 896),
    "noodly_belfry": (78000, 1152, 1408),
    "noodly_reliquary": (80000, 1280, 640),
    "noodly_sanctum": (82000, 960, 768),
}

#: Each key: its flag, what the player reads when he takes it, and the sheet
#: of the key and of its door.
KEYS = {
    "marinara": ("noodly_key_marinara", "the Marinara Key"),
    "pesto": ("noodly_key_pesto", "the Pesto Key"),
    "parmesan": ("noodly_key_parmesan", "the Parmesan Key"),
    "colander": ("noodly_key_colander", "the Colander Key"),
}

#: The box of each kind of enemy (its whole size).
BOX = {
    "npc_fsm_noodling": (40, 40),
    "npc_spaghetti_event": (34, 56),
    "stochastic_parrot": (76, 60),
}


def solid(name, x, y, w, h):
    return f'( type: "Solid", px: [{x}, {y}], size: [{w}, {h}], fields: ( name: "{name}" ) ),'


def ledge(name, x, y, w):
    return f'( type: "OneWayPlatform", px: [{x}, {y}], size: [{w}, 16], fields: ( name: "{name}" ) ),'


def hazard(name, x, y, w, h=16):
    return f'( type: "HazardBlock", px: [{x}, {y}], size: [{w}, {h}], fields: ( name: "{name}" ) ),'


def door(zone, name, x, floor_y, to_room, to_zone):
    """A way to another room: a door that a press opens, on the floor at `floor_y`."""
    return (f'( type: "LoadingZone", px: [{x}, {floor_y - 96}], size: [48, 96], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def gate(wall_id, name, x, y, gated_by, look, h=GATE_H, w=GATE_W):
    """A door that stands until `gated_by` is true, drawn with the sheet `look`."""
    return (f'( type: "LockWall", px: [{x}, {y}], size: [{w}, {h}], fields: ( id: "{wall_id}", name: "{name}", '
            f'gated_by: "{gated_by}", look: "{look}" ) ),')


def key_door(wall_id, key, x, y):
    """The door that the key `key` opens."""
    flag, key_name = KEYS[key]
    return gate(wall_id, f"a door of {key_name[4:]}", x, y, flag, f"temple_gate_{key}")


def key(key, x, y):
    """The key `key`, with its middle at `x`, `y`. It is taken once."""
    flag, key_name = KEYS[key]
    return (f'( type: "PickupSpawn", px: [{x - 17}, {y - 17}], size: [34, 34], fields: ( name: "{key_name}", '
            f'kind: "flag:{flag}", respawn: "Never", sprite: "temple_key_{key}" ) ),')


def pickup(name, kind, x, y):
    return (f'( type: "PickupSpawn", px: [{x - 17}, {y - 17}], size: [34, 34], fields: ( name: "{name}", '
            f'kind: "{kind}", respawn: "Never" ) ),')


def chest(name, reward, x, floor_y):
    return (f'( type: "ChestSpawn", px: [{x}, {floor_y - 58}], size: [76, 58], fields: ( name: "{name}", '
            f'reward: "{reward}" ) ),')


def lever(switch_id, name, prompt, x, floor_y, action="ToggleFlag", target=""):
    """A lever on the floor at `floor_y`. Its state is `world.switch_on <id>`."""
    link = f', target_encounter: "{target}"' if target else ""
    return (f'( type: "Switch", px: [{x}, {floor_y - 32}], size: [24, 32], fields: ( id: "{switch_id}", name: "{name}", '
            f'prompt: "{prompt}", action: "{action}"{link} ) ),')


def pool_noodle(room, n, x, floor_y):
    """A pool noodle on the floor at `floor_y`: `Attack` picks it up, and
    `Attack` swings it."""
    return (f'( type: "GroundItem", px: [{x}, {floor_y - 12}], size: [72, 12], fields: ( id: "{room}_pool_noodle_{n}", '
            f'name: "pool noodle", held_item: "pool_noodle" ) ),')


def sign(room, n, text, x, y, w=320):
    """A line of writing in the room, with its middle over `x`."""
    return (f'( type: "DebugLabel", px: [{x - w // 2}, {y}], size: [{w}, 24], fields: ( name: "{room}_sign_{n}", '
            f'text: "{text}", category: "Custom" ) ),')


def enemy(room, n, character, x, feet_y, brain="combatant"):
    """An enemy whose feet are at `feet_y` (a flyer: where the foot of its box is)."""
    w, h = BOX[character]
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{room}_{character}_{n}", '
            f'brain: "{brain}", character_id: "{character}", respawn: "OnRoomReenter" ) ),')


def meatball(room, n, x, top_y, bottom_y, speed=150):
    """A thing that hurts and goes up and down between two heights."""
    return (f'( type: "DamageVolume", px: [{x - 22}, {top_y - 22}], size: [44, 44], fields: ( name: "{room}_meatball_{n}", '
            f'damage: 1.0, path_points: "{x},{top_y};{x},{bottom_y}", path_speed: {speed}.0, path_mode: "PingPong" ) ),')


def fight(room, encounter, floor, again_x=140, lock_x=176, trigger_x=320):
    """A fight that shuts the room: the wall that shuts it, the strip that
    starts it (as tall as the room, so no body goes over it), and the lever
    that makes it ready again (an encounter with no lever starts again each
    time a body enters the strip)."""
    return [
        (f'( type: "LockWall", px: [{lock_x}, {WALL}], size: [{GATE_W}, {floor - WALL}], fields: ( id: "{room}_lock", '
         f'name: "{room}_lock", look: "temple_gate_iron" ) ),'),
        (f'( type: "EncounterTrigger", px: [{trigger_x}, {WALL}], size: [64, {floor - WALL}], fields: ( id: "{encounter}", '
         f'name: "{encounter}_trigger", camera_zoom: 1.2 ) ),'),
        lever(f"{room}_again", "the bell of the fight", "Ring for the fight again", again_x, floor, action="ResetEncounter",
              target=encounter),
    ]


def shell(w, h):
    floor = h - 32
    return floor, [
        solid("floor", 0, floor, w, 32),
        solid("ceiling", 0, 0, w, WALL),
        solid("left_wall", 0, WALL, WALL, floor - WALL),
        solid("right_wall", w - WALL, WALL, WALL, floor - WALL),
    ]


def room(rid, entities, label, connect=(), music=MUSIC, title_at=(240, 32)):
    world_x, w, h = ROOMS_AT[rid]
    body = "\n        ".join(list(entities) + [
        f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),',
        sign(rid, "title", label, title_at[0], title_at[1], 400),
    ])
    links = "\n        ".join(connect)
    return f"""// Written by specs/noodly_temple/make_specs.py. Change the script, not this file.
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
    music_track: "{music}",
    entities: [
        {body}
    ],
    connect_to: [
        {links}
    ],
)
"""


#: Where the door of the level is in the central hub: the hub's developer
#: wing (`specs/gate_network/make_gates.py`). The public way in is the
#: temple's own ring in the narthex (ALDENTE, `data/gate_network.ron`).
HUB_LEVEL = "central_hub_dev_wing"
HUB_DOOR = (948, 368)
#: What the player reads at that door.
HUB_DOOR_NAME = "to the Noodly Temple"
HUB_DOOR_ID = "noodly_hub_door"


def narthex():
    rid = "noodly_narthex"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [110, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_to_hub", "to the central hub", 32, floor, "central_hub_complex", HUB_DOOR_ID),
        # The temple's ring and its terminal: the way to the Noodle dimension.
        *gates.as_ron(gates.ring("Noodly Gate (Narthex)", "noodly_narthex_gate", 80, floor, "the gate")),
        *gates.as_ron(gates.terminal("Noodly Gate (Narthex)", "noodly_gate_terminal", 380, floor)),
        door(f"{rid}_east", "to the nave", w - WALL - 80, floor, "noodly_nave", "noodly_nave_west"),
        # Ramen-nujan says what the temple is (Jon, 2026-10-12: he is the one
        # at the door, not a sexton).
        (f'( type: "NpcSpawn", px: [480, {floor - 70}], size: [42, 70], fields: ( character_id: "npc_ramen_nujan", '
         f'prompt: "Talk", dialogue_id: "noodly_ramen_nujan", brain_override: "stand_still" ) ),'),
        # The alms: a room over the way out, shut by a green door. The player
        # has no green key for a long time.
        solid("alms_floor", 560, 240, w - WALL - 560, 32),
        solid("alms_lintel", 560, WALL, GATE_W, 96),
        key_door(f"{rid}_alms_door", "pesto", 560, 112),
        chest("the alms of the temple", "currency:150", 700, 240),
        pickup("a heart of the alms", "health:2", 860, 200),
        ledge("alms_step", 380, 304, 144),
        sign(rid, "alms", "The alms. A seal of pesto shuts them.", 700, 60),
    ]
    connect = [(f'( target_room: "{HUB_LEVEL}", px: [{HUB_DOOR[0]}, {HUB_DOOR[1]}], size: [48, 96], id: "{HUB_DOOR_ID}", '
                f'name: "{HUB_DOOR_NAME}", target_zone: "{rid}_to_hub", activation: "Door", bidirectional: true ),')]
    return rid, room(rid, e, "The Noodly Temple. Each seal has a key, and each key is hidden.", connect)


def chapel(rid, name, west, top, doors, zone, door_name, to_room, to_zone, label):
    """A chapel in a wall of the nave: a room 160 high whose one opening, to
    the hall, is shut by `doors` (a list of keys, from the hall inward). Its
    way out to another room is at its far end."""
    _, w, _ = ROOMS_AT[rid]
    # How far the chapel goes into the wall: each door more is 96 deeper.
    depth = 240 + 96 * (len(doors) - 1)
    out = []
    if west:
        # `mouth` is the door at the hall. The floor goes 96 past it: a porch.
        mouth = WALL + depth
        roof_x, floor_x = WALL, WALL
        gate_xs = [mouth - 96 * i for i in range(len(doors))]
        door_x, sign_x = 64, mouth + 150
    else:
        mouth = w - WALL - depth - GATE_W
        roof_x, floor_x = mouth, mouth - 96
        gate_xs = [mouth + 96 * i for i in range(len(doors))]
        door_x, sign_x = w - WALL - 96, mouth - 120
    out.append(solid(f"{name}_roof", roof_x, top - 32, depth + GATE_W, 32))
    out.append(solid(f"{name}_floor", floor_x, top + 160, depth + GATE_W + 96, 32))
    for i, (gx, key_id) in enumerate(zip(gate_xs, doors)):
        out.append(solid(f"{name}_lintel_{i}", gx, top, GATE_W, 32))
        out.append(key_door(f"{rid}_{name}_door_{i}", key_id, gx, top + 32))
    out.append(door(zone, door_name, door_x, top + 160, to_room, to_zone))
    out.append(sign(rid, name, label, sign_x, top - 64, 280))
    return out


def nave():
    rid = "noodly_nave"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    e += [
        f'( type: "PlayerStart", px: [110, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "to the narthex", 32, floor, "noodly_narthex", "noodly_narthex_east"),
        door(f"{rid}_east", "to the scullery", w - WALL - 80, floor, "noodly_scullery", "noodly_scullery_west"),
        f'( type: "ShrineSpawn", px: [778, {floor - 80}], size: [44, 80], fields: ( id: "{rid}_shrine", name: "the shrine of the nave" ) ),',
        sign(rid, "scullery", "The scullery. It is open.", w - 190, floor - 150, 280),
    ]
    e += chapel(rid, "cistern", True, 576, ["marinara"], f"{rid}_cistern", "to the cistern", "noodly_cistern",
                "noodly_cistern_west", "The cistern")
    e += chapel(rid, "belfry", True, 256, ["marinara"], f"{rid}_belfry", "to the belfry", "noodly_belfry",
                "noodly_belfry_door", "The belfry")
    e += chapel(rid, "reliquary", False, 576, ["pesto", "parmesan"], f"{rid}_reliquary", "to the reliquary",
                "noodly_reliquary", "noodly_reliquary_west", "The reliquary")
    e += chapel(rid, "sanctum", False, 256, ["colander"], f"{rid}_sanctum", "to the sanctum", "noodly_sanctum",
                "noodly_sanctum_entry", "The sanctum")
    # Steps up the hall, for a body that does not fly.
    for n, (x, y) in enumerate(((432, 880), (1008, 880), (720, 784), (480, 672), (960, 672), (720, 560), (480, 448),
                                 (960, 448), (720, 336), (480, 224), (960, 224))):
        e.append(ledge(f"step_{n}", x, y, 160))
    return rid, room(rid, e, "The nave. The shrine keeps your place.", title_at=(800, floor - 170))


def scullery():
    rid = "noodly_scullery"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    brawl = f"{rid}_brawl"
    e += fight(rid, brawl, floor)
    e += [
        f'( type: "PlayerStart", px: [96, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "to the nave", 32, floor, "noodly_nave", "noodly_nave_east"),
        ledge("rack_a", 480, 480, 160),
        ledge("rack_b", 800, 480, 160),
        ledge("rack_c", 640, 368, 160),
        # The east wall of the kitchen: the larder over the pantry.
        solid("store_wall_top", 1120, WALL, GATE_W, 208),
        key_door(f"{rid}_larder_door", "parmesan", 1120, 224),
        solid("store_wall_mid", 1120, 352, GATE_W, 128),
        gate(f"{rid}_pantry_door", "the grille of the pantry", 1120, 480, f"encounter.cleared {brawl}", "temple_gate_iron"),
        solid("pantry_roof", 1152, 352, w - WALL - 1152, 32),
        key("marinara", 1270, floor - 60),
        chest("the best of the larder", "health:3", 1190, 352),
        pickup("the purse of the cook", "currency:120", 1330, 300),
        sign(rid, "pantry", "The pantry opens when the kitchen is quiet.", 930, floor - 190, 340),
        sign(rid, "larder", "The larder. A seal of parmesan.", 960, 180, 300),
    ]
    return rid, room(rid, e, "The scullery. The cooks do not like guests.")


def cistern():
    rid = "noodly_cistern"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    # The deck: the dry floor that a body walks on and that each gate stands
    # on. The pools are pits in it, down to the floor of the room. No gate is
    # at a pool: the game has no water that flows, so a gate that held water
    # back would open on water that stays where it is.
    deck = 768
    pools = ((336, 448), (560, 704), (832, 992), (1216, 1424))
    a, b, c = "noodly_wheel_a", "noodly_wheel_b", "noodly_wheel_c"
    on = "world.switch_on {}".format
    off = "not world.switch_on {}".format
    # The look of a gate says which wheel it answers, and how.
    opened_by = "temple_gate_wheel_{}".format
    shut_by = "temple_gate_wheel_{}_shut".format
    x = WALL
    for n, (x0, x1) in enumerate(pools):
        e.append(solid(f"deck_{n}", x, deck, x0 - x, floor - deck))
        e.append(f'( type: "WaterVolume", px: [{x0}, {deck + 16}], size: [{x1 - x0}, {floor - deck - 16}], fields: ( id: "{rid}_pool_{n}", name: "a pool" ) ),')
        x = x1
    e.append(solid(f"deck_{len(pools)}", x, deck, w - WALL - x, floor - deck))
    e += [
        f'( type: "PlayerStart", px: [96, {deck - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "to the nave", 32, deck, "noodly_nave", "noodly_nave_cistern"),
        # The landing: as tall as the room. Wheel I is in it, by its pool.
        lever(a, "wheel I", "Turn wheel I", 270, deck),
        sign(rid, "wheel_a", "Wheel I", 282, deck - 80, 120),
        # The wall of the landing: the high gate (wheel II) and the low gate
        # (wheel I).
        solid("landing_wall_top", 480, WALL, GATE_W, 224),
        gate(f"{rid}_gate_high", "the high gate", 480, 240, on(b), opened_by(2)),
        solid("landing_wall_mid", 480, 368, GATE_W, 272),
        gate(f"{rid}_gate_low", "the low gate", 480, 640, on(a), opened_by(1)),
        sign(rid, "high", "II opens", 420, 200, 120),
        sign(rid, "low", "I opens", 420, 600, 120),
        # The floor between the gallery and the basin, with the well in it.
        solid("gallery_floor_west", 512, 368, 432, 32),
        solid("gallery_floor_east", 1040, 368, 128, 32),
        # The gallery: wheel III, and the gate of the well (shut by wheel I).
        lever(c, "wheel III", "Turn wheel III", 690, 368),
        sign(rid, "wheel_c", "Wheel III", 702, 288, 120),
        solid("well_wall_top", 912, WALL, GATE_W, 224),
        gate(f"{rid}_gate_well", "the gate of the well", 912, 240, off(a), shut_by(1)),
        sign(rid, "well", "I shuts", 850, 200, 120),
        solid("well_back", 1040, WALL, 128, 352),
        # The basin: two pools, and wheel II on the deck between them.
        lever(b, "wheel II", "Turn wheel II", 756, deck),
        sign(rid, "wheel_b", "Wheel II", 768, deck - 80, 120),
        # The lock: two gates in a row (shut by wheel I, opened by wheel III).
        solid("lock_roof", 1040, 400, 128, 240),
        gate(f"{rid}_gate_lock_a", "the first gate of the lock", 1040, 640, off(a), shut_by(1)),
        gate(f"{rid}_gate_lock_b", "the second gate of the lock", 1136, 640, on(c), opened_by(3)),
        sign(rid, "lock", "I shuts. III opens.", 1000, 600, 200),
        # The vault: as tall as the room. The key is on a shelf in it.
        ledge("key_shelf", 1296, 304, 160),
        key("pesto", 1376, 260),
        # A cell with a gold door, for the end of the level.
        solid("cell_roof", 1456, 560, w - WALL - 1456, 32),
        solid("cell_wall", 1456, 592, GATE_W, 48),
        key_door(f"{rid}_cell_door", "parmesan", 1456, 640),
        chest("the hoard of the cistern", "currency:200", 1496, deck),
        # By the pools: a pool noodle on the landing, and one on the shelf of
        # the vault.
        pool_noodle(rid, 1, 150, deck),
        pool_noodle(rid, 2, 1298, 304),
        sign(rid, "noodle", "A pool noodle. Attack takes it.", 190, deck - 130, 260),
        enemy(rid, 1, "npc_fsm_noodling", 720, 190),
        enemy(rid, 2, "npc_fsm_noodling", 860, 560),
        enemy(rid, 3, "npc_fsm_noodling", 1340, 480),
    ]
    return rid, room(rid, e, "The cistern. A wheel that opens one gate shuts another.")


def belfry():
    rid = "noodly_belfry"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    bells = [f"noodly_bell_{n}" for n in (1, 2, 3)]
    # A bell that was rung stays rung: the flag of a lever that was used.
    rung = "switch_{}_used".format
    seal = "temple_gate_bell"
    e += [
        f'( type: "PlayerStart", px: [470, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_door", "to the nave", 552, floor, "noodly_nave", "noodly_nave_belfry"),
        # West of the shaft: the high room (a green door) and the low room (a
        # wall that breaks), in solid stone.
        solid("west_top", WALL, WALL, 384, 304),
        solid("west_mid", WALL, 480, 384, 576),
        solid("west_low", WALL, 1216, 384, floor - 1216),
        solid("west_wall_top", 400, WALL, GATE_W, 336),
        key_door(f"{rid}_high_door", "pesto", 400, 352),
        solid("west_wall_mid", 400, 480, GATE_W, 608),
        (f'( type: "BreakablePlatform", px: [400, 1088], size: [{GATE_W}, {GATE_H}], fields: ( name: "a cracked wall", '
         f'max_hp: 3.0, respawn: "Never", collision: "Solid", trigger: "OnHit" ) ),'),
        solid("west_wall_low", 400, 1216, GATE_W, floor - 1216),
        # East of the shaft: the vault of the key (three seals) and the room
        # of the meatballs.
        solid("east_top", 752, WALL, w - WALL - 752, 208),
        solid("east_mid", 752, 384, w - WALL - 752, 256),
        solid("east_low", 752, 800, w - WALL - 752, floor - 800),
        solid("east_wall_top", 720, WALL, GATE_W, 240),
        gate(f"{rid}_seal_1", "the first seal", 720, 256, rung(bells[0]), seal),
        solid("seal_lintel_2", 816, 224, GATE_W, 32),
        gate(f"{rid}_seal_2", "the second seal", 816, 256, rung(bells[1]), seal),
        solid("seal_lintel_3", 912, 224, GATE_W, 32),
        gate(f"{rid}_seal_3", "the third seal", 912, 256, rung(bells[2]), seal),
        key("parmesan", 1040, 330),
        solid("east_wall_mid", 720, 384, GATE_W, 288),
        solid("east_wall_low", 720, 800, GATE_W, floor - 800),
        # The low bell, behind the cracked wall.
        lever(bells[0], "the low bell", "Ring the low bell", 90, 1216, action="ToggleFlag"),
        enemy(rid, 1, "npc_spaghetti_event", 230, 1216, brain="medium_striker"),
        sign(rid, "bell_1", "The low bell", 110, 1100, 160),
        sign(rid, "crack", "This wall is cracked.", 576, 1050, 240),
        # The bell of the meatballs.
        lever(bells[1], "the bell of the meatballs", "Ring the bell", 1080, 800, action="ToggleFlag"),
        meatball(rid, 1, 840, 680, 770, 130),
        meatball(rid, 2, 930, 770, 680, 150),
        meatball(rid, 3, 1010, 680, 770, 170),
        sign(rid, "bell_2", "The middle bell", 1000, 650, 180),
        # The high bell, behind the green door.
        lever(bells[2], "the high bell", "Ring the high bell", 90, 480, action="ToggleFlag"),
        pickup("a heart by the high bell", "health:2", 250, 440),
        sign(rid, "bell_3", "The high bell", 110, 360, 160),
        sign(rid, "seals", "Three bells, three seals.", 576, 190, 260),
        enemy(rid, 1, "stochastic_parrot", 520, 980, brain="sky_parrot"),
        enemy(rid, 2, "stochastic_parrot", 600, 560, brain="sky_parrot"),
        enemy(rid, 1, "npc_fsm_noodling", 560, 760),
    ]
    # Steps up the shaft, from side to side.
    for n, y in enumerate(range(1264, 240, -112)):
        e.append(ledge(f"step_{n}", 432 if n % 2 == 0 else 592, y, 128))
    return rid, room(rid, e, "The belfry. Three bells.", title_at=(576, floor - 190))


def reliquary():
    rid = "noodly_reliquary"
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(w, h)
    vigil = f"{rid}_vigil"
    e += fight(rid, vigil, floor)
    e += [
        f'( type: "PlayerStart", px: [96, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_west", "to the nave", 32, floor, "noodly_nave", "noodly_nave_reliquary"),
        ledge("stall_a", 448, 464, 144),
        ledge("stall_b", 768, 464, 144),
        ledge("stall_c", 608, 336, 144),
        hazard("sauce_a", 480, floor - 16, 96),
        hazard("sauce_b", 784, floor - 16, 96),
        solid("vault_wall", 1056, WALL, GATE_W, 464),
        gate(f"{rid}_vault_door", "the grille of the vault", 1056, 480, f"encounter.cleared {vigil}", "temple_gate_iron"),
        key("colander", 1168, floor - 60),
        sign(rid, "vault", "The vault opens when the vigil ends.", 860, floor - 190, 320),
    ]
    return rid, room(rid, e, "The reliquary. His own keep watch here.")


def sanctum():
    rid = "noodly_sanctum"
    _, w, h = ROOMS_AT[rid]
    floor = h - 32
    e = [
        solid("floor", 0, floor, w, 32),
        solid("left_wall", 0, 0, WALL, floor),
        solid("right_wall", w - WALL, 0, WALL, floor),
        solid("ceiling", WALL, 0, w - 2 * WALL, WALL),
        # The hall of `flying_spaghetti_monster_arena`: a ledge by each wall.
        ledge("ledge_left", 96, 592, 192),
        ledge("ledge_right", 672, 592, 192),
        f'( type: "PlayerStart", px: [96, {floor - 64}], size: [24, 48], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_entry", "to the nave", 160, floor, "noodly_nave", "noodly_nave_sanctum"),
        ('( type: "BossSpawn", px: [480, 240], size: [168, 150], fields: ( name: "Flying Spaghetti Monster", '
         'brain: "PhaseScript:flying_spaghetti_monster_boss", encounter_id: "noodly_temple.fsm" ) ),'),
    ]
    return rid, room(rid, e, "The sanctum.", music=BOSS_MUSIC)


ROOMS = (narthex, nave, scullery, cistern, belfry, reliquary, sanctum)


def hub_door_is_placed():
    """The central hub has the door of this level already."""
    world = json.loads(Path(LDTK).read_text())
    hub = next((level for level in world["levels"] if level["identifier"] == HUB_LEVEL), None)
    return hub is not None and HUB_DOOR_ID in json.dumps(hub)


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}_area.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    if "--apply" in argv:
        for path in made:
            spec = path
            if path == made[0] and hub_door_is_placed():
                # The hub is not replaced, so its door is there already: make
                # the narthex again without the link that would place it.
                spec = Path(tempfile.mkdtemp()) / path.name
                spec.write_text(re.sub(r"connect_to: \[.*?\n    \],", "connect_to: [],", path.read_text(), flags=re.S))
            done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", "area", "create", str(spec), "--ldtk",
                                   str(LDTK), "--replace-existing"], cwd=TOOLS, capture_output=True, text=True)
            print(done.stdout + done.stderr)
            # A room names doors in rooms that are made after it, so the map
            # is not whole until the last room is made: before that, it is
            # enough that the room was written.
            if done.returncode != 0 and (path == made[-1] or "wrote new level" not in done.stdout + done.stderr):
                return done.returncode
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
