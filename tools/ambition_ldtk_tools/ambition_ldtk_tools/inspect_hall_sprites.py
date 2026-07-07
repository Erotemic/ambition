#!/usr/bin/env python3
"""Walk the Hall of Characters in `hall_of_characters.ldtk` and report,
per NpcSpawn pedestal, whether its catalog id will render a real sprite
or fall back to a colored-rectangle placeholder.

The Hall lives in its own secondary-world `.ldtk` file (regenerated
wholesale from the catalog), not in sandbox.ldtk.

Reads:
  - `crates/ambition_actors/assets/data/character_catalog.ron`
  - `crates/ambition_content/assets/worlds/hall_of_characters.ldtk`
  - `crates/ambition_actors/assets/sprites/`

For each NpcSpawn in `hall_of_characters`:
  - "ok"          — catalog entry + manifest on disk (sheet will load)
  - "no_manifest" — catalog entry exists but `<target>_spritesheet.ron` is missing
  - "no_idle"     — manifest exists but lacks an Idle-equivalent row
  - "no_catalog"  — character_id isn't in the catalog (LDtk drift)

Useful for: confirming what should render in the Hall after a regen,
diagnosing placeholder pedestals, and producing a per-character
checklist for renderer-publisher cleanup.

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.inspect_hall_sprites
```

Pass `--ldtk` / `--catalog` / `--sprites-dir` to override the paths.
Pass `--only-issues` to suppress the "ok" rows.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
CATALOG_PATH = (
    REPO_ROOT
    / "crates"
    / "ambition_actors"
    / "assets"
    / "data"
    / "character_catalog.ron"
)
LDTK_PATH = (
    REPO_ROOT
    / "crates"
    / "ambition_actors"
    / "assets"
    / "ambition"
    / "worlds"
    / "hall_of_characters.ldtk"
)
SPRITES_DIR = REPO_ROOT / "crates" / "ambition_actors" / "assets" / "sprites"

# Mirror of the Rust `CharacterAnim::from_name` Idle-equivalent
# aliases. Keep in sync with
# `crates/ambition_actors/src/presentation/character_sprites/anim.rs`.
IDLE_ALIASES = {"idle", "opening", "rest", "front_idle", "side_idle"}


def hall_npc_spawns(ldtk_path: Path) -> list[dict]:
    data = json.loads(ldtk_path.read_text())
    for level in data.get("levels", []):
        if level.get("identifier") != "hall_of_characters":
            continue
        spawns = []
        for layer in level.get("layerInstances", []) or []:
            for inst in layer.get("entityInstances", []) or []:
                if inst.get("__identifier") != "NpcSpawn":
                    continue
                fields = {
                    f["__identifier"]: f.get("__value")
                    for f in inst.get("fieldInstances", []) or []
                }
                spawns.append(fields)
        return spawns
    return []


def find_manifest_path(spritesheet_field: str, sprites_dir: Path) -> Path | None:
    """The catalog stores `spritesheet:` as `sprites/foo_spritesheet.png`
    (or `sprites/foo_boss/foo_boss_spritesheet.png` for subdirs).
    The matching manifest is `<basename>.ron`."""
    rel = spritesheet_field.removeprefix("sprites/")
    ron_rel = rel.removesuffix(".png") + ".ron"
    path = sprites_dir / ron_rel
    return path if path.exists() else None


def manifest_has_idle(manifest_path: Path) -> bool:
    from .ron_parse import load as ron_load

    try:
        records = ron_load(manifest_path.read_text())
    except Exception:
        return False
    if not isinstance(records, list):
        return False
    for record in records:
        for row in record.get("rows", []) or []:
            anim = row.get("animation") or row.get("name")
            if isinstance(anim, str) and anim in IDLE_ALIASES:
                return True
    return False


def classify(character_id: str, catalog: dict, sprites_dir: Path) -> tuple[str, str]:
    """Return (status_code, detail_message)."""
    if character_id not in catalog["characters"]:
        return ("no_catalog", "character_id not in catalog")
    entry = catalog["characters"][character_id]
    spritesheet = entry["spritesheet"]
    png_path = sprites_dir / spritesheet.removeprefix("sprites/")
    manifest = find_manifest_path(spritesheet, sprites_dir)
    if manifest is None:
        return (
            "no_manifest",
            f"missing {png_path.parent.name}/{png_path.name.replace('.png', '.ron')}",
        )
    if not png_path.exists():
        return ("no_png", f"missing {png_path}")
    if not manifest_has_idle(manifest):
        return ("no_idle", f"manifest {manifest.name} lacks Idle-equivalent row")
    return ("ok", "")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH)
    parser.add_argument("--ldtk", type=Path, default=LDTK_PATH)
    parser.add_argument("--sprites-dir", type=Path, default=SPRITES_DIR)
    parser.add_argument(
        "--only-issues",
        action="store_true",
        help="Suppress the 'ok' rows; print only fallback cases.",
    )
    args = parser.parse_args(argv)

    from .ron_parse import load as ron_load

    catalog = ron_load(args.catalog.read_text())
    spawns = hall_npc_spawns(args.ldtk)
    print(f"# {len(spawns)} NpcSpawn pedestals in `hall_of_characters`.\n")

    counts: dict[str, int] = {
        "ok": 0,
        "no_manifest": 0,
        "no_png": 0,
        "no_idle": 0,
        "no_catalog": 0,
    }
    for spawn in sorted(spawns, key=lambda s: s.get("character_id", "")):
        cid = spawn.get("character_id", "")
        status, detail = classify(cid, catalog, args.sprites_dir)
        counts[status] += 1
        if args.only_issues and status == "ok":
            continue
        line = f"  [{status:<11}] {cid:40s}"
        if detail:
            line += f"  {detail}"
        print(line)
    print()
    print("# summary:")
    for status, count in counts.items():
        if count == 0:
            continue
        print(f"  {status}: {count}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
