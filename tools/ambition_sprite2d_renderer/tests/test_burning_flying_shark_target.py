import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

from ambition_sprite2d_renderer.targets.characters import burning_flying_shark


def test_burning_flying_shark_render_smoke(tmp_path: Path):
    outputs = burning_flying_shark.render(tmp_path)
    assert outputs
    expected = {
        "burning_flying_shark_spritesheet.png",
        "burning_flying_shark_spritesheet.yaml",
    }
    found = {p.name for p in outputs}
    assert expected.issubset(found)
    for name in expected:
        assert (tmp_path / name).exists()
