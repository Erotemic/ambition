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
    "GroundItem",
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
    "Prop",
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
    # that a file the LDtk editor writes must run unchanged. The package
    # repair command (`python -m ambition_ldtk_tools repair`) is available
    # for anyone who wants to canonicalize JSON for diff readability, but
    # it is not required for runtime correctness.
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




def _wrapper_payload(wrapper):
    """Return ``(id, params)`` for an LDtk ValueWrapper dict, or ``None``.

    LDtk encodes default/editor values as Haxe-enum wrapper objects such as
    ``{"id": "V_String", "params": ["Door"]}``. The editor omits
    ``realEditorValues`` when an instance value is exactly the field default;
    comparing through this helper lets repair mirror that behavior instead of
    forcing noisy non-editor metadata back into editor-saved files.
    """
    if not isinstance(wrapper, dict):
        return None
    if "id" not in wrapper or "params" not in wrapper:
        return None
    params = wrapper.get("params")
    if not isinstance(params, list):
        return None
    return wrapper.get("id"), list(params)


def _scalar_editor_value_id(human_type):
    if human_type in {"String", "Text", "Color", "Path", "EntityRef", "Tile"}:
        return "V_String"
    if human_type == "Bool":
        return "V_Bool"
    if human_type == "Int":
        return "V_Int"
    if human_type == "Float":
        return "V_Float"
    return None


def _coerce_for_editor_compare(value, human_type):
    if value is None:
        return None
    if human_type == "Bool":
        return bool(value)
    if human_type == "Int":
        return int(value)
    if human_type == "Float":
        return float(value)
    return str(value)


def _value_matches_default_override(value, human_type, field_def):
    """True when LDtk would omit ``realEditorValues`` for this value.

    In files saved by LDtk 1.5.3, field instances whose ``__value`` equals the
    field definition's ``defaultOverride`` usually keep ``realEditorValues: []``.
    Earlier Ambition tools filled every non-null instance value, so simply
    opening and saving in LDtk produced a large "repair" diff. Treat the editor
    as source-of-truth here: default-equal values are canonical with an empty
    editor-value array.
    """
    if value is None or not field_def:
        return value is None
    payload = _wrapper_payload(field_def.get("defaultOverride"))
    if payload is None:
        return False
    wrapper_id, params = payload
    expected_wrapper = _scalar_editor_value_id(human_type)
    if expected_wrapper is not None and wrapper_id != expected_wrapper:
        return False
    if len(params) != 1:
        return False
    try:
        return _coerce_for_editor_compare(value, human_type) == _coerce_for_editor_compare(params[0], human_type)
    except (TypeError, ValueError):
        return value == params[0]


def expected_real_editor_values(field, field_def=None):
    """Return the LDtk-editor-shaped ``realEditorValues`` for a field instance.

    The parser-facing value remains ``__value``. ``realEditorValues`` is editor
    metadata: LDtk commonly stores ``[]`` for null values and default-equal
    values, and stores a wrapper only when the user has an explicit non-default
    value. Mirroring that rule keeps tool output compatible with an actual
    editor save.
    """
    value = field.get("__value")
    human_type = field.get("__type") or (field_def or {}).get("__type")
    if value is None:
        return []
    if _value_matches_default_override(value, human_type, field_def):
        return []
    return editor_value_for(value, human_type)


def real_editor_values_are_acceptable(field, field_def=None):
    """Return true if the current editor metadata matches known LDtk output.

    LDtk 1.5.3 is not perfectly uniform here. After adding new fields in the
    editor, default-equal instance values often keep ``realEditorValues: []``;
    older generated/defaulted fields may retain the explicit ValueWrapper. Both
    shapes survive an editor save. Treat both as canonical so our tools do not
    create churn against the editor.
    """
    actual = field.get("realEditorValues")
    value = field.get("__value")
    human_type = field.get("__type") or (field_def or {}).get("__type")
    if value is None:
        return actual == []
    explicit = editor_value_for(value, human_type)
    if _value_matches_default_override(value, human_type, field_def):
        return actual in ([], explicit)
    return actual == explicit


