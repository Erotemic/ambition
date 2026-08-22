#!/usr/bin/env python3
"""Synchronize Mary-O LDtk entity definitions with the declared entity manifest.

Uses the shared definition-upsert path so definition edits and instance-side LDtk
bookkeeping are normalized together."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
TOOLS = REPO / "tools" / "ambition_ldtk_tools"
TARGET = REPO / "game" / "ambition_demo_mary_o" / "assets" / "worlds" / "mary_o.ldtk"
SPEC = TARGET.with_name("mary_o.entities.json")


def run_tool(*args: str) -> int:
    print("::", " ".join(str(a) for a in args))
    return subprocess.run(
        [sys.executable, "-m", "ambition_ldtk_tools", *args],
        cwd=REPO,
        env={"PYTHONPATH": str(TOOLS), "PATH": "/usr/bin:/bin"},
    ).returncode


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="report what would change and write nothing",
    )
    parser.add_argument(
        "--drop-instance-values",
        action="append",
        default=[],
        metavar="MaryOBlock.FIELD",
        help=(
            "Authorize destroying the values the level's instances carry for "
            "this field. For ONE run by a human who has read what dies — never "
            "wired into a script or a CI job."
        ),
    )
    args = parser.parse_args()

    if not TARGET.exists():
        sys.exit(f"{TARGET.relative_to(REPO)} does not exist yet — run author_mary_o_ldtk.py first")

    # `upsert-entity` runs the repair + schema-validate post-pass itself, which
    # is what re-derives the instance-side bookkeeping a definition edit
    # invalidates (`realEditorValues` go stale the moment a default moves). One
    # command, and the file it leaves behind is one LDtk will open.
    argv = [
        "def",
        "upsert-entity",
        str(SPEC),
        "--ldtk",
        str(TARGET),
        "--dry-run" if args.dry_run else "--in-place",
        # Mary-O's noun, not the engine's — see the flag's help.
        "--game-owned",
    ]
    for path in args.drop_instance_values:
        argv.extend(["--drop-instance-values", path])

    rc = run_tool(*argv)
    if rc != 0:
        sys.exit(rc)
    verb = "would sync" if args.dry_run else "synced"
    print(f"{verb} Mary-O's entity definitions into {TARGET.relative_to(REPO)}")


if __name__ == "__main__":
    main()
