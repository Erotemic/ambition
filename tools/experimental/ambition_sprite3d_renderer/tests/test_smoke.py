from pathlib import Path

from PIL import Image

from gen3d_blender_lab.adapters import get_adapter
from gen3d_blender_lab.config import CharacterJob
from gen3d_blender_lab.sheet import render_spritesheet


def _fake_render_requests(adapter, job, requests, mode="frames", blender_executable=None):
    for req in requests:
        path = Path(req["out_path"])
        path.parent.mkdir(parents=True, exist_ok=True)
        img = Image.new("RGBA", (req["width"], req["height"]), (0, 0, 0, 0))
        img.save(path)
    return {"requests": list(requests), "mode": mode}


def test_goblin_sheet_smoke(tmp_path: Path, monkeypatch):
    monkeypatch.setattr("gen3d_blender_lab.sheet.render_requests", _fake_render_requests)
    job = CharacterJob(target="goblin", seed=1, archetype="shadow_scout", held_item="spear", animations=["idle"])
    adapter = get_adapter("goblin")
    out = tmp_path / "goblin.png"
    manifest = render_spritesheet(adapter, job, out, tmp_path / "goblin.yaml")
    assert out.exists()
    assert len(manifest["frames"]) == adapter.animations()["idle"]["frames"]


def test_robot_sheet_smoke(tmp_path: Path, monkeypatch):
    monkeypatch.setattr("gen3d_blender_lab.sheet.render_requests", _fake_render_requests)
    job = CharacterJob(target="robot", seed=7, archetype="scout_bot", animations=["idle"])
    adapter = get_adapter("robot")
    out = tmp_path / "robot.png"
    manifest = render_spritesheet(adapter, job, out, tmp_path / "robot.yaml")
    assert out.exists()
    assert len(manifest["frames"]) == adapter.animations()["idle"]["frames"]
