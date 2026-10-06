#!/usr/bin/env python3
"""Clone a whole level (room) in an LDtk file under a new identifier.

The copy is the same room — its IntGrid layers, its entities and their
fields, its level fields — with fresh identities: a new level uid and iid, a
new iid for every layer and entity instance, every EntityRef that pointed
INSIDE the source level re-pointed at its copy (a ref out of the level is
kept), and its `activeArea` set to the new identifier, so the copy is its own
room. It is placed at `--world-x/--world-y`.

Use it when a room needs a second, independent instance: the Hall of Bosses'
own copy of a boss arena that the main game also uses (Jon, 2026-10-06). Edit
the copy afterwards with `entity set-field` / `entity delete` / `level
set-field`: its entities keep their authored `id`/`name` fields, so a door id
or a placement's `encounter_id` that must be unique has to be changed.

Usage:

    ambition_ldtk_tools level clone <level_id> <new_id> --world-x X --world-y Y
        [--ldtk PATH] (--in-place | --output PATH) [--backup] [--no-repair]
"""

from __future__ import annotations

import argparse
import copy
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/level_clone.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import allocate_iid
from ambition_ldtk_tools.edit.postprocess import run_repair_and_validate
from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk
from ambition_ldtk_tools.ldtk.transaction import LdtkTransaction


def _levels(project: dict) -> list:
    return project.get("levels", [])


def _find_level(project: dict, level_id: str) -> dict:
    for level in _levels(project):
        if level.get("identifier") == level_id:
            return level
    present = ", ".join(level.get("identifier", "?") for level in _levels(project))
    raise SystemExit(f"level '{level_id}' not found. Levels: {present}")


def _remap_refs(value, iids: dict, level_iid: tuple[str, str], layer_iids: dict):
    """Re-point every EntityRef (a dict with `entityIid`) whose entity is in
    the cloned level at the entity's copy, in place."""
    if isinstance(value, list):
        for item in value:
            _remap_refs(item, iids, level_iid, layer_iids)
    elif isinstance(value, dict):
        if "entityIid" in value and value.get("entityIid") in iids:
            value["entityIid"] = iids[value["entityIid"]]
            if value.get("levelIid") == level_iid[0]:
                value["levelIid"] = level_iid[1]
            if value.get("layerIid") in layer_iids:
                value["layerIid"] = layer_iids[value["layerIid"]]
        else:
            for item in value.values():
                _remap_refs(item, iids, level_iid, layer_iids)


def clone_level(project: dict, source_id: str, new_id: str, world_x: int, world_y: int) -> dict:
    """Append a copy of `source_id` named `new_id` at (world_x, world_y) and
    return it."""
    if any(level.get("identifier") == new_id for level in _levels(project)):
        raise SystemExit(f"level '{new_id}' already exists")
    source = _find_level(project, source_id)
    level = copy.deepcopy(source)
    dx, dy = world_x - int(source["worldX"]), world_y - int(source["worldY"])

    uid = int(project.get("nextUid", 1))
    project["nextUid"] = uid + 1
    level["identifier"] = new_id
    level["uid"] = uid
    level["iid"] = f"{new_id}-{uid:04d}"
    level["worldX"], level["worldY"] = world_x, world_y
    level["__neighbours"] = []

    iids: dict = {}
    layer_iids: dict = {}
    for layer in level.get("layerInstances") or []:
        new_layer_iid, _ = allocate_iid(project, layer["__identifier"])
        layer_iids[layer["iid"]] = new_layer_iid
        layer["iid"] = new_layer_iid
        layer["levelId"] = uid
        for entity in layer.get("entityInstances", []):
            new_iid, _ = allocate_iid(project, entity["__identifier"])
            iids[entity["iid"]] = new_iid
            entity["iid"] = new_iid
            if "__worldX" in entity:
                entity["__worldX"] = int(entity["__worldX"]) + dx
            if "__worldY" in entity:
                entity["__worldY"] = int(entity["__worldY"]) + dy
    _remap_refs(level.get("layerInstances"), iids, (source["iid"], level["iid"]), layer_iids)
    _remap_refs(level.get("fieldInstances"), iids, (source["iid"], level["iid"]), layer_iids)

    for field in level.get("fieldInstances", []):
        if field.get("__identifier") == "activeArea":
            field["__value"] = new_id
            field["realEditorValues"] = [{"id": "V_String", "params": [new_id]}]

    project["levels"].append(level)
    return level


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("level_id", help="Identifier of the level to copy")
    parser.add_argument("new_id", help="Identifier of the copy")
    parser.add_argument("--world-x", type=int, required=True)
    parser.add_argument("--world-y", type=int, required=True)
    parser.add_argument("--ldtk", type=Path, default=default_sandbox_ldtk())
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument("--no-repair", action="store_true")
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT / "tools" / "ambition_ldtk_tools" / "schemas" / "ldtk" / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if not args.in_place and args.output is None:
        print("error: choose --in-place or --output <path>", file=sys.stderr)
        return 2

    tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output, backup=args.backup)
    level = clone_level(tx.project, args.level_id, args.new_id, args.world_x, args.world_y)
    entities = sum(len(layer.get("entityInstances", [])) for layer in level.get("layerInstances") or [])
    summary = (
        f"  - cloned level '{args.level_id}' as '{args.new_id}' "
        f"({level['pxWid']}x{level['pxHei']} at {args.world_x},{args.world_y}, {entities} entities)"
    )
    print(summary)
    tx.note_changed([summary])
    target_path = tx.finish(noop_message="level clone: nothing changed", write_message="wrote {path}")
    if target_path is None or args.no_repair:
        return 0
    return run_repair_and_validate(target_path, args.schema)


if __name__ == "__main__":
    raise SystemExit(main())
