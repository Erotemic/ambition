"""Tests for `mount split` — fused composite → linked mount+rider (ADR 0020)."""

from __future__ import annotations

from ambition_ldtk_tools.mount_split import split_composites


def _string_field(identifier: str, uid: int, value):
    return {
        "__identifier": identifier,
        "__type": "String",
        "__value": value,
        "__tile": None,
        "defUid": uid,
        "realEditorValues": [{"id": "V_String", "params": [value]}],
    }


def _project(brain: str, name: str) -> dict:
    """A minimal project with one fused composite EnemySpawn."""
    return {
        "iid": "test-world",
        "nextUid": 9000,
        "defs": {
            "entities": [
                {
                    "identifier": "EnemySpawn",
                    "uid": 2008,
                    "color": "#FF0000",
                    "fieldDefs": [
                        {"identifier": "name", "uid": 3018, "__type": "String"},
                        {"identifier": "brain", "uid": 3019, "__type": "String"},
                    ],
                }
            ]
        },
        "levels": [
            {
                "iid": "lvl-1",
                "identifier": "cove",
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "iid": "layer-1",
                        "entityInstances": [
                            {
                                "__identifier": "EnemySpawn",
                                "iid": "EnemySpawn-1",
                                "px": [100, 200],
                                "__grid": [6, 12],
                                "width": 108,
                                "height": 96,
                                "defUid": 2008,
                                "fieldInstances": [
                                    _string_field("name", 3018, name),
                                    _string_field("brain", 3019, brain),
                                ],
                            }
                        ],
                    }
                ],
            }
        ],
    }


def _field(entity, name):
    return next(f for f in entity["fieldInstances"] if f["__identifier"] == name)


def test_split_produces_linked_mount_and_rider():
    project = _project("pirate_on_shark", "Burning Flying Shark")
    changes = split_composites(project)
    assert len(changes) == 1

    # The EnemySpawn def gained a mounted_on EntityRef field.
    es_def = project["defs"]["entities"][0]
    mounted_on = next(f for f in es_def["fieldDefs"] if f["identifier"] == "mounted_on")
    assert mounted_on["__type"] == "EntityRef"

    ents = project["levels"][0]["layerInstances"][0]["entityInstances"]
    assert len(ents) == 2, "one rider (rewritten) + one new mount"
    by_brain = {_field(e, "brain")["__value"]: e for e in ents}

    rider = by_brain["pirate_shark_rider"]
    mount = by_brain["burning_flying_shark"]

    # Rider: proper rider name (not the mount's), linked to the mount's iid.
    assert _field(rider, "name")["__value"] == "Pirate Raider"
    ref = _field(rider, "mounted_on")["__value"]
    assert ref["entityIid"] == mount["iid"]
    assert ref["levelIid"] == "lvl-1" and ref["layerIid"] == "layer-1"
    # Neither entity carries an explicit `id` field (FeatureId == iid).
    assert not any(f["__identifier"] == "id" for f in rider["fieldInstances"])
    assert not any(f["__identifier"] == "id" for f in mount["fieldInstances"])
    # Mount keeps the shark name + sits at the rider's position.
    assert _field(mount, "name")["__value"] == "Burning Flying Shark"
    assert mount["px"] == [100, 200]


def test_heavy_rider_name_strips_on_shark_suffix():
    project = _project("pirate_heavy_on_shark", "Iron Mary on Shark")
    split_composites(project)
    ents = project["levels"][0]["layerInstances"][0]["entityInstances"]
    rider = next(e for e in ents if _field(e, "brain")["__value"] == "pirate_heavy_shark_rider")
    assert _field(rider, "name")["__value"] == "Iron Mary"


def test_non_composite_spawns_are_untouched():
    project = _project("pirate_raider", "Deck Swabber")
    changes = split_composites(project)
    assert changes == []
    ents = project["levels"][0]["layerInstances"][0]["entityInstances"]
    assert len(ents) == 1
    assert _field(ents[0], "brain")["__value"] == "pirate_raider"
