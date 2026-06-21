from __future__ import annotations

import json

from ambition_ldtk_tools.ldtk import (
    ApplyEntityLayerTagRule,
    LdtkTransaction,
    MoveEntitiesToLayer,
)


def mini_project() -> dict:
    return {
        "nextUid": 100,
        "defs": {
            "layers": [
                {"__type": "Entities", "type": "Entities", "identifier": "Ambition", "uid": 1, "requiredTags": [], "excludedTags": []},
                {"__type": "Entities", "type": "Entities", "identifier": "AmbitionCameras", "uid": 2, "requiredTags": [], "excludedTags": []},
            ],
            "entities": [{"identifier": "CameraZone", "uid": 10, "tags": []}],
            "tilesets": [],
        },
        "levels": [
            {
                "identifier": "room",
                "worldX": 0,
                "worldY": 0,
                "pxWid": 128,
                "pxHei": 128,
                "layerInstances": [
                    {
                        "__identifier": "Ambition",
                        "__type": "Entities",
                        "layerDefUid": 1,
                        "iid": "Ambition-inst",
                        "entityInstances": [
                            {"__identifier": "CameraZone", "iid": "CameraZone-1", "fieldInstances": []}
                        ],
                    },
                    {
                        "__identifier": "AmbitionCameras",
                        "__type": "Entities",
                        "layerDefUid": 2,
                        "iid": "AmbitionCameras-inst",
                        "entityInstances": [],
                    },
                ],
            }
        ],
    }


def write_json(path, project: dict) -> None:
    path.write_text(json.dumps(project, indent=2) + "\n")


def test_transaction_noop_leaves_file_unchanged(tmp_path) -> None:
    path = tmp_path / "mini.ldtk"
    write_json(path, mini_project())
    before = path.read_text()

    tx = LdtkTransaction(path, in_place=True)
    result = tx.apply(MoveEntitiesToLayer(to_layer="AmbitionCameras", from_layer="Ambition", identifier="Missing"))
    assert not result.changed
    assert tx.finish(noop_message="noop") is None
    assert path.read_text() == before


def test_transaction_writes_patch_once(tmp_path) -> None:
    path = tmp_path / "mini.ldtk"
    write_json(path, mini_project())

    tx = LdtkTransaction(path, in_place=True)
    result = tx.apply(MoveEntitiesToLayer(to_layer="AmbitionCameras", from_layer="Ambition", identifier="CameraZone"))
    assert result.changed
    assert len(result.messages) == 1
    target = tx.finish()
    assert target == path

    after = json.loads(path.read_text())
    layers = after["levels"][0]["layerInstances"]
    by_name = {layer["__identifier"]: layer for layer in layers}
    assert by_name["Ambition"]["entityInstances"] == []
    assert by_name["AmbitionCameras"]["entityInstances"][0]["iid"] == "CameraZone-1"


def test_patch_rule_op_is_idempotent() -> None:
    project = mini_project()
    op = ApplyEntityLayerTagRule(entity_type="CameraZone", to_layer="AmbitionCameras", from_layer="Ambition", tag="Camera")
    first = op.apply(project)
    second = op.apply(project)
    assert first.changed
    assert first.messages
    assert not second.changed
    assert not second.messages
