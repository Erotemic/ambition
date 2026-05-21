#!/usr/bin/env python3
"""Tests for `ambition_ldtk_tools.edit.spec_diff`.

The spec-diff tool reports drift between area spec YAMLs and the
live LDtk file. These tests build minimal in-memory inputs and
exercise `diff_one` directly so the tool can be regression-pinned
without relying on the actual repo's specs (which evolve).
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"
sys.path.insert(0, str(PKG_ROOT))

from ambition_ldtk_tools.edit.spec_diff import diff_one  # noqa: E402


def _level(identifier: str, x: int, y: int, w: int, h: int) -> dict:
    return {"identifier": identifier, "worldX": x, "worldY": y, "pxWid": w, "pxHei": h}


def test_clean_match_returns_no_diff() -> None:
    spec = {
        "level_id": "intro_wake_room",
        "world_x": 0,
        "world_y": 0,
        "px_wid": 1024,
        "px_hei": 384,
    }
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 0, 1024, 384)}
    matches, diffs = diff_one(spec, levels)
    assert matches is True
    assert diffs == []


def test_world_x_drift_is_reported() -> None:
    spec = {
        "level_id": "intro_wake_room",
        "world_x": 100000,  # stale
        "world_y": 0,
        "px_wid": 1024,
        "px_hei": 384,
    }
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 0, 1024, 384)}
    matches, diffs = diff_one(spec, levels)
    assert matches is False
    assert len(diffs) == 1
    assert "world_x" in diffs[0]
    assert "100000" in diffs[0]
    assert "live=0" in diffs[0]


def test_multiple_drifts_all_reported() -> None:
    spec = {
        "level_id": "drain_alley",
        "world_x": 106000,
        "world_y": 0,
        "px_wid": 1024,
        "px_hei": 512,  # stale: live is 1024 now
    }
    levels = {"drain_alley": _level("drain_alley", 3904, 0, 1024, 1024)}
    matches, diffs = diff_one(spec, levels)
    assert matches is False
    assert len(diffs) == 2
    fields = " ".join(diffs)
    assert "world_x" in fields
    assert "px_hei" in fields


def test_non_area_spec_skipped_quietly() -> None:
    # An entity-add / door spec has level_id but no world_x / px_wid
    # fields; diff_one returns (True, None) so the CLI prints SKIP.
    spec = {
        "level_id": "central_hub_main",
        "px": [908, 624],
        "size": [48, 96],
    }
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 0, 1024, 384)}
    matches, diffs = diff_one(spec, levels)
    assert matches is True
    assert diffs is None


def test_area_spec_with_missing_level_id_fails() -> None:
    # Area-spec coords present but no level_id / id — can't match.
    spec = {"world_x": 0, "world_y": 0, "px_wid": 1024, "px_hei": 384}
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 0, 1024, 384)}
    matches, diffs = diff_one(spec, levels)
    assert matches is False
    assert any("no `level_id`" in d for d in (diffs or []))


def test_area_spec_with_unknown_level_id_fails() -> None:
    spec = {
        "level_id": "imaginary_room",
        "world_x": 0,
        "world_y": 0,
        "px_wid": 1024,
        "px_hei": 384,
    }
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 0, 1024, 384)}
    matches, diffs = diff_one(spec, levels)
    assert matches is False
    assert any("imaginary_room" in d for d in (diffs or []))


def test_partial_spec_compares_only_present_fields() -> None:
    # If spec only sets world_x (no px_wid), only that field is checked.
    spec = {"level_id": "intro_wake_room", "world_x": 0}
    levels = {"intro_wake_room": _level("intro_wake_room", 0, 9999, 9999, 9999)}
    matches, diffs = diff_one(spec, levels)
    assert matches is True
    assert diffs == []
