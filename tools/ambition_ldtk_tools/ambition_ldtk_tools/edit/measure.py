#!/usr/bin/env python3
"""entity measure: report an entity's size + center + nearest solids.

Read-only. Answers placement questions like "how big is the boss spawn,
where is its center, and how much open space is around it?" without
opening the LDtk editor or eyeballing the JSON. Pairs with `entity query`
(which finds the entity / its iid) and `intgrid query` (raw cell values):
this one is entity-centric and adds the surrounding-collision context an
author needs before `entity move` / `entity add`.

The center assumes a top-left pivot (the Ambition authoring convention);
the nearest-Solid probe walks the Collision IntGrid out from the center
cell in each cardinal direction and reports the distance in px (or `edge`
when the room boundary is reached before any Solid).

Examples:
  python -m ambition_ldtk_tools entity measure --level goblin_encounter --identifier BossSpawn
  python -m ambition_ldtk_tools entity measure --iid <iid>
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[4]

from ambition_ldtk_tools.area_authoring import load_project  # noqa: E402
from ambition_ldtk_tools.edit.intgrid import find_intgrid_layer  # noqa: E402
from ambition_ldtk_tools.edit.query import collect  # noqa: E402
from ambition_ldtk_tools.edit.set_field import find_level  # noqa: E402

DEFAULT_LDTK = (
    REPO_ROOT
    / "crates"
    / "ambition_actors"
    / "assets"
    / "ambition"
    / "worlds"
    / "sandbox.ldtk"
)

SOLID_VALUE = 1


def _nearest_solid_px(layer: dict, cell_x: int, cell_y: int) -> dict[str, int | None]:
    grid = int(layer["__gridSize"])
    c_wid = int(layer["__cWid"])
    c_hei = int(layer["__cHei"])
    csv = layer.get("intGridCsv") or []

    def scan(dx: int, dy: int) -> int | None:
        x, y, steps = cell_x + dx, cell_y + dy, 1
        while 0 <= x < c_wid and 0 <= y < c_hei:
            if csv[y * c_wid + x] == SOLID_VALUE:
                return steps * grid
            x += dx
            y += dy
            steps += 1
        return None

    return {
        "left": scan(-1, 0),
        "right": scan(1, 0),
        "up": scan(0, -1),
        "down": scan(0, 1),
    }


def main(argv=None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--ldtk", type=Path, default=DEFAULT_LDTK)
    ap.add_argument("--level", help="restrict to one level id")
    ap.add_argument("--identifier", help="entity type, e.g. BossSpawn / EnemySpawn")
    ap.add_argument("--iid", help="exact entity iid")
    ap.add_argument(
        "--layer",
        default="Collision",
        help="IntGrid layer for the nearest-solid probe (default Collision)",
    )
    args = ap.parse_args(argv)

    project = load_project(args.ldtk)
    rows = collect(project, args.level, args.identifier, [], args.iid)
    if not rows:
        print("no matching entities (try --level / --identifier / --iid)")
        return 1

    for row in rows:
        px = row["px"] or [0, 0]
        w = row["size"][0] or 0
        h = row["size"][1] or 0
        center = (px[0] + w // 2, px[1] + h // 2)
        print(
            f"{row['identifier']} [{row['iid']}] in '{row['level']}': "
            f"px=({px[0]},{px[1]}) size=({w},{h}) center=({center[0]},{center[1]})"
        )
        try:
            level = find_level(project, row["level"])
            layer = find_intgrid_layer(level, args.layer)
        except SystemExit:
            print(f"  (no '{args.layer}' IntGrid layer — skipping nearest-solid probe)")
            continue
        grid = int(layer["__gridSize"])
        near = _nearest_solid_px(layer, center[0] // grid, center[1] // grid)
        parts = [
            f"{d}={'edge' if near[d] is None else f'{near[d]}px'}"
            for d in ("left", "right", "up", "down")
        ]
        print(f"  nearest Solid: {', '.join(parts)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
