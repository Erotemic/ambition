#!/usr/bin/env python3
"""Feature tests for ``ambition_ldtk_tools area create`` beyond the
basic smoketest:

- ``--dry-run`` builds the level entirely in memory, prints a preview,
  and does NOT mutate the file.
- ``connect_to:`` inserts a reciprocal ``LoadingZone`` into an existing
  target level.
- Biome metadata (``biome`` / ``music_track`` / etc.) is emitted as
  level field instances.
- Unknown entity identifiers and unknown fields produce actionable
  error messages with suggestions.

Run directly:
    python tools/ambition_ldtk_tools/tests/test_area_authoring_features.py

Exit code 0 means every feature behaves as documented.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

# tools/ambition_ldtk_tools/tests/test_area_authoring_features.py -> repo root
REPO_ROOT = Path(__file__).resolve().parents[3]
LDTK_PATH = REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "ambition" / "worlds" / "sandbox.ldtk"
PKG_ROOT = REPO_ROOT / "tools" / "ambition_ldtk_tools"


def run_tool(spec_path: Path, ldtk_path: Path, *extra_args: str) -> subprocess.CompletedProcess:
    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools",
        "area",
        "create",
        str(spec_path),
        "--ldtk",
        str(ldtk_path),
        *extra_args,
    ]
    env = {**os.environ, "PYTHONPATH": str(PKG_ROOT)}
    return subprocess.run(cmd, capture_output=True, text=True, env=env)


def fail(msg: str) -> None:
    print(f"FAIL: {msg}", file=sys.stderr)
    raise SystemExit(1)


def test_dry_run_does_not_modify_file(td: Path) -> None:
    spec_path = td / "dry.json"
    spec_path.write_text(json.dumps({
        "id": "dryrun_area",
        "level_id": "dryrun_level",
        "world_x": 24000,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 128,
        "fill_collision": "empty",
        "entities": [
            {
                "type": "PlayerStart",
                "px": [16, 16],
                "size": [28, 46],
                "fields": {"name": "dry_spawn"},
            },
        ],
    }))
    ldtk_copy = td / "sandbox_dry.ldtk"
    shutil.copy2(LDTK_PATH, ldtk_copy)
    before_mtime = ldtk_copy.stat().st_mtime_ns
    before_size = ldtk_copy.stat().st_size

    r = run_tool(spec_path, ldtk_copy, "--dry-run")
    if r.returncode != 0:
        fail(f"dry-run exited {r.returncode}: {r.stderr}")
    if "preview" not in r.stdout:
        fail(f"dry-run did not print 'preview' header. stdout: {r.stdout!r}")
    if "no file written" not in r.stdout:
        fail("dry-run did not announce that no file was written")

    after_mtime = ldtk_copy.stat().st_mtime_ns
    after_size = ldtk_copy.stat().st_size
    if after_mtime != before_mtime or after_size != before_size:
        fail("dry-run mutated the LDtk file (mtime/size changed)")
    project = json.loads(ldtk_copy.read_text())
    if any(l.get("identifier") == "dryrun_level" for l in project.get("levels", [])):
        fail("dry-run wrote the new level into the LDtk file")
    print("ok: dry-run does not modify file or insert level")


def test_biome_metadata_lands_as_level_fields(td: Path) -> None:
    ldtk_copy = td / "sandbox_biome.ldtk"
    shutil.copy2(LDTK_PATH, ldtk_copy)

    spec_path = td / "biome_spec.json"
    spec_path.write_text(json.dumps({
        "id": "biome_area",
        "level_id": "biome_level",
        "world_x": 24500,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 128,
        "fill_collision": "empty",
        "biome": "cave",
        "music_track": "original_lofi_loop",
        "ambient_profile": "damp",
        "visual_theme": "blue",
        "entities": [
            {
                "type": "PlayerStart",
                "px": [16, 16],
                "size": [28, 46],
                "fields": {"name": "biome_spawn"},
            },
        ],
    }))
    r = run_tool(spec_path, ldtk_copy, "--no-repair")
    if r.returncode != 0:
        fail(f"biome metadata run exited {r.returncode}: {r.stderr}")

    project = json.loads(ldtk_copy.read_text())
    new_level = next(
        (l for l in project["levels"] if l["identifier"] == "biome_level"),
        None,
    )
    if new_level is None:
        fail("biome_level was not inserted")
    fields = {
        f["__identifier"]: f.get("__value")
        for f in new_level.get("fieldInstances", [])
    }
    expected = {
        "activeArea": "biome_area",
        "biome": "cave",
        "music_track": "original_lofi_loop",
        "ambient_profile": "damp",
        "visual_theme": "blue",
    }
    for k, v in expected.items():
        if fields.get(k) != v:
            fail(f"level field {k!r} = {fields.get(k)!r}, want {v!r}")
    print("ok: biome / music / ambient / theme land as level field instances")


def test_connect_to_inserts_reciprocal_loading_zone(td: Path) -> None:
    ldtk_copy = td / "sandbox_connect.ldtk"
    shutil.copy2(LDTK_PATH, ldtk_copy)

    # Place the new level in a fresh corridor of the world frame so it
    # does not overlap existing levels.
    spec_path = td / "connect_spec.json"
    spec_path.write_text(json.dumps({
        "id": "connect_area",
        "level_id": "connect_level",
        "world_x": 25000,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 128,
        "fill_collision": "empty",
        "entities": [
            {
                "type": "PlayerStart",
                "px": [16, 16],
                "size": [28, 46],
                "fields": {"name": "connect_spawn"},
            },
            {
                "type": "LoadingZone",
                "px": [200, 16],
                "size": [48, 96],
                "fields": {
                    "id": "connect_entry",
                    "name": "connect_entry",
                    "activation": "Door",
                    "target_room": "central_hub_main",
                    "target_zone": "connect_door",
                    "bidirectional": True,
                },
            },
        ],
        "connect_to": [
            {
                "target_room": "central_hub_main",
                # Pick coordinates that don't overlap any existing entity in
                # central_hub_main. The hub is sparse near (240, 600) and
                # this 16x96 footprint slots into the lower-floor zone.
                "px": [240, 600],
                "size": [16, 96],
                "id": "connect_door",
                "target_zone": "connect_entry",
                "activation": "Door",
                "bidirectional": True,
            },
        ],
    }))
    r = run_tool(spec_path, ldtk_copy, "--no-repair")
    if r.returncode != 0:
        fail(f"connect_to run exited {r.returncode}\n--- stdout ---\n{r.stdout}\n--- stderr ---\n{r.stderr}")

    project = json.loads(ldtk_copy.read_text())
    hub = next(
        (l for l in project["levels"] if l["identifier"] == "central_hub_main"),
        None,
    )
    if hub is None:
        fail("central_hub_main not in project")
    ambition = next(
        (lay for lay in hub["layerInstances"] if lay["__identifier"] == "Ambition"),
        None,
    )
    if ambition is None:
        fail("central_hub_main has no Ambition layer")
    found = None
    for inst in ambition["entityInstances"]:
        if inst["__identifier"] != "LoadingZone":
            continue
        fields = {f["__identifier"]: f.get("__value") for f in inst.get("fieldInstances", [])}
        if fields.get("id") == "connect_door":
            found = (inst, fields)
            break
    if found is None:
        fail("reciprocal LoadingZone 'connect_door' not inserted into central_hub_main")
    inst, fields = found
    if fields.get("target_room") != "connect_level":
        fail(f"reciprocal LoadingZone target_room {fields.get('target_room')!r} != 'connect_level'")
    if fields.get("target_zone") != "connect_entry":
        fail(f"reciprocal LoadingZone target_zone {fields.get('target_zone')!r} != 'connect_entry'")
    print("ok: connect_to inserts reciprocal LoadingZone into the target level")


def test_unknown_entity_type_suggestion(td: Path) -> None:
    ldtk_copy = td / "sandbox_typo.ldtk"
    shutil.copy2(LDTK_PATH, ldtk_copy)
    spec_path = td / "typo_spec.json"
    spec_path.write_text(json.dumps({
        "id": "typo_area",
        "level_id": "typo_level",
        "world_x": 26000,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 128,
        "fill_collision": "empty",
        "entities": [
            {"type": "PlayerStrt", "px": [16, 16], "size": [28, 46]},
        ],
    }))
    r = run_tool(spec_path, ldtk_copy, "--dry-run")
    if r.returncode == 0:
        fail("typo run unexpectedly succeeded")
    msg = (r.stderr or "") + (r.stdout or "")
    if "Did you mean 'PlayerStart'" not in msg:
        fail(f"missing suggestion for typo. output: {msg}")
    print("ok: unknown entity type produces 'Did you mean ...' suggestion")


def test_unknown_field_rejected(td: Path) -> None:
    ldtk_copy = td / "sandbox_field.ldtk"
    shutil.copy2(LDTK_PATH, ldtk_copy)
    spec_path = td / "field_spec.json"
    spec_path.write_text(json.dumps({
        "id": "field_area",
        "level_id": "field_level",
        "world_x": 26500,
        "world_y": 1024,
        "px_wid": 256,
        "px_hei": 128,
        "fill_collision": "empty",
        "entities": [
            {
                "type": "PlayerStart",
                "px": [16, 16],
                "size": [28, 46],
                "fields": {"not_a_real_field": 42},
            },
        ],
    }))
    r = run_tool(spec_path, ldtk_copy, "--dry-run")
    if r.returncode == 0:
        fail("unknown-field run unexpectedly succeeded")
    msg = (r.stderr or "") + (r.stdout or "")
    if "no field 'not_a_real_field'" not in msg:
        fail(f"missing actionable error for unknown field. output: {msg}")
    print("ok: unknown field produces actionable error")


def main() -> int:
    if not LDTK_PATH.exists():
        print(f"error: missing source LDtk file {LDTK_PATH}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        test_dry_run_does_not_modify_file(td)
        test_biome_metadata_lands_as_level_fields(td)
        test_connect_to_inserts_reciprocal_loading_zone(td)
        test_unknown_entity_type_suggestion(td)
        test_unknown_field_rejected(td)
    print("PASS: all author_ldtk_area feature tests")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
