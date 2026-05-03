#!/usr/bin/env python3
"""One-shot migration: rewrite a few legacy LDtk identifiers as Surface."""
from __future__ import annotations

import json
from pathlib import Path

PATH = Path("/home/joncrall/code/ambition/crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk")

# (iid, expected legacy identifier, surface field overrides)
MIGRATIONS = [
    ("Solid-0002", "Solid", {"collision": "Solid"}),
    ("OneWayPlatform-0011", "OneWayPlatform", {"collision": "OneWayUp"}),
    ("5e418960-21a0-11f1-a1bc-8b5f68aa6b42", "Breakable", {
        "collision": "Solid",
        "breakability": "BreakOnStand",
        "respawn": "OnRoomReload",
        "hp": 3,
    }),
    # First HazardBlock by iid; we look up the first HazardBlock to migrate.
    ("__first_hazard__", "HazardBlock", {"contact": "Damage", "damage": 1}),
]

SURFACE_DEF_UID = 4020
FIELD_DEF_UIDS = {
    "name": 4025,
    "collision": 4021,
    "breakability": 4022,
    "contact": 4023,
    "respawn": 4024,
    "hp": 4026,
    "damage": 4027,
    "rebound_x": 4028,
    "rebound_y": 4029,
    "respawn_seconds": 4030,
}
ENUM_TYPES = {
    "collision": "LocalEnum.SurfaceCollision",
    "breakability": "LocalEnum.SurfaceBreakability",
    "contact": "LocalEnum.SurfaceContact",
    "respawn": "LocalEnum.SurfaceRespawn",
}


def make_field_instances(name: str, overrides: dict) -> list[dict]:
    fields = []
    fields.append({
        "__identifier": "name",
        "__type": "String",
        "__value": name,
        "__tile": None,
        "defUid": FIELD_DEF_UIDS["name"],
        "realEditorValues": [{"id": "V_String", "params": [name]}],
    })
    for key in ("collision", "breakability", "contact", "respawn"):
        value = overrides.get(key, {
            "collision": "None",
            "breakability": "Indestructible",
            "contact": "None",
            "respawn": "Never",
        }[key])
        fields.append({
            "__identifier": key,
            "__type": ENUM_TYPES[key],
            "__value": value,
            "__tile": None,
            "defUid": FIELD_DEF_UIDS[key],
            "realEditorValues": [{"id": "V_String", "params": [value]}],
        })
    for key in ("hp", "damage", "rebound_x", "rebound_y", "respawn_seconds"):
        value = float(overrides.get(key, 0))
        # Match existing LDtk emit style: integer-valued floats serialize as
        # 0 not 0.0, so keep them as ints when they are whole numbers.
        emit = int(value) if value == int(value) else value
        fields.append({
            "__identifier": key,
            "__type": "Float",
            "__value": emit,
            "__tile": None,
            "defUid": FIELD_DEF_UIDS[key],
            "realEditorValues": [{"id": "V_Float", "params": [emit]}],
        })
    return fields


def find_entity(project: dict, iid: str, identifier: str) -> dict | None:
    for level in project.get("levels", []):
        for layer in level.get("layerInstances", []) or []:
            for entity in layer.get("entityInstances", []) or []:
                if iid == "__first_hazard__":
                    if entity.get("__identifier") == identifier:
                        return entity
                elif entity.get("iid") == iid and entity.get("__identifier") == identifier:
                    return entity
    return None


def migrate_entity(entity: dict, overrides: dict) -> None:
    name = next(
        (
            f.get("__value")
            for f in entity.get("fieldInstances", [])
            if f.get("__identifier") == "name" and isinstance(f.get("__value"), str)
        ),
        entity.get("__identifier", "Surface"),
    )
    entity["__identifier"] = "Surface"
    entity["__smartColor"] = "#7C3AED"
    entity["defUid"] = SURFACE_DEF_UID
    entity["fieldInstances"] = make_field_instances(str(name), overrides)


def main() -> int:
    project = json.loads(PATH.read_text())
    migrated = []
    for iid, identifier, overrides in MIGRATIONS:
        entity = find_entity(project, iid, identifier)
        if entity is None:
            print(f"could not find {identifier} {iid}; skipping")
            continue
        migrate_entity(entity, overrides)
        migrated.append((iid, identifier, entity.get("iid")))
    PATH.write_text(json.dumps(project, indent="\t") + "\n")
    print(f"migrated {len(migrated)} entities to Surface")
    for iid, ident, real_iid in migrated:
        print(f"  - {ident} {iid} ({real_iid}) -> Surface")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
