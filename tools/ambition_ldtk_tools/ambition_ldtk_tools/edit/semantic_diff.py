#!/usr/bin/env python3
"""Semantic LDtk diffs for agent/code-review workflows.

Raw LDtk JSON diffs are noisy because editor caches, layer arrays, and world
coordinates change in large blocks. This module compares the authored concepts
agents usually care about: level positions/sizes, entity layer placement, entity
fields, IntGrid cell counts, layer defs, entity defs, and tilesets.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Iterable

from ambition_ldtk_tools.ldtk import load_project


@dataclass(frozen=True)
class Change:
    kind: str
    path: str
    before: Any
    after: Any
    detail: str



def level_map(project: dict) -> dict[str, dict]:
    return {str(level.get("identifier")): level for level in project.get("levels", [])}


def field_map(obj: dict) -> dict[str, Any]:
    return {
        str(field.get("__identifier")): field.get("__value")
        for field in obj.get("fieldInstances", []) or []
    }


def iter_entities(level: dict) -> Iterable[tuple[str, dict]]:
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__type") != "Entities":
            continue
        layer_id = str(layer.get("__identifier"))
        for entity in layer.get("entityInstances", []) or []:
            yield layer_id, entity


def entity_key(entity: dict) -> str:
    return str(entity.get("iid") or f"{entity.get('__identifier')}@{entity.get('px')}")


def entity_snapshot(level: dict) -> dict[str, dict]:
    rows: dict[str, dict] = {}
    for layer_id, entity in iter_entities(level):
        rows[entity_key(entity)] = {
            "identifier": entity.get("__identifier"),
            "layer": layer_id,
            "px": list(entity.get("px") or [0, 0]),
            "size": [entity.get("width"), entity.get("height")],
            "fields": field_map(entity),
        }
    return rows


def intgrid_counts(level: dict) -> dict[str, dict[int, int]]:
    rows: dict[str, dict[int, int]] = {}
    for layer in level.get("layerInstances", []) or []:
        if layer.get("__type") != "IntGrid":
            continue
        counts: dict[int, int] = {}
        for raw in layer.get("intGridCsv") or []:
            value = int(raw or 0)
            if value:
                counts[value] = counts.get(value, 0) + 1
        rows[str(layer.get("__identifier"))] = counts
    return rows


def def_identifiers(project: dict, name: str) -> set[str]:
    return {str(row.get("identifier")) for row in project.get("defs", {}).get(name, []) or []}


def tileset_snapshot(project: dict) -> dict[str, dict]:
    return {
        str(row.get("identifier")): {
            "relPath": row.get("relPath"),
            "tileGridSize": row.get("tileGridSize"),
            "size": [row.get("pxWid"), row.get("pxHei")],
        }
        for row in project.get("defs", {}).get("tilesets", []) or []
    }


def entity_visual_snapshot(project: dict) -> dict[str, dict]:
    return {
        str(row.get("identifier")): {
            "renderMode": row.get("renderMode"),
            "tileRenderMode": row.get("tileRenderMode"),
            "tilesetId": row.get("tilesetId"),
            "tileRect": row.get("tileRect"),
            "uiTileRect": row.get("uiTileRect"),
        }
        for row in project.get("defs", {}).get("entities", []) or []
        if row.get("renderMode") == "Tile" or row.get("tilesetId") is not None or row.get("tileRect") is not None or row.get("uiTileRect") is not None
    }


def semantic_changes(before: dict, after: dict) -> list[Change]:
    changes: list[Change] = []
    a_levels = level_map(before)
    b_levels = level_map(after)
    for level_id in sorted(set(a_levels) | set(b_levels)):
        a = a_levels.get(level_id)
        b = b_levels.get(level_id)
        if a is None:
            changes.append(Change("level_added", level_id, None, _level_pos(b), f"added level {level_id}"))
            continue
        if b is None:
            changes.append(Change("level_removed", level_id, _level_pos(a), None, f"removed level {level_id}"))
            continue
        if _level_pos(a) != _level_pos(b):
            changes.append(Change("level_moved", level_id, _level_pos(a), _level_pos(b), f"{level_id}: moved {_level_pos(a)} -> {_level_pos(b)}"))
        if _level_size(a) != _level_size(b):
            changes.append(Change("level_resized", level_id, _level_size(a), _level_size(b), f"{level_id}: resized {_level_size(a)} -> {_level_size(b)}"))
        if field_map(a) != field_map(b):
            for key in sorted(set(field_map(a)) | set(field_map(b))):
                av = field_map(a).get(key)
                bv = field_map(b).get(key)
                if av != bv:
                    changes.append(Change("level_field", f"{level_id}.{key}", av, bv, f"{level_id}: field {key} {av!r} -> {bv!r}"))

        a_ents = entity_snapshot(a)
        b_ents = entity_snapshot(b)
        for iid in sorted(set(a_ents) | set(b_ents)):
            ae = a_ents.get(iid)
            be = b_ents.get(iid)
            path = f"{level_id}/{iid}"
            if ae is None:
                changes.append(Change("entity_added", path, None, be, f"{level_id}: added {be['identifier']} {iid} on {be['layer']}"))
                continue
            if be is None:
                changes.append(Change("entity_removed", path, ae, None, f"{level_id}: removed {ae['identifier']} {iid} from {ae['layer']}"))
                continue
            if ae["layer"] != be["layer"]:
                changes.append(Change("entity_layer", path, ae["layer"], be["layer"], f"{level_id}: {be['identifier']} {iid} layer {ae['layer']} -> {be['layer']}"))
            if ae["px"] != be["px"]:
                changes.append(Change("entity_moved", path, ae["px"], be["px"], f"{level_id}: {be['identifier']} {iid} moved {ae['px']} -> {be['px']}"))
            if ae["size"] != be["size"]:
                changes.append(Change("entity_resized", path, ae["size"], be["size"], f"{level_id}: {be['identifier']} {iid} resized {ae['size']} -> {be['size']}"))
            if ae["fields"] != be["fields"]:
                for key in sorted(set(ae["fields"]) | set(be["fields"])):
                    av = ae["fields"].get(key)
                    bv = be["fields"].get(key)
                    if av != bv:
                        changes.append(Change("entity_field", f"{path}.{key}", av, bv, f"{level_id}: {be['identifier']} {iid} field {key} {av!r} -> {bv!r}"))

        a_int = intgrid_counts(a)
        b_int = intgrid_counts(b)
        for layer_id in sorted(set(a_int) | set(b_int)):
            if a_int.get(layer_id) != b_int.get(layer_id):
                changes.append(Change("intgrid_counts", f"{level_id}/{layer_id}", a_int.get(layer_id, {}), b_int.get(layer_id, {}), f"{level_id}: IntGrid {layer_id} value counts changed"))

    for name, kind in [("layers", "layer_def"), ("entities", "entity_def")]:
        a_ids = def_identifiers(before, name)
        b_ids = def_identifiers(after, name)
        for ident in sorted(b_ids - a_ids):
            changes.append(Change(f"{kind}_added", ident, None, ident, f"added {kind} {ident}"))
        for ident in sorted(a_ids - b_ids):
            changes.append(Change(f"{kind}_removed", ident, ident, None, f"removed {kind} {ident}"))

    a_ts = tileset_snapshot(before)
    b_ts = tileset_snapshot(after)
    for ident in sorted(set(a_ts) | set(b_ts)):
        if a_ts.get(ident) != b_ts.get(ident):
            changes.append(Change("tileset", ident, a_ts.get(ident), b_ts.get(ident), f"tileset {ident} changed"))

    a_visual = entity_visual_snapshot(before)
    b_visual = entity_visual_snapshot(after)
    for ident in sorted(set(a_visual) | set(b_visual)):
        if a_visual.get(ident) != b_visual.get(ident):
            changes.append(Change(
                "entity_def_visual",
                ident,
                a_visual.get(ident),
                b_visual.get(ident),
                f"entity def {ident} editor visual changed",
            ))
    return changes


def _level_pos(level: dict | None) -> list[int | None]:
    if level is None:
        return [None, None]
    return [level.get("worldX"), level.get("worldY")]


def _level_size(level: dict | None) -> list[int | None]:
    if level is None:
        return [None, None]
    return [level.get("pxWid"), level.get("pxHei")]


def format_text(changes: list[Change]) -> str:
    if not changes:
        return "No semantic LDtk changes detected.\n"
    by_kind: dict[str, list[Change]] = {}
    for change in changes:
        by_kind.setdefault(change.kind, []).append(change)
    lines = ["Semantic LDtk diff", f"changes: {len(changes)}", ""]
    for kind in sorted(by_kind):
        lines.append(f"{kind} ({len(by_kind[kind])}):")
        for change in by_kind[kind]:
            lines.append(f"  - {change.detail}")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description="Semantic LDtk diff for reviewable agent edits.")
    ap.add_argument("action", choices=["semantic"])
    ap.add_argument("before", type=Path)
    ap.add_argument("after", type=Path)
    ap.add_argument("--format", choices=["text", "json"], default="text")
    ap.add_argument("--kind", action="append", default=[], help="Filter by change kind; repeatable.")
    args = ap.parse_args(argv)

    changes = semantic_changes(load_project(args.before), load_project(args.after))
    if args.kind:
        kinds = set(args.kind)
        changes = [c for c in changes if c.kind in kinds]
    if args.format == "json":
        print(json.dumps([asdict(c) for c in changes], indent=2, sort_keys=True))
    else:
        print(format_text(changes), end="")
    return 1 if changes else 0


if __name__ == "__main__":
    raise SystemExit(main())
