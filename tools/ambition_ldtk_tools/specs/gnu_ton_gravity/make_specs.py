#!/usr/bin/env python3
"""Write the rooms of the GNU-ton gravity handle, and the hub door to it.

    python specs/gnu_ton_gravity/make_specs.py          # writes the specs
    python specs/gnu_ton_gravity/make_specs.py --apply  # and makes the rooms
    python specs/gnu_ton_gravity/make_specs.py --apply --hub  # and the hub's door

Run it from `tools/ambition_ldtk_tools`.

The handle is the gravity works of the Science dimension (Jon, 2026-10-24:
GNU-ton lives in the Science dimension, with its orbital station). Its rooms
are in that dimension's world file (`science.ldtk`), beside the station's
(`specs/orbital_station/`). It is a level of gravity and portal puzzles, and
GNU-ton waits at its end. Each puzzle room teaches ONE idea where a mistake
costs nothing, tests it where a mistake costs a short walk, and then turns it
with an idea of an earlier room. The design, room by room, is in `README.md`
here.

The numbers the gaps are made from (the robot, `platformer_defaults.ron`,
measured in the game): a jump rises 83 px, a jump and a double jump 139 px,
a run is 270 px/s, the body is 30 x 48. A GravityZone pulls with the normal
gravity (2250 px/s^2) in its `dir`; a body takes the zone's gravity as soon
as its box touches the zone, and the smaller zone wins where two overlap. A
`HazardBlock` replays the room from its PlayerStart. Every step up in the
level is 64 px or less, so one jump climbs it.

The robot also flies, blinks and climbs walls. No room can take that away
(there is no room field for it), so a room only shuts a way with solid wall.
Each room here is made for a robot that runs and jumps; where a wall can
stand round the way on, it does.
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

sys.path.insert(0, str(HERE.parent / "gate_network"))
import gates  # noqa: E402

WORLD_LDTK = default_worlds_dir() / "science.ldtk"
SANDBOX_LDTK = default_worlds_dir() / "sandbox.ldtk"

#: The rooms, in the order the player meets them.
ORCHARD = "gnu_ton_orchard"
WELL = "gnu_ton_vector_well"
HALL = "gnu_ton_conservation_hall"
GALLERY = "gnu_ton_inversion_gallery"
APPROACH = "gnu_ton_works_approach"
ARENA = "gnu_ton_shoulders"

#: The station's rooms the works join (`specs/orbital_station/make_specs.py`):
#: the spoke's door at the orchard, and the lower end of the approach's lift.
SPOKE = "orbital_station_spoke"
SPOKE_WORKS_DOOR = "spoke_works_door"
SPOKE_LIFT_DOOR = "spoke_lift_door"

#: Rooms of the first draft (2026-10-23) that this level replaces.
RETIRED = ("gnu_ton_outer_handle", "gnu_ton_proofworks")

#: The hub's door to the handle (`central_hub_main`, `sandbox.ldtk`).
HUB_ROOM = "central_hub_complex"
HUB_DOOR = "gnu_ton_handle_hub_door"
HUB_ARRIVAL = "gnu_ton_handle_hub_arrival"

#: GNU-ton's placement in the main game. His progress is saved under this id;
#: the hall's copy is `hall.gnu_ton`.
GNU_TON_PLACEMENT = "gnu_ton_handle.gnu_ton"

#: The gravity works are open trusses of the orbital station, under glass,
#: with the planet below (Jon, 2026-10-24: a space station space biome).
PUZZLE_BIOME = "orbital_truss"
PUZZLE_PARALLAX = "orbital_truss"
PUZZLE_BG = "#0E1424"
PUZZLE_MUSIC = "standing_on_shoulders"
WALL = 16

#: Each room: (world_x, world_y, width, height).
ROOMS_AT = {
    ORCHARD: (0, 0, 2816, 960),
    WELL: (3072, -640, 1280, 1600),
    HALL: (4608, -704, 2560, 1024),
    GALLERY: (7424, -704, 2560, 960),
    APPROACH: (10240, -2560, 1280, 1408),
    ARENA: (10240, -1024, 1920, 1280),
}


# ---------------------------------------------------------------------------
# Entities
# ---------------------------------------------------------------------------


def _value(v) -> str:
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, float):
        return f"{v:.1f}" if v == int(v) else repr(v)
    if isinstance(v, int):
        return str(v)
    return f'"{v}"'


def entity(etype, x, y, w, h, ref=None, **fields) -> str:
    field_text = ", ".join(f"{k}: {_value(v)}" for k, v in fields.items())
    ref_text = f'ref: "{ref}", ' if ref else ""
    return f'( type: "{etype}", {ref_text}px: [{x}, {y}], size: [{w}, {h}], fields: ( {field_text} ) ),'


def solid(name, x, y, w, h):
    return entity("Solid", x, y, w, h, name=name)


def ledge(name, x, y, w):
    return entity("OneWayPlatform", x, y, w, 16, name=name)


def hazard(name, x, y, w, h=16):
    return entity("HazardBlock", x, y, w, h, name=name)


def field(zone_id, x, y, w, h, direction, amplitude=0.0, freq=1.0):
    """A GravityZone. `amplitude` slides it along x (px), `freq` in rad/s."""
    return entity(
        "GravityZone", x, y, w, h,
        id=zone_id, name=zone_id.replace("_", " "), dir=direction,
        oscillate_amplitude=float(amplitude), oscillate_freq=float(freq),
    )


def portal(pid, link, x, y, normal, color, length=96):
    """A portal on the face it is snapped to. `normal`: up = on a floor,
    down = on a ceiling, right = on a wall that faces right, left = on a
    wall that faces left. `length` is the opening."""
    w, h = (length, 18) if normal in ("up", "down") else (18, length)
    return entity("Portal", x, y, w, h, id=pid, name=pid.replace("_", " "), color=color, normal=normal, link=link)


def door(zone, name, x, floor, to_room, to_zone, bidirectional=True):
    return entity(
        "LoadingZone", x, floor - 96, 48, 96,
        id=zone, name=name, activation="Door", target_room=to_room, target_zone=to_zone, bidirectional=bidirectional,
    )


def start(rid, x, floor):
    return entity("PlayerStart", x, floor - 46, 28, 46, name=f"{rid}_spawn")


def coin(name, x, y, amount=10):
    """A reward: `x, y` is the coin's top left."""
    return entity("PickupSpawn", x, y, 24, 24, name=name, kind=f"currency:{amount}", respawn="Never")


