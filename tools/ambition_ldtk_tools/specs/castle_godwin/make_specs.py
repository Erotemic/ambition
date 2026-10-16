#!/usr/bin/env python3
"""Write the rooms of the Evil dimension's first place, Castle Godwin, and the
gate that leads to it from the central hub.

    python specs/castle_godwin/make_specs.py          # writes the specs
    python specs/castle_godwin/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

The Evil dimension is its own world file, `evil.ldtk`: one world file is one
dimension. Its rooms:

- `evil_gate_forest`: the gate of the dimension stands in a small dark
  forest (theme `evil_forest`). East of it, past a sign, is the front of the
  castle (`castle_godwin_entrance`, after the fortress doors of Super Mario
  World), with its name over the door. The door goes in. The forest is only
  big enough to hold the gate and the way in.
- `castle_godwin_gatehouse`, `castle_godwin_barracks`, `castle_godwin_tower`,
  `castle_godwin_great_hall`: rooms of combat (theme `castle_godwin`), one
  after the other. The soldiers are `npc_fascist_enforcer`. The great hall
  is a fight that shuts the room (encounter `castle_godwin_muster`,
  `data/encounters/goblin_encounter.ron`).

And in the Home dimension, in the central hub (`sandbox.ldtk`): a stone shelf
high on the east side with the gate on it and its switch. Both gates follow
that one switch (the pair is one link, `data/gate_portals.ron`).

A room names doors in rooms that are made after it, so the worlds are whole
only after the last room: run `--apply` to make them all, in order.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOLS = HERE.parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from ambition_ldtk_tools.ldtk.paths import default_worlds_dir  # noqa: E402

EVIL_LDTK = default_worlds_dir() / "evil.ldtk"
SANDBOX_LDTK = default_worlds_dir() / "sandbox.ldtk"

FOREST_THEME, CASTLE_THEME = "evil_forest", "castle_godwin"
FOREST_MUSIC, CASTLE_MUSIC = "oppressive_crush", "military_iron_resolve"
FOREST_BG, CASTLE_BG = "#1A0A0C", "#0E0E14"
WALL = 16

#: The ids `data/gate_portals.ron` names.
SWITCH = "evil_gate_switch"
HOME_ZONE = "evil_gate_home_zone"
EVIL_ZONE = "evil_gate_return_zone"
HOME_RING, HOME_PORTAL = "Evil Gate Ring (Home)", "Evil Gate Portal (Home)"
EVIL_RING, EVIL_PORTAL = "Evil Gate Ring (Evil)", "Evil Gate Portal (Evil)"

FOREST = "evil_gate_forest"
GATEHOUSE = "castle_godwin_gatehouse"
BARRACKS = "castle_godwin_barracks"
TOWER = "castle_godwin_tower"
GREAT_HALL = "castle_godwin_great_hall"
THRONE = "castle_godwin_throne"
#: The fight of the great hall: its encounter id.
MUSTER = "castle_godwin_muster"

#: Each room: where it is in the world, and its size.
ROOMS_AT = {
    FOREST: (0, 1760, 720),
    GATEHOUSE: (2000, 1600, 768),
    BARRACKS: (3792, 2560, 768),
    TOWER: (6544, 1024, 1792),
    GREAT_HALL: (7760, 1920, 896),
    THRONE: (9920, 1920, 896),
}

#: The boss of the castle: Wolfenstein, the monster of the regime with the head
#: of a mad wolf. Its pattern is `wolfenstein_boss` (`data/boss_profiles.ron`);
#: its placement id is its progress (`castle_godwin.wolfenstein`).
BOSS_NAME, BOSS_BRAIN, BOSS_ENCOUNTER = "Wolfenstein", "PhaseScript:wolfenstein_boss", "castle_godwin.wolfenstein"
BOSS_BOX = (60, 100)

#: The soldier of the regime, and his box (183 cm).
SOLDIER = "npc_fascist_enforcer"
SOLDIER_BOX = (32, 72)
#: The regime's other fighters: the Iron Trooper with his chain gun (205 cm)
#: and the lab's Failed Prototypes, which come in packs. Each its box.
HEAVY, PROTOTYPE, HOUND = "npc_evil_heavy", "npc_evil_failed_prototype", "npc_evil_attack_hound"
BOX = {SOLDIER: SOLDIER_BOX, HEAVY: (36, 80), PROTOTYPE: (32, 40), HOUND: (52, 50)}

SPRITES = TOOLS.parents[1] / "crates/ambition_platformer2d_actor_monolith/assets/sprites"
#: Pixels of a castle prop's frame to one world unit (`castle_godwin.PX`).
PROP_PX = 2
#: The doorway of the castle's front, in the art's world units: its left edge
#: and its width (`castle_godwin.DOOR`). The numbers are the art's.
ENTRANCE_DOOR_X, ENTRANCE_DOOR_W = 144, 72


def solid(name, x, y, w, h):
    return f'( type: "Solid", px: [{x}, {y}], size: [{w}, {h}], fields: ( name: "{name}" ) ),'


def ledge(name, x, y, w):
    return f'( type: "OneWayPlatform", px: [{x}, {y}], size: [{w}, 16], fields: ( name: "{name}" ) ),'


def label(name, x, y, text, w=420):
    return f'( type: "DebugLabel", px: [{x}, {y}], size: [{w}, 24], fields: ( name: "{name}", text: "{text}", category: "Custom" ) ),'


def door(zone, name, x, floor, to_room, to_zone):
    return (f'( type: "LoadingZone", px: [{x}, {floor - 96}], size: [48, 96], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def gate(ring, portal, zone, x, floor, to_room, to_zone, name):
    """A gate ring standing on `floor` at `x` (its left edge), its portal in
    it, and the door of the portal: a Door the size of the opening."""
    top = floor - 192
    return [
        f'( type: "Prop", px: [{x}, {top}], size: [192, 192], fields: ( name: "{ring}", kind: "gate_ring" ) ),',
        f'( type: "Prop", px: [{x}, {top}], size: [192, 192], fields: ( name: "{portal}", kind: "gate_portal" ) ),',
        (f'( type: "LoadingZone", px: [{x + 48}, {top}], size: [96, 192], fields: ( id: "{zone}", name: "{name}", '
         f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: false ) ),'),
    ]


def soldier(room, n, x, feet_y, character=SOLDIER):
    """A fighter of the regime (a soldier unless `character` says) whose
    feet are at `feet_y`."""
    w, h = BOX[character]
    kind = {SOLDIER: "soldier", HEAVY: "heavy", PROTOTYPE: "prototype", HOUND: "hound"}[character]
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{room}_{kind}_{n}", '
            f'brain: "combatant", character_id: "{character}", respawn: "OnRoomReenter" ) ),')


def pack(room, x, feet_y, size=3, gap=44):
    """A pack of Failed Prototypes, side by side from `x`."""
    return [soldier(room, i + 1, x + i * gap, feet_y, PROTOTYPE) for i in range(size)]


def body_box(kind):
    """The body box of `kind`'s published sheet in frame pixels, (x, y, w, h)."""
    text = (SPRITES / f"{kind}_spritesheet.ron").read_text()
    m = re.search(r"body_pixel_bbox: Some\(\(x: (\d+), y: (\d+), w: (\d+), h: (\d+)\)\)", text)
    if not m:
        raise SystemExit(f"{kind}: no body_pixel_bbox in its published RON; publish the sprite first")
    return tuple(int(v) for v in m.groups())