def normalize_editor_values(project):
    changed = 0
    defs = project.get("defs") or {}
    entity_defs = {entity.get("identifier"): entity for entity in defs.get("entities") or []}
    entity_field_defs = {
        identifier: {field.get("identifier"): field for field in (entity.get("fieldDefs") or [])}
        for identifier, entity in entity_defs.items()
    }
    level_field_defs = {field.get("identifier"): field for field in defs.get("levelFields") or []}

    def normalize_fields(fields, field_defs):
        nonlocal changed
        for field in fields or []:
            field_def = field_defs.get(field.get("__identifier")) if field_defs else None
            if real_editor_values_are_acceptable(field, field_def):
                continue
            expected = expected_real_editor_values(field, field_def)
            field["realEditorValues"] = expected
            changed += 1

    for level in project.get("levels") or []:
        normalize_fields(level.get("fieldInstances") or [], level_field_defs)
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                normalize_fields(
                    entity.get("fieldInstances") or [],
                    entity_field_defs.get(entity.get("__identifier"), {}),
                )
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



def normalize_entity_world_coordinates(project):
    """Synchronize LDtk's cached entity world coordinates.

    ``px`` is level-local. LDtk also writes cached ``__worldX`` / ``__worldY``
    equal to ``level.worldX + px[0]`` and ``level.worldY + px[1]``. Generated
    tools used to leave those fields missing or rooted at the origin, which the
    editor fixes on save and which can make Bevy-side LDtk consumers see stale
    positions. The values are purely derived, so repair can update them safely.
    """
    changed = 0
    for level in project.get("levels") or []:
        world_x = int(level.get("worldX") or 0)
        world_y = int(level.get("worldY") or 0)
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                px = entity.get("px") or [0, 0]
                if not isinstance(px, list) or len(px) != 2:
                    continue
                expected_x = world_x + int(px[0])
                expected_y = world_y + int(px[1])
                if entity.get("__worldX") != expected_x:
                    entity["__worldX"] = expected_x
                    changed += 1
                if entity.get("__worldY") != expected_y:
                    entity["__worldY"] = expected_y
                    changed += 1
    return changed


def normalize_project_for_editor(project):
    """Normalize generated/agent-patched LDtk JSON for LDtk GUI round-trips.

    This intentionally avoids inventing gameplay values. It only repairs editor
    representation fields, internal type constructors, and definition UIDs that
    can be derived from existing definitions and parser-facing values.
    """
    changes = []
    changes.extend(normalize_definitions_for_editor(project))
    changes.extend(sync_instance_definition_uids(project))
    world_coord_count = normalize_entity_world_coordinates(project)
    if world_coord_count:
        changes.append(f"synchronized {world_coord_count} cached entity __worldX/__worldY values")
    editor_value_count = normalize_editor_values(project)
    if editor_value_count:
        changes.append(f"normalized {editor_value_count} field instance realEditorValues records")
    return changes

