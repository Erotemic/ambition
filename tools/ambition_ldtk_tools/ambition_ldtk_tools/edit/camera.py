#!/usr/bin/env python3
"""Camera-zone audit and auto-cover helpers for LDtk files."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ambition_ldtk_tools.edit.entity_layer_rules import DEFAULT_LDTK
from ambition_ldtk_tools.ldtk import (
    LdtkTransaction,
    alloc_uid,
    default_field_value,
    ensure_entities_layer_def,
    ensure_entities_layer_instance,
    find_entity_def as _find_entity_def_or_none,
    find_layer_instance,
    find_level as _find_level_or_none,
    load_project,
)

CAMERA_LAYER = "AmbitionCameras"
CAMERA_ENTITY = "CameraZone"


@dataclass(frozen=True)
class CameraIssue:
    severity: str
    level: str
    message: str
    fixable: bool = False



def find_level(project: dict, level_id: str) -> dict:
    level = _find_level_or_none(project, level_id)
    if level is None:
        raise SystemExit(f"level {level_id!r} not found")
    return level


def find_entity_def(project: dict, identifier: str) -> dict:
    ent = _find_entity_def_or_none(project, identifier)
    if ent is None:
        raise SystemExit(f"entity def {identifier!r} not found")
    return ent


def iter_camera_zones(level: dict):
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__type") != "Entities":
            continue
        for entity in layer.get("entityInstances") or []:
            if entity.get("__identifier") == CAMERA_ENTITY:
                yield layer, entity


def intgrid_bbox(level: dict, layer_name: str = "Collision") -> tuple[int, int, int, int] | None:
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__identifier") != layer_name or layer.get("__type") != "IntGrid":
            continue
        grid = int(layer.get("__gridSize") or 16)
        c_wid = int(layer.get("__cWid") or 0)
        c_hei = int(layer.get("__cHei") or 0)
        csv = layer.get("intGridCsv") or []
        xs: list[int] = []
        ys: list[int] = []
        for cy in range(c_hei):
            for cx in range(c_wid):
                idx = cy * c_wid + cx
                if idx < len(csv) and int(csv[idx] or 0) != 0:
                    xs.append(cx)
                    ys.append(cy)
        if not xs:
            return None
        return min(xs) * grid, min(ys) * grid, (max(xs) + 1) * grid, (max(ys) + 1) * grid
    return None


def target_camera_rect(level: dict, margin: int) -> tuple[int, int, int, int]:
    bbox = intgrid_bbox(level)
    if bbox is None:
        bbox = (0, 0, int(level.get("pxWid") or 0), int(level.get("pxHei") or 0))
    x0, y0, x1, y1 = bbox
    x0 = max(0, x0 - margin)
    y0 = max(0, y0 - margin)
    x1 = min(int(level.get("pxWid") or x1), x1 + margin)
    y1 = min(int(level.get("pxHei") or y1), y1 + margin)
    return x0, y0, max(1, x1 - x0), max(1, y1 - y0)


def covers(a: tuple[int, int, int, int], b: tuple[int, int, int, int]) -> bool:
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax <= bx and ay <= by and ax + aw >= bx + bw and ay + ah >= by + bh


def entity_rect(entity: dict) -> tuple[int, int, int, int]:
    px = entity.get("px") or [0, 0]
    return int(px[0]), int(px[1]), int(entity.get("width") or 0), int(entity.get("height") or 0)


def collect_camera_issues(project: dict, level_filter: str | None = None, margin: int = 0) -> list[CameraIssue]:
    issues: list[CameraIssue] = []
    levels = project.get("levels", []) or []
    for level in levels:
        if level_filter and level.get("identifier") != level_filter:
            continue
        target = target_camera_rect(level, margin)
        cameras = list(iter_camera_zones(level))
        if not cameras:
            issues.append(CameraIssue("warning", str(level.get("identifier")), "no CameraZone found", fixable=True))
            continue
        if not any(layer.get("__identifier") == CAMERA_LAYER for layer, _ in cameras):
            issues.append(CameraIssue("error", str(level.get("identifier")), f"CameraZone exists but not on {CAMERA_LAYER}", fixable=True))
        if not any(covers(entity_rect(entity), target) for _, entity in cameras):
            issues.append(CameraIssue("warning", str(level.get("identifier")), f"no CameraZone covers target play rect {target}", fixable=True))
    return issues



def field_instances_for_camera(entity_def: dict, level_id: str):
    fields = []
    for fdef in entity_def.get("fieldDefs", []) or []:
        ident = fdef.get("identifier")
        value = default_field_value(fdef)
        if ident in {"id", "name"}:
            value = f"{level_id}_camera"
        fields.append({
            "__identifier": ident,
            "__type": fdef.get("__type"),
            "__value": value,
            "__tile": None,
            "defUid": fdef.get("uid"),
            "realEditorValues": [],
        })
    return fields


def autocover_camera(project: dict, level_id: str, margin: int, create: bool) -> str:
    level = find_level(project, level_id)
    x, y, w, h = target_camera_rect(level, margin)
    cameras = list(iter_camera_zones(level))
    target_layer_def = ensure_entities_layer_def(project, CAMERA_LAYER, clone_from="Ambition")
    target_layer = ensure_entities_layer_instance(project, level, CAMERA_LAYER, dest_def=target_layer_def, clone_from="Ambition")
    if cameras:
        layer, ent = cameras[0]
        if layer is not target_layer:
            try:
                layer.get("entityInstances", []).remove(ent)
            except ValueError:
                pass
            target_layer.setdefault("entityInstances", []).append(ent)
        ent["px"] = [x, y]
        ent["width"] = w
        ent["height"] = h
        ent["__grid"] = [x // 16, y // 16]
        ent["__worldX"] = int(level.get("worldX") or 0) + x
        ent["__worldY"] = int(level.get("worldY") or 0) + y
        return f"updated CameraZone {ent.get('iid')} in {level_id} to {(x, y, w, h)}"
    if not create:
        raise SystemExit(f"{level_id} has no CameraZone; rerun with --create")
    entity_def = find_entity_def(project, CAMERA_ENTITY)
    iid = f"CameraZone-{alloc_uid(project)}"
    ent = {
        "__identifier": CAMERA_ENTITY,
        "__grid": [x // 16, y // 16],
        "__pivot": [0, 0],
        "__tags": [],
        "__tile": None,
        "__smartColor": entity_def.get("color", "#86A8E7"),
        "iid": iid,
        "width": w,
        "height": h,
        "defUid": entity_def.get("uid"),
        "px": [x, y],
        "fieldInstances": field_instances_for_camera(entity_def, str(level.get("identifier"))),
        "__worldX": int(level.get("worldX") or 0) + x,
        "__worldY": int(level.get("worldY") or 0) + y,
    }
    target_layer.setdefault("entityInstances", []).append(ent)
    return f"created CameraZone {iid} in {level_id} at {(x, y, w, h)}"


def format_issues(issues: list[CameraIssue]) -> str:
    if not issues:
        return "camera audit passed.\n"
    lines = ["Camera audit issues:"]
    for issue in issues:
        suffix = " [fixable]" if issue.fixable else ""
        lines.append(f"  {issue.severity}: {issue.level}: {issue.message}{suffix}")
    return "\n".join(lines) + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Audit/fix CameraZone placement and coverage.")
    ap.add_argument("action", choices=["audit", "auto-cover"])
    ap.add_argument("ldtk", type=Path, nargs="?", default=DEFAULT_LDTK)
    ap.add_argument("--level")
    ap.add_argument("--margin", type=int, default=0)
    ap.add_argument("--create", action="store_true")
    ap.add_argument("--format", choices=["text", "json"], default="text")
    ap.add_argument("--in-place", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args(argv)

    if args.action == "auto-cover":
        if not args.level:
            raise SystemExit("camera auto-cover requires --level")
        if not args.in_place and not args.output:
            raise SystemExit("camera auto-cover requires --in-place or --output")
        tx = LdtkTransaction(args.ldtk, in_place=args.in_place, output=args.output)
        msg = autocover_camera(tx.project, args.level, args.margin, args.create)
        tx.note_changed([msg])
        out = tx.write_if_changed()
        print(f"{msg}; wrote {out}")
        project = tx.project
    else:
        project = load_project(args.ldtk)
    issues = collect_camera_issues(project, args.level, args.margin)
    if args.format == "json":
        print(json.dumps([issue.__dict__ for issue in issues], indent=2, sort_keys=True))
    else:
        print(format_issues(issues), end="")
    return 1 if any(i.severity == "error" for i in issues) else 0


if __name__ == "__main__":
    raise SystemExit(main())
