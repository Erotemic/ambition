#!/usr/bin/env python3
"""Tiny local-frame room spec compiler for sandbox-friendly LDtk authoring.

This is deliberately modest: it compiles a compact JSON/RON-ish data tree into
an existing level by painting IntGrid rectangles and creating common entities.
It is not a replacement for `area create`; it is a higher-level, agent-friendly
patch layer for generated rooms.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.edit.camera import autocover_camera
from ambition_ldtk_tools.edit.entity_layer_rules import DEFAULT_LDTK, ensure_entities_layer_def, ensure_entities_layer_instance, write_project

def _ron_load(text: str):
    try:
        from ambition_ldtk_tools.ron_parse import load as ron_load  # type: ignore
    except BaseException as ex:  # pragma: no cover
        raise SystemExit(
            "non-JSON room specs require python-ron / pyron; "
            "use .json specs in minimal environments"
        ) from ex
    return ron_load(text)

INTGRID_VALUES = {
    "solid": 1,
    "one_way": 2,
    "blink_soft": 3,
    "blink_hard": 4,
    "hazard": 5,
}

ENTITY_LAYERS = {
    "CameraZone": "AmbitionCameras",
    "GravityZone": "Ambition",
    "LoadingZone": "Ambition",
    "PlayerStart": "Ambition",
}


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() == ".json":
        return json.loads(text)
    return _ron_load(text)


def find_level(project: dict, level_id: str) -> dict:
    for level in project.get("levels", []) or []:
        if level.get("identifier") == level_id:
            return level
    raise SystemExit(f"level {level_id!r} not found")


def find_entity_def(project: dict, ident: str) -> dict:
    for ent in project.get("defs", {}).get("entities", []) or []:
        if ent.get("identifier") == ident:
            return ent
    raise SystemExit(f"entity def {ident!r} not found")


def alloc_uid(project: dict) -> int:
    next_uid = int(project.get("nextUid", 1))
    project["nextUid"] = next_uid + 1
    return next_uid


def layer_by_id(level: dict, ident: str) -> dict:
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__identifier") == ident:
            return layer
    raise SystemExit(f"level {level.get('identifier')} has no layer {ident!r}")


def paint_intgrid_rect(layer: dict, rect: list[int], value: int) -> int:
    x, y, w, h = map(int, rect)
    grid = int(layer.get("__gridSize") or 16)
    c_wid = int(layer.get("__cWid") or 0)
    c_hei = int(layer.get("__cHei") or 0)
    csv = layer.setdefault("intGridCsv", [0] * (c_wid * c_hei))
    changed = 0
    x0, y0 = max(0, x // grid), max(0, y // grid)
    x1, y1 = min(c_wid, (x + w + grid - 1) // grid), min(c_hei, (y + h + grid - 1) // grid)
    for cy in range(y0, y1):
        for cx in range(x0, x1):
            idx = cy * c_wid + cx
            if idx < len(csv) and csv[idx] != value:
                csv[idx] = value
                changed += 1
    return changed


def entity_field_instances(entity_def: dict, fields: dict[str, Any]) -> list[dict]:
    rows = []
    for fdef in entity_def.get("fieldDefs", []) or []:
        ident = fdef.get("identifier")
        if ident not in fields:
            continue
        value = fields[ident]
        rows.append({
            "__identifier": ident,
            "__type": fdef.get("__type"),
            "__value": value,
            "__tile": None,
            "defUid": fdef.get("uid"),
            "realEditorValues": [],
        })
    return rows


def add_entity(project: dict, level: dict, ident: str, rect: list[int], fields: dict[str, Any]) -> dict:
    ent_def = find_entity_def(project, ident)
    layer_id = ENTITY_LAYERS.get(ident, "Ambition")
    layer_def = ensure_entities_layer_def(project, layer_id, clone_from="Ambition")
    layer = ensure_entities_layer_instance(project, level, layer_id, dest_def=layer_def, clone_from="Ambition")
    x, y, w, h = map(int, rect)
    uid = alloc_uid(project)
    ent = {
        "__identifier": ident,
        "__grid": [x // 16, y // 16],
        "__pivot": [ent_def.get("pivotX", 0), ent_def.get("pivotY", 0)],
        "__tags": [],
        "__tile": None,
        "__smartColor": ent_def.get("color", "#ffffff"),
        "iid": f"{ident}-{uid}",
        "width": w,
        "height": h,
        "defUid": ent_def.get("uid"),
        "px": [x, y],
        "fieldInstances": entity_field_instances(ent_def, fields),
        "__worldX": int(level.get("worldX") or 0) + x,
        "__worldY": int(level.get("worldY") or 0) + y,
    }
    layer.setdefault("entityInstances", []).append(ent)
    return ent


def compile_spec(project: dict, spec: dict, *, dry_run: bool = False) -> list[str]:
    level_id = spec.get("level") or spec.get("identifier")
    if not level_id:
        raise SystemExit("room spec requires level")
    level = find_level(project, str(level_id))
    report: list[str] = [f"compile room spec for {level_id}"]
    for item in spec.get("intgrid", []) or []:
        layer_id = item.get("layer", "Collision")
        value = item.get("value", item.get("type", "solid"))
        if isinstance(value, str):
            value = int(value) if value.isdigit() else INTGRID_VALUES.get(value, 1)
        rect = item.get("rect") or item.get("px")
        if not rect:
            raise SystemExit(f"intgrid item missing rect: {item}")
        changed = paint_intgrid_rect(layer_by_id(level, layer_id), rect, int(value)) if not dry_run else 0
        report.append(f"  paint {layer_id} {rect} value={value} changed={changed}")
    for item in spec.get("entities", []) or []:
        ident = item.get("type") or item.get("identifier")
        rect = item.get("rect") or [*item.get("px", [0, 0]), *item.get("size", [32, 32])]
        fields = item.get("fields") or {}
        if not dry_run:
            ent = add_entity(project, level, str(ident), rect, fields)
            report.append(f"  add {ident} {ent['iid']} at {rect}")
        else:
            report.append(f"  add {ident} at {rect}")
    camera = spec.get("camera")
    if camera:
        if camera is True:
            camera = {}
        margin = int(camera.get("margin", 0))
        if not dry_run:
            report.append("  " + autocover_camera(project, str(level_id), margin, bool(camera.get("create", True))))
        else:
            report.append(f"  camera auto-cover margin={margin}")
    return report


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Compile a compact room spec into LDtk edits.")
    ap.add_argument("action", choices=["compile"])
    ap.add_argument("spec", type=Path)
    ap.add_argument("--ldtk", type=Path, default=DEFAULT_LDTK)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args(argv)

    project = load_project(args.ldtk)
    report = compile_spec(project, load_spec(args.spec), dry_run=args.dry_run)
    print("\n".join(report))
    if not args.dry_run:
        if args.in_place:
            write_project(args.ldtk, project)
        elif args.output:
            write_project(args.output, project)
        else:
            raise SystemExit("room spec compile requires --dry-run, --in-place, or --output")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
