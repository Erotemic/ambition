from pathlib import Path

from PIL import Image

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.cli import draw_all, draw_canonicals
from proc2d_character_lab.config import CharacterJob
from proc2d_character_lab.entities import ENTITY_SPECS, write_entity_sprites


def _alpha_bbox_metrics(img):
    bbox = img.getchannel("A").getbbox()
    assert bbox is not None
    x1, y1, x2, y2 = bbox
    return {
        "w": x2 - x1,
        "h": y2 - y1,
        "area": sum(img.getchannel("A").histogram()[11:]),
        "bottom": y2,
        "bbox": bbox,
    }


def test_render_default_assets(tmp_path):
    out_dir = tmp_path / "assets"
    outputs = draw_all("proc2d_character_lab/configs", out_dir)
    outputs += draw_canonicals("proc2d_character_lab/configs", out_dir / "canonicals")
    expected = {
        out_dir / "boss_spritesheet.png",
        out_dir / "boss_spritesheet.yaml",
        out_dir / "robot_spritesheet.png",
        out_dir / "robot_spritesheet.yaml",
        out_dir / "goblin_spritesheet.png",
        out_dir / "goblin_spritesheet.yaml",
        out_dir / "canonicals" / "boss_canonical.png",
        out_dir / "canonicals" / "robot_canonical.png",
        out_dir / "canonicals" / "goblin_canonical.png",
        out_dir / "canonicals" / "canonicals_contact_sheet.png",
    }
    assert expected.issubset(set(map(Path, outputs)))
    for path in expected:
        assert path.exists(), path


def test_animation_sets_include_blink_parts_and_dash():
    for target in ["robot", "goblin"]:
        adapter = get_adapter(target)
        assert "blink_out" in adapter.animations()
        assert "blink_in" in adapter.animations()
        assert "dash" in adapter.animations()


def test_death_frames_keep_visible_mass_and_anchor():
    for cfg in ["robot.yaml", "goblin.yaml", "boss.yaml"]:
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


def test_blink_parts_are_teleport_not_eyelid_blink():
    for target in ["robot", "goblin"]:
        adapter = get_adapter(target)
        adapter.sample_spec(CharacterJob.load(Path("proc2d_character_lab/configs") / f"{target}.yaml"))
        generator = adapter.generator
        for name in ["blink_out", "blink_in"]:
            info = adapter.animations()[name]
            for idx in range(info["frames"]):
                pose = generator.pose_for_animation(name, idx, info["frames"])
                assert not pose.blink


def test_entity_sprites_render(tmp_path):
    out_dir = tmp_path / "entities"
    outputs = write_entity_sprites(out_dir)
    expected = {out_dir / spec.filename for spec in ENTITY_SPECS}
    expected.add(out_dir / "entity_contact_sheet.png")
    expected.add(out_dir / "entity_manifest.yaml")
    assert expected.issubset(set(map(Path, outputs)))
    for path in expected:
        assert path.exists(), path
        if path.suffix == ".png":
            img = Image.open(path).convert("RGBA")
            assert img.getchannel("A").getbbox() is not None, path


def test_entity_manifest_contains_current_feature_families(tmp_path):
    out_dir = tmp_path / "entities"
    write_entity_sprites(out_dir)
    manifest = (out_dir / "entity_manifest.yaml").read_text()
    for token in ["FeatureVisualKind::Chest", "FeatureVisualKind::Breakable", "FeatureVisualKind::Pickup", "FeatureVisualKind::Boss", "ActorKind::MovingPlatform"]:
        assert token in manifest


def test_boss_animation_set_matches_rust_boss_attack_kind():
    adapter = get_adapter("boss")
    keys = set(adapter.animations())
    expected = {
        "rest",
        "floor_slam",
        "side_sweep",
        "spike_halo",
        "dash_echo",
        "hit",
        "death",
    }
    assert keys == expected
    assert "spit" not in keys
    assert "beam_fire" not in keys
    assert "teleport_out" not in keys


def test_boss_attack_rows_render_non_empty():
    job = CharacterJob.load(Path("proc2d_character_lab/configs/boss.yaml"))
    adapter = get_adapter("boss")
    spec = adapter.sample_spec(job)
    for name in ["rest", "floor_slam", "side_sweep", "spike_halo", "dash_echo"]:
        info = adapter.animations()[name]
        img = adapter.render_frame(spec, name, info["frames"] // 2, (128, 128), job)
        assert img.getchannel("A").getbbox() is not None, name
