import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

import yaml

from ambition_sprite2d_renderer.targets.tiles import town_tileset


def test_town_tileset_render(tmp_path: Path):
    paths = town_tileset.render(tmp_path)
    assert len(paths) == 3
    for path in paths:
        assert path.exists()

    manifest = yaml.safe_load((tmp_path / town_tileset.SHEET_FILES[1]).read_text())
    assert manifest["target"] == town_tileset.TARGET_NAME
    assert manifest["sheet_kind"] == "tileset"
    assert manifest["view"] == "side_scroller"
    assert manifest["tile_count"] == 96
    assert manifest["atlas_columns"] == 8
    assert manifest["atlas_rows"] == 12
    assert manifest["tile_order"][0] == "grass_top"
    assert manifest["tile_order"][-1] == "banner_blue"

    assert manifest["tiles"]["grass_slope_up"]["category"] == "terrain"
    assert manifest["tiles"]["wall_plaster_plain"]["category"] == "wall_plaster"
    assert manifest["tiles"]["door_shop"]["layer"] == "structure"
    assert manifest["tiles"]["roof_blue_chimney"]["atlas_row"] >= 0
    assert manifest["tiles"]["lamp_post"]["layer"] == "overlay"
    assert manifest["tiles"]["market_stall_red"]["layer"] == "overlay"

    assert len(manifest["tile_groups"]["terrain"]) == 8
    assert len(manifest["tile_groups"]["roof_red"]) == 8
    assert len(manifest["tile_groups"]["civic_prop"]) == 8
