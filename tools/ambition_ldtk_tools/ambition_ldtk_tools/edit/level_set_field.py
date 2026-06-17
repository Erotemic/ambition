#!/usr/bin/env python3
"""Set level field instance values on existing LDtk levels.

Use this to update level-scoped metadata (biome, music_track,
ambient_profile, visual_theme, parallax_theme, lighting_hint, etc.)
without hand-editing the LDtk JSON. The tool reuses
`build_level_field_instances`-style coercion through the existing
level field defs so values land in the canonical shape the LDtk
editor expects, then re-runs the standard repair + validate pass.

CLI form (single level, one or more fields):

    python -m ambition_ldtk_tools.edit.level_set_field \\
        --level gnu_ton_arena \\
        --set music_track=standing_on_shoulders \\
        --set ambient_profile=hum \\
        --in-place

Spec form (multiple levels / multiple fields per spec):

    levels:
      - level_id: gnu_ton_arena
        fields:
          music_track: standing_on_shoulders
      - level_id: hall_of_bosses
        fields:
          music_track: gradient_sentinel_corridor

The tool errors out if:
  * the level doesn't exist;
  * a named field isn't declared on `defs.levelFields` (add it via
    `tools/add_biome_level_fields.py` or the relevant scaffold script
    first — silent write-through would leave the LDtk editor refusing
    to load the field next time).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/level_set_field.py
#   -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    coerce_field_value,
    load_project,
    make_field_instance,
    write_project,
)


def load_spec(path: Path) -> dict:
    text = path.read_text()
    if path.suffix.lower() in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except ImportError as ex:  # pragma: no cover
            raise SystemExit(f"YAML spec but pyyaml not installed: {ex}")
        return yaml.safe_load(text)
    return json.loads(text)


def find_level(project: dict, level_id: str) -> dict:
    for lev in project.get("levels", []):
        if lev.get("identifier") == level_id:
            return lev
    raise SystemExit(
        f"level '{level_id}' not found. Levels: "
        + ", ".join(l.get("identifier") for l in project.get("levels", []))
    )


def find_level_field_def(project: dict, field_name: str) -> dict:
    defs = project.get("defs", {})
    for fd in defs.get("levelFields") or []:
        if fd.get("identifier") == field_name:
            return fd
    known = [fd.get("identifier") for fd in defs.get("levelFields") or []]
    raise SystemExit(
        f"level field '{field_name}' is not declared in defs.levelFields. "
        f"Known: {known}. Add it first (e.g. `tools/add_biome_level_fields.py`)."
    )


def apply_level_field_edit(
    project: dict,
    level: dict,
    field_name: str,
    new_value,
) -> None:
    field_def = find_level_field_def(project, field_name)
    type_str = field_def.get("__type") or field_def.get("type") or "String"
    coerced = coerce_field_value(type_str, new_value)
    instance_payload = make_field_instance(field_def, coerced)
    for fi in level.setdefault("fieldInstances", []):
        if fi.get("__identifier") == field_name:
            fi.clear()
            fi.update(instance_payload)
            return
    level["fieldInstances"].append(instance_payload)


def _parse_cli_set(values: list[str]) -> dict[str, str]:
    out: dict[str, str] = {}
    for entry in values or []:
        if "=" not in entry:
            raise SystemExit(f"--set expects key=value, got {entry!r}")
        key, val = entry.split("=", 1)
        out[key.strip()] = val
    return out


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("spec", type=Path, nargs="?")
    parser.add_argument(
        "--level",
        type=str,
        default=None,
        help="Target level identifier (use with --set).",
    )
    parser.add_argument(
        "--set",
        action="append",
        default=[],
        help="Single field assignment, key=value. Repeatable.",
    )
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_gameplay_core"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    parser.add_argument("--in-place", action="store_true")
    parser.add_argument("--output", type=Path, default=None)
    parser.add_argument("--backup", action="store_true")
    parser.add_argument("--no-repair", action="store_true")
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "ambition_ldtk_tools"
        / "schemas"
        / "ldtk"
        / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)
    if not args.in_place and args.output is None:
        parser.error("choose --in-place or --output <path>")

    if args.spec is None:
        if not args.level or not args.set:
            parser.error("either pass a spec file, or use --level <id> --set key=value")
        edits_by_level: list[tuple[str, dict[str, str]]] = [
            (args.level, _parse_cli_set(args.set))
        ]
    else:
        spec = load_spec(args.spec)
        if not isinstance(spec, dict) or "levels" not in spec:
            return _fail("spec must be a mapping with `levels: [...]`")
        edits_by_level = []
        for entry in spec["levels"]:
            level_id = entry.get("level_id")
            fields = entry.get("fields") or {}
            if not level_id or not fields:
                return _fail(
                    f"each spec entry needs `level_id` and non-empty `fields`: {entry!r}"
                )
            edits_by_level.append((level_id, fields))

    project = load_project(args.ldtk)
    summary: list[str] = []
    for level_id, fields in edits_by_level:
        level = find_level(project, level_id)
        for fname, fvalue in fields.items():
            apply_level_field_edit(project, level, fname, fvalue)
            summary.append(f"{level_id}.{fname} = {fvalue!r}")

    target_path = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    write_project(target_path, project)
    print(f"applied {len(summary)} level-field edit(s):")
    for line in summary:
        print(f"  {line}")
    if args.no_repair:
        return 0

    cmd = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.repair",
        str(target_path),
        "--in-place",
    ]
    print("$ " + " ".join(cmd))
    if subprocess.run(cmd).returncode != 0:
        return 1
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target_path)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
