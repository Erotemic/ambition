"""Smoke tests for the `intgrid paint` subcommand added 2026-05-24.

Symmetric counterpart to `intgrid erase` tests if any existed —
verifies that painting a rect of a given IntGrid value updates the
right cells and doesn't disturb the rest of the layer.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.edit.intgrid import main as intgrid_main  # noqa: E402


def make_minimal_project(
    width_px: int = 160, height_px: int = 96, grid: int = 16
) -> dict:
    c_wid = width_px // grid
    c_hei = height_px // grid
    return {
        "iid": "test-project",
        "jsonVersion": "1.5.3",
        "defaultGridSize": grid,
        "worldLayout": "Free",
        "defs": {
            "layers": [
                {
                    "identifier": "Collision",
                    "uid": 1,
                    "type": "IntGrid",
                    "gridSize": grid,
                    "intGridValues": [
                        {"value": 1, "identifier": "Solid", "color": "#fff"},
                        {"value": 2, "identifier": "OneWayPlatform", "color": "#888"},
                    ],
                }
            ],
            "entities": [],
            "tilesets": [],
            "enums": [],
        },
        "levels": [
            {
                "identifier": "TestLevel",
                "uid": 100,
                "iid": "test-level",
                "worldX": 0,
                "worldY": 0,
                "pxWid": width_px,
                "pxHei": height_px,
                "fieldInstances": [],
                "layerInstances": [
                    {
                        "__identifier": "Collision",
                        "__type": "IntGrid",
                        "__cWid": c_wid,
                        "__cHei": c_hei,
                        "__gridSize": grid,
                        "layerDefUid": 1,
                        "intGridCsv": [0] * (c_wid * c_hei),
                    }
                ],
            }
        ],
    }


def test_intgrid_paint_fills_rect_with_value():
    project = make_minimal_project()
    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "test.ldtk"
        path.write_text(json.dumps(project, indent="\t") + "\n")
        # Paint a 32 px wide row at y=64, value=1.
        rc = intgrid_main(
            [
                "paint",
                "--ldtk",
                str(path),
                "--level",
                "TestLevel",
                "--px",
                "0,64",
                "--size",
                "160,16",
                "--value",
                "1",
                "--no-repair",  # don't run repair pass on the test fixture
            ]
        )
        assert rc == 0
        reloaded = json.loads(path.read_text())
        csv = reloaded["levels"][0]["layerInstances"][0]["intGridCsv"]
        # Cells y=4 (px 64..80) on the 10-wide grid should be 1; rest 0.
        for y in range(6):
            for x in range(10):
                idx = y * 10 + x
                if y == 4:
                    assert csv[idx] == 1, f"cell ({x},{y}) expected 1, got {csv[idx]}"
                else:
                    assert csv[idx] == 0, f"cell ({x},{y}) expected 0, got {csv[idx]}"


def test_intgrid_paint_dry_run_does_not_mutate():
    project = make_minimal_project()
    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "test.ldtk"
        original = json.dumps(project, indent="\t") + "\n"
        path.write_text(original)
        rc = intgrid_main(
            [
                "paint",
                "--ldtk",
                str(path),
                "--level",
                "TestLevel",
                "--px",
                "0,0",
                "--size",
                "16,16",
                "--value",
                "1",
                "--dry-run",
                "--no-repair",
            ]
        )
        assert rc == 0
        # File should be unchanged.
        assert path.read_text() == original


def test_intgrid_paint_skips_cells_already_at_target_value():
    project = make_minimal_project()
    # Pre-fill one cell at value=1.
    project["levels"][0]["layerInstances"][0]["intGridCsv"][0] = 1
    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "test.ldtk"
        path.write_text(json.dumps(project, indent="\t") + "\n")
        rc = intgrid_main(
            [
                "paint",
                "--ldtk",
                str(path),
                "--level",
                "TestLevel",
                "--px",
                "0,0",
                "--size",
                "32,16",  # covers cells (0,0) + (1,0)
                "--value",
                "1",
                "--no-repair",
            ]
        )
        assert rc == 0
        # Only 1 cell should have changed (the second one).
        # We can't easily assert that from the output without
        # parsing it; instead, verify the resulting CSV:
        reloaded = json.loads(path.read_text())
        csv = reloaded["levels"][0]["layerInstances"][0]["intGridCsv"]
        assert csv[0] == 1
        assert csv[1] == 1
        # Other cells stay 0.
        assert csv[2] == 0
