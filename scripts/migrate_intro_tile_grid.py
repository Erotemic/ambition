#!/usr/bin/env python3
"""One-shot migration: regrid `intro.ldtk` tile layers from 32→16 px.

Before this script ran, the `intro_lab` tileset was 32×32 and the
`town` tileset was 64×64 — but the Collision IntGrid layer was on a
16-px grid. The visual tile layer (`IntroLabTiles`) used a 32-px grid
that didn't line up with the 16-px collision cells, so the rendered
intro slice had misaligned wall edges (a 16-px-wide collision wall
sits inside a single 32-px tile; the tile draws empty space outside
the actual wall).

This script:

1. Updates the two tileset definitions in `defs.tilesets[]` to the
   regenerated 16×16 PNG dimensions / cell sizes.
2. Updates the `IntroLabTiles` layer def's `gridSize` from 32 to 16.
3. Walks every layer-instance of `IntroLabTiles` in every level:
   - bumps `__gridSize` from 32 to 16
   - doubles `__cWid` and `__cHei` (same world pixel extent, twice as
     many cells)
   - clears `gridTiles[]` (the 32-px placements are stale; rerun
     `tileset paint` to repopulate from the Collision IntGrid)

Run after re-rendering and installing the tilesets:

```bash
python3 -m ambition_sprite2d_renderer render intro_lab_tileset
python3 -m ambition_sprite2d_renderer install intro_lab_tileset
python3 -m ambition_sprite2d_renderer render town_tileset
python3 -m ambition_sprite2d_renderer install town_tileset
python3 scripts/migrate_intro_tile_grid.py
# then repaint from collision:
PYTHONPATH=tools/ambition_ldtk_tools \\
python3 -m ambition_ldtk_tools tileset paint \\
    crates/ambition_content/assets/worlds/intro.ldtk \\
    intro_lab --layer IntroLabTiles --from-intgrid Collision \\
    --map 1=0 --all-levels --overwrite --in-place
```

Idempotent: running twice is a no-op (the grid sizes are already 16).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from PIL import Image

REPO_ROOT = Path(__file__).resolve().parents[1]
LDTK_PATH = REPO_ROOT / "crates/ambition_content/assets/worlds/intro.ldtk"
INTRO_LAB_PNG = (
    REPO_ROOT / "crates/ambition_actors/assets/sprites/intro_lab_tileset.png"
)
TOWN_PNG = REPO_ROOT / "crates/ambition_actors/assets/sprites/town_tileset.png"

NEW_TILE_GRID = 16
LAYER_IDENTIFIER = "IntroLabTiles"
TILESET_IDENTIFIERS = ("intro_lab", "town")


def png_size(path: Path) -> tuple[int, int]:
    with Image.open(path) as im:
        return im.size


def migrate(project: dict) -> tuple[int, int, int]:
    """Return (tileset_defs_updated, layer_defs_updated, layer_instances_updated)."""
    tileset_defs = 0
    layer_defs = 0
    layer_instances = 0
    new_sizes = {
        "intro_lab": png_size(INTRO_LAB_PNG),
        "town": png_size(TOWN_PNG),
    }
    for ts in project.get("defs", {}).get("tilesets", []):
        ident = ts.get("identifier")
        if ident in TILESET_IDENTIFIERS:
            px_w, px_h = new_sizes[ident]
            ts["pxWid"] = px_w
            ts["pxHei"] = px_h
            ts["tileGridSize"] = NEW_TILE_GRID
            tileset_defs += 1

    for layer in project.get("defs", {}).get("layers", []):
        if layer.get("identifier") == LAYER_IDENTIFIER:
            layer["gridSize"] = NEW_TILE_GRID
            layer_defs += 1

    for level in project.get("levels", []):
        for inst in level.get("layerInstances", []):
            if inst.get("__identifier") != LAYER_IDENTIFIER:
                continue
            old_grid = int(inst.get("__gridSize", NEW_TILE_GRID))
            if old_grid == NEW_TILE_GRID:
                # Already migrated (or never had a different grid).
                # Still safe: keep cells/tiles untouched.
                continue
            scale = old_grid // NEW_TILE_GRID
            inst["__gridSize"] = NEW_TILE_GRID
            inst["__cWid"] = int(inst.get("__cWid", 0)) * scale
            inst["__cHei"] = int(inst.get("__cHei", 0)) * scale
            # Stale tile placements: clear and let `tileset paint`
            # rebuild from the Collision IntGrid.
            inst["gridTiles"] = []
            layer_instances += 1

    return tileset_defs, layer_defs, layer_instances


def main() -> int:
    text = LDTK_PATH.read_text()
    project = json.loads(text)
    ts_updated, ld_updated, li_updated = migrate(project)
    LDTK_PATH.write_text(json.dumps(project, indent="\t"))
    print(
        f"migrated: tileset_defs={ts_updated} "
        f"layer_defs={ld_updated} layer_instances={li_updated}"
    )
    print("next: `tileset paint` to repopulate gridTiles from Collision IntGrid.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
