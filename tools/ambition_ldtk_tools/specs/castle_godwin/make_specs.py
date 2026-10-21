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
    # The throne room, and east of it the long gallery the Inugami chases you
    # down, and the hall at its end where you finish it (Jon, 2026-10-20).
    THRONE: (9920, 9600, 2304),
}

#: The boss of the castle: Wolfenstein, the monster of the regime with the head
#: of a mad wolf. Its pattern is `wolfenstein_boss` (`data/boss_profiles.ron`);
#: its placement id is its progress (`castle_godwin.wolfenstein`).
BOSS_NAME, BOSS_BRAIN, BOSS_ENCOUNTER = "Wolfenstein", "PhaseScript:wolfenstein_boss", "castle_godwin.wolfenstein"
#: His body box: big (Jon, 2026-10-18), and the serum grows him again.
BOSS_BOX = (84, 140)
#: The Inugami (Jon, 2026-10-20): his head, risen as a red spirit when he
#: falls. A boss of its own (`wolfenstein_inugami`), which sleeps until his
#: placement is cleared (its `wakes_after` names his BossSpawn) and rises
#: from his body; where it is placed only says where it waits.
INUGAMI_NAME, INUGAMI_BRAIN, INUGAMI_ENCOUNTER = "Inugami", "PhaseScript:wolfenstein_inugami", "castle_godwin.wolfenstein_inugami"
INUGAMI_BOX = (176, 212)

#: The soldier of the regime, and his box (183 cm).
SOLDIER = "npc_fascist_enforcer"
SOLDIER_BOX = (32, 72)
#: The regime's other fighters: the Iron Trooper with his chain gun (205 cm)
#: and the lab's Failed Prototypes, which come in packs. Each its box.
HEAVY, PROTOTYPE, HOUND = "npc_evil_heavy", "npc_evil_failed_prototype", "npc_evil_attack_hound"
#: The Risen Conscript, the castle's first and easiest enemy (a slow dead
#: soldier); the Shock Warden behind its electric shield; the Searchlight
#: Crawler on its spider legs.
ZOMBIE, WARDEN, CRAWLER = "npc_evil_zombie", "npc_shock_warden", "npc_searchlight_crawler"
BOX = {SOLDIER: SOLDIER_BOX, HEAVY: (36, 80), PROTOTYPE: (32, 40), HOUND: (52, 50), ZOMBIE: (30, 67), WARDEN: (40, 64),
       CRAWLER: (48, 34)}
KIND = {SOLDIER: "soldier", HEAVY: "heavy", PROTOTYPE: "prototype", HOUND: "hound", ZOMBIE: "zombie", WARDEN: "warden",
        CRAWLER: "crawler"}

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
    kind = KIND[character]
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{room}_{kind}_{n}", '
            f'brain: "combatant", character_id: "{character}", respawn: "OnRoomReenter" ) ),')


def pack(room, x, feet_y, size=3, gap=44):
    """A pack of Failed Prototypes, side by side from `x`."""
    return [soldier(room, i + 1, x + i * gap, feet_y, PROTOTYPE) for i in range(size)]


def heal(name, x, floor, amount=3):
    """A heart that heals `amount`, on `floor`."""
    return f'( type: "PickupSpawn", px: [{x}, {floor - 24}], size: [24, 24], fields: ( name: "{name}", kind: "health:{amount}" ) ),'


def shrine(rid, x, floor, name):
    """A shrine to rest at: it heals, and the castle starts again from it."""
    return f'( type: "ShrineSpawn", px: [{x}, {floor - 80}], size: [44, 80], fields: ( id: "{rid}_shrine", name: "{name}" ) ),'


def body_box(kind):
    """The body box of `kind`'s published sheet in frame pixels, (x, y, w, h)."""
    text = (SPRITES / f"{kind}_spritesheet.ron").read_text()
    m = re.search(r"body_pixel_bbox: Some\(\(x: (\d+), y: (\d+), w: (\d+), h: (\d+)\)\)", text)
    if not m:
        raise SystemExit(f"{kind}: no body_pixel_bbox in its published RON; publish the sprite first")
    return tuple(int(v) for v in m.groups())


def throne_middle(x):
    """The middle of the iron throne's art, its frame's left at `x`."""
    bx, _, bw, _ = body_box("castle_iron_throne")
    return round(x + (bx + bw / 2) / PROP_PX)


