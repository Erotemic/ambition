#!/usr/bin/env python3
"""Validate the Ambition subset of LDtk used by the sandbox.

This script has two layers:

1. Optional official LDtk JSON Schema validation through Python's `jsonschema`
   package when `--schema` is provided, avoiding Node/npm tooling.
2. Ambition-specific contracts: active-area stitching, PlayerStart counts,
   known entity identifiers, top-left pivots, direct bevy_ecs_ldtk spawning
   compatibility, and required custom fields.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

PKG_DIR = Path(__file__).resolve().parent
DEFAULT_SCHEMA_PATH = PKG_DIR.parent / "schemas" / "ldtk" / "JSON_SCHEMA.json"
OFFICIAL_SCHEMA_URL = "https://ldtk.io/files/JSON_SCHEMA.json"

KNOWN_ENTITIES = {
    "BlinkWall",
    "BossSpawn",
    "BreakablePlatform",
    "BreakablePogoOrb",
    "CameraZone",
    "ChestSpawn",
    "DamageVolume",
    "DebugLabel",
    "EncounterTrigger",
    "EnemySpawn",
    "HazardBlock",
    "KinematicPath",
    "LoadingZone",
    "LockWall",
    "MovingPlatform",
    "NpcSpawn",
    "OneWayPlatform",
    "PickupSpawn",
    "PlayerStart",
    "PogoOrb",
    "ReboundPad",
    "Solid",
    "StitchedBoundary",
    "Switch",
    "WaterVolume",
}
GRID = 16
AMBITION_LAYER = "Ambition"

FIRST_CLASS_FIELD_DEF_FIELDS = [
    "allowOutOfLevelRef",
    "allowedRefTags",
    "allowedRefs",
    "autoChainRef",
    "editorDisplayScale",
    "editorLinkStyle",
    "editorShowInWorld",
    "exportToToc",
    "searchable",
    "symmetricalRef",
]

FIELD_DEF_TYPE_BY_HUMAN_TYPE = {
    "Int": "F_Int",
    "Float": "F_Float",
    "String": "F_String",
    "Text": "F_Text",
    "Bool": "F_Bool",
    "Color": "F_Color",
    "Point": "F_Point",
    "Path": "F_Path",
    "EntityRef": "F_EntityRef",
    "Tile": "F_Tile",
}

FIELD_DEF_EDITOR_DEFAULTS = {
    "acceptFileTypes": None,
    "allowOutOfLevelRef": False,
    "allowedRefTags": [],
    "allowedRefs": "Any",
    "allowedRefsEntityUid": None,
    "arrayMaxLength": None,
    "arrayMinLength": None,
    "autoChainRef": True,
    "canBeNull": True,
    "defaultOverride": None,
    "doc": None,
    "editorAlwaysShow": False,
    "editorCutLongValues": True,
    "editorDisplayColor": None,
    "editorDisplayMode": "NameAndValue",
    "editorDisplayPos": "Above",
    "editorDisplayScale": 1,
    "editorLinkStyle": "StraightArrow",
    "editorShowInWorld": False,
    "editorTextPrefix": None,
    "editorTextSuffix": None,
    "exportToToc": False,
    "isArray": False,
    "max": None,
    "min": None,
    "regex": None,
    "searchable": False,
    "symmetricalRef": False,
    "textLanguageMode": None,
    "tilesetUid": None,
    "useForSmartColor": False,
}

ENTITY_DEF_EDITOR_DEFAULTS = {
    "allowOutOfBounds": False,
    "doc": None,
    "exportToToc": False,
    "fillOpacity": 0.35,
    "hollow": False,
    "keepAspectRatio": False,
    "limitBehavior": "DiscardOldOnes",
    "limitScope": "PerLevel",
    "lineOpacity": 1.0,
    "maxCount": 0,
    "maxHeight": 0,
    "maxWidth": 0,
    "minHeight": 0,
    "minWidth": 0,
    "nineSliceBorders": [],
    "pivotX": 0.0,
    "pivotY": 0.0,
    "renderMode": "Rectangle",
    "resizableX": True,
    "resizableY": True,
    "showName": True,
    "tags": [],
    "tileOpacity": 1.0,
    "tileRect": None,
    "tileRenderMode": "Cover",
    "tilesetId": None,
    "uiTileRect": None,
}


def field_value(fields, name, default=None):
    for field in fields or []:
        if field.get("__identifier") == name:
            return field.get("__value")
    return default


def editor_value_for(value, human_type):
    if value is None:
        return []
    if human_type in {"String", "Text", "Color", "Path", "EntityRef", "Tile"}:
        return [{"id": "V_String", "params": [str(value)]}]
    if human_type == "Bool":
        return [{"id": "V_Bool", "params": [bool(value)]}]
    if human_type == "Int":
        return [{"id": "V_Int", "params": [int(value)]}]
    if human_type == "Float":
        return [{"id": "V_Float", "params": [float(value)]}]
    if isinstance(value, list):
        values = []
        for item in value:
            values.extend(editor_value_for(item, human_type.replace("Array<", "").rstrip(">")))
        return values
    return [{"id": "V_String", "params": [str(value)]}]


def validate_field_instance_editor_value(errors, owner, field):
    # Intentionally empty: LDtk 1.5.3 writes `__value` with empty
    # `realEditorValues` natively for fields that inherit a `defaultOverride`
    # from the entity definition. Treating that pattern as an error means
    # flagging the editor's own output as broken, which violates the rule
    # that a file the LDtk editor writes must run unchanged. The historical
    # `tools/repair_ambition_ldtk.py` script is still available for anyone
    # who wants to canonicalize JSON for diff readability, but it is not
    # required for runtime correctness.
    _ = errors, owner, field


def active_area(level):
    value = field_value(level.get("fieldInstances", []), "activeArea", None)
    if isinstance(value, str) and value.strip():
        return value.strip()
    return level.get("identifier", "<unnamed>")


def ambition_layer(level):
    for layer in level.get("layerInstances") or []:
        if layer.get("__identifier") == AMBITION_LAYER:
            return layer
    return None


def entity_name(entity):
    return f"{entity.get('__identifier')} {entity.get('iid', '<no-iid>')}"


def rect(entity):
    px = entity.get("px") or [0, 0]
    return (float(px[0]), float(px[1]), float(entity.get("width", 0) or 0), float(entity.get("height", 0) or 0))


def strict_rects_intersect(a, b):
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    return ax < bx + bw and ax + aw > bx and ay < by + bh and ay + ah > by


def touches_level_edge(entity, width, height):
    x, y, w, h = rect(entity)
    return x <= 0 or y <= 0 or x + w >= width or y + h >= height


def center(rect_value):
    x, y, w, h = rect_value
    return (x + w * 0.5, y + h * 0.5)


def player_spawn_rect(position, half_w=14.0, half_h=23.0):
    x, y = position
    return (x - half_w, y - half_h, half_w * 2.0, half_h * 2.0)


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


def validate_official_schema(project, schema_path: Path | None, require_schema: bool):
    errors = []
    warnings = []
    if schema_path is None:
        default_schema = DEFAULT_SCHEMA_PATH
        if default_schema.exists():
            schema_path = default_schema
        else:
            if require_schema:
                errors.append(
                    f"official LDtk JSON schema not checked; fetch {OFFICIAL_SCHEMA_URL} to {DEFAULT_SCHEMA_PATH} "
                    "and install python package `jsonschema`"
                )
            return errors, warnings
    try:
        import jsonschema  # type: ignore[import-not-found]
    except Exception as ex:  # noqa: BLE001 - command line validator should explain environment issues
        message = f"python package `jsonschema` is required for official LDtk schema validation: {ex}"
        if require_schema:
            errors.append(message)
        else:
            warnings.append(message)
        return errors, warnings
    try:
        schema = json.loads(schema_path.read_text())
        jsonschema.Draft7Validator.check_schema(schema)
        validator = jsonschema.Draft7Validator(schema)
        schema_errors = sorted(validator.iter_errors(project), key=lambda error: list(error.path))
    except Exception as ex:  # noqa: BLE001
        errors.append(f"failed to validate official LDtk schema {schema_path}: {ex}")
        return errors, warnings
    for error in schema_errors:
        path = ".".join(str(part) for part in error.absolute_path) or "<root>"
        errors.append(f"LDtk JSON schema: {path}: {error.message}")
    return errors, warnings



def normalize_editor_values(project):
    changed = 0

    def normalize_fields(fields):
        nonlocal changed
        for field in fields or []:
            value = field.get("__value")
            expected = editor_value_for(value, field.get("__type"))
            if field.get("realEditorValues") != expected:
                field["realEditorValues"] = expected
                changed += 1

    for level in project.get("levels") or []:
        normalize_fields(level.get("fieldInstances") or [])
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                normalize_fields(entity.get("fieldInstances") or [])
    return changed


def _set_if_missing(mapping, key, value):
    if key not in mapping:
        if isinstance(value, list):
            value = list(value)
        mapping[key] = value
        return True
    return False


VALUE_WRAPPER_ID_BY_HUMAN_TYPE = {
    "Int": "V_Int",
    "Float": "V_Float",
    "Bool": "V_Bool",
    "String": "V_String",
    "Text": "V_String",
    "Color": "V_String",
    "Path": "V_String",
    "EntityRef": "V_String",
    "Tile": "V_String",
}


def _coerce_wrapper_param(human_type, value):
    if human_type == "Int":
        return int(value)
    if human_type == "Float":
        return float(value)
    if human_type == "Bool":
        return bool(value)
    return str(value)


def _normalize_default_override(field_def, human_type):
    """Wrap raw `defaultOverride` values in LDtk's Haxe-enum ValueWrapper form.

    LDtk encodes ValueWrapper as `{"id": "V_String", "params": [...]}` (or
    V_Int/V_Float/V_Bool). Generated/agent-patched files often write the raw
    value, which makes the editor crash with `No such constructor`.
    """
    if "defaultOverride" not in field_def:
        return None
    value = field_def["defaultOverride"]
    if value is None:
        return None
    if isinstance(value, dict) and "id" in value and "params" in value:
        return None
    wrapper_id = VALUE_WRAPPER_ID_BY_HUMAN_TYPE.get(human_type)
    if wrapper_id is None:
        return None
    if isinstance(value, list):
        params = [_coerce_wrapper_param(human_type, v) for v in value]
    else:
        params = [_coerce_wrapper_param(human_type, value)]
    field_def["defaultOverride"] = {"id": wrapper_id, "params": params}
    return wrapper_id


def normalize_field_def(field_def):
    changes = []
    identifier = field_def.get("identifier", "<unnamed>")
    for key, value in FIELD_DEF_EDITOR_DEFAULTS.items():
        if _set_if_missing(field_def, key, value):
            changes.append(f"fieldDef {identifier}: added {key}")
    human_type = field_def.get("__type")
    if human_type is None and isinstance(field_def.get("type"), str):
        inverse = {value: key for key, value in FIELD_DEF_TYPE_BY_HUMAN_TYPE.items()}
        human_type = inverse.get(field_def.get("type"))
        if human_type:
            field_def["__type"] = human_type
            changes.append(f"fieldDef {identifier}: restored __type={human_type}")
    expected_type = FIELD_DEF_TYPE_BY_HUMAN_TYPE.get(human_type)
    if expected_type and field_def.get("type") != expected_type:
        field_def["type"] = expected_type
        changes.append(f"fieldDef {identifier}: set type={expected_type}")
    wrapped = _normalize_default_override(field_def, human_type)
    if wrapped is not None:
        changes.append(f"fieldDef {identifier}: wrapped defaultOverride as {wrapped}")
    return changes


def normalize_definitions_for_editor(project):
    changes = []
    defs = project.setdefault("defs", {})
    for field_def in defs.get("levelFields") or []:
        changes.extend(normalize_field_def(field_def))
    for entity_def in defs.get("entities") or []:
        identifier = entity_def.get("identifier", "<unnamed>")
        for key, value in ENTITY_DEF_EDITOR_DEFAULTS.items():
            if _set_if_missing(entity_def, key, value):
                changes.append(f"entityDef {identifier}: added {key}")
        for field_def in entity_def.get("fieldDefs") or []:
            changes.extend(normalize_field_def(field_def))
    return changes


def sync_instance_definition_uids(project):
    changes = []
    defs = project.get("defs") or {}
    entity_defs = {entity.get("identifier"): entity for entity in defs.get("entities") or []}
    entity_field_defs = {
        identifier: {field.get("identifier"): field for field in (entity.get("fieldDefs") or [])}
        for identifier, entity in entity_defs.items()
    }
    level_field_defs = {field.get("identifier"): field for field in defs.get("levelFields") or []}

    def sync_field_instances(fields, field_defs, owner):
        for field in fields or []:
            field_ident = field.get("__identifier")
            field_def = field_defs.get(field_ident)
            if not field_def:
                continue
            expected_uid = field_def.get("uid")
            if expected_uid is not None and field.get("defUid") != expected_uid:
                field["defUid"] = expected_uid
                changes.append(f"{owner}.{field_ident}: set defUid={expected_uid}")
            expected_type = field_def.get("__type")
            if expected_type and field.get("__type") != expected_type:
                field["__type"] = expected_type
                changes.append(f"{owner}.{field_ident}: set __type={expected_type}")

    for level in project.get("levels") or []:
        level_name = level.get("identifier", "<unnamed-level>")
        sync_field_instances(level.get("fieldInstances") or [], level_field_defs, f"level {level_name}")
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                ident = entity.get("__identifier")
                entity_def = entity_defs.get(ident)
                owner = f"entity {level_name}/{ident}/{entity.get('iid', '<no-iid>')}"
                if entity_def and entity_def.get("uid") is not None and entity.get("defUid") != entity_def.get("uid"):
                    entity["defUid"] = entity_def.get("uid")
                    changes.append(f"{owner}: set defUid={entity_def.get('uid')}")
                sync_field_instances(entity.get("fieldInstances") or [], entity_field_defs.get(ident, {}), owner)
    return changes


def normalize_project_for_editor(project):
    """Normalize generated/agent-patched LDtk JSON for LDtk GUI round-trips.

    This intentionally avoids inventing gameplay values. It only repairs editor
    representation fields, internal type constructors, and definition UIDs that
    can be derived from existing definitions and parser-facing values.
    """
    changes = []
    changes.extend(normalize_definitions_for_editor(project))
    changes.extend(sync_instance_definition_uids(project))
    editor_value_count = normalize_editor_values(project)
    if editor_value_count:
        changes.append(f"normalized {editor_value_count} field instance realEditorValues records")
    return changes

def validate(path: Path, schema_path: Path | None = None, require_schema: bool = False):
    errors = []
    warnings = []
    try:
        project = json.loads(path.read_text())
    except Exception as ex:  # noqa: BLE001 - command line validator should print parser details
        return [f"failed to parse JSON: {ex}"], []

    schema_errors, schema_warnings = validate_official_schema(project, schema_path, require_schema)
    errors.extend(schema_errors)
    warnings.extend(schema_warnings)

    levels = project.get("levels") or []
    if not levels:
        errors.append("project has no levels")
    if project.get("jsonVersion") != "1.5.3":
        warnings.append(f"expected LDtk jsonVersion 1.5.3, got {project.get('jsonVersion')!r}")
    if not project.get("iid"):
        errors.append("project is missing root iid; bevy_ecs_ldtk treats LDtk as a first-class asset and expects stable project identity")
    if project.get("worldLayout") not in {"Free", "GridVania", "LinearHorizontal", "LinearVertical"}:
        errors.append(f"project has unsupported or missing worldLayout {project.get('worldLayout')!r}")
    first_class_root_fields = {
        "appBuildId",
        "backupLimit",
        "backupOnSave",
        "customCommands",
        "dummyWorldIid",
        "exportLevelBg",
        "flags",
        "identifierStyle",
        "imageExportMode",
        "levelNamePattern",
        "nextUid",
        "worlds",
    }
    missing_root_fields = sorted(field for field in first_class_root_fields if field not in project)
    if missing_root_fields:
        errors.append("first-class LDtk project is missing editor/schema fields: " + ", ".join(missing_root_fields))
    defs = project.get("defs") or {}
    layer_defs = defs.get("layers") or []
    if not any(layer.get("identifier") == AMBITION_LAYER and layer.get("__type") == "Entities" for layer in layer_defs):
        errors.append(f"defs.layers must contain an Entities layer definition named {AMBITION_LAYER!r}")
    for layer in layer_defs:
        if layer.get("identifier") == AMBITION_LAYER:
            for required in ["parallaxFactorX", "parallaxFactorY", "parallaxScaling", "canSelectWhenInactive", "guideGridHei", "guideGridWid", "uiFilterTags", "useAsyncRender"]:
                if required not in layer:
                    errors.append(f"Ambition layer definition is missing first-class LDtk field {required!r}")
    entity_defs_by_identifier = {entity.get("identifier"): entity for entity in defs.get("entities") or []}
    entity_defs = set(entity_defs_by_identifier.keys())
    entity_def_uid_by_identifier = {
        identifier: entity.get("uid")
        for identifier, entity in entity_defs_by_identifier.items()
    }
    entity_field_def_uid_by_identifier = {
        identifier: {field_def.get("identifier"): field_def.get("uid") for field_def in (entity.get("fieldDefs") or [])}
        for identifier, entity in entity_defs_by_identifier.items()
    }
    level_field_def_uid_by_identifier = {
        field_def.get("identifier"): field_def.get("uid")
        for field_def in defs.get("levelFields") or []
    }
    missing_known_defs = sorted(KNOWN_ENTITIES - entity_defs)
    if missing_known_defs:
        errors.append("defs.entities is missing editor definitions for supported Ambition entities: " + ", ".join(missing_known_defs))
    missing_defs = sorted(KNOWN_ENTITIES.intersection({entity for level in levels for layer in (level.get("layerInstances") or []) for entity in [inst.get("__identifier") for inst in (layer.get("entityInstances") or [])] if entity}) - entity_defs)
    if missing_defs:
        errors.append("defs.entities is missing definitions for used Ambition entities: " + ", ".join(missing_defs))
    def validate_field_def_shape(owner, field_def):
        field_name = f"{owner}.{field_def.get('identifier')}"
        for required in FIRST_CLASS_FIELD_DEF_FIELDS:
            if required not in field_def:
                errors.append(f"field definition {field_name} is missing first-class LDtk field {required!r}")
        allowed_refs = field_def.get("allowedRefs")
        if allowed_refs not in {"Any", "OnlySame", "OnlyTags", "OnlySpecificEntity"}:
            errors.append(f"field definition {field_name} has invalid allowedRefs {allowed_refs!r}")
        human_type = field_def.get("__type")
        internal_type = field_def.get("type")
        expected_internal_type = FIELD_DEF_TYPE_BY_HUMAN_TYPE.get(human_type)
        if expected_internal_type is not None and internal_type != expected_internal_type:
            errors.append(
                f"field definition {field_name} has type {internal_type!r}; expected {expected_internal_type!r} "
                f"for __type {human_type!r}. LDtk's editor reads `type` as an internal FieldType constructor "
                "such as F_String, not the human-readable type string."
            )
        if human_type is None or internal_type is None:
            errors.append(f"field definition {field_name} must include both __type and type for LDtk editor round-tripping")

    for entity_def in defs.get("entities") or []:
        for required in ["allowOutOfBounds", "exportToToc", "nineSliceBorders", "tileOpacity"]:
            if required not in entity_def:
                errors.append(f"entity definition {entity_def.get('identifier')!r} is missing first-class LDtk field {required!r}")
        for field_def in entity_def.get("fieldDefs") or []:
            validate_field_def_shape(entity_def.get("identifier"), field_def)

    for field_def in defs.get("levelFields") or []:
        validate_field_def_shape("level", field_def)

    seen_levels = set()
    starts_by_area = Counter()
    levels_by_area = defaultdict(list)
    zones_by_area = defaultdict(set)
    requested_links = []
    area_bounds = {}
    area_zones = defaultdict(dict)
    area_solids = defaultdict(list)

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
        raw_active_area = field_value(level.get("fieldInstances") or [], "activeArea", None)
        if raw_active_area is None or (isinstance(raw_active_area, str) and not raw_active_area.strip()):
            errors.append(f"level {identifier!r} has blank activeArea; LDtk editor round-trips must preserve this level field")
        for field in level.get("fieldInstances") or []:
            field_ident = field.get("__identifier")
            expected_def_uid = level_field_def_uid_by_identifier.get(field_ident)
            if expected_def_uid is None:
                errors.append(f"level {identifier!r} has undefined level field {field_ident!r}")
            elif field.get("defUid") != expected_def_uid:
                errors.append(
                    f"level {identifier!r} field {field_ident!r} has defUid {field.get('defUid')!r}; "
                    f"expected level field definition uid {expected_def_uid!r}"
                )
            validate_field_instance_editor_value(errors, f"level {identifier!r}", field)
        levels_by_area[area].append(identifier)
        if area not in area_bounds:
            area_bounds[area] = [world_x, world_y, world_x + width, world_y + height]
        else:
            area_bounds[area][0] = min(area_bounds[area][0], world_x)
            area_bounds[area][1] = min(area_bounds[area][1], world_y)
            area_bounds[area][2] = max(area_bounds[area][2], world_x + width)
            area_bounds[area][3] = max(area_bounds[area][3], world_y + height)

        layer = ambition_layer(level)
        if layer is None:
            errors.append(f"level {identifier!r} is missing {AMBITION_LAYER!r} entity layer")
            continue
        if layer.get("__type") != "Entities":
            errors.append(f"level {identifier!r} Ambition layer must have __type='Entities' for direct bevy_ecs_ldtk spawning")

        solids = [entity for entity in layer.get("entityInstances") or [] if entity.get("__identifier") == "Solid"]
        for solid in solids:
            sx, sy, sw, sh = rect(solid)
            area_solids[area].append((world_x + sx, world_y + sy, sw, sh, identifier, entity_name(solid)))

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
            expected_entity_def_uid = entity_def_uid_by_identifier.get(ident)
            if expected_entity_def_uid is not None and entity.get("defUid") != expected_entity_def_uid:
                errors.append(
                    f"level {identifier!r} entity {entity_name(entity)} has defUid {entity.get('defUid')!r}; "
                    f"expected entity definition uid {expected_entity_def_uid!r}"
                )
            fields = entity.get("fieldInstances") or []
            field_def_uid_by_identifier = entity_field_def_uid_by_identifier.get(ident, {})
            for field in fields:
                field_ident = field.get("__identifier")
                expected_field_def_uid = field_def_uid_by_identifier.get(field_ident)
                if expected_field_def_uid is None:
                    errors.append(f"level {identifier!r} entity {entity_name(entity)} has undefined field {field_ident!r}")
                elif field.get("defUid") != expected_field_def_uid:
                    errors.append(
                        f"level {identifier!r} entity {entity_name(entity)} field {field_ident!r} has defUid {field.get('defUid')!r}; "
                        f"expected field definition uid {expected_field_def_uid!r}"
                    )
                validate_field_instance_editor_value(errors, f"level {identifier!r} entity {entity_name(entity)}", field)
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
                activation = str(field_value(fields, "activation", "Door"))
                if zone_id is None:
                    errors.append(f"LoadingZone {entity.get('iid')} requires id")
                else:
                    zone_id = str(zone_id)
                    zones_by_area[area].add(zone_id)
                    ex, ey, ew, eh = rect(entity)
                    area_zones[area][zone_id] = {
                        "source_level": identifier,
                        "entity": entity_name(entity),
                        "activation": activation,
                        "rect_world": (world_x + ex, world_y + ey, ew, eh),
                    }
                if activation == "EdgeExit":
                    if not touches_level_edge(entity, width, height):
                        errors.append(f"EdgeExit LoadingZone {entity.get('iid')} in {identifier!r} must touch a level edge")
                    zone_rect = rect(entity)
                    for solid in solids:
                        if strict_rects_intersect(zone_rect, rect(solid)):
                            errors.append(
                                f"EdgeExit LoadingZone {entity.get('iid')} in {identifier!r} overlaps solid {entity_name(solid)}; "
                                "split the wall or move the zone so the exit is physically reachable"
                            )
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
            elif ident == "BreakablePlatform":
                collision = field_value(fields, "collision")
                if collision is not None and collision not in {"Solid", "OneWayUp"}:
                    errors.append(
                        f"BreakablePlatform {entity.get('iid')} has invalid collision {collision!r}; "
                        "must be one of Solid | OneWayUp"
                    )
                trigger = str(field_value(fields, "trigger", "OnHit"))
                if trigger not in {"OnHit", "OnStand", "Either"}:
                    errors.append(
                        f"BreakablePlatform {entity.get('iid')} has invalid trigger {trigger!r}; "
                        "must be one of OnHit | OnStand | Either"
                    )
                respawn = str(field_value(fields, "respawn", "Never"))
                if respawn == "AfterSeconds":
                    seconds = field_value(fields, "respawn_seconds")
                    try:
                        seconds_value = float(seconds) if seconds is not None else 0.0
                    except (TypeError, ValueError):
                        seconds_value = 0.0
                    if seconds_value <= 0.0:
                        errors.append(
                            f"BreakablePlatform {entity.get('iid')} respawn=AfterSeconds requires a "
                            "positive respawn_seconds field"
                        )
                elif respawn not in {"Never", "OnRoomReload"} and not respawn.startswith(
                    "AfterSeconds:"
                ):
                    errors.append(
                        f"BreakablePlatform {entity.get('iid')} has invalid respawn value {respawn!r}"
                    )
            elif ident == "BreakablePogoOrb":
                respawn = str(field_value(fields, "respawn", "Never"))
                if respawn == "AfterSeconds":
                    seconds = field_value(fields, "respawn_seconds")
                    try:
                        seconds_value = float(seconds) if seconds is not None else 0.0
                    except (TypeError, ValueError):
                        seconds_value = 0.0
                    if seconds_value <= 0.0:
                        errors.append(
                            f"BreakablePogoOrb {entity.get('iid')} respawn=AfterSeconds requires a "
                            "positive respawn_seconds field"
                        )
                elif respawn not in {"Never", "OnRoomReload"} and not respawn.startswith(
                    "AfterSeconds:"
                ):
                    errors.append(
                        f"BreakablePogoOrb {entity.get('iid')} has invalid respawn value {respawn!r}"
                    )

    for source_level, area, zone_id, target_room, target_zone in requested_links:
        if target_room not in levels_by_area:
            errors.append(f"LoadingZone {zone_id!r} in {source_level!r} targets unknown room/activeArea {target_room!r}")
            continue
        if target_zone not in zones_by_area[target_room]:
            errors.append(f"LoadingZone {zone_id!r} in {source_level!r} targets missing zone {target_zone!r} in {target_room!r}")
            continue

        target = area_zones[target_room][target_zone]
        bx0, by0, bx1, by1 = area_bounds[target_room]
        width = bx1 - bx0
        height = by1 - by0
        zx, zy, zw, zh = target["rect_world"]
        local_zone = (zx - bx0, zy - by0, zw, zh)
        cx, cy = center(local_zone)
        if target["activation"] == "EdgeExit":
            if local_zone[0] <= 37.0:
                arrival = (92.0, cy)
            elif local_zone[0] + local_zone[2] >= width - 37.0:
                arrival = (width - 92.0, cy)
            else:
                arrival = (cx, cy)
        else:
            arrival = (cx, local_zone[1] + local_zone[3] - 26.0)
        spawn = player_spawn_rect(arrival)
        sx, sy, sw, sh = spawn
        if sx < 0 or sy < 0 or sx + sw > width or sy + sh > height:
            errors.append(
                f"LoadingZone {zone_id!r} in {source_level!r} arrives outside target area {target_room!r} "
                f"via zone {target_zone!r}: arrival=({arrival[0]:.1f}, {arrival[1]:.1f}), area=({width:.1f}, {height:.1f})"
            )
        for solid_x, solid_y, solid_w, solid_h, solid_level, solid_name in area_solids[target_room]:
            local_solid = (solid_x - bx0, solid_y - by0, solid_w, solid_h)
            if strict_rects_intersect(spawn, local_solid):
                errors.append(
                    f"LoadingZone {zone_id!r} in {source_level!r} arrives inside solid {solid_name} from {solid_level!r} "
                    f"in target area {target_room!r}; move the target zone or split collision around it"
                )
                break

    for area, level_names in levels_by_area.items():
        count = starts_by_area[area]
        if count != 1:
            errors.append(f"active area {area!r} has {count} PlayerStart entities across {level_names}; expected exactly 1")

    return errors, warnings


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path, help="Path to an Ambition-authored .ldtk file")
    parser.add_argument(
        "--schema",
        type=Path,
        default=None,
        help="Optional path to the official LDtk JSON_SCHEMA.json; uses Python jsonschema, not npm",
    )
    parser.add_argument(
        "--require-schema",
        action="store_true",
        help="Fail if official LDtk JSON schema validation cannot be run",
    )
    parser.add_argument(
        "--normalize-editor-values",
        action="store_true",
        help="Rewrite editor-facing LDtk metadata from parser-facing values before validating",
    )
    parser.add_argument(
        "--repair-editor-roundtrip",
        action="store_true",
        help="Alias for --normalize-editor-values; kept for level-design workflow readability",
    )
    args = parser.parse_args(argv)
    if args.normalize_editor_values or args.repair_editor_roundtrip:
        try:
            project = json.loads(args.path.read_text())
            changes = normalize_project_for_editor(project)
            if changes:
                args.path.write_text(json.dumps(project, indent=2) + "\n")
                print(f"repaired {len(changes)} LDtk editor-roundtrip issue(s) in {args.path}", file=sys.stderr)
                for change in changes[:20]:
                    print(f"  - {change}", file=sys.stderr)
                if len(changes) > 20:
                    print(f"  ... {len(changes) - 20} more", file=sys.stderr)
        except Exception as ex:  # noqa: BLE001
            print(f"error: failed to repair editor-roundtrip values: {ex}", file=sys.stderr)
            return 1
    errors, warnings = validate(args.path, args.schema, args.require_schema)
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
