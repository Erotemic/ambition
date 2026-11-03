#!/usr/bin/env python3
"""Write the rooms of the Art dimension (`docs/planning/game/art-dimension.md`).

    python specs/art_dimension/make_specs.py          # writes the specs
    python specs/art_dimension/make_specs.py --apply  # and makes the rooms

Run it from `tools/ambition_ldtk_tools`.

A world where a thing that somebody makes keeps going after they stop. Its
gate is EASEL (`data/gate_network.ron`). The rooms out in the world join by
side and floor exits on a flat map (`specs/check_flat_map.py art.ldtk`):

                 [ the sketchbook cliffs          ]
                   ^ lift            gap v
    [ the painter's plaza ][ the sculpture garden ] -door-> [ the unfinished gallery ]
            drain v                     well v
    [ the inkfall caverns                         ]

- `art_painters_plaza`: the ring and its terminal at the west end; the
  Curator, Pablo Peckasso and Rob Boss; easels and paint pots. A palette
  lift carries a rider up through the roof to the cliffs. The drain in the
  floor has a grate that the lever under it opens (a shortcut up from the
  caverns, opened from the far side). East to the garden. Not dangerous.
- `art_sketchbook_cliffs`: paper ledges up to the cyan pigment at the east
  end; doodle imps in threes, origami birds; a gap in the floor drops down
  into the garden (one way: a shortcut down).
- `art_sculpture_garden`: the living statues, a brush knight, Frida Collage.
  A scaffold up the head that nobody finished to the yellow pigment. An ink
  well down to the caverns. The colour gate: three doors, each shut until
  its pigment is taken (`world.flag_set art_pigment_<colour>`). Behind them,
  the door to the gallery.
- `art_inkfall_caverns`: under the plaza and the garden. Pools of ink that
  hurt, ink wisps, the magenta pigment; the lever of the plaza's drain, and
  a ladder up the well to the garden.
- `art_gallery_unfinished`: a building, by its door in the garden. A vigil
  (`art_gallery_vigil`, `data/encounters/goblin_encounter.ron`) shuts the
  room; when it is over, the grille of the store room opens.

The plaza, the cliffs and the garden are a loop; the plaza, the caverns and
the garden are a loop.

The art is the sprite tools' `targets/props/art_props.py` (the pigments, the
colour gates, the grate, the palette lift, the easel, the crayon, the pots)
and `targets/characters/` (the cast). A prop sheet that is 2 frame px to a
world unit is placed at half its frame.
"""
from __future__ import annotations

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / "gate_network"))

from district_kit import WALL, WORLDS, edge, ensure_world, entity, door, ledge, make_rooms, paint, prop, resident, room, run, sign, solid, start  # noqa: E402

import gates  # noqa: E402

WORLD = WORLDS / "art.ldtk"
GENERATOR = "specs/art_dimension/make_specs.py"
UID_OFFSET = 1100000

PLAZA = "art_painters_plaza"
CLIFFS = "art_sketchbook_cliffs"
GARDEN = "art_sculpture_garden"
CAVERNS = "art_inkfall_caverns"
GALLERY = "art_gallery_unfinished"

TOWN_MUSIC = "artists_guild_chiaroscuro"
CAVE_MUSIC = "mages_guild_arcane_lanterns"
BG = "#2A1E44"

#: Each room: where it is in the world, and its size.
ROOMS_AT = {
    PLAZA: ((0, 1200), (2400, 1024)),
    CLIFFS: ((1200, 176), (2800, 1024)),
    GARDEN: ((2400, 1200), (2400, 1024)),
    CAVERNS: ((0, 2224), (4800, 1024)),
    # A building: out of the way of the rooms of the flat map.
    GALLERY: ((5200, 1200), (1600, 640)),
}
#: The floor of the rooms out in the world: 64 of ground under it.
FLOOR = 960
H = 1024
CEILING = 32

