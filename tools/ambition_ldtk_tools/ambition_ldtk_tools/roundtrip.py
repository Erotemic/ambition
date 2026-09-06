#!/usr/bin/env python3
"""Check whether an Ambition LDtk file is ready for safe tool edits.

This is a non-mutating smoke check. It verifies that the package repair pass would make no semantic editor-metadata changes
and then runs the validator. It does not require one unique JSON formatting layout;
an actual LDtk editor save is considered canonical. Use it before opening
`sandbox.ldtk` in LDtk, or in CI after generated/agent-patched level edits.

Run from the repo root with:

    PYTHONPATH=tools/ambition_ldtk_tools \
    python -m ambition_ldtk_tools roundtrip \
      game/ambition_content/assets/worlds/sandbox.ldtk
"""

from __future__ import annotations

import argparse
import json
import shlex
import sys
from pathlib import Path

from ambition_ldtk_tools.validate import normalize_project_for_editor, validate


def canonical(project: dict) -> str:
    return json.dumps(project, sort_keys=True, separators=(",", ":"))


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


def print_repair_hint(path: Path, changes: list[str]) -> None:
    print(
        f"error: {path} is not canonical for the Ambition LDtk tool pipeline",
        file=sys.stderr,
    )
    print("repair command:", file=sys.stderr)
    print(f"  {cli_command('repair', path, '--in-place')}", file=sys.stderr)
    print("diagnostics:", file=sys.stderr)
    print(f"  {cli_command('repair', path, '--check')}", file=sys.stderr)
    print(f"  git diff -- {shlex.quote(str(path))}", file=sys.stderr)
    print("planned repair groups:", file=sys.stderr)
    for change in changes[:50]:
        print(f"  - {change}", file=sys.stderr)
    if len(changes) > 50:
        print(f"  ... {len(changes) - 50} more", file=sys.stderr)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "path", type=Path, help="Path to an Ambition-authored .ldtk file"
    )
    parser.add_argument(
        "--schema",
        type=Path,
        default=None,
        help="Optional official LDtk JSON schema path",
    )
    parser.add_argument(
        "--require-schema",
        action="store_true",
        help="Fail if official LDtk schema validation cannot run",
    )
    parser.add_argument(
        "--secondary-world",
        action="append",
        type=Path,
        default=None,
        help=(
            "Additional .ldtk source files whose levels the runtime merges on "
            "top of `path` (see ldtk_world/loading.rs::SECONDARY_WORLD_FILES). "
            "Forwarded to the inner validate() call so cross-file "
            "LoadingZone targets resolve without false-positives. May be "
            "passed multiple times."
        ),
    )
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
        print_repair_hint(args.path, changes)
        return 1

    errors, warnings = validate(
        args.path,
        args.schema,
        args.require_schema,
        secondary_worlds=args.secondary_world,
    )
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"OK: {args.path} is valid and safe for Ambition LDtk tools")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
