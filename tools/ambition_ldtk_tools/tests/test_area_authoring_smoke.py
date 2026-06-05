#!/usr/bin/env python3
"""Smoke test for ``ambition_ldtk_tools area create``.

Copies the real sandbox ``.ldtk`` to a temp file, drops in a tiny test
area via the authoring tool, verifies validation passes, and confirms
the new level + entities round-trip through the standard repair pass
without losing fields.

Run directly:
    python tools/ambition_ldtk_tools/tests/test_area_authoring_smoke.py

Exit code 0 means the tool produced an editor-roundtrip-clean file
that validates against both Ambition semantics and the LDtk schema.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# tools/ambition_ldtk_tools/tests/test_area_authoring_smoke.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[3]
LDTK_PATH = (
    REPO_ROOT
    / "crates"
    / "ambition_sandbox"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"

# Minimal but realistic: a PlayerStart, a floor solid, a loading zone back
# to the central hub. Placement is well past the existing levels' rightmost
# edge so the overlap check accepts it.
SPEC = {
    "id": "smoketest_area",
    "level_id": "smoketest_level",
    "world_x": 22000,
    "world_y": 1024,
    "px_wid": 512,
    "px_hei": 256,
    "fill_collision": "solid_floor",
    "entities": [
        {
            "type": "PlayerStart",
            "px": [32, 32],
            "size": [28, 46],
            "fields": {"name": "smoketest_start"},
        },
        {
            "type": "Solid",
            "px": [0, 240],
            "size": [512, 16],
            "fields": {"name": "smoketest_floor"},
        },
        {
            "type": "LoadingZone",
            "px": [16, 192],
            "size": [48, 48],
            "fields": {
                "id": "smoketest_exit",
                "name": "smoketest_exit",
                "activation": "walk",
                "target_room": "central_hub_complex",
                # Target a real existing LoadingZone in the hub so the
                # validator's room-graph check accepts the link.
                "target_zone": "east_exit",
                "bidirectional": False,
            },
        },
    ],
}


def main() -> int:
    if not LDTK_PATH.exists():
        print(f"error: missing source LDtk file {LDTK_PATH}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        ldtk_copy = td / "sandbox.ldtk"
        spec_path = td / "spec.json"
        shutil.copy2(LDTK_PATH, ldtk_copy)
        spec_path.write_text(json.dumps(SPEC, indent=2))

        cmd = [
            sys.executable,
            "-m",
            "ambition_ldtk_tools",
            "area",
            "create",
            str(spec_path),
            "--ldtk",
            str(ldtk_copy),
        ]
        env = {"PYTHONPATH": str(PKG_ROOT)}
        import os

        env = {**os.environ, **env}
        print("$ " + " ".join(cmd))
        result = subprocess.run(cmd, capture_output=True, text=True, env=env)
        print(result.stdout, end="")
        if result.stderr:
            print(result.stderr, file=sys.stderr, end="")
        if result.returncode != 0:
            print(f"FAIL: tool exited with {result.returncode}", file=sys.stderr)
            return result.returncode

        # Parse the produced file and assert the new level is present with
        # the expected entities + their fields.
        project = json.loads(ldtk_copy.read_text())
        new_level = next(
            (l for l in project["levels"] if l["identifier"] == "smoketest_level"),
            None,
        )
        if new_level is None:
            print("FAIL: new level was not appended", file=sys.stderr)
            return 1
        active_area = next(
            (
                f["__value"]
                for f in new_level.get("fieldInstances", [])
                if f["__identifier"] == "activeArea"
            ),
            None,
        )
        if active_area != SPEC["id"]:
            print(
                f"FAIL: activeArea mismatch: got {active_area!r}, want {SPEC['id']!r}",
                file=sys.stderr,
            )
            return 1
        ambition_layer = next(
            l for l in new_level["layerInstances"] if l["__identifier"] == "Ambition"
        )
        ents = {e["__identifier"]: e for e in ambition_layer["entityInstances"]}
        # Static-collision entities (Solid / OneWayPlatform / BlinkWall)
        # are lowered to IntGrid cells, NOT emitted on the Ambition
        # layer. Verify the lowering happened: Solid must be absent from
        # the Ambition entities AND the Collision IntGrid must show the
        # painted cells where the rect was authored.
        if "Solid" in ents:
            print(
                "FAIL: Solid was emitted as an entity instead of being "
                "lowered to IntGrid cells",
                file=sys.stderr,
            )
            return 1
        for kind in ("PlayerStart", "LoadingZone"):
            if kind not in ents:
                print(f"FAIL: missing entity {kind} in new level", file=sys.stderr)
                return 1
            # Field instances must carry realEditorValues (filled by repair)
            # so the LDtk GUI can save the level without nulling fields.
            for fi in ents[kind].get("fieldInstances", []):
                if fi.get("__value") is not None and not fi.get("realEditorValues"):
                    print(
                        f"FAIL: {kind}.{fi['__identifier']} missing realEditorValues",
                        file=sys.stderr,
                    )
                    return 1
        # IntGrid lowering check: the spec painted a Solid at px (0, 240)
        # size (512, 16) on a 16-px grid. That's row cy=240/16=15 across
        # the full 32-cell-wide level → cells [cy=15, cx=0..31] should be
        # value 1 (Solid). The bottom row from `solid_floor` (cy=15
        # again, value 1) overlaps; the painted Solid replaces those
        # cells with the same value. The smoketest checks the row was
        # painted, not the exact cell-set, since both the floor fill
        # and the lowered Solid converge on the same cells.
        coll = next(
            l for l in new_level["layerInstances"] if l["__identifier"] == "Collision"
        )
        c_wid = coll["__cWid"]
        target_row = 240 // 16
        row_start = target_row * c_wid
        row_cells = coll["intGridCsv"][row_start : row_start + c_wid]
        if not all(v == 1 for v in row_cells):
            print(
                f"FAIL: Collision row {target_row} should be all-1 (Solid lowering), "
                f"got {row_cells}",
                file=sys.stderr,
            )
            return 1

        # Confirm field types coerced correctly.
        lz = ents["LoadingZone"]
        bidir = next(
            f for f in lz["fieldInstances"] if f["__identifier"] == "bidirectional"
        )
        if bidir["__value"] is not False:
            print(
                f"FAIL: LoadingZone.bidirectional should be False, got {bidir['__value']!r}",
                file=sys.stderr,
            )
            return 1

        # The Collision IntGrid layer should match the level's c_wid * c_hei.
        col = next(
            l for l in new_level["layerInstances"] if l["__identifier"] == "Collision"
        )
        c_wid = SPEC["px_wid"] // 16
        c_hei = SPEC["px_hei"] // 16
        if len(col["intGridCsv"]) != c_wid * c_hei:
            print(
                f"FAIL: intGridCsv length {len(col['intGridCsv'])} != "
                f"{c_wid * c_hei} (cWid * cHei)",
                file=sys.stderr,
            )
            return 1

        # And the floor row should be 1s while the rest is 0s.
        bottom_row = col["intGridCsv"][(c_hei - 1) * c_wid : c_hei * c_wid]
        if any(v != 1 for v in bottom_row):
            print(
                "FAIL: solid_floor fill did not produce a 1-row at the bottom",
                file=sys.stderr,
            )
            return 1
        upper_row = col["intGridCsv"][:c_wid]
        if any(v != 0 for v in upper_row):
            print(
                "FAIL: solid_floor fill incorrectly populated the top row",
                file=sys.stderr,
            )
            return 1

    print("PASS: author_ldtk_area produced a valid, editor-roundtrip-clean level")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
