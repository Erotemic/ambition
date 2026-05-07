#!/usr/bin/env python3
"""Check whether an Ambition LDtk file is ready for LDtk GUI editing.

This is a non-mutating smoke check. It verifies that the repair tool would make
no changes and then runs the validator. Use it before opening `sandbox.ldtk` in
LDtk, or in CI after generated/agent-patched level edits.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from ambition_ldtk_tools.validate import normalize_project_for_editor, validate


def canonical(project: dict) -> str:
    return json.dumps(project, sort_keys=True, separators=(",", ":"))


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", type=Path, help="Path to an Ambition-authored .ldtk file")
    parser.add_argument("--schema", type=Path, default=None, help="Optional official LDtk JSON schema path")
    parser.add_argument("--require-schema", action="store_true", help="Fail if official LDtk schema validation cannot run")
    args = parser.parse_args(argv)

    try:
        project = json.loads(args.path.read_text())
    except Exception as ex:  # noqa: BLE001
        print(f"error: failed to read {args.path}: {ex}", file=sys.stderr)
        return 1

    before = canonical(project)
    changes = normalize_project_for_editor(project)
    after = canonical(project)
    if before != after:
        print(
            f"error: {args.path} is not editor-roundtrip clean; run tools/repair_ambition_ldtk.py --in-place {args.path}",
            file=sys.stderr,
        )
        for change in changes[:50]:
            print(f"  - {change}", file=sys.stderr)
        if len(changes) > 50:
            print(f"  ... {len(changes) - 50} more", file=sys.stderr)
        return 1

    errors, warnings = validate(args.path, args.schema, args.require_schema)
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: {args.path} is valid and editor-roundtrip clean")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
