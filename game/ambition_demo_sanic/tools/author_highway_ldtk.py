#!/usr/bin/env python3
"""Author game/ambition_demo_sanic/assets/worlds/sanic_highway.ldtk — Act 2.

Generated through the sanctioned `ambition_ldtk_tools` pipeline, like Act 1
(`author_speedway_ldtk.py`); the .ldtk JSON is never hand-edited. Re-run from
the repo root after changing the layout below:

    python3 game/ambition_demo_sanic/tools/author_highway_ldtk.py

Act 2 is a course with height: the ground rolls, climbs and drops across a
room twice as tall as the screen. The ground is PAINTED (`Terrain` cells, the
sky bridge a one-cell `Track`, the tunnel roof and the finish tower `Terrain`
too): open the level in LDtk to see it. The key points below are rasterized
into the slope palette by `ambition_ldtk_tools.terrain`. Every loop is data
(`SurfaceLoop` attached to the painted floor under it).

The course, left to right:

  1. Rolling descent from the start plateau into loop A.
  2. The big climb to a kicker lip. SPEED DECIDES THE ROUTE: fast off the lip,
     he clears the chasm onto the sky bridge (loop B, rings, quick); slow, he
     drops into the valley (badniks walking its slopes, a spike dip).
  3. SECRET: at the valley's west end, the cliff face has a breakable wall.
     Roll into it and it opens onto a cave in the rock: a ring monitor, a
     speed monitor and a ring stash.
  4. Both routes meet and run into the upside-down tunnel: a GravityZone
     points gravity up and he rides the ceiling.
  5. Loop D (the big one), then a plunge into a halfpipe and a climb you need
     speed for. A booster at the bottom helps; too slow and you roll back.
  6. SECRET: on the plateau, a ledge over the running line carries a spring
     that throws him up to a sky island with a ring monitor.
  7. The spike gauntlet, and the finish.
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
sys.path.insert(0, str(TOOLS))
from ambition_ldtk_tools import terrain  # noqa: E402

GRID = 16
TARGET = REPO / "game" / "ambition_demo_sanic" / "assets" / "worlds" / "sanic_highway.ldtk"
MAP_ASSETS_TARGET = REPO / "game" / "ambition_map_assets" / "ambition_demo_sanic" / "worlds" / "sanic_highway.ldtk"

ROOM_ID = "sanic_highway"
# Act 2 leads to the dark third act.
NEXT_ROOM = "sanic_darkness"
MUSIC_TRACK = "velocity"

LEVEL_W = 16000  # a multiple of the 16px grid
LEVEL_H = 1600


# ── The ground, as key points joined by smooth (cosine) curves ─────────────
# (x, y), y down. Each run is painted as ground cells from these key points.

# West: start plateau, rolling descent, loop A's flat, the big climb, the lip.
WEST = [
    (0, 1100), (700, 1100), (1000, 1050), (1300, 1180), (1600, 1130),
    (1950, 1300), (2300, 1300),
    # Loop A's flat (2300..3900).
    (3900, 1300),
    # The big climb, and the kicker lip: its last rise is what throws a fast
    # runner across the chasm.
    (4700, 900), (4800, 895), (4850, 880),
]
LIP_X = 4850

# The valley below the chasm: the slow road. Its west end runs on under the
# cliff into the secret cave, behind the breakable wall.
VALLEY = [
    (3900, 1450), (4850, 1450), (5300, 1450), (5500, 1400), (5700, 1450),
    (5760, 1510), (6000, 1510), (6060, 1450),
    (6500, 1420), (6900, 1450), (7500, 1200),
]
VALLEY_SPIKES = (5760, 6000, 1510)  # a dip lined with spikes

# The sky bridge: the fast road. A track, not ground, so it is not filled.
BRIDGE = [(5250, 960), (6800, 960), (7500, 1200)]

# East: the tunnel, loop D, the halfpipe, the plateau, the gauntlet, the finish.
EAST = [
    (7500, 1200), (9100, 1200),
    # Loop D's flat (9100..10800).
    (10800, 1200),
    # The plunge and the halfpipe.
    (11500, 1480), (11800, 1480),
    # The climb you need speed for.
    (12500, 1000),
    (13400, 1000),
    (14000, 1200), (LEVEL_W, 1200),
]

# (centre_x, radius, chain) — each attaches to the chain its span lies on.
LOOP_A = (3050, 200, "highway_west")
LOOP_B = (6000, 170, "highway_bridge")
LOOP_D = (9900, 240, "highway_east")

# The upside-down tunnel: a ceiling he rides with gravity pointing UP.
TUNNEL = (7800, 9000)
TUNNEL_FLOOR = 1200
TUNNEL_CEILING = TUNNEL_FLOOR - 176

# Secrets.
CAVE = (3960, 4850, 1450)  # x0, x1, floor y — inside the rock of the climb
CAVE_WALL_X = 4850
# The sky spring stands on a ledge above the running line, so a runner passes
# under it; only a player who jumps up to the ledge is thrown to the island.
SKY_LEDGE = (13160, 860, 160)  # x, y, width
SKY_ISLAND = (13280, 480, 420)  # x, y, width

FINISH_TOWER_X = 15880

BADNIK_CHARACTER_ID = "sanic_badnik"
BADNIK_DISPLAY_NAME = "Sanic Badnik"
RING_SIZE = (18, 18)  # the same side as Act 1 and a scattered ring
RING_SPRITE = "sanic_ring_prop"


def height_at(keys: list[tuple[float, float]], x: float) -> float:
    for (x0, y0), (x1, y1) in zip(keys, keys[1:]):
        if x0 <= x <= x1:
            if abs(y1 - y0) < 0.5 or x1 == x0:
                return y0
            t = (x - x0) / (x1 - x0)
            return y0 + (y1 - y0) * (1 - math.cos(math.pi * t)) / 2
    raise ValueError(f"x {x} is off the run")


def rect(etype: str, px: tuple[float, float], size: tuple[float, float], **fields) -> dict:
    entry: dict = {"type": etype, "px": [int(px[0]), int(px[1])], "size": [int(size[0]), int(size[1])]}
    if fields:
        entry["fields"] = fields
    return entry


def ring(cx: float, cy: float) -> dict:
    return rect(
        "PickupSpawn",
        (round(cx - RING_SIZE[0] / 2), round(cy - RING_SIZE[1] / 2)),
        RING_SIZE,
        name="ring",
        kind="currency:1",
        sprite=RING_SPRITE,
    )


def rings_over(keys: list[tuple[float, float]], x0: float, x1: float, lift: float = 36, step: float = 60) -> list[dict]:
    """A line of rings following the ground `lift` above it."""
    out, x = [], x0
    while x <= x1 + 1e-3:
        out.append(ring(x, height_at(keys, x) - lift))
        x += step
    return out


def ring_arc(x0: float, x1: float, cy: float, rise: float, n: int) -> list[dict]:
    return [
        ring(x0 + (x1 - x0) * i / (n - 1), cy - rise * math.sin(math.pi * i / (n - 1)))
        for i in range(n)
    ]


def surface_loop(name: str, centre_x: float, radius: float, attach_to: str, floor) -> dict:
    """A loop whose box is its circle, drawn where it is built.

    The ramp's foot is the converter's default `approach` left of the centre,
    and the loop's bottom stands the speedway's rise above the floor there
    (``floor(x)`` is that floor's painted height)."""
    ramp_x = centre_x - radius * 460 / 180
    cy = floor(ramp_x) - radius * 84 / 180 - radius
    return rect(
        "SurfaceLoop", (round(centre_x - radius), round(cy - radius)), (round(2 * radius),) * 2,
        name=name, attach_to=attach_to,
    )


def badnik(ground_y, x: float) -> dict:
    # Dropped just above the ground it will walk: a badnik rides the surface
    # now, so a spawn inside a hill would walk the floor under it.
    return rect(
        "EnemySpawn",
        (x, ground_y(x + 14) - 48),
        (28, 32),
        brain=BADNIK_CHARACTER_ID,
        character_id=BADNIK_CHARACTER_ID,
        name=BADNIK_DISPLAY_NAME,
    )


def spring(x: float, floor_y: float, up: float) -> dict:
    return rect("ReboundPad", (x, floor_y - 24), (48, 24), impulseX=0, impulseY=-up)


def booster(x: float, floor_y: float, push: float) -> dict:
    # Mostly ALONG the floor: to a riding body that only ever raises speed.
    return rect("ReboundPad", (x, floor_y - 22), (72, 22), impulseX=push, impulseY=-120)


def monitor(name: str, x: float, floor_y: float) -> dict:
    return rect("OneWayPlatform", (x, floor_y - 26), (26, 26), name=name)


def ring_placements() -> list[dict]:
    rings: list[dict] = []
    rings += rings_over(WEST, 240, 560)
    rings += rings_over(WEST, 1000, 1900, step=90)
    rings += ring_arc(2500, 2800, 1230, 50, 6)
    rings += rings_over(WEST, 3950, 4600, step=80)  # up the climb
    # The fast road: the arc off the lip, the bridge.
    rings += ring_arc(4900, 5200, 860, 70, 6)
    rings += [ring(x, 924) for x in range(5300, 5700, 60)]
    rings += [ring(x, 924) for x in range(6500, 6780, 60)]
    # The slow road.
    rings += rings_over(VALLEY, 4950, 5500, step=90)
    rings += ring_arc(5770, 5990, 1420, 60, 5)  # over the spike dip
    # The secret cave's stash.
    rings += [ring(x, CAVE[2] - 36) for x in range(4000, 4400, 50)]
    rings += [ring(x, CAVE[2] - 90) for x in range(4025, 4400, 50)]
    # Along the tunnel ceiling (upside down).
    rings += [ring(x, TUNNEL_CEILING + 36) for x in range(7900, 8950, 80)]
    rings += rings_over(EAST, 11000, 11700, step=100)
    rings += rings_over(EAST, 12600, 13100, step=80)
    # The sky island.
    x, y, w = SKY_ISLAND
    rings += [ring(xx, y - 30) for xx in range(x + 20, x + w - 60, 50)]
    rings += ring_arc(14350, 14850, 1100, 80, 7)  # over the gauntlet
    rings += rings_over(EAST, 15000, 15500, step=80)
    return rings


def painted_run(keys: list[tuple[float, float]]):
    """One run of ground as painted: its columns, heights, and ``y(x)``."""
    x0, x1 = int(keys[0][0]) // GRID, int(keys[-1][0]) // GRID
    heights = terrain.quantize(terrain.cosine_profile(keys), x0, x1, GRID)
    return x0, x1, heights, terrain.surface(heights, x0, GRID)


# The painted height of each run. Things that stand on the ground stand on
# THESE, not on the key-point curves (up to half a cell away).
west_y = painted_run(WEST)[3]
valley_y = painted_run(VALLEY)[3]
east_y = painted_run(EAST)[3]


def painted() -> dict:
    """The level's cells: the three ground runs, the carved cave, the tunnel
    roof and the finish tower on `Terrain`; the sky bridge on `Track`."""
    rows = LEVEL_H // GRID
    cells: dict = {}
    for keys in (WEST, VALLEY, EAST):
        x0, x1, _, _ = painted_run(keys)
        cells.update(terrain.ground(terrain.cosine_profile(keys), x0, x1, GRID, bottom=rows))
    # SECRET 1: the cave, carved out of the climb's rock down to the valley
    # floor that runs on under the cliff. Its mouth is the cliff face at
    # CAVE_WALL_X, closed by the breakable wall.
    cave_floor_row = int(valley_y(CAVE[0])) // GRID
    for cx in range(CAVE[0] // GRID, CAVE_WALL_X // GRID):
        for cy in range(cave_floor_row - 7, cave_floor_row):
            cells.pop((cx, cy), None)
    # The tunnel roof: four cells of rock whose underside is the ceiling run.
    cells.update(terrain.ceiling(
        lambda x: TUNNEL_CEILING, TUNNEL[0] // GRID, TUNNEL[1] // GRID, GRID,
        top=(TUNNEL_CEILING - 64) // GRID,
    ))
    # The finish tower stands on the road: a wall a runner meets.
    for cx in range(FINISH_TOWER_X // GRID, (FINISH_TOWER_X + 32) // GRID):
        for cy in range(944 // GRID, int(east_y(cx * GRID)) // GRID):
            cells[(cx, cy)] = terrain.FULL
    bridge = terrain.band(
        terrain.cosine_profile(BRIDGE), int(BRIDGE[0][0]) // GRID, int(BRIDGE[-1][0]) // GRID,
        GRID, thickness=1,
    )
    return {
        "level_id": ROOM_ID,
        "layers": {
            "Terrain": [[cx, cy, v] for (cx, cy), v in sorted(cells.items())],
            "Track": [[cx, cy, v] for (cx, cy), v in sorted(bridge.items())],
        },
    }


def area_spec() -> dict:
    x_island, y_island, w_island = SKY_ISLAND
    entities = [
        rect("PlayerStart", (146, 1100 - 46), (28, 46), name="highway_start"),
        # Each loop attaches to the painted floor under it (loop B: the bridge).
        surface_loop("highway_loop_a", LOOP_A[0], LOOP_A[1], "terrain", west_y),
        surface_loop("highway_loop_b", LOOP_B[0], LOOP_B[1], "terrain", lambda x: 960),
        surface_loop("highway_loop_d", LOOP_D[0], LOOP_D[1], "terrain", east_y),
        # The upside-down tunnel: the painted roof's underside faces down, into
        # the tunnel; the GravityZone points gravity up inside, so he falls
        # onto it and rides it. Leaving the zone gives gravity back and he
        # drops to the floor.
        rect("GravityZone", (TUNNEL[0] + 80, TUNNEL_CEILING), (TUNNEL[1] - TUNNEL[0] - 160, TUNNEL_FLOOR - TUNNEL_CEILING), name="tunnel_flip", dir="up"),
        # Boosters: into the climb, into loop D, and at the halfpipe's bottom.
        booster(3700, west_y(3730), 1300),
        booster(9300, east_y(9330), 1120),
        booster(11620, east_y(11650), 1400),
        # The valley's way back up to the others: a spring at its east end.
        spring(7000, valley_y(7024), 1000),
        # SECRET 2: a ledge over the running line with a spring on it, and the
        # island the spring reaches.
        rect("OneWayPlatform", (SKY_LEDGE[0], SKY_LEDGE[1]), (SKY_LEDGE[2], 16), name="sky_ledge"),
        spring(SKY_LEDGE[0] + SKY_LEDGE[2] / 2 - 24, SKY_LEDGE[1], 1450),
        rect("OneWayPlatform", (x_island, y_island), (w_island, 16), name="sky_island"),
        # The valley's spike dip, and the gauntlet before the finish.
        rect("DamageVolume", (VALLEY_SPIKES[0] + 10, valley_y((VALLEY_SPIKES[0] + VALLEY_SPIKES[1]) / 2) - 16), (VALLEY_SPIKES[1] - VALLEY_SPIKES[0] - 20, 16), name="valley_spikes", damage=1),
        spring(14250, east_y(14274), 800),
        rect("DamageVolume", (14400, 1184), (96, 16), name="gauntlet_spikes_1", damage=1),
        rect("DamageVolume", (14580, 1184), (96, 16), name="gauntlet_spikes_2", damage=1),
        rect("DamageVolume", (14760, 1184), (64, 16), name="gauntlet_spikes_3", damage=1),
        # Badniks walk the slopes now: on the rolling descent, the valley, the
        # plateau and the run-in.
        badnik(west_y, 1180),
        badnik(west_y, 1700),
        badnik(valley_y, 5400),
        badnik(valley_y, 6400),
        badnik(east_y, 12900),
        badnik(east_y, 15100),
    ]
    entities += ring_placements()
    return {
        "id": ROOM_ID,
        "level_id": ROOM_ID,
        "world_x": 0,
        "world_y": 0,
        "px_wid": LEVEL_W,
        "px_hei": LEVEL_H,
        "fill_collision": "empty",
        "bg_color": "#0f1a2a",
        "mode": "sanic",
        "music_track": MUSIC_TRACK,
        "entities": entities,
    }


def named_blocks() -> dict:
    """Blocks a rule finds BY NAME, added after the area is built.

    `area create` lowers static-collision nouns (`Solid`, `OneWayPlatform`) into
    the IntGrid, which keeps their shape and drops their name. A monitor or a
    breakable wall is found by its name, so these are added as entities.
    """
    x_island, y_island, w_island = SKY_ISLAND
    return {
        "level_id": ROOM_ID,
        "entities": [
            # SECRET 1: the cave in the climb's rock, behind a wall a roll breaks.
            rect("Solid", (CAVE_WALL_X, valley_y(CAVE_WALL_X + 16) - 112), (32, 112), name="breakable_cave_wall"),
            monitor("monitor_rings_cave", 4460, valley_y(4473)),
            monitor("monitor_speed_cave", 4560, valley_y(4573)),
            # SECRET 2's reward.
            monitor("monitor_rings_sky", x_island + w_island - 50, y_island),
        ],
    }


def run_tool(*args: str) -> None:
    cmd = [sys.executable, "-m", "ambition_ldtk_tools", *args]
    print("::", " ".join(str(a) for a in args))
    env = {"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"}
    result = subprocess.run(cmd, cwd=REPO, env=env)
    if result.returncode != 0:
        sys.exit(f"tool step failed: {' '.join(args)}")


def main() -> None:
    # The world lives in the map-assets submodule and is reached through a
    # tracked symlink (see `real_target` in author_speedway_ldtk.py): generate
    # into the REAL path and keep the link.
    target = MAP_ASSETS_TARGET
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        target.unlink()
    run_tool("world", "init", str(target), "--identifier", "ambition-sanic-highway-world")
    # The shared defs lack the ring's sheet field; extend it first.
    run_tool("def", "update-entity", "PickupSpawn", str(target), "--add-field", "sprite:String:",
             "--in-place", "--no-repair")
    with tempfile.TemporaryDirectory() as tmp:
        spec = Path(tmp) / "sanic_highway_area.json"
        spec.write_text(json.dumps(area_spec(), indent=2))
        run_tool("area", "create", str(spec), "--ldtk", str(target))
        blocks = Path(tmp) / "sanic_highway_named_blocks.json"
        blocks.write_text(json.dumps(named_blocks(), indent=2))
        run_tool("entity", "add", str(blocks), "--ldtk", str(target), "--in-place")
        cells = Path(tmp) / "sanic_highway_terrain.json"
        cells.write_text(json.dumps(painted()))
        run_tool("terrain", "paint", str(cells), "--ldtk", str(target))
    # A loop's box is its circle, big enough to cover the rings around it:
    # loops live on their own layer, drawn under the rest.
    run_tool("entity", "change-layer", str(target), "--identifier", "SurfaceLoop",
             "--from-layer", "Ambition", "--to-layer", "AmbitionLoops", "--in-place")
    # After the area: this step validates the project, and a project with no
    # levels does not validate.
    run_tool("level", "add-field-def", "next_room", "--type", "String", str(target), "--in-place")
    run_tool("level", "set-field", "--ldtk", str(target), "--level", ROOM_ID, "--set", f"next_room={NEXT_ROOM}", "--in-place")
    run_tool("repair", str(target), "--in-place")
    run_tool("validate", str(target))
    if not TARGET.is_symlink():
        TARGET.parent.mkdir(parents=True, exist_ok=True)
        TARGET.symlink_to(Path("../../../ambition_map_assets/ambition_demo_sanic/worlds/sanic_highway.ldtk"))
    print(f"authored {target.relative_to(REPO)}")


if __name__ == "__main__":
    main()
