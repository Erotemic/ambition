from __future__ import annotations

from ambition_ldtk_tools.edit.entity_layer_rules import (
    apply_editor_layer_rule,
    change_layer,
    collect_rule_violations,
)


def entity(identifier: str, iid: str, px=(0, 0)) -> dict:
    return {
        "__identifier": identifier,
        "iid": iid,
        "px": [px[0], px[1]],
        "width": 64,
        "height": 64,
        "fieldInstances": [],
    }


def layer(identifier: str, uid: int, entities: list[dict] | None = None) -> dict:
    return {
        "__identifier": identifier,
        "__type": "Entities",
        "layerDefUid": uid,
        "iid": f"{identifier}-inst",
        "entityInstances": list(entities or []),
    }


def mini_project() -> dict:
    return {
        "nextUid": 100,
        "defs": {
            "layers": [
                {
                    "__type": "Entities",
                    "type": "Entities",
                    "identifier": "Ambition",
                    "uid": 1,
                    "requiredTags": [],
                    "excludedTags": [],
                },
                {
                    "__type": "Entities",
                    "type": "Entities",
                    "identifier": "AmbitionCameras",
                    "uid": 2,
                    "requiredTags": [],
                    "excludedTags": [],
                },
            ],
            "entities": [
                {"identifier": "CameraZone", "uid": 10, "tags": []},
                {"identifier": "LoadingZone", "uid": 11, "tags": []},
            ],
        },
        "levels": [
            {
                "identifier": "symmetry_room",
                "layerInstances": [
                    layer("Ambition", 1, [entity("CameraZone", "CameraZone-1"), entity("LoadingZone", "LoadingZone-1")]),
                    layer("AmbitionCameras", 2, []),
                ],
            }
        ],
    }


def find_layer(level: dict, identifier: str) -> dict:
    return next(layer for layer in level["layerInstances"] if layer["__identifier"] == identifier)


def test_change_layer_moves_selected_entity_only() -> None:
    project = mini_project()
    moved = change_layer(
        project,
        level_filter="symmetry_room",
        from_layer="Ambition",
        to_layer="AmbitionCameras",
        iid=None,
        identifier="CameraZone",
        field_filters=[],
    )
    assert len(moved) == 1
    level = project["levels"][0]
    ambition = find_layer(level, "Ambition")
    cameras = find_layer(level, "AmbitionCameras")
    assert [e["__identifier"] for e in ambition["entityInstances"]] == ["LoadingZone"]
    assert [e["iid"] for e in cameras["entityInstances"]] == ["CameraZone-1"]


def test_check_entity_rules_reports_wrong_layer() -> None:
    project = mini_project()
    violations = collect_rule_violations(project, {"CameraZone": "AmbitionCameras"})
    assert len(violations) == 1
    assert violations[0].level == "symmetry_room"
    assert violations[0].layer == "Ambition"


def test_apply_editor_layer_rule_uses_ldtk_tags() -> None:
    project = mini_project()
    changes = apply_editor_layer_rule(
        project,
        entity_type="CameraZone",
        to_layer="AmbitionCameras",
        from_layer="Ambition",
        tag="Camera",
    )
    assert changes
    entities = project["defs"]["entities"]
    cam = next(entity for entity in entities if entity["identifier"] == "CameraZone")
    assert "Camera" in cam["tags"]
    ambition = next(layer for layer in project["defs"]["layers"] if layer["identifier"] == "Ambition")
    cameras = next(layer for layer in project["defs"]["layers"] if layer["identifier"] == "AmbitionCameras")
    assert "Camera" in ambition["excludedTags"]
    assert "Camera" in cameras["requiredTags"]
