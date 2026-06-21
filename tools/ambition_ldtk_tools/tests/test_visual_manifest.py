from __future__ import annotations

import json
from pathlib import Path

from ambition_ldtk_tools.edit.policy import collect_policy_issues
from ambition_ldtk_tools.edit.semantic_diff import semantic_changes
from ambition_ldtk_tools.edit.visual_manifest import (
    apply_manifest,
    default_icon_manifest,
    generate_editor_icons,
    preview_manifest_html,
    validate_manifest,
)


def entity_def(identifier: str, uid: int) -> dict:
    return {
        "identifier": identifier,
        "uid": uid,
        "tags": [],
        "width": 32,
        "height": 32,
        "color": "#ffffff",
        "fieldDefs": [],
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


def mini_project() -> dict:
    return {
        "nextUid": 100,
        "defs": {
            "layers": [layer_def("Ambition", 1), layer_def("AmbitionCameras", 2)],
            "entities": [
                entity_def("CameraZone", 10),
                entity_def("LoadingZone", 11),
                entity_def("PlayerStart", 12),
            ],
            "tilesets": [],
        },
        "levels": [],
    }


def test_generate_icons_suggest_apply_validate_and_diff(tmp_path: Path) -> None:
    project = mini_project()
    ldtk = tmp_path / "world.ldtk"
    ldtk.write_text(json.dumps(project))
    icons = tmp_path / "editor_icons.png"
    info = generate_editor_icons(icons, tile_size=32, entities=["CameraZone", "LoadingZone", "PlayerStart"])
    assert info["size"] == [256, 32]
    manifest = default_icon_manifest(ldtk, icons, 32, ["CameraZone", "LoadingZone", "PlayerStart"])

    before = json.loads(json.dumps(project))
    messages = apply_manifest(project, ldtk, manifest)
    assert any("added tileset EditorIcons" in msg for msg in messages)
    assert any("linked CameraZone" in msg for msg in messages)

    issues = validate_manifest(project, ldtk, manifest)
    assert not [i for i in issues if i.severity == "error"]
    kinds = {c.kind for c in semantic_changes(before, project)}
    assert "tileset" in kinds
    assert "entity_def_visual" in kinds


def test_policy_reports_stale_visual_refs() -> None:
    project = mini_project()
    project["defs"]["entities"][0]["tilesetId"] = 999
    project["defs"]["entities"][0]["tileRect"] = {"tilesetUid": 999, "x": 0, "y": 0, "w": 32, "h": 32}
    issues = collect_policy_issues(project, {"CameraZone": "AmbitionCameras"})
    assert any(i.code == "stale_entity_tileset_uid" for i in issues)


def test_preview_manifest_html_lists_icons(tmp_path: Path) -> None:
    ldtk = tmp_path / "world.ldtk"
    manifest = {
        "editor_icons": {"identifier": "EditorIcons", "path": str(tmp_path / "icons.png"), "tile_width": 32},
        "entity_icons": {"CameraZone": {"tileset": "EditorIcons", "index": 0}},
    }
    html = preview_manifest_html(ldtk, manifest)
    assert "CameraZone" in html
    assert "EditorIcons" in html
