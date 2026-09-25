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
"""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

from author_highway_ldtk import (
    REPO, RING_SIZE, chain, badnik, booster, height_at, monitor, points_field,
    rect, ring, ring_arc, rings_over, run_tool, spring, surface_loop,
)

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


def area_spec() -> dict:
    def portal(name: str, x: int, y: int, color: str, normal: str) -> dict:
        # A vertical rift catches a body running across its plane. Its
        # 128px opening fits a standing Sanic body.
        return rect("Portal", (x - 8, y - 64), (16, 128),
                    name=name, color=color, normal=normal)

    entities = [
        rect("PlayerStart", (144, 1594), (28, 46), name="darkness_start"),
        chain("darkness_west", WEST, True),
        chain("darkness_viaduct", VIADUCT, True),
        chain("darkness_vault", VAULT, True),
        chain("darkness_basin", BASIN, True),
        chain("darkness_east", EAST, True),
        chain("darkness_high_west", HIGH_WEST, False),
        chain("darkness_high_east", HIGH_EAST, False),
        surface_loop("darkness_loop_dusk", 2500, 210, "darkness_west", 1810),
        surface_loop("darkness_loop_viaduct", 7100, 185, "darkness_viaduct", 1780),
        surface_loop("darkness_loop_vault", 11800, 220, "darkness_vault", 1500),
        surface_loop("darkness_loop_east", 27200, 230, "darkness_east", 1740),
        # This track faces down. The zone pulls a body to it.
        rect("SurfaceChain", (VAULT_X[0], VAULT_CEILING - 16), (16, 16),
             name="darkness_vault_ceiling",
             points=points_field([(VAULT_X[1], VAULT_CEILING),
                                  (VAULT_X[0], VAULT_CEILING)]), closed=False),
        rect("Solid", (VAULT_X[0], VAULT_CEILING - 64),
             (VAULT_X[1] - VAULT_X[0], 64), name="darkness_vault_roof"),
        rect("GravityZone", (VAULT_X[0] + 80, VAULT_CEILING),
             (VAULT_X[1] - VAULT_X[0] - 160, 240),
             name="darkness_vault_flip", dir="up"),
        # The high roads pass over the lower rifts. A runner on the lower
        # road crosses their plane and exits further east.
        portal("viaduct_entrance", 8150,
               round(height_at(VIADUCT, 8150) - 24), "purple", "left"),
        portal("viaduct_exit", 10100, 1476, "yellow", "right"),
        portal("shadow_entrance", 26500,
               round(height_at(EAST, 26500) - 24), "green", "left"),
        portal("shadow_exit", 29400, 1396, "magenta", "right"),
        # The rift on the vault ceiling leads to the second high road.
        portal("vault_secret_entrance", 11800, VAULT_CEILING + 24, "teal", "left"),
        portal("vault_secret_exit", 22000, 1136, "red", "right"),
        # Springs reach the optional roads. The lower road stays complete.
        spring(5480, height_at(VIADUCT, 5504), 1700),
        spring(21500, height_at(EAST, 21524), 1700),
        booster(800, 1640, 1100),
        booster(3950, height_at(WEST, 3980), 1200),
        booster(7850, height_at(VIADUCT, 7880), 1300),
        booster(10100, 1500, 1050),
        booster(15800, 2000, 1500),
        booster(26300, height_at(EAST, 26330), 1250),
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
        rect("Solid", (FINISH_X, 1164), (32, 256), name="darkness_finish_tower"),
    ]
    for keys, positions in [
        (WEST, [1400, 4550]), (VIADUCT, [6300, 7500, 9000]),
        (VAULT, [13500]), (BASIN, [15300, 17500, 19900]),
        (EAST, [21100, 23900, 25900, 29600, 30600]),
    ]:
        entities += [badnik(keys, x) for x in positions]
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
            monitor("monitor_rings_viaduct", 7700, 1260),
            monitor("monitor_rings_east", 23900, 1160),
        ],
    }


def main() -> None:
    MAP_TARGET.parent.mkdir(parents=True, exist_ok=True)
    run_tool("world", "init", str(MAP_TARGET), "--identifier",
             "ambition-sanic-darkness-world", "--force")
    for entity, field in (
        ("PickupSpawn", "sprite:String:"),
        ("SurfaceLoop", "attach_to:String:"),
        ("SurfaceChain", "fill:Bool:false"),
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
