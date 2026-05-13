#!/usr/bin/env python3
"""Repair generated or agent-patched Ambition LDtk files for safe tool/editor use.

This tool does not invent gameplay values. It normalizes editor-facing metadata
that can be derived from the LDtk definitions, level origins, and parser-facing values:

- FieldDef internal `type` constructors such as `F_String`
- required LDtk editor metadata keys on entity/field definitions
- instance `defUid` values from definitions
- field instance `realEditorValues` when they are missing/stale for non-default values

Run this before opening a heavily generated or agent-patched `.ldtk` file in the
LDtk GUI, or after hand-patching JSON:

    PYTHONPATH=tools/ambition_ldtk_tools \
    python -m ambition_ldtk_tools repair \
      crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
      --in-place
"""
from __future__ import annotations

import argparse
import json
import shlex
import shutil
import sys
from pathlib import Path

from ambition_ldtk_tools.validate import normalize_project_for_editor, validate


def load_project(path: Path) -> dict:
    return json.loads(path.read_text())


def write_project(path: Path, project: dict) -> None:
    """Write the project in LDtk-editor-shaped JSON via the serializer.

    Earlier versions wrote `json.dumps(project, indent="\t")`, which produced
    a fully-expanded layout that diffed against an editor-saved file as tens of
    thousands of pure formatting lines. The `editor_format` serializer mirrors
    the editor's mixed inline / multi-line layout closely enough that
    tool-edited files diff cleanly against editor-edited ones.
    """
    from ambition_ldtk_tools.editor_format import dump_editor_style

    path.write_text(dump_editor_style(project))


def cli_command(subcommand: str, path: Path, *extra: str) -> str:
    parts = [
        "PYTHONPATH=tools/ambition_ldtk_tools",
        "python",
        "-m",
        "ambition_ldtk_tools",
        subcommand,
        str(path),
        *extra,
    ]
    return " ".join(shlex.quote(part) for part in parts)


def print_change_groups(changes: list[str]) -> None:
    for change in changes[:50]:
        print(f"  - {change}")
    if len(changes) > 50:
        print(f"  ... {len(changes) - 50} more")


def print_check_failure(path: Path, changes: list[str]) -> None:
    print(
        f"error: {path} needs LDtk canonicalization ({len(changes)} change groups)",
        file=sys.stderr,
    )
    print("repair command:", file=sys.stderr)
    print(f"  {cli_command('repair', path, '--in-place')}", file=sys.stderr)
    print("diff inspection after repair:", file=sys.stderr)
    print(f"  git diff -- {shlex.quote(str(path))}", file=sys.stderr)
    for change in changes[:50]:
        print(f"  - {change}", file=sys.stderr)
    if len(changes) > 50:
        print(f"  ... {len(changes) - 50} more", file=sys.stderr)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path, help="Path to an Ambition-authored .ldtk file")
    parser.add_argument("--output", type=Path, default=None, help="Write repaired JSON to this path instead of editing in place")
    parser.add_argument("--in-place", action="store_true", help="Rewrite the input file in place")
    parser.add_argument("--backup", action="store_true", help="When using --in-place, write <file>.bak before modifying")
    parser.add_argument("--check", action="store_true", help="Do not write; fail if repairs would be needed")
    parser.add_argument("--schema", type=Path, default=None, help="Optional official LDtk JSON schema path for post-repair validation")
    parser.add_argument("--require-schema", action="store_true", help="Fail if official LDtk schema validation cannot run")
    args = parser.parse_args(argv)

    if args.check and (args.output or args.in_place):
        parser.error("--check cannot be combined with --output or --in-place")
    if args.output is None and not args.in_place and not args.check:
        parser.error("choose --in-place, --output <path>, or --check")

    try:
        project = load_project(args.path)
    except Exception as ex:  # noqa: BLE001
        print(f"error: failed to read {args.path}: {ex}", file=sys.stderr)
        return 1

    original = json.dumps(project, sort_keys=True, separators=(",", ":"))
    changes = normalize_project_for_editor(project)
    repaired = json.dumps(project, sort_keys=True, separators=(",", ":"))
    would_change = original != repaired

    if args.check:
        if would_change:
            print_check_failure(args.path, changes)
            return 1
        errors, warnings = validate(args.path, args.schema, args.require_schema)
        for warning in warnings:
            print(f"warning: {warning}", file=sys.stderr)
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        if errors:
            return 1
        print(f"OK: {args.path} is already canonical for Ambition LDtk tools")
        return 0

    target = args.output or args.path
    if args.in_place and args.backup and would_change:
        backup = args.path.with_suffix(args.path.suffix + ".bak")
        shutil.copy2(args.path, backup)
        print(f"wrote backup: {backup}", file=sys.stderr)
    if would_change:
        write_project(target, project)
        print(f"repaired {len(changes)} LDtk canonicalization issue(s): {target}")
        print_change_groups(changes)
        print("diagnostic: inspect the resulting file diff with:")
        print(f"  git diff -- {shlex.quote(str(target))}")
    else:
        print(f"OK: {args.path} already needed no repair")

    errors, warnings = validate(target, args.schema, args.require_schema)
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: {target} passes Ambition LDtk validation after repair")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
