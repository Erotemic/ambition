from pathlib import Path

from ambition_sprite2d_renderer.targets import creator


def test_creator_render_smoke(tmp_path: Path):
    outputs = creator.render(tmp_path)
    assert outputs
    expected = {
        "creator_spritesheet.png",
        "creator_spritesheet.yaml",
    }
    found = {p.name for p in outputs}
    assert expected.issubset(found)
    for name in expected:
        assert (tmp_path / name).exists()
