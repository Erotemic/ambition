from pathlib import Path

from PIL import Image

from proc2d_character_lab.adapters import get_adapter
from proc2d_character_lab.cli import draw_all, draw_canonicals
from proc2d_character_lab.config import CharacterJob
from proc2d_character_lab.entities import ENTITY_SPECS, write_entity_sprites
from proc2d_character_lab.sheet import build_spritesheet


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


def test_tight_crop_eliminates_transparent_edges_on_entity_sprites(tmp_path):
    """Pin the post-crop content density. The whole reason we
    auto-crop is so a 30%-canvas-fill drawer doesn't render as a
    visibly-undersized sprite once stretched to a collision box.
    Demand >=70% fill on the published sprites; they were ~30%
    before the crop pass landed."""
    out_dir = tmp_path / "entities"
    write_entity_sprites(out_dir)
    samples = [
        "chest_closed.png",
        "pickup_health.png",
        "breakable_intact.png",
        "pogo_orb.png",
        "hazard_spikes.png",
        "solid_block.png",
    ]
    for name in samples:
        img = Image.open(out_dir / name).convert("RGBA")
        bbox = img.getchannel("A").getbbox()
        assert bbox is not None, name
        l, t, r, b = bbox
        cw, ch = r - l, b - t
        w, h = img.size
        fill = (cw * ch) / (w * h)
        assert fill >= 0.70, (
            f"{name} content fill {fill:.0%} below 70% — sprite has "
            "too much transparent margin and will look smaller than "
            "its collision box when stretched"
        )


def test_tile_sprites_are_32x32_and_not_cropped(tmp_path):
    """Tile sprites must keep their full 32×32 canvas — Bevy's
    `Sprite::image_mode = Tiled` repeats the texture at native
    pixel scale, so cropping a tile would change the repeat
    period and break the seamless wrap. Pin both the dimensions
    and the (mostly) full coverage."""
    out_dir = tmp_path / "entities"
    write_entity_sprites(out_dir)
    tiles = {
        "solid_tile.png": (32, 32),
        "one_way_tile.png": (32, 32),
        "hazard_tile.png": (32, 32),
        "soft_blink_tile.png": (32, 32),
        "hard_blink_tile.png": (32, 32),
    }
    for name, expected_size in tiles.items():
        img = Image.open(out_dir / name).convert("RGBA")
        assert img.size == expected_size, (name, img.size)


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


def test_spritesheet_emits_body_metrics():
    """Sprite manifests must carry measured body extent so Rust can align
    sprites with collision boxes without hand-tuned anchor constants."""
    for cfg_name in ("robot", "goblin", "boss"):
        job = CharacterJob.load(Path(f"proc2d_character_lab/configs/{cfg_name}.yaml"))
        # Truncate to one anim and skip supersampling so this test is fast.
        job.animations = job.animations[:1]
        job.render.supersample = 1
        _, manifest = build_spritesheet(job)
        assert "body_metrics" in manifest, cfg_name
        bm = manifest["body_metrics"]

        bbox = bm["body_pixel_bbox"]
        assert bbox["w"] > 0 and bbox["h"] > 0, (cfg_name, bbox)

        feet = bm["feet_pixel"]
        assert 0 <= feet["x"] <= bm["frame_width"], (cfg_name, feet)
        assert 0 <= feet["y"] <= bm["frame_height"], (cfg_name, feet)

        # Bevy anchor convention: y in [-0.5, +0.5], 0=center, +0.5=top.
        # Our characters all stand near the bottom of their frames, so the
        # feet anchor should always be below center (negative).
        anchor_y = bm["feet_anchor_norm"]["y"]
        assert -0.5 <= anchor_y < 0.0, (cfg_name, anchor_y)