def switch(sid, x, y, prompt, action):
    return entity("Switch", x, y, 24, 32, id=sid, name=sid.replace("_", " "), prompt=prompt, target_encounter="", action=action)


def label(name, x, y, text, w=640):
    return entity("DebugLabel", x, y, w, 24, name=name, text=text, category="Custom")


def camera(rid, w, h, zoom=None):
    extra = {"zoom": zoom} if zoom else {}
    return entity("CameraZone", 0, 0, w, h, id=f"{rid}_camera", name=f"{rid}_camera", mode="Default", **extra)


def room(rid, entities, title, title_y=96, biome=PUZZLE_BIOME, parallax=PUZZLE_PARALLAX, music=PUZZLE_MUSIC, bg=PUZZLE_BG, zoom=None, extra_fields=""):
    world_x, world_y, w, h = ROOMS_AT[rid]
    body = "\n        ".join(entities + [camera(rid, w, h, zoom), label(f"{rid}_title", 64, title_y, title, 900)])
    return f"""// Written by specs/gnu_ton_gravity/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    bg_color: "{bg}",
    biome: "{biome}",
    parallax_theme: "{parallax}",
    music_track: "{music}",{extra_fields}
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


# ---------------------------------------------------------------------------
# The rooms
# ---------------------------------------------------------------------------


def orchard():
    """1. THE ORCHARD — here an apple falls up (an up field).

    Teach: a column of up-gravity stands over plain floor. Walk in and you
    fall onto the ceiling; walk out and you fall back. A coin waits in a
    nook of the ceiling that only a body falling up reaches.
    Test: a chasm too wide to jump, with a column over it from bank to bank.
    Walk in on one bank, walk the ceiling, walk out over the far bank.
    Twist: the last column swings from bank to bank. A body on the ceiling
    stays in it only by walking with it.

    The chasms are 192 deep, deeper than a double jump, and the steps out of
    each are on its near side: a fall costs the walk back up.
    """
    rid = ORCHARD
    _, _, w, h = ROOMS_AT[rid]
    floor, vault = 704, 80
    pit_floor = 896
    nook_x, nook_w = 432, 80
    e = [
        # The vault, with a nook over the first column.
        solid("vault_west", 0, 0, nook_x, vault),
        solid("vault_nook_cap", nook_x, 0, nook_w, 16),
        solid("vault_east", nook_x + nook_w, 0, w - nook_x - nook_w, vault),
        solid("west_wall", 0, vault, WALL, floor - vault),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
        # The ground: three banks and two chasms.
        solid("west_bank", 0, floor, 960, h - floor),
        solid("chasm_one_floor", 960, pit_floor, 544, h - pit_floor),
        solid("middle_bank", 1504, floor, 384, h - floor),
        solid("chasm_two_floor", 1888, pit_floor, 576, h - pit_floor),
        solid("east_bank", 2464, floor, w - 2464, h - floor),
        # The way out of each chasm, on its near side.
        ledge("chasm_one_step_low", 960, 832, 96),
        ledge("chasm_one_step_high", 1056, 768, 96),
        ledge("chasm_two_step_low", 1888, 832, 96),
        ledge("chasm_two_step_high", 1984, 768, 96),
        # Teach: over plain floor, up to the ceiling's nook.
        # Out of its east side you fall back to the floor, short of the next tree.
        field("orchard_first_tree", 352, 16, 240, floor - 16, "up"),
        coin("orchard_nook_apple", nook_x + 28, 20),
        # Test: from bank to bank over the first chasm. Its foot is 32 over
        # the banks, so a body on a bank is in it and a body in the chasm is not.
        field("orchard_bridge_tree", 896, vault, 672, floor - 32 - vault, "up"),
        # Twist: the swinging tree. Its centre goes 256 to each side of
        # 2176: at the west end it is 80 over the middle bank, at the east
        # end 80 over the east bank. 0.5 rad/s: 128 px/s at the most, about
        # half a run.
        field("orchard_swinging_tree", 2064, vault, 224, floor - 32 - vault, "up", amplitude=256, freq=0.5),
        coin("orchard_swing_apple", 2164, vault + 8),
        # Doors.
        door("orchard_hub_door", "back to the hub", 64, floor, HUB_ROOM, HUB_DOOR),
        start(rid, 176, floor),
        # The spoke to the station, by the way in: the works and the station
        # are one geography.
        door("orchard_spoke_door", "the spoke to the station", 272, floor, SPOKE, SPOKE_WORKS_DOOR),
        door("orchard_east_door", "on to the vector well", w - 112, floor, WELL, "well_bottom_door"),
        # The trail's ring (ASCENT, `data/gate_network.ron`) and its terminal,
        # on the west bank between the first tree and the first chasm.
        *gates.as_ron(gates.ring("Gravity Trail Gate", "orchard_gate", 568, floor, "the gate")),
        *gates.as_ron(gates.terminal("Gravity Trail Gate", "orchard_gate_terminal", 836, floor)),
    ]
    return rid, room(rid, e, "THE ORCHARD: HERE AN APPLE FALLS UP")


def vector_well():
    """2. THE VECTOR WELL — every wall is a floor (fields to the side).

    Teach: on the floor of the well a field pulls to the right. Walk in and
    the east wall is the floor; walk up it. Over the top of that field you
    fall back to the floor of the well.
    Test: over the right field, a band of left-pulling field crosses the
    well. Walk up the east wall into it and you fall across the well onto
    the west wall; walk up that.
    Twist: at the top of the west wall an up field (the orchard's idea)
    takes you to the vault, and along the vault, out of the field, is the
    way on: right, left, up, down — falling has four directions.

    Every mistake ends on the floor of the well.
    """
    rid = WELL
    _, _, w, h = ROOMS_AT[rid]
    floor, vault = h - 64, 48
    shaft_x = 320
    passage_floor = 144
    corridor_top = floor - 128
    e = [
        solid("floor", 0, floor, w, h - floor),
        solid("vault", 0, 0, w, vault),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
        solid("passage_west_wall", 0, vault, WALL, passage_floor - vault),
        solid("west_rock", 0, passage_floor, shaft_x, corridor_top - passage_floor),
        solid("corridor_west_wall", 0, corridor_top, WALL, floor - corridor_top),
        # Teach: the floor of the well pulls right, onto the east wall.
        field("well_east_pull", 800, 1024, w - 800, floor - 1024, "right"),
        coin("well_east_wall_coin", w - WALL - 28, 1072),
        # Test: across the well, a band that pulls left.
        field("well_west_pull", shaft_x, 560, w - shaft_x, 1024 - 560, "left"),
        # Twist: up, by the west wall, to the vault and the passage.
        field("well_up_pull", shaft_x, vault, 320, 560 - vault, "up"),
        coin("well_vault_coin", 560, vault + 4),
        door("well_bottom_door", "back to the orchard", 48, floor, ORCHARD, "orchard_east_door"),
        start(rid, 176, floor),
        door("well_top_door", "on to the conservation hall", 48, passage_floor, HALL, "hall_west_door"),
    ]
    return rid, room(rid, e, "THE VECTOR WELL: EVERY WALL IS A FLOOR")


def conservation_hall():
    """3. THE CONSERVATION HALL — two places made one (portals).

    Teach: a wall too tall to jump. In the floor before it is a portal; its
    other end is on a post on top of the wall. Walk in, come out on top.
    Test: the top of the wall is shut in by a wall to the vault. Its only way
    on is down: off its edge, into a portal that is the whole floor of the
    basin below. Its other end is in the next wall, facing a pit too wide to
    jump. What falls far comes out fast, and flies over.
    Twist: the door on is in a gallery shut in by solid wall. Its way in is a
    portal in the floor of the hall whose other end is in the gallery's
    floor, under a field that pulls up: you fall in down, come out going up,
    and go on falling up, onto the ceiling (the orchard's idea). Walk the
    ceiling out of the field and drop to the door.

    The pit's floor is a hazard: the hall starts again at its door.
    """
    rid = HALL
    _, _, w, h = ROOMS_AT[rid]
    floor, vault = 896, 48
    shelf_x, shelf_top, shelf_end = 576, 448, 992
    pillar_x = 1280
    pit_end = 1568
    gallery_x, gallery_floor = 1856, 432
    basin = pillar_x - shelf_end
    e = [
        solid("vault", 0, 0, w, vault),
        solid("west_wall", 0, vault, WALL, floor - vault),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
        solid("floor_west", 0, floor, pillar_x + 32, h - floor),
        solid("pit_bed", pillar_x + 32, h - 16, pit_end - pillar_x - 32, 16),
        hazard("pit_hazard", pillar_x + 32, h - 32, pit_end - pillar_x - 32),
        solid("floor_east", pit_end, floor, w - pit_end, h - floor),
        # Teach: the shelf, too tall to jump, and the portal to its top.
        solid("shelf", shelf_x, shelf_top, shelf_end - shelf_x, floor - shelf_top),
        solid("shelf_post", shelf_x, 256, 32, shelf_top - 256),
        portal("hall_floor_door", "hall_shelf_pair", 352, floor - 9, "up", "purple"),
        portal("hall_post_door", "hall_shelf_pair", shelf_x + 32 - 9, shelf_top - 96, "right", "yellow"),
        coin("hall_shelf_coin", shelf_end - 64, shelf_top - 32),
        # Test: the shelf is shut in on the east by the pillar, to the vault.
        # The basin between them is one portal; the pillar's east face holds
        # its other end, over the pit.
        solid("pillar", pillar_x, vault, 32, floor - vault),
        # A pair opens as long as its shorter end: both ends are the basin's width.
        portal("hall_basin_door", "hall_fling_pair", shelf_end, floor - 9, "up", "teal", length=basin),
        portal("hall_pillar_door", "hall_fling_pair", pillar_x + 32 - 9, 448, "right", "red", length=basin),
        # Twist: the gallery, shut in by solid wall; in by its ceiling.
        solid("gallery_floor", gallery_x, gallery_floor, w - gallery_x, 48),
        solid("gallery_west_wall", gallery_x, vault, 32, gallery_floor - vault),
        # Its foot is a body's height over the gallery's floor: a body that
        # stands on the floor is out of it, a body that rises is in it.
        field("hall_gallery_pull", gallery_x + 32, vault, 2304 - gallery_x - 32, gallery_floor - 48 - vault, "up"),
        portal("hall_well_door", "hall_gallery_pair", 2112, floor - 9, "up", "green"),
        portal("hall_gallery_door", "hall_gallery_pair", 2016, gallery_floor - 9, "up", "magenta"),
        coin("hall_gallery_coin", 1952, vault + 4),
        door("hall_west_door", "back to the vector well", 48, floor, WELL, "well_top_door"),
        start(rid, 176, floor),
        door("hall_gallery_exit", "on to the inversion gallery", w - 112, gallery_floor, GALLERY, "gallery_west_door"),
    ]
    return rid, room(rid, e, "THE CONSERVATION HALL: TWO PLACES MADE ONE")


def inversion_gallery():
    """4. THE INVERSION GALLERY — the whole room turns over (switches).

    Teach: a lever by the door turns the room's gravity up; a lever on the
    ceiling above it turns it down again.
    Test: the floor stops at a chasm with a hazard in it; the ceiling goes
    on. Turn the room over and walk the ceiling across.
    Twist: the ceiling has a hole in it, too wide to jump, with a hazard in
    it. Before it is a portal in the ceiling (the hall's idea), and past it
    the other end, on the face of a wall that hangs from the ceiling. Upside
    down, the road steps down to the first portal: walk off the step, fall
    up into it, and come out of the hanging wall past the hole. Past it, a column of DOWN field (the orchard's field,
    turned) stands from the ceiling to the floor: in an upside-down room it
    is the one place that falls down, onto the ledge of GNU-ton's door and a
    shrine.

    A fall into a hazard starts the gallery again, the right way up.
    """
    rid = GALLERY
    _, _, w, h = ROOMS_AT[rid]
    floor, vault = 896, 64
    chasm_x, chasm_end = 640, 1408
    # The ceiling is 96 lower (a thicker vault) from the west wall to the
    # step: upside down, the road steps down to the first portal.
    step_x, step_low = 1600, 160
    hole_x, hole_end = 1760, 2080
    down_x = 2304
    e = [
        solid("vault_west", 0, 0, step_x, step_low),
        solid("vault_before_hole", step_x, 0, hole_x - step_x, vault),
        solid("vault_east", hole_end, 0, w - hole_end, vault),
        hazard("hole_hazard", hole_x, 0, hole_end - hole_x),
        solid("west_wall", 0, step_low, WALL, floor - step_low),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
        solid("floor_west", 0, floor, chasm_x, h - floor),
        hazard("chasm_hazard", chasm_x, h - 16, chasm_end - chasm_x),
        solid("floor_east", chasm_end, floor, w - chasm_end, h - floor),
        # Teach: the levers.
        switch("gallery_up_lever", 320, floor - 32, "Turn gravity up", "SetGravityUp"),
        switch("gallery_down_lever", 400, step_low, "Turn gravity down", "SetGravityDown"),
        coin("gallery_ceiling_coin", 1000, step_low + 4),
        # Twist: over the hole by two portals in the ceiling.
        portal("gallery_step_door", "gallery_hole_pair", step_x, vault - 9, "down", "cyan", length=hole_x - step_x),
        solid("hanging_wall", hole_end, vault, 32, 192),
        portal("gallery_far_door", "gallery_hole_pair", hole_end + 32 - 9, vault + 32, "right", "rose", length=hole_x - step_x),
        # The column that remembers down, over the door's ledge.
        field("gallery_down_column", down_x, vault, w - WALL - down_x, floor - vault, "down"),
        entity("ShrineSpawn", down_x + 48, floor - 80, 44, 80, id="gallery_shrine", name="the shrine before GNU-ton"),
        door("gallery_west_door", "back to the conservation hall", 48, floor, HALL, "hall_gallery_exit"),
        start(rid, 176, floor),
        door("gallery_arena_door", "on to the approach", w - 112, floor, APPROACH, "approach_gallery_door"),
    ]
    return rid, room(rid, e, "THE INVERSION GALLERY: THE WHOLE ROOM TURNS OVER", title_y=step_low + 48)


def approach():
    """5. THE APPROACH — the climb to GNU-ton, and the way back.

    The gallery solved, the works open up: a tall hall under glass, the
    giant's shoulders above. A column of up field by the door lifts the
    robot to the vault (the orchard's idea, one last time, with nothing to
    lose) along the face of a tower too tall to jump; walk the vault out of
    it and drop onto the tower, where GNU-ton's door is. By the way in, the works' lift goes down to the spoke, by the
    orchard: the shortcut back, once the gallery is behind you. Its gate in
    the spoke is let go from the lift's side.
    """
    rid = APPROACH
    _, _, w, h = ROOMS_AT[rid]
    floor, vault = h - 64, 48
    column_x, column_w = 512, 192
    landing_y = 352
    e = [
        solid("floor", 0, floor, w, h - floor),
        solid("vault", 0, 0, w, vault),
        solid("west_wall", 0, vault, WALL, floor - vault),
        solid("east_wall", w - WALL, vault, WALL, floor - vault),
        door("approach_gallery_door", "back to the inversion gallery", 48, floor, GALLERY, "gallery_arena_door"),
        start(rid, 176, floor),
        door("approach_lift_door", "the lift down to the spoke", 304, floor, SPOKE, SPOKE_LIFT_DOOR),
        label("approach_lift_sign", 240, floor - 160, "LIFT: DOWN TO THE SPOKE AND THE ORCHARD", 420),
        # The climb: the column stands against the face of a tower too tall
        # to jump. A body that walks into it falls up along the face to the
        # vault; past the column's east edge it falls onto the tower's top.
        field("approach_column", column_x, vault, column_w, floor - 32 - vault, "up"),
        solid("landing", column_x + column_w, landing_y, w - WALL - column_x - column_w, floor - landing_y),
        coin("approach_landing_coin", 1040, landing_y - 32),
        door("approach_arena_door", "to GNU-ton", w - 144, landing_y, ARENA, "shoulders_entry"),
        # The prelude (round 3, 2026-10-24): GNU-ton on his own feet waits on
        # the tower by his door and duels the player there. Nearly beaten, he
        # runs out by it, and the player chases him to his giant
        # (`bosses/gnu_ton/prelude.rs` in `ambition_content`).
        entity("EnemySpawn", 920, landing_y - 64, 40, 64, ref="gnu_ton_prelude", name="GNU-ton", brain="combatant",
               character_id="npc_gnu_ton", respawn="DeadStaysDead", disposition="Hostile"),
        # The military survey ring (hidden address TESLA) on the tower's top:
        # receive only, and only from the military band
        # (`docs/planning/game/gate-network.md`).
        *gates.as_ron(gates.ring("Trail Survey Gate", "trail_survey_gate", column_x + column_w - 32, landing_y, "the survey ring")),
        label("approach_survey_sign", column_x + column_w + 16, landing_y - 232, "SURVEY RING (MILITARY): receive only", 300),
    ]
    return rid, room(rid, e, "THE APPROACH: HE STANDS ON THE SHOULDERS OF A GIANT", title_y=landing_y + 64)


def shoulders():
    """5. THE SHOULDERS OF GIANTS — GNU-ton's arena.

    The hall's arena (`gnu_ton_arena`, `sandbox.ldtk`), as it is: the same
    floor, walls, two low steps, giant and rider, so his fight is the fight of
    the hall. You come in at the top and the pad under you breaks. His placement
    is `gnu_ton_handle.gnu_ton`. The way out is a door home in an alcove
    past the east wall, behind a grille that his defeat opens.
    """
    rid = ARENA
    _, _, w, h = ROOMS_AT[rid]
    floor = 1248
    east = 1776
    alcove_top = 1136
    e = [
        solid("floor", 0, floor, w, h - floor),
        solid("ceiling", 0, 0, w, 16),
        solid("left_wall", 0, 16, 16, floor - 16),
        # The east wall stands where the hall's does: the fight measures it.
        solid("east_wall", east, 16, 16, alcove_top - 16),
        solid("alcove_roof", east + 16, alcove_top - 32, w - east - 16, 32),
        solid("alcove_back", w - 16, 16, 16, floor - 16),
        entity("LockWall", east, alcove_top, 16, floor - alcove_top, id="shoulders_exit_grille",
               name="the grille of the way home", gated_by=f"boss.cleared {GNU_TON_PLACEMENT}", look="temple_gate_iron"),
        door("shoulders_home_door", "home to the hub", east + 40, floor, HUB_ROOM, HUB_DOOR, bidirectional=False),
        # The entry and the pad that breaks.
        solid("entry_ledge", 16, 208, 288, 16),
        entity("BreakablePlatform", 304, 208, 160, 16, name="spawn_descent_pad", max_hp=3, collision="Solid",
               trigger="OnStand", respawn="Never"),
        entity("PlayerStart", 368, 162, 28, 46, name=f"{rid}_spawn"),
        door("shoulders_entry", "back to the approach", 64, 208, APPROACH, "approach_arena_door"),
        # Two low steps at the walls, and no more (Jon, 2026-10-24: "avoid too
        # many platforms in the room, because the mechanism to get onto the
        # boss should be the shoulders, back and portals"). The giant walks
        # the hall; its back is reached up the slope from its rump, or
        # through the pair of its hammer.
        ledge("perch_left_low", 176, 1120, 160),
        ledge("perch_right_low", 1456, 1120, 160),
        # The giant and its rider, as in the hall.
        entity("EnemySpawn", 581, 784, 630, 464, ref="giant_gnu_mount", name="Giant GNU", brain="giant_gnu",
               character_id="npc_giant_gnu", disposition="Peaceful", respawn="OnRoomReenter", depth_plane="Behind"),
        entity("BossSpawn", 1125, 692, 54, 96, name="GNU-ton", brain="PhaseScript:gnu_ton_rider",
               mounted_on="giant_gnu_mount", encounter_id=GNU_TON_PLACEMENT),
    ]
    world_x, world_y, _, _ = ROOMS_AT[rid]
    body = "\n        ".join(e + [
        camera(rid, w, h, 1.25),
        label(f"{rid}_title", 128, 1216, "GNU-TON: STAND ON THE SHOULDERS OF A GIANT; STRIKE A FIST WHILE IT IS STUCK", 1536),
    ])
    return rid, f"""// Written by specs/gnu_ton_gravity/make_specs.py. Change the script, not this file.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {world_x},
    world_y: {world_y},
    px_wid: {w},
    px_hei: {h},
    fill_collision: "empty",
    biome: "{PUZZLE_BIOME}",
    parallax_theme: "{PUZZLE_PARALLAX}",
    music_track: "standing_on_shoulders_v2",
    ambient_profile: "hum",
    visual_theme: "default",
    entities: [
        {body}
    ],
    connect_to: [],
)
"""


ROOMS = (orchard, vector_well, conservation_hall, inversion_gallery, approach, shoulders)

HUB_DOOR_YAML = f"""# The hub's door to the GNU-ton gravity handle: one door, both ways, with the
# orchard's door. Written by specs/gnu_ton_gravity/make_specs.py.
level_id: central_hub_main
edits:
  - target: {{identifier: LoadingZone, match: {{id: {HUB_DOOR}}}}}
    fields: {{target_room: "{ORCHARD}", target_zone: "orchard_hub_door", bidirectional: true}}
"""

#: Where the hub's door stands: on the hub's main floor between the Hall of
#: Biomes' door and the gate stack's, clear of the Kernel Guide's walk
#: (`door free-spots central_hub_main`).
HUB_DOOR_AT = (1504, 880)

HUB_DOOR_MOVE_YAML = f"""# The hub's door to the GNU-ton gravity handle stands clear of the Kernel
# Guide, whose talk took its Interact. Written by specs/gnu_ton_gravity/make_specs.py.
level_id: central_hub_main
moves:
  - target: {{identifier: LoadingZone, match: {{id: {HUB_DOOR}}}}}
    px: [{HUB_DOOR_AT[0]}, {HUB_DOOR_AT[1]}]
"""

HUB_ARRIVAL_DELETE_YAML = f"""# The first draft's arrival-only zone in the hub: the door is two-way now.
level_id: central_hub_main
deletes:
  - target: {{identifier: LoadingZone, match: {{id: {HUB_ARRIVAL}}}}}
"""


def run(*args) -> tuple[int, str]:
    done = subprocess.run([sys.executable, "-m", "ambition_ldtk_tools", *args], cwd=TOOLS, capture_output=True, text=True)
    print(done.stdout + done.stderr)
    return done.returncode, done.stdout + done.stderr


def main(argv):
    made = []
    for build in ROOMS:
        rid, text = build()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        made.append(path)
        print(path.relative_to(TOOLS))
    if "--apply" not in argv:
        return 0
    # The rooms are made again from nothing: a room of the first draft may
    # stand where a room of this one goes.
    world = WORLD_LDTK.read_text()
    for retired in (*RETIRED, *(build.__name__ and path.stem for build, path in zip(ROOMS, made))):
        if f'"identifier": "{retired}"' in world:
            code, _ = run("level", "delete", retired, "--ldtk", str(WORLD_LDTK), "--in-place", "--no-repair")
            if code != 0:
                return code
    # Each room's doors name the other rooms, so the world is whole only
    # after the last room: until then it is enough that each was written.
    for path in made:
        code, out = run("area", "create", str(path), "--ldtk", str(WORLD_LDTK), "--replace-existing")
        if code != 0 and "wrote new level" not in out and "replaced level" not in out:
            return code
    # The hub's door (only with `--hub`: the hub is the gate network's, and
    # these edits are the first draft's; they change nothing on a hub that
    # has them). Until it names the orchard, the hub's check finds a door to
    # a room that is gone.
    if "--hub" not in argv:
        code, _ = run("validate", str(WORLD_LDTK))
        return code
    hub = HERE / "hub_door.yaml"
    hub.write_text(HUB_DOOR_YAML)
    code, _ = run("entity", "set-field", str(hub), "--ldtk", str(SANDBOX_LDTK), "--in-place")
    hub.unlink()
    if code != 0:
        return code
    move = HERE / "hub_door_move.yaml"
    move.write_text(HUB_DOOR_MOVE_YAML)
    code, _ = run("entity", "move", str(move), "--ldtk", str(SANDBOX_LDTK), "--in-place")
    move.unlink()
    if code != 0:
        return code
    if HUB_ARRIVAL in SANDBOX_LDTK.read_text():
        delete = HERE / "hub_arrival_delete.yaml"
        delete.write_text(HUB_ARRIVAL_DELETE_YAML)
        code, _ = run("entity", "delete", str(delete), "--ldtk", str(SANDBOX_LDTK), "--in-place")
        delete.unlink()
        if code != 0:
            return code
    for checked in (WORLD_LDTK, SANDBOX_LDTK):
        code, _ = run("validate", str(checked))
        if code != 0:
            return code
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
