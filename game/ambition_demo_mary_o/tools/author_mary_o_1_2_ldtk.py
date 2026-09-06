#!/usr/bin/env python3
"""Author World 1-2 into the Mary-O LDtk project through `ambition_ldtk_tools`.

The script applies the declared entity-definition/spec edits and delegates final
project normalization to the shared LDtk tooling."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_demo_mary_o" / "assets" / "worlds" / "mary_o.ldtk"

AREA = "mary_o_1_2"
SURFACE_AREA = "mary_o_1_1"

# ── Mirrored from game/ambition_demo_mary_o/src/level_1_2.rs (see the header) ──
T = 32
WIDTH_TILES = 56  # the playable corridor, between the end walls
HEIGHT_TILES = 14
SLAB_TILES = 2  # roof, floor and end-wall thickness
CHASM = (28, 33)  # corridor columns with no floor under them

X0 = SLAB_TILES * T  # the inside face of the left wall
LEVEL_W = (WIDTH_TILES + 2 * SLAB_TILES) * T
LEVEL_H = HEIGHT_TILES * T
FLOOR_TOP = (HEIGHT_TILES - SLAB_TILES) * T

SHELF_COLUMN = 12
SHELF_TILES = 6
SHELF_HEIGHT_TILES = 4  # above the floor

POLE_COLUMN = WIDTH_TILES - 8
POLE_WIDTH = T // 2
POLE_TILES = 6

FERRY_TILES = 3
FERRY_SPEED = 90.0

ARRIVAL_COLUMN = 3
EXIT_COLUMN = WIDTH_TILES - 3

# 1-1's side of the trip, mirrored from `lib.rs`'s `vault_bounds()` and its
# ground constants — the two functions this script replaces.
VAULT_RIGHT = 23 * T + 18 * T
VAULT_FLOOR_TOP = 15 * T + (9 - 2) * T
SURFACE_GROUND_TOP = 15 * T - 2 * T
SURFACE_RETURN_COLUMN = 57


def rect(etype: str, px: tuple[int, int], size: tuple[int, int], **fields) -> dict:
    entry: dict = {"type": etype, "px": list(px), "size": list(size)}
    if fields:
        entry["fields"] = fields
    return entry


def corridor(column: float) -> int:
    """A corridor column, in level px."""
    return int(X0 + column * T)


def terrain() -> list[dict]:
    """Everything that lowers to IntGrid: no identity, tiled art, paintable.

    The roof is unbroken — that is what makes the level read as underground
    rather than as a pit — and the floor comes in two runs with the chasm
    between them."""
    return [
        rect(
            "Solid",
            (X0, 0),
            (WIDTH_TILES * T, SLAB_TILES * T),
            name="cavern_roof",
        ),
        rect(
            "Solid",
            (X0, FLOOR_TOP),
            (CHASM[0] * T, SLAB_TILES * T),
            name="cavern_floor_near",
        ),
        rect(
            "Solid",
            (corridor(CHASM[1]), FLOOR_TOP),
            ((WIDTH_TILES - CHASM[1]) * T, SLAB_TILES * T),
            name="cavern_floor_far",
        ),
        rect("Solid", (0, 0), (X0, LEVEL_H), name="cavern_wall_left"),
        rect(
            "Solid",
            (X0 + WIDTH_TILES * T, 0),
            (SLAB_TILES * T, LEVEL_H),
            name="cavern_wall_right",
        ),
        # The coin shelf: one raised run, reachable with an ordinary jump, and
        # the first thing the room teaches — its ceiling is low enough to matter.
        rect(
            "Solid",
            (corridor(SHELF_COLUMN), FLOOR_TOP - SHELF_HEIGHT_TILES * T),
            (SHELF_TILES * T, T),
            name="coin_shelf",
        ),
    ]


def fixtures() -> list[dict]:
    """The spawn, the ferry, and the two ends of the corridor.

    ⚠ the spawn sits under the arrival shaft on the near floor, so a body that
    somehow reaches this room without a transition still starts somewhere sane.
    `PlayerStart` is authored by its CORNER and the engine spawns at its centre,
    hence the half-size offset."""
    spawn = (corridor(ARRIVAL_COLUMN), FLOOR_TOP - 2 * T)
    ferry = (FERRY_TILES * T, T // 2)
    # 1-2's shafts are a tile wide and three tall, standing ON the floor.
    zone = (2 * T, 3 * T)
    return [
        rect(
            "PlayerStart",
            (spawn[0] - 14, spawn[1] - 23),
            (28, 46),
            name="mary_o_1_2_start",
        ),
        # The ferry starts on the NEAR lip, so the ride is always available
        # rather than something you wait for on the wrong side of a gap you
        # cannot cross. It sweeps exactly the chasm, lip to lip.
        rect(
            "MovingPlatform",
            (corridor(CHASM[0]), FLOOR_TOP - 3 * T - ferry[1] // 2),
            ferry,
            name="Underground Ferry",
            sweep_dx=float((CHASM[1] - CHASM[0]) * T - ferry[0]),
            speed=FERRY_SPEED,
        ),
        rect(
            "LoadingZone",
            (corridor(ARRIVAL_COLUMN) - zone[0] // 2, FLOOR_TOP - zone[1]),
            zone,
            id="mary_o_1_2_arrival",
            name="From the vault",
            activation="Walk",
        ),
        # The alcove at the far end that returns you to the surface.
        rect(
            "LoadingZone",
            (corridor(EXIT_COLUMN) - zone[0] // 2, FLOOR_TOP - zone[1]),
            zone,
            id="mary_o_1_2_exit",
            name="Up to the surface",
            activation="Walk",
            target_room=SURFACE_AREA,
            target_zone="mary_o_1_1_surface_return",
            bidirectional=False,
        ),
    ]


def named_blocks() -> list[dict]:
    """Added AFTER `area create` so the name survives the IntGrid lowering.

    ONE-WAY, not solid, for the reason 1-1's pole is: a flagpole you can walk
    into is a wall, and a wall parks the body half a width away from the pole's
    centre — permanently outside a grab band measured from that centre.

    ⚠ it stands SHORT of the exit alcove on purpose, so the two affordances do
    not overlap: a body walking the last stretch meets the pole first."""
    return [
        rect(
            "OneWayPlatform",
            (corridor(POLE_COLUMN), FLOOR_TOP - POLE_TILES * T),
            (POLE_WIDTH, POLE_TILES * T),
            name="goal_pole",
        ),
    ]


def surface_zones() -> list[dict]:
    """1-1's two ends of the trip — the last coordinates it kept in Rust.

    Walk-in zones, not a third pipe: the vault's own pipes answer a directional
    press (Jon's rule), and a shaft in the floor is a different affordance rather
    than a competing one."""
    # 1-1's are a tile wide and a tile and a half tall — narrower than 1-2's,
    # because the vault floor has less room to spare than the corridor does.
    zone = (T, 3 * T // 2)
    return [
        # The open shaft at the vault's far end. It sits ON the vault floor; the
        # return pipe once shipped floating 48px clear of its own band
        # (`cbc6902d2`), so "does this meet the floor" is asserted in both rooms.
        rect(
            "LoadingZone",
            (VAULT_RIGHT - 2 * zone[0], VAULT_FLOOR_TOP - zone[1]),
            zone,
            id="mary_o_1_1_descent",
            name="Down to 1-2",
            activation="Walk",
            target_room=AREA,
            target_zone="mary_o_1_2_arrival",
            bidirectional=False,
        ),
        # Where 1-2 puts you back on the surface: past pit B, on the long run
        # before the stair pyramid. Going underground is a SHORTCUT — you skip
        # two pits — so the route competes with the surface run rather than
        # merely detouring from it. A LANDING PAD: it names no target.
        rect(
            "LoadingZone",
            (SURFACE_RETURN_COLUMN * T - zone[0] // 2, SURFACE_GROUND_TOP - zone[1]),
            zone,
            id="mary_o_1_1_surface_return",
            name="Back to the surface",
            activation="Walk",
        ),
    ]


def area_spec() -> dict:
    return {
        "id": AREA,
        "level_id": AREA,
        # Below 1-1 (which occupies y 0..768), because that is where it is.
        "world_x": 0,
        "world_y": 1024,
        "px_wid": LEVEL_W,
        "px_hei": LEVEL_H,
        "fill_collision": "empty",
        "bg_color": "#14101c",
        "entities": terrain() + fixtures(),
    }


def run_tool(*args: str) -> None:
    cmd = [sys.executable, "-m", "ambition_ldtk_tools", *args]
    print("::", " ".join(str(a) for a in args))
    env = {"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"}
    result = subprocess.run(cmd, cwd=REPO, env=env)
    if result.returncode != 0:
        sys.exit(f"tool step failed: {' '.join(args)}")


def main() -> None:
    if not TARGET.exists():
        sys.exit(f"REFUSED: {TARGET.relative_to(REPO)} does not exist; 1-1 comes first.")
    project = json.loads(TARGET.read_text())
    if any(level.get("identifier") == AREA for level in project["levels"]):
        sys.exit(
            f"REFUSED: {TARGET.relative_to(REPO)} already has the {AREA!r} level.\n"
            "\n"
            "This script BOOTSTRAPS the area from the constants at the top of it "
            "and would author a second copy. The .ldtk file is the level now.\n"
            "\n"
            "  • to change the layout: edit it in LDtk, not here"
        )
    with tempfile.TemporaryDirectory() as tmp:
        area = Path(tmp) / "mary_o_1_2_area.json"
        area.write_text(json.dumps(area_spec(), indent=2))
        # `--no-repair` on every step but the last. Each tool step otherwise
        # ends in a full-project validate, and the file is only consistent once
        # BOTH ends of both trips exist: 1-2's exit names a zone in 1-1 that this
        # script has not added yet, and 1-1's descent names one in 1-2.
        run_tool("area", "create", str(area), "--ldtk", str(TARGET), "--no-repair")
        named = Path(tmp) / "mary_o_1_2_named.json"
        named.write_text(
            json.dumps({"level_id": AREA, "entities": named_blocks()}, indent=2)
        )
        run_tool("entity", "add", str(named), "--ldtk", str(TARGET), "--in-place", "--no-repair")
        surface = Path(tmp) / "mary_o_1_1_zones.json"
        surface.write_text(
            json.dumps({"level_id": SURFACE_AREA, "entities": surface_zones()}, indent=2)
        )
        run_tool("entity", "add", str(surface), "--ldtk", str(TARGET), "--in-place", "--no-repair")
    run_tool("repair", str(TARGET), "--in-place")
    run_tool("validate", str(TARGET))
    print(f"authored {AREA} into {TARGET.relative_to(REPO)}")


if __name__ == "__main__":
    main()
