import pytest

# Full-resolution render path; opt in with --run-slow-render (see GOALS.md).
pytestmark = pytest.mark.slow_render

from pathlib import Path

from ambition_sprite2d_renderer.authoring.adapters import get_adapter
from ambition_sprite2d_renderer.cli import draw_review
from ambition_sprite2d_renderer.registry import CharacterJob


REVIEW_DIR = (
    Path(__file__).resolve().parents[1]
    / "ambition_sprite2d_renderer"
    / "configs"
    / "review"
)


def test_review_configs_render(tmp_path):
    out_dir = tmp_path / "review"
    outputs = draw_review(REVIEW_DIR, out_dir)
    pngs = sorted(out_dir.rglob("*.png"))
    assert outputs
    assert len(pngs) >= 13  # 6 sheets + 6 canonicals + contact sheet
    assert (out_dir / "general_hero_spritesheet.png").exists()
    assert (out_dir / "absurd_general_spritesheet.png").exists()
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


def test_absurd_general_has_shouting_trope_spec(tmp_path):
    job = CharacterJob.load(REVIEW_DIR / "absurd_general.yaml")
    adapter = get_adapter(job.target)
    spec = adapter.sample_spec(job)
    assert spec.archetype == "absurd_general"
    assert spec.outfit == "general_uniform"
    assert spec.hair_style == "general_hat"
    assert spec.accessory == "medals"
    assert spec.prop == "baton"
    img = adapter.render_single(spec, "talk", 2, job)
    out = tmp_path / "absurd_general_talk.png"
    img.save(out)
    assert out.exists()
    assert img.getchannel("A").getbbox() is not None


def test_oiler_and_erdish_review_specs_render(tmp_path):
    adapter = get_adapter("toon")
    # Source of truth: the `oiler` / `erdish` entries in
    # ToonSideGenerator.PRESETS. The Oiler preset deliberately uses
    # `scholar_queue` (powdered-wig silhouette) so the silhouette reads
    # "Euler with tools" rather than generic mechanic.
    checks = {
        "oiler.yaml": ("Oiler", "banyan", "savant_cap", "wrench", "satchel"),
        "erdish.yaml": (
            "Erdish",
            "long_coat",
            "combed_back_balding",
            "tablet",
            "satchel",
        ),
    }
    for filename, (name, outfit, hair_style, prop, accessory) in checks.items():
        job = CharacterJob.load(REVIEW_DIR / filename)
        spec = adapter.sample_spec(job)
        assert spec.name == name
        assert spec.outfit == outfit
        assert spec.hair_style == hair_style
        assert spec.prop == prop
        assert spec.accessory == accessory
        img = adapter.render_single(spec, "idle", 1, job)
        out = tmp_path / f"{job.output_name}.png"
        img.save(out)
        assert out.exists()
        assert img.getchannel("A").getbbox() is not None
