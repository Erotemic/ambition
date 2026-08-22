#!/usr/bin/env python3
"""What `asset editor-art` has to get right for the editor to show the level.

Two things in that tool can be wrong while everything still runs, validates and
writes a plausible-looking `.ldtk` — so they are the two things pinned here.

* The quadrant phase. Engine tile art is 32px and collision is authored on a
  16px grid, so one texture is four cells and four rules, one per quadrant. Get
  a modulo or an offset backwards and the editor still draws masonry — just
  masonry whose mortar lines do not join up, which reads as "the art is a bit
  off" rather than as a bug.
* Where a character's frame IS on its sheet. A published sheet is
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


def test_the_art_goes_on_its_own_layer_so_collision_stays_visible():
    """Editor art must use a separate AutoLayer so collision stays visible."""
    art = editor_art.Placement("solid_tile", x=0, y=0, w=32, h=32)
    collision = {
        "identifier": "Collision",
        "type": "IntGrid",
        "uid": 10,
        "gridSize": 16,
        "intGridValues": [{"identifier": "Solid", "value": 1}],
        "autoRuleGroups": [],
        "tilesetDefUid": None,
        "displayOpacity": 1.0,
        "inactiveOpacity": 0.6,
    }
    project = {"nextUid": 100, "defs": {"layers": [collision]}, "levels": []}

    editor_art.apply_auto_rules(project, 7, {"Solid": "solid_tile"}, {"solid_tile": art})

    assert collision["autoRuleGroups"] == [], "the collision layer keeps drawing its cells"
    assert collision["tilesetDefUid"] is None
    assert collision["inactiveOpacity"] < 0.6, "it fades so the art below shows through"

    layers = project["defs"]["layers"]
    art_layer = next(l for l in layers if l["identifier"] == "CollisionArt")
    assert art_layer["type"] == "AutoLayer"
    assert art_layer["autoSourceLayerDefUid"] == collision["uid"], "it reads the collision"
    assert layers.index(art_layer) > layers.index(collision), "and draws behind it"
    assert len(art_layer["autoRuleGroups"][0]["rules"]) == 4


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


def test_closing_a_string_into_an_enum_refuses_a_value_it_cannot_spell():
    """The dropdown may only land if every authored word survives it.

    Turning a free-text field into an enum is what lets a placement's art follow
    its own value — but an enum holds only what it spells, so the one that
    matters is the check that a value already in the level is not about to
    become unsayable.
    """
    from ambition_ldtk_tools.edit.upsert_entity import _stale_field_names

    def project_with(kind: str) -> dict:
        return {
            "defs": {
                "entities": [
                    {
                        "identifier": "MaryOBlock",
                        "fieldDefs": [{"identifier": "kind", "__type": "String"}],
                    }
                ]
            },
            "levels": [
                {
                    "layerInstances": [
                        {
                            "entityInstances": [
                                {
                                    "__identifier": "MaryOBlock",
                                    "fieldInstances": [
                                        {"__identifier": "kind", "__value": kind}
                                    ],
                                }
                            ]
                        }
                    ]
                }
            ],
        }

    spec = {
        "identifier": "MaryOBlock",
        "fields": [
            {
                "name": "kind",
                "type": "Enum",
                "enum": "MaryOBlockKind",
                "values": ["Question", "Brick"],
            }
        ],
    }

    assert _stale_field_names(project_with("Brick"), spec) == {}
    refused = _stale_field_names(project_with("Quasar"), spec)
    assert "kind" in refused and "'Quasar'" in refused["kind"]


def test_the_art_layer_arrives_with_its_tiles_already_in_it():
    """Generated AutoLayers must include their `autoLayerTiles` cache.

    LDtk does not re-evaluate rules when merely opening the file, and the cache
    is a pure function of the authored cells.
    """
    art = editor_art.Placement("solid_tile", x=0, y=0, w=32, h=32)
    collision = {
        "identifier": "Collision",
        "type": "IntGrid",
        "uid": 10,
        "gridSize": 16,
        "intGridValues": [{"identifier": "Solid", "value": 1}],
        "autoRuleGroups": [],
        "tilesetDefUid": None,
        "displayOpacity": 1.0,
        "inactiveOpacity": 0.6,
    }
    level = {
        "uid": 1,
        "layerInstances": [
            {
                "layerDefUid": 10,
                "__cWid": 2,
                "__cHei": 2,
                # one solid cell, bottom-left
                "intGridCsv": [0, 0, 1, 0],
            }
        ],
    }
    project = {
        "nextUid": 100,
        "defs": {
            "layers": [collision],
            "tilesets": [{"uid": 7, "tileGridSize": 16, "__cWid": editor_art.ATLAS_COLS}],
        },
        "levels": [level],
    }

    editor_art.apply_auto_rules(project, 7, {"Solid": "solid_tile"}, {"solid_tile": art})

    art_layer = next(
        layer for layer in project["defs"]["layers"] if layer["identifier"] == "CollisionArt"
    )
    baked = next(
        inst
        for inst in level["layerInstances"]
        if inst["layerDefUid"] == art_layer["uid"]
    )["autoLayerTiles"]
    assert len(baked) == 1, "the one painted cell is the one baked tile"
    assert baked[0]["px"] == [0, 16], "at the cell it came from"
    # cell (0,1) is the texture's BOTTOM-left quadrant, one atlas row down.
    assert baked[0]["src"] == [0, 16]
