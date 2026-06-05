import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

from ambition_sprite2d_renderer.targets.props import interdimensional_gate


def test_interdimensional_gate_render_smoke(tmp_path: Path):
    outputs = interdimensional_gate.render(tmp_path)
    assert outputs
    expected = {
        "interdimensional_gate_ring_spritesheet.png",
        "interdimensional_gate_ring_spritesheet.yaml",
        "interdimensional_gate_portal_spritesheet.png",
        "interdimensional_gate_portal_spritesheet.yaml",
    }
    found = {p.name for p in outputs}
    assert expected.issubset(found)
    for name in expected:
        assert (tmp_path / name).exists()
