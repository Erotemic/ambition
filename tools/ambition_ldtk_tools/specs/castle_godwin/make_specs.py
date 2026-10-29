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

The forest's gate is a dialed ring of the gate network (address CITADEL,
`data/gate_network.ron`), with its terminal beside it: a traveller dials home
(LEGAL is written on the terminal) and walks in. The central hub's public
gate dials it (`specs/gate_network/make_gates.py`).

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

sys.path.insert(0, str(HERE.parent / "gate_network"))
import gates  # noqa: E402

EVIL_LDTK = default_worlds_dir() / "evil.ldtk"
SANDBOX_LDTK = default_worlds_dir() / "sandbox.ldtk"

FOREST_THEME, CASTLE_THEME = "evil_forest", "castle_godwin"
#: The gatehouse's own music (set in the editor, 2026-10).
GATEHOUSE_MUSIC = "tritone_decree"
#: The old drain's door in the gatehouse: under its west gallery, at the
#: bottom of a pit in its floor (`GATEHOUSE_GROUND` thick).
PASSAGE_DOOR_X = 1000
GATEHOUSE_GROUND = 128
FOREST_MUSIC, CASTLE_MUSIC = "oppressive_crush", "military_iron_resolve"
FOREST_BG, CASTLE_BG = "#1A0A0C", "#0E0E14"
WALL = 16

#: The ring of the forest: its zone (`data/gate_portals.ron`,
#: `data/gate_network.ron`), its props' label and its terminal.
EVIL_ZONE = "evil_gate_return_zone"
EVIL_GATE = "Evil Gate (Evil)"
EVIL_TERMINAL = "evil_gate_terminal"

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
    GATEHOUSE: (2000, 1600, 832),
    BARRACKS: (3792, 2560, 768),
    TOWER: (6544, 1024, 1792),
    GREAT_HALL: (7760, 1920, 896),
    # The throne room under the great hall's east end, and the way the
    # Inugami chases you out of it: down a drop, then west under the whole
    # castle to the hall under the gatehouse, where you finish it (Jon,
    # 2026-10-24). Under the castle's other rooms, so it starts at the
    # gatehouse's left.
    THRONE: (2000, 10480, 2688),
}
#: How far down the world a room is: 0 unless given. The throne room is
#: under the tower's foot (1792), so it overlaps no room.
ROOMS_Y = {THRONE: 1856}

#: The boss of the castle: Wolfenstein, the monster of the regime with the head
#: of a mad wolf. Its pattern is `wolfenstein_boss` (`data/boss_profiles.ron`);
#: its placement id is its progress (`castle_godwin.wolfenstein`).
BOSS_NAME, BOSS_BRAIN, BOSS_ENCOUNTER = "Wolfenstein", "PhaseScript:wolfenstein_boss", "castle_godwin.wolfenstein"
#: His body box: big (Jon, 2026-10-18), and the serum grows him again.
BOSS_BOX = (84, 140)
#: The Inugami (Jon, 2026-10-20): his head, risen as a red spirit when he
#: falls. A boss of its own (`wolfenstein_inugami`), which sleeps until his
#: placement is cleared (its `wakes_after` names his BossSpawn) and rises
#: from his body; where it is placed only says where it waits. Its name is
#: his (Jon, 2026-10-24: "name should be 'Inugami Wolfenstein'").
INUGAMI_NAME, INUGAMI_BRAIN, INUGAMI_ENCOUNTER = "Inugami Wolfenstein", "PhaseScript:wolfenstein_inugami", "castle_godwin.wolfenstein_inugami"
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
#: The Spectral Wolf: a dog of the regime, dead and risen, that fades between
#: its attacks.
WOLF = "npc_evil_spectral_wolf"
BOX = {SOLDIER: SOLDIER_BOX, HEAVY: (36, 80), PROTOTYPE: (32, 40), HOUND: (52, 50), ZOMBIE: (30, 67), WARDEN: (40, 64),
       CRAWLER: (48, 34), WOLF: (56, 56)}
