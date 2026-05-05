#!/usr/bin/env python3
"""Repair basement-area LoadingZone doors that overlap floor solids and
have mismatched paired sizes. Adjusts each door's `px`, `__grid`,
`width`, and `height` (and the cached `__worldX`/`__worldY` markers)
so that:

- Every basement-area door has size 48x96 (player-fitting "door" shape
  matching the door_zone sprite) — uniform across hub-side doors and
  the matching branch-side return_doors so the runtime no longer emits
  "doors mismatch" warnings.
- Every door's bottom edge lines up with the top of the level's
  basement floor (first row of IntGrid value-1 solids encountered
  beneath the door's current position), so the door zone no longer
  cuts into solid blocks and the `door_arrival` spawn is above ground.
- The lab_door (basement → mob_lab) is moved left to fit before the
  basement's right wall.

Run: `python3 tools/fix_basement_doors.py` (writes in place).
"""
from __future__ import annotations

import json
from pathlib import Path

LDTK_PATH = Path(__file__).resolve().parent.parent / "crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk"

GRID = 16
DOOR_W = 48
DOOR_H = 96

# (level_identifier, zone_id, desired_x_or_None_to_keep)
# desired_x is the new px[0]; if None we keep the door's current x and
# only retarget y to sit on the floor.
ADJUSTMENTS: list[tuple[str, str, int | None]] = [
    ("central_hub_basement", "hazard_door", None),
    ("central_hub_basement", "enemy_door", None),
    ("central_hub_basement", "boss_door", None),
    # The 130x132 doors collapse to 48x96; horizontally re-center each
    # one within its previous footprint so the basement layout still
    # reads as three evenly-spaced doorways on the right side.
    ("central_hub_basement", "breakable_door", 1121),  # was 1080..1210, center 1145 -> 1121
    ("central_hub_basement", "treasure_door", 1381),
    ("central_hub_basement", "npc_door", 1641),
    # lab_door must clear the right wall (cells 116-118 are solid).
    # Place it right of npc_door but inside the playable region.
    ("central_hub_basement", "lab_door", 1808),
    # Branch return_doors — keep their current x (they are at px=95 or
    # 127 in their own levels), just retarget y to floor and resize.
    ("basement_hazards", "return_door", None),
    ("basement_enemies", "return_door", None),
    ("basement_boss", "return_door", None),
    ("basement_breakables", "return_door", None),
    ("basement_treasure", "return_door", None),
    ("basement_npcs", "return_door", None),
]


def find_floor_top_row(intgrid: list[int], cwid: int, chei: int, cx0: int, cx1: int, cy_start: int) -> int:
    """Walk down from cy_start row by row; return the row index of the
    first row that is entirely solid (intgrid value 1) across the
    door's column span. That row is the top of the floor; the door
    bottom should be set to row * GRID."""
    for cy in range(cy_start, chei):
        if all(intgrid[cy * cwid + cx] == 1 for cx in range(cx0, cx1)):
            return cy
    raise RuntimeError(f"no solid floor row found below cy={cy_start} for door span x[{cx0},{cx1})")


def main() -> None:
    text = LDTK_PATH.read_text()
    data = json.loads(text)

    fixes: list[str] = []
    for level in data["levels"]:
        name = level["identifier"]
        cwid = chei = 0
        intgrid: list[int] = []
        for layer in level.get("layerInstances", []):
            if layer["__identifier"] == "Collision":
                cwid = layer["__cWid"]
                chei = layer["__cHei"]
                intgrid = layer["intGridCsv"]
                break

        for layer in level.get("layerInstances", []):
            if layer["__identifier"] != "Ambition":
                continue
            for ent in layer.get("entityInstances", []):
                if ent["__identifier"] != "LoadingZone":
                    continue
                fields = {f["__identifier"]: f["__value"] for f in ent["fieldInstances"]}
                zid = fields.get("id")
                target = next(((lvl, zid_, want_x) for (lvl, zid_, want_x) in ADJUSTMENTS if lvl == name and zid_ == zid), None)
                if not target:
                    continue
                _, _, want_x = target

                old_x, old_y = ent["px"]
                old_w, old_h = ent["width"], ent["height"]

                new_w, new_h = DOOR_W, DOOR_H
                new_x = want_x if want_x is not None else old_x

                # Compute door cell column span.
                cx0 = new_x // GRID
                cx1 = max(cx0 + 1, (new_x + new_w + GRID - 1) // GRID)
                # Start scanning for floor at the door's current top row.
                cy_search_start = old_y // GRID + 1
                floor_row = find_floor_top_row(intgrid, cwid, chei, cx0, cx1, cy_search_start)
                # New door bottom = floor_row * GRID; door top = bottom - h.
                new_y = floor_row * GRID - new_h
                if new_y < 0:
                    raise RuntimeError(f"{name}/{zid}: computed y={new_y} would push door above the level")

                ent["px"] = [new_x, new_y]
                ent["width"] = new_w
                ent["height"] = new_h
                ent["__grid"] = [new_x // GRID, new_y // GRID]
                # __worldX/__worldY include level worldX/worldY offset.
                world_x = level.get("worldX", 0) + new_x
                world_y = level.get("worldY", 0) + new_y
                ent["__worldX"] = world_x
                ent["__worldY"] = world_y

                fixes.append(
                    f"  {name}/{zid}: {old_w}x{old_h}@({old_x},{old_y}) -> "
                    f"{new_w}x{new_h}@({new_x},{new_y})"
                )

    # Pretty-write with the same minor indent style LDtk uses (LDtk
    # uses tabs in some parts; just write JSON normally — repair tool
    # will canonicalize).
    LDTK_PATH.write_text(json.dumps(data, indent=2))
    print("Door fixes applied:")
    for line in fixes:
        print(line)


if __name__ == "__main__":
    main()
