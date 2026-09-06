#!/usr/bin/env python3
"""Check that every sheet the publish roster claims is actually on disk.

⛔⛔ THIS EXISTS BECAUSE A TEST FAILED ON A MISSING ASSET AND READ LIKE A CODE
BUG. `ambition_render`'s
``a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one``
uses `goblin_cave_dagger` as its canonical RIGHT-drawn sheet.
`record_for_sheet_key` returned `None` — not because handedness broke, but
because no roster line ever published that sheet — so `cargo test --workspace
--lib` failed with a panic naming handedness. Found 2026-08-30, the first time
that gate ran to completion.

⭐ THE SIBLING CHECK ALREADY EXISTED AND ANSWERED A DIFFERENT QUESTION.
`check_quality_variants_are_fresh.py` asks *"is the published tier art OLDER
than its source?"* — it says nothing about art that was never published at all,
because a file that does not exist cannot be stale. Freshness and PRESENCE are
two different failures and only one of them had an instrument.

⚠ GENERATED ART IS GITIGNORED, so "it works on my checkout" is the EXPECTED
symptom of this class rather than a surprising one: whoever rendered the target
by hand has it and nobody else does.

Usage::

    python3 scripts/check_published_sheets_are_present.py
    python3 scripts/check_published_sheets_are_present.py --asset-root <dir>

Exit 1 names the missing sheets and the command that publishes them."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# One implementation of "what does this target install", shared with the sprite
# regen cache.
sys.path.insert(0, str(REPO_ROOT / "scripts" / "lib"))
from sprite_install_names import claimed_install_names  # noqa: E402
DEFAULT_ASSET_ROOT = (
    REPO_ROOT / "crates/ambition_platformer2d_actor_monolith/assets"
)
SPRITES_SH = REPO_ROOT / "scripts/regen/sprites.sh"

#: The roster arrays in `sprites.sh` that name TARGETS this script can check.
#:
#: ⚠ `publish_targets` is the REAL roster and is mostly `"${other[@]}"`
#: expansions, so it is parsed for the bare names it adds directly
#: (`entities`, `puppy_slug`, `mockingbird_boss`) and the arrays it expands are
#: listed alongside it. Missing one of these is how `gnu_ton_apple` stayed
#: unpublished while every check passed.
#:
#: ⛔ `faction_cues` is deliberately excluded — its comment says those YAMLs
#: live in `configs/factions/`, which no discovery surface registers, so they
#: are named products rather than registered targets.
ROSTER_ARRAYS = (
    "review_cues",
    "tackon_targets",
    "pirate_targets",
    "publish_targets",
)


def roster_targets(sprites_sh: Path) -> list[str]:
    """Parse the publish roster out of `sprites.sh`.

    ⛔ PARSED FROM THE SHELL RATHER THAN DUPLICATED HERE. A second copy of the
    roster in Python would drift from the one that actually publishes, and a
    checker that disagrees with the thing it checks is worse than no checker.
    """
    text = sprites_sh.read_text(encoding="utf-8")
    found: list[str] = []
    for name in ROSTER_ARRAYS:
        match = re.search(rf"^{name}=\((.*?)^\)", text, re.MULTILINE | re.DOTALL)
        if not match:
            continue
        body = re.sub(r"#[^\n]*", "", match.group(1))
        found.extend(tok for tok in body.split() if re.fullmatch(r"[a-z0-9_]+", tok))
    return found


def claimed_names(targets: list[str]) -> dict[str, list[str]] | None:
    """What each target DECLARES it installs, asked of the renderer itself.

    ⛔⛔ NEVER GUESS `<target>_spritesheet.ron`. The reasoning, and the four
    targets that disprove the guess, live with the implementation in
    `scripts/lib/sprite_install_names.py` — shared because the sprite regen
    cache needs the same answer and used to guess it independently.

    Returns `None` when the renderer package is not importable, which is an
    ordinary state on a machine that has not set up the tool venv — the caller
    reports "cannot check" rather than inventing a verdict.
    """
    return claimed_install_names(targets)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--asset-root", type=Path, default=DEFAULT_ASSET_ROOT)
    parser.add_argument("--sprites-sh", type=Path, default=SPRITES_SH)
    args = parser.parse_args()

    sprites_dir = args.asset_root / "sprites"
    if not sprites_dir.is_dir():
        print(f"no sprites directory at {sprites_dir}", file=sys.stderr)
        return 1
    if not args.sprites_sh.is_file():
        print(f"no publish roster at {args.sprites_sh}", file=sys.stderr)
        return 1

    targets = roster_targets(args.sprites_sh)
    if not targets:
        print(f"could not parse any roster array from {args.sprites_sh}", file=sys.stderr)
        return 1

    claimed = claimed_names(targets)
    if claimed is None:
        print(
            "cannot check: the sprite renderer is not importable here, so what "
            "each target installs is unknown.\n"
            "  run through the tool venv: scripts/regen/sprites.sh --check-toolchain"
        )
        return 0

    missing: dict[str, list[str]] = {}
    checked = 0
    for target, names in sorted(claimed.items()):
        if not names:
            continue
        checked += 1
        absent = [n for n in names if not (sprites_dir / n).is_file()]
        # ⭐ ALL-OR-NOTHING, NOT ANY. A target mid-publish or one whose optional
        # sidecars differ per machine would otherwise report forever; a target
        # that published NOTHING is the failure this exists to catch.
        if absent and len(absent) == len(names):
            missing[target] = absent

    if not missing:
        print(f"all {checked} rostered target(s) have published art under {sprites_dir}")
        return 0

    print(
        f"{len(missing)} rostered target(s) published NOTHING — the game and its "
        "tests resolve these by name and get nothing:"
    )
    for target, names in sorted(missing.items()):
        print(f"       missing  {target}  ({names[0]}{', …' if len(names) > 1 else ''})")
    print()
    print("  fix: ./scripts/regen/sprites.sh --target <name>   (one target)")
    print("       ./scripts/regen/assets.sh                    (the whole roster)")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
