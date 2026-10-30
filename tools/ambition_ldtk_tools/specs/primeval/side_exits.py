#!/usr/bin/env python3
"""Join the primeval trail's rooms by side exits, on a flat map.

    python specs/primeval/side_exits.py --apply

Run it from `tools/ambition_ldtk_tools`.

Jon (2026-10-30): "we want to use side loading zones primarily. doors are for
entering buildings. use vertical and horizontal space to connect the world.
this will let us build a mostly faithful flat map". The trail's rooms joined
by doors at their ends, 10000 px apart in the world. This moves each of those
doors to the room's edge and makes it an edge exit (same entity, id and
target), opens the wall there, and places the rooms so they touch with their floors level
(`specs/check_flat_map.py`). The tyrant's hall keeps its door: it is a
building.

The primeval generator (`make_specs.py`) is older than its levels (gate
terminals, water and enemies were tuned in the LDtk after it), so this edits
the levels in place, as `specs/scenery_overlay.py` does. It does what is not
done yet, so it may be run again.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))

from district_kit import WORLDS, run  # noqa: E402

WORLD = WORLDS / "dinosaur.ldtk"
WALL = 16
#: The trail, west to east: (room, its west exit, its east exit). The
#: trailhead's west door goes to the central hub (another world) and stays.
TRAIL = [
    ("primeval_trailhead", None, "primeval_trailhead_east"),
    ("primeval_run", "primeval_run_west", "primeval_run_east"),
    ("primeval_rookery", "primeval_rookery_west", "primeval_rookery_east"),
    ("primeval_ridge", "primeval_ridge_west", "primeval_ridge_east"),
    ("primeval_canopy", "primeval_canopy_west", None),
]


def levels() -> dict:
    return {lv["identifier"]: lv for lv in json.loads(WORLD.read_text())["levels"]}


def zone(level: dict, zone_id: str) -> dict:
    for layer in level["layerInstances"]:
        for e in layer["entityInstances"]:
            f = {x["__identifier"]: x["__value"] for x in e["fieldInstances"]}
            if e["__identifier"] == "LoadingZone" and f.get("id") == zone_id:
                return dict(e=e, f=f)
    raise SystemExit(f"{level['identifier']}: no zone {zone_id}")


def floor_of(level: dict, zone_entry: dict) -> int:
    return zone_entry["e"]["px"][1] + zone_entry["e"]["height"]


def to_edge(level: dict, zone_id: str, east: bool) -> int:
    z = zone(level, zone_id)
    if z["f"].get("activation") == "EdgeExit":
        return 0
    rid, floor = level["identifier"], floor_of(level, z)
    x = level["pxWid"] - WALL if east else 0
    spec = HERE / ".side_exit.yaml"
    target = f"{{identifier: LoadingZone, match: {{id: {zone_id}}}}}"
    spec.write_text(f"level_id: {rid}\nmoves:\n  - target: {target}\n    px: [{x}, {floor - 96}]\n    size: [{WALL}, 96]\n")
    # Each step writes the file and then checks the whole world; between the
    # two steps a door stands at the room's edge, which the check refuses
    # (its arrival is outside the room). Only the end state is checked: the
    # last `validate` of `main`.
    run("entity", "move", str(spec), "--ldtk", str(WORLD), "--in-place")
    spec.write_text(f"level_id: {rid}\nedits:\n  - target: {target}\n    fields: {{activation: EdgeExit}}\n")
    run("entity", "set-field", str(spec), "--ldtk", str(WORLD), "--in-place")
    spec.unlink()
    if zone(levels()[rid], zone_id)["f"].get("activation") != "EdgeExit":
        raise SystemExit(f"{rid}: {zone_id} did not become an edge exit")
    code, _ = run("intgrid", "paint", "--ldtk", str(WORLD), "--level", rid, "--layer", "Collision", "--value", "0",
                  "--px", f"{x},{floor - 112}", "--size", f"{WALL},112")
    return code


def place_rooms() -> None:
    """World positions: each room against the last, floors level."""
    lv = levels()
    first = lv[TRAIL[0][0]]
    x = first["worldX"]
    floor_y = first["worldY"] + floor_of(first, zone(first, TRAIL[0][2]))
    where = {}
    for rid, west, east in TRAIL:
        level = lv[rid]
        floor = floor_of(level, zone(level, west or east))
        where[rid] = (x, floor_y - floor)
        x += level["pxWid"]
    text = WORLD.read_text()
    for rid, (wx, wy) in where.items():
        # The level's own worldX/worldY, right after its identifier.
        pattern = re.compile(r'("identifier": "' + rid + r'",.*?"worldX": )(-?\d+)(,\s*"worldY": )(-?\d+)', re.S)
        text, n = pattern.subn(lambda m: f"{m.group(1)}{wx}{m.group(3)}{wy}", text, count=1)
        if n != 1:
            raise SystemExit(f"{rid}: no worldX/worldY after its identifier")
    WORLD.write_text(text)


def main(argv) -> int:
    if "--apply" not in argv:
        print(__doc__)
        return 0
    for rid, west, east in TRAIL:
        for zone_id, is_east in ((west, False), (east, True)):
            if zone_id and (code := to_edge(levels()[rid], zone_id, is_east)):
                return code
    place_rooms()
    # Entities cache their world coordinates; the repair recomputes them from
    # the moved levels.
    run("repair", str(WORLD), "--in-place")
    code, _ = run("validate", str(WORLD))
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
