"""Test the space_debug_labels tool — shifts overlapping DebugLabel
entities down so the debug overlay text is readable.

Mirror of the spawn-overlap validator test pattern: build a minimal
in-memory level, run the shifting function, assert the rects no
longer overlap."""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.edit.space_debug_labels import (  # noqa: E402
    rects_overlap,
    shift_overlapping_labels_in_level,
)


def make_label(iid: str, px: tuple[int, int], size: tuple[int, int], text: str):
    return {
        "__identifier": "DebugLabel",
        "iid": iid,
        "px": list(px),
        "width": size[0],
        "height": size[1],
        "fieldInstances": [{"__identifier": "text", "__value": text}],
    }


def test_overlap_predicate():
    # Basic positive + negative cases.
    assert rects_overlap((0, 0, 10, 10), (5, 5, 10, 10))
    assert rects_overlap((0, 0, 10, 10), (0, 0, 10, 10))
    assert not rects_overlap((0, 0, 10, 10), (20, 0, 10, 10))
    # Edge-touching is not overlap (strict <).
    assert not rects_overlap((0, 0, 10, 10), (10, 0, 10, 10))


def test_shift_overlapping_labels_pushes_second_down():
    level = {
        "identifier": "test_level",
        "worldX": 0,
        "worldY": 0,
        "pxWid": 1024,
        "pxHei": 1024,
        "layerInstances": [
            {
                "__identifier": "Ambition",
                "entityInstances": [
                    make_label("a", (100, 100), (200, 24), "first"),
                    make_label("b", (150, 110), (200, 24), "second_overlaps"),
                ],
            }
        ],
    }
    moved = shift_overlapping_labels_in_level(level)
    assert len(moved) == 1
    label_id, text, old_px, new_px = moved[0]
    assert label_id == "test_level"
    assert "second_overlaps" in text
    assert old_px == (150, 110)
    # Second label should be below the first (100 + 24 + 8 padding = 132).
    assert new_px == (150, 132)


def test_shift_idempotent_after_first_pass():
    # Run twice; the second call should find nothing to move.
    level = {
        "identifier": "idem",
        "worldX": 0,
        "worldY": 0,
        "pxWid": 1024,
        "pxHei": 1024,
        "layerInstances": [
            {
                "__identifier": "Ambition",
                "entityInstances": [
                    make_label("a", (100, 100), (200, 24), "first"),
                    make_label("b", (150, 110), (200, 24), "second"),
                ],
            }
        ],
    }
    first = shift_overlapping_labels_in_level(level)
    second = shift_overlapping_labels_in_level(level)
    assert len(first) == 1
    assert second == []


def test_shift_skips_non_overlapping_pair():
    level = {
        "identifier": "spread",
        "worldX": 0,
        "worldY": 0,
        "pxWid": 1024,
        "pxHei": 1024,
        "layerInstances": [
            {
                "__identifier": "Ambition",
                "entityInstances": [
                    make_label("a", (100, 100), (200, 24), "first"),
                    make_label("b", (100, 500), (200, 24), "far"),
                ],
            }
        ],
    }
    moved = shift_overlapping_labels_in_level(level)
    assert moved == []
