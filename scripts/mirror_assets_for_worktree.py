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
point of a worktree is to change something: mirror the files individually so a
regenerated sprite can land as a REAL file in the worktree, replacing that one
symlink, while every other asset still points at the shared copy. Directory
links would send that write straight into main's assets, which is exactly the
accident this exists to prevent.

## ⛔⛔ THE PER-FILE LINK IS ONLY HALF THE PROTECTION — THE WRITER MUST UNLINK

This paragraph used to end "and the main checkout never sees it". **That was
false, and it was the reassurance that stopped anyone checking.** A per-file
symlink prevents nothing on its own: `Image.save()`, `Path.write_text()` and
`shutil.copy2` all OPEN THE DESTINATION FOR WRITING, and an open-for-write
FOLLOWS a symlink. Measured 2026-09-02 rather than argued —
`open(link, "wb").write(...)` changed the TARGET's bytes and left the link in
place — so regenerating assets in a worktree silently rewrote the checkout every
other session builds and gates from.

⇒ **The invariant is: a writer publishing into this tree must unlink a symlinked
destination first.** Two roads were fixed once the hazard was measured — the
variant generator (`generate_visual_quality_variants.py`, all three write sites)
and the renderer's publish (`_copy_sheet_files`) — each with the premise itself
under test, because a guard asserting "main was untouched" passes trivially on a
system where writes never followed links.

⚠ **A THIRD ROAD ADDED LATER WILL NOT BE PROTECTED BY THIS FILE.** Nothing here
can enforce the rule on a writer it has never heard of; the enforcement lives in
`scripts/tests/test_asset_writes_do_not_follow_worktree_symlinks.py`, which
checks the known write sites by source. Add a new publisher, add it there.


A fresh worktree gets EMPTY submodule directories, and one of them is not art:
`game/ambition_map_assets` holds every `.ldtk` world. The files under
`game/*/assets/worlds/` are symlinks into it, so an uninitialised submodule makes
them dangling links and any authoring or validation work dies at minute one on a
raw `FileNotFoundError` naming a path that visibly exists. That is a worse
failure than a missing sprite: nothing about the traceback says "submodule".

So this initialises them first. ⚠ **it is not a mirror** — a submodule is real
version-controlled content with its own commits, and linking a worktree's
submodule at the main checkout's would put an edit made here into main's index.
`git submodule update --init` gives the worktree its own checkout, which is what
a worktree is for.

## Usage

    python scripts/mirror_assets_for_worktree.py              # mirror into cwd's worktree
    python scripts/mirror_assets_for_worktree.py --dry-run
    python scripts/mirror_assets_for_worktree.py --prune      # drop links whose target is gone
    python scripts/mirror_assets_for_worktree.py --force      # re-link even local real files
    python scripts/mirror_assets_for_worktree.py --no-submodules   # assets only

It takes NO path argument and is run FROM INSIDE the worktree; it finds the
primary checkout itself.

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
    # Content-side generated art lives outside the monolith and must be mirrored
    # with the rest of the generated asset tree.
    # The vanity card's part sheet, produced by scripts/regen/sprites.sh. The old
    # `vanity_card` tree was the superseded full-frame card and no longer
    # exists; mirroring that name copied nothing while the sheet the shipped
    # manifest actually names was left behind.
    "game/ambition_content/assets/vanity_card_made_this_meme",
    "game/ambition_content/assets/backgrounds",
    "game/ambition_content/assets/concept_art",
    "game/ambition_content/assets/icons",
    "game/ambition_content/assets/manual-art",
)


# Submodules a worktree needs CHECKED OUT rather than mirrored, with the reason
# a worker will hit if they are not. these are ordinary git content, not
# generated assets: `git submodule update --init` gives the worktree its own
# checkout, and symlinking them at main's would route an edit made here into
# main's index.
REQUIRED_SUBMODULES = (
    (
        "game/ambition_map_assets",
        "every .ldtk world — `game/*/assets/worlds/*.ldtk` are symlinks into it, "
        "so without this any LDtk read fails with a bare FileNotFoundError on a "
        "path that looks like it exists",
    ),
)