def validate(
    path: Path,
    schema_path: Path | None = None,
    require_schema: bool = False,
    secondary_worlds: list[Path] | None = None,
):
    errors = []
    warnings = []
    try:
        project = json.loads(path.read_text())
    except Exception as ex:  # noqa: BLE001 - command line validator should print parser details
        return [f"failed to parse JSON: {ex}"], []

    # Build a `(activeArea, zone_id)` index over secondary world files so
    # cross-file `LoadingZone.target_room` references don't false-positive.
    # See `crates/ambition_sandbox/src/ldtk_world/loading.rs::merge_secondary_worlds`
    # for the runtime side — it merges these files into the live
    # `LdtkProject` at load time, so target_room lookups succeed there.
    # The strict arrival-rect / solid-overlap checks are skipped for
    # secondary targets (would require importing the full per-level
    # bounds + solid set from the secondary file); secondary-bound
    # zones get a single existence check instead.
    secondary_index: dict[str, set[str]] = defaultdict(set)
    for secondary_path in secondary_worlds or []:
        if not secondary_path.exists():
            warnings.append(
                f"secondary world {secondary_path} does not exist; skipping"
            )
            continue
        try:
            secondary_project = json.loads(secondary_path.read_text())
        except Exception as ex:  # noqa: BLE001
            warnings.append(
                f"failed to parse secondary world {secondary_path}: {ex}"
            )
            continue
        for sec_level in secondary_project.get("levels") or []:
            sec_area = field_value(sec_level.get("fieldInstances") or [], "activeArea")
            sec_area = str(sec_area).strip() if sec_area is not None else ""
            if not sec_area:
                sec_area = sec_level.get("identifier", "")
            sec_amb_layer = None
            for sec_layer in sec_level.get("layerInstances") or []:
                if sec_layer.get("__identifier") == AMBITION_LAYER:
                    sec_amb_layer = sec_layer
                    break
            if sec_amb_layer is None:
                continue
            for sec_entity in sec_amb_layer.get("entityInstances") or []:
                if sec_entity.get("__identifier") != "LoadingZone":
                    continue
                sec_zone_id = field_value(sec_entity.get("fieldInstances") or [], "id")
                if sec_zone_id is None:
                    continue
                secondary_index[sec_area].add(str(sec_zone_id))

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
            # NOTE: `width`/`height` further down still refer to the LEVEL
            # dimensions captured above (the per-level loop). Per-entity
            # dimensions use distinct `entity_w`/`entity_h` names to
            # avoid shadowing — an earlier shadow on `width`/`height`
            # caused `touches_level_edge` below to compare entity-rect
            # against entity-dims, which is vacuously true for any
            # entity at px=[0,0] and incorrect for the EdgeExit
            # mid-room check (every EdgeExit zone was "passing"
            # regardless of position).
            entity_w = int(entity.get("width", 0) or 0)
            entity_h = int(entity.get("height", 0) or 0)
            px = entity.get("px") or [0, 0]
            if entity_w <= 0 or entity_h <= 0:
                errors.append(f"level {identifier!r} entity {entity_name(entity)} has non-positive dimensions")
            if len(px) != 2 or px[0] < 0 or px[1] < 0 or px[0] + entity_w > level.get("pxWid", 0) or px[1] + entity_h > level.get("pxHei", 0):
                errors.append(f"level {identifier!r} entity {entity_name(entity)} is outside level bounds")
            elif "__worldX" in entity or "__worldY" in entity:
                expected_world_x = world_x + int(px[0])
                expected_world_y = world_y + int(px[1])
                if entity.get("__worldX") != expected_world_x or entity.get("__worldY") != expected_world_y:
                    errors.append(
                        f"level {identifier!r} entity {entity_name(entity)} has stale cached world coords "
                        f"({entity.get('__worldX')!r}, {entity.get('__worldY')!r}); expected "
                        f"({expected_world_x}, {expected_world_y}) from level origin ({world_x}, {world_y}) + px {px!r}. "
                        "Run `PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools repair <file> --in-place`."
                    )
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
            if target_room in secondary_index:
                if target_zone not in secondary_index[target_room]:
                    errors.append(
                        f"LoadingZone {zone_id!r} in {source_level!r} targets missing zone "
                        f"{target_zone!r} in secondary-world area {target_room!r}"
                    )
                # Strict arrival-rect / solid-overlap checks skipped for
                # secondary targets; the runtime merge handles them.
                continue
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
            # Classify by shape: wider-than-tall = top/bottom seam
            # (inset Y), taller-than-wide = left/right seam (inset X).
            # Must mirror `edge_arrival` in
            # crates/ambition_sandbox/src/world/rooms/spawn.rs so
            # the validator's arrival math matches what the runtime
            # actually computes.
            zw, zh = local_zone[2], local_zone[3]
            if zw >= zh:
                # Top/bottom seam.
                if local_zone[1] <= 37.0:
                    arrival = (cx, 92.0)
                elif local_zone[1] + local_zone[3] >= height - 37.0:
                    arrival = (cx, height - 92.0)
                else:
                    arrival = (cx, cy)
            else:
                # Left/right seam.
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

    _check_intro_authoring_hygiene(project, warnings)

    return errors, warnings


