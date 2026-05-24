import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

from ambition_sprite2d_renderer.targets.props import intro_cart


def test_intro_cart_render_smoke(tmp_path: Path):
    outputs = intro_cart.render(tmp_path)
    assert outputs
    expected = {
        "intro_cart_spritesheet.png",
        "intro_cart_spritesheet.yaml",
    }
    found = {p.name for p in outputs}
    assert expected.issubset(found)
    for name in expected:
        assert (tmp_path / name).exists()
