#!/usr/bin/env python3
"""What `asset editor-art` has to get right for the editor to show the level.

Two things in that tool can be wrong while everything still runs, validates and
writes a plausible-looking `.ldtk` — so they are the two things pinned here.

* **The quadrant phase.** Engine tile art is 32px and collision is authored on a
  16px grid, so one texture is four cells and four rules, one per quadrant. Get
  a modulo or an offset backwards and the editor still draws masonry — just
  masonry whose mortar lines do not join up, which reads as "the art is a bit
  off" rather than as a bug.
* **Where a character's frame IS on its sheet.** A published sheet is
  atlas-packed, so `frame_width` is the DESIGN size and not the packing pitch.
  Multiplying an index by it lands on some other frame, which is exactly the
  bug that shipped a strip of three overlapping robots as the player's icon.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.edit import editor_art  # noqa: E402


def test_a_32px_texture_is_reassembled_from_its_four_quadrants(tmp_path):
    """The rules put the texture back together, cell by cell.

    Reads the rules the way an evaluator does — match a cell by
    `(cx - xOffset) % xModulo`, take the tile the rule names — and checks that a
    2×2 block of cells receives the four quarters of the source image in the
    order they appear in it.
    """
    art = editor_art.Placement("solid_tile", x=0, y=0, w=32, h=32)
    layer = {
        "identifier": "Collision",
        "type": "IntGrid",
        "intGridValues": [{"identifier": "Solid", "value": 1}],
    }
    project = {"nextUid": 1, "defs": {}}

    rules = editor_art.auto_rules_for_layer(
        layer, {"Solid": "solid_tile"}, {"solid_tile": art}, project
    )
    assert len(rules) == 4, "a 32×32 texture on a 16px grid is four cells"

    drawn = {}
    for cell_y in range(4):
        for cell_x in range(4):
            for rule in rules:
                x_ok = (cell_x - rule["xOffset"]) % rule["xModulo"] == 0
                y_ok = (cell_y - rule["yOffset"]) % rule["yModulo"] == 0
                if x_ok and y_ok:
                    drawn[(cell_x, cell_y)] = rule["tileRectsIds"][0][0]
                    break

    assert len(drawn) == 16, "every solid cell is claimed by exactly one rule"
    # Tile ids run left-to-right across the atlas, which is ATLAS_COLS wide.
    top_left, top_right = 0, 1
    bottom_left, bottom_right = editor_art.ATLAS_COLS, editor_art.ATLAS_COLS + 1
    assert drawn[(0, 0)] == top_left
    assert drawn[(1, 0)] == top_right
    assert drawn[(0, 1)] == bottom_left
    assert drawn[(1, 1)] == bottom_right
    # ...and it repeats, so the texture tiles across the whole surface.
    assert drawn[(2, 2)] == top_left
    assert drawn[(3, 2)] == top_right


def test_a_sheet_frame_comes_from_the_sidecar_rects(tmp_path):
    """A frame's rect is read, never computed from `frame_width`."""
    (tmp_path / "hero_spritesheet.yaml").write_text(
        "target: hero\n"
        "frame_width: 224\n"
        "frame_height: 224\n"
        "rows:\n"
        "- animation: idle\n"
        "  rects:\n"
        "  - {x: 1390, y: 1, w: 71, h: 101}\n"
        "  - {x: 1455, y: 104, w: 71, h: 101}\n"
    )

    assert editor_art.sheet_frame_rect("hero", tmp_path, "idle", 0) == (1390, 1, 71, 101)
    assert editor_art.sheet_frame_rect("hero", tmp_path, "idle", 1) == (1455, 104, 71, 101)

    with pytest.raises(SystemExit, match="2 frames"):
        editor_art.sheet_frame_rect("hero", tmp_path, "idle", 7)
    with pytest.raises(SystemExit, match="no 'run' animation"):
        editor_art.sheet_frame_rect("hero", tmp_path, "run", 0)