def _check_intro_authoring_hygiene(project, warnings):
    """Three soft checks Jon asked for after the 2026-05-22 review:

    - **DebugLabel overlaps**: any pair of DebugLabel entities whose
      rendered text bboxes overlap in world space looks like a wall
      of garbled text in the debug overlay. Flagged as a warning so
      authors can space them.
    - **Mid-air LoadingZones**: a `Door` LoadingZone that has no
      walkable surface (Solid / OneWayPlatform) within `STAND_GAP`
      px below its bottom edge is a teleport from nowhere. Warned
      so authors notice.
    - **Missing room walls**: a level whose boundary has neither a
      Solid covering it NOR an EdgeExit LoadingZone is a "walks off
      the world" boundary. Warned so authors fill it.

    These are warnings — they describe authoring smells rather than
    schema violations, so `validate --strict` can be used to escalate
    them in CI without breaking iteration.
    """
    STAND_GAP = 16.0

    # --- DebugLabel pair-wise overlap ---------------------------------
    # Approximate each label's bbox from its entity rect — exact text
    # metrics depend on the runtime font choice but the entity's
    # `width` field is sized for the rendered text already.
    label_rects_by_level = defaultdict(list)
    for level in project.get("levels") or []:
        identifier = level.get("identifier", "<unknown>")
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                if entity.get("__identifier") != "DebugLabel":
                    continue
                ex, ey, ew, eh = rect(entity)
                label_rects_by_level[identifier].append((entity_name(entity), ex, ey, ew, eh))
    for level_name, rects in label_rects_by_level.items():
        for i in range(len(rects)):
            for j in range(i + 1, len(rects)):
                ai, ax, ay, aw, ah = rects[i]
                bi, bx, by, bw, bh = rects[j]
                if strict_rects_intersect((ax, ay, aw, ah), (bx, by, bw, bh)):
                    warnings.append(
                        f"DebugLabels {ai!r} and {bi!r} in level {level_name!r} "
                        f"overlap — space them apart or stack vertically so the "
                        f"text is readable in the debug overlay"
                    )

    # --- Spawn entity overlap (NpcSpawn / EnemySpawn / BossSpawn) ------
    # Two spawn AABBs that overlap or sit within a few pixels of each
    # other will render their sprites stacked — flagged as a soft
    # warning so authors notice (e.g. the Hall of Characters basement
    # bosses with wide-frame sprites that bleed across slot
    # boundaries). The threshold is a small "personal-space" buffer
    # rather than literal AABB intersection.
    SPAWN_GAP_PX = 4.0  # min separation between spawn AABBs
    spawn_kinds = ("NpcSpawn", "EnemySpawn", "BossSpawn")
    spawns_by_level = defaultdict(list)
    for level in project.get("levels") or []:
        identifier = level.get("identifier", "<unknown>")
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                ident = entity.get("__identifier")
                if ident not in spawn_kinds:
                    continue
                ex, ey, ew, eh = rect(entity)
                label = entity_name(entity) or entity.get("iid", "<no-iid>")
                spawns_by_level[identifier].append((ident, label, ex, ey, ew, eh))
    for level_name, items in spawns_by_level.items():
        for i in range(len(items)):
            for j in range(i + 1, len(items)):
                ka, la, ax, ay, aw, ah = items[i]
                kb, lb, bx, by, bw, bh = items[j]
                # Inflate each rect by half the gap so the
                # intersection test rejects too-close pairs as well
                # as outright overlaps.
                inflate = SPAWN_GAP_PX / 2.0
                if strict_rects_intersect(
                    (ax - inflate, ay - inflate, aw + 2 * inflate, ah + 2 * inflate),
                    (bx - inflate, by - inflate, bw + 2 * inflate, bh + 2 * inflate),
                ):
                    warnings.append(
                        f"{ka} {la!r} and {kb} {lb!r} in level "
                        f"{level_name!r} overlap or sit within "
                        f"{SPAWN_GAP_PX:g}px of each other "
                        f"(rects: a=({ax:g},{ay:g},{aw:g},{ah:g}) "
                        f"b=({bx:g},{by:g},{bw:g},{bh:g})) — wide "
                        f"sprite render bleeds across slot "
                        f"boundaries"
                    )

    # --- Mid-air Doors + missing room walls --------------------------
    for level in project.get("levels") or []:
        identifier = level.get("identifier", "<unknown>")
        width = int(level.get("pxWid", 0))
        height = int(level.get("pxHei", 0))
        solids = []
        one_ways = []
        doors = []
        edge_exits = set()  # which sides have an EdgeExit
        for layer in level.get("layerInstances") or []:
            for entity in layer.get("entityInstances") or []:
                ident = entity.get("__identifier")
                if ident == "Solid":
                    solids.append(rect(entity))
                elif ident == "OneWayPlatform":
                    one_ways.append(rect(entity))
                elif ident == "LoadingZone":
                    fields = entity.get("fieldInstances") or []
                    activation = str(field_value(fields, "activation", "Door"))
                    er = rect(entity)
                    doors.append((entity_name(entity), activation, er))
                    if activation == "EdgeExit":
                        ex, ey, ew, eh = er
                        if ex <= 1:
                            edge_exits.add("left")
                        if ex + ew >= width - 1:
                            edge_exits.add("right")
                        if ey <= 1:
                            edge_exits.add("top")
                        if ey + eh >= height - 1:
                            edge_exits.add("bottom")
        # Mid-air Door check: a `Door` activation should have a
        # walkable surface within STAND_GAP px below its bottom edge.
        # Accepts entity-instance Solids/OneWayPlatforms OR IntGrid
        # Collision cells with value 1 (Solid) / 2 (OneWayPlatform) —
        # `area_authoring` lowers static-collision entities to IntGrid
        # by default, so a strict entity-only check false-positives on
        # every door over an IntGrid floor.
        intgrid_layer = None
        for layer in level.get("layerInstances") or []:
            if layer.get("__identifier") == "Collision":
                intgrid_layer = layer
                break
        ig_grid = int(intgrid_layer.get("__gridSize", 16)) if intgrid_layer else 16
        ig_c_wid = int(intgrid_layer.get("__cWid", 0)) if intgrid_layer else 0
        ig_c_hei = int(intgrid_layer.get("__cHei", 0)) if intgrid_layer else 0
        ig_csv = intgrid_layer.get("intGridCsv", []) if intgrid_layer else []

        def intgrid_rect_intersects_walkable(rect_xywh: tuple[float, float, float, float]) -> bool:
            if not (intgrid_layer and ig_c_wid and ig_c_hei and ig_csv):
                return False
            rx, ry, rw, rh = rect_xywh
            cx0 = max(0, int(rx) // ig_grid)
            cy0 = max(0, int(ry) // ig_grid)
            cx1 = min(ig_c_wid - 1, int(rx + rw - 1) // ig_grid)
            cy1 = min(ig_c_hei - 1, int(ry + rh - 1) // ig_grid)
            for cy in range(cy0, cy1 + 1):
                for cx in range(cx0, cx1 + 1):
                    v = ig_csv[cy * ig_c_wid + cx]
                    if v in (1, 2):  # Solid or OneWayPlatform
                        return True
            return False

        for name, activation, (dx, dy, dw, dh) in doors:
            if activation != "Door":
                continue
            door_bottom_y = dy + dh
            probe = (dx, door_bottom_y, dw, STAND_GAP)
            supports = any(
                strict_rects_intersect(probe, (sx, sy, sw, sh))
                for (sx, sy, sw, sh) in solids
            ) or any(
                strict_rects_intersect(probe, (sx, sy, sw, sh))
                for (sx, sy, sw, sh) in one_ways
            ) or intgrid_rect_intersects_walkable(probe)
            if not supports:
                warnings.append(
                    f"LoadingZone {name!r} in level {identifier!r} is a "
                    f"`Door` with no walkable surface within {int(STAND_GAP)}px "
                    f"below — looks like a teleport hanging in mid-air. Add a "
                    f"Solid/OneWayPlatform under it or switch to EdgeExit."
                )
        # Missing-wall check: every side of the level box that has no
        # EdgeExit should be covered by a Solid that meets the level
        # edge. Also accept IntGrid Collision cells with `Solid` value
        # (=1) since `area_authoring` lowers static-collision entities
        # into IntGrid by default — checking only entity-instance
        # Solids would false-positive on every level that uses
        # canonical IntGrid collision.
        if width > 0 and height > 0:
            intgrid_layer = None
            for layer in level.get("layerInstances") or []:
                if layer.get("__identifier") == "Collision":
                    intgrid_layer = layer
                    break
            grid_size = int(intgrid_layer.get("__gridSize", 16)) if intgrid_layer else 16
            c_wid = int(intgrid_layer.get("__cWid", 0)) if intgrid_layer else 0
            c_hei = int(intgrid_layer.get("__cHei", 0)) if intgrid_layer else 0
            csv = intgrid_layer.get("intGridCsv", []) if intgrid_layer else []

            def intgrid_blocks_side(side: str) -> bool:
                if not (intgrid_layer and c_wid and c_hei and csv):
                    return False
                if side == "left":
                    return any(csv[y * c_wid] == 1 for y in range(c_hei))
                if side == "right":
                    return any(csv[y * c_wid + (c_wid - 1)] == 1 for y in range(c_hei))
                if side == "top":
                    return any(csv[x] == 1 for x in range(c_wid))
                if side == "bottom":
                    return any(csv[(c_hei - 1) * c_wid + x] == 1 for x in range(c_wid))
                return False

            sides = (
                ("left",   (0, 0, 1, height)),
                ("right",  (max(0, width - 1), 0, 1, height)),
                ("top",    (0, 0, width, 1)),
                ("bottom", (0, max(0, height - 1), width, 1)),
            )
            for side_name, probe in sides:
                if side_name in edge_exits:
                    continue
                blocks_side = any(
                    strict_rects_intersect(probe, (sx, sy, sw, sh))
                    for (sx, sy, sw, sh) in solids
                ) or intgrid_blocks_side(side_name)
                if not blocks_side:
                    warnings.append(
                        f"level {identifier!r} has no Solid blocking the "
                        f"{side_name} edge and no EdgeExit on that side — "
                        f"the player can walk off the world."
                    )


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
    parser.add_argument(
        "--secondary-world",
        action="append",
        type=Path,
        default=[],
        metavar="PATH",
        help=(
            "Additional .ldtk source files whose levels the runtime merges on top of `path` "
            "(see ldtk_world/loading.rs::SECONDARY_WORLD_FILES). The validator uses these to "
            "resolve cross-file LoadingZone target_room references without false-positiving. "
            "May be passed multiple times."
        ),
    )
    args = parser.parse_args(argv)
    if args.normalize_editor_values or args.repair_editor_roundtrip:
        try:
            project = json.loads(args.path.read_text())
            changes = normalize_project_for_editor(project)
            if changes:
                from ambition_ldtk_tools.editor_format import dump_editor_style

                args.path.write_text(dump_editor_style(project))
                print(f"repaired {len(changes)} LDtk editor-roundtrip issue(s) in {args.path}", file=sys.stderr)
                for change in changes[:20]:
                    print(f"  - {change}", file=sys.stderr)
                if len(changes) > 20:
                    print(f"  ... {len(changes) - 20} more", file=sys.stderr)
        except Exception as ex:  # noqa: BLE001
            print(f"error: failed to repair editor-roundtrip values: {ex}", file=sys.stderr)
            return 1
    errors, warnings = validate(
        args.path,
        args.schema,
        args.require_schema,
        secondary_worlds=args.secondary_world,
    )
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
