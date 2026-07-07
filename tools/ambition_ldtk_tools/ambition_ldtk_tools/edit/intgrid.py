#!/usr/bin/env python3
"""IntGrid cell editing — paint, erase, and summarize Collision /
Water / Climbable IntGrid layers.

Until now Ambition's authoring loop lowered Solid / OneWayPlatform /
BlinkWall / HazardBlock entities into IntGrid cells at `area create`
time, but had no way to inspect or surgically edit those cells
afterwards. That's a problem when:

  - A wall placed by `area create` overlaps a LoadingZone slot the
    player now needs to walk through (the user's 2026-05-16 EdgeExit
    feedback — walls overlapping side-exit zones blocked the
    transition).
  - You want to verify what surfaces a room actually has before
    snapping a door (`door snap` only sees existing cells; this
    lets you SEE those cells).
  - You want to clear a small region without re-authoring the
    whole level from spec.

Two subcommands today (`paint` is reserved but unimplemented; add
when a real need lands rather than speculatively):

  intgrid summarize --level X [--layer Collision]
    Print a per-value cell count + bounding box per IntGrid layer.

  intgrid erase --level X --px X,Y --size W,H [--layer Collision]
    Zero out every IntGrid cell that overlaps the given rect.

Default layer is `Collision`; pass `--layer Water` or
`--layer Climbable` for those.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Iterable

# tools/ambition_ldtk_tools/ambition_ldtk_tools/edit/intgrid.py -> repo root.
REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import (  # noqa: E402
    load_project,
    write_project,
)
from ambition_ldtk_tools.edit.set_field import find_level  # noqa: E402

# IntGrid layer value → identifier lookup. Mirrored from sandbox.ldtk's
# defs.layers; kept here so we can pretty-print rather than show raw ints.
LAYER_VALUE_NAMES: dict[str, dict[int, str]] = {
    "Collision": {
        1: "Solid",
        2: "OneWayUp",
        3: "BlinkSoft",
        4: "BlinkHard",
        5: "Hazard",
    },
    "Water": {1: "ClearWater", 2: "MurkyWater"},
    "Climbable": {1: "Ladder", 2: "Vine", 3: "Wall"},
}


def find_intgrid_layer(level: dict, layer_id: str) -> dict:
    for li in level.get("layerInstances", []):
        if li.get("__identifier") == layer_id and li.get("__type") == "IntGrid":
            return li
    raise SystemExit(
        f"level '{level['identifier']}' has no IntGrid layer '{layer_id}'. "
        f"Known IntGrid layers: "
        + ", ".join(
            li.get("__identifier", "?")
            for li in level.get("layerInstances", [])
            if li.get("__type") == "IntGrid"
        )
    )


def _iter_overlap_cells(
    layer: dict, px: int, py: int, w: int, h: int
) -> Iterable[tuple[int, int]]:
    grid = int(layer["__gridSize"])
    c_wid = int(layer["__cWid"])
    c_hei = int(layer["__cHei"])
    cx0 = max(0, px // grid)
    cy0 = max(0, py // grid)
    cx1 = min(c_wid, (px + w + grid - 1) // grid)
    cy1 = min(c_hei, (py + h + grid - 1) // grid)
    for cy in range(cy0, cy1):
        for cx in range(cx0, cx1):
            yield cx, cy


def _cmd_summarize(args: argparse.Namespace) -> int:
    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_intgrid_layer(level, args.layer)
    grid = int(layer["__gridSize"])
    c_wid = int(layer["__cWid"])
    c_hei = int(layer["__cHei"])
    csv = layer.get("intGridCsv") or []
    if len(csv) != c_wid * c_hei:
        raise SystemExit(f"intGridCsv length {len(csv)} != cWid*cHei={c_wid * c_hei}")
    counts: dict[int, int] = {}
    bboxes: dict[int, tuple[int, int, int, int]] = {}
    for cy in range(c_hei):
        for cx in range(c_wid):
            v = csv[cy * c_wid + cx]
            if v == 0:
                continue
            counts[v] = counts.get(v, 0) + 1
            if v not in bboxes:
                bboxes[v] = (cx, cy, cx, cy)
            else:
                bx0, by0, bx1, by1 = bboxes[v]
                bboxes[v] = (min(bx0, cx), min(by0, cy), max(bx1, cx), max(by1, cy))
    print(
        f"# {args.layer} in '{args.level}': {c_wid}×{c_hei} cells "
        f"(gridSize={grid}, room {c_wid * grid}×{c_hei * grid}px)"
    )
    if not counts:
        print("  (no non-zero cells)")
        return 0
    names = LAYER_VALUE_NAMES.get(args.layer, {})
    for v in sorted(counts):
        bx0, by0, bx1, by1 = bboxes[v]
        px0, py0 = bx0 * grid, by0 * grid
        px1, py1 = (bx1 + 1) * grid - 1, (by1 + 1) * grid - 1
        name = names.get(v, f"value{v}")
        print(
            f"  value={v} ({name}): {counts[v]:>4} cells, "
            f"bbox cells=({bx0},{by0})..({bx1},{by1}) "
            f"px=({px0},{py0})..({px1},{py1})"
        )
    return 0


def _cmd_query(args: argparse.Namespace) -> int:
    """Read-only: report the IntGrid values present in a px/size rect.

    The read-only complement of `paint`/`erase` — answers "what's at this
    location?" (Solid / Hazard / ladder / water) without opening the LDtk
    editor or grepping `intGridCsv`. Mirrors the erase/paint --px/--size
    surface so an author can probe the exact rect they're about to edit.
    Never modifies the file.
    """
    px = _parse_pair(args.px, "px")
    size = _parse_pair(args.size, "size")
    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_intgrid_layer(level, args.layer)
    grid = int(layer["__gridSize"])
    c_wid = int(layer["__cWid"])
    csv = layer.get("intGridCsv") or []
    names = LAYER_VALUE_NAMES.get(args.layer, {})
    by_value: dict[int, list[tuple[int, int]]] = {}
    for cx, cy in _iter_overlap_cells(layer, px[0], px[1], size[0], size[1]):
        v = csv[cy * c_wid + cx]
        by_value.setdefault(v, []).append((cx, cy))
    total = sum(len(cells) for cells in by_value.values())
    print(
        f"# {args.layer} query in '{args.level}': "
        f"px=({px[0]},{px[1]}) size=({size[0]},{size[1]}), "
        f"gridSize={grid} — {total} cell(s) overlap"
    )
    if total == 0:
        print("  (rect is outside the layer's cell grid)")
        return 0
    for v in sorted(by_value):
        cells = by_value[v]
        name = "empty" if v == 0 else names.get(v, f"value{v}")
        if args.verbose:
            listing = ", ".join(f"({cx},{cy})" for cx, cy in cells)
        else:
            sample = ", ".join(f"({cx},{cy})" for cx, cy in cells[:6])
            listing = sample + ("" if len(cells) <= 6 else f", +{len(cells) - 6} more")
        print(f"  value={v} ({name}): {len(cells):>4} cell(s)  cells={listing}")
    return 0


def _cmd_paint(args: argparse.Namespace) -> int:
    """Set every IntGrid cell overlapping a px/size rect to a target value.

    Symmetric with `_cmd_erase` (which sets cells to 0). Use this to add
    a Solid floor / wall to a level whose IntGrid is empty in that
    region without resorting to entity-instance Solids (which then need
    to lower).
    """
    px = _parse_pair(args.px, "px")
    size = _parse_pair(args.size, "size")
    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_intgrid_layer(level, args.layer)
    c_wid = int(layer["__cWid"])
    csv = list(layer.get("intGridCsv") or [])
    value = int(args.value)
    painted = 0
    overwrote = 0
    detail: list[tuple[int, int, int]] = []
    for cx, cy in _iter_overlap_cells(layer, px[0], px[1], size[0], size[1]):
        idx = cy * c_wid + cx
        prev = csv[idx]
        if prev == value:
            continue
        csv[idx] = value
        painted += 1
        if prev != 0:
            overwrote += 1
        detail.append((cx, cy, prev))
    layer["intGridCsv"] = csv
    print(
        f"intgrid paint: level={args.level} layer={args.layer} "
        f"rect px={list(px)} size={list(size)} value={value} "
        f"painted {painted} cell(s) ({overwrote} overwrote a non-zero value)"
    )
    if args.verbose and detail:
        names = LAYER_VALUE_NAMES.get(args.layer, {})
        target_name = names.get(value, f"value{value}")
        for cx, cy, prev in detail:
            prev_name = names.get(prev, f"value{prev}")
            print(f"  cell ({cx},{cy}) {prev} ({prev_name}) -> {value} ({target_name})")
    if painted == 0:
        return 0  # no-op is still success
    if args.dry_run:
        return 0

    target = args.output or args.ldtk
    write_project(target, project)
    if args.no_repair:
        return 0
    import subprocess

    rc = subprocess.run(
        [
            sys.executable,
            "-m",
            "ambition_ldtk_tools.repair",
            str(target),
            "--in-place",
        ]
    ).returncode
    if rc != 0:
        return rc
    cmd_val = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.validate",
        str(target),
    ]
    return subprocess.run(cmd_val).returncode


def _cmd_erase(args: argparse.Namespace) -> int:
    px = _parse_pair(args.px, "px")
    size = _parse_pair(args.size, "size")
    project = load_project(args.ldtk)
    level = find_level(project, args.level)
    layer = find_intgrid_layer(level, args.layer)
    c_wid = int(layer["__cWid"])
    csv = list(layer.get("intGridCsv") or [])
    cleared = 0
    detail: list[tuple[int, int, int]] = []
    for cx, cy in _iter_overlap_cells(layer, px[0], px[1], size[0], size[1]):
        idx = cy * c_wid + cx
        v = csv[idx]
        if v != 0:
            csv[idx] = 0
            cleared += 1
            detail.append((cx, cy, v))
    layer["intGridCsv"] = csv
    print(
        f"intgrid erase: level={args.level} layer={args.layer} "
        f"rect px={list(px)} size={list(size)} cleared {cleared} cell(s)"
    )
    if args.verbose and detail:
        names = LAYER_VALUE_NAMES.get(args.layer, {})
        for cx, cy, v in detail:
            label = names.get(v, f"value{v}")
            print(f"  cell ({cx},{cy}) was {v} ({label})")
    if cleared == 0:
        return 0  # no-op is still success
    if args.dry_run:
        return 0

    target = args.output or args.ldtk
    write_project(target, project)
    if args.no_repair:
        return 0
    import subprocess

    rc = subprocess.run(
        [
            sys.executable,
            "-m",
            "ambition_ldtk_tools.repair",
            str(target),
            "--in-place",
        ]
    ).returncode
    if rc != 0:
        return rc
    cmd_val = [
        sys.executable,
        "-m",
        "ambition_ldtk_tools.validate",
        str(target),
    ]
    return subprocess.run(cmd_val).returncode


def _parse_pair(raw: str, label: str) -> tuple[int, int]:
    parts = raw.split(",")
    if len(parts) != 2:
        raise SystemExit(f"--{label} expects X,Y; got {raw!r}")
    return int(parts[0]), int(parts[1])


def _add_shared_args(p: argparse.ArgumentParser) -> None:
    """Add --ldtk + --layer to a subparser. Subparser-local instead of
    parent-level so callers can put them in any order relative to the
    subcommand on the CLI (argparse subparsers don't inherit top-level
    args after the subcommand fires)."""
    p.add_argument(
        "--ldtk",
        type=Path,
        default=REPO_ROOT
        / "crates"
        / "ambition_actors"
        / "assets"
        / "ambition"
        / "worlds"
        / "sandbox.ldtk",
    )
    p.add_argument(
        "--layer",
        default="Collision",
        choices=sorted(LAYER_VALUE_NAMES),
        help="IntGrid layer to operate on (default Collision)",
    )


def main(argv=None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])

    sub = parser.add_subparsers(dest="action", required=True)

    sp_sum = sub.add_parser("summarize", help="Print per-value cell counts + bboxes")
    _add_shared_args(sp_sum)
    sp_sum.add_argument("--level", required=True)
    sp_sum.set_defaults(func=_cmd_summarize)

    sp_query = sub.add_parser(
        "query",
        help="Read-only: list the IntGrid values present in a px/size rect "
        "(what collision/hazard/etc. is here)",
    )
    _add_shared_args(sp_query)
    sp_query.add_argument("--level", required=True)
    sp_query.add_argument("--px", required=True, help="top-left X,Y of the rect")
    sp_query.add_argument("--size", required=True, help="W,H of the rect")
    sp_query.add_argument(
        "--verbose",
        action="store_true",
        help="list every cell (otherwise only a per-value sample)",
    )
    sp_query.set_defaults(func=_cmd_query)

    sp_erase = sub.add_parser(
        "erase",
        help="Zero out every cell overlapping the given px/size rect",
    )
    _add_shared_args(sp_erase)
    sp_erase.add_argument("--level", required=True)
    sp_erase.add_argument("--px", required=True, help="top-left X,Y of the rect")
    sp_erase.add_argument("--size", required=True, help="W,H of the rect")
    sp_erase.add_argument(
        "--verbose",
        action="store_true",
        help="print every cleared cell (otherwise only the count)",
    )
    sp_erase.add_argument("--output", type=Path, default=None)
    sp_erase.add_argument("--dry-run", action="store_true")
    sp_erase.add_argument("--no-repair", action="store_true")
    sp_erase.set_defaults(func=_cmd_erase)

    sp_paint = sub.add_parser(
        "paint",
        help="Set every cell overlapping the given px/size rect to "
        "--value (1 Solid, 2 OneWayPlatform, etc.)",
    )
    _add_shared_args(sp_paint)
    sp_paint.add_argument("--level", required=True)
    sp_paint.add_argument("--px", required=True, help="top-left X,Y of the rect")
    sp_paint.add_argument("--size", required=True, help="W,H of the rect")
    sp_paint.add_argument(
        "--value",
        required=True,
        type=int,
        help="IntGrid value (1=Solid, 2=OneWayPlatform, 3=BlinkSoft, "
        "4=BlinkHard, 5=Hazard) — see edit/intgrid.py::LAYER_VALUE_NAMES.",
    )
    sp_paint.add_argument(
        "--verbose",
        action="store_true",
        help="print every painted cell (otherwise only the count)",
    )
    sp_paint.add_argument("--output", type=Path, default=None)
    sp_paint.add_argument("--dry-run", action="store_true")
    sp_paint.add_argument("--no-repair", action="store_true")
    sp_paint.set_defaults(func=_cmd_paint)

    args = parser.parse_args(argv)
    return args.func(args) or 0


if __name__ == "__main__":
    raise SystemExit(main())
