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
    prune_unused_tilesets,
    validate_manifest,
    write_png,
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


def test_repointing_an_icon_orphans_the_old_tileset_and_prune_removes_it(
    tmp_path: Path,
) -> None:
    """⭐ Renaming a sheet is what leaves a def naming a PNG nobody ships.

    `player_robot_spritesheet.png` became `player_robot_v3_spritesheet.png`.
    Applying the new manifest repoints PlayerStart but leaves the def it used
    to point at, still carrying the dead relPath — which is what the package
    asset guard trips over. Apply alone does not clean that up; pruning does.
    """
    sheet = tmp_path / "hero_v3_spritesheet.png"
    write_png(sheet, 64, 64, bytes(64 * 64 * 4))

    project = mini_project()
    project["defs"]["tilesets"].append(
        {
            "identifier": "sprite_hero",
            "uid": 50,
            "relPath": "../sprites/hero_spritesheet.png",
            "pxWid": 128,
            "pxHei": 128,
            "tileGridSize": 32,
            "__cWid": 4,
            "__cHei": 4,
        }
    )
    player = project["defs"]["entities"][2]
    player["tilesetId"] = 50
    player["tileRect"] = {"tilesetUid": 50, "x": 0, "y": 0, "w": 32, "h": 32}

    ldtk = tmp_path / "world.ldtk"
    ldtk.write_text(json.dumps(project))
    manifest = {
        "tilesets": [
            {
                "identifier": "sprite_hero_v3",
                "path": str(sheet),
                "tile_width": 32,
                "tile_height": 32,
                "tags": ["sprite"],
            }
        ],
        "entity_icons": {
            "PlayerStart": {"tileset": "sprite_hero_v3", "tile": [0, 0, 32, 32]}
        },
    }

    apply_manifest(project, ldtk, manifest)
    identifiers = [ts["identifier"] for ts in project["defs"]["tilesets"]]
    assert "sprite_hero_v3" in identifiers
    assert "sprite_hero" in identifiers, "apply alone leaves the orphan behind"

    messages = prune_unused_tilesets(project)
    assert any("sprite_hero" in msg for msg in messages)
    assert [ts["identifier"] for ts in project["defs"]["tilesets"]] == ["sprite_hero_v3"]


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
