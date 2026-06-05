"""Tests for the generic floor-snap (`snap_entity_to_surface`) and the
`entity snap-to-floor` CLI added 2026-06-04.

The snap reads a level's Collision IntGrid and drops a rect so its bottom
rests on the first Solid(1) or OneWayUp(2) surface beneath its x-span. The
OneWayUp case is the one that matters in practice: the hub's main floor is
authored as one-way platforms, which `entity measure` (Solid-only) reports
as `down=edge` — so a hand-picked y floats. These tests pin that one-way
platforms count as floor, that the lowest surface wins by default, that
`prefer_y` biases the choice, and that a gap raises rather than floating.
"""

from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.area_authoring import snap_entity_to_surface  # noqa: E402
from ambition_ldtk_tools.edit.snap_to_floor import main as snap_main  # noqa: E402


def _project(
    csv: list[int], c_wid: int, c_hei: int, grid: int = 16, entities=None
) -> dict:
    layers = [
        {
            "__identifier": "Collision",
            "__type": "IntGrid",
            "__cWid": c_wid,
            "__cHei": c_hei,
            "__gridSize": grid,
            "layerDefUid": 1,
            "intGridCsv": csv,
            "entityInstances": [],
        }
    ]
    if entities is not None:
        layers.append(
            {
                "__identifier": "Ambition",
                "__type": "Entities",
                "__cWid": c_wid,
                "__cHei": c_hei,
                "__gridSize": grid,
                "layerDefUid": 2,
                "intGridCsv": [],
                "entityInstances": entities,
            }
        )
    return {
        "iid": "p",
        "jsonVersion": "1.5.3",
        "defaultGridSize": grid,
        "worldLayout": "Free",
        "defs": {"layers": [], "entities": [], "tilesets": [], "enums": []},
        "levels": [
            {
                "identifier": "L",
                "uid": 1,
                "iid": "lvl",
                "worldX": 0,
                "worldY": 0,
                "pxWid": c_wid * grid,
                "pxHei": c_hei * grid,
                "fieldInstances": [],
                "layerInstances": layers,
            }
        ],
    }


def _floor_row(c_wid: int, c_hei: int, row: int, value: int) -> list[int]:
    csv = [0] * (c_wid * c_hei)
    for cx in range(c_wid):
        csv[row * c_wid + cx] = value
    return csv


def test_snaps_to_solid_floor():
    # 4x3 grid, bottom cell row (cy=2, py=32) Solid. A 16x16 rect at x=0
    # lands with its bottom on the floor top -> y = 32 - 16 = 16.
    proj = _project(_floor_row(4, 3, row=2, value=1), 4, 3)
    x, y, kind = snap_entity_to_surface(proj, "L", 0, 16, 16)
    assert (x, y, kind) == (0, 16, "Solid")


def test_treats_oneway_up_as_floor():
    # The hub case: floor authored as OneWayUp (value 2) still snaps.
    proj = _project(_floor_row(4, 3, row=2, value=2), 4, 3)
    x, y, kind = snap_entity_to_surface(proj, "L", 0, 16, 16)
    assert (y, kind) == (16, "OneWayUp")


def test_errors_when_no_floor_under_span():
    # Floor only under cols 0-1; a rect at x=48 (col 3) has nothing below.
    csv = [0] * (4 * 3)
    csv[2 * 4 + 0] = 1
    csv[2 * 4 + 1] = 1
    proj = _project(csv, 4, 3)
    with pytest.raises(SystemExit):
        snap_entity_to_surface(proj, "L", 48, 16, 16)


def test_prefer_y_biases_surface_choice():
    # A ledge (cy=2, py=32) above the ground (cy=6, py=96).
    cw, ch = 4, 7
    csv = [0] * (cw * ch)
    for cx in range(cw):
        csv[2 * cw + cx] = 1
        csv[6 * cw + cx] = 1
    proj = _project(csv, cw, ch)
    # Default: lowest surface wins (ground at py=96 -> y=80).
    _, y_default, _ = snap_entity_to_surface(proj, "L", 0, 16, 16)
    assert y_default == 80
    # prefer_y near the ledge picks the ledge (py=32 -> y=16).
    _, y_pref, _ = snap_entity_to_surface(proj, "L", 0, 16, 16, prefer_y=16)
    assert y_pref == 16


def _run(argv: list[str]) -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = snap_main(argv)
    return rc, buf.getvalue()


def test_cli_dry_run_reports_landing_without_writing():
    sw = {
        "__identifier": "Switch",
        "iid": "sw-x",
        "px": [16, 0],
        "width": 16,
        "height": 16,
        "fieldInstances": [],
    }
    proj = _project(_floor_row(4, 3, row=2, value=1), 4, 3, entities=[sw])
    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "t.ldtk"
        path.write_text(json.dumps(proj))
        rc, out = _run(
            ["--ldtk", str(path), "--level", "L", "--iid", "sw-x", "--dry-run"]
        )
        assert rc == 0, out
        assert "sw-x" in out and "rests on Solid" in out
        # Dry-run must not mutate the file.
        assert json.loads(path.read_text())["levels"][0]["layerInstances"][1][
            "entityInstances"
        ][0]["px"] == [16, 0]


def test_cli_snaps_in_place_with_x_reposition():
    sw = {
        "__identifier": "Switch",
        "iid": "sw-x",
        "px": [99, 0],
        "width": 16,
        "height": 16,
        "fieldInstances": [],
    }
    proj = _project(_floor_row(4, 3, row=2, value=1), 4, 3, entities=[sw])
    with tempfile.TemporaryDirectory() as td:
        path = Path(td) / "t.ldtk"
        path.write_text(json.dumps(proj))
        rc, _ = _run(
            [
                "--ldtk",
                str(path),
                "--level",
                "L",
                "--iid",
                "sw-x",
                "--x",
                "0",
                "--in-place",
                "--no-repair",
            ]
        )
        assert rc == 0
        ent = json.loads(path.read_text())["levels"][0]["layerInstances"][1][
            "entityInstances"
        ][0]
        assert ent["px"] == [0, 16]
        assert ent["__grid"] == [0, 1]
        assert ent["__worldX"] == 0 and ent["__worldY"] == 16
