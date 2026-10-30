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

- `math_gate_hall`: the gate hall, a vaulted undercroft under the street
  (theme `math_undercroft`). Travellers to and from the dimension arrive by
  the gate, which stands in its cradle (`gate_cradle`: a plinth with stairs,
  walked on zero-speed platforms) in the middle of the floor. A door at its
  east end goes up to the street. Townsfolk wander (`townsfolk_roam`).
- `math_town_street`: the street at the foot of the terraces. Ledges climb
  the cliff to the first terrace and on to the second.
- Inside three of its houses (Jon, 2026-10-22: "Enterable interiors"; theme
  `math_interior`, furniture `targets/props/math_town_interiors.py`):
  `math_cafe` on the first terrace, `math_surveyors_office` on the second,
  `math_observatory` on the summit. Each is entered by the door its house
  front paints; the room's door is drawn over the painted one, as tall.

The hall's gate is a dialed ring of the gate network (address TANGENT,
`data/gate_network.ron`), with its terminal beside the cradle; LEGAL, the
way home, is written on it.

And in the Home dimension, in `gate_stack_lower` (`intro.ldtk`, the gate
station by the creator's lab), Ring 7: a dialed ring tuned to the Math line
only, powered through its breaker (`math_gate_switch`), with its own
terminal. `specs/gate_network/make_gates.py` turns an old Gate 7 into it.
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
sys.path.insert(0, str(HERE.parent))
import gates  # noqa: E402

MATH_LDTK = default_worlds_dir() / "math.ldtk"
INTRO_LDTK = default_worlds_dir() / "intro.ldtk"

BIOME = "math_town"
MUSIC = "math_town_day"
BG = "#BFD7EE"
WALL = 16

#: The ids the engine side names (`ambition_content::math_town`).
SWITCH = "math_gate_switch"
HOME_ZONE = "math_gate_home_zone"
MATH_ZONE = "math_gate_return_zone"
HOME_RING, HOME_PORTAL = "Math Gate Ring (Home)", "Math Gate Portal (Home)"
MATH_RING, MATH_PORTAL = "Math Gate Ring (Math)", "Math Gate Portal (Math)"
HALL, STREET = "math_gate_hall", "math_town_street"
ZENO = "math_zeno_aqueduct"
LOBBY, CORRIDOR, LIFT = "math_hotel_lobby", "math_hotel_corridor", "math_hotel_lift"
#: The hotel's ballroom (a game show of three doors) and the closets behind
#: its doors: a goat, a car, a goat.
BALLROOM = "math_hotel_ballroom"
CLOSETS = ("math_hotel_door_1", "math_hotel_door_2", "math_hotel_door_3")
#: Out of town east of the summit (Jon, 2026-10-30: "out of town zones with
#: enemies"): the Fractal Woods.
WOODS = "math_fractal_woods"
CAFE, OFFICE, OBSERVATORY = "math_cafe", "math_surveyors_office", "math_observatory"

#: Each room: where it is in the world, and its size.
#: Where the buildings' insides are in the world (y).
INSIDES_Y = 2400

ROOMS_AT = {
    HALL: (0, 1600, 768),
    STREET: (1712, 3200, 1344),
    # Each room is about as tall as the camera shows over a body on its floor.
    CAFE: (5100, 960, 448),
    OFFICE: (6200, 1120, 448),
    OBSERVATORY: (7500, 800, 704),
    # Out of town, west of the street along the aqueduct (Jon, 2026-10-30:
    # "out of town zones with enemies").
    ZENO: (1712 - 3200, 3200, 896),
    # Hylbert's Hotel (Jon, 2026-10-30): its lobby, and the corridor east of
    # it, its floor level with the lobby's.
    LOBBY: (8500, 1280, 576),
    CORRIDOR: (8500 + 1280, 1600, 448),
    # The lift's shaft, behind the lobby's lift gate (a door: it is inside).
    LIFT: (11600, 640, 1600),
    # The ballroom, through a door west of the front desk, and its closets.
    BALLROOM: (12400, 1280, 576),
    CLOSETS[0]: (13760, 480, 320),
    CLOSETS[1]: (14320, 480, 320),
    CLOSETS[2]: (14880, 480, 320),
    WOODS: (1712 + 3200, 2400, 768),
}
#: A room's world y when it is not 0. The rooms of the world touch where they
#: join (`specs/check_flat_map.py`): Zeno's Aqueduct is west of the street at
#: its deck, so the gate hall, which is UNDER the street, is placed there.
ROOMS_Y = {
    HALL: 1344,
    # The buildings' insides (doors: anywhere in the world) are under the
    # town, out of the way of the woods east of the summit.
    **{rid: INSIDES_Y for rid in (CAFE, OFFICE, OBSERVATORY, LOBBY, LIFT, BALLROOM, *CLOSETS)},
    CORRIDOR: INSIDES_Y + (576 - 64) - (448 - 64),
    # The woods' floor (y 704) level with the summit (the street's y 560).
    WOODS: 560 - 704,
}

#: Pixels of a town prop's frame to one world unit (`math_town_facades.PX`,
#: `math_town_terraces.PX`).
PROP_PX = 2


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
    in it, and the door of the portal (`specs/gate_network/gates.py`)."""
    (_, _, top, _, _, _), _, (_, zx, _, zw, zh, _) = gates.ring("", zone, x, floor, name)
    r = gates.RING
    return [
        f'( type: "Prop", px: [{x}, {top}], size: [{r}, {r}], fields: ( name: "{ring}", kind: "gate_ring" ) ),',
        f'( type: "Prop", px: [{x}, {top}], size: [{r}, {r}], fields: ( name: "{portal}", kind: "gate_portal" ) ),',
        (f'( type: "LoadingZone", px: [{zx}, {top}], size: [{zw}, {zh}], fields: ( id: "{zone}", name: "{name}", '
         f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: false ) ),'),
    ]


def room(rid, entities, label_text, theme=BIOME):
    world_x, w, h = ROOMS_AT[rid]
    world_y = ROOMS_Y.get(rid, 0)
    body = "\n        ".join(entities + [
        f'( type: "CameraZone", px: [0, 0], size: [{w}, {h}], fields: ( id: "{rid}_camera", name: "{rid}_camera", mode: "Default" ) ),',
        label(f"{rid}_label", 70, 40, label_text, min(900, w - 140)),
    ])
    return f"""// Written by specs/math_town/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{BG}",
    biome: "{theme}",
    parallax_theme: "{theme}",
    music_track: "{MUSIC}",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


def wanderer(name, character, x, floor, h=64, dialogue=None):
    """A townsperson who walks about the room and pays the player no mind,
    and says `dialogue` when spoken to."""
    del name  # An NpcSpawn has no name field; the character is its name.
    talk = f', dialogue_id: "{dialogue}"' if dialogue else ""
    return (f'( type: "NpcSpawn", px: [{x}, {floor - h}], size: [32, {h}], fields: ( '
            f'character_id: "{character}", brain_override: "townsfolk_roam"{talk} ) ),')


def line(name, points, boarding="auto"):
    """A walkable line the room's art draws (a stair, a ramp): a hidden
    `SurfaceChain`, one-way, ridden by every body that walks
    (`walkable_lines`). `boarding: "up"` is a stair beside other ground: a
    body steps onto it from that ground only while it holds up."""
    pts = "; ".join(f"{round(x)},{round(y)}" for x, y in points)
    x0, y0 = points[0]
    extra = ', boarding: "up"' if boarding == "up" else ""
    return (f'( type: "SurfaceChain", px: [{round(x0)}, {round(y0) - 16}], size: [16, 16], fields: ( name: "{name}", '
            f'points: "{pts}", closed: false, visual: "none"{extra} ) ),')


def still(name, x, y, w, h):
    """A zero-speed one-way platform with no picture: a tread of the
    cradle's stairs, or its plinth (the cradle prop draws them)."""
    return (f'( type: "MovingPlatform", px: [{x}, {y}], size: [{w}, {h}], fields: ( id: "{name}", name: "{name}", '
            f'speed: 0.0, visual: "none", one_way: true ) ),')


#: The gate cradle (`targets/props/gate_cradle.py`): its size in world units,
#: its plinth, and its stairs. The numbers are the art's.
CRADLE_W, CRADLE_H = 480, 212
PLINTH_W, PLINTH_H = 288, 48
STEP_RUN, STEP_RISE = 8, 4
STEPS = PLINTH_H // STEP_RISE


def cradle(prefix, cx, floor):
    """The cradle centred on `cx`, and the line a body walks over it: up the
    stair at one end, across the plinth, down the stair at the other.
    Returns the entities and the top of the plinth."""
    x0 = cx - CRADLE_W // 2
    p0, p1 = cx - PLINTH_W // 2, cx + PLINTH_W // 2
    top = floor - PLINTH_H
    run = STEPS * STEP_RUN
    e = [f'( type: "Prop", px: [{x0}, {floor - CRADLE_H}], size: [{CRADLE_W}, {CRADLE_H}], fields: ( name: "{prefix} Cradle", kind: "gate_cradle" ) ),',
         line(f"{prefix}_cradle_walk", [(p0 - run, floor), (p0, top), (p1, top), (p1 + run, floor)])]
    return e, top


def gate_hall():
    """The gate hall: a vaulted undercroft under the street. The gate stands
    in its cradle in the middle of the floor, with stairs up to its plinth;
    galleries along the walls; the door up to the town at the east end. The
    Manifest Office's booth, the customs desk for axioms, the departures
    board, a porter's cart, polyhedral lamps."""
    rid = HALL
    _, w, h = ROOMS_AT[rid]
    floor = h - 64
    cx = w // 2
    stand, plinth = cradle("math_gate", cx, floor)
    e = [
        # The arrivals hall's furniture (`targets/props/math_hall_props.py`,
        # drawn on this room's numbers): first, so it draws behind the gate.
        framed("hall_lamps", "math_hall_lamps", 0, 48),
        framed("hall_west", "math_hall_west", 16, 300),
        framed("hall_east", "math_hall_east", 1150, 300),
        solid("floor", 0, floor, w, 64),
        solid("vault", 0, 0, w, 48),
        solid("west_wall", 0, 48, WALL, floor - 48),
        solid("east_wall", w - WALL, 48, WALL, floor - 48),
        f'( type: "PlayerStart", px: [{cx - 330}, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        *stand,
        *gate(MATH_RING, MATH_PORTAL, MATH_ZONE, cx - gates.RING // 2, plinth, "", "", "the gate"),
        # The ring's terminal, on the floor east of the cradle.
        *gates.as_ron(gates.terminal("Math Gate (Math)", "math_gate_terminal", cx + 260, floor)),
        # Galleries along the walls, and the ledges up to them.
        ledge("west_ledge", 336, floor - 96, 96),
        ledge("west_gallery", 64, floor - 192, 352),
        ledge("east_ledge", w - 432, floor - 96, 96),
        ledge("east_gallery", w - 416, floor - 192, 352),
        door(f"{rid}_up", "up to the town", w - 112, floor, STREET, f"{STREET}_down"),
        wanderer("traveller_trent", "npc_trent", 220, floor, dialogue="trent"),
        wanderer("traveller_walter", "npc_walter", 1180, floor, dialogue="walter"),
        wanderer("porter_beaver", "npc_busy_beaver", 1320, floor, 48, dialogue="busy_beaver"),
        label("hall_sign_gate", cx - 150, floor - 300, "GATE OF THE MATH DIMENSION", 300),
        label("hall_sign_axioms", 64, floor - 236, "Arrivals: please declare all axioms", 380),
        label("hall_sign_infinity", w - 444, floor - 236, "Infinity is not a valid destination", 380),
        label("hall_sign_manifest", 40, floor - 40, "<- Manifest Office (closed for recounting)", 420),
    ]
    return rid, room(rid, e, "The gate hall under the Math town.", theme="math_undercroft")


def door(zone, name, x, floor, to_room, to_zone):
    return (f'( type: "LoadingZone", px: [{x}, {floor - 96}], size: [48, 96], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def body_box(kind):
    """The body box of `kind`'s published sheet in frame pixels, (x, y, w, h):
    where its art is in its frame (the accepted publication's)."""
    from district_scenery import body_box as published_body_box

    return published_body_box(kind)


def framed(name, kind, fx, fy):
    """Prop `kind` whose FRAME's top left is at (fx, fy): the prop's box is
    the art's body box in the frame."""
    bx, by, bw, bh = body_box(kind)
    return (f'( type: "Prop", px: [{round(fx + bx / PROP_PX)}, {round(fy + by / PROP_PX)}], '
            f'size: [{round(bw / PROP_PX)}, {round(bh / PROP_PX)}], fields: ( name: "{name}", kind: "{kind}" ) ),')


def standing(name, kind, x, floor):
    """Prop `kind` with the left of its art at `x` and its art's bottom on
    `floor`. Returns the entity and the top left of its frame."""
    bx, by, bw, bh = body_box(kind)
    fx, fy = x - bx / PROP_PX, floor - (by + bh) / PROP_PX
    return framed(name, kind, fx, fy), (fx, fy)


#: Where each house front's art draws a place to stand, as (x0, x1, y) in
#: frame units (`targets/props/math_town_facades.py`, the house's margin
#: included). The numbers are the art's: change them with it.
HOUSE_WALKS = {
    "math_house_rose": [(21, 197, 22)],
    "math_house_dome": [(90, 144, 172)],
    "math_house_tower": [(40, 128, 220)],
    "math_house_flat": [(21, 127, 24)],
}


#: Where each house front one can go into paints its door: the middle of the
#: door across the frame, and its height (world units; the house's margin
#: included). The numbers are the art's (`math_town_facades`): change them
#: with it. The arcade has no door: one goes in under its third arch.
HOUSE_DOORS = {
    "math_house_gable": (24 + 170 - 42, 64),
    "math_house_arcade": (24 + 2.5 * 210 / 4, 72),
    "math_observatory": (24 + 190 / 2, 56),
}

#: The width to the height of the door the game draws (`DOOR_SPRITE_ASPECT`).
DOOR_ASPECT = 126 / 242


def painted_door(zone, name, cx, floor, h, to_room, to_zone):
    """A door `h` tall centred on `cx`: the game draws its door as tall as
    the zone, so this one covers the door the house front paints."""
    w = round(h * DOOR_ASPECT)
    return (f'( type: "LoadingZone", px: [{round(cx - w / 2)}, {floor - h}], size: [{w}, {h}], fields: ( id: "{zone}", name: "{name}", '
            f'activation: "Door", target_room: "{to_room}", target_zone: "{to_zone}", bidirectional: true ) ),')


def house(prefix, kind, x, floor):
    """The front of house `kind` with the left of its art at `x`, standing on
    `floor`, and a still one-way platform on each place its art draws to
    stand on."""
    prop, (fx, fy) = standing(prefix, kind, x, floor)
    e = [prop]
    for i, (x0, x1, y) in enumerate(HOUSE_WALKS.get(kind, [])):
        e.append(still(f"{prefix}_walk_{i}", round(fx + x0), round(fy + y), round(x1 - x0), 4))
    return e


#: The street room, from the bottom: the street, the first terrace, the
#: second, the summit; each cliff's foot; the passage under the first
#: terrace; the aqueduct's deck.
STREET_Y, T1, T2, SUMMIT = 1280, 1024, 784, 560
CLIFF1, CLIFF2, CLIFF3 = 1760, 2400, 2880
TUNNEL_TOP = STREET_Y - 96
DECK = SUMMIT

#: The aqueduct (`math_town_terraces`): a span's width, and which span stands
#: on which ground.
SPAN_W = 160
#: The switchback stair (`math_town_terraces`): the left of its frame, its
#: landing, and where flight 2 leaves the landing. The numbers are the art's:
#: change them with it.
STAIR_X = CLIFF1 - 320
LANDING_W, F2_X0 = 64, 320 - 32 * 9
#: The ladders: (name, x, top, bottom). Painted on the `Climbable` layer.
LADDERS = [
    # Up a pier of the aqueduct from the street, clear of the stair (up at
    # the stair's landing boards flight 2).
    ("aqueduct", 8 * SPAN_W + 16, DECK - 16, STREET_Y),
    ("terrace_2", CLIFF2 - 48, T2 - 16, T1),
    ("summit", CLIFF3 - 48, SUMMIT - 16, T2),
]


def foot(x):
    """The ground under `x`: the street, the first terrace or the second."""
    return STREET_Y if x < CLIFF1 else T1 if x < CLIFF2 else T2


def street():
    """The town climbs: the street, a stair to the first terrace, ladders to
    the second and to the summit, and an aqueduct whose deck crosses the whole
    town from the summit. The street goes on under the first terrace in a
    vaulted passage, to a gate that is shut (the long way round, for later)."""
    rid = STREET
    _, w, h = ROOMS_AT[rid]
    e = [
        solid("street", 0, STREET_Y, w, h - STREET_Y),
        solid("terrace_1", CLIFF1, T1, w - CLIFF1, TUNNEL_TOP - T1),
        solid("terrace_2", CLIFF2, T2, w - CLIFF2, T1 - T2),
        solid("summit", CLIFF3, SUMMIT, w - CLIFF3, T2 - SUMMIT),
        # The west wall, open where the aqueduct's deck leaves town.
        solid("west_wall_high", 0, 0, WALL, DECK - 112),
        solid("west_wall_low", 0, DECK + 16, WALL, STREET_Y - DECK - 16),
        edge(f"{rid}_aqueduct_west", "along the aqueduct, out of town", 0, DECK - 96, 96, ZENO, f"{ZENO}_east"),
        solid("east_wall", w - WALL, 0, WALL, SUMMIT - 112),
        edge(f"{rid}_summit_east", "into the Fractal Woods", w - WALL, SUMMIT - 96, 96, WOODS, f"{WOODS}_west"),
        solid("passage_end", w - WALL, TUNNEL_TOP, WALL, STREET_Y - TUNNEL_TOP),
        f'( type: "PlayerStart", px: [160, {STREET_Y - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        door(f"{rid}_down", "down to the gate hall", 96, STREET_Y, HALL, f"{HALL}_up"),
    ]
    # Furthest back first (the room's scenery draws in the order it is
    # listed): the aqueduct, behind everything.
    for i in range(CLIFF3 // SPAN_W):
        x = i * SPAN_W
        ground = foot(x + 8)
        kind = {STREET_Y: "street", T1: "t1", T2: "t2"}[ground]
        e.append(framed(f"aqueduct_span_{i:02d}", f"math_aqueduct_span_{kind}", x, DECK))
    e.append(still("aqueduct_deck", WALL, DECK, CLIFF3 - WALL, 4))
    # The passage under the first terrace, its mouth and its gate.
    for i in range((w - CLIFF1) // 160):
        e.append(framed(f"passage_bay_{i}", "math_tunnel_bay", CLIFF1 + i * 160, TUNNEL_TOP))
    e.append(framed("passage_mouth", "math_tunnel_mouth", CLIFF1 - 16, STREET_Y - 112))
    e.append(framed("passage_gate", "math_tunnel_gate", w - WALL - 48, TUNNEL_TOP))
    # Hylbert's Hotel, in the passage's third bay: its front over the bay, and
    # its door over the revolving door the front paints.
    e.append(framed("hylbert_hotel_front", "math_hotel_entrance", HOTEL_BAY_X, TUNNEL_TOP))
    e.append(painted_door(f"{LOBBY}_street", "into Hylbert's Hotel", HOTEL_BAY_X + 80, STREET_Y, 82, LOBBY, f"{LOBBY}_door"))
    e.append(label("hotel_sign", HOTEL_BAY_X - 70, TUNNEL_TOP - 34, "HYLBERT'S HOTEL: no vacancy (vacancy)", 300))
    # Homes in the rock of each cliff.
    for name, kind, x, y in [
        ("rock_home_t1_a", "wide", 1960, T1 + 40),
        ("rock_home_t1_b", "narrow", 2620, T1 + 36),
        ("rock_home_t2_a", "wide", 2470, T2 + 88),
        ("rock_home_t2_b", "narrow", 3010, T2 + 72),
        ("rock_home_summit", "wide", 2960, SUMMIT + 100),
    ]:
        e.append(framed(name, f"math_cliff_home_{kind}", x, y))
    # The stair: its picture, and two lines: the landing with flight 1 down
    # to the foot of the cliff, and flight 2 up to the terrace. Both are
    # boarded holding up, so the street goes on past the foot of the stair and
    # the landing on down flight 1.
    e.append(framed("switchback_stair", "math_switchback_stair", STAIR_X, T1))
    landing_y = T1 + 128
    e.append(line("stair_flight_1", [(STAIR_X, landing_y), (STAIR_X + LANDING_W, landing_y), (CLIFF1, STREET_Y)], "up"))
    e.append(line("stair_flight_2", [(STAIR_X + F2_X0, landing_y), (CLIFF1, T1)], "up"))
    # The parapet at the open end of the landing (the art's).
    e.append(solid("stair_parapet", STAIR_X, landing_y - 48, 16, 48))
    # The houses: the street's, then the first terrace's, the surveyor's
    # office on the second, the observatory on the summit.
    # The doors into the houses one can go into, over the doors their fronts
    # paint (`HOUSE_DOORS`).
    for prefix, kind, x, ground, room_id in [
        ("cafe_house", "math_house_gable", CLIFF1 + 48, T1, CAFE),
        ("surveyors_office", "math_house_arcade", CLIFF2 + 48, T2, OFFICE),
        ("observatory", "math_observatory", CLIFF3 + 52, SUMMIT, OBSERVATORY),
    ]:
        _, (fx, _fy) = standing(prefix, kind, x, ground)
        cx, door_h = HOUSE_DOORS[kind]
        e.append(painted_door(f"{room_id}_street", f"into the {room_id.removeprefix('math_').replace('_', ' ')}", fx + cx, ground, door_h, room_id, f"{room_id}_door"))
    e += [
        *house("rose_house", "math_house_rose", 192, STREET_Y),
        *house("dome_house", "math_house_dome", 432, STREET_Y),
        *house("tower_house", "math_house_tower", 704, STREET_Y),
        *house("shop_house", "math_house_shop", 912, STREET_Y),
        *house("cafe_house", "math_house_gable", CLIFF1 + 48, T1),
        *house("narrow_house", "math_house_flat", CLIFF1 + 280, T1),
        *house("surveyors_office", "math_house_arcade", CLIFF2 + 48, T2),
        *house("observatory", "math_observatory", CLIFF3 + 52, SUMMIT),
    ]
    for name, kind, x, ground in [
        ("cypress_door", "math_cypress", 154, STREET_Y),
        ("olive_street", "math_olive", 1190, STREET_Y),
        ("olive_terrace", "math_olive", CLIFF1 + 420, T1),
        ("cypress_terrace", "math_cypress", CLIFF1 + 520, T1),
        ("cypress_office", "math_cypress", CLIFF2 + 340, T2),
    ]:
        e.append(standing(name, kind, x, ground)[0])
    # Townsfolk: the Math dimension's own.
    e += [
        wanderer("townsperson_peggy", "npc_peggy", 420, STREET_Y, dialogue="peggy"),
        wanderer("townsperson_victor", "npc_victor", 900, STREET_Y, dialogue="victor"),
        wanderer("townsperson_lovelace", "npc_data_lovelace", 1300, STREET_Y, dialogue="data_lovelace"),
        wanderer("townsperson_olivia", "npc_olivia", CLIFF1 + 60, T1, dialogue="olivia"),
        # Canter and Hylbert are at work inside, and Erdish is at the cafe.
        wanderer("townsperson_leib", "npc_leib_knives", CLIFF2 + 120, T2, dialogue="leib_knives"),
        wanderer("stargazer_hypatia", "npc_hypatia_prime", CLIFF3 + 60, SUMMIT, dialogue="hypatia_prime"),
        label("street_sign", 180, STREET_Y - 300, "Euclid Street (the long way round)", 380),
        label("passage_sign", w - 300, TUNNEL_TOP + 12, "The long way round: shut", 240),
        label("office_sign", CLIFF2 + 60, T2 - 300, "Surveyor's Office", 240),
        label("observatory_sign", CLIFF3 + 60, SUMMIT - 270, "Observatory", 200),
    ]
    return rid, room(rid, e, "The town of the Math dimension: the street under the terraces.")


# ---- Inside the houses ---------------------------------------------------------
#
# Each room is one screen: a tiled floor, plaster walls and a ceiling (theme
# `math_interior`), its furniture as props (`math_town_interiors`), and the door
# back out at its west end. The heights of the walks the art draws are the
# art's (`math_town_interiors.COUNTER_H`, `GALLERY_FLOOR`, `PLATFORM_FLOOR`).
INTERIOR = "math_interior"
INSIDE_FLOOR = 64
INSIDE_CEILING = 32
COUNTER_H, GALLERY_FLOOR, PLATFORM_FLOOR, PIECE_MARGIN = 40, 90, 60, 6


def shell(rid):
    """A room's floor, walls and ceiling, its camera, and its door out at its
    west end. Returns the entities and its floor."""
    _, w, h = ROOMS_AT[rid]
    floor = h - INSIDE_FLOOR
    street_zone = f"{rid}_street"
    return [
        solid("floor", 0, floor, w, INSIDE_FLOOR),
        solid("ceiling", 0, 0, w, INSIDE_CEILING),
        solid("west_wall", 0, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
        solid("east_wall", w - WALL, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
        door(f"{rid}_door", "out to the street", 40, floor, STREET, street_zone),
    ], floor


def piece(name, kind, x, bottom):
    """Furniture `kind` with the left of its drawing at `x` and its foot at
    `bottom`: the piece's frame has `PIECE_MARGIN` round the drawing."""
    return standing(name, kind, x, bottom)[0]


def cafe():
    """The cafe of the first terrace, "The Convergent Series": windows on the
    terrace, the menu, three marble tables, the counter (walked on) with its
    espresso machine, the shelf of jars, lamps. Erdish drinks the coffee;
    Emmy Noether keeps the counter."""
    rid = CAFE
    e, floor = shell(rid)
    _, w, _h = ROOMS_AT[rid]
    counter_x = 620
    e += [
        f'( type: "PlayerStart", px: [96, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        piece("cafe_lamps", "math_cafe_lamps", 120, INSIDE_CEILING + 140),
        piece("cafe_window_west", "math_cafe_window", 140, floor - 150),
        piece("cafe_menu", "math_cafe_menu", 330, floor - 190),
        piece("cafe_window_east", "math_cafe_window", 500, floor - 150),
        piece("cafe_shelf", "math_cafe_shelf", 650, floor - 200),
        piece("cafe_table_a", "math_cafe_table", 170, floor),
        piece("cafe_table_b", "math_cafe_table", 330, floor),
        piece("cafe_table_c", "math_cafe_table", 470, floor),
        piece("cafe_counter", "math_cafe_counter", counter_x, floor),
        still("cafe_counter_top", counter_x, floor - COUNTER_H, 220, 4),
        wanderer("cafe_erdish", "npc_erdish", 300, floor, dialogue="erdish"),
        wanderer("cafe_noether", "npc_emmy_noether", counter_x + 240, floor, dialogue="emmy_noether_cafe"),
    ]
    return rid, room(rid, e, "The Convergent Series: the cafe of the first terrace.", theme=INTERIOR)


def surveyors_office():
    """The Surveyor's Office: the great map of the town on the wall, the
    drafting table, the cabinet of drawers, the theodolite, and the gallery
    of rolled maps up its ladder (walked on). Canter works here."""
    rid = OFFICE
    e, floor = shell(rid)
    _, w, _h = ROOMS_AT[rid]
    gallery_x = w - WALL - 380
    gallery_top = floor - 300 + GALLERY_FLOOR
    e += [
        f'( type: "PlayerStart", px: [96, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        piece("office_map", "math_office_map", 130, floor - 120),
        piece("office_gallery", "math_office_gallery", gallery_x, floor),
        still("office_gallery_floor", gallery_x + 20, gallery_top, 380 - 20, 4),
        piece("office_drafting", "math_office_drafting", 470, floor),
        piece("office_cabinet", "math_office_cabinet", 620, floor),
        piece("office_theodolite", "math_office_theodolite", 104, floor),
        wanderer("office_canter", "npc_georg_canter", 420, floor, dialogue="georg_canter"),
    ]
    return rid, room(rid, e, "The Surveyor's Office of the second terrace.", theme=INTERIOR)


def observatory():
    """The Observatory on the summit: the dome open on the night, the brass
    telescope on its pier pointing up the slit, the platform up its stair
    (walked on), the star chart and the astronomer's desk. Hylbert watches."""
    rid = OBSERVATORY
    _, w, h = ROOMS_AT[rid]
    floor = h - INSIDE_FLOOR
    dome_h = 260
    e = [
        solid("floor", 0, floor, w, INSIDE_FLOOR),
        solid("west_wall", 0, dome_h, WALL, floor - dome_h),
        solid("east_wall", w - WALL, dome_h, WALL, floor - dome_h),
        door(f"{rid}_door", "out to the summit", 40, floor, STREET, f"{rid}_street"),
        f'( type: "PlayerStart", px: [96, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        piece("obs_dome", "math_obs_dome", 0, dome_h),
        piece("obs_chart", "math_obs_chart", 110, floor - 150),
        piece("obs_desk", "math_obs_desk", 100, floor),
        piece("obs_telescope", "math_obs_telescope", 250, floor),
    ]
    # The platform at the east wall, and its stair up from the floor: the
    # stair's line, then the platform's.
    platform_w = 420
    px0 = w - WALL - platform_w
    top = floor - 260 + PLATFORM_FLOOR
    stair_top = px0 + round(platform_w * 0.55)
    e += [
        piece("obs_platform", "math_obs_platform", px0, floor),
        line("obs_stair", [(px0 + 6, floor), (stair_top, top), (w - WALL, top)], "up"),
        # Hylbert keeps his hotel's desk now (`hotel_lobby`).
    ]
    return rid, room(rid, e, "The Observatory on the summit.", theme=INTERIOR)


# ---- Out of town ---------------------------------------------------------------

#: Zeno's Aqueduct: the town's aqueduct goes on west to its reservoir. Its
#: deck is the street's (y 560); the valley under it is at y 832. Its piers
#: stand half as far apart each time (`ZENO_PIERS`), so the dam is always
#: just a little further: the art is `targets/props/math_zeno_props.py`, drawn
#: on these numbers.
ZENO_FLOOR = 832
ZENO_DAM_X = 1920
ZENO_PIERS = [ZENO_DAM_X + (3200 - ZENO_DAM_X) / 2**k for k in range(9)]
#: The reservoir behind the dam, and The Limit, the island at its far end.
ZENO_WATER_TOP = 624
ZENO_ISLAND = (176, 560, 600)
#: The top lefts of the art's frames (`math_zeno_props.PIECES`).
ZENO_AQUEDUCT_FRAME = (ZENO_DAM_X - 80, DECK - 200)
ZENO_RESERVOIR_FRAME = (0, ZENO_ISLAND[2] - 200)


def zeno_enemy(n, character, x, feet_y, w, h, brain, profile=None):
    extra = f', brain_profile: "{profile}"' if profile else ""
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{ZENO}_{character}_{n}", '
            f'brain: "{brain}", character_id: "{character}", respawn: "OnRoomReenter"{extra} ) ),')


def zeno_aqueduct():
    """Out of town: the aqueduct's deck goes on west from the street to the
    dam of the town's reservoir, over a valley. The piers close up toward the
    dam by halves, and their signs count it: half way, three quarters, seven
    eighths. Mites and slugs live in the valley, an owl on a pier and a mite
    on the deck. Behind the dam the reservoir, and at its far end The Limit,
    an island with a gazebo. A ladder goes from the valley up to the deck."""
    rid = ZENO
    _, w, h = ROOMS_AT[rid]
    g = ZENO_FLOOR
    ix0, ix1, itop = ZENO_ISLAND
    e = [
        framed("zeno_reservoir", "math_zeno_reservoir", *ZENO_RESERVOIR_FRAME),
        framed("zeno_aqueduct", "math_zeno_aqueduct", *ZENO_AQUEDUCT_FRAME),
        solid("valley", ZENO_DAM_X, g, w - ZENO_DAM_X, h - g),
        solid("reservoir_bed", 0, g, ZENO_DAM_X, h - g),
        solid("west_wall", 0, 0, WALL, g),
        solid("east_wall_high", w - WALL, 0, WALL, DECK - 112),
        solid("east_wall_low", w - WALL, DECK + 16, WALL, g - DECK - 16),
        edge(f"{rid}_east", "along the aqueduct, back to town", w - WALL, DECK - 96, 96, STREET, f"{STREET}_aqueduct_west"),
        ledge("deck", ZENO_DAM_X, DECK, w - ZENO_DAM_X - WALL),
        f'( type: "PlayerStart", px: [{w - 200}, {DECK - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        solid("dam", ZENO_DAM_X - 64, DECK, 64, g - DECK),
        f'( type: "WaterVolume", px: [{WALL}, {ZENO_WATER_TOP}], size: [{ZENO_DAM_X - 64 - WALL}, {g - ZENO_WATER_TOP}], fields: ( id: "{rid}_reservoir", name: "the reservoir" ) ),',
        solid("the_limit", ix0, itop, ix1 - ix0, g - itop),
        label(f"{rid}_dam_sign", ZENO_DAM_X - 120, DECK - 120, "THE RESERVOIR (arrived, in the limit)", 380),
        label(f"{rid}_limit_sign", (ix0 + ix1) // 2 - 150, itop - 170, "THE LIMIT", 300),
        # Zeno, on the deck between the 7/8 and 3/4 piers: always halfway
        # to the reservoir (`math.yarn` `zeno_halfway`).
        f'( type: "NpcSpawn", px: [{2150 - 16}, {DECK - 64}], size: [32, 64], fields: ( character_id: "npc_math_zeno", '
        f'brain_override: "stand_still", prompt: "Talk", dialogue_id: "zeno_halfway" ) ),',
        # The enemies of the valley and the deck.
        zeno_enemy(1, "npc_dividing_mite", 2320, g, 40, 52, "dividing_mite"),
        zeno_enemy(2, "npc_dividing_mite", 2760, g, 40, 52, "dividing_mite"),
        zeno_enemy(3, "npc_puppy_slug", 2540, g, 28, 44, "combatant", "patroller"),
        zeno_enemy(4, "npc_puppy_slug", 2980, g, 28, 44, "combatant", "patroller"),
        zeno_enemy(5, "npc_exploding_mite", 2420, DECK, 38, 66, "exploding_mite"),
        zeno_enemy(6, "owl", round(ZENO_PIERS[1]) - 18, DECK - 40, 36, 48, "combatant"),
    ]
    # The fraction over each pier, counted from the town.
    for k, x in enumerate(ZENO_PIERS[1:6], start=1):
        e.append(label(f"{rid}_pier_{k}", round(x) - 60, DECK - 70 - 26 * (k % 2), f"{2**k - 1}/{2**k} of the way", 120))
    return rid, room(rid, e, "Zeno's Aqueduct: the reservoir is just a little further.")


# ---- Hylbert's Hotel ------------------------------------------------------------

#: Its front is in the passage's third bay (`math_hotel_props`).
HOTEL_BAY_X = CLIFF1 + 2 * 160
#: The corridor's doors, every 240 from x 160 (the art's), each with its
#: number over it.
CORRIDOR_DOORS = list(range(160, 1600 - 100, 240))


def hotel_lobby():
    """Hylbert's Hotel: the lobby. Hylbert at the front desk (its top walked
    on), the key rack behind him that never ends, the lift, the stair. East,
    the corridor."""
    rid = LOBBY
    _, w, h = ROOMS_AT[rid]
    floor = h - INSIDE_FLOOR
    e = [
        f'( type: "Prop", px: [16, {INSIDE_CEILING}], size: [{w - 32}, {floor - INSIDE_CEILING}], fields: ( name: "hotel_lobby_room", kind: "math_hotel_lobby_room" ) ),',
        solid("floor", 0, floor, w, INSIDE_FLOOR),
        solid("ceiling", 0, 0, w, INSIDE_CEILING),
        solid("west_wall", 0, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
        solid("east_wall", w - WALL, INSIDE_CEILING, WALL, floor - 96 - INSIDE_CEILING - 16),
        door(f"{rid}_door", "out to the street", 40, floor, STREET, f"{rid}_street"),
        edge(f"{rid}_east", "to the rooms", w - WALL, floor - 96, 96, CORRIDOR, f"{CORRIDOR}_west"),
        door(f"{rid}_lift", "into the lift", 956, floor, LIFT, f"{LIFT}_door"),
        door(f"{rid}_ballroom", "to the ballroom", 300, floor, BALLROOM, f"{BALLROOM}_door"),
        f'( type: "PlayerStart", px: [120, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        still("hotel_desk_top", 520, floor - 48, 240, 4),
        wanderer("hotel_hylbert", "npc_davy_hylbert", 640, floor, dialogue="davy_hylbert"),
        label("hotel_vacancy", 380, 64, "NO VACANCY  /  VACANCY", 520),
    ]
    return rid, room(rid, e, "Hylbert's Hotel: the lobby.", theme=INTERIOR)


def hotel_corridor():
    """The corridor of rooms, which goes on: its east end comes back in at
    its west end, so walking east never runs out of rooms; walking west
    always reaches the lobby. Its doors are numbered from n."""
    rid = CORRIDOR
    _, w, h = ROOMS_AT[rid]
    floor = h - INSIDE_FLOOR
    e = [
        f'( type: "Prop", px: [0, {INSIDE_CEILING}], size: [{w}, {floor - INSIDE_CEILING}], fields: ( name: "hotel_corridor_room", kind: "math_hotel_corridor_room" ) ),',
        solid("floor", 0, floor, w, INSIDE_FLOOR),
        solid("ceiling", 0, 0, w, INSIDE_CEILING),
        solid("west_wall", 0, INSIDE_CEILING, WALL, floor - 96 - INSIDE_CEILING - 16),
        solid("east_wall", w - WALL, INSIDE_CEILING, WALL, floor - 96 - INSIDE_CEILING - 16),
        edge(f"{rid}_west", "back to the lobby", 0, floor - 96, 96, LOBBY, f"{LOBBY}_east"),
        (f'( type: "LoadingZone", px: [{w - WALL}, {floor - 96}], size: [16, 96], fields: ( id: "{rid}_east", name: "more rooms", '
         f'activation: "EdgeExit", target_room: "{rid}", target_zone: "{rid}_west", bidirectional: false ) ),'),
        f'( type: "PlayerStart", px: [120, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
    ]
    for k, x in enumerate(CORRIDOR_DOORS):
        e.append(label(f"{rid}_room_{k}", x - 40, floor - 176, "n" if k == 0 else f"n+{k}", 80))
    # Guests, each at a door (Jon, 2026-10-30: "character"): the barber,
    # Furrier and Python Goras (their talk is `math.yarn`'s `hotel_*`).
    for character, dialogue, door_k in (("npc_paradox_barber", "hotel_paradox_barber", 2),
                                        ("npc_joseph_furrier", "hotel_joseph_furrier", 3),
                                        ("npc_python_goras", "hotel_python_goras", 5)):
        x = CORRIDOR_DOORS[door_k] + 50
        e.append(f'( type: "NpcSpawn", px: [{x}, {floor - 64}], size: [32, 64], fields: ( character_id: "{character}", '
                 f'brain_override: "stand_still", prompt: "Talk", dialogue_id: "{dialogue}" ) ),')
    return rid, room(rid, e, "Hylbert's Hotel: the rooms. There is always one more.", theme=INTERIOR)


#: The lift: its car (as Eureka's tether's: `specs/eureka/make_specs.py`)
#: waits a hop over the landing, under the shaft; a body that comes up
#: through a floor edge stands 69 px over it (`rooms::spawn`), so the car's
#: top is there and a body that comes round the shaft lands on it.
LIFT_CAR_X, LIFT_CAR_W, LIFT_CAR_H = 320, 128, 22
LIFT_EDGE_FEET = 68
LIFT_HOP = 40


def hotel_lift():
    """The lift, which only goes up: its car carries a rider out of the top
    of the shaft, and the top comes back in at the bottom, where the car is
    waiting. Hop off at the landing for the door back to the lobby. The
    floors are numbered from n."""
    rid = LIFT
    _, w, h = ROOMS_AT[rid]
    car_top = h - LIFT_EDGE_FEET
    landing = car_top + LIFT_HOP
    e = [
        f'( type: "Prop", px: [16, 0], size: [{w - 32}, {h}], fields: ( name: "hotel_lift_shaft", kind: "math_hotel_lift_shaft" ) ),',
        solid("west_wall", 0, 0, WALL, h),
        solid("east_wall", w - WALL, 0, WALL, h),
        solid("landing", WALL, landing, LIFT_CAR_X - 32 - WALL, h - landing),
        door(f"{rid}_door", "back to the lobby", 48, landing, LOBBY, f"{LOBBY}_lift"),
        f'( type: "PlayerStart", px: [140, {landing - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        (f'( type: "MovingPlatform", px: [{LIFT_CAR_X}, {car_top}], size: [{LIFT_CAR_W}, {LIFT_CAR_H}], fields: ( id: "hotel_lift_car", '
         f'name: "the lift", speed: 260.0, lift_dy: {-(car_top + 60)}.0, one_way: true ) ),'),
        (f'( type: "LoadingZone", px: [{LIFT_CAR_X}, 0], size: [{LIFT_CAR_W}, 16], fields: ( id: "{rid}_top", name: "up", '
         f'activation: "EdgeExit", target_room: "{rid}", target_zone: "{rid}_bottom", bidirectional: false ) ),'),
        (f'( type: "LoadingZone", px: [{LIFT_CAR_X}, {h - 16}], size: [{LIFT_CAR_W}, 16], fields: ( id: "{rid}_bottom", name: "the bottom of the shaft", '
         f'activation: "EdgeExit", target_room: "{rid}", target_zone: "{rid}_top", bidirectional: false ) ),'),
        label(f"{rid}_dial", 60, landing - 200, "Floors: 1, 2, 3, ..., and up", 300),
    ]
    for k, y in enumerate(range(h - 360, 100, -320)):
        e.append(label(f"{rid}_floor_{k}", LIFT_CAR_X + LIFT_CAR_W + 20, y, "FLOOR n" if k == 0 else f"FLOOR n+{k}", 160))
    return rid, room(rid, e, "Hylbert's Hotel: the lift. It only goes up.", theme=INTERIOR)


#: The ballroom's stage doors (x), one to each closet; what is behind each.
STAGE_DOORS = (360, 616, 872)
BEHIND = (("math_goat", "A GOAT."), ("math_car", "A CAR! You did not switch. You did not need to. This time."), ("math_goat", "A GOAT. Statistically, you were going to meet me."))


def hotel_ballroom():
    """The hotel's ballroom, made a game show: three doors on the stage,
    behind one a car and behind the others goats. Out of the lobby by the
    door west of the desk."""
    rid = BALLROOM
    _, w, h = ROOMS_AT[rid]
    floor = h - INSIDE_FLOOR
    e = [
        f'( type: "Prop", px: [16, {INSIDE_CEILING}], size: [{w - 32}, {floor - INSIDE_CEILING}], fields: ( name: "ballroom_stage", kind: "math_ballroom_room" ) ),',
        solid("floor", 0, floor, w, INSIDE_FLOOR),
        solid("ceiling", 0, 0, w, INSIDE_CEILING),
        solid("west_wall", 0, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
        solid("east_wall", w - WALL, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
        door(f"{rid}_door", "back to the lobby", 40, floor, LOBBY, f"{LOBBY}_ballroom"),
        f'( type: "PlayerStart", px: [120, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        *[door(f"{rid}_door_{k + 1}", f"door number {k + 1}", x, floor, CLOSETS[k], f"{CLOSETS[k]}_door") for k, x in enumerate(STAGE_DOORS)],
        label("ballroom_sign", 360, 80, "TONIGHT: PICK A DOOR. THEN SWITCH. ALWAYS SWITCH.", 560),
        label("ballroom_sign_odds", 940, 270, "STAY 1/3   SWITCH 2/3", 300),
        # The host, west of the stage (`math.yarn` `hotel_monty_carlo`).
        f'( type: "NpcSpawn", px: [{230 - 16}, {floor - 64}], size: [32, 64], fields: ( character_id: "npc_math_monty", '
        f'brain_override: "stand_still", prompt: "Talk", dialogue_id: "hotel_monty_carlo" ) ),',
    ]
    return rid, room(rid, e, "Hylbert's Hotel: the ballroom. Tonight: three doors.", theme=INTERIOR)


def hotel_closet(k):
    """Behind stage door k: a small room and what is in it."""
    def make():
        rid = CLOSETS[k]
        _, w, h = ROOMS_AT[rid]
        floor = h - INSIDE_FLOOR
        kind, words = BEHIND[k]
        e = [
            solid("floor", 0, floor, w, INSIDE_FLOOR),
            solid("ceiling", 0, 0, w, INSIDE_CEILING),
            solid("west_wall", 0, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
            solid("east_wall", w - WALL, INSIDE_CEILING, WALL, floor - INSIDE_CEILING),
            door(f"{rid}_door", "back to the stage", 40, floor, BALLROOM, f"{BALLROOM}_door_{k + 1}"),
            f'( type: "PlayerStart", px: [110, {floor - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
            standing(f"{rid}_prize", kind, 220, floor)[0],
            label(f"{rid}_words", 120, 70, words, 340),
        ]
        if kind == "math_car":
            e.append(f'( type: "PickupSpawn", px: [400, {floor - 24}], size: [24, 24], fields: ( name: "{rid}_prize_money", kind: "currency:100" ) ),')
        return rid, room(rid, e, f"Behind door number {k + 1}.", theme=INTERIOR)
    return make


#: The woods (the art, `targets/props/math_woods_props.py`, is drawn on
#: these): the ground, the great tree's limbs walked on (x0, x1, y).
WOODS_FLOOR = 704
WOODS_LIMBS = ((540, 700, 592), (720, 880, 480), (560, 720, 368), (660, 800, 256))


def woods_enemy(n, character, x, feet_y, w, h, brain, profile=None):
    extra = f', brain_profile: "{profile}"' if profile else ""
    return (f'( type: "EnemySpawn", px: [{x}, {feet_y - h}], size: [{w}, {h}], fields: ( name: "{WOODS}_{character}_{n}", '
            f'brain: "{brain}", character_id: "{character}", respawn: "OnRoomReenter"{extra} ) ),')


def fractal_woods():
    """Out of town east of the summit: the Fractal Woods. The great fractal
    tree to climb by its limbs (a chest at the top), a hedge clipped to a
    Koch curve, a Sierpinski rock, a topiary of the Mandelbrot set; puppy
    slugs and dividing mites."""
    rid = WOODS
    _, w, h = ROOMS_AT[rid]
    g = WOODS_FLOOR
    e = [
        framed("woods_far_west", "woods_far_west", 40, 300),
        framed("woods_far_east", "woods_far_east", 1192, 300),
        framed("woods_tree", "woods_great_tree", 420, 20),
        framed("woods_things", "woods_curiosities", 1080, 400),
        solid("ground", 0, g, w, h - g),
        solid("west_wall", 0, 0, WALL, g - 112),
        solid("east_wall", w - WALL, 0, WALL, g),
        edge(f"{rid}_west", "back to the summit", 0, g - 96, 96, STREET, f"{STREET}_summit_east"),
        f'( type: "PlayerStart", px: [100, {g - 46}], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),',
        *[ledge(f"limb_{i}", x0, y, x1 - x0) for i, (x0, x1, y) in enumerate(WOODS_LIMBS)],
        # Ivo, the groundskeeper, by the way in (`math.yarn` `woods_ivo`).
        f'( type: "NpcSpawn", px: [{320 - 16}, {g - 64}], size: [32, 64], fields: ( character_id: "npc_math_ivo", '
        f'brain_override: "stand_still", prompt: "Talk", dialogue_id: "woods_ivo" ) ),',
        f'( type: "ChestSpawn", px: [{WOODS_LIMBS[3][0] + 30}, {WOODS_LIMBS[3][2] - 58}], size: [76, 58], fields: ( name: "the self-similar acorn", reward: "currency:75" ) ),',
        label("woods_sign", 160, g - 220, "THE FRACTAL WOODS: every tree a forest, every branch a tree", 520),
        label("woods_sign_koch", 1010, g - 150, "KOCH HEDGE: infinitely long. Please do not trim.", 420),
        label("woods_sign_mandel", 1830, g - 320, "MANDELBROT BUSH: zoom in for more bush", 400),
        woods_enemy(1, "npc_puppy_slug", 1000, g, 28, 44, "combatant", "patroller"),
        woods_enemy(2, "npc_puppy_slug", 1760, g, 28, 44, "combatant", "patroller"),
        woods_enemy(3, "npc_dividing_mite", 1380, g, 40, 52, "dividing_mite"),
        woods_enemy(4, "npc_dividing_mite", 2280, g, 40, 52, "dividing_mite"),
    ]
    return rid, room(rid, e, "The Fractal Woods: it goes all the way down.")


ROOMS = (gate_hall, street, cafe, surveyors_office, observatory, zeno_aqueduct, hotel_lobby, hotel_corridor, hotel_lift,
         hotel_ballroom, *[hotel_closet(k) for k in range(3)], fractal_woods)

#: What is painted on an IntGrid layer of a room: (room, layer, value, px, size).
PAINT = [(STREET, "Climbable", 1, (x, top), (32, bottom - top)) for _name, x, top, bottom in LADDERS] + [
    # The ladder up to the office's gallery, at its west end (the art's).
    (OFFICE, "Climbable", 1, (ROOMS_AT[OFFICE][1] - WALL - 380 - 8, ROOMS_AT[OFFICE][2] - INSIDE_FLOOR - 300 + GALLERY_FLOOR - 16), (32, 300 - GALLERY_FLOOR + 16)),
    # Zeno's: up the first pier from the valley to the deck (the art's).
    (ZENO, "Climbable", 1, (3200 - WALL - 96, DECK - 16), (32, ZENO_FLOOR - DECK + 16)),
]


def home_gate_yaml() -> str:
    """Ring 7 in `gate_stack_lower`: the ring, its portal, its zone, its
    breaker and its terminal, on the floor east of the old gate."""
    floor = 736
    x = 1102
    r, zw = gates.RING, gates.ZONE_W
    top = floor - r
    terminal = gates.as_yaml("gate_stack_lower", gates.terminal("Ring 7 (Home)", "ring7_terminal", 1430, floor)).split("entities:\n", 1)[1]
    return f"""# Written by specs/math_town/make_specs.py. Change the script, not this file.
# Ring 7 of the gate station: the Math line.
level_id: gate_stack_lower
entities:
  - type: Prop
    px: [{x}, {top}]
    size: [{r}, {r}]
    fields: {{name: "{HOME_RING}", kind: gate_ring}}
  - type: Prop
    px: [{x}, {top}]
    size: [{r}, {r}]
    fields: {{name: "{HOME_PORTAL}", kind: gate_portal}}
  - type: LoadingZone
    px: [{x + (r - zw) // 2}, {top}]
    size: [{zw}, {r}]
    fields:
      id: {HOME_ZONE}
      name: ring 7
      activation: Door
      target_room: ""
      target_zone: ""
      bidirectional: false
  - type: Switch
    px: [{x + 296}, {floor - 32}]
    size: [24, 32]
    fields:
      id: {SWITCH}
      name: Ring 7 breaker
      prompt: Throw the breaker
      action: ResetEncounter
      target_encounter: ""
  - type: DebugLabel
    px: [{x - 20}, {top - 32}]
    size: [260, 24]
    fields:
      name: gate_stack_sign_gate_seven
      text: "RING 7: the Math line (power: the breaker)"
      category: Custom
{terminal}"""


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
        for room_id, layer, value, (x, y), (w, h) in PAINT:
            code, _ = run("intgrid", "paint", "--ldtk", str(MATH_LDTK), "--level", room_id, "--layer", layer,
                          "--value", str(value), "--px", f"{x},{y}", "--size", f"{w},{h}")
            if code != 0:
                return code
        for world in (MATH_LDTK, INTRO_LDTK):
            code, _ = run("validate", str(world))
            if code != 0:
                return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
