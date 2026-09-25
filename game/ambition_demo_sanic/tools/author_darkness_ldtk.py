#!/usr/bin/env python3
"""Author Act 3, the dark course.

Run this script from the repository root. It writes the LDtk world through
ambition_ldtk_tools. The course has six sections:

1. The dusk descent builds speed through two loops.
2. A broken viaduct offers a high road above the lower running line.
3. The eclipse vault turns gravity up for a ceiling run.
4. A deep basin stores speed for a long climb and a raised observatory.
5. A second high road crosses a field of hazards.
6. The final descent, loop, and clear lane end the act.

The lower road connects the whole course. High roads give rings and avoid
hazards. They rejoin the lower road before the next section.

The ground is PAINTED: the lower road is `Terrain` cells down to the level's
floor, the high roads are one-cell `Track` bands (you can jump up through
them), and the vault's roof is `Terrain` hanging from above. Open the level in
LDtk to see it; `ambition_ldtk_tools.terrain` rasterizes the key points below
into the slope palette, and the engine traces the painted cells back into
rideable surfaces.
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from author_highway_ldtk import (
    BADNIK_CHARACTER_ID, BADNIK_DISPLAY_NAME, REPO, RING_SIZE, TOOLS, booster, monitor,
    rect, ring, ring_arc, rings_over, run_tool, spring, surface_loop,
)

import sys

sys.path.insert(0, str(TOOLS))
from ambition_ldtk_tools import terrain  # noqa: E402

GRID = 16

ROOM_ID = "sanic_darkness"
NEXT_ROOM = "sanic_speedway"
MUSIC_TRACK = "to_be_superluminal_is_to_live_in_darkness"
LEVEL_W = 32000
LEVEL_H = 2400
TARGET = REPO / "game/ambition_demo_sanic/assets/worlds/sanic_darkness.ldtk"
MAP_TARGET = REPO / "game/ambition_map_assets/ambition_demo_sanic/worlds/sanic_darkness.ldtk"

# Each filled run is a continuous lower road. A flat span holds each loop.
WEST = [
    (0, 1640), (700, 1640), (1300, 1810), (1700, 1810),
    (3500, 1810), (4300, 1580), (5200, 1580),
]
VIADUCT = [
    (5200, 1580), (5900, 1780), (7800, 1780),
    (9900, 1500),
]
VAULT = [
    (9900, 1500), (11200, 1500), (13000, 1500),
    (14000, 1740), (14900, 1740),
]
BASIN = [
    (14900, 1740), (15400, 2000), (16000, 2000),
    (20500, 1450),
]
EAST = [
    (20500, 1450), (21400, 1640), (23200, 1640),
    (24100, 1490), (25500, 1490), (26400, 1740),
    (28000, 1740), (29100, 1420), (LEVEL_W, 1420),
]

HIGH_WEST = [(5600, 1260), (7900, 1260), (9900, 1500)]
HIGH_EAST = [(21700, 1160), (24400, 1160), (25500, 1490)]
VAULT_X = (10500, 12800)
VAULT_CEILING = 1260
OBSERVATORY = (19900, 820, 480)
FINISH_X = LEVEL_W - 120


def rings() -> list[dict]:
    out: list[dict] = []
    for keys, start, end, step in [
        (WEST, 240, 1100, 88), (WEST, 1800, 3400, 90),
        (WEST, 4100, 5050, 90), (VIADUCT, 5300, 5850, 80),
        (VIADUCT, 6500, 7650, 92), (VIADUCT, 8500, 9700, 90),
        (VAULT, 13000, 13900, 90), (BASIN, 15100, 16400, 100),
        (BASIN, 16800, 18000, 95), (BASIN, 19400, 20400, 90),
        (EAST, 20500, 21300, 90), (EAST, 26000, 27800, 90),
        (EAST, 29100, 31500, 90),
    ]:
        out += rings_over(keys, start, end, step=step)
    out += ring_arc(2400, 2750, 1720, 90, 8)
    out += ring_arc(26900, 27300, 1640, 105, 8)
    out += [ring(x, 1224) for x in range(5700, 7900, 74)]
    out += [ring(x, 1124) for x in range(21800, 24400, 78)]
    out += [ring(x, VAULT_CEILING + 38) for x in range(10650, 12750, 85)]
    out += [ring(x, OBSERVATORY[1] - 36)
            for x in range(OBSERVATORY[0] + 24, OBSERVATORY[0] + OBSERVATORY[2] - 24, 44)]
    return out


#: The whole lower road, one profile: the runs share their end points.
LOWER = WEST + VIADUCT[1:] + VAULT[1:] + BASIN[1:] + EAST[1:]
LOWER_HEIGHTS = terrain.quantize(terrain.cosine_profile(LOWER), 0, LEVEL_W // GRID, GRID)
#: The painted lower road's height. Things that stand on the ground stand on
#: THIS, not the key-point curve it was drawn from (up to half a cell away): a
#: booster placed by the curve ended half-buried at the foot of a painted slope.
ground_y = terrain.surface(LOWER_HEIGHTS, 0, GRID)


def high_road_y(keys):
    """The painted top of a high road (a `Track` band drawn from ``keys``)."""
    x0 = keys[0][0] // GRID
    heights = terrain.quantize(terrain.cosine_profile(keys), x0, keys[-1][0] // GRID, GRID)
    return terrain.surface(heights, x0, GRID)


#: A monitor on a high road sits on a shelf a jump above it. On the road itself
#: it was a wall to anyone not rolling, and the vault's portal drops a runner
#: onto the east road: a Right-only run ended against it.
SHELF_W, SHELF_LIFT = 96, 96
SHELVES = {
    "monitor_rings_viaduct": (7660, high_road_y(HIGH_WEST)),
    "monitor_rings_east": (23860, high_road_y(HIGH_EAST)),
}


def shelf(name: str) -> tuple[float, float]:
    """(x, top y) of the shelf that carries monitor ``name``."""
    x, road_y = SHELVES[name]
    return x, road_y(x + SHELF_W / 2) - SHELF_LIFT


def on_ground_badnik(x: float) -> dict:
    return rect(
        "EnemySpawn", (x, ground_y(x + 14) - 48), (28, 32),
        brain=BADNIK_CHARACTER_ID, character_id=BADNIK_CHARACTER_ID, name=BADNIK_DISPLAY_NAME,
    )


def area_spec() -> dict:
    def portal(name: str, x: int, y: int, color: str, normal: str) -> dict:
        # A vertical rift catches a body running across its plane. Its
        # 128px opening fits a standing Sanic body.
        return rect("Portal", (x - 8, y - 64), (16, 128),
                    name=name, color=color, normal=normal)

    entities = [
        rect("PlayerStart", (144, 1594), (28, 46), name="darkness_start"),
        # Each loop attaches to the painted floor under it.
        surface_loop("darkness_loop_dusk", 2500, 210, "terrain", 1808),
        surface_loop("darkness_loop_viaduct", 7100, 185, "terrain", 1776),
        surface_loop("darkness_loop_vault", 11800, 220, "terrain", 1504),
        surface_loop("darkness_loop_east", 27200, 230, "terrain", 1744),
        # The vault's roof is painted; its underside faces down, and the zone
        # pulls a body up onto it.
        rect("GravityZone", (VAULT_X[0] + 80, VAULT_CEILING),
             (VAULT_X[1] - VAULT_X[0] - 160, 240),
             name="darkness_vault_flip", dir="up"),
        # The high roads pass over the lower rifts. A runner on the lower
        # road crosses their plane and exits further east.
        portal("viaduct_entrance", 8150,
               round(ground_y(8150) - 24), "purple", "left"),
        portal("viaduct_exit", 10100, 1476, "yellow", "right"),
        portal("shadow_entrance", 26500,
               round(ground_y(26500) - 24), "green", "left"),
        portal("shadow_exit", 29400, 1396, "magenta", "right"),
        # The rift on the vault ceiling leads to the second high road.
        portal("vault_secret_entrance", 11800, VAULT_CEILING + 24, "teal", "left"),
        portal("vault_secret_exit", 22000, 1136, "red", "right"),
        # Springs reach the optional roads. The lower road stays complete.
        spring(5480, ground_y(5504), 1700),
        spring(21500, ground_y(21524), 1700),
        booster(800, ground_y(830), 1100),
        booster(3950, ground_y(3980), 1200),
        booster(7850, ground_y(7880), 1300),
        booster(10100, ground_y(10130), 1050),
        booster(15800, ground_y(15830), 1500),
        booster(26300, ground_y(26330), 1250),
        # Short hazard groups have a clear gap between them.
        rect("DamageVolume", (6120, 1764), (112, 16), name="viaduct_spikes_a", damage=1),
        rect("DamageVolume", (6830, 1764), (112, 16), name="viaduct_spikes_b", damage=1),
        rect("DamageVolume", (22300, 1624), (96, 16), name="shadow_spikes_a", damage=1),
        rect("DamageVolume", (22800, 1624), (96, 16), name="shadow_spikes_b", damage=1),
        rect("DamageVolume", (23300, 1624), (96, 16), name="shadow_spikes_c", damage=1),
        rect("DamageVolume", (28350, 1632), (96, 16), name="final_spikes", damage=1),
        # A jump reaches the ledge. Its spring lifts Sanic to the
        # observatory above the basin's far rim.
        rect("OneWayPlatform", (19580, 1350), (180, 16), name="observatory_ledge"),
        spring(19650, 1350, 1450),
        rect("OneWayPlatform", (OBSERVATORY[0], OBSERVATORY[1]),
             (OBSERVATORY[2], 16), name="observatory"),
        *[rect("OneWayPlatform", shelf(name), (SHELF_W, 16), name=f"{name}_shelf")
          for name in SHELVES],
    ]
    for keys, positions in [
        (WEST, [1400, 4550]), (VIADUCT, [6300, 7500, 9000]),
        (VAULT, [13500]), (BASIN, [15300, 17500, 19900]),
        (EAST, [21100, 23900, 25900, 29600, 30600]),
    ]:
        entities += [on_ground_badnik(x) for x in positions]
    entities += rings()
    return {
        "id": ROOM_ID, "level_id": ROOM_ID, "world_x": 0, "world_y": 0,
        "px_wid": LEVEL_W, "px_hei": LEVEL_H,
        "fill_collision": "empty", "bg_color": "#090b1d",
        "mode": "sanic", "music_track": MUSIC_TRACK, "entities": entities,
    }


def named_blocks() -> dict:
    return {
        "level_id": ROOM_ID,
        "entities": [
            monitor("monitor_rings_observatory", OBSERVATORY[0] + 100, OBSERVATORY[1]),
            monitor("monitor_speed_observatory", OBSERVATORY[0] + 260, OBSERVATORY[1]),
            *[monitor(name, shelf(name)[0] + 35, shelf(name)[1]) for name in SHELVES],
        ],
    }


def painted() -> dict:
    """The level's painted cells: `Terrain` (the lower road, the vault roof,
    the finish tower) and `Track` (the two high roads)."""
    columns, rows = LEVEL_W // GRID, LEVEL_H // GRID
    lower = terrain.cosine_profile(LOWER)
    cells = terrain.ground(lower, 0, columns, GRID, bottom=rows)
    # How far the painted road strays from the key-point curve, reported so a
    # change to the keys that the palette cannot follow is seen.
    heights = LOWER_HEIGHTS
    worst = max(abs(h * GRID / terrain.Q - lower(i * GRID)) for i, h in enumerate(heights))
    print(f"lower road: {len(cells)} cells, within {worst:.1f}px of its curve")
    assert worst < GRID, "the lower road is steeper than 45° somewhere"
    # The vault roof: four cells of rock whose underside is the ceiling run.
    roof_underside = terrain.ceiling(
        lambda x: VAULT_CEILING, VAULT_X[0] // GRID, VAULT_X[1] // GRID, GRID,
        top=(VAULT_CEILING - 64) // GRID,
    )
    cells.update(roof_underside)
    # The finish tower stands on the road: a wall a runner meets.
    tower_foot = heights[FINISH_X // GRID] // terrain.Q
    for cx in range(FINISH_X // GRID, (FINISH_X + 32) // GRID):
        for cy in range(1164 // GRID, tower_foot):
            cells[(cx, cy)] = terrain.FULL
    track: dict = {}
    for keys in (HIGH_WEST, HIGH_EAST):
        x0, x1 = keys[0][0] // GRID, keys[-1][0] // GRID
        track.update(terrain.band(terrain.cosine_profile(keys), x0, x1, GRID, thickness=1))
    return {
        "level_id": ROOM_ID,
        "layers": {
            "Terrain": [[cx, cy, v] for (cx, cy), v in sorted(cells.items())],
            "Track": [[cx, cy, v] for (cx, cy), v in sorted(track.items())],
        },
    }


def main() -> None:
    MAP_TARGET.parent.mkdir(parents=True, exist_ok=True)
    run_tool("world", "init", str(MAP_TARGET), "--identifier",
             "ambition-sanic-darkness-world", "--force")
    for entity, field in (
        ("PickupSpawn", "sprite:String:"),
        ("SurfaceLoop", "attach_to:String:"),
    ):
        run_tool("def", "update-entity", entity, str(MAP_TARGET),
                 "--add-field", field, "--in-place", "--no-repair")
    with tempfile.TemporaryDirectory() as tmp:
        area = Path(tmp) / "darkness_area.json"
        area.write_text(json.dumps(area_spec(), indent=2))
        run_tool("area", "create", str(area), "--ldtk", str(MAP_TARGET))
        blocks = Path(tmp) / "darkness_named_blocks.json"
        blocks.write_text(json.dumps(named_blocks(), indent=2))
        run_tool("entity", "add", str(blocks), "--ldtk", str(MAP_TARGET), "--in-place")
        cells = Path(tmp) / "darkness_terrain.json"
        cells.write_text(json.dumps(painted()))
        run_tool("terrain", "paint", str(cells), "--ldtk", str(MAP_TARGET))
    run_tool("level", "add-field-def", "next_room", "--type", "String",
             str(MAP_TARGET), "--in-place")
    run_tool("level", "set-field", "--ldtk", str(MAP_TARGET), "--level", ROOM_ID,
             "--set", f"next_room={NEXT_ROOM}", "--in-place")
    run_tool("repair", str(MAP_TARGET), "--in-place")
    run_tool("validate", str(MAP_TARGET))
    if not TARGET.is_symlink():
        TARGET.parent.mkdir(parents=True, exist_ok=True)
        TARGET.symlink_to(Path("../../../ambition_map_assets/ambition_demo_sanic/worlds/sanic_darkness.ldtk"))


if __name__ == "__main__":
    main()
