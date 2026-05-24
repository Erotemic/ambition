#!/usr/bin/env python3
"""Publish every catalog-referenced character sprite into the
sandbox's `assets/sprites/` directory.

Reads the character catalog
(`crates/ambition_sandbox/assets/data/character_catalog.ron`) and,
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
    --sprites-dir crates/ambition_sandbox/assets/sprites
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
CATALOG_PATH = REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "data" / "character_catalog.ron"
RENDERER_DIR = REPO_ROOT / "tools" / "ambition_sprite2d_renderer"
SPRITES_DIR = REPO_ROOT / "crates" / "ambition_sandbox" / "assets" / "sprites"


def renderer_target_for_catalog_entry(spritesheet_path: str) -> str | None:
    """Derive the renderer target name from a catalog entry's
    `spritesheet:` field. Returns `None` for catalog entries whose
    sprite lives in a special subdir (gnu_ton_boss/, mockingbird_boss/)
    — those need their own publisher and are skipped here.

    Examples:
      "sprites/architect_spritesheet.png"        -> "architect"
      "sprites/player_robot_spritesheet.png"     -> "player_robot"
      "sprites/gnu_ton_boss/gnu_ton_boss_*.png"  -> None (subdir)
    """
    if not spritesheet_path.startswith("sprites/"):
        return None
    relative = spritesheet_path.removeprefix("sprites/")
    if "/" in relative:
        # Subdir cases (gnu_ton_boss, mockingbird_boss) — bespoke
        # publishers handle these. The caller invokes them
        # explicitly.
        return None
    if not relative.endswith("_spritesheet.png"):
        return None
    return relative.removesuffix("_spritesheet.png")


def expected_outputs(target: str) -> list[str]:
    """List the runtime files publishing `target` should produce.
    Used by the post-publish coverage check."""
    return [
        f"{target}_spritesheet.png",
        f"{target}_spritesheet.ron",
    ]


def publish_target(
    target: str, renderer_dir: Path, sprites_dir: Path, dry_run: bool, verbose: bool
) -> tuple[bool, str]:
    """Run `publish <target> --dest-root <sprites_dir>`. Returns
    `(ok, summary)` where `ok` is True iff the runtime files now
    exist on disk."""
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
        for rel in expected_outputs(target)
        if not (sprites_dir / rel).exists()
    ]
    if missing:
        return False, f"missing {missing}"
    return True, "ok"


def load_catalog_targets(catalog_path: Path) -> Iterable[tuple[str, str]]:
    """Yield (character_id, renderer_target) pairs for catalog
    entries whose sprite is publishable via the standard
    `publish <target>` path. Subdir entries and entries without
    a `sprites/<name>_spritesheet.png` path are filtered out."""
    from .ron_parse import load as ron_load

    data = ron_load(catalog_path.read_text())
    for cid, entry in data["characters"].items():
        target = renderer_target_for_catalog_entry(entry["spritesheet"])
        if target is None:
            continue
        yield cid, target


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", type=Path, default=CATALOG_PATH)
    parser.add_argument("--renderer-dir", type=Path, default=RENDERER_DIR)
    parser.add_argument("--sprites-dir", type=Path, default=SPRITES_DIR)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("-v", "--verbose", action="store_true",
                        help="Print stderr from each renderer publish on failure.")
    args = parser.parse_args(argv)

    pairs = sorted(set(load_catalog_targets(args.catalog)))
    print(f"# {len(pairs)} catalog entries map to a standard renderer target.")

    ok_count = 0
    fail_count = 0
    skipped_count = 0
    failures: list[tuple[str, str, str]] = []
    for cid, target in pairs:
        ok, summary = publish_target(
            target, args.renderer_dir, args.sprites_dir, args.dry_run, args.verbose
        )
        if ok:
            ok_count += 1
            print(f"  [ok] {cid:40s} target={target} ({summary})")
        else:
            fail_count += 1
            failures.append((cid, target, summary))
            print(f"  [fail] {cid:40s} target={target} ({summary})")

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