def submodule_is_populated(root: Path, rel: str) -> bool:
    """A submodule directory that git has actually checked out.

    ⚠ the directory always EXISTS in a fresh worktree — it is just empty, which is
    exactly why the failure is illegible. `.git` (a file, for a submodule) is the
    thing that only appears once it is initialised.
    """
    return (root / rel / ".git").exists()


def init_submodules(root: Path, dry_run: bool) -> list[str]:
    """Check out the submodules a worktree cannot work without.

    Returns human-readable problems; an empty list means everything needed is
    present. ⛔ **a failure here is REPORTED, never swallowed** — the whole reason
    this exists is that the downstream symptom (a dangling symlink) says nothing
    about its cause, and a mirror script that quietly failed to fix it would just
    move the illegibility one step earlier.
    """
    problems: list[str] = []
    for rel, why in REQUIRED_SUBMODULES:
        if submodule_is_populated(root, rel):
            print(f"  {rel}: already checked out")
            continue
        if dry_run:
            print(f"  {rel}: would `git submodule update --init` ({why})")
            continue
        print(f"  {rel}: initialising — {why}")
        result = subprocess.run(
            ["git", "submodule", "update", "--init", rel],
            cwd=root,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0 or not submodule_is_populated(root, rel):
            detail = (result.stderr or result.stdout or "").strip().splitlines()
            problems.append(
                f"{rel} could not be checked out: {detail[-1] if detail else 'unknown error'}\n"
                f"      it holds {why}.\n"
                f"      Fix it by hand from the worktree root:\n"
                f"        git submodule update --init {rel}\n"
                f"      If that cannot reach the network, copy it from the primary "
                f"checkout instead — but do NOT symlink it: a submodule is "
                f"version-controlled content and a link would send edits made here "
                f"into the main checkout's index."
            )
    return problems


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
    parser.add_argument(
        "--no-submodules",
        action="store_true",
        help="skip the submodule checkout and mirror generated assets only",
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

    # submodules FIRST. A worker's first LDtk command is what discovers
    # this gap today, and it discovers it as a traceback rather than as advice.
    submodule_problems: list[str] = []
    if not args.no_submodules:
        print("checking out submodules this worktree needs")
        submodule_problems = init_submodules(dest, args.dry_run)

    print(f"mirroring generated assets\n  from {source}\n  into {dest}")
    total_linked = total_kept = total_replaced = total_pruned = 0
    # ⛔ A NAME THAT COPIES NOTHING IS THE BUG THIS LIST HAS ALREADY HAD ONCE.
    # The `vanity_card` entry above records it: a superseded tree stayed in the
    # list, mirrored nothing, and the sheet the shipped manifest actually names
    # was left behind — silently, because a missing source is just `continue`.
    # Four entries are in that state today (`game/ambition_content/assets/`
    # `backgrounds`, `concept_art`, `icons`, `manual-art`; absent in main too).
    # They are harmless while nothing is there and become wrong the moment a
    # tree is added under a DIFFERENT name, so the skip is reported rather than
    # silent — and rather than deleting entries that may be forward-looking.
    absent: list[str] = []
    for rel in MIRRORED_TREES:
        src_root = source / rel
        if not src_root.is_dir():
            absent.append(rel)
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

    if absent:
        print(
            f"  ⚠ {len(absent)} mirrored-tree entr(y/ies) name a source that does "
            "not exist and were skipped:"
        )
        for rel in absent:
            print(f"      {rel}")
        print(
            "    Harmless while nothing is there. ⛔ If a tree WAS added under a "
            "different name, this is the `vanity_card` bug again: the mirror "
            "copies nothing and the art is missing only in worktrees."
        )

    verb = "would link" if args.dry_run else "linked"
    print(
        f"{verb} {total_linked} file(s); kept {total_kept} local file(s)"
        + (f"; replaced {total_replaced}" if total_replaced else "")
        + (f"; pruned {total_pruned} dangling" if total_pruned else "")
    )
    if total_kept and not args.force:
        print("  (local files outrank the shared copy — pass --force to overwrite)")

    if submodule_problems:
        print(
            "\n⛔ this worktree is NOT ready — a submodule it needs is not checked "
            "out:",
            file=sys.stderr,
        )
        for problem in submodule_problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
