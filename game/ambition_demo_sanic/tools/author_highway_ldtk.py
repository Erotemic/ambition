#!/usr/bin/env python3
"""Author game/ambition_demo_sanic/assets/worlds/sanic_highway.ldtk — Act 2.

Generated through the sanctioned `ambition_ldtk_tools` pipeline, like Act 1
(`author_speedway_ldtk.py`); the .ldtk JSON is never hand-edited. Re-run from
the repo root after changing the layout below:

    python3 game/ambition_demo_sanic/tools/author_highway_ldtk.py

Unlike Act 1, EVERY loop here is data: a `SurfaceLoop` marker with
`attach_to` naming the floor chain it joins. The engine builds the ramp, the
full revolution, the crossover deck and the runout, and splits the floor where
they meet it (`ae::World::attach_loop`), so this script places a loop by its
centre and radius and nothing else.

The course, left to right:
  hills → loop A → spring tower → pit 1 → loops B+C back to back → hills
  → pit 2 (spring + stepping stones) → the upside-down tunnel (a GravityZone
  flips gravity and he rides the ceiling) → loop D (the big one) → spike
  gauntlet → finish.
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
TARGET = REPO / "game" / "ambition_demo_sanic" / "assets" / "worlds" / "sanic_highway.ldtk"
MAP_ASSETS_TARGET = REPO / "game" / "ambition_map_assets" / "ambition_demo_sanic" / "worlds" / "sanic_highway.ldtk"

ROOM_ID = "sanic_highway"
# Where this act's goal leads: back to Act 1, so the demo is a two-act cycle.
NEXT_ROOM = "sanic_speedway"
MUSIC_TRACK = "velocity"

LEVEL_W = 15040  # a multiple of the 16px grid
LEVEL_H = 720
FLOOR_TOP = 672

PIT_1 = (4400, 4700)
PIT_2 = (9200, 9500)

# (centre_x, radius) — each attaches to the floor chain its span lies on.
LOOP_A = (2400, 200)
LOOP_B = (5700, 180)
LOOP_C = (7120, 220)
LOOP_D = (12500, 240)

# The upside-down tunnel: a ceiling he rides with gravity pointing UP.
TUNNEL = (10200, 11400)
TUNNEL_CEILING = FLOOR_TOP - 176

HILL_STEP = 25.0
HILLS_WEST = [(300.0, 800.0, 80.0), (800.0, 1400.0, 64.0)]
HILLS_EAST = [(8100.0, 8600.0, 70.0), (8600.0, 9100.0, 90.0)]

BADNIK_CHARACTER_ID = "sanic_badnik"
BADNIK_DISPLAY_NAME = "Sanic Badnik"
RING_SIZE = (18, 18)  # the same side as Act 1 and a scattered ring
RING_SPRITE = "sanic_ring_prop"


def hill_points(start: float, end: float, amplitude: float) -> list[tuple[float, float]]:
    out = []
    x = start + HILL_STEP
    while x < end - HILL_STEP / 2:
        t = (x - start) / (end - start)
        out.append((round(x, 1), round(FLOOR_TOP - amplitude * math.sin(math.pi * t) ** 2, 1)))
        x += HILL_STEP
    return out


def floor_points(start: float, end: float, hills: list[tuple[float, float, float]]) -> list[tuple[float, float]]:
    pts: list[tuple[float, float]] = [(start, FLOOR_TOP)]
    for hs, he, amp in hills:
        pts.append((hs, FLOOR_TOP))
        pts += hill_points(hs, he, amp)
        pts.append((he, FLOOR_TOP))
    pts.append((end, FLOOR_TOP))
    # Adjacent hills share an endpoint; keep one.
    dedup: list[tuple[float, float]] = []
    for p in pts:
        if not dedup or abs(dedup[-1][0] - p[0]) > 0.5:
            dedup.append(p)
    return dedup


def points_field(points: list[tuple[float, float]]) -> str:
    return "; ".join(f"{x:g},{y:g}" for x, y in points)


def rect(etype: str, px: tuple[int, int], size: tuple[int, int], **fields) -> dict:
    entry: dict = {"type": etype, "px": [int(px[0]), int(px[1])], "size": list(size)}
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


def ring_line(x0: float, x1: float, cy: float, step: float = 60.0) -> list[dict]:
    out, x = [], x0
    while x <= x1 + 1e-3:
        out.append(ring(x, cy))
        x += step
    return out


def ring_arc(x0: float, x1: float, cy: float, rise: float, n: int) -> list[dict]:
    return [
        ring(x0 + (x1 - x0) * i / (n - 1), cy - rise * math.sin(math.pi * i / (n - 1)))
        for i in range(n)
    ]


def surface_loop(name: str, floor: str, centre_x: float, radius: float) -> dict:
    # The marker's centre x is the loop's; the floor decides its height.
    cy = FLOOR_TOP - 84 - radius
    return rect("SurfaceLoop", (centre_x - 8, cy - 8), (16, 16), name=name, radius=radius, attach_to=floor)


def badnik(x: int) -> dict:
    return rect(
        "EnemySpawn",
        (x, 640),
        (28, 32),
        brain=BADNIK_CHARACTER_ID,
        character_id=BADNIK_CHARACTER_ID,
        name=BADNIK_DISPLAY_NAME,
    )


def ring_placements() -> list[dict]:
    rings: list[dict] = []
    rings += ring_line(220, 280, 640)
    rings += ring_arc(420, 700, 620, 60, 5)
    rings += ring_arc(900, 1300, 620, 48, 6)
    rings += ring_line(3220, 3300, 640)
    rings += ring_arc(3500, 3900, 620, 70, 6)  # off the booster
    rings += ring_arc(4420, 4680, 600, 90, 5)  # over pit 1
    rings += ring_line(8150, 8250, 620)
    rings += ring_arc(8650, 9050, 600, 70, 6)
    rings += ring_arc(9220, 9480, 560, 120, 5)  # over pit 2
    rings += ring_line(10300, 11300, TUNNEL_CEILING + 36, 80.0)  # along the ceiling
    rings += ring_line(13600, 13700, 640)
    rings += ring_arc(13780, 14140, 560, 90, 5)  # over the spike gauntlet
    rings += ring_line(14300, 14500, 640)
    return rings


def area_spec() -> dict:
    west = floor_points(0, PIT_1[0], HILLS_WEST)
    middle = floor_points(PIT_1[1], PIT_2[0], HILLS_EAST)
    east = floor_points(PIT_2[1], LEVEL_W, [])
    entities = [
        rect("PlayerStart", (146, 626), (28, 46), name="highway_start"),
        # The momentum floor, one chain per ground run; the pits split it.
        rect("SurfaceChain", (0, 656), (16, 16), name="highway_floor_west", points=points_field(west), closed=False),
        rect("SurfaceChain", (PIT_1[1], 656), (16, 16), name="highway_floor_middle", points=points_field(middle), closed=False),
        rect("SurfaceChain", (PIT_2[1], 656), (16, 16), name="highway_floor_east", points=points_field(east), closed=False),
        # Loops are data: each names the floor it joins.
        surface_loop("highway_loop_a", "highway_floor_west", *LOOP_A),
        surface_loop("highway_loop_b", "highway_floor_middle", *LOOP_B),
        surface_loop("highway_loop_c", "highway_floor_middle", *LOOP_C),
        surface_loop("highway_loop_d", "highway_floor_east", *LOOP_D),
        # The ceiling of the upside-down tunnel, authored RIGHT→LEFT so its
        # riding side faces down, into the tunnel. The GravityZone below points
        # gravity up inside the tunnel, so he falls onto it and rides it; leaving
        # the zone gives gravity back and he drops to the floor.
        rect(
            "SurfaceChain",
            (TUNNEL[0], TUNNEL_CEILING - 16),
            (16, 16),
            name="highway_tunnel_ceiling",
            points=points_field([(TUNNEL[1], TUNNEL_CEILING), (TUNNEL[0], TUNNEL_CEILING)]),
            closed=False,
        ),
        rect("Solid", (TUNNEL[0], TUNNEL_CEILING - 32), (TUNNEL[1] - TUNNEL[0], 32), name="tunnel_roof"),
        rect("GravityZone", (TUNNEL[0] + 80, TUNNEL_CEILING), (TUNNEL[1] - TUNNEL[0] - 160, FLOOR_TOP - TUNNEL_CEILING), name="tunnel_flip", dir="up"),
        # Solid ground, split around the pits; the finish tower caps the run.
        rect("Solid", (0, FLOOR_TOP), (PIT_1[0], 48), name="highway_ground_west"),
        rect("Solid", (PIT_1[1], FLOOR_TOP), (PIT_2[0] - PIT_1[1], 48), name="highway_ground_middle"),
        rect("Solid", (PIT_2[1], FLOOR_TOP), (LEVEL_W - PIT_2[1], 48), name="highway_ground_east"),
        rect("Solid", (14880, 416), (32, 256), name="highway_finish_tower"),
        rect("HazardBlock", (PIT_1[0], 704), (PIT_1[1] - PIT_1[0], 16), name="pit_1_hazard"),
        rect("HazardBlock", (PIT_2[0], 704), (PIT_2[1] - PIT_2[0], 16), name="pit_2_hazard"),
        # A speed booster after loop A, then a clean run-up to pit 1's launcher.
        # (A vertical spring here threw a full-speed runner into the pit.)
        rect("ReboundPad", (3350, 650), (72, 22), impulseX=1400, impulseY=-120),
        # Pit launchers. A pad whose impulse is mostly ALONG the floor is a speed
        # booster to a riding body (it only ever raises speed), so a launcher
        # must be a spring: straight up, keeping the run speed he arrives with.
        rect("ReboundPad", (4300, 648), (48, 24), impulseX=0, impulseY=-820),
        rect("ReboundPad", (9080, 648), (48, 24), impulseX=0, impulseY=-820),
        # A stepping stone over pit 1 for a runner who arrives slow.
        rect("OneWayPlatform", (4520, 560), (64, 16), name="pit_1_stone"),
        # Stepping stones over pit 2 for a runner who arrives slow.
        rect("OneWayPlatform", (9250, 560), (64, 16), name="pit_2_stone_1"),
        rect("OneWayPlatform", (9390, 520), (64, 16), name="pit_2_stone_2"),
        # Loop D's feed booster.
        rect("ReboundPad", (11700, 650), (72, 22), impulseX=1120, impulseY=-260),
        # The spike gauntlet, and the pad that clears it.
        rect("ReboundPad", (13720, 648), (48, 24), impulseX=700, impulseY=-760),
        rect("DamageVolume", (13800, 656), (96, 16), name="gauntlet_spikes_1", damage=1),
        rect("DamageVolume", (13980, 656), (96, 16), name="gauntlet_spikes_2", damage=1),
        rect("DamageVolume", (14160, 656), (64, 16), name="gauntlet_spikes_3", damage=1),
        *(badnik(x) for x in (3560, 3780, 8000, 9700, 11900, 14350)),
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
    # The shared defs lack the fields these entities author; extend them first.
    for entity, field in (("PickupSpawn", "sprite:String:"), ("SurfaceLoop", "attach_to:String:")):
        run_tool("def", "update-entity", entity, str(target), "--add-field", field, "--in-place", "--no-repair")
    with tempfile.TemporaryDirectory() as tmp:
        spec = Path(tmp) / "sanic_highway_area.json"
        spec.write_text(json.dumps(area_spec(), indent=2))
        run_tool("area", "create", str(spec), "--ldtk", str(target))
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
