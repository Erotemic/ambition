"""The LDtk tileset census must refuse an empty sweep, and must not miss a USER.

`measure_ldtk_tileset_usage.py` establishes the fact the whole player-sheet
retarget rests on: **no level layer uses the editor-preview tileset**, so
dropping it to a cheaper tier costs no drawn pixel. If that were wrong — if some
layer did draw from it — a cheaper tier would be a QUALITY decision instead of a
free one, and the patch in `dev/patches/` would be a regression.

⛔⛔ SO THE DANGEROUS FAILURE IS A FALSE "NOBODY USES IT". Two ways to produce
one: a scan that finds no worlds at all (submodule not checked out — refused
loudly), and a layer-use test that looks in the wrong place. LDtk stores a
level's `layerInstances` inline OR in a separate file per level
(`externalRelPath`); a census that only reads inline layers would report "no
layer uses this" about a project whose layers it never opened.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/measure_ldtk_tileset_usage.py"


def load():
    spec = importlib.util.spec_from_file_location("ldtk_usage", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _world(path: Path, *, uid: int = 7, layer_uid: int | None = None,
           external: bool = False) -> None:
    level = {"identifier": "start"}
    if external:
        level["externalRelPath"] = "levels/start.ldtkl"
    else:
        level["layerInstances"] = [{"__tilesetDefUid": layer_uid}]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({
        "defs": {
            "tilesets": [{
                "uid": uid, "identifier": "sprite_player_robot_v3",
                "relPath": "../sprites/player_robot_v3_spritesheet.png",
                "pxWid": 3072, "pxHei": 2484, "tileGridSize": 256,
            }],
            "entities": [{
                "identifier": "PlayerStart", "tilesetId": uid,
                "tileRect": {"tilesetUid": uid, "x": 0, "y": 0, "w": 256, "h": 256},
                "uiTileRect": {"tilesetUid": uid, "x": 0, "y": 0, "w": 256, "h": 256},
            }],
        },
        "levels": [level],
    }))


def test_a_layer_that_uses_the_tileset_is_reported_as_drawn(tmp_path):
    """⭐ THE CLAIM THE RETARGET RESTS ON, IN ITS DANGEROUS DIRECTION. If a layer
    draws from the tileset, the census must say so — a false 'nobody uses it'
    turns a free change into a quality regression."""
    module = load()
    _world(tmp_path / "w.ldtk", uid=7, layer_uid=7)
    report = module.inspect(tmp_path / "w.ldtk")
    assert report["tilesets"][0]["used_by_a_layer"] is True


def test_a_layer_using_a_different_tileset_is_not_confused_for_this_one(tmp_path):
    module = load()
    _world(tmp_path / "w.ldtk", uid=7, layer_uid=99)
    report = module.inspect(tmp_path / "w.ldtk")
    assert report["tilesets"][0]["used_by_a_layer"] is False
    assert report["tilesets"][0]["entity_users"], (
        "the entity preview is still the tileset's only consumer, and must be "
        "reported — that is what makes the retarget safe rather than unused"
    )


def test_external_levels_are_counted_so_absence_is_not_overclaimed(tmp_path):
    """⛔ A level may store its layers in a SEPARATE FILE. Reporting
    'used_by_a_layer: false' about layers never opened is the false all-clear
    this census must not give silently."""
    module = load()
    _world(tmp_path / "w.ldtk", uid=7, external=True)
    report = module.inspect(tmp_path / "w.ldtk")
    assert report["levels_external"] == 1, (
        "the census must count externalised levels so the report can say its "
        "layer-use answer is only what this file shows"
    )


def test_it_refuses_when_no_world_is_found(tmp_path, monkeypatch, capsys):
    module = load()
    monkeypatch.setattr(module, "MAP_ASSETS", tmp_path / "absent")
    assert module.main([]) == 2
    out = capsys.readouterr().out
    assert "NO `.ldtk` WORLDS" in out and "Absent is not zero" in out


def test_the_real_worlds_still_declare_the_sheet_this_census_is_about():
    """⭐ POSITIVE CONTROL against the live submodule: if the declaration is gone
    the retarget has landed and this whole instrument is spent — which should be
    noticed, not silently reported as zero."""
    module = load()
    if not module.MAP_ASSETS.is_dir():
        pytest.skip("game/ambition_map_assets is not checked out")
    naming = [
        r for r in (module.inspect(w) for w in module.worlds()) if r
        for t in r["tilesets"] if t["relPath"] == module.TARGET_REL
    ]
    assert naming, (
        "no world declares the full-resolution player sheet any more — either "
        "the retarget landed (delete this instrument) or the declaration moved"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