KIND = {SOLDIER: "soldier", HEAVY: "heavy", PROTOTYPE: "prototype", HOUND: "hound", ZOMBIE: "zombie", WARDEN: "warden",
        CRAWLER: "crawler", WOLF: "wolf"}

SPRITES = TOOLS.parents[1] / "game/ambition_content_library/generated/runtime/sprites"
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
    world_y = ROOMS_Y.get(rid, 0)
    cameras = cameras or [f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),']
    body = "\n        ".join(entities + cameras + [
        label(f"{rid}_label", 70, 40, label_text, 900),
    ])
    return f"""// Written by specs/castle_godwin/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
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


def castle(rid, entities, label_text, theme=CASTLE_THEME, music=CASTLE_MUSIC):
    return room(rid, entities, label_text, theme, music, CASTLE_BG)


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
        *gates.as_ron(gates.ring(EVIL_GATE, EVIL_ZONE, 64, floor, "the gate")),
        *gates.as_ron(gates.terminal(EVIL_GATE, EVIL_TERMINAL, 364, floor)),
        # Scenery, furthest back first: the castle's front, the thorns, the sign.
        standing("castle_godwin_front", "castle_godwin_entrance", entrance_x, floor),
        standing("thorns_gate", "evil_thorn_bush", 430, floor),
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
        # One spectral wolf on the road, to try it (Jon, 2026-10-22: "We will
        # put one in the outside of the castle so we can test it, but I want
        # to use it more properly later").
        soldier(rid, 1, 820, floor, WOLF),
    ]
    return rid, room(rid, e, "The forest of the Evil dimension: the way to Castle Godwin.", FOREST_THEME, FOREST_MUSIC, FOREST_BG)


def gatehouse():
    """The first room inside: the dead at the gate, two soldiers, a dog and
    a crawler; a raised dais and a gallery."""
    rid = GATEHOUSE
    _, w, h = ROOMS_AT[rid]
    # Its floor is thick: the old drain's mouth is a pit in it.
    floor, e = shell(rid, ground=GATEHOUSE_GROUND)
    pit_x, pit_w, pit_d = PASSAGE_DOOR_X - 24, 96, 96
    e = [x for x in e if '"floor"' not in x]
    e += [
        solid("floor_west", 0, floor, pit_x, h - floor),
        solid("drain_pit_floor", pit_x, floor + pit_d, pit_w, h - floor - pit_d),
        solid("floor_east", pit_x + pit_w, floor, w - pit_x - pit_w, h - floor),
    ]
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
        # The old drain down under the castle: its mouth is a pit in the
        # floor, shut by a block of cracked stone flush with the floor until
        # the lever in the hall where the Inugami dies breaks it (Jon,
        # 2026-10-24: "you can open a secret passage back"; 2026-10-25: "Do
        # a cracked stone look"). Then its door is at the bottom of the pit.
        door(f"{rid}_passage", "down the old drain", PASSAGE_DOOR_X, floor + pit_d, THRONE, f"{THRONE}_passage"),
        standing("drain_mouth_rubble", "castle_drain_breach", pit_x + 12, floor + pit_d),
        lock_wall(f"{rid}_drain_mouth", pit_x, floor, pit_w, pit_d, f"world.switch_on {DRAIN_LEVER}", "the cracked floor over the old drain",
                  look="castle_cracked_floor"),
    ]
    return rid, castle(rid, e, "Castle Godwin: the gatehouse.", music=GATEHOUSE_MUSIC)


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
#: left of its frame, and the coils at its sides, from the arena's left.
THRONE_X = 560
COIL_WEST_X, COIL_EAST_X = 460, 760


#: The throne room and the way the Inugami chases you out of it. One room,
#: because a boss lives in its room (Jon, 2026-10-24: "let's make it a
#: straight level that goes down instead of up. And have it sort of loop back
#: topologically under the castle so when the fight is over you are close to
#: the start of the level and you can open a secret passage back"):
#:
#: and then (2026-10-25): "yes loop back. so right, down, then left":
#:
#: - the throne room, Wolfenstein's arena, at the east end;
#: - the way east (`EAST_W`): the Inugami crashes through the arena's east
#:   wall, and you run east from it along a hall as high as the arena;
#: - the drop: at its end you fall down a shaft (`SHAFT_W`);
#: - the run: a straight way west under it, the arena and the whole castle, a
#:   step down every `RUN_STEP` (`RUN_STEPS` of them), `RUN_CLEAR` high: the
#:   head flies it after you with room over it, and nothing hangs in its way;
#: - the hall at its west end (`FINAL_W`), under the gatehouse, where Kagero
#:   breaks its ward and you finish it. A lever there opens the old drain up
#:   to the gatehouse.
THRONE_W, EAST_W, SHAFT_W, FINAL_W = 1920, 1920, 640, 1600
RUN_STEPS, RUN_DROP, RUN_CLEAR = 10, 96, 640
#: The arena: its vault, and the top of its floor (784 under the vault's
#: underside, as the castle's other halls; Wolfenstein's lightning measures it).
ARENA_VAULT, ARENA_FLOOR = 48, 832
#: The arena's floor is this thick over the run.
ARENA_SLAB = 192


def camera(name, x, y, w, h, extra=""):
    return f'( type: "CameraZone", px: [{x}, {y}], size: [{w}, {h}], fields: ( id: "{name}", name: "{name}", mode: "Default"{extra} ) ),'


def lock_wall(rid, x, y, w, h, gated_by, name, look="temple_gate_iron"):
    return (f'( type: "LockWall", px: [{x}, {y}], size: [{w}, {h}], fields: ( id: "{rid}", name: "{name}", '
            f'gated_by: "{gated_by}", look: "{look}" ) ),')


def poster(name, kind, x, top):
    """A poster, the left of its art at `x` and its top at `top`: on
    stone (Jon, 2026-10-24: "The poster should probably be on solid wall ...
    Right now they float in the air")."""
    bx, by, bw, bh = body_box(kind)
    return (f'( type: "Prop", px: [{x}, {top}], size: [{round(bw / PROP_PX)}, {round(bh / PROP_PX)}], '
            f'fields: ( name: "{name}", kind: "{kind}" ) ),')


#: The lever of the old drain: a mechanism the save keeps
#: (`world.switch_on`), so the drain stays open.
DRAIN_LEVER = "castle_godwin_drain_lever"
#: The cracked wall across it: as wide as its look (`castle_cracked_wall`,
#: 64 x 160), so the stone is not stretched.
DRAIN_WALL_W = 64


def throne():
    """The throne room: Wolfenstein's arena. His throne and its two coils
    stand near the door you come in by: the room is dark, and his intro
    starts when you come near enough to see him on it. He leaps back from
    the throne into the room, where the fight is: a flat floor, two low
    galleries to jump to over his sweeps, and a high one.

    The arena's east wall is an iron gate over a sill: it opens when the
    Inugami is past its first phase (`boss.past_first_phase`): the head
    crashes through it, and it chases you out (Jon, 2026-10-22: "You
    shouldn't be able to move into a chase zone until you damage him enough
    in phase 1"). No soldier stands in the way: the chase is the castle's own.

    - The way east: a hall as high as the arena, the lab's junk to jump.
    - The drop: at its end a shaft goes down under it.
    - The run: straight west under the castle, a step down at a time; the
      lab's junk to jump, coils, and the regime's posters on its barricades.
    - The hall at the end: where Kagero breaks its ward and you finish it.
      Its west wall holds the old drain up to the gatehouse, behind a wall of
      cracked stone that its lever breaks; the lever is caged until the
      Inugami is dead.
    """
    rid = THRONE
    _, w, h = ROOMS_AT[rid]
    arena_x = w - SHAFT_W - EAST_W - THRONE_W
    gate_x = arena_x + THRONE_W
    shaft_x = gate_x + EAST_W
    step = (w - FINAL_W) // RUN_STEPS
    assert FINAL_W + step * RUN_STEPS == w, (w, step)
    lo = ARENA_FLOOR
    slab_bottom = lo + ARENA_SLAB
    run_floor0 = slab_bottom + RUN_CLEAR
    floors = [run_floor0 + RUN_DROP * k for k in range(RUN_STEPS)]
    final_floor = floors[-1] + RUN_DROP
    final_vault = final_floor - (ARENA_FLOOR - ARENA_VAULT)
    assert final_floor + 64 == h, (final_floor, h)
    bw, bh = BOSS_BOX
    sill = 40
    ax = lambda x: arena_x + x  # noqa: E731
    e = [
        f'( type: "PlayerStart", px: [{ax(128)}, {lo - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        # ── The throne room ──
        solid("vault", arena_x, 0, w - arena_x, ARENA_VAULT),
        solid("arena_floor", arena_x, lo, THRONE_W + EAST_W, ARENA_SLAB),
        solid("arena_west_wall", arena_x, ARENA_VAULT, WALL, lo - ARENA_VAULT),
        door(f"{rid}_west", "back to the great hall", ax(64), lo, GREAT_HALL, f"{GREAT_HALL}_east"),
        ledge("gallery_west", ax(960), lo - 160, 256),
        ledge("gallery_east", ax(THRONE_W - 480), lo - 160, 256),
        ledge("high_gallery", ax(1240), lo - 320, 320),
        standing("coil_throne_west", "castle_tesla_coil", ax(COIL_WEST_X), lo),
        standing("iron_throne", "castle_iron_throne", ax(THRONE_X), lo),
        standing("coil_throne_east", "castle_tesla_coil", ax(COIL_EAST_X), lo),
        # He starts on his throne, dead: his box on the floor in front of
        # its middle; his conductor lifts him into the seat.
        (f'( type: "BossSpawn", ref: "wolfenstein", px: [{throne_middle(ax(THRONE_X)) - bw // 2}, {lo - bh}], size: [{bw}, {bh}], '
         f'fields: ( name: "{BOSS_NAME}", brain: "{BOSS_BRAIN}", encounter_id: "{BOSS_ENCOUNTER}" ) ),'),
        # His head, which rises from him when he falls.
        (f'( type: "BossSpawn", px: [{throne_middle(ax(THRONE_X)) - INUGAMI_BOX[0] // 2}, {lo - bh - INUGAMI_BOX[1] - 40}], '
         f'size: [{INUGAMI_BOX[0]}, {INUGAMI_BOX[1]}], fields: ( name: "{INUGAMI_NAME}", brain: "{INUGAMI_BRAIN}", '
         f'encounter_id: "{INUGAMI_ENCOUNTER}", wakes_after: "wolfenstein" ) ),'),
        # The east end of the throne room: a sill (his fight measures it as
        # his wall) and the iron gate over it, which stands until the head
        # breaks it open.
        solid("crash_sill", gate_x - WALL, lo - sill, 2 * WALL, sill),
        lock_wall(f"{rid}_crash_gate", gate_x - WALL, ARENA_VAULT, 2 * WALL, lo - sill - ARENA_VAULT,
                  f"boss.past_first_phase {INUGAMI_ENCOUNTER}", "the east gate of the throne room"),
        # ── The way east: as high as the arena; nothing over the head, and
        # nothing on the floor higher than the run's barricades ──
        standing("east_vat", "castle_specimen_vat", gate_x + 360, lo),
        solid("east_rubble", gate_x + 640, lo - 40, 96, 40),
        standing("east_coil", "castle_tesla_coil", gate_x + 980, lo),
        solid("east_barricade", gate_x + 1300, lo - 80, 112, 80),
        poster("east_poster", "castle_propaganda_poster_c", gate_x + 1300 + 33, lo - 76),
        standing("east_table", "castle_operating_table", gate_x + 1560, lo),
        # ── The drop: a shaft down at its end ──
        solid("shaft_east_wall", w - WALL, ARENA_VAULT, WALL, floors[0] - ARENA_VAULT),
    ]
    # ── The run: west under the castle, a step down at a time ──
    for k, floor in enumerate(floors):
        x1 = w - step * k
        x0 = x1 - step
        e.append(solid(f"run_floor_{k}", x0, floor, step, h - floor))
        ceiling = floor - RUN_CLEAR
        # Its vault: the castle's masonry over it, or the arena's floor; the
        # shaft is open over its first step.
        if x0 < arena_x:
            e.append(solid(f"run_vault_{k}", x0, 0, min(x1, arena_x) - x0, ceiling))
        if x1 > arena_x and ceiling > slab_bottom:
            e.append(solid(f"run_vault_{k}_under_arena", max(x0, arena_x), slab_bottom, min(x1, shaft_x) - max(x0, arena_x),
                           ceiling - slab_bottom))
    # What the run passes, by its step and how far along the step from its
    # east end: barricades of the regime with its posters on them, the lab's
    # junk, and its coils (the castle's lightning: Jon, 2026-10-24).
    def on_step(k, along):
        return w - step * k - along

    barricade_h, barricade_w = 80, 112
    run_posters = ["castle_propaganda_poster_a", "castle_wanted_poster", "castle_propaganda_poster_b", "castle_propaganda_poster_c"]
    for n, k in enumerate((2, 4, 6, 8)):
        x = on_step(k, 520)
        e.append(solid(f"run_barricade_{n}", x, floors[k] - barricade_h, barricade_w, barricade_h))
        kind = run_posters[n]
        _, _, pw, ph = body_box(kind)
        pw, ph = round(pw / PROP_PX), round(ph / PROP_PX)
        e.append(poster(f"run_poster_{n}", kind, x + (barricade_w - pw) // 2, floors[k] - barricade_h + max(4, (barricade_h - ph) // 2)))
    for n, k in enumerate((1, 3, 5, 7, 9)):
        e.append(standing(f"run_coil_{n}", "castle_tesla_coil", on_step(k, 300), floors[k]))
    e += [
        solid("run_rubble_a", on_step(1, 700), floors[1] - 40, 96, 40),
        standing("run_vat", "castle_specimen_vat", on_step(3, 660), floors[3]),
        standing("run_table", "castle_operating_table", on_step(5, 700), floors[5]),
        solid("run_rubble_b", on_step(7, 720), floors[7] - 48, 128, 48),
        standing("run_searchlight", "castle_searchlight", on_step(9, 640), floors[9]),
    ]
    # ── The hall at the end: where you finish it ──
    lintel_x, lintel_w, opening = WALL, 192, 160
    e += [
        solid("final_floor", 0, final_floor, FINAL_W, h - final_floor),
        solid("final_vault", 0, 0, FINAL_W, final_vault),
        solid("final_west_wall", 0, final_vault, WALL, final_floor - final_vault),
        standing("final_coil_west", "castle_tesla_coil", 380, final_floor),
        ledge("final_gallery_west", 300, final_floor - 160, 256),
        ledge("final_gallery_east", FINAL_W - 516, final_floor - 160, 256),
        ledge("final_high", FINAL_W // 2 - 160, final_floor - 320, 320),
        standing("final_coil_east", "castle_tesla_coil", FINAL_W - 220, final_floor),
        # The old drain: an opening in the west wall under a stone lintel, its
        # door at the back, and a wall of cracked stone across it that the
        # lever breaks (Jon, 2026-10-25: "Do a cracked stone look"); the
        # breach behind it shows when it is broken through.
        solid("drain_lintel", lintel_x, final_vault, lintel_w, final_floor - opening - final_vault),
        door(f"{rid}_passage", "up the old drain", lintel_x + 40, final_floor, GATEHOUSE, f"{GATEHOUSE}_passage"),
        standing("drain_breach", "castle_drain_breach", lintel_x + lintel_w - DRAIN_WALL_W - 4, final_floor),
        lock_wall(f"{rid}_drain_wall", lintel_x + lintel_w - DRAIN_WALL_W, final_floor - opening, DRAIN_WALL_W, opening,
                  f"world.switch_on {DRAIN_LEVER}", "the cracked wall of the old drain", look="castle_cracked_wall"),
        lever(DRAIN_LEVER, "the lever of the old drain", "Pull the lever", 300, final_floor, action="ToggleFlag"),
        # The lever is walled into a niche of the drain's cracked stone until
        # the Inugami is dead; then the stone breaks open.
        lock_wall(f"{rid}_lever_cage", 284, final_floor - 64, 56, 64, f"boss.cleared {INUGAMI_ENCOUNTER}", "the cage of the lever",
                  look="castle_cracked_niche"),
        # The regime's posters on the lintel.
        poster("final_poster_a", "castle_propaganda_poster_a", lintel_x + 24, final_floor - opening - 96),
        poster("final_poster_b", "castle_wanted_poster", lintel_x + 112, final_floor - opening - 88),
    ]
    # Each section frames itself, so the head behind you is in the picture
    # (Jon, 2026-10-24: "the camera should let the player go much further
    # right, so we can have the inugami head on screen"). On the way east you
    # are held right of the middle, the head on your left; on the run west
    # you are held left of it, the head on your right. The drop between them
    # is framed on you: the hand-over from one side to the other is across
    # it, eased by position. A zoom under 1 is 1, and 1 shows some 490
    # across: the chase is framed as wide as his arena's fight (about 800
    # across), you 220 off the middle and low in it, so the head over you is
    # in it too (measured 2026-10-24: at zoom 1 and 420 you were out of view,
    # and the head's top was cut).
    cameras = [
        camera(f"{rid}_camera", arena_x, 0, THRONE_W, slab_bottom, ', clamp_mode: "zone_bounds"'),
        camera(f"{rid}_east_camera", gate_x, 0, EAST_W, slab_bottom, ", zoom: 1.6, target_offset_x: -220.0, target_offset_y: -110.0"),
        camera(f"{rid}_drop_camera", shaft_x, 0, SHAFT_W, floors[0], ", zoom: 1.6, target_offset_y: 120.0"),
        camera(f"{rid}_run_camera", FINAL_W, slab_bottom, shaft_x - FINAL_W, h - slab_bottom, ", zoom: 1.6, target_offset_x: 220.0, target_offset_y: -110.0"),
        # Not clamped to itself: its east side is open to the run, and a
        # camera that came in at the run's offset was stopped at its edge.
        camera(f"{rid}_final_camera", 0, final_vault - 48, FINAL_W, h - final_vault + 48, ', zoom: 1.6, target_offset_y: -60.0, clamp_mode: "room_bounds"'),
    ]
    text = room(rid, e, "Castle Godwin: the throne room of Wolfenstein, and the way out of it under the castle.", "castle_godwin_throne",
                THRONE_MUSIC, CASTLE_BG, cameras)
    return rid, text


ROOMS = (forest, gatehouse, barracks, tower, great_hall, throne)

#: The fight of the great hall: its waves are authored in
#: `data/encounters/goblin_encounter.ron` (`castle_godwin_muster`), in room
#: coordinates (box centres; the floor is at y = 832).


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
    if "--apply" in argv:
        # Each room's doors name the other rooms, so the
        # worlds are not whole until all are made: until then it is enough
        # that each room was written. The last check is the one that must pass.
        for path in made:
            code, out = run("area", "create", str(path), "--ldtk", str(EVIL_LDTK), "--replace-existing")
            if code != 0 and "wrote new level" not in out and "replaced level" not in out:
                return code
        for world in (EVIL_LDTK, SANDBOX_LDTK):
            code, _ = run("validate", str(world))
            if code != 0:
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
