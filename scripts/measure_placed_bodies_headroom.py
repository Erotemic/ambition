#!/usr/bin/env python3
"""Does each placed character with an authored height fit under what is over it?

A catalog row's `standing_height` sizes its collision body. A level places a
character by its feet, so a taller body reaches higher into the room. For each
placement of a character whose row states a height, this reads the level's
IntGrid layers and reports a placement whose body column (12 wide, from its
feet up to its height) covers a nonzero cell, and whether it did at 48 too
(the height of a `Standard` row that states none).

    python scripts/measure_placed_bodies_headroom.py

Reads the `.ldtk` files of Ambition's worlds and Ambition's character catalog.
It does not see a solid that is an entity (a block, a platform), and it does
not ask whether a body can walk where its brain will take it.
"""
from __future__ import annotations

import glob
import json
import os
import re
from collections import Counter

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CATALOG = os.path.join(REPO, "game/ambition_content/assets/data/character_catalog.ron")
WORLDS = ["game/ambition_content/assets/worlds/*.ldtk", "game/ambition_map_assets/ambition_content/worlds/*.ldtk"]
UNSTATED = 48.0
CHARACTER_FIELDS = ("character_id", "character", "archetype", "enemy_id")


def stated_heights() -> dict[str, float]:
    rows = re.split(r'\n        "([a-z0-9_]+)": \(\n', open(CATALOG).read())
    out = {}
    for k in range(1, len(rows), 2):
        height = re.search(r"\n            standing_height: Some\(([\d.]+)\)", rows[k + 1])
        if height:
            out[rows[k]] = float(height.group(1))
    return out


def covers(grid: dict, cx: float, feet: float, height: float):
    size, wide, high, cells = grid["__gridSize"], grid["__cWid"], grid["__cHei"], grid["intGridCsv"]
    for y in range(int((feet - height) // size), int((feet - 1) // size) + 1):
        for x in range(int((cx - 6) // size), int((cx + 6) // size) + 1):
            if 0 <= x < wide and 0 <= y < high and cells[y * wide + x] != 0:
                return (grid["__identifier"], cells[y * wide + x], x, y)
    return None


def main() -> int:
    heights = stated_heights()
    files, seen, placed, found = [], set(), 0, []
    for pattern in WORLDS:
        files += glob.glob(os.path.join(REPO, pattern))
    for path in sorted(files):
        real = os.path.realpath(path)
        if real in seen:
            continue
        seen.add(real)
        for level in json.load(open(path)).get("levels", []):
            layers = level.get("layerInstances") or []
            grids = [layer for layer in layers if layer["__type"] == "IntGrid" and layer.get("intGridCsv")]
            for layer in layers:
                if layer["__type"] != "Entities":
                    continue
                for entity in layer["entityInstances"]:
                    who = next((f["__value"] for f in entity["fieldInstances"]
                                if f["__identifier"] in CHARACTER_FIELDS and isinstance(f["__value"], str)), None)
                    if who not in heights:
                        continue
                    placed += 1
                    (px, py), (pivot_x, pivot_y) = entity["px"], entity["__pivot"]
                    feet = py - pivot_y * entity["height"] + entity["height"]
                    cx = px - pivot_x * entity["width"] + entity["width"] / 2
                    for grid in grids:
                        cell = covers(grid, cx, feet, heights[who])
                        if cell:
                            was = "also at 48" if covers(grid, cx, feet, UNSTATED) else "NEW at its height"
                            found.append((os.path.basename(path), level["identifier"], who, heights[who], cell, was))
    print(f"{len(heights)} rows state a height; {placed} placements of them; {len(found)} cover a nonzero IntGrid cell")
    print(dict(Counter(row[-1] for row in found)))
    for row in found:
        print("  ", row)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