#: The palette lift's shaft through the plaza's roof (plaza x0, x1): the
#: cliffs' hatch is over it (cliffs x = plaza x - 1200).
LIFT_SHAFT = (1664, 1792)
LIFT_W, LIFT_HOP, LIFT_SPEED = 112, 40, 240.0
#: The plaza's drain, and the caverns' shaft under it (the same x).
DRAIN = (1200, 1264)
DRAIN_LEVER = "art_drain_lever"
#: The gap in the cliffs' floor (cliffs x0, x1) over the garden (garden x =
#: cliffs x - 1200).
CLIFF_GAP = (2400, 2560)
#: The ink well in the garden's floor (garden x0, x1), and the caverns'
#: shaft under it (caverns x = garden x + 2400).
WELL = (1600, 1696)
#: The colour gate: one door for each pigment, left edges in the garden.
COLOUR_DOORS = (("cyan", 1904), ("magenta", 2016), ("yellow", 2128))
DOOR_W, DOOR_H = 48, 192
#: The gallery's vigil.
VIGIL = "art_gallery_vigil"
GALLERY_FLOOR = 576

#: The box of each kind of creature (its whole size).
BOX = {
    "doodle_imp": (32, 42), "ink_wisp": (36, 44), "origami_bird": (36, 30),
    "brush_knight": (56, 84), "living_statue": (110, 150),
}


def enemy(rid, n, character, x, feet):
    """A creature whose feet (a flyer: the foot of its box) are at `feet`."""
    w, h = BOX[character]
    return entity("EnemySpawn", x, feet - h, w, h, name=f"{rid}_{character}_{n}", brain="combatant",
                  character_id=character, respawn="OnRoomReenter")


def pigment(colour, x, y):
    """The pigment `colour`, its middle at `x`, `y`: taken once, and its flag
    stays set (`world.flag_set art_pigment_<colour>`)."""
    return entity("PickupSpawn", x - 17, y - 17, 34, 34, name=f"the {colour} pigment", kind=f"flag:art_pigment_{colour}",
                  respawn="Never", sprite=f"art_pigment_{colour}")


def lock_wall(wall_id, name, x, y, w, h, gated_by, look):
    """A wall drawn as `look` that stands until `gated_by` is true."""
    return entity("LockWall", x, y, w, h, id=wall_id, name=name, gated_by=gated_by, look=look)


def lever(switch_id, name, prompt, x, floor, action="ToggleFlag", target=""):
    """A lever on `floor`; its state is `world.switch_on <switch_id>`."""
    fields = dict(id=switch_id, name=name, prompt=prompt, action=action)
    if target:
        fields["target_encounter"] = target
    return entity("Switch", x, floor - 32, 24, 32, **fields)


def ink(name, x, w):
    """A pool of ink on the floor: it hurts."""
    return entity("HazardBlock", x, FLOOR - 16, w, 16, name=name)


def vertical_edge(zone, name, x0, x1, y, to_room, to_zone):
    """An edge exit across the top (`y` 0) or the bottom of a room."""
    return entity("LoadingZone", x0, y, x1 - x0, 16, id=zone, name=name, activation="EdgeExit", target_room=to_room,
                  target_zone=to_zone, bidirectional=True)


def easel(n, x, floor):
    return prop(f"easel_{n}", "art_easel", x, floor - 88, 64, 88)


def crayon(n, x, floor):
    return prop(f"crayon_{n}", "art_giant_crayon", x, floor - 128, 32, 128)


def pots(n, x, floor):
    return prop(f"paint_pots_{n}", "art_paint_pots", x, floor - 48, 72, 48)


def build(rid, entities, title, *, music=TOWN_MUSIC, biome="art", dangerous=False):
    at, size = ROOMS_AT[rid]
    return rid, room(rid, entities, at=at, size=size, biome=biome, music=music, bg=BG, title=title, generator=GENERATOR,
                     dangerous=dangerous)


