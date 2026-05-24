"""Smoke tests for `synth_boss_manifest` — the one-shot that
converts boss JSON spritesheet manifests into runtime-compatible
RON `SheetRecord` files.

Pin three invariants:
  - The output is a `Vec<SheetRecord>`-shaped list (wraps the
    record in `[ ... ]`).
  - The first row's animation is renamed to `rest` when the source
    name isn't already an Idle-alias (covers the mockingbird
    `hover`-as-idle case).
  - Per-frame `rects` are grid-derived from frame_width/height +
    row_index when the source JSON doesn't supply them.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.synth_boss_manifest import (  # noqa: E402
    normalize_row,
    synthesize,
)
from ambition_ldtk_tools.ron_parse import load as ron_load  # noqa: E402


def test_normalize_row_aliases_first_row_to_rest():
    """If the source row isn't already an Idle alias and it's the
    first row, rename to `rest`."""
    out = normalize_row(
        {"animation": "hover", "row_index": 0, "frames": 6, "duration_ms": 110},
        frame_w=576,
        frame_h=216,
        is_first_row=True,
    )
    assert out["animation"] == "rest", \
        f"first non-idle row should rename to 'rest'; got {out['animation']!r}"


def test_normalize_row_preserves_existing_idle_name():
    out = normalize_row(
        {"animation": "idle", "row_index": 0, "frames": 4, "duration_ms": 130},
        frame_w=128,
        frame_h=128,
        is_first_row=True,
    )
    assert out["animation"] == "idle"


def test_normalize_row_preserves_non_first_row_names():
    out = normalize_row(
        {"animation": "attack", "row_index": 2, "frames": 4, "duration_ms": 90},
        frame_w=128,
        frame_h=128,
        is_first_row=False,
    )
    assert out["animation"] == "attack"


def test_normalize_row_derives_rects_from_frame_size():
    out = normalize_row(
        {"animation": "rest", "row_index": 0, "frames": 3, "duration_ms": 100},
        frame_w=100,
        frame_h=50,
        is_first_row=True,
    )
    assert out["rects"] == [
        {"x": 0, "y": 0, "w": 100, "h": 50, "anchors": {}},
        {"x": 100, "y": 0, "w": 100, "h": 50, "anchors": {}},
        {"x": 200, "y": 0, "w": 100, "h": 50, "anchors": {}},
    ]


def test_synthesize_emits_vec_sheetrecord():
    json_manifest = {
        "target": "test_boss",
        "frame_size": [256, 128],
        "rows": [
            {"name": "hover", "row": 0, "frames": 4, "duration_ms": 100},
            {"name": "attack", "row": 1, "frames": 6, "duration_ms": 80},
        ],
    }
    with tempfile.TemporaryDirectory() as td:
        json_path = Path(td) / "test_boss_spritesheet_manifest.json"
        json_path.write_text(json.dumps(json_manifest))
        out_path = synthesize(json_path)
        assert out_path.name == "test_boss_spritesheet.ron"
        data = ron_load(out_path.read_text())
        # Vec<SheetRecord> shape — top-level is a list.
        assert isinstance(data, list), f"output must be a list; got {type(data).__name__}"
        assert len(data) == 1
        record = data[0]
        assert record["target"] == "test_boss"
        assert record["frame_width"] == 256
        assert record["frame_height"] == 128
        # First row aliased to `rest`.
        assert record["rows"][0]["animation"] == "rest"
        assert record["rows"][1]["animation"] == "attack"
