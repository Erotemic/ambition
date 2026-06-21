"""Tests for the manifest-authored sprite tuning path
(`character_sprites::registry::SheetTuningSpec`).

Per ADR 0017's V3/D4 migration: per-character `collision_scale` and
`frame_sample_inset` (gameplay-tuning fields) should travel through
the renderer's manifest, not stay hand-coded in Rust `*_SHEET` consts.

These tests pin the renderer-side half of the flow without running
the actual sprite rendering pipeline."""

from __future__ import annotations

from ambition_sprite2d_renderer.registry import CharacterJob
from ambition_sprite2d_renderer.authoring.sheet import (
    _adapter_manifest_to_ron,
    _adapter_tuning_to_ron,
)
from ambition_sprite2d_renderer.authoring.tackon_sheet import _ron_tuning


def test_character_job_from_dict_picks_up_sheet_tuning():
    """The `sheet_tuning:` block in `configs/<target>.yaml` ends up
    on `CharacterJob.sheet_tuning`. Without this hop the value never
    reaches the manifest."""
    job = CharacterJob.from_dict(
        {
            "target": "robot",
            "sheet_tuning": {"collision_scale": 2.1, "frame_sample_inset": 1},
        }
    )
    assert job.sheet_tuning == {"collision_scale": 2.1, "frame_sample_inset": 1}


def test_character_job_from_dict_accepts_alias_tuning():
    """Accept both `sheet_tuning:` (canonical) and `tuning:` (shorter)
    in the yaml. The renderer prefers `sheet_tuning:` when both
    are present so the keys can't clash."""
    job = CharacterJob.from_dict(
        {
            "target": "robot",
            "tuning": {"collision_scale": 1.5, "frame_sample_inset": 0},
        }
    )
    assert job.sheet_tuning == {"collision_scale": 1.5, "frame_sample_inset": 0}


def test_character_job_from_dict_no_tuning_leaves_none():
    """The common case for the migration: an existing yaml that
    doesn't carry tuning should produce `sheet_tuning=None`, and the
    Rust runtime falls back to the hardcoded `SheetTuning` const."""
    job = CharacterJob.from_dict({"target": "robot"})
    assert job.sheet_tuning is None


def test_adapter_tuning_to_ron_emits_some_block():
    """The adapter pipeline's RON emitter wraps the tuning in
    `Some((...))` because the Rust field is `Option<SheetTuningSpec>`."""
    out = _adapter_tuning_to_ron({"collision_scale": 2.1, "frame_sample_inset": 1})
    assert "Some((" in out
    assert "collision_scale: 2.1" in out
    assert "frame_sample_inset: 1" in out


def test_adapter_tuning_to_ron_empty_for_none():
    """Missing/None tuning emits no field — the Rust loader's
    `#[serde(default)]` leaves the field `None` and falls back to
    the hardcoded const."""
    assert _adapter_tuning_to_ron(None) == ""
    assert (
        _adapter_tuning_to_ron({})
        == "    tuning: Some((\n        collision_scale: 1.0,\n        frame_sample_inset: 0,\n    )),\n"
    )


def test_tackon_tuning_to_ron_matches_adapter():
    """Both pipelines (tackon + adapter) must produce identical
    `tuning:` blocks so the Rust loader doesn't care which path
    emitted the manifest."""
    tuning = {"collision_scale": 3.0, "frame_sample_inset": 2}
    assert _ron_tuning(tuning) == _adapter_tuning_to_ron(tuning)


def test_adapter_manifest_to_ron_includes_tuning_when_present():
    """End-to-end: a manifest dict with `tuning` becomes RON output
    containing the `tuning: Some(...)` block. Confirms the emitter
    plumbs the field through."""
    manifest = {
        "target": "robot",
        "image": "robot_spritesheet.png",
        "label_width": 100,
        "frame_width": 128,
        "frame_height": 128,
        "animations": {},
        "tuning": {"collision_scale": 2.1, "frame_sample_inset": 1},
    }
    ron_text = _adapter_manifest_to_ron(manifest)
    assert "tuning: Some((" in ron_text
    assert "collision_scale: 2.1" in ron_text


def test_adapter_manifest_to_ron_omits_tuning_when_absent():
    """When `tuning` is not in the manifest, the RON output should
    not contain a `tuning:` line at all — the Rust loader's
    `#[serde(default)]` handles the absence."""
    manifest = {
        "target": "robot",
        "image": "robot_spritesheet.png",
        "label_width": 100,
        "frame_width": 128,
        "frame_height": 128,
        "animations": {},
    }
    ron_text = _adapter_manifest_to_ron(manifest)
    assert "tuning:" not in ron_text