def standing(name, kind, x, floor):
    """Prop `kind` with the left of its FRAME at `x` and its art's bottom on
    `floor`; the prop's box is the art's body box in the frame."""
    bx, by, bw, bh = body_box(kind)
    fx, fy = x, floor - (by + bh) / PROP_PX
    return (f'( type: "Prop", px: [{round(fx + bx / PROP_PX)}, {round(fy + by / PROP_PX)}], '
            f'size: [{round(bw / PROP_PX)}, {round(bh / PROP_PX)}], fields: ( name: "{name}", kind: "{kind}" ) ),')


def room(rid, entities, label_text, theme, music, bg):
    world_x, w, h = ROOMS_AT[rid]
    body = "\n        ".join(entities + [
        f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),',
        label(f"{rid}_label", 70, 40, label_text, 900),
    ])
    return f"""// Written by specs/castle_godwin/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: 0,
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{bg}",
    biome: "{theme}",
    parallax_theme: "{theme}",
    music_track: "{music}",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


def castle(rid, entities, label_text):
    return room(rid, entities, label_text, CASTLE_THEME, CASTLE_MUSIC, CASTLE_BG)


def shell(rid, ground=64, ceiling=48):
    """The floor, the vault and the two walls of a castle room. Returns the
    top of the floor and the entities."""
    _, w, h = ROOMS_AT[rid]
    floor = h - ground
    return floor, [
        f'( type: "PlayerStart", px: [128, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        solid("floor", 0, floor, w, ground),
        solid("vault", 0, 0, w, ceiling),
        solid("west_wall", 0, ceiling, WALL, floor - ceiling),
        solid("east_wall", w - WALL, ceiling, WALL, floor - ceiling),
    ]


# ---------------------------------------------------------------------------
# The rooms
# ---------------------------------------------------------------------------


def forest():
    """The gate in a clearing of black trees; east, the road to the castle's
    front, with the sign at the road and thorns at the foot of the wall."""
    rid = FOREST
    _, w, h = ROOMS_AT[rid]
    floor = h - 64
    entrance_x = w - 352
    e = [
        solid("ground", 0, floor, w, 64),
        solid("west_wall", 0, 0, WALL, floor),
        solid("east_wall", w - WALL, 0, WALL, floor),
        f'( type: "PlayerStart", px: [360, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        *gate(EVIL_RING, EVIL_PORTAL, EVIL_ZONE, 112, floor, "central_hub_complex", HOME_ZONE, "to the Home dimension"),
        # Scenery, furthest back first: the castle's front, the thorns, the sign.
        standing("castle_godwin_front", "castle_godwin_entrance", entrance_x, floor),
        standing("thorns_gate", "evil_thorn_bush", 330, floor),
        standing("thorns_road_a", "evil_thorn_bush", 900, floor),
        standing("thorns_road_b", "evil_thorn_bush", 1180, floor),
        standing("thorns_wall", "evil_thorn_bush", entrance_x - 60, floor),
        standing("castle_godwin_road_sign", "castle_godwin_sign", 1000, floor),
        door(f"{rid}_castle_door", "into Castle Godwin", entrance_x + ENTRANCE_DOOR_X + (ENTRANCE_DOOR_W - 48) // 2, floor,
             GATEHOUSE, f"{GATEHOUSE}_west"),
    ]
    return rid, room(rid, e, "The forest of the Evil dimension: the way to Castle Godwin.", FOREST_THEME, FOREST_MUSIC, FOREST_BG)


def gatehouse():
    """The first room inside: three soldiers, a raised dais and a gallery."""
    rid = GATEHOUSE
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    e += [
        door(f"{rid}_west", "out to the forest", 64, floor, FOREST, f"{FOREST}_castle_door"),
        door(f"{rid}_east", "on to the barracks", w - 112, floor, BARRACKS, f"{BARRACKS}_west"),
        solid("dais", 560, floor - 64, 192, 64),
        ledge("gallery_west", 880, floor - 160, 224),
        ledge("gallery_east", 1200, floor - 256, 192),
        soldier(rid, 1, 640, floor - 64),
        soldier(rid, 2, 980, floor - 160),
        soldier(rid, 3, 1320, floor),
        # The guard dogs of the gate.
        soldier(rid, 1, 1000, floor, HOUND),
        soldier(rid, 2, 1180, floor, HOUND),
    ]
    return rid, castle(rid, e, "Castle Godwin: the gatehouse.")


def barracks():
    """A long hall of bunks in tiers, and pillars that hang from the vault to
    hide behind. Five soldiers."""
    rid = BARRACKS
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    e += [
        door(f"{rid}_west", "back to the gatehouse", 64, floor, GATEHOUSE, f"{GATEHOUSE}_east"),
        door(f"{rid}_east", "up the tower", w - 112, floor, TOWER, f"{TOWER}_foot"),
    ]
    for i, x in enumerate((360, 1000, 1640)):
        e += [ledge(f"bunk_{i}_low", x, floor - 112, 224), ledge(f"bunk_{i}_high", x, floor - 224, 224)]
    for i, x in enumerate((760, 1400, 2040)):
        e.append(solid(f"pillar_{i}", x, 48, 64, floor - 208))
    e += [
        soldier(rid, 1, 520, floor),
        soldier(rid, 2, 1100, floor - 224),
        soldier(rid, 3, 1300, floor),
        soldier(rid, 4, 1760, floor - 112),
        soldier(rid, 5, 2200, floor),
        soldier(rid, 1, 700, floor, HOUND),
        # The far end of the barracks: the lab's door is near, and a pack got
        # out; an Iron Trooper holds the way to the tower.
        *pack(rid, 1840, floor),
        soldier(rid, 1, 2300, floor, HEAVY),
    ]
    return rid, castle(rid, e, "Castle Godwin: the barracks.")


def tower():
    """A climb: ledges from side to side up the tower, a soldier on every
    other ledge, and the door to the great hall at the top."""
    rid = TOWER
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    top_floor = 256
    e += [
        door(f"{rid}_foot", "down to the barracks", 64, floor, BARRACKS, f"{BARRACKS}_east"),
        solid("top_floor", w - 384, top_floor, 384 - WALL, 32),
        door(f"{rid}_top", "into the great hall", w - 112, top_floor, GREAT_HALL, f"{GREAT_HALL}_west"),
    ]
    n = 0
    y = floor - 176
    k = 0
    while y > top_floor + 96:
        x = 160 if k % 2 == 0 else w - 160 - 288
        e.append(ledge(f"ledge_{k}", x, y, 288))
        if k % 2 == 1:
            n += 1
            e.append(soldier(rid, n, x + 128, y))
        y -= 176
        k += 1
    # The top of the tower: an Iron Trooper, and a pack on the first ledge.
    e.append(soldier(rid, 1, w - 260, top_floor, HEAVY))
    e += pack(rid, 200, floor - 176)
    return rid, castle(rid, e, "Castle Godwin: the tower.")


def lever(name, label_text, prompt, x, floor, action="ResetEncounter", target=""):
    return (f'( type: "Switch", px: [{x}, {floor - 32}], size: [24, 32], fields: ( id: "{name}", name: "{label_text}", '
            f'prompt: "{prompt}", action: "{action}", target_encounter: "{target}" ) ),')


def great_hall():
    """The fight of the castle: a grille shuts the way back, and the muster
    comes in three waves (`castle_godwin_muster`). Two galleries and a
    dais to fight from."""
    rid = GREAT_HALL
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    e += [
        door(f"{rid}_west", "back down the tower", 64, floor, TOWER, f"{TOWER}_top"),
        door(f"{rid}_east", "to the throne room", w - 112, floor, THRONE, f"{THRONE}_west"),
        # The way on to the throne is shut until the muster is beaten.
        (f'( type: "LockWall", px: [{w - 176}, 48], size: [32, {floor - 48}], fields: ( id: "{rid}_throne_grille", '
         f'name: "the grille of the throne room", gated_by: "encounter.cleared {MUSTER}", look: "temple_gate_iron" ) ),'),
        (f'( type: "LockWall", px: [176, 48], size: [32, {floor - 48}], fields: ( id: "{rid}_lock", '
         f'name: "{rid}_lock", look: "temple_gate_iron" ) ),'),
        (f'( type: "EncounterTrigger", px: [320, 48], size: [64, {floor - 48}], fields: ( id: "{MUSTER}", '
         f'name: "{MUSTER}_trigger", camera_zoom: 1.2 ) ),'),
        lever(f"{rid}_again", "the gong of the muster", "Strike the gong", 128, floor, target=MUSTER),
        ledge("gallery_west", 448, floor - 192, 320),
        ledge("gallery_east", w - 768, floor - 192, 320),
        solid("dais", w // 2 - 160, floor - 48, 320, 48),
        ledge("balcony", w // 2 - 192, floor - 352, 384),
    ]
    return rid, castle(rid, e, "Castle Godwin: the great hall.")


def throne():
    """The throne room: Wolfenstein's arena. A flat floor to fight on, two
    low galleries to jump to over his sweeps, and his throne at the east end
    (scenery of the room's look: the banners and braziers)."""
    rid = THRONE
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    bw, bh = BOSS_BOX
    e += [
        door(f"{rid}_west", "back to the great hall", 64, floor, GREAT_HALL, f"{GREAT_HALL}_east"),
        ledge("gallery_west", 416, floor - 160, 256),
        ledge("gallery_east", w - 672, floor - 160, 256),
        ledge("high_gallery", w // 2 - 160, floor - 320, 320),
        (f'( type: "BossSpawn", px: [{w - 520}, {floor - bh}], size: [{bw}, {bh}], fields: ( name: "{BOSS_NAME}", '
         f'brain: "{BOSS_BRAIN}", encounter_id: "{BOSS_ENCOUNTER}" ) ),'),
    ]
    # A boss arena's camera frames the fight (`Encounter`), as the other arenas do.
    text = castle(rid, e, "Castle Godwin: the throne room of Wolfenstein.")
    return rid, text.replace(f'id: "{rid}_camera", name: "{rid}_camera", mode: "Default"', f'id: "{rid}_camera", name: "{rid}_camera", mode: "Encounter"')


ROOMS = (forest, gatehouse, barracks, tower, great_hall, throne)

#: The fight of the great hall, as its room places it: (x, y) of each soldier's
#: box centre, by wave. Printed for `data/encounters/goblin_encounter.ron`.
def muster_waves():
    _, w, h = ROOMS_AT[GREAT_HALL]
    floor = h - 64
    cy = floor - SOLDIER_BOX[1] // 2
    gy = floor - 192 - SOLDIER_BOX[1] // 2
    return [
        ("wave 1 — the guard and its dog", [(900, cy), (1200, cy)]),
        ("wave 2 — the galleries, and a pack", [(600, gy), (w - 600, gy), (1500, cy)]),
        ("wave 3 — the muster, and the Iron Trooper", [(700, cy), (w // 2, floor - 48 - SOLDIER_BOX[1] // 2), (1300, cy), (w - 600, gy)]),
    ]


def home_gate_yaml() -> str:
    """The gate in the central hub: a stone shelf high on the east side, the
    ring and its portal on it, the door of the portal, the switch."""
    shelf_y, x = 400, 1440
    top = shelf_y - 192
    return f"""# Written by specs/castle_godwin/make_specs.py. Change the script, not this file.
# The gate to the Evil dimension, on a shelf high in the central hub.
level_id: central_hub_main
entities:
  - type: Solid
    px: [1408, {shelf_y}]
    size: [384, 32]
    fields: {{name: evil_gate_shelf}}
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
      name: to the Evil dimension
      activation: Door
      target_room: {FOREST}
      target_zone: {EVIL_ZONE}
      bidirectional: false
  - type: Switch
    px: [{x + 240}, {shelf_y - 32}]
    size: [24, 32]
    fields:
      id: {SWITCH}
      name: {SWITCH}
      prompt: Power the gate
      action: ResetEncounter
      target_encounter: ""
"""


def home_gate_is_placed() -> bool:
    return HOME_ZONE in SANDBOX_LDTK.read_text()


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
    for name, mobs in muster_waves():
        print(name, mobs)
    if "--apply" in argv:
        # Each room's doors name the other rooms and the hub gate, so the
        # worlds are not whole until all are made: until then it is enough
        # that each room was written. The last check is the one that must pass.
        for path in made:
            code, out = run("area", "create", str(path), "--ldtk", str(EVIL_LDTK), "--replace-existing")
            if code != 0 and "wrote new level" not in out and "replaced level" not in out:
                return code
        if not home_gate_is_placed():
            code, _ = run("entity", "add", str(gate_path), "--ldtk", str(SANDBOX_LDTK), "--in-place")
            if code != 0:
                return code
        for world in (EVIL_LDTK, SANDBOX_LDTK):
            code, _ = run("validate", str(world))
            if code != 0:
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
