#!/usr/bin/env python3
"""Publish every catalog-referenced character sprite into the
sandbox's `assets/sprites/` directory.

Reads the character catalog
(`crates/ambition_actors/assets/data/character_catalog.ron`) and,
for each entry, asks the renderer to publish the matching target.
This replaces the hand-maintained `tackon_targets` + `review_cues`
+ standalone publisher loops in `regen_sprites.sh` with a single
catalog-driven iteration.

## Why

Jon's mid-run note: "Any steps we can take to unify the rendering
process for characters will likely be a long-term win that we can
afford to look into."

Today the renderer has three publish patterns:
  - Tackon Python targets (`targets/characters/*.py`) with a
    `TARGETS` dict that `publish <name>` walks to copy
    `<name>_spritesheet.{png,yaml,ron}` to `dest-root`.
  - YAML-adapter targets (`configs/*.yaml` and `configs/review/*.yaml`)
    that `draw-review` + `draw-all` render to a scratch dir, then a
    hand-maintained `review_cues` list in `regen_sprites.sh` copies
    them into place.
  - Bespoke one-offs (gnu_ton_boss subdir publisher,
    mockingbird_boss_sprite_generator.py) with their own paths.

This script bridges the first two patterns into one driver: the
catalog's `spritesheet:` field is the on-disk path; from it we
derive the renderer target name. Bespoke one-offs still need their
own invocation (we list them explicitly so they're easy to find).

## Usage

```bash
PYTHONPATH=tools/ambition_ldtk_tools \\
python -m ambition_ldtk_tools.publish_catalog_sprites \\
    --renderer-dir tools/ambition_sprite2d_renderer \\
    --sprites-dir crates/ambition_actors/assets/sprites
```

`--dry-run` prints what it would do without invoking the renderer.

## Exit codes

  0 — every target with a configured target name published cleanly.
  1 — at least one target failed (logged with `[fail]`); the rest
      were attempted and the missing files reported in summary.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import Iterable

REPO_ROOT = Path(__file__).resolve().parents[3]
CATALOG_PATH = (
    REPO_ROOT
    / "crates"
    / "ambition_actors"
    / "assets"
    / "data"
    / "character_catalog.ron"
)
RENDERER_DIR = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"
SPRITES_DIR = REPO_ROOT / "crates" / "ambition_actors" / "assets" / "sprites"


def renderer_target_for_catalog_entry(spritesheet_path: str) -> str | None:
    """Derive the renderer target name from a catalog entry's
    `spritesheet:` field. Returns the target name for both top-level
    sprites and subdir multi-file targets (gnu_ton_boss/,
    mockingbird_boss/) — both publish via `publish <target>` with
    the unified tack-on API. `expected_outputs` knows how to find
    each target's files.

    Examples:
      "sprites/architect_spritesheet.png"        -> "architect"
      "sprites/player_robot_spritesheet.png"     -> "player_robot"
      "sprites/gnu_ton_boss/gnu_ton_boss_*.png"  -> "gnu_ton_boss"
    """
    if not spritesheet_path.startswith("sprites/"):
        return None
    relative = spritesheet_path.removeprefix("sprites/")
    if "/" in relative:
        # Subdir case: `gnu_ton_boss/gnu_ton_boss_spritesheet.png`
        # → target name is the subdir name. Multi-file tack-on
        # targets (currently mockingbird_boss + gnu_ton_boss) use
        # this pattern; the `publish <target>` command still works
        # uniformly because the target's `install()` function lays
        # files into `<dest-root>/<target>/`.
        subdir, _, filename = relative.partition("/")
        if not filename.endswith("_spritesheet.png"):
            return None
        return subdir
    if not relative.endswith("_spritesheet.png"):
        return None
    return relative.removesuffix("_spritesheet.png")


def is_subdir_target(target: str, catalog_path_for_target: str) -> bool:
    """Detect subdir-vs-top-level layout from the catalog's
    `spritesheet:` path. Subdir targets publish into
    `<sprites_dir>/<target>/<files>` rather than
    `<sprites_dir>/<files>`. The publish command is identical;
    only the post-publish file-existence check needs to know."""
    relative = catalog_path_for_target.removeprefix("sprites/")
    return "/" in relative


def expected_outputs(target: str, subdir: bool = False) -> list[str]:
    """List the runtime files publishing `target` should produce
    (relative to the sprites dir). Used by the post-publish coverage
    check. Subdir targets (multi-file tack-ons like
    `mockingbird_boss`) install into `<target>/<file>` per their
    `install()` function."""
    base_files = [
        f"{target}_spritesheet.png",
        f"{target}_spritesheet.ron",
    ]
    if subdir:
        return [f"{target}/{name}" for name in base_files]
    return base_files


def publish_target(
    target: str,
    renderer_dir: Path,
    sprites_dir: Path,
    dry_run: bool,
    verbose: bool,
    subdir: bool = False,
) -> tuple[bool, str]:
    """Run `publish <target> --dest-root <sprites_dir>`. Returns
    `(ok, summary)` where `ok` is True iff the runtime files now
    exist on disk. `subdir=True` looks for files under
    `<sprites_dir>/<target>/` (multi-file tack-on layout)."""
    cmd = [
        sys.executable,
        "-m",
        "ambition_sprite2d_renderer",
        "publish",
        target,
        "--dest-root",
        str(sprites_dir),
    ]
    if dry_run:
        return True, "dry-run"
    result = subprocess.run(
        cmd,
        cwd=str(renderer_dir),
        env={"PYTHONPATH": ".", "PATH": "/usr/bin:/bin"},
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        if verbose:
            sys.stderr.write(result.stderr)
        return False, f"publish exit {result.returncode}"
    # Verify the expected outputs landed.
    missing = [
        rel
        for rel in expected_outputs(target, subdir=subdir)
        if not (sprites_dir / rel).exists()
    ]
    if missing:
        return False, f"missing {missing}"
    return True, "ok"


def load_catalog_targets(catalog_path: Path) -> Iterable[tuple[str, str, bool]]:
    """Yield (character_id, renderer_target, is_subdir) tuples for
    catalog entries whose sprite is publishable via the standard
    `publish <target>` path. Entries without a recognized
    `sprites/...` shape are filtered out.

    The third element flags whether the target uses the multi-file
    subdir layout (e.g. `gnu_ton_boss`, `mockingbird_boss`) so the
    caller can look for outputs under `<sprites_dir>/<target>/`.
    """
    from .ron_parse import load as ron_load

    data = ron_load(catalog_path.read_text())
    for cid, entry in data["characters"].items():
        target = renderer_target_for_catalog_entry(entry["spritesheet"])
        if target is None:
            continue
        yield cid, target, is_subdir_target(target, entry["spritesheet"])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH)
    parser.add_argument("--renderer-dir", type=Path, default=RENDERER_DIR)
    parser.add_argument("--sprites-dir", type=Path, default=SPRITES_DIR)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Print stderr from each renderer publish on failure.",
    )
    args = parser.parse_args(argv)

    triples = sorted(set(load_catalog_targets(args.catalog)))
    subdir_count = sum(1 for _, _, sd in triples if sd)
    print(
        f"# {len(triples)} catalog entries map to a renderer target "
        f"({subdir_count} multi-file subdir, {len(triples) - subdir_count} top-level)."
    )

    ok_count = 0
    fail_count = 0
    skipped_count = 0
    failures: list[tuple[str, str, str]] = []
    for cid, target, subdir in triples:
        ok, summary = publish_target(
            target,
            args.renderer_dir,
            args.sprites_dir,
            args.dry_run,
            args.verbose,
            subdir=subdir,
        )
        layout = "subdir" if subdir else "top"
        if ok:
            ok_count += 1
            print(f"  [ok] {cid:40s} target={target} layout={layout} ({summary})")
        else:
            fail_count += 1
            failures.append((cid, target, summary))
            print(f"  [fail] {cid:40s} target={target} layout={layout} ({summary})")

    print(
        f"\n# summary: {ok_count} published, {fail_count} failed, "
        f"{skipped_count} skipped."
    )
    if failures:
        print("\n# failures (sprite falls back to colored rectangle):")
        for cid, target, summary in failures:
            print(f"  - {cid}: {target} — {summary}")
    return 1 if fail_count else 0


if __name__ == "__main__":
    sys.exit(main())
