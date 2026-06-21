"""Tests for the LDtk visual-manifest emitter (sprite -> LDtk-editor bridge).

The emitter is the producer half consumed by
`ambition_ldtk_tools.edit.visual_manifest`; these tests pin the manifest
shape and the sheet-discovery rules (frame size from either sidecar flavor,
layer/aux variants skipped) without needing Pillow or the full renderer.
"""

from __future__ import annotations

import json
from pathlib import Path

from ambition_sprite2d_renderer.ldtk_manifest import (
    DEFAULT_ENTITY_SPRITE_MAP,
    build_manifest,
    discover_sheets,
    _frame_size_from_yaml_scan,
)


def test_default_entity_map_covers_the_placeable_characters() -> None:
    # Pin the wired set so a future edit can't silently drop an entity's real
    # sprite back to a gizmo. Values are sprite stems (resolved at apply time).
    assert DEFAULT_ENTITY_SPRITE_MAP == {
        "PlayerStart": "player_robot",
        "NpcSpawn": "merchant_prototype",
        "EnemySpawn": "goblin",
        "BossSpawn": "gnu_ton_boss",
    }


def _make_sheet(directory: Path, stem: str) -> None:
    """An (empty) sheet PNG — discovery globs it; dimensions are the consumer's job."""
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{stem}_spritesheet.png").write_bytes(b"")


def _scene(tmp_path: Path) -> Path:
    sprites = tmp_path / "sprites"
    # Simple character sheet: frame size from the YAML sidecar.
    _make_sheet(sprites, "hero")
    (sprites / "hero_spritesheet.yaml").write_text("frame_width: 32\nframe_height: 48\n")
    # Boss sheet in a subdir: frame size from the JSON manifest sidecar.
    _make_sheet(sprites / "bigboss", "bigboss")
    (sprites / "bigboss" / "bigboss_spritesheet_manifest.json").write_text(
        json.dumps({"frame_size": [64, 80], "rows": [{"name": "rest", "row": 0}]})
    )
    # Layer variant — no sidecar of its own -> must be skipped.
    _make_sheet(sprites, "hero_full")
    # Sheet with no sidecar at all -> cannot tell LDtk the grid -> skipped.
    _make_sheet(sprites, "mystery")
    # Aux render -> excluded by suffix.
    _make_sheet(sprites, "hero_canonical")
    return sprites


def test_discover_sheets_keeps_only_sheets_with_a_sidecar(tmp_path: Path) -> None:
    sheets = discover_sheets(_scene(tmp_path))
    assert set(sheets) == {"hero", "bigboss"}, "layer/aux/sidecar-less sheets are skipped"
    assert sheets["hero"]["frame_size"] == (32, 48)
    assert sheets["bigboss"]["frame_size"] == (64, 80)


def test_build_manifest_all_sheets_shape(tmp_path: Path) -> None:
    sprites = _scene(tmp_path)
    manifest = build_manifest(
        sprites, repo_root=tmp_path, entity_map={"PlayerStart": "hero"}, all_sheets=True
    )
    by_id = {t["identifier"]: t for t in manifest["tilesets"]}
    assert set(by_id) == {"sprite_hero", "sprite_bigboss"}
    assert by_id["sprite_hero"]["tile_width"] == 32
    assert by_id["sprite_hero"]["tile_height"] == 48
    assert by_id["sprite_hero"]["tags"] == ["sprite"]
    # Paths are repo-root-relative and POSIX so the committed manifest is portable.
    assert by_id["sprite_hero"]["path"] == "sprites/hero_spritesheet.png"
    assert by_id["sprite_bigboss"]["path"] == "sprites/bigboss/bigboss_spritesheet.png"
    # Entity icon points at frame 0 of the mapped sheet.
    assert manifest["entity_icons"] == {
        "PlayerStart": {"tileset": "sprite_hero", "tile": [0, 0, 32, 48]}
    }


def test_build_manifest_curated_registers_only_referenced_sheets(tmp_path: Path) -> None:
    sprites = _scene(tmp_path)
    manifest = build_manifest(
        sprites, repo_root=tmp_path, entity_map={"PlayerStart": "hero"}, all_sheets=False
    )
    assert [t["identifier"] for t in manifest["tilesets"]] == ["sprite_hero"]
    assert "PlayerStart" in manifest["entity_icons"]


def test_entity_map_entry_with_no_sheet_is_dropped(tmp_path: Path) -> None:
    sprites = _scene(tmp_path)
    manifest = build_manifest(
        sprites, repo_root=tmp_path, entity_map={"PlayerStart": "ghost"}, all_sheets=False
    )
    assert manifest["tilesets"] == []
    assert manifest["entity_icons"] == {}


def test_yaml_scan_fallback_reads_frame_size(tmp_path: Path) -> None:
    path = tmp_path / "x_spritesheet.yaml"
    path.write_text("target: toon\nframe_width: 90\nframe_height: 112\nborder: 4\n")
    assert _frame_size_from_yaml_scan(path) == (90, 112)
