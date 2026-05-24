import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

import yaml

from ambition_sprite2d_renderer.targets.tiles import intro_lab_tileset


def test_intro_lab_tileset_render_smoke(tmp_path: Path):
    outputs = intro_lab_tileset.render(tmp_path)
    assert outputs
    expected = {
        "intro_lab_tileset.png",
        "intro_lab_tileset.yaml",
        "intro_lab_tileset_preview_labeled.png",
    }
    found = {p.name for p in outputs}
    assert expected.issubset(found)
    for name in expected:
        assert (tmp_path / name).exists()

    manifest = yaml.safe_load((tmp_path / "intro_lab_tileset.yaml").read_text())
    assert manifest["tile_width"] == 32
    assert manifest["tile_height"] == 32
    assert manifest["columns"] == 16
    assert manifest["tile_count"] >= 80
    assert "ldtk" in manifest
    assert "LabSolids" in manifest["groups"]["by_ldtk_layer"]
    assert "LabProps" in manifest["groups"]["by_ldtk_layer"]
