#!/usr/bin/env python3
"""Unit test for area-authoring EntityRef mount-link resolution (ADR 0020 / G4).

`build_level` supports a spec-local `ref` handle on a mount entity plus a
`mounted_on` EntityRef field on the rider that names it. After every entity has
an iid, the handle is resolved into LDtk's canonical
`{entityIid, layerIid, levelIid, worldIid}` object, and the target entity def
gains the `mounted_on` field def on demand. This pins that the GNU-ton pair
(a `gnu_ton_rider` BossSpawn → its `giant_gnu` EnemySpawn mount) resolves.
"""

from __future__ import annotations

from pathlib import Path

from ambition_ldtk_tools.area_authoring import build_level, load_project

REPO_ROOT = Path(__file__).resolve().parents[3]
LDTK_PATH = REPO_ROOT / "crates" / "ambition_content" / "assets" / "worlds" / "sandbox.ldtk"


def _fields(entity: dict) -> dict:
    return {f["__identifier"]: f.get("__value") for f in entity.get("fieldInstances", [])}


def test_build_level_resolves_boss_mounted_on_ref_handle():
    project = load_project(LDTK_PATH)
    spec = {
        "id": "gnu_ton_ref_probe",
        "level_id": "gnu_ton_ref_probe",
        # A free world slot far from every live level (positive-x fixture space).
        "world_x": 60000,
        "world_y": 0,
        "px_wid": 512,
        "px_hei": 512,
        "fill_collision": "empty",
        "entities": [
            {
                "type": "EnemySpawn",
                "ref": "giant_gnu_mount",
                "px": [100, 100],
                "size": [220, 220],
                "fields": {"name": "Giant GNU", "brain": "giant_gnu"},
            },
            {
                "type": "BossSpawn",
                "px": [120, 60],
                "size": [54, 96],
                "fields": {
                    "name": "GNU-ton",
                    "brain": "PhaseScript:gnu_ton_rider",
                    "mounted_on": "giant_gnu_mount",
                },
            },
        ],
    }

    level = build_level(project, spec)
    ambition = next(
        layer
        for layer in level["layerInstances"]
        if layer["__identifier"] == "Ambition"
    )
    ents = {e["__identifier"]: e for e in ambition["entityInstances"]}
    mount, rider = ents["EnemySpawn"], ents["BossSpawn"]

    # The rider's mounted_on resolved from the handle into a real EntityRef
    # pointing at the mount's iid (which has no explicit id → FeatureId == iid).
    ref = _fields(rider)["mounted_on"]
    assert isinstance(ref, dict), "mounted_on must resolve to an EntityRef object"
    assert ref["entityIid"] == mount["iid"]
    assert ref["layerIid"] == ambition["iid"]
    assert ref["levelIid"] == level["iid"]
    assert ref["worldIid"] == project.get("iid")

    # The BossSpawn entity def gained the mounted_on field def on demand,
    # with a cross-type "Any" allowedRefs (rider BossSpawn → mount EnemySpawn).
    boss_def = next(
        d for d in project["defs"]["entities"] if d["identifier"] == "BossSpawn"
    )
    mounted_on_def = next(
        f for f in boss_def["fieldDefs"] if f["identifier"] == "mounted_on"
    )
    assert mounted_on_def["__type"] == "EntityRef"
    assert mounted_on_def["allowedRefs"] == "Any"


def test_build_level_unknown_ref_handle_fails_loudly():
    import pytest

    project = load_project(LDTK_PATH)
    spec = {
        "id": "gnu_ton_ref_probe_bad",
        "level_id": "gnu_ton_ref_probe_bad",
        "world_x": 60000,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 256,
        "fill_collision": "empty",
        "entities": [
            {
                "type": "BossSpawn",
                "px": [10, 10],
                "size": [54, 96],
                "fields": {
                    "name": "GNU-ton",
                    "brain": "PhaseScript:gnu_ton_rider",
                    "mounted_on": "does_not_exist",
                },
            },
        ],
    }
    with pytest.raises(SystemExit):
        build_level(project, spec)


if __name__ == "__main__":
    test_build_level_resolves_boss_mounted_on_ref_handle()
    test_build_level_unknown_ref_handle_fails_loudly()
    print("ok")
