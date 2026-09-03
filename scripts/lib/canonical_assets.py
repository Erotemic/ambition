"""Is THIS checkout the box whose generated assets are the canonical ones?

Ten asset ratchets — five in `test_shipped_sheet_pages_are_claimed.py`, five in
`test_tier_variants_are_actually_smaller.py` — were gated on
`AMBITION_ASSETS_ARE_CANONICAL`, and ⛔ **nothing in the repository ever set it**,
so no lane had evaluated them and two planning paragraphs called them ratchets
anyway (queue.md).

⭐ **THE GATE WAS NOT A MISTAKE, which is why the fix is not deleting it.** The
sprite tree is gitignored and machine-local; every PNG under `assets/sprites*` is
generated, and `mirror_assets_for_worktree.py` gives a worktree SYMLINKS at the
main checkout's copies. On a box that regenerates cleanly the `KNOWN_` lists read
as stale and the guards fail for the wrong reason — which is the failure the gate
was added to prevent.

⇒ **So detect the condition instead of asking someone to remember a variable.**
A checkout whose sprite tree holds REAL FILES generated it; one holding symlinks
is borrowing another checkout's, and its assets are not its own to ratchet.

    absent tree          → not canonical (nothing to check)
    real dir, symlinks   → not canonical (a mirrored worktree — this box)
    real dir, real files → CANONICAL (the box that generated them)
    env var set          → canonical, whatever the tree looks like

⚠ **The env variable stays as an override**, because it is the only way to
exercise the enabled path somewhere the detection says no, and because removing
a documented opt-in would break anyone using it.

⛔ **The enabled arm is UNEXERCISED on a mirrored worktree** — this file's own
tests build real and symlinked trees in `tmp_path` to check both answers, but
"the ten ratchets pass on the canonical box" is a claim only that box can make.
Nothing here should be read as evidence that they do.
"""

from __future__ import annotations

import os
from pathlib import Path

#: The opt-in that predates the detection. Kept as an override, never required.
CANONICAL_ENV = "AMBITION_ASSETS_ARE_CANONICAL"

#: The generated tier trees. `sprites` alone is enough to answer the question —
#: the variants are produced from it by the same run — but a box may have been
#: interrupted, so any tree holding real files counts.
SPRITE_TREE_GLOB = "crates/ambition_platformer2d_actor_monolith/assets/sprites*"

#: How many files to look at before deciding. The trees hold thousands and the
#: answer is uniform: `mirror_assets_for_worktree.py` links every file or none.
#: ⚠ Not 1 — a single stray real file in a mirrored tree (a regenerated sprite,
#: which the mirror exists to allow) would flip the answer for the whole box.
SAMPLE = 25


def sprite_trees(repo: Path) -> list[Path]:
    """Every generated sprite tier tree that exists, in a stable order."""
    return sorted(p for p in repo.glob(SPRITE_TREE_GLOB) if p.is_dir())


def assets_are_canonical(repo: Path, env: dict[str, str] | None = None) -> bool:
    """True when this checkout's generated assets are its OWN.

    ⚠ Deliberately conservative: anything it cannot establish reads as NOT
    canonical, because a false yes turns ten ratchets red on a box whose lists
    were never meant to describe it — the exact failure the original gate
    prevented.
    """
    environ = os.environ if env is None else env
    if environ.get(CANONICAL_ENV):
        return True

    real = 0
    linked = 0
    for tree in sprite_trees(repo):
        for png in sorted(tree.rglob("*.png")):
            if png.is_symlink():
                linked += 1
            else:
                real += 1
            if real + linked >= SAMPLE:
                break
        if real + linked >= SAMPLE:
            break

    if real + linked == 0:
        return False
    # A mirrored tree is entirely links apart from what the worktree regenerated
    # itself, so "any link at all" is the honest test for borrowed assets.
    return linked == 0


def why_not(repo: Path) -> str:
    """A skip reason that says what was LOOKED AT, not just that it skipped.

    ⛔ A skip that only says "set this variable" is how ten assertions sat
    unevaluated while two planning paragraphs called them ratchets.
    """
    trees = sprite_trees(repo)
    if not trees:
        return (
            f"no generated sprite tree under {SPRITE_TREE_GLOB} — this checkout "
            "has never regenerated assets, so it has nothing of its own to "
            f"ratchet. Set {CANONICAL_ENV}=1 to force these on anyway."
        )
    return (
        f"{len(trees)} sprite tree(s) present but holding SYMLINKS — this is a "
        "mirrored worktree borrowing another checkout's generated assets "
        "(scripts/mirror_assets_for_worktree.py), so its KNOWN_ lists describe "
        f"a tree it did not produce. Set {CANONICAL_ENV}=1 to force them on."
    )