def standing(name, kind, x, floor):
    """Prop `kind` with the left of its FRAME at `x` and its art's bottom on
    `floor`; the prop's box is the art's body box in the frame."""
    bx, by, bw, bh = body_box(kind)
    fx, fy = x, floor - (by + bh) / PROP_PX
    return (f'( type: "Prop", px: [{round(fx + bx / PROP_PX)}, {round(fy + by / PROP_PX)}], '
            f'size: [{round(bw / PROP_PX)}, {round(bh / PROP_PX)}], fields: ( name: "{name}", kind: "{kind}" ) ),')


def room(rid, entities, label_text, theme, music, bg, cameras=None):
    """A room's spec. `cameras`: its camera zones; one over the whole room
    when none are given."""
    world_x, w, h = ROOMS_AT[rid]
    cameras = cameras or [f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),']
    body = "\n        ".join(entities + cameras + [
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


def castle(rid, entities, label_text, theme=CASTLE_THEME):
    return room(rid, entities, label_text, theme, CASTLE_MUSIC, CASTLE_BG)


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
        # The dead of the regime walk the road: the first fight of the
        # dimension, one, then two, then one at the door.
        soldier(rid, 1, 640, floor, ZOMBIE),
        soldier(rid, 2, 1060, floor, ZOMBIE),
        soldier(rid, 3, 1130, floor, ZOMBIE),
        soldier(rid, 4, entrance_x - 120, floor, ZOMBIE),
    ]
    return rid, room(rid, e, "The forest of the Evil dimension: the way to Castle Godwin.", FOREST_THEME, FOREST_MUSIC, FOREST_BG)


def gatehouse():
    """The first room inside: the dead at the gate, two soldiers, a dog and
    a crawler; a raised dais and a gallery."""
    rid = GATEHOUSE
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    e += [
        door(f"{rid}_west", "out to the forest", 64, floor, FOREST, f"{FOREST}_castle_door"),
        door(f"{rid}_east", "on to the barracks", w - 112, floor, BARRACKS, f"{BARRACKS}_west"),
        # The guard post at the gate, the posters, a searchlight at the far door.
        standing("guard_post", "castle_guard_post", 200, floor),
        standing("wanted_robot", "castle_wanted_poster", 420, floor - 150),
        standing("poster_obey", "castle_propaganda_poster_a", 830, floor - 170),
        standing("poster_papers", "castle_propaganda_poster_c", 1110, floor - 150),
        standing("searchlight_gate", "castle_searchlight", 1380, floor),
        solid("dais", 560, floor - 64, 192, 64),
        ledge("gallery_west", 880, floor - 160, 224),
        ledge("gallery_east", 1200, floor - 256, 192),
        # The dead at the gate, a soldier on the dais and one on the gallery,
        # a dog, and a crawler under the far searchlight.
        soldier(rid, 1, 380, floor, ZOMBIE),
        soldier(rid, 2, 470, floor, ZOMBIE),
        soldier(rid, 1, 640, floor - 64),
        soldier(rid, 2, 980, floor - 160),
        soldier(rid, 1, 1100, floor, HOUND),
        soldier(rid, 1, 1300, floor, CRAWLER),
    ]
    return rid, castle(rid, e, "Castle Godwin: the gatehouse.")


def barracks():
    """A long hall of bunks in tiers, and pillars that hang from the vault to
    hide behind. Soldiers, crawlers on the bunks, a Shock Warden in the
    middle, a pack and the Iron Trooper at the far end."""
    rid = BARRACKS
    _, w, h = ROOMS_AT[rid]
    floor, e = shell(rid)
    e += [
        door(f"{rid}_west", "back to the gatehouse", 64, floor, GATEHOUSE, f"{GATEHOUSE}_east"),
        door(f"{rid}_east", "up the tower", w - 112, floor, TOWER, f"{TOWER}_foot"),
    ]
    # The bunks: each bed's picture under its two one-way mattresses.
    for i, x in enumerate((360, 1000, 1640)):
        e += [standing(f"bunk_bed_{i}", "castle_bunk_bed", x, floor),
              ledge(f"bunk_{i}_low", x, floor - 112, 224), ledge(f"bunk_{i}_high", x, floor - 224, 224)]
    e += [
        standing("lockers_west", "castle_locker_row", 180, floor),
        standing("mess_table", "castle_mess_table", 1250, floor),
        standing("lockers_east", "castle_locker_row", 1900, floor),
        standing("poster_order", "castle_propaganda_poster_b", 880, floor - 260),
        standing("wanted_barracks", "castle_wanted_poster", 2060, floor - 150),
        standing("poster_obey_barracks", "castle_propaganda_poster_a", 2420, floor - 170),
    ]
    for i, x in enumerate((760, 1400, 2040)):
        e.append(solid(f"pillar_{i}", x, 48, 64, floor - 208))
    e += [
        soldier(rid, 1, 520, floor),
        soldier(rid, 2, 1100, floor - 224),
        soldier(rid, 1, 700, floor, HOUND),
        # Crawlers on the bunks; a heart under the mess table for the room's
        # middle.
        soldier(rid, 1, 1060, floor - 112, CRAWLER),
        soldier(rid, 2, 1700, floor - 224, CRAWLER),
        heal("barracks_heart", 1330, floor),
        # A Shock Warden holds the middle of the hall behind its shield.
        soldier(rid, 1, 1500, floor, WARDEN),
        soldier(rid, 3, 1760, floor - 112),
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
            # A soldier, then a crawler, up the climb.
            e.append(soldier(rid, n, x + 128, y, SOLDIER if n % 2 == 1 else CRAWLER))
        y -= 176
        k += 1
    # The lab on the floor of the tower: the vats the prototypes came out of,
    # the coils; the operating table on the top floor.
    e += [standing(f"vat_{i}", "castle_specimen_vat", x, floor) for i, x in enumerate((300, 400, 500))]
    e += [standing(f"coil_{i}", "castle_tesla_coil", x, floor) for i, x in enumerate((640, 860))]
    e.append(standing("operating_table", "castle_operating_table", w - 380, top_floor))
    # The top of the tower: a Shock Warden at the door, and a pack on the
    # first ledge. Past the warden, a shrine: the last rest before the muster
    # and the throne.
    e.append(soldier(rid, 1, w - 330, top_floor, WARDEN))
    e.append(shrine(rid, w - 190, top_floor, "the shrine of the lab tower"))
    e += pack(rid, 200, floor - 176)
    # The tower is the castle's lab: where the prototypes were made.
    return rid, castle(rid, e, "Castle Godwin: the lab tower.", theme="castle_godwin_lab")


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
        # The rally: the podium on the dais, searchlights at its sides, posters.
        standing("rally_podium", "castle_rally_podium", w // 2 - 64, floor - 48),
        standing("searchlight_west", "castle_searchlight", w // 2 - 260, floor),
        standing("searchlight_east", "castle_searchlight", w // 2 + 196, floor),
        standing("poster_hall_obey", "castle_propaganda_poster_a", 560, floor - 300),
        standing("poster_hall_order", "castle_propaganda_poster_b", w - 620, floor - 300),
        ledge("balcony", w // 2 - 192, floor - 352, 384),
    ]
    return rid, castle(rid, e, "Castle Godwin: the great hall.")


#: The throne room's music while he sleeps: the regime's dread.
THRONE_MUSIC = "oppressive_crush"

#: The throne stands near the door you come in by (Jon, 2026-10-18): the
#: left of its frame, and the coils at its sides.
THRONE_X = 560
COIL_WEST_X, COIL_EAST_X = 460, 760


#: The throne room and the way the Inugami chases you out of it (Jon,
#: 2026-10-20: the chase goes "through level geometry ... new geometry that is
#: part of the castle"; "we can make the castle bigger and better"). One room,
#: because a boss lives in its room: the throne room; east of it the long lab
#: gallery; the stair tower you climb; the battlements under the sky; and the
#: hall at the end where you finish it (`FINAL_W` wide, the last of the room).
#: Each section's left edge and its floor (the top of its floor).
THRONE_W, GALLERY_W, TOWER_W, WALK_W, FINAL_W = 1920, 2560, 960, 2560, 1600
LOW_FLOOR, HIGH_FLOOR = 2240, 880
#: The throne room and the gallery are as tall as the castle's other halls.
LOW_VAULT = LOW_FLOOR - 784
#: The top of the stair tower, and of the sky over the battlements and hall.
TOWER_VAULT, SKY = 400, 96


def camera(name, x, y, w, h, extra=""):
    return f'( type: "CameraZone", px: [{x}, {y}], size: [{w}, {h}], fields: ( id: "{name}", name: "{name}", mode: "Default"{extra} ) ),'


def throne():
    """The throne room: Wolfenstein's arena. His throne and its two coils
    stand near the door you come in by: the room is dark, and his intro
    starts when you come near enough to see him on it. He leaps back from
    the throne into the room, where the fight is: a flat floor, two low
    galleries to jump to over his sweeps, and a high one.

    East of it, behind an iron gate over a sill, the castle goes on: the
    Inugami breaks the gate open when it rises (`boss.engaged`) and chases
    you out. No soldier stands in the way: the chase is the castle's own.

    - The lab gallery: rubble and the lab's furniture to jump, ledges to get
      above its dives.
    - The stair tower: you climb it ledge over ledge, the head under you.
    - The battlements: the castle's roof under the sky, a broken walkway with
      a catwalk under its gaps, the head over the parapet.
    - The hall at the end of the battlements: where you finish it.
    """
    rid = THRONE
    _, w, h = ROOMS_AT[rid]
    gallery_x = THRONE_W
    tower_x = gallery_x + GALLERY_W
    walk_x = tower_x + TOWER_W
    final_x = walk_x + WALK_W
    assert final_x + FINAL_W == w, (final_x, FINAL_W, w)
    lo, hi = LOW_FLOOR, HIGH_FLOOR
    bw, bh = BOSS_BOX
    sill = 40
    e = [
        f'( type: "PlayerStart", px: [128, {lo - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        # The masonry: the floor under everything low; over the throne room
        # and the gallery; under the battlements and the hall.
        solid("floor", 0, lo, tower_x + TOWER_W, h - lo),
        solid("masonry_over_halls", 0, 0, tower_x, LOW_VAULT),
        solid("masonry_under_roof", walk_x, hi, w - walk_x, h - hi),
        solid("west_wall", 0, LOW_VAULT, WALL, lo - LOW_VAULT),
        # ── The throne room ──
        door(f"{rid}_west", "back to the great hall", 64, lo, GREAT_HALL, f"{GREAT_HALL}_east"),
        ledge("gallery_west", 960, lo - 160, 256),
        ledge("gallery_east", THRONE_W - 480, lo - 160, 256),
        ledge("high_gallery", 1240, lo - 320, 320),
        standing("coil_throne_west", "castle_tesla_coil", COIL_WEST_X, lo),
        standing("iron_throne", "castle_iron_throne", THRONE_X, lo),
        standing("coil_throne_east", "castle_tesla_coil", COIL_EAST_X, lo),
        standing("wanted_throne", "castle_wanted_poster", THRONE_W - 300, lo - 150),
        # He starts on his throne, dead: his box on the floor in front of
        # its middle; his conductor lifts him into the seat.
        (f'( type: "BossSpawn", ref: "wolfenstein", px: [{throne_middle(THRONE_X) - bw // 2}, {lo - bh}], size: [{bw}, {bh}], '
         f'fields: ( name: "{BOSS_NAME}", brain: "{BOSS_BRAIN}", encounter_id: "{BOSS_ENCOUNTER}" ) ),'),
        # His head, which rises from him when he falls.
        (f'( type: "BossSpawn", px: [{throne_middle(THRONE_X) - INUGAMI_BOX[0] // 2}, {lo - bh - INUGAMI_BOX[1] - 40}], '
         f'size: [{INUGAMI_BOX[0]}, {INUGAMI_BOX[1]}], fields: ( name: "{INUGAMI_NAME}", brain: "{INUGAMI_BRAIN}", '
         f'encounter_id: "{INUGAMI_ENCOUNTER}", wakes_after: "wolfenstein" ) ),'),
        # The east end of the throne room: a sill (his fight measures it as
        # his wall) and the iron gate over it, which stands until the head
        # rises and breaks it open.
        solid("gallery_sill", gallery_x - WALL, lo - sill, 2 * WALL, sill),
        (f'( type: "LockWall", px: [{gallery_x - WALL}, {LOW_VAULT}], size: [{2 * WALL}, {lo - sill - LOW_VAULT}], fields: ( id: "{rid}_gallery_gate", '
         f'name: "the gate of the long gallery", gated_by: "boss.engaged {INUGAMI_ENCOUNTER}", look: "temple_gate_iron" ) ),'),
        # ── The lab gallery ──
        solid("rubble_a", gallery_x + 420, lo - 48, 96, 48),
        standing("gallery_vat_a", "castle_specimen_vat", gallery_x + 660, lo),
        ledge("gallery_ledge_a", gallery_x + 860, lo - 176, 288),
        standing("gallery_table", "castle_operating_table", gallery_x + 1100, lo),
        solid("rubble_b", gallery_x + 1400, lo - 64, 128, 64),
        standing("gallery_poster_a", "castle_propaganda_poster_c", gallery_x + 1640, lo - 300),
        solid("gallery_step", gallery_x + 1800, lo - 32, 520, 32),
        standing("gallery_vat_b", "castle_specimen_vat", gallery_x + 1940, lo - 32),
        ledge("gallery_ledge_b", gallery_x + 2060, lo - 208, 288),
        # ── The stair tower: in at the bottom from the gallery, out at the top
        # onto the battlements ──
        solid("tower_vault", tower_x, TOWER_VAULT - 48, TOWER_W, 48),
        solid("tower_west_wall", tower_x - WALL, TOWER_VAULT, WALL, LOW_VAULT - TOWER_VAULT),
        solid("tower_east_wall", tower_x + TOWER_W, TOWER_VAULT, WALL, hi - 176 - TOWER_VAULT),
        solid("tower_east_base", tower_x + TOWER_W, hi, WALL, lo - hi),
    ]
    # The stair: ledges from side to side, each a jump over the last.
    rises = (lo - hi) // 176
    for k in range(1, rises + 1):
        y = lo - 176 * k
        x = tower_x + (64 if k % 2 else TOWER_W - 64 - 352)
        e.append(ledge(f"tower_stair_{k}", x, y, 352))
    e += [
        standing("tower_coil", "castle_tesla_coil", tower_x + TOWER_W // 2 - 28, lo),
        standing("tower_poster", "castle_propaganda_poster_b", tower_x + TOWER_W // 2 - 40, lo - 900),
        # The landing at the top, out to the battlements.
        solid("tower_landing", tower_x + TOWER_W - 352, hi, 352, 32),
        # ── The battlements: the roof of the castle, under the sky ──
        solid("parapet_low", walk_x + 200, hi - 32, 64, 32),
        solid("parapet_mid", walk_x + 1120, hi - 32, 64, 32),
        standing("walk_searchlight_a", "castle_searchlight", walk_x + 560, hi),
        # A gap in the walkway, with a catwalk under it and a way back up.
        solid("walk_gap_catwalk", walk_x + 760, hi + 240, 400, 32),
        ledge("walk_gap_step", walk_x + 1000, hi + 112, 160),
        ledge("walk_ledge_a", walk_x + 1360, hi - 176, 288),
        solid("parapet_high", walk_x + 1760, hi - 48, 96, 48),
        standing("walk_searchlight_b", "castle_searchlight", walk_x + 2040, hi),
        ledge("walk_ledge_b", walk_x + 2180, hi - 208, 256),
        # ── The hall at the end: where you finish it ──
        solid("final_east_wall", w - WALL, SKY, WALL, hi - SKY),
        solid("final_vault", final_x, SKY - 48, FINAL_W, 48),
        standing("final_coil_west", "castle_tesla_coil", final_x + 120, hi),
        ledge("final_gallery_west", final_x + 260, hi - 160, 256),
        ledge("final_gallery_east", w - 516, hi - 160, 256),
        ledge("final_high", final_x + FINAL_W // 2 - 160, hi - 320, 320),
        standing("final_coil_east", "castle_tesla_coil", w - 180, hi),
        standing("final_poster", "castle_propaganda_poster_a", final_x + FINAL_W // 2 - 40, hi - 420),
    ]
    # Cut the gap out of the battlements' masonry: the walkway is two pieces.
    e = [x for x in e if '"masonry_under_roof"' not in x]
    e += [
        solid("roof_west", walk_x, hi, 760, h - hi),
        solid("roof_east", walk_x + 1160, hi, w - walk_x - 1160, h - hi),
        solid("roof_gap_floor", walk_x + 760, hi + 272, 400, h - hi - 272),
    ]
    # Each section frames itself; the long ones look ahead of you (you run
    # east, and up the tower).
    cameras = [
        camera(f"{rid}_camera", 0, LOW_VAULT, THRONE_W, h - LOW_VAULT, ', clamp_mode: "zone_bounds"'),
        camera(f"{rid}_gallery_camera", gallery_x, LOW_VAULT, GALLERY_W, h - LOW_VAULT, ", zoom: 0.85, target_offset_x: 160.0"),
        camera(f"{rid}_tower_camera", tower_x, TOWER_VAULT - 48, TOWER_W, h - TOWER_VAULT + 48, ', zoom: 0.9, target_offset_y: -120.0, clamp_mode: "zone_bounds"'),
        camera(f"{rid}_walk_camera", walk_x, 0, WALK_W, hi + 400, ", zoom: 0.85, target_offset_x: 160.0"),
        camera(f"{rid}_final_camera", final_x, 0, FINAL_W, hi + 64, ', clamp_mode: "zone_bounds"'),
    ]
    text = room(rid, e, "Castle Godwin: the throne room of Wolfenstein, and the way out of it.", "castle_godwin_throne", THRONE_MUSIC, CASTLE_BG, cameras)
    return rid, text


ROOMS = (forest, gatehouse, barracks, tower, great_hall, throne)

#: The fight of the great hall: its waves are authored in
#: `data/encounters/goblin_encounter.ron` (`castle_godwin_muster`), in room
#: coordinates (box centres; the floor is at y = 832).


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
