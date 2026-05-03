from pathlib import Path

from PIL import Image

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.cli import draw_all, draw_canonicals
from proc2d_character_lab.config import CharacterJob


def _alpha_bbox_metrics(img):
    bbox = img.getchannel("A").getbbox()
    assert bbox is not None
    x1, y1, x2, y2 = bbox
    return {
        "w": x2 - x1,
        "h": y2 - y1,
        "area": sum(1 for v in img.getchannel("A").getdata() if v > 10),
        "bottom": y2,
        "bbox": bbox,
    }


def test_render_default_assets(tmp_path):
    out_dir = tmp_path / "assets"
    outputs = draw_all("proc2d_character_lab/configs", out_dir)
    outputs += draw_canonicals("proc2d_character_lab/configs", out_dir / "canonicals")
    expected = {
        out_dir / "robot_spritesheet.png",
        out_dir / "robot_spritesheet.yaml",
        out_dir / "goblin_spritesheet.png",
        out_dir / "goblin_spritesheet.yaml",
        out_dir / "canonicals" / "robot_canonical.png",
        out_dir / "canonicals" / "goblin_canonical.png",
        out_dir / "canonicals" / "canonicals_contact_sheet.png",
    }
    assert expected.issubset(set(map(Path, outputs)))
    for path in expected:
        assert path.exists(), path


def test_animation_sets_include_blink_and_dash():
    for target in ["robot", "goblin"]:
        adapter = get_adapter(target)
        assert "blink" in adapter.animations()
        assert "dash" in adapter.animations()


def test_death_frames_keep_visible_mass_and_anchor():
    for cfg in ["robot.yaml", "goblin.yaml"]:
        job = CharacterJob.load(Path("proc2d_character_lab/configs") / cfg)
        adapter = get_adapter(job.target)
        spec = adapter.sample_spec(job)
        info = adapter.animations()["death"]
        frames = [adapter.render_frame(spec, "death", idx, (128, 128), job) for idx in range(info["frames"])]
        metrics = [_alpha_bbox_metrics(img) for img in frames]
        first = metrics[0]
        min_area = min(m["area"] for m in metrics)
        # The old failure mode collapsed to a much smaller sprite.  Allow pose changes,
        # but require the visible mass and ground anchor to stay broadly consistent.
        assert min_area >= first["area"] * 0.78, (job.target, metrics)
        for m in metrics:
            assert m["bottom"] >= first["bottom"] - 8, (job.target, metrics)
            assert m["w"] >= first["w"] * 0.70, (job.target, metrics)


def test_blink_animation_is_teleport_not_eyelid_blink():
    for target in ["robot", "goblin"]:
        adapter = get_adapter(target)
        spec = adapter.sample_spec(CharacterJob.load(Path("proc2d_character_lab/configs") / f"{target}.yaml"))
        # The animation named "blink" represents Ambition's teleport ability.
        # Incidental eyelid blinking can happen in idle, but the blink row itself
        # should not close the eye / visor as its primary action.
        generator = adapter.generator
        info = adapter.animations()["blink"]
        for idx in range(info["frames"]):
            pose = generator.pose_for_animation("blink", idx, info["frames"])
            assert not pose.blink
