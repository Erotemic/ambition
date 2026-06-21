"""Per-animation hurtbox/hitbox publishing for the tack-on `build_sheet` path.

Bosses look animation boxes up by a GENERIC gameplay key (``rest`` /
``side_sweep`` / ...), not the sheet's own row names. A tack-on boss
(the flying spaghetti monster) therefore has to remap its rows to those
keys via ``animation_key_map`` so the published per-pose hurtboxes are
actually consumed instead of falling back to the coarse idle bbox."""

from __future__ import annotations

import tempfile
from pathlib import Path

from PIL import Image

from ambition_sprite2d_renderer.authoring.tackon_sheet import build_sheet


def _solid(_anim, _frame_idx, _nframes):
    # A small opaque square inset into a 64x64 frame so the union alpha
    # bbox is a stable, non-degenerate rectangle.
    img = Image.new("RGBA", (64, 64), (0, 0, 0, 0))
    for y in range(10, 40):
        for x in range(12, 50):
            img.putpixel((x, y), (255, 0, 0, 255))
    return img


def test_animation_key_map_publishes_remapped_hurtboxes_and_hitboxes():
    rows = [("idle", 2, 100), ("whip", 2, 100)]
    with tempfile.TemporaryDirectory() as d:
        outputs = build_sheet(
            target="probe",
            rows=rows,
            render_fn=_solid,
            out_dir=Path(d),
            frame_size=(64, 64),
            auto_crop=False,  # keep frame coords predictable for the assertion
            animation_key_map={"idle": "rest", "whip": "side_sweep"},
            attack_hitboxes={
                "side_sweep": {"bbox": {"x": 5, "y": 6, "w": 7, "h": 8}}
            },
        )
        ron = Path(outputs["ron"]).read_text()

    # Remapped to the gameplay keys, NOT the row names.
    assert '"rest": (' in ron
    assert '"side_sweep": (' in ron
    assert '"idle":' not in ron.split("animations:")[1]
    assert '"whip":' not in ron.split("animations:")[1]
    # Idle published a hurtbox; the attack published BOTH a hurtbox (auto from
    # the drawn body) and the authored hitbox.
    assert ron.count("hurtbox") == 2
    assert "hitbox: Some((bbox: Some((x: 5, y: 6, w: 7, h: 8)))" in ron


def test_no_key_map_leaves_body_metrics_single_bbox():
    """Sheets that don't opt in keep the legacy single-bbox body_metrics
    (no `animations` block) so existing tack-on generators are unchanged."""
    rows = [("idle", 2, 100)]
    with tempfile.TemporaryDirectory() as d:
        outputs = build_sheet(
            target="probe2",
            rows=rows,
            render_fn=_solid,
            out_dir=Path(d),
            frame_size=(64, 64),
            auto_crop=False,
        )
        ron = Path(outputs["ron"]).read_text()
    assert "animations:" not in ron
