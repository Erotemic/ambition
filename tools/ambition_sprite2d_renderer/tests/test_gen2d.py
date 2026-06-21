from pathlib import Path

import pytest
from PIL import Image, ImageChops
import yaml

from ambition_sprite2d_renderer.authoring.adapters import get_adapter
from ambition_sprite2d_renderer.authoring.animation_vocab import (
    ADVANCED_PLAYER_ANIMATION_ORDER,
    EXTENDED_PLAYER_ANIMATION_ORDER,
)
from ambition_sprite2d_renderer.cli import draw_all, draw_canonicals
from ambition_sprite2d_renderer.registry import CharacterJob
from ambition_sprite2d_renderer.targets.props.entities import (
    ENTITY_SPECS,
    write_entity_sprites,
)
from ambition_sprite2d_renderer.authoring.sheet import build_spritesheet


# Resolve configs relative to the package, not the cwd.
CONFIGS = (
    Path(__file__).resolve().parent.parent / "ambition_sprite2d_renderer" / "configs"
)
REVIEW_CONFIGS = CONFIGS / "review"


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


@pytest.mark.slow_render
def test_render_default_assets(tmp_path):
    out_dir = tmp_path / "assets"
    outputs = draw_all(str(CONFIGS), out_dir)
    outputs += draw_canonicals(str(CONFIGS), out_dir / "canonicals")
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


@pytest.mark.slow_render
def test_death_frames_keep_visible_mass_and_anchor():
    for cfg in ["robot.yaml", "goblin.yaml", "boss.yaml"]:
        job = CharacterJob.load(Path(str(CONFIGS)) / cfg)
        adapter = get_adapter(job.target)
        spec = adapter.sample_spec(job)
        info = adapter.animations()["death"]
        frames = [
            adapter.render_frame(spec, "death", idx, (128, 128), job)
            for idx in range(info["frames"])
        ]
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
        adapter.sample_spec(CharacterJob.load(Path(str(CONFIGS)) / f"{target}.yaml"))
        generator = adapter.generator
        for name in ["blink_out", "blink_in"]:
            info = adapter.animations()[name]
            for idx in range(info["frames"]):
                pose = generator.pose_for_animation(name, idx, info["frames"])
                assert not pose.blink


@pytest.mark.slow_render
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


@pytest.mark.slow_render
def test_entity_manifest_contains_current_feature_families(tmp_path):
    out_dir = tmp_path / "entities"
    write_entity_sprites(out_dir)
    manifest = (out_dir / "entity_manifest.yaml").read_text()
    for token in [
        "FeatureVisualKind::Chest",
        "FeatureVisualKind::Breakable",
        "FeatureVisualKind::Pickup",
        "FeatureVisualKind::Boss",
        "ActorKind::MovingPlatform",
    ]:
        assert token in manifest


@pytest.mark.slow_render
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