def plaza():
    rid = PLAZA
    w = ROOMS_AT[rid][1][0]
    s0, s1 = LIFT_SHAFT
    d0, d1 = DRAIN
    e = [
        solid("ground_west", 0, FLOOR, d0, H - FLOOR),
        solid("ground_east", d1, FLOOR, w - d1, H - FLOOR),
        # The drain: its grate shut until the lever under it in the caverns
        # is pulled; a ladder (`LADDERS`) down the hole.
        lock_wall("art_plaza_drain_grate", "the drain's grate", d0, FLOOR, d1 - d0, H - FLOOR,
                  f"world.switch_on {DRAIN_LEVER}", "art_drain_grate"),
        vertical_edge("plaza_drain", "down the drain to the caverns", d0, d1, H - 16, CAVERNS, "caverns_drain"),
        solid("roof_west", 0, 0, s0, CEILING),
        solid("roof_east", s1, 0, w - s1, CEILING),
        vertical_edge("plaza_lift_top", "up to the sketchbook cliffs", s0, s1, 0, CLIFFS, "cliffs_lift"),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, FLOOR - 112 - CEILING),
        edge("plaza_east", "to the sculpture garden", w - WALL, FLOOR, GARDEN, "garden_west"),
        start(rid, 460, FLOOR),
        # The ring (EASEL) and its terminal: the way in and the way home.
        *gates.as_ron(gates.ring("Plaza Gate", "art_plaza_gate", 64, FLOOR, "the gate")),
        *gates.as_ron(gates.terminal("Plaza Gate", "art_gate_terminal", 368, FLOOR)),
        resident("npc_art_curator", "art_curator", 560, FLOOR, h=86),
        easel(1, 760, FLOOR),
        resident("npc_art_pablo_peckasso", "art_pablo_peckasso", 880, FLOOR, h=64),
        easel(2, 940, FLOOR),
        crayon(1, 1060, FLOOR),
        sign("plaza_sign_drain", d0 + 32, FLOOR - 140, "DRAIN: ink goes down, not you", 320),
        pots(1, 1340, FLOOR),
        easel(3, 1440, FLOOR),
        resident("npc_art_rob_boss", "art_rob_boss", 1560, FLOOR, h=74),
        # The palette lift: it waits a hop over the floor under the open roof
        # (as the Eureka tether's car): hop on and it carries you up.
        entity("MovingPlatform", s0 + (s1 - s0 - LIFT_W) // 2, FLOOR - LIFT_HOP, LIFT_W, 16, id="art_plaza_palette_lift",
               name="the palette lift", speed=LIFT_SPEED, lift_dy=float(-(FLOOR - LIFT_HOP + 60)), one_way=True,
               visual="art_palette_lift"),
        sign("plaza_sign_lift", (s0 + s1) // 2, FLOOR - 220, "STAND ON THE PALETTE TO GO UP", 340),
        crayon(2, 1900, FLOOR),
        pots(2, 2040, FLOOR),
        ledge("plaza_stage", 1960, FLOOR - 96, 240),
        sign("plaza_sign_gate", 208, FLOOR - 340, "EASEL: the gate is also an easel", 360),
        sign("plaza_sign_east", 2150, FLOOR - 200, "TO THE GARDEN: please do not finish the statues", 420),
    ]
    return build(rid, e, "THE PAINTER'S PLAZA")


