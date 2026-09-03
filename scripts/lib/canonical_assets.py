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
import subprocess
import sys
from collections.abc import Callable
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


def _sample_tree_files(repo: Path) -> tuple[int, int]:
    """(real, symlink) counts over the first `SAMPLE` pngs found, in order.

    Shared by the predicate and by `why_not` ON PURPOSE: a skip reason derived
    from a different sample than the decision is how the two come to disagree.
    """
    real = 0
    linked = 0
    for tree in sprite_trees(repo):
        for png in sorted(tree.rglob("*.png")):
            if png.is_symlink():
                linked += 1
            else:
                real += 1
            if real + linked >= SAMPLE:
                return real, linked
    return real, linked


def variants_are_fresh(repo: Path) -> tuple[bool, str]:
    """Are the generated quality tiers current with their sources?

    ⛔ OWNING THE FILES IS NOT THE SAME AS THEM BEING CURRENT, and the size
    ratchets need the second. `check_quality_variants_are_fresh.py` already
    answers it, so this shells out rather than reimplementing the mtime and
    manifest rules — one definition of "fresh", not two that drift.

    ⚠ A checker that cannot RUN is not a pass: any failure to execute it
    returns False with the reason, because a stale-box false finding is exactly
    what this precondition exists to prevent.
    """
    script = repo / "scripts" / "check_quality_variants_are_fresh.py"
    if not script.is_file():
        return False, f"{script} is missing, so freshness could not be established."
    try:
        done = subprocess.run(
            [sys.executable, str(script)],
            cwd=repo, capture_output=True, text=True, timeout=300,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        return False, f"check_quality_variants_are_fresh.py could not run: {exc}."
    if done.returncode == 0:
        return True, "quality tiers are current."
    detail = (done.stdout + done.stderr).strip().splitlines()
    head = detail[0] if detail else "no output"
    return False, (
        f"`python3 scripts/check_quality_variants_are_fresh.py` exits "
        f"{done.returncode}: {head}"
    )


def assets_are_canonical(
    repo: Path,
    env: dict[str, str] | None = None,
    fresh: Callable[[Path], tuple[bool, str]] | None = None,
) -> bool:
    """True when this checkout's generated assets are its OWN.

    ⚠ Deliberately conservative: anything it cannot establish reads as NOT
    canonical, because a false yes turns ten ratchets red on a box whose lists
    were never meant to describe it — the exact failure the original gate
    prevented.

    `fresh` overrides the quality-tier freshness probe and exists for tests
    that build a synthetic tree: a fixture repo has no `scripts/` directory, so
    the real probe would call it stale and the fixture would be testing the
    probe rather than the file-ownership rule it means to test.
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
    if linked != 0:
        return False

    # ⛔⛔ REAL FILES ARE NOT ENOUGH — THEY ALSO HAVE TO BE CURRENT.
    #
    # Owning the files says the ratchets are about THIS box; it says nothing
    # about whether they are measuring content. On a box whose tier variants are
    # stale build output, `..._that_is_not_smaller` compares a fresh source page
    # against an old reduced one and reports a SIZE finding that is really a
    # regeneration-history finding. ⚠ Observed 2026-09-03 the hour this module
    # landed: one box named actor/author/medic/officer that way, with 82 stale
    # files under it, while a freshly regenerated box reported zero.
    #
    # ⇒ A stale box must SKIP, loudly, naming the freshness check — not produce
    # a false content finding, and not have the assertions widened to tolerate
    # it.
    return (fresh or variants_are_fresh)(repo)[0]


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

    # ⛔ DERIVE THE REASON, DO NOT ASSERT IT. The first version returned the
    # symlink sentence for EVERY checkout that had a tree, without looking at a
    # single file — so a box skipping for any other cause was told it was a
    # mirrored worktree and sent to `mirror_assets_for_worktree.py` for a
    # problem it did not have. ⚠ Which is exactly what this function's own
    # docstring says it exists to avoid, and it shipped that way for an hour.
    real, linked = _sample_tree_files(repo)
    if real + linked == 0:
        return (
            f"{len(trees)} sprite tree(s) present but holding NO png files — "
            "nothing has been generated into them, so there is no content to "
            f"ratchet. Set {CANONICAL_ENV}=1 to force these on anyway."
        )
    if linked:
        return (
            f"{len(trees)} sprite tree(s) present and {linked} of the first "
            f"{real + linked} png(s) sampled are SYMLINKS — this is a mirrored "
            "worktree borrowing another checkout's generated assets "
            "(scripts/mirror_assets_for_worktree.py), so its KNOWN_ lists "
            f"describe a tree it did not produce. Set {CANONICAL_ENV}=1 to "
            "force them on."
        )
    fresh, detail = variants_are_fresh(repo)
    if not fresh:
        return (
            f"sprite trees are this checkout's OWN ({real} real png(s) sampled, "
            "no symlinks) but the quality tiers are STALE, so a size assertion "
            "here measures regeneration history rather than content: "
            f"{detail} ⇒ Regenerate the variants, or set {CANONICAL_ENV}=1 to "
            "force these on and read the results knowing that."
        )
    return (
        "assets look canonical here — if a guard still skipped, the reason is "
        "not one this function knows about."
    )
