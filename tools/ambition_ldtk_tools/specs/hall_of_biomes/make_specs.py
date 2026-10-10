#!/usr/bin/env python3
"""Make the area specs of the hall of biomes: one hall and one room for each biome.

The hall is one long room with a door for each biome. Each door has the name of
its biome over it and goes to a small exhibit room that names that biome as its
`parallax_theme`. Each exhibit room is the same room: a floor, a block, a block
in the air, two one-way platforms, a ladder, a pool of water, a soft and a hard blink wall and a strip of hazard. So each part of the
dressing of a biome (the sky, the skin of the ground, the decor, the motes, the
door, the ladder, the water) is in view in one place.

`--apply` is for a map that does not have these rooms. A room and the hall
have doors to each other, and a door to a door that is not there is refused,
so a room of a made hall can not be made again alone: take the rooms out of
the map first (`git checkout` of the map, or `level delete --no-repair` for
each), then apply.

Run from `tools/ambition_ldtk_tools/`, with the interpreter of that tool:

    python specs/hall_of_biomes/make_specs.py          # writes the .ron specs
    python specs/hall_of_biomes/make_specs.py --apply  # and makes the rooms

A new biome is one row in `BIOMES` here (and its rows in the art tables: see
`docs/systems/parallax-backgrounds.md`).
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LDTK = "../../game/ambition_map_assets/ambition_content/worlds/sandbox.ldtk"

#: (the key of `ParallaxTheme`, the name on the door, the `biome` of the room).
BIOMES = [
    ("hub", "Hub", "hub"),
    ("hub_clean", "Hub, clean", "hub"),
    ("hub_corrupt", "Hub, corrupt", "hub"),
    ("lab", "Lab", "lab"),
    ("alarm", "Lab, raided", "lab"),
    ("basement", "Basement", "basement"),
    ("undertown", "Under-town", "basement"),
    ("boss", "Boss vault", "boss"),
    ("cave", "Cave", "cave"),
    ("forest", "Forest", "forest"),
    ("cove", "Cove", "cove"),
    ("water", "Deep water", "water"),
    ("skybridge", "Skybridge", "hub"),
    ("open_sky", "Open sky", "cove"),
    ("eclipse", "Eclipse", "lab"),
]

HALL = "hall_of_biomes"
#: Free world space under the last row of rooms of the sandbox world.
WORLD_X, WORLD_Y = 0, 8704
ROOM_W, ROOM_H = 1280, 720
DOOR_STEP = 128
HALL_W = 160 + DOOR_STEP * len(BIOMES) + 96
HALL_H = 416
FLOOR = HALL_H - 32


def room_id(key: str) -> str:
    return f"biome_{key}"


def door_x(index: int) -> int:
    return 192 + DOOR_STEP * index


def hall_spec() -> str:
    doors = []
    back = []
    for i, (key, name, _biome) in enumerate(BIOMES):
        # The door of the room, back to the hall: made with the hall, in the
        # room (`connect_to`). A door to a door that does not exist is refused,
        # so the two doors of a pair are made in one step.
        back.append(
            f'        ( target_room: "{room_id(key)}", px: [48, 528], size: [48, 96], id: "{room_id(key)}_entry", '
            f'name: "to hall of biomes", target_zone: "hall_to_{room_id(key)}", activation: "Door", bidirectional: true ),'
        )
        doors.append(
            f'        ( type: "LoadingZone", px: [{door_x(i)}, {FLOOR - 96}], size: [48, 96], fields: ( '
            f'id: "hall_to_{room_id(key)}", name: "{name}", activation: "Door", '
            f'target_room: "{room_id(key)}", target_zone: "{room_id(key)}_entry", bidirectional: true ) ),'
        )
    return f'''// hall_of_biomes: a hall with a door for each biome of the sandbox.
// Made by specs/hall_of_biomes/make_specs.py: do not edit by hand.
//
//   area create specs/hall_of_biomes/hall_of_biomes_area.ron --replace-existing
(
    id: "{HALL}",
    level_id: "{HALL}",
    world_x: {WORLD_X},
    world_y: {WORLD_Y},
    px_wid: {HALL_W},
    px_hei: {HALL_H},
    fill_collision: "empty",
    bg_color: "#141019",
    biome: "hub",
    entities: [
        ( type: "Solid", px: [0, {FLOOR}], size: [{HALL_W}, 32], fields: ( name: "floor" ) ),
        ( type: "Solid", px: [0, 0], size: [{HALL_W}, 16], fields: ( name: "ceiling" ) ),
        ( type: "Solid", px: [0, 16], size: [16, {FLOOR - 16}], fields: ( name: "left_wall" ) ),
        ( type: "Solid", px: [{HALL_W - 16}, 16], size: [16, {FLOOR - 16}], fields: ( name: "right_wall" ) ),
        ( type: "PlayerStart", px: [112, {FLOOR - 46}], size: [28, 46], fields: ( name: "{HALL}_spawn" ) ),
        ( type: "LoadingZone", px: [32, {FLOOR - 96}], size: [48, 96], fields: ( id: "{HALL}_entry", name: "to central hub", activation: "Door", target_room: "central_hub_complex", target_zone: "{HALL}_door", bidirectional: true ) ),
        ( type: "DebugLabel", px: [192, 64], size: [900, 24], fields: ( name: "{HALL}_sign", text: "Hall of biomes. Each door goes to one biome: the same room in each, in the art of that biome.", category: "Custom" ) ),
{chr(10).join(doors)}
    ],
    connect_to: [
        ( target_room: "central_hub_main", px: [1432, 880], size: [48, 96], id: "{HALL}_door", name: "to hall of biomes", target_zone: "{HALL}_entry", activation: "Door", bidirectional: true ),
{chr(10).join(back)}
    ],
)
'''


def exhibit_spec(index: int, key: str, name: str, biome: str) -> str:
    rid = room_id(key)
    wx = WORLD_X + (index % 8) * (ROOM_W + 64)
    wy = WORLD_Y + HALL_H + 64 + (index // 8) * (ROOM_H + 64)
    return f'''// {rid}: the exhibit room of the biome `{key}` in the hall of biomes.
// Made by specs/hall_of_biomes/make_specs.py: do not edit by hand.
(
    id: "{rid}",
    level_id: "{rid}",
    world_x: {wx},
    world_y: {wy},
    px_wid: {ROOM_W},
    px_hei: {ROOM_H},
    fill_collision: "empty",
    bg_color: "#141019",
    biome: "{biome}",
    entities: [
        // The shell.
        ( type: "Solid", px: [0, 624], size: [{ROOM_W}, 96], fields: ( name: "floor" ) ),
        ( type: "Solid", px: [0, 0], size: [{ROOM_W}, 16], fields: ( name: "ceiling" ) ),
        ( type: "Solid", px: [0, 16], size: [16, 608], fields: ( name: "left_wall" ) ),
        ( type: "Solid", px: [{ROOM_W - 16}, 16], size: [16, 608], fields: ( name: "right_wall" ) ),
        // A block on the floor and a block in the air: the cap, the sides and the under side of the ground.
        ( type: "Solid", px: [336, 528], size: [160, 96], fields: ( name: "block" ) ),
        ( type: "Solid", px: [640, 384], size: [224, 48], fields: ( name: "block_in_the_air" ) ),
        // Two rims that hold the pool.
        ( type: "Solid", px: [976, 560], size: [16, 64], fields: ( name: "pool_rim_left" ) ),
        ( type: "Solid", px: [1200, 560], size: [16, 64], fields: ( name: "pool_rim_right" ) ),
        ( type: "WaterVolume", px: [992, 576], size: [208, 48], fields: ( id: "{rid}_pool", name: "pool" ) ),
        ( type: "OneWayPlatform", px: [192, 448], size: [112, 16], fields: ( name: "one_way_low" ) ),
        ( type: "OneWayPlatform", px: [928, 432], size: [128, 16], fields: ( name: "one_way_high" ) ),
        ( type: "PlayerStart", px: [128, 578], size: [28, 46], fields: ( name: "{rid}_spawn" ) ),
        // The door back to the hall is made with the hall (its `connect_to`).
        ( type: "DebugLabel", px: [112, 48], size: [900, 24], fields: ( name: "{rid}_sign", text: "{name}  (parallax_theme: {key})", category: "Custom" ) ),
    ],
)
'''


#: Painted on the IntGrid layers of each exhibit room: (layer, value, what it
#: is, px, size). The values are those of the layer in the LDtk file.
PAINT = [
    ("Climbable", 1, "a ladder", (608, 384), (32, 240)),
    ("Collision", 3, "a soft blink wall", (544, 528), (32, 96)),
    ("Collision", 4, "a hard blink wall", (880, 528), (32, 96)),
    ("Collision", 5, "a strip of hazard", (736, 608), (96, 16)),
]


def write_specs() -> list[Path]:
    paths = [HERE / f"{HALL}_area.ron"]
    paths[0].write_text(hall_spec())
    for i, (key, name, biome) in enumerate(BIOMES):
        path = HERE / f"{room_id(key)}_area.ron"
        path.write_text(exhibit_spec(i, key, name, biome))
        paths.append(path)
    return paths


def run(module: str, *args: str) -> None:
    print("+", module, " ".join(args), flush=True)
    subprocess.run([sys.executable, "-m", module, *args], check=True)


def apply(paths: list[Path]) -> None:
    # Each room first, with no door. Then the hall, which makes the two doors
    # of each pair: its own, and the one in the room.
    for path in paths[1:] + paths[:1]:
        run("ambition_ldtk_tools", "area", "create", str(path), "--ldtk", LDTK, "--replace-existing")
    for key, _name, _biome in BIOMES:
        for layer, value, _what, (x, y), (w, h) in PAINT:
            run("ambition_ldtk_tools", "intgrid", "paint", "--ldtk", LDTK, "--level", room_id(key),
                "--layer", layer, "--value", str(value), "--px", f"{x},{y}", "--size", f"{w},{h}")
        run("ambition_ldtk_tools.edit.level_set_field", "--ldtk", LDTK, "--level", room_id(key),
            "--set", f"parallax_theme={key}", "--in-place")


def main() -> int:
    paths = write_specs()
    print(f"wrote {len(paths)} specs to {HERE}")
    if "--apply" in sys.argv[1:]:
        apply(paths)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