def cliffs():
    """The sketchbook cliffs: paper ledges up to the cyan pigment."""
    rid = CLIFFS
    w = ROOMS_AT[rid][1][0]
    h0, h1 = LIFT_SHAFT[0] - 1200, LIFT_SHAFT[1] - 1200
    g0, g1 = CLIFF_GAP
    e = [
        solid("ground_west", 0, FLOOR, h0, H - FLOOR),
        solid("ground_mid", h1, FLOOR, g0 - h1, H - FLOOR),
        solid("ground_east", g1, FLOOR, w - g1, H - FLOOR),
        # The hatch over the lift's shaft: come up through it and stand on
        # it; drop through it to go down.
        ledge("lift_hatch", h0, FLOOR, h1 - h0),
        vertical_edge("cliffs_lift", "down to the plaza", h0, h1, H - 16, PLAZA, "plaza_lift_top"),
        vertical_edge("cliffs_gap", "down through the gap to the garden", g0, g1, H - 16, GARDEN, "garden_drop"),
        solid("ceiling", 0, 0, w, CEILING),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, FLOOR - CEILING),
        start(rid, h1 + 120, FLOOR),
        sign("cliffs_sign", 700, 180, "THE SKETCHBOOK CLIFFS: drawn in one sitting, not finished in any", 520),
        sign("cliffs_sign_bankzy", 1180, FLOOR - 120, "BANKZY WAS HERE", 260),
        sign("cliffs_sign_gap", (g0 + g1) // 2, FLOOR - 160, "THE PAGE ENDS HERE (mind it)", 320),
        # The first margin: three doodles and a bird.
        enemy(rid, 1, "doodle_imp", 820, FLOOR),
        enemy(rid, 2, "doodle_imp", 870, FLOOR),
        enemy(rid, 3, "doodle_imp", 920, FLOOR),
        enemy(rid, 1, "origami_bird", 1000, FLOOR - 260),
        ledge("page_a", 1040, FLOOR - 64, 160),
        ledge("page_b", 1240, FLOOR - 128, 160),
        ledge("page_c", 1440, FLOOR - 192, 192),
        enemy(rid, 4, "doodle_imp", 1480, FLOOR - 192),
        ledge("page_d", 1680, FLOOR - 256, 160),
        enemy(rid, 2, "origami_bird", 1720, FLOOR - 420),
        ledge("page_e", 1900, FLOOR - 320, 192),
        ledge("page_f", 2140, FLOOR - 384, 160),
        enemy(rid, 3, "origami_bird", 2200, FLOOR - 560),
        ledge("page_g", 2380, FLOOR - 448, 192),
        ledge("page_h", 2600, FLOOR - 512, 184),
        # The second margin, by the gap.
        enemy(rid, 5, "doodle_imp", 2160, FLOOR),
        enemy(rid, 6, "doodle_imp", 2220, FLOOR),
        enemy(rid, 7, "doodle_imp", 2280, FLOOR),
        # Cyan is high, where the pages end.
        pigment("cyan", 2700, FLOOR - 512 - 40),
        crayon(1, 2650, FLOOR),
    ]
    return build(rid, e, "THE SKETCHBOOK CLIFFS", biome="art_sketchbook", dangerous=True)