@pytest.mark.slow_render
def test_tile_sprites_match_authored_dimensions_and_skip_crop(tmp_path):
    """Tile sprites must keep their full authored canvas — Bevy's
    `Sprite::image_mode = Tiled` repeats the texture at native
    pixel scale, so cropping (or wrong-axis sizing) would change
    the repeat period and break the seamless wrap.

    Sizes are authored to match the typical IntGrid block height:
    - solid + blink walls: 32×32 (multi-cell vertical surfaces).
    - one-way + hazard: 32×16 (typical one-cell-tall rows; a
      32-tall texture would get vertically stretched on a 16-tall
      block, compressing the plate/spike pattern)."""
    out_dir = tmp_path / "entities"
    write_entity_sprites(out_dir)
    tiles = {
        "solid_tile.png": (32, 32),
        "one_way_tile.png": (32, 16),
        "hazard_tile.png": (32, 16),
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


@pytest.mark.slow_render
def test_boss_attack_rows_render_non_empty():
    job = CharacterJob.load(CONFIGS / "boss.yaml")
    adapter = get_adapter("boss")
    spec = adapter.sample_spec(job)
    for name in ["rest", "floor_slam", "side_sweep", "spike_halo", "dash_echo"]:
        info = adapter.animations()[name]
        img = adapter.render_frame(spec, name, info["frames"] // 2, (128, 128), job)
        assert img.getchannel("A").getbbox() is not None, name


@pytest.mark.slow_render
def test_spritesheet_emits_body_metrics():
    """Sprite manifests must carry measured body extent so Rust can align
    sprites with collision boxes without hand-tuned anchor constants."""
    for cfg_name in ("robot", "goblin", "boss"):
        job = CharacterJob.load(Path(CONFIGS / f"{cfg_name}.yaml"))
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


@pytest.mark.slow_render
def test_sandbag_adapter_participates_in_character_pipeline(tmp_path):
    job = CharacterJob.load(CONFIGS / "sandbag.yaml")
    # This test pins base geometry + crop behavior, not texture resolution, so
    # render at 1x (the fleet default is 2x, which would scale these dims).
    job.render.render_scale = 1
    adapter = get_adapter("sandbag")
    animations = adapter.animations()
    for name in [
        "idle",
        "walk",
        "run",
        "jump",
        "fall",
        "slash",
        "hit",
        "death",
        "blink_out",
        "blink_in",
        "dash",
    ]:
        assert name in animations
    sheet, manifest = build_spritesheet(job)
    assert sheet.size[1] == 11 * 128
    assert manifest["target"] == "sandbag"
    assert manifest["crop"]["enabled"] is False
    assert manifest["frame_width"] == 128
    assert manifest["frame_height"] == 128


@pytest.mark.slow_render
def test_extended_player_review_rows_render_non_empty(tmp_path):
    job = CharacterJob.load(REVIEW_CONFIGS / "player_extended.yaml")
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    for name in EXTENDED_PLAYER_ANIMATION_ORDER + ADVANCED_PLAYER_ANIMATION_ORDER:
        info = adapter.animations()[name]
        img = adapter.render_frame(spec, name, info["frames"] // 2, (128, 128), job)
        assert img.getchannel("A").getbbox() is not None, name


@pytest.mark.slow_render
def test_draw_all_renders_core_runtime_configs(tmp_path):
    outputs = draw_all(str(CONFIGS), tmp_path)
    output_names = {Path(p).name for p in outputs}
    assert "robot_spritesheet.png" in output_names
    assert "goblin_spritesheet.png" in output_names
    assert "boss_spritesheet.png" in output_names
    assert "sandbag_spritesheet.png" in output_names
    assert "player_extended_spritesheet.png" not in output_names


@pytest.mark.slow_render
def test_review_configs_do_not_overwrite_variants(tmp_path):
    # Render a compact temporary subset so this contract stays fast while the
    # full review directory can continue growing.
    subset = tmp_path / "review_configs"
    subset.mkdir()
    for name in [
        "robot_runner.yaml",
        "robot_guardian.yaml",
        "player_social_review.yaml",
        "sandbag_full_review.yaml",
    ]:
        (subset / name).write_text((REVIEW_CONFIGS / name).read_text())
    outputs = draw_all(str(subset), tmp_path / "out")
    output_names = {Path(p).name for p in outputs}
    assert "robot_runner_spritesheet.png" in output_names
    assert "robot_guardian_spritesheet.png" in output_names
    assert "player_social_review_spritesheet.png" in output_names
    assert "sandbag_full_review_spritesheet.png" in output_names


@pytest.mark.slow_render
def test_sandbag_full_review_uses_shared_animation_vocabulary(tmp_path):
    job = CharacterJob.load(REVIEW_CONFIGS / "sandbag_full_review.yaml")
    adapter = get_adapter(job.target)
    missing = [name for name in job.animations if name not in adapter.animations()]
    assert missing == []
    for name in [
        "land",
        "roll",
        "slide",
        "pickup",
        "throw",
        "aim",
        "shoot",
        "charge",
        "cast",
        "celebrate",
        "sleep",
        "hover",
        "stomp",
    ]:
        info = adapter.animations()[name]
        img = adapter.render_frame(
            adapter.sample_spec(job), name, info["frames"] // 2, (128, 128), job
        )
        assert img.getchannel("A").getbbox() is not None, name


@pytest.mark.slow_render
def test_ability_item_icons_render(tmp_path):
    from ambition_sprite2d_renderer.targets.icons.item_icons import (
        ICON_SPECS,
        write_item_icons,
    )

    out_dir = tmp_path / "icons"
    outputs = write_item_icons(out_dir)
    output_names = {Path(p).name for p in outputs}
    assert "ability_icon_manifest.yaml" in output_names
    assert "ability_icon_contact_sheet.png" in output_names
    for spec in ICON_SPECS:
        path = out_dir / spec.filename
        assert path.exists(), spec.key
        img = Image.open(path).convert("RGBA")
        assert img.size == (64, 64), spec.key
        assert img.getchannel("A").getbbox() is not None, spec.key


@pytest.mark.slow_render
def test_shrine_prop_renders(tmp_path):
    from ambition_sprite2d_renderer.targets.props import shrine

    paths = shrine.render(tmp_path / "props")
    names = {Path(path).name for path in paths}
    assert "shrine.png" in names
    assert "shrine_spritesheet.png" in names
    assert "shrine_spritesheet.yaml" in names

    manifest = yaml.safe_load((tmp_path / "props" / "shrine_spritesheet.yaml").read_text())
    assert manifest["target"] == "shrine"
    assert [row["animation"] for row in manifest["rows"]] == ["idle", "activate"]

    idle0 = shrine.render_frame("idle", 0, 6)
    idle1 = shrine.render_frame("idle", 1, 6)
    activate0 = shrine.render_frame("activate", 0, 8)
    assert ImageChops.difference(idle0, idle1).getbbox() is not None
    assert ImageChops.difference(idle0, activate0).getbbox() is not None

    path = shrine.write_shrine_prop(tmp_path / "props")
    assert path.name == "shrine.png"
    assert path.exists()
    img = Image.open(path).convert("RGBA")
    # Tall 11:20 prop matching the in-game 44x80 footprint, with drawn content.
    assert img.size == (88, 160)
    assert img.getchannel("A").getbbox() is not None


@pytest.mark.slow_render
def test_review_npc_variants_have_distinct_specs_and_render(tmp_path):
    samples = [
        "goblin_brute_hammer.yaml",
        "goblin_shaman_staff.yaml",
        "goblin_cave_dagger.yaml",
        "goblin_frost_sword.yaml",
        "goblin_desert_bow.yaml",
        "robot_medic.yaml",
        "robot_miner.yaml",
        "robot_archivist.yaml",
    ]
    fingerprints = set()
    for name in samples:
        job = CharacterJob.load(REVIEW_CONFIGS / name)
        adapter = get_adapter(job.target)
        spec = adapter.sample_spec(job)
        data = adapter.spec_dict(spec)
        fingerprints.add(
            (
                job.target,
                data.get("palette_name"),
                data.get("archetype"),
                data.get("held_item"),
            )
        )
        img = adapter.render_frame(spec, "idle", 0, (128, 128), job)
        assert img.getchannel("A").getbbox() is not None, name
    assert len(fingerprints) == len(samples)


@pytest.mark.slow_render
def test_music_faction_lineup_renders_selective_animation_sets(tmp_path):
    from ambition_sprite2d_renderer.authoring.faction_lineup import (
        FactionLineup,
        write_faction_lineup,
    )

    config = CONFIGS / "factions" / "music_factions.yaml"
    lineup = FactionLineup.load(config)
    cue_ids = {f.music_cue for f in lineup.factions}
    assert {
        "first_goblin_tune_v2",
        "long_lofi_drift",
        "pulse_drift_voyage",
        "tech_bros_disruption",
        "dinosaur_liberators",
        "env_advocacy_solace",
        "military_iron_resolve",
        "glasswood_canopy",
    }.issubset(cue_ids)
    outputs = write_faction_lineup(config, tmp_path / "factions")
    output_names = {Path(p).name for p in outputs}
    assert "faction_lineup_manifest.yaml" in output_names
    assert "faction_leaders_contact_sheet.png" in output_names
    assert "goblin_cantina_chieftain_spritesheet.png" in output_names
    assert "lofi_radio_host_spritesheet.png" in output_names
    # The lineup is deliberately selective: no character should inherit the
    # entire full review animation vocabulary by accident.
    for _faction, _character, job in lineup.iter_jobs():
        assert 3 <= len(job.animations) <= 8


def test_music_faction_leader_specs_have_visual_identity_tokens():
    from ambition_sprite2d_renderer.authoring.faction_lineup import FactionLineup

    lineup = FactionLineup.load(CONFIGS / "factions" / "music_factions.yaml")
    identities = {
        (character.target, character.archetype, character.held_item)
        for _faction, character, _job in lineup.iter_jobs()
    }
    assert len(identities) >= 9
    archetypes = {
        character.archetype for _faction, character, _job in lineup.iter_jobs()
    }
    for token in [
        "drift_dj",
        "pulse_captain",
        "tech_disruptor",
        "dinosaur_liberator",
        "iron_marshal",
        "glasswood_warden",
    ]:
        assert token in archetypes
