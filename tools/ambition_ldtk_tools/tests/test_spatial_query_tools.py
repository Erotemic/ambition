"""Smoke tests for the read-only spatial-query subcommands added 2026-06-02.

`intgrid query`, `entity measure`, and `gates audit` answer the placement
questions an LLM map author has before adding a gate (see
docs/concepts/llm-spatial-authoring-discipline.md). They never mutate the
project, so these tests build a tiny in-memory LDtk file and assert on the
stdout each command prints.
"""
from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(REPO_ROOT / "tools" / "ambition_ldtk_tools"))

from ambition_ldtk_tools.edit.gates import main as gates_main  # noqa: E402
from ambition_ldtk_tools.edit.intgrid import main as intgrid_main  # noqa: E402
from ambition_ldtk_tools.edit.measure import main as measure_main  # noqa: E402


def _entity(ident: str, iid: str, x: int, y: int, w: int, h: int, fields: dict) -> dict:
    return {
        "__identifier": ident,
        "iid": iid,
        "px": [x, y],
        "width": w,
        "height": h,
        "fieldInstances": [
            {"__identifier": k, "__value": v} for k, v in fields.items()
        ],
    }


def make_project(grid: int = 16) -> dict:
    """160x96 px room: a Solid floor on the bottom cell row + a few entities."""
    c_wid, c_hei = 10, 6
    csv = [0] * (c_wid * c_hei)
    for x in range(c_wid):  # bottom row (cell y=5, px 80..96) is Solid
        csv[5 * c_wid + x] = 1
    return {
        "iid": "test-project",
        "jsonVersion": "1.5.3",
        "defaultGridSize": grid,
        "worldLayout": "Free",
        "defs": {"layers": [], "entities": [], "tilesets": [], "enums": []},
        "levels": [
            {
                "identifier": "TestLevel",
                "uid": 100,
                "iid": "test-level",
                "worldX": 0,
                "worldY": 0,
                "pxWid": 160,
                "pxHei": 96,
                "fieldInstances": [],
                "layerInstances": [
                    {
                        "__identifier": "Collision",
                        "__type": "IntGrid",
                        "__cWid": c_wid,
                        "__cHei": c_hei,
                        "__gridSize": grid,
                        "layerDefUid": 1,
                        "intGridCsv": csv,
                        "entityInstances": [],
                    },
                    {
                        "__identifier": "Entities",
                        "__type": "Entities",
                        "__cWid": c_wid,
                        "__cHei": c_hei,
                        "__gridSize": grid,
                        "layerDefUid": 2,
                        "intGridCsv": [],
                        "entityInstances": [
                            _entity(
                                "Switch", "sw-1", 32, 16, 16, 16,
                                {
                                    "id": "test_switch",
                                    "action": "ResetEncounter",
                                    "target_encounter": "test_enc",
                                    "prompt": "Flip the gate",
                                },
                            ),
                            _entity(
                                "Switch", "sw-2", 64, 16, 16, 16,
                                {
                                    "id": "spout_switch",
                                    "action": "ResetEncounter",
                                    "target_encounter": "",
                                    "prompt": "Toggle spout",
                                },
                            ),
                            _entity(
                                "LockWall", "lw-1", 80, 0, 16, 80,
                                {"id": "test_enc_lock"},
                            ),
                            _entity(
                                "EncounterTrigger", "et-1", 100, 0, 32, 80,
                                {"id": "test_enc"},
                            ),
                            _entity("BreakablePlatform", "bp-1", 0, 0, 16, 16, {}),
                        ],
                    },
                ],
            }
        ],
    }


def _run(fn, argv: list[str]) -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = fn(argv)
    return rc, buf.getvalue()


def _write_project() -> tuple[tempfile.TemporaryDirectory, Path]:
    td = tempfile.TemporaryDirectory()
    path = Path(td.name) / "test.ldtk"
    path.write_text(json.dumps(make_project(), indent="\t") + "\n")
    return td, path


def test_intgrid_query_reports_solid_in_rect():
    td, path = _write_project()
    with td:
        # The bottom px row (80..96) is the Solid floor.
        rc, out = _run(
            intgrid_main,
            ["query", "--ldtk", str(path), "--level", "TestLevel",
             "--px", "0,80", "--size", "160,16"],
        )
        assert rc == 0
        assert "Solid" in out, out


def test_intgrid_query_reports_nothing_above_the_floor():
    td, path = _write_project()
    with td:
        # The top cell row (0..16) is all empty.
        rc, out = _run(
            intgrid_main,
            ["query", "--ldtk", str(path), "--level", "TestLevel",
             "--px", "0,0", "--size", "160,16"],
        )
        assert rc == 0
        assert "Solid" not in out, out


def test_entity_measure_center_and_nearest_solid():
    td, path = _write_project()
    with td:
        rc, out = _run(
            measure_main,
            ["--ldtk", str(path), "--level", "TestLevel", "--iid", "sw-1"],
        )
        assert rc == 0
        # Switch sw-1 px=(32,16) size 16x16 -> center (40,24).
        assert "center=(40,24)" in out, out
        # Center cell (2,1); floor at cell y=5 -> 4 cells * 16px = 64px down.
        assert "down=64px" in out, out


def test_entity_measure_missing_entity_returns_nonzero():
    td, path = _write_project()
    with td:
        rc, out = _run(
            measure_main,
            ["--ldtk", str(path), "--level", "TestLevel", "--iid", "nope"],
        )
        assert rc == 1
        assert "no matching" in out.lower(), out


def test_gates_audit_lists_targets_and_consumed_by_id():
    td, path = _write_project()
    with td:
        rc, out = _run(gates_main, ["--ldtk", str(path), "--level", "TestLevel"])
        assert rc == 0
        # An encounter-gating switch shows its target.
        assert "test_switch" in out and "test_enc" in out, out
        # A switch with no target_encounter is flagged as bus-consumed, not orphan.
        assert "consumed by id" in out, out
        # Lock wall, trigger, and breakable all surface.
        assert "test_enc_lock" in out, out
        assert "BreakablePlatform" in out, out


def test_gates_audit_empty_level_is_graceful():
    td = tempfile.TemporaryDirectory()
    with td:
        path = Path(td.name) / "empty.ldtk"
        project = make_project()
        project["levels"][0]["layerInstances"][1]["entityInstances"] = []
        path.write_text(json.dumps(project, indent="\t") + "\n")
        rc, out = _run(gates_main, ["--ldtk", str(path), "--level", "TestLevel"])
        assert rc == 0
        assert "no gating elements" in out.lower(), out
