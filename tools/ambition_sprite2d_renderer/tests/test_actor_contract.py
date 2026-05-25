from pathlib import Path

from ambition_sprite2d_renderer.config import CharacterJob
from ambition_sprite2d_renderer.sheet import write_spritesheet
from ambition_sprite2d_renderer.target_registry import discover_all_targets


CONFIGS = Path(__file__).resolve().parents[1] / "ambition_sprite2d_renderer" / "configs"


def test_write_spritesheet_emits_optional_actor_contract(tmp_path: Path):
    job = CharacterJob.load(CONFIGS / "goblin.yaml")
    job.render.frame_width = 64
    job.render.frame_height = 64
    job.render.supersample = 1
    job.animations = ["idle", "walk", "slash"]
    image_path, manifest_path = write_spritesheet(
        job,
        tmp_path / "goblin_spritesheet.png",
        tmp_path / "goblin_spritesheet.yaml",
    )
    actor_path = tmp_path / "goblin_actor.ron"
    assert image_path.exists()
    assert manifest_path.exists()
    assert actor_path.exists()
    text = actor_path.read_text()
    assert 'schema_version: 1' in text
    assert 'character_id: "goblin"' in text
    assert '"action.melee.primary"' in text
    assert 'missing_information: []' in text
    assert 'collision: Some' in text
    assert 'hurtbox: Some' in text
    assert '"hand_r"' in text
    assert '"weapon_tip"' in text


def test_character_job_accepts_sparse_actor_contract_fields():
    job = CharacterJob.from_dict(
        {
            "target": "toon",
            "name": "Sparse Zombie",
            "output_name": "zombie_shambler",
            "actor": {"character_id": "npc_zombie_shambler"},
            "body": {"body_plan": "HumanoidBiped", "traits": ["undead", "no_hands"]},
            "capabilities": {"traversal": {"walk": True, "jump": None}},
            "brain": {"default_preset": "melee_brute_slow"},
            "actions": {"default_preset": "zombie_bite"},
            "sockets": {"mouth": {"point": {"x": 20.0, "y": 30.0}}},
        }
    )
    assert job.actor["character_id"] == "npc_zombie_shambler"
    assert job.body["traits"] == ["undead", "no_hands"]
    assert "mouth" in job.sockets


def test_contract_derives_runtime_fields_from_body_metrics_without_requiring_hands():
    from ambition_sprite2d_renderer.actor_contract import build_actor_contract, to_ron

    manifest = {
        "target": "toon",
        "image": "zombie_shambler_spritesheet.png",
        "body_metrics": {
            "body_pixel_bbox": {"x": 5, "y": 10, "w": 30, "h": 50},
            "feet_pixel": {"x": 20, "y": 63},
        },
        "rows": [
            {"animation": "shamble_idle", "row_index": 0, "frame_count": 1, "duration_ms": 100, "rects": []},
            {"animation": "bite", "row_index": 1, "frame_count": 1, "duration_ms": 100, "rects": []},
        ],
    }
    ron = to_ron(build_actor_contract(
        stem="zombie_shambler",
        target="toon",
        image="zombie_shambler_spritesheet.png",
        sheet_manifest="zombie_shambler_spritesheet.ron",
        manifest=manifest,
        job_data={"surface": "adapter", "tags": []},
        authoring={
            "body": {"body_plan": "HumanoidBiped", "traits": ["undead", "no_hands"]},
            "actions": {"default_preset": "zombie_bite"},
            "sockets": {"mouth": {"point": {"x": 22.0, "y": 26.0}}},
        },
    ))
    assert 'collision: Some' in ron
    assert 'hurtbox: Some' in ron
    assert '"mouth"' in ron
    assert '"hand_r"' not in ron
    assert 'melee origin socket' not in ron


def test_tackon_target_render_sheet_includes_actor_sidecar(tmp_path: Path):
    targets = discover_all_targets().targets
    target = targets["sandbag"]
    outputs = target.render_sheet(tmp_path)
    actor_path = tmp_path / "sandbag_actor.ron"
    assert actor_path.exists()
    assert actor_path in outputs
    text = actor_path.read_text()
    assert 'character_id: "sandbag"' in text
    assert 'default_preset: Some("sandbag_punch")' in text


def test_catalog_defaults_enrich_actor_contract_when_available(tmp_path: Path):
    from ambition_sprite2d_renderer.actor_contract import build_actor_contract, to_ron

    manifest = {
        "target": "toon",
        "image": "erdish_spritesheet.png",
        "rows": [
            {"animation": "idle", "row_index": 0, "frame_count": 1, "duration_ms": 100, "rects": []},
            {"animation": "walk", "row_index": 1, "frame_count": 1, "duration_ms": 100, "rects": []},
        ],
    }
    ron = to_ron(build_actor_contract(
        stem="erdish",
        target="toon",
        image="erdish_spritesheet.png",
        sheet_manifest="erdish_spritesheet.ron",
        manifest=manifest,
        job_data={"surface": "adapter", "tags": []},
        authoring={},
    ))
    assert 'character_id: "npc_erdish"' in ron
    assert 'display_name: Some("Erdish")' in ron
    assert 'default_preset: Some("patrol_peaceful")' in ron
    assert 'default_preset: Some("peaceful")' in ron


def test_json_manifest_targets_can_emit_actor_sidecar(tmp_path: Path):
    from ambition_sprite2d_renderer.target_registry import _ensure_actor_sidecars

    manifest_path = tmp_path / "mockingbird_boss_spritesheet_manifest.json"
    manifest_path.write_text(
        '{"target":"mockingbird_boss","rows":[{"name":"hover","frames":2,"duration_ms":120}]}',
        encoding="utf8",
    )
    outputs = _ensure_actor_sidecars(
        target_name="mockingbird_boss",
        render_dir=tmp_path,
        paths=[manifest_path],
        actor_metadata={"actor": {"character_id": "npc_mockingbird_boss"}, "tags": ["boss"]},
    )
    actor_path = tmp_path / "mockingbird_boss_actor.ron"
    assert actor_path.exists()
    assert actor_path in outputs
    text = actor_path.read_text(encoding="utf8")
    assert 'character_id: "npc_mockingbird_boss"' in text
    assert '"locomotion.hover"' in text


def test_every_registered_character_target_advertises_actor_sidecar():
    targets = discover_all_targets().targets
    missing = []
    for name, target in targets.items():
        if target.category != "characters":
            continue
        if f"{name}_actor.ron" not in target.sheet_files:
            missing.append(name)
    assert missing == []
