#!/usr/bin/env python3
"""Auto-layout LDtk levels from LoadingZone graph structure.

This module is now a thin CLI adapter. The layout implementation lives under
``ambition_ldtk_tools.edit.layout`` so graph construction, strategies, SVG
rendering, and LDtk writeback can evolve independently.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

from ambition_ldtk_tools.editor_format import dump_editor_style
from ambition_ldtk_tools.edit.layout.graph import *  # noqa: F401,F403 - compatibility exports
from ambition_ldtk_tools.edit.layout.model import Point
from ambition_ldtk_tools.edit.layout.model import *  # noqa: F401,F403 - compatibility exports
from ambition_ldtk_tools.edit.layout.strategies import auto_layout
from ambition_ldtk_tools.edit.layout.strategies import *  # noqa: F401,F403 - compatibility exports
from ambition_ldtk_tools.edit.layout.svg import write_svg_report
from ambition_ldtk_tools.edit.layout.svg import *  # noqa: F401,F403 - compatibility exports
from ambition_ldtk_tools.edit.layout.writeback import write_report
from ambition_ldtk_tools.edit.layout.writeback import *  # noqa: F401,F403 - compatibility exports

REPO_ROOT = Path(__file__).resolve().parents[4]


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("action", choices=["auto-layout"], help="Subcommand action.")
    parser.add_argument("ldtk", type=Path, help="Target .ldtk file to layout.")
    parser.add_argument(
        "--start",
        default="central_hub_main",
        help="Start level identifier or activeArea to anchor at --origin (default central_hub_main).",
    )
    parser.add_argument(
        "--origin",
        default="0,0",
        help="World coordinate for the requested start level/area top-left (default 0,0).",
    )
    parser.add_argument(
        "--strategy",
        choices=["greedy", "layered", "clustered"],
        default="greedy",
        help="Layout backend: greedy door-near packing, layered rank layout, or linkage-clustered packing.",
    )
    parser.add_argument("--gap", type=int, default=256, help="Preferred px distance from a source group to newly placed linked groups/ranks.")
    parser.add_argument("--padding", type=int, default=None, help="Minimum px padding between packed group rectangles. Defaults to --gap / 4 for compact legacy behavior.")
    parser.add_argument("--cluster-min-links", type=int, default=2, help="For --strategy clustered, minimum undirected LoadingZone edge count needed to merge two low-degree groups.")
    parser.add_argument("--cluster-degree-limit", type=int, default=4, help="For --strategy clustered, do not merge groups whose graph degree is above this limit.")
    parser.add_argument(
        "--grid",
        type=int,
        default=None,
        help="Snap group anchors to this grid. Defaults to project worldGridWidth.",
    )
    parser.add_argument("--dry-run", action="store_true", help="Print the proposed layout without writing.")
    parser.add_argument("--report", type=Path, default=None, help="Optional text report output path.")
    parser.add_argument("--svg-report", type=Path, default=None, help="Optional SVG preview of the proposed editor layout. Works with --dry-run.")
    parser.add_argument("--svg-max-width", type=int, default=1800, help="Maximum SVG viewport width in pixels for --svg-report.")
    parser.add_argument("--lock", action="append", default=[], metavar="LEVEL_OR_AREA", help="Keep this level/activeArea group at its current editor position; may be repeated.")
    parser.add_argument("--lock-field", default="layoutLocked", help="Optional LDtk level field name used as a persistent layout lock (default layoutLocked).")
    parser.add_argument("--ignore-field-locks", action="store_true", help="Ignore persistent level lock fields and use only --lock.")
    parser.add_argument("--in-place", action="store_true", help="Write back to the input .ldtk path.")
    parser.add_argument("--output", type=Path, default=None, help="Output path (alternative to --in-place).")
    parser.add_argument("--backup", action="store_true", help="When using --in-place, copy original to <ldtk>.bak first.")
    parser.add_argument("--no-repair", action="store_true", help="Skip repair + validate post-pass.")
    parser.add_argument(
        "--schema",
        type=Path,
        default=REPO_ROOT / "tools" / "ambition_ldtk_tools" / "schemas" / "ldtk" / "JSON_SCHEMA.json",
    )
    args = parser.parse_args(argv)

    if args.action != "auto-layout":
        return _fail(f"unknown world action '{args.action}'")
    if not args.ldtk.exists():
        return _fail(f"ldtk file not found: {args.ldtk}")
    if args.dry_run and (args.in_place or args.output is not None):
        return _fail("--dry-run cannot be combined with --in-place or --output")
    if not args.dry_run and not args.in_place and args.output is None:
        return _fail("choose --dry-run, --in-place, or --output <path>")

    try:
        ox_s, oy_s = args.origin.split(",", 1)
        origin = Point(int(ox_s), int(oy_s))
    except Exception:
        return _fail("--origin must be X,Y")

    project = json.loads(args.ldtk.read_text())
    result = auto_layout(
        project,
        start=args.start,
        origin=origin,
        grid=args.grid,
        gap=args.gap,
        padding=args.padding,
        lock=args.lock,
        lock_field=args.lock_field,
        respect_field_locks=not args.ignore_field_locks,
        strategy=args.strategy,
        cluster_min_links=args.cluster_min_links,
        cluster_degree_limit=args.cluster_degree_limit,
    )
    print(result.report, end="")
    print(
        f"planned/moved {result.moved_levels} level(s); "
        f"updated cached coords for {result.updated_entities} entit(y/ies)."
    )
    if args.report:
        write_report(args.report, result.report)
        print(f"wrote report: {args.report}")
    if args.svg_report:
        write_svg_report(args.svg_report, result, max_width=args.svg_max_width)
        print(f"wrote svg report: {args.svg_report}")
    if args.dry_run:
        return 0

    target = args.output or args.ldtk
    if args.in_place and args.backup:
        backup = args.ldtk.with_suffix(args.ldtk.suffix + ".bak")
        shutil.copy2(args.ldtk, backup)
        print(f"wrote backup: {backup}")
    target.write_text(dump_editor_style(project))
    print(f"wrote {target}")

    if args.no_repair:
        return 0
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.repair", str(target), "--in-place"]
    print("$ " + " ".join(cmd))
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        return rc
    cmd = [sys.executable, "-m", "ambition_ldtk_tools.validate", str(target)]
    if args.schema and args.schema.exists():
        cmd.extend(["--schema", str(args.schema), "--require-schema"])
    print("$ " + " ".join(cmd))
    return subprocess.run(cmd).returncode


def _fail(msg: str) -> int:
    print(f"error: {msg}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
