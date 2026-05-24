import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

import yaml

from ambition_sprite2d_renderer.targets.props import creator_lab_props


def test_creator_lab_props_render(tmp_path: Path):
    paths = creator_lab_props.render(tmp_path)
    assert len(paths) == 3
    for path in paths:
        assert path.exists()

    manifest = yaml.safe_load((tmp_path / creator_lab_props.SHEET_FILES[1]).read_text())
    assert manifest["target"] == creator_lab_props.TARGET_NAME
    assert manifest["sheet_kind"] == "prop_spritesheet"
    assert manifest["prop_order"]
    assert "genesis_vat" in manifest["props"]
    assert len(manifest["props"]["portal_calibrator"]["frames"]) == 4
