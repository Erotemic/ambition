from __future__ import annotations

from pathlib import Path

from ambition_ldtk_tools.ldtk import (
    alloc_uid,
    default_character_catalog,
    default_hall_ldtk,
    default_sandbox_ldtk,
    default_sprite_assets_dir,
    entity_field_value,
    ensure_entities_layer_def,
    ensure_entities_layer_instance,
    find_layer_def,
    find_layer_instance,
    iter_entities,
    path_from_ldtk,
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


def test_canonical_repo_asset_paths_follow_content_split() -> None:
    repo = Path(__file__).resolve().parents[3]
    assert default_character_catalog(repo) == (
        repo / "game" / "ambition_content" / "assets" / "data" / "character_catalog.ron"
    )
    assert default_sandbox_ldtk(repo) == (
        repo / "game" / "ambition_content" / "assets" / "worlds" / "sandbox.ldtk"
    )
    assert default_hall_ldtk(repo) == (
        repo / "game" / "ambition_content" / "assets" / "worlds" / "hall_of_characters.ldtk"
    )
    assert default_sprite_assets_dir(repo) == (
        repo / "crates" / "ambition_actors" / "assets" / "sprites"
    )
    content_sprite_mount = repo / "game" / "ambition_content" / "assets" / "sprites"
    assert content_sprite_mount.resolve() == default_sprite_assets_dir(repo).resolve()
    assert default_character_catalog(repo).is_file()
    assert default_sandbox_ldtk(repo).is_file()
    assert default_hall_ldtk(repo).is_file()


def test_cross_crate_sprites_use_the_game_source_virtual_mount(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    (repo / "crates").mkdir(parents=True)
    (repo / "tools").mkdir()
    ldtk = repo / "game" / "ambition_content" / "assets" / "worlds" / "sandbox.ldtk"
    sprite = (
        repo
        / "crates"
        / "ambition_actors"
        / "assets"
        / "sprites"
        / "player_robot_spritesheet.png"
    )
    ldtk.parent.mkdir(parents=True)
    sprite.parent.mkdir(parents=True)
    ldtk.write_text("{}")
    sprite.write_bytes(b"png")

    rel = rel_to_ldtk(ldtk, sprite)
    assert rel == "../sprites/player_robot_spritesheet.png"
    assert path_from_ldtk(ldtk, rel) == sprite.resolve()


def test_authoritative_world_tilesets_are_runtime_safe_and_resolvable() -> None:
    import json

    worlds = default_sandbox_ldtk().parent
    for ldtk in sorted(worlds.glob("*.ldtk")):
        project = json.loads(ldtk.read_text())
        for tileset in project.get("defs", {}).get("tilesets", []):
            rel = tileset.get("relPath")
            if not rel:
                continue
            assert "crates/" not in rel, f"{ldtk}: unsafe repository traversal {rel!r}"
            assert not Path(rel).is_absolute(), f"{ldtk}: absolute tileset path {rel!r}"
            resolved = path_from_ldtk(ldtk, rel)
            if rel.startswith("../sprites/"):
                expected = default_sprite_assets_dir(ldtk) / rel.removeprefix("../sprites/")
                assert resolved == expected.absolute(), (ldtk, rel, resolved, expected)
