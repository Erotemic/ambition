#!/usr/bin/env python3
"""Does a dimension's world file make a faithful flat map?

    python specs/check_flat_map.py science.ldtk [math.ldtk ...]

Run it from `tools/ambition_ldtk_tools`. It reports, and changes nothing.

Jon (2026-10-30): rooms out in the world join by side loading zones (edge
exits, across a side or through a floor or ceiling); a door is for going
into a building; and the rooms' world positions make "a mostly faithful flat
map". For each edge exit and the zone it leads to, the two rooms must meet
where they join: a side exit's zones at the same world height (their bottoms,
the walk line) on touching edges, a floor or ceiling exit's zones over the
same world x on touching edges. Rooms must not overlap.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from district_kit import WORLDS  # noqa: E402

TOLERANCE = 2


def zones(world: dict) -> dict:
    out = {}
    for level in world["levels"]:
        for layer in level.get("layerInstances") or []:
            for e in layer["entityInstances"]:
                if e["__identifier"] != "LoadingZone":
                    continue
                f = {x["__identifier"]: x["__value"] for x in e["fieldInstances"]}
                if f.get("activation") != "EdgeExit" or not f.get("id"):
                    continue
                x, y = e["px"]
                out[f["id"]] = dict(level=level, x=level["worldX"] + x, y=level["worldY"] + y, w=e["width"], h=e["height"],
                                    rx=x, ry=y, target_room=f.get("target_room"), target_zone=f.get("target_zone"))
    return out


def side(z) -> str:
    lv = z["level"]
    if z["rx"] <= TOLERANCE:
        return "left"
    if z["rx"] + z["w"] >= lv["pxWid"] - TOLERANCE:
        return "right"
    if z["ry"] <= TOLERANCE:
        return "top"
    return "bottom"


def check(path: Path) -> list[str]:
    world = json.loads(path.read_text())
    found = []
    levels = world["levels"]
    for i, a in enumerate(levels):
        for b in levels[i + 1:]:
            if (a["worldX"] < b["worldX"] + b["pxWid"] and b["worldX"] < a["worldX"] + a["pxWid"]
                    and a["worldY"] < b["worldY"] + b["pxHei"] and b["worldY"] < a["worldY"] + a["pxHei"]):
                found.append(f"overlap: {a['identifier']} and {b['identifier']}")
    all_zones = zones(world)
    seen = set()
    for zid, z in all_zones.items():
        other = all_zones.get(z["target_zone"] or "")
        if other is None or (z["target_zone"], zid) in seen:
            continue
        seen.add((zid, z["target_zone"]))
        a, b = z["level"]["identifier"], other["level"]["identifier"]
        sa, sb = side(z), side(other)
        # A room whose side leads into its own other side is a deliberate
        # wrap (Hylbert's corridor), not a gap in the map.
        if a == b and {sa, sb} in ({"left", "right"}, {"top", "bottom"}):
            continue
        if {sa, sb} == {"left", "right"}:
            ok_edge = abs((z["x"] + (z["w"] if sa == "right" else 0)) - (other["x"] + (other["w"] if sb == "right" else 0))) <= 32
            dy = (z["y"] + z["h"]) - (other["y"] + other["h"])
            if not ok_edge or abs(dy) > TOLERANCE:
                found.append(f"side exit {a}:{zid} -> {b}:{z['target_zone']}: edges {'touch' if ok_edge else 'do not touch'}, walk lines differ by {dy}")
        elif {sa, sb} == {"top", "bottom"}:
            ok_edge = abs((z["y"] + (z["h"] if sa == "bottom" else 0)) - (other["y"] + (other["h"] if sb == "bottom" else 0))) <= 32
            overlap = min(z["x"] + z["w"], other["x"] + other["w"]) - max(z["x"], other["x"])
            if not ok_edge or overlap < min(z["w"], other["w"]) * 0.5:
                found.append(f"vertical exit {a}:{zid} -> {b}:{z['target_zone']}: edges {'touch' if ok_edge else 'do not touch'}, x overlap {overlap}")
        else:
            found.append(f"edge exit {a}:{zid} ({sa}) -> {b}:{z['target_zone']} ({sb}): not opposite sides")
    return found


def main(argv) -> int:
    for name in argv or ["science.ldtk"]:
        path = WORLDS / name
        found = check(path)
        print(f"{name}: {len(found)} finding(s)")
        for line in found:
            print("  " + line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
