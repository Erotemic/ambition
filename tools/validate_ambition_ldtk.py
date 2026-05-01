#!/usr/bin/env python3
"""Validate the Ambition subset of LDtk used by the sandbox.

This deliberately validates gameplay-authoring semantics rather than the full
LDtk JSON schema. Use LDtk's official JSON schema for editor-format validation,
and this script for Ambition-specific contracts: active-area stitching,
PlayerStart counts, known entity identifiers, top-left pivots, and required
custom fields.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

KNOWN_ENTITIES = {
    "PlayerStart",
    "Solid",
    "OneWayPlatform",
    "BlinkWall",
    "HazardBlock",
    "PogoOrb",
    "ReboundPad",
    "LoadingZone",
    "DamageVolume",
    "KinematicPath",
    "NpcSpawn",
    "PickupSpawn",
    "ChestSpawn",
    "Breakable",
    "EnemySpawn",
    "BossSpawn",
    "DebugLabel",
    "CameraZone",
    "StitchedBoundary",
}
GRID = 16
AMBITION_LAYER = "Ambition"


def field_value(fields, name, default=None):
    for field in fields or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return default


def active_area(level):
    return field_value(level.get("fieldInstances", []), "activeArea", level.get("identifier", "<unnamed>"))


def ambition_layer(level):
    for layer in level.get("layerInstances") or []:
        if layer.get("__identifier") == AMBITION_LAYER:
            return layer
    return None


def entity_name(entity):
    return f"{entity.get('__identifier')} {entity.get('iid', '<no-iid>')}"


def parse_points(value):
    points = []
    for pair in str(value or "").split(";"):
        if not pair.strip():
            continue
        parts = [part.strip() for part in pair.split(",")]
        if len(parts) != 2:
            continue
        try:
            points.append((float(parts[0]), float(parts[1])))
        except ValueError:
            continue
    return points


def validate(path: Path):
    errors = []
    warnings = []
    try:
        project = json.loads(path.read_text())
    except Exception as ex:  # noqa: BLE001 - command line validator should print parser details
        return [f"failed to parse JSON: {ex}"], []

    levels = project.get("levels") or []
    if not levels:
        errors.append("project has no levels")
    if project.get("jsonVersion") != "1.5.3":
        warnings.append(f"expected LDtk jsonVersion 1.5.3, got {project.get('jsonVersion')!r}")

    seen_levels = set()
    starts_by_area = Counter()
    levels_by_area = defaultdict(list)
    zones_by_area = defaultdict(set)
    requested_links = []

    for level in levels:
        identifier = level.get("identifier", "<unnamed>")
        if identifier in seen_levels:
            errors.append(f"duplicate level identifier {identifier!r}")
        seen_levels.add(identifier)
        width = int(level.get("pxWid", 0) or 0)
        height = int(level.get("pxHei", 0) or 0)
        if width <= 0 or height <= 0:
            errors.append(f"level {identifier!r} has non-positive dimensions {width}x{height}")
        world_x = int(level.get("worldX", 0) or 0)
        world_y = int(level.get("worldY", 0) or 0)
        if world_x % GRID or world_y % GRID:
            warnings.append(f"level {identifier!r} origin ({world_x}, {world_y}) is not {GRID}px aligned")
        area = str(active_area(level))
        levels_by_area[area].append(identifier)

        layer = ambition_layer(level)
        if layer is None:
            errors.append(f"level {identifier!r} is missing {AMBITION_LAYER!r} entity layer")
            continue
        for entity in layer.get("entityInstances") or []:
            ident = entity.get("__identifier")
            if ident not in KNOWN_ENTITIES:
                errors.append(f"level {identifier!r} has unsupported entity {ident!r} ({entity.get('iid')})")
            width = int(entity.get("width", 0) or 0)
            height = int(entity.get("height", 0) or 0)
            px = entity.get("px") or [0, 0]
            if width <= 0 or height <= 0:
                errors.append(f"level {identifier!r} entity {entity_name(entity)} has non-positive dimensions")
            if len(px) != 2 or px[0] < 0 or px[1] < 0 or px[0] + width > level.get("pxWid", 0) or px[1] + height > level.get("pxHei", 0):
                errors.append(f"level {identifier!r} entity {entity_name(entity)} is outside level bounds")
            pivot = entity.get("__pivot", [0, 0])
            if len(pivot) == 2 and (abs(float(pivot[0])) > 1e-6 or abs(float(pivot[1])) > 1e-6):
                errors.append(f"level {identifier!r} entity {entity_name(entity)} must use top-left pivot [0, 0]")
            fields = entity.get("fieldInstances") or []
            if ident == "PlayerStart":
                starts_by_area[area] += 1
            elif ident == "BlinkWall" and field_value(fields, "tier", "Soft") not in {"Soft", "Hard"}:
                errors.append(f"BlinkWall {entity.get('iid')} has invalid tier")
            elif ident == "ReboundPad" and (field_value(fields, "impulseX") is None or field_value(fields, "impulseY") is None):
                errors.append(f"ReboundPad {entity.get('iid')} requires impulseX and impulseY")
            elif ident == "DebugLabel" and field_value(fields, "text") is None:
                errors.append(f"DebugLabel {entity.get('iid')} requires text")
            elif ident == "LoadingZone":
                zone_id = field_value(fields, "id")
                target_room = field_value(fields, "target_room")
                target_zone = field_value(fields, "target_zone")
                if zone_id is None:
                    errors.append(f"LoadingZone {entity.get('iid')} requires id")
                else:
                    zones_by_area[area].add(str(zone_id))
                if target_room is None or target_zone is None:
                    errors.append(f"LoadingZone {entity.get('iid')} requires target_room and target_zone")
                else:
                    requested_links.append((identifier, area, str(zone_id), str(target_room), str(target_zone)))
            elif ident == "KinematicPath":
                if len(parse_points(field_value(fields, "points", ""))) < 2:
                    errors.append(f"KinematicPath {entity.get('iid')} requires at least two points")
                if field_value(fields, "speed") is None:
                    errors.append(f"KinematicPath {entity.get('iid')} requires speed")
                if field_value(fields, "mode", "PingPong") not in {"Once", "Loop", "PingPong"}:
                    errors.append(f"KinematicPath {entity.get('iid')} has invalid mode")
            elif ident == "DamageVolume":
                has_any_path = any(field_value(fields, name) is not None for name in ("path_points", "path_speed", "path_mode"))
                if has_any_path:
                    if len(parse_points(field_value(fields, "path_points", ""))) < 2:
                        errors.append(f"DamageVolume {entity.get('iid')} path_points requires at least two points")
                    if field_value(fields, "path_speed") is None:
                        errors.append(f"DamageVolume {entity.get('iid')} path requires path_speed")
                    if field_value(fields, "path_mode", "PingPong") not in {"Once", "Loop", "PingPong"}:
                        errors.append(f"DamageVolume {entity.get('iid')} has invalid path_mode")
            elif ident == "Breakable":
                respawn = str(field_value(fields, "respawn", "Never"))
                if not (respawn in {"Never", "OnRoomReload", "Persistent", "None"} or respawn.startswith("AfterSeconds:")):
                    errors.append(f"Breakable {entity.get('iid')} has invalid respawn value {respawn!r}")

    for source_level, area, zone_id, target_room, target_zone in requested_links:
        if target_room not in levels_by_area:
            errors.append(f"LoadingZone {zone_id!r} in {source_level!r} targets unknown room/activeArea {target_room!r}")
        elif target_zone not in zones_by_area[target_room]:
            errors.append(f"LoadingZone {zone_id!r} in {source_level!r} targets missing zone {target_zone!r} in {target_room!r}")

    for area, level_names in levels_by_area.items():
        count = starts_by_area[area]
        if count != 1:
            errors.append(f"active area {area!r} has {count} PlayerStart entities across {level_names}; expected exactly 1")

    return errors, warnings


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path, help="Path to an Ambition-authored .ldtk file")
    args = parser.parse_args(argv)
    errors, warnings = validate(args.path)
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: {args.path} passes Ambition LDtk validation ({len(warnings)} warnings)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
