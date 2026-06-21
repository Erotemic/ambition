from __future__ import annotations

from pathlib import Path

from ambition_ldtk_tools.ldtk import (
    alloc_uid,
    entity_field_value,
    ensure_entities_layer_def,
    ensure_entities_layer_instance,
    find_layer_def,
    find_layer_instance,
    iter_entities,
    rel_to_ldtk,
)


def mini_project() -> dict:
    return {
        "nextUid": 100,
        "defs": {
            "layers": [
                {"identifier": "Ambition", "uid": 1, "__type": "Entities", "type": "Entities", "requiredTags": [], "excludedTags": []},
            ],
            "entities": [],
            "tilesets": [],
        },
        "levels": [
            {
                "identifier": "room",
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "__type": "Entities",
                        "layerDefUid": 1,
                        "iid": "Ambition-inst",
                        "entityInstances": [
                            {
                                "__identifier": "LoadingZone",
                                "iid": "LoadingZone-1",
                                "fieldInstances": [{"__identifier": "target_room", "__value": "next_room"}],
                            }
                        ],
                    }
                ],
            }
        ],
    }


def test_alloc_uid_and_shared_layer_creation() -> None:
    project = mini_project()
    assert alloc_uid(project) == 100
    assert project["nextUid"] == 101

    dest_def = ensure_entities_layer_def(project, "AmbitionCameras", clone_from="Ambition")
    assert dest_def["identifier"] == "AmbitionCameras"
    assert find_layer_def(project, "AmbitionCameras") is dest_def

    level = project["levels"][0]
    dest_layer = ensure_entities_layer_instance(
        project,
        level,
        "AmbitionCameras",
        dest_def=dest_def,
        clone_from="Ambition",
    )
    assert dest_layer["__identifier"] == "AmbitionCameras"
    assert find_layer_instance(level, "AmbitionCameras") is dest_layer


def test_iter_entities_and_field_lookup() -> None:
    project = mini_project()
    rows = list(iter_entities(project))
    assert len(rows) == 1
    assert rows[0].level["identifier"] == "room"
    assert entity_field_value(rows[0].entity, "target_room") == "next_room"


def test_rel_to_ldtk_uses_forward_slashes(tmp_path: Path) -> None:
    ldtk = tmp_path / "worlds" / "sandbox.ldtk"
    asset = tmp_path / "sprites" / "editor_icons.png"
    ldtk.parent.mkdir(parents=True)
    asset.parent.mkdir(parents=True)
    ldtk.write_text("{}")
    asset.write_bytes(b"")
    assert rel_to_ldtk(ldtk, asset) == "../sprites/editor_icons.png"
