#!/usr/bin/env python3
"""Symlink a worktree's generated assets at the main checkout's, file by file.

Generated art, audio and packs are gitignored, so a fresh `git worktree` has
none of them. That is not a cosmetic gap: the sheet registry is baked from those
directories at build time, so an assetless worktree compiles a binary with an
EMPTY sheet table. Around forty tests then fail for reasons that have nothing to
do with the change under test, the game runs with no art, and the only way to
tell the difference between "my change broke this" and "this worktree has no
assets" is to stash and re-run.

Regenerating them instead costs several minutes and a full duplicate on disk,
for bytes that are identical to the ones already sitting in the main checkout.

## Why file-by-file and not directory symlinks

Linking the directories would be one line and would take the override away. The
point of a worktree is to change something: mirror the files individually and a
regenerated sprite lands as a REAL file in the worktree, replacing that one
symlink, while every other asset still points at the shared copy — and the main
checkout never sees it. Directory links would send that write straight into
main's assets, which is exactly the accident this exists to prevent.

## Usage

    python scripts/mirror_assets_for_worktree.py              # mirror into cwd's worktree
    python scripts/mirror_assets_for_worktree.py --dry-run
    python scripts/mirror_assets_for_worktree.py --prune      # drop links whose target is gone
    python scripts/mirror_assets_for_worktree.py --force      # re-link even local real files

Local real files are LEFT ALONE without `--force`: a file you generated here
outranks the shared one, always, and silently reverting it would undo work.
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path
from typing import Iterable, List, Tuple

# The generated trees, relative to the repo root. Each is gitignored, produced
# by a `regen_*.sh`, and identical between checkouts of the same commit.
MIRRORED_TREES = (
    "crates/ambition_platformer2d_actor_monolith/assets/sprites",
    "crates/ambition_platformer2d_actor_monolith/assets/sprites_0_5x",
    "crates/ambition_platformer2d_actor_monolith/assets/sprites_0_25x",
    "crates/ambition_platformer2d_actor_monolith/assets/sprites_potato",
    "crates/ambition_platformer2d_actor_monolith/assets/sprite_packs",
    "crates/ambition_platformer2d_actor_monolith/assets/audio",
    "crates/ambition_platformer2d_actor_monolith/assets/fonts",
    "crates/ambition_platformer2d_actor_monolith/assets/backgrounds/parallax_layers_0_5x",
    "crates/ambition_platformer2d_actor_monolith/assets/backgrounds/parallax_layers_0_25x",
    "crates/ambition_platformer2d_actor_monolith/assets/backgrounds/parallax_layers_potato",
    # ⚠ Content-side generated art too, and it is easy to forget because it sits
    # under a different crate. The first version of this list stopped at the
    # monolith and left `vanity_card`'s frames behind; the app integration suite
    # then failed twenty tests in this worktree — declared art paths, doors, the
    # HUD, the smash roster — none of which look like a missing picture until you
    # diff the asset trees.
    "game/ambition_content/assets/vanity_card",
    "game/ambition_content/assets/backgrounds",
    "game/ambition_content/assets/concept_art",
    "game/ambition_content/assets/icons",
    "game/ambition_content/assets/manual-art",
)


def main_checkout(here: Path) -> Path:
    """The repository's PRIMARY working tree — the one a worktree branched from.

    `git rev-parse --git-common-dir` names the shared `.git`; its parent is the
    main checkout. Asking `git worktree list` for the first row would work too
    and would break the day somebody reorders it.
    """
    common = subprocess.run(
        ["git", "rev-parse", "--git-common-dir"],
        cwd=here,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return Path(common).resolve().parent


def repo_root(here: Path) -> Path:
    top = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        cwd=here,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    return Path(top).resolve()


def walk_files(root: Path) -> Iterable[Path]:
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in filenames:
            yield Path(dirpath) / name


def mirror(
    source_root: Path, dest_root: Path, force: bool, dry_run: bool
) -> Tuple[int, int, int]:
    linked = kept = replaced = 0
    for src in walk_files(source_root):
        rel = src.relative_to(source_root)
        dst = dest_root / rel
        if dst.is_symlink():
            if dst.resolve() == src.resolve():
                continue
            if not force:
                kept += 1
                continue
            if not dry_run:
                dst.unlink()
            replaced += 1
        elif dst.exists():
            # A REAL file here is local work. It wins.
            if not force:
                kept += 1
                continue
            if not dry_run:
                dst.unlink()
            replaced += 1
        if not dry_run:
            dst.parent.mkdir(parents=True, exist_ok=True)
            dst.symlink_to(src)
        linked += 1
    return linked, kept, replaced


def prune(dest_root: Path, dry_run: bool) -> int:
    dropped = 0
    if not dest_root.exists():
        return 0
    for path in walk_files(dest_root):
        if path.is_symlink() and not path.exists():
            if not dry_run:
                path.unlink()
            dropped += 1
    return dropped


def main(argv: List[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--force",
        action="store_true",
        help="replace local real files with links to the shared copy",
    )
    parser.add_argument(
        "--prune", action="store_true", help="also drop links whose target is gone"
    )
    args = parser.parse_args(argv)

    here = Path.cwd()
    dest = repo_root(here)
    source = main_checkout(here)
    if source == dest:
        print(
            "refusing: this IS the main checkout, so there is nothing to mirror "
            "from. Run it inside a worktree.",
            file=sys.stderr,
        )
        return 2

    print(f"mirroring generated assets\n  from {source}\n  into {dest}")
    total_linked = total_kept = total_replaced = total_pruned = 0
    for rel in MIRRORED_TREES:
        src_root = source / rel
        if not src_root.is_dir():
            continue
        dst_root = dest / rel
        if args.prune:
            total_pruned += prune(dst_root, args.dry_run)
        linked, kept, replaced = mirror(src_root, dst_root, args.force, args.dry_run)
        total_linked += linked
        total_kept += kept
        total_replaced += replaced
        if linked or kept or replaced:
            print(f"  {rel}: +{linked} linked, {kept} local kept, {replaced} replaced")

    verb = "would link" if args.dry_run else "linked"
    print(
        f"{verb} {total_linked} file(s); kept {total_kept} local file(s)"
        + (f"; replaced {total_replaced}" if total_replaced else "")
        + (f"; pruned {total_pruned} dangling" if total_pruned else "")
    )
    if total_kept and not args.force:
        print("  (local files outrank the shared copy — pass --force to overwrite)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
