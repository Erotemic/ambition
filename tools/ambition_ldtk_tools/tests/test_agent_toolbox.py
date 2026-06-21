from __future__ import annotations

import json
from pathlib import Path

from ambition_ldtk_tools.edit.assets import asset_catalog, link_entity_tile
from ambition_ldtk_tools.edit.camera import autocover_camera, collect_camera_issues
from ambition_ldtk_tools.edit.policy import collect_policy_issues, fix_policy
from ambition_ldtk_tools.edit.room_spec import compile_spec
from ambition_ldtk_tools.edit.semantic_diff import semantic_changes


def entity_def(identifier: str, uid: int, fields: list[dict] | None = None) -> dict:
    return {
        "identifier": identifier,
        "uid": uid,
        "tags": [],
        "width": 32,
        "height": 32,
        "color": "#ffffff",
        "fieldDefs": fields or [],
        "tilesetId": None,
        "tileRect": None,
        "uiTileRect": None,
        "renderMode": "Rectangle",
    }


def layer_def(identifier: str, uid: int, typ: str = "Entities") -> dict:
    return {
        "identifier": identifier,
        "uid": uid,
        "__type": typ,
        "type": typ,
        "requiredTags": [],
        "excludedTags": [],
        "gridSize": 16,
    }


def entity(identifier: str, iid: str, px=(0, 0), size=(32, 32)) -> dict:
    return {
        "__identifier": identifier,
        "iid": iid,
        "px": [px[0], px[1]],
        "width": size[0],
        "height": size[1],
        "fieldInstances": [],
    }


def entity_layer(identifier: str, uid: int, entities: list[dict] | None = None) -> dict:
    return {
        "__identifier": identifier,
        "__type": "Entities",
        "layerDefUid": uid,
        "iid": f"{identifier}-inst",
        "entityInstances": list(entities or []),
    }


def intgrid_layer(identifier: str = "Collision") -> dict:
    csv = [0] * (8 * 8)
    # small platform bbox in the middle
    for i in range(2, 6):
        csv[5 * 8 + i] = 1
    return {
        "__identifier": identifier,
        "__type": "IntGrid",
        "__gridSize": 16,
        "__cWid": 8,
        "__cHei": 8,
        "intGridCsv": csv,
    }


def mini_project() -> dict:
    return {
        "nextUid": 100,
        "defs": {
            "layers": [
                layer_def("Collision", 1, "IntGrid"),
                layer_def("Ambition", 2),
                layer_def("AmbitionCameras", 3),
            ],
            "entities": [
                entity_def("CameraZone", 10, [
                    {"identifier": "id", "__type": "String", "uid": 11, "canBeNull": False, "defaultOverride": {"params": [""]}},
                    {"identifier": "name", "__type": "String", "uid": 12, "canBeNull": False, "defaultOverride": {"params": ["Camera Zone"]}},
                ]),
                entity_def("LoadingZone", 20),
                entity_def("PlayerStart", 21),
            ],
            "tilesets": [
                {
                    "identifier": "test_tiles",
                    "uid": 50,
                    "relPath": "../sprites/test_tiles.png",
                    "tileGridSize": 16,
                    "pxWid": 32,
                    "pxHei": 32,
                }
            ],
        },
        "levels": [
            {
                "identifier": "room_a",
                "uid": 200,
                "worldX": 0,
                "worldY": 0,
                "pxWid": 128,
                "pxHei": 128,
                "fieldInstances": [],
                "layerInstances": [
                    intgrid_layer(),
                    entity_layer("Ambition", 2, [entity("CameraZone", "CameraZone-1")]),
                    entity_layer("AmbitionCameras", 3, []),
                ],
            }
        ],
    }


def test_semantic_diff_reports_entity_layer_and_level_move() -> None:
    before = mini_project()
    after = mini_project()
    after["levels"][0]["worldX"] = 256
    cam = after["levels"][0]["layerInstances"][1]["entityInstances"].pop()
    after["levels"][0]["layerInstances"][2]["entityInstances"].append(cam)
    kinds = {c.kind for c in semantic_changes(before, after)}
    assert "level_moved" in kinds
    assert "entity_layer" in kinds


def test_policy_fix_moves_camera_zones() -> None:
    project = mini_project()
    issues = collect_policy_issues(project, {"CameraZone": "AmbitionCameras"})
    assert any(i.code == "entity_wrong_layer" for i in issues)
    moved = fix_policy(project, {"CameraZone": "AmbitionCameras"})
    assert moved == 1
    assert not [i for i in collect_policy_issues(project, {"CameraZone": "AmbitionCameras"}) if i.severity == "error"]


def test_camera_autocover_updates_or_creates_camera_zone() -> None:
    project = mini_project()
    msg = autocover_camera(project, "room_a", margin=16, create=True)
    assert "updated CameraZone" in msg
    issues = collect_camera_issues(project, "room_a", margin=16)
    assert not [i for i in issues if i.severity == "error"]


def test_asset_catalog_and_entity_tile_link(tmp_path: Path) -> None:
    project = mini_project()
    # Minimal PNG header for dimension scan.
    png = tmp_path / "sprite.png"
    png.write_bytes(b"\x89PNG\r\n\x1a\n" + b"\x00\x00\x00\rIHDR" + (16).to_bytes(4, "big") + (16).to_bytes(4, "big") + b"\x08\x06\x00\x00\x00")
    ldtk = tmp_path / "world.ldtk"
    ldtk.write_text(json.dumps(project))
    cat = asset_catalog(project, ldtk, tmp_path)
    assert cat["pngs"]
    msg = link_entity_tile(project, "PlayerStart", "test_tiles", (0, 0, 16, 16))
    assert "PlayerStart" in msg
    ent = next(e for e in project["defs"]["entities"] if e["identifier"] == "PlayerStart")
    assert ent["renderMode"] == "Tile"
    assert ent["tileRect"]["w"] == 16


def test_room_spec_compile_paints_and_adds_entity() -> None:
    project = mini_project()
    report = compile_spec(
        project,
        {
            "level": "room_a",
            "intgrid": [{"layer": "Collision", "rect": [0, 0, 32, 32], "value": "solid"}],
            "entities": [{"type": "PlayerStart", "rect": [16, 16, 32, 32], "fields": {}}],
            "camera": {"create": True, "margin": 0},
        },
    )
    assert any("paint Collision" in line for line in report)
    ambition = next(l for l in project["levels"][0]["layerInstances"] if l["__identifier"] == "Ambition")
    assert any(e["__identifier"] == "PlayerStart" for e in ambition["entityInstances"])
