from pathlib import Path

from ambition_sprite2d_renderer.adapters import get_adapter
from ambition_sprite2d_renderer.cli import draw_review
from ambition_sprite2d_renderer.config import CharacterJob


REVIEW_DIR = Path(__file__).resolve().parents[1] / "ambition_sprite2d_renderer" / "configs" / "review"



def test_review_configs_render(tmp_path):
    out_dir = tmp_path / "review"
    outputs = draw_review(REVIEW_DIR, out_dir)
    pngs = sorted(out_dir.rglob("*.png"))
    assert outputs
    assert len(pngs) >= 11  # 5 sheets + 5 canonicals + contact sheet
    assert (out_dir / "general_hero_spritesheet.png").exists()
    assert (out_dir / "canonicals" / "canonicals_contact_sheet.png").exists()



def test_toon_target_supports_overrides(tmp_path):
    job = CharacterJob.from_dict(
        {
            "target": "toon",
            "name": "Override Tester",
            "archetype": "architect",
            "animations": ["idle"],
            "spec": {
                "torso_w": 25.0,
                "torso_h": 33.0,
                "outfit": "long_coat",
            },
            "render": {
                "single_width": 96,
                "single_height": 96,
                "supersample": 2,
            },
        }
    )
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    assert spec.name == "Override Tester"
    assert spec.torso_w == 25.0
    assert spec.torso_h == 33.0
    img = adapter.render_single(spec, "idle", 0, job)
    out = tmp_path / "override.png"
    img.save(out)
    assert out.exists()
    assert img.size == (96, 96)