def garden():
    """The sculpture garden: the statues, the scaffold to the yellow pigment,
    the ink well down, the colour gate and the gallery's door behind it."""
    rid = GARDEN
    w = ROOMS_AT[rid][1][0]
    k0, k1 = WELL
    d0, d1 = CLIFF_GAP[0] - 1200, CLIFF_GAP[1] - 1200
    gates_x0 = COLOUR_DOORS[0][1] - 16
    gates_x1 = COLOUR_DOORS[-1][1] + DOOR_W + 16
    e = [
        solid("ground_west", 0, FLOOR, k0, H - FLOOR),
        solid("ground_east", k1, FLOOR, w - k1, H - FLOOR),
        vertical_edge("garden_well", "down the ink well to the caverns", k0, k1, H - 16, CAVERNS, "caverns_well"),
        solid("roof_west", 0, 0, d0, CEILING),
        solid("roof_east", d1, 0, w - d1, CEILING),
        vertical_edge("garden_drop", "up to the sketchbook cliffs", d0, d1, 0, CLIFFS, "cliffs_gap"),
        solid("west_wall", 0, CEILING, WALL, FLOOR - 112 - CEILING),
        edge("garden_west", "back to the plaza", 0, FLOOR, PLAZA, "plaza_east"),
        solid("east_wall", w - WALL, CEILING, WALL, FLOOR - CEILING),
        start(rid, 140, FLOOR),
        resident("npc_art_frida_collage", "art_frida_collage", 360, FLOOR, h=62),
        easel(1, 420, FLOOR),
        sign("garden_sign", 640, 160, "THE SCULPTURE GARDEN: stand behind them, not in front", 480),
        enemy(rid, 1, "living_statue", 700, FLOOR),
        pots(1, 920, FLOOR),
        enemy(rid, 1, "brush_knight", 1060, FLOOR),
        # The scaffold up the head that nobody finished: yellow is on top.
        ledge("scaffold_1", 1260, FLOOR - 64, 112),
        ledge("scaffold_2", 1380, FLOOR - 128, 112),
        ledge("scaffold_3", 1260, FLOOR - 192, 112),
        ledge("scaffold_top", 1360, FLOOR - 256, 160),
        pigment("yellow", 1440, FLOOR - 256 - 40),
        sign("garden_sign_well", (k0 + k1) // 2, FLOOR - 160, "THE INK WELL: deeper than it looks", 320),
        enemy(rid, 2, "living_statue", 1740, FLOOR),
        # The colour gate: a lintel over the three doors, so the only way is
        # through them.
        solid("colour_gate_lintel", gates_x0, CEILING, gates_x1 - gates_x0, FLOOR - DOOR_H - CEILING),
        *[lock_wall(f"art_colour_gate_{colour}", f"the {colour} door", x, FLOOR - DOOR_H, DOOR_W, DOOR_H,
                    f"world.flag_set art_pigment_{colour}", f"art_colour_gate_{colour}") for colour, x in COLOUR_DOORS],
        sign("garden_sign_gate", (gates_x0 + gates_x1) // 2, FLOOR - DOOR_H - 120, "CYAN, MAGENTA, YELLOW. BRING ALL THREE.", 420),
        door("garden_gallery_door", "into the unfinished gallery", 2280, FLOOR, GALLERY, "gallery_door"),
    ]
    return build(rid, e, "THE SCULPTURE GARDEN", dangerous=True)


def caverns():
    """Under the plaza and the garden: the ink falls here and pools."""
    rid = CAVERNS
    w = ROOMS_AT[rid][1][0]
    d0, d1 = DRAIN
    k0, k1 = WELL[0] + 2400, WELL[1] + 2400
    e = [
        solid("floor", 0, FLOOR, w, H - FLOOR),
        solid("roof_west", 0, 0, d0, CEILING),
        solid("roof_mid", d1, 0, k0 - d1, CEILING),
        solid("roof_east", k1, 0, w - k1, CEILING),
        vertical_edge("caverns_drain", "up the drain to the plaza", d0, d1, 0, PLAZA, "plaza_drain"),
        vertical_edge("caverns_well", "up the ink well to the garden", k0, k1, 0, GARDEN, "garden_well"),
        solid("west_wall", 0, CEILING, WALL, FLOOR - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, FLOOR - CEILING),
        start(rid, k0 - 160, FLOOR),
        # The lever of the plaza's drain: the far side of the shortcut.
        lever(DRAIN_LEVER, "the drain's lever", "Open the drain", d1 + 40, FLOOR),
        sign("caverns_sign_lever", d1 + 52, FLOOR - 140, "DRAIN GRATE (pull for daylight)", 320),
        sign("caverns_sign", 2400, 160, "THE INKFALL CAVERNS: everything above ends up here", 480),
        ink("ink_pool_a", 1600, 160),
        ledge("stone_a", 1600, FLOOR - 96, 160),
        enemy(rid, 1, "ink_wisp", 1900, FLOOR - 160),
        ink("ink_pool_b", 2160, 224),
        ledge("stone_b", 2160, FLOOR - 112, 96),
        ledge("stone_c", 2300, FLOOR - 176, 96),
        # Magenta is low, where the ink falls.
        ledge("magenta_shelf", 2440, FLOOR - 240, 160),
        pigment("magenta", 2520, FLOOR - 240 - 40),
        enemy(rid, 2, "ink_wisp", 2600, FLOOR - 360),
        ink("ink_pool_c", 2840, 192),
        ledge("stone_d", 2860, FLOOR - 96, 160),
        enemy(rid, 3, "ink_wisp", 3200, FLOOR - 140),
        enemy(rid, 1, "doodle_imp", 3400, FLOOR),
        ink("ink_pool_d", 3560, 128),
        enemy(rid, 4, "ink_wisp", 3700, FLOOR - 300),
        enemy(rid, 1, "brush_knight", 600, FLOOR),
        sign("caverns_sign_well", (k0 + k1) // 2, FLOOR - 200, "UP: the garden. DOWN: you are down.", 380),
    ]
    return build(rid, e, "THE INKFALL CAVERNS", music=CAVE_MUSIC, biome="art_inkfall", dangerous=True)


def gallery():
    """The unfinished gallery: a vigil of the half-made shuts the room; when
    it is over, the grille of the store room opens."""
    rid = GALLERY
    w, h = ROOMS_AT[rid][1]
    g = GALLERY_FLOOR
    store = 1312
    e = [
        solid("floor", 0, g, w, h - g),
        solid("ceiling", 0, 0, w, CEILING),
        solid("west_wall", 0, CEILING, WALL, g - CEILING),
        solid("east_wall", w - WALL, CEILING, WALL, g - CEILING),
        door("gallery_door", "out to the garden", 40, g, GARDEN, "garden_gallery_door"),
        start(rid, 120, g),
        # The vigil: the wall that shuts the room, the strip that starts it,
        # and the bell that makes it ready again.
        entity("LockWall", 176, CEILING, 32, g - CEILING, id="art_gallery_lock", name="the gallery's shutter",
               look="temple_gate_iron"),
        entity("EncounterTrigger", 320, CEILING, 64, g - CEILING, id=VIGIL, name=f"{VIGIL}_trigger", camera_zoom=1.2),
        lever("art_gallery_again", "the curator's bell", "Ring for the vigil again", 140, g, action="ResetEncounter",
              target=VIGIL),
        ledge("plinth_a", 520, g - 96, 160),
        ledge("plinth_b", 840, g - 160, 160),
        ledge("plinth_c", 1080, g - 96, 128),
        easel(1, 640, g),
        easel(2, 960, g),
        # The store room: what the gallery keeps back.
        solid("store_wall", store, CEILING, 32, g - 128 - CEILING),
        lock_wall("art_gallery_store_grille", "the grille of the store room", store, g - 128, 32, 128,
                  f"encounter.cleared {VIGIL}", "temple_gate_iron"),
        entity("ChestSpawn", 1440, g - 58, 76, 58, name="the last thing nobody finished", reward="currency:150"),
        sign("gallery_sign", 800, 120, "THE UNFINISHED GALLERY: every work here is still being made", 520),
        sign("gallery_sign_store", 1440, g - 200, "STORE ROOM: not for display", 280),
    ]
    return build(rid, e, "THE UNFINISHED GALLERY")


#: The ladders (the IntGrid `Climbable` layer): down each hole in a floor,
#: and up each shaft to it.
LADDERS = (
    (PLAZA, DRAIN[0] + 16, FLOOR - 64, H - FLOOR + 64),
    (CAVERNS, DRAIN[0] + 16, 0, FLOOR),
    (GARDEN, WELL[0] + 32, FLOOR - 64, H - FLOOR + 64),
    (CAVERNS, WELL[0] + 2400 + 32, 0, FLOOR),
)

ROOMS = (plaza, cliffs, garden, caverns, gallery)


def main(argv):
    paths = []
    for make in ROOMS:
        rid, text = make()
        path = HERE / f"{rid}.ron"
        path.write_text(text)
        print(path.relative_to(HERE.parents[1]))
        paths.append(path)
    if "--apply" not in argv:
        return 0
    if code := ensure_world(WORLD, UID_OFFSET):
        return code
    # A world made from the sandbox's defs has no `dangerous` level field.
    code, _ = run("level", "add-field-def", "dangerous", "--type", "Bool", "--default", "false", "--doc",
                  "Out of town: an NPC on the player's side joins the player's fights here (RoomMetadata::dangerous).",
                  str(WORLD), "--in-place")
    if code:
        return code
    if code := make_rooms(WORLD, paths, keep_position="--relayout" not in argv):
        return code
    for rid, x, y, h in LADDERS:
        if code := paint(WORLD, rid, "Climbable", 1, x, y, 32, h):
            return code
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
