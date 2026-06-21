#!/usr/bin/env python3
"""Room-level inspection, rendering, and debug bundling helpers.

This module is a thin CLI adapter. Inspection, rendering, and bundle writing
live under ``ambition_ldtk_tools.room_support``.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from ambition_ldtk_tools.area_authoring import load_project
from ambition_ldtk_tools.room_support.bundle import write_bundle
from ambition_ldtk_tools.room_support.inspect import DEFAULT_LDTK, REPO_ROOT, format_summary_text, room_summary
from ambition_ldtk_tools.room_support.render import render_room_png, render_room_svg


def _cmd_describe(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    summary = room_summary(project, args.level)
    if args.format == "json":
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(format_summary_text(summary, include_entities=args.entities), end="")
    return 0


def _cmd_render(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    out = args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    suffix = out.suffix.lower()
    if suffix == ".png":
        render_room_png(project, args.level, out, max_width=args.max_width)
    elif suffix == ".svg" or not suffix:
        if not suffix:
            out = out.with_suffix(".svg")
        out.write_text(render_room_svg(project, args.level, max_width=args.max_width))
    else:
        raise SystemExit("room render --out must end in .svg or .png")
    print(f"wrote {out}")
    return 0


def _cmd_bundle_debug(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    write_bundle(
        project=project,
        ldtk=args.ldtk,
        level_id=args.level,
        out=args.out,
        repo_root=args.repo_root,
        render_format=args.render_format,
        include_debug=not args.no_debug,
        run_validate=args.validate,
    )
    print(f"wrote {args.out}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--ldtk",
        type=Path,
        default=DEFAULT_LDTK,
        help="LDtk project path (default: Ambition sandbox.ldtk)",
    )
    sub = parser.add_subparsers(dest="action", required=True)

    describe = sub.add_parser("describe", help="Print a structured room summary")
    describe.add_argument("--level", required=True, help="level identifier")
    describe.add_argument("--format", choices=["text", "json"], default="text")
    describe.add_argument("--entities", action="store_true", help="include every entity row")
    describe.set_defaults(func=_cmd_describe)

    render = sub.add_parser("render", help="Render room geometry/entities to SVG or PNG")
    render.add_argument("--level", required=True, help="level identifier")
    render.add_argument("--out", required=True, type=Path, help="output .svg or .png")
    render.add_argument("--max-width", type=int, default=1400, help="maximum rendered pixel width")
    render.set_defaults(func=_cmd_render)

    bundle = sub.add_parser("bundle-debug", help="Create a chat-sandbox friendly room debug tarball")
    bundle.add_argument("--level", required=True, help="level identifier")
    bundle.add_argument("--out", required=True, type=Path, help="output .tar.gz")
    bundle.add_argument("--repo-root", type=Path, default=REPO_ROOT)
    bundle.add_argument("--render-format", choices=["svg", "png"], default="svg")
    bundle.add_argument("--no-debug", action="store_true", help="do not include debug_traces JSON files")
    bundle.add_argument("--validate", action="store_true", help="include validate command output")
    bundle.set_defaults(func=_cmd_bundle_debug)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    raise SystemExit(main())
