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
    assert 'missing_information' in text
    assert 'socket hand_r: absent' in text


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
