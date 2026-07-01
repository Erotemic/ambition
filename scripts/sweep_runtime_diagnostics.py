#!/usr/bin/env python3
"""Publisher sweep: relocate leaked diagnostics out of the runtime sprite roots.

The sprite generators emit human-only diagnostics — canonical reference poses,
labeled preview sheets, pixel-grid debug overlays — right next to the runtime
sheet records and page images. This sweep enforces the publish boundary from
``docs/planning/engine/data-driven-sprites-and-characters.md``: the runtime
asset roots hold runtime artifacts only.

Diagnostics are MOVED, not deleted, into a staging diagnostics dir (default
``target/ambition_publish/diagnostics/<root>/...``) so they stay available for
humans while never shipping in the game bundle.

The diagnostic classification here MIRRORS the Rust source of truth in
``crates/ambition_gameplay_core/src/asset_publish/classify.rs``. Keep the two in
sync; the Rust ``shipped_runtime_roots_have_no_leaked_diagnostics`` test is what
fails if a diagnostic survives under a runtime root, and this sweep is what
keeps that test green after a regen.
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

# Runtime sprite roots relative to the repo root. Mirrors
# asset_publish::RUNTIME_SPRITE_ROOTS.
RUNTIME_SPRITE_ROOTS = (
    "crates/ambition_gameplay_core/assets/sprites",
    "crates/ambition_gameplay_core/assets/sprites_0_5x",
    "crates/ambition_gameplay_core/assets/sprites_0_25x",
    "crates/ambition_gameplay_core/assets/sprites_potato",
)

# Filename suffixes that mark a visual diagnostic. Mirrors DIAGNOSTIC_SUFFIXES
# in classify.rs.
DIAGNOSTIC_SUFFIXES = (
    "_canonical.png",
    "_canonical_transparent.png",
    "_preview_labeled.png",
    "_parts_debug.png",
    "_debug.png",
)

# Directory names the generator writes reference galleries into. Mirrors
# in_diagnostic_dir() in classify.rs.
DIAGNOSTIC_DIRS = ("canonicals", "diagnostics")


def is_diagnostic(rel_path: Path) -> bool:
    """True if a path (relative to a runtime root) is a visual diagnostic."""
    if any(part in DIAGNOSTIC_DIRS for part in rel_path.parts):
        return True
    name = rel_path.name
    if name == "canonicals_contact_sheet.png":
        return True
    return any(name.endswith(suffix) for suffix in DIAGNOSTIC_SUFFIXES)


def sweep_root(root: Path, dest_base: Path, *, dry_run: bool) -> list[Path]:
    """Move diagnostics out of one runtime root; return their relative paths."""
    if not root.exists():
        return []
    moved: list[Path] = []
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(root)
        if not is_diagnostic(rel):
            continue
        moved.append(rel)
        if dry_run:
            continue
        dest = dest_base / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(path), str(dest))
    return moved


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Repository root (default: two levels up from this script).",
    )
    parser.add_argument(
        "--dest",
        type=Path,
        default=None,
        help="Diagnostics staging base (default: <repo>/target/ambition_publish/diagnostics).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Report what would move without touching the filesystem.",
    )
    args = parser.parse_args()

    repo_root: Path = args.repo_root
    dest_base: Path = args.dest or (repo_root / "target/ambition_publish/diagnostics")

    total = 0
    for rel_root in RUNTIME_SPRITE_ROOTS:
        root = repo_root / rel_root
        root_name = Path(rel_root).name
        moved = sweep_root(root, dest_base / root_name, dry_run=args.dry_run)
        total += len(moved)
        if moved:
            verb = "would move" if args.dry_run else "moved"
            print(f"{rel_root}: {verb} {len(moved)} diagnostic file(s)")

    if total == 0:
        print("runtime sprite roots are already clean")
    else:
        verb = "would relocate" if args.dry_run else "relocated"
        print(f"{verb} {total} diagnostic file(s) -> {dest_base}")


if __name__ == "__main__":
    main()
