#!/usr/bin/env python3
"""Rebuild the `water_world` level: drop the old level, re-author
the geometry from `tools/specs/water_world_area.yaml`, then paint
the IntGrid `Water` layer with two pools (Clear left, Murky right).

`author_ldtk_area.py` only knows about the Collision IntGrid layer,
so the Water cells are painted here as a post-processing step. The
authoring rectangles below intentionally avoid the `pool_divider`
column so neither pool overlaps the dividing wall.
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LDTK_PATH = REPO_ROOT / "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"
SPEC_PATH = REPO_ROOT / "tools/ambition_ldtk_tools/specs/water_world_area.yaml"
AUTHOR_TOOL = REPO_ROOT / "tools/author_ldtk_area.py"
COMPACT_TOOL = REPO_ROOT / "tools/compact_ldtk_json.py"

GRID = 16

# px-space rects to paint in the Water IntGrid layer.
# (px, py, w, h, value)  value = 1 (Clear) or 2 (Murky)
WATER_RECTS = [
    # Clear pool: from the left pool wall to the divider column,
    # surface at y=448, bottom at y=720 (top of pool_floor).
    (16, 448, 944, 272, 1),
    # Murky pool: from divider's right edge to the right wall.
    (1040, 448, 544, 272, 2),
]


def drop_existing_water_world(data: dict) -> bool:
    levels = data["levels"]
    before = len(levels)
    data["levels"] = [level for level in levels if level["identifier"] != "water_world"]
    return len(data["levels"]) < before


def ensure_water_layer(data: dict, level: dict) -> dict:
    """Return the level's Water IntGrid layer instance, allocating
    and inserting one if `author_ldtk_area.py` did not (it only seeds
    Collision + Ambition layers today)."""
    for inst in level["layerInstances"]:
        if inst["__identifier"] == "Water":
            return inst
    # Pull cWid/cHei + grid from the Collision layer so sizes line up.
    collision = next(inst for inst in level["layerInstances"] if inst["__identifier"] == "Collision")
    cw = collision["__cWid"]
    ch = collision["__cHei"]
    grid_size = collision["__gridSize"]
    # Find the Water layer def's uid.
    layer_def = next(layer for layer in data["defs"]["layers"] if layer["identifier"] == "Water")
    next_uid = int(data.get("nextUid", 1))
    data["nextUid"] = next_uid + 1
    iid = f"Water-{next_uid:04d}"
    water_inst = {
        "__identifier": "Water",
        "__type": "IntGrid",
        "__cWid": cw,
        "__cHei": ch,
        "__gridSize": grid_size,
        "__opacity": 0.6,
        "__pxTotalOffsetX": 0,
        "__pxTotalOffsetY": 0,
        "__tilesetDefUid": None,
        "__tilesetRelPath": None,
        "iid": iid,
        "levelId": level["uid"],
        "layerDefUid": layer_def["uid"],
        "pxOffsetX": 0,
        "pxOffsetY": 0,
        "visible": True,
        "optionalRules": [],
        "intGridCsv": [0] * (cw * ch),
        "autoLayerTiles": [],
        "seed": 0,
        "overrideTilesetUid": None,
        "gridTiles": [],
        "entityInstances": [],
    }
    # Insert right after Collision so editor draw order matches every
    # other level in the project.
    idx = next(i for i, inst in enumerate(level["layerInstances"]) if inst is collision)
    level["layerInstances"].insert(idx + 1, water_inst)
    return water_inst


def paint_rect(csv: list[int], cw: int, ch: int, px: int, py: int, w: int, h: int, value: int) -> int:
    cx0 = px // GRID
    cy0 = py // GRID
    cx1 = (px + w + GRID - 1) // GRID
    cy1 = (py + h + GRID - 1) // GRID
    painted = 0
    for cy in range(cy0, cy1):
        for cx in range(cx0, cx1):
            if 0 <= cx < cw and 0 <= cy < ch:
                csv[cy * cw + cx] = value
                painted += 1
    return painted


def main() -> None:
    # 1. Drop existing water_world.
    data = json.loads(LDTK_PATH.read_text())
    if drop_existing_water_world(data):
        LDTK_PATH.write_text(json.dumps(data, indent=2))
        print("removed existing water_world level")
    else:
        print("no existing water_world level to drop")

    # 2. Re-author from spec via author_ldtk_area.py.
    result = subprocess.run(
        [sys.executable, str(AUTHOR_TOOL), str(SPEC_PATH)],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    print(result.stdout.strip().splitlines()[-3:][0] if result.stdout.strip() else "(no output)")

    # 3. Paint Water IntGrid cells in the new water_world level.
    data = json.loads(LDTK_PATH.read_text())
    level = next(level for level in data["levels"] if level["identifier"] == "water_world")
    water = ensure_water_layer(data, level)
    cw, ch = water["__cWid"], water["__cHei"]
    csv = list(water["intGridCsv"])
    if len(csv) != cw * ch:
        # author_ldtk_area's seeded layer is the right size; if the
        # length drifted, re-zero rather than corrupting the grid.
        csv = [0] * (cw * ch)
    total = 0
    for px, py, w, h, value in WATER_RECTS:
        total += paint_rect(csv, cw, ch, px, py, w, h, value)
    water["intGridCsv"] = csv
    LDTK_PATH.write_text(json.dumps(data, indent=2))
    print(f"painted {total} Water cells across {len(WATER_RECTS)} rect(s)")

    # 4. Compact arrays back to LDtk's mixed style + final validation.
    subprocess.run([sys.executable, str(COMPACT_TOOL), str(LDTK_PATH)], cwd=REPO_ROOT, check=True)


if __name__ == "__main__":
    main()
