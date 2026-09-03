"""A patch in `dev/patches/` must be FINDABLE and must still APPLY.

These are proposed-not-applied changes: each one is a decision waiting on Jon,
with the implementation already written. That only works if the row asking the
question points at the patch answering it.

⛔ ON 2026-09-03 ONE OF THE FOUR WAS NAMED BY NOTHING.
`portrait-tiers-are-never-baked-20260902.patch` had existed for a day; its
question WAS recorded in `awaiting-maintainer-decision.md` and that row did not
mention the patch, so the answer sat in a directory nobody had reason to open.
A deferral nobody can find becomes the real rule by default — the decision gets
made by nothing happening.

⚠ THIS DOES NOT CHECK THAT THE PATCH APPLIES. It checks discoverability only,
which is the failure that actually occurred. A patch that has gone stale against
the tree is a different problem and needs `git apply --check`, which costs a
worktree and is not what this guard is for.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
PATCHES = REPO / "dev/patches"


def patch_files() -> list[Path]:
    return sorted(PATCHES.glob("*.patch")) if PATCHES.is_dir() else []


def prose_naming(name: str) -> list[str]:
    """Markdown under docs/ or dev/ that names the patch, excluding the patch
    directory itself — a patch mentioning its own filename proves nothing."""
    result = subprocess.run(
        ["git", "grep", "-l", "--fixed-strings", name, "--", "docs/**/*.md", "dev/**/*.md"],
        cwd=REPO, capture_output=True, text=True,
    )
    return [p for p in result.stdout.split() if not p.startswith("dev/patches/")]


def test_there_are_patches_to_check():
    """⛔ THE PREMISE. With an empty directory the parametrised test below
    collects nothing and the file passes while checking no patch at all."""
    assert patch_files(), "no patches found — has dev/patches/ moved?"


@pytest.mark.parametrize("patch", patch_files(), ids=lambda p: p.name)
def test_a_document_names_this_patch(patch: Path):
    naming = prose_naming(patch.name)
    assert naming, (
        f"nothing under docs/ or dev/ names `{patch.name}`, so the decision it "
        "implements cannot be found from the row that asks for it. Name it in "
        "the planning row that raises the question."
    )


#: Phrases that tell a reader the file is a PROPOSAL rather than a record of
#: something that landed. The upstream patch states it as an instruction
#: ("Apply to a fork of leafwing-input-manager v0.20.0, then wire it in"), which
#: is the same fact in the imperative.
STATUS_PHRASES = ("NOT APPLIED", "PROPOSED", "APPLY TO", "APPLY IT")


@pytest.mark.parametrize("patch", patch_files(), ids=lambda p: p.name)
def test_the_patch_announces_that_it_has_not_landed(patch: Path):
    """A patch file read out of context looks exactly like a commit that
    landed, so it has to say otherwise in its own first lines."""
    head = patch.read_text(errors="replace")[:600].upper()
    assert any(phrase in head for phrase in STATUS_PHRASES), (
        f"{patch.name} does not say it is unapplied in its opening lines"
    )


def owning_repo(target: str) -> tuple[Path, int]:
    """The repo that actually HOLDS `target`, and the `-p` level to reach it.

    ⛔ A PATH INSIDE A SUBMODULE IS NOT IN THE OUTER REPO'S INDEX. `git apply`
    from the repo root can read those files off disk, so a working-tree check
    appears to work — but `--cached` there sees a gitlink and nothing else, so
    the committed-state comparison below has to run INSIDE the submodule.
    """
    parts = Path(target).parts
    for depth in range(len(parts) - 1, 0, -1):
        candidate = REPO.joinpath(*parts[:depth])
        if (candidate / ".git").exists():
            return candidate, 1 + depth
    return REPO, 1


def apply_check(repo: Path, patch: Path, strip: int, cached: bool) -> subprocess.CompletedProcess:
    argv = ["git", "apply", "--check", f"-p{strip}"]
    if cached:
        argv.append("--cached")
    return subprocess.run(
        argv + [str(patch)], cwd=repo, capture_output=True, text=True
    )


def dirty_targets(repo: Path, targets: list[str], strip: int) -> list[str]:
    """Which of the patch's targets have uncommitted edits in their own repo."""
    inside = ["/".join(Path(t).parts[strip - 1:]) for t in targets]
    result = subprocess.run(
        ["git", "status", "--porcelain", "--"] + inside,
        cwd=repo, capture_output=True, text=True,
    )
    return [line[3:].strip() for line in result.stdout.splitlines() if line.strip()]


def target_paths(patch: Path) -> list[str]:
    """The `b/` side of every file header — what the patch would write."""
    return [
        line[6:].strip()
        for line in patch.read_text(errors="replace").splitlines()
        if line.startswith("+++ b/")
    ]


@pytest.mark.parametrize("patch", patch_files(), ids=lambda p: p.name)
def test_a_patch_against_this_tree_still_applies(patch: Path):
    """⛔⛔ THE FAILURE THIS GUARD EXISTS FOR, and it is not hypothetical.
    `leafwing-0.20-pressall-shortcircuit.patch` sat for SIX WEEKS described as
    *"the exact patch, with rationale and wiring instructions"* while its hunk
    header (`@@ -173,6 +173,11 @@`) disagreed with its own body — both
    `git apply` and `patch(1)` refused it. It had been written by hand and never
    test-applied. Found and repaired 2026-09-03; nothing would have caught it.

    ⚠ A patch can also go stale silently as the tree moves under it, which has
    the same end state: a decision row pointing at an answer that cannot be used.

    ⓘ UPSTREAM PATCHES ARE NOT CHECKABLE HERE and are not silently skipped —
    see the companion test below, which asserts they are reported.
    """
    targets = target_paths(patch)
    assert targets, f"{patch.name} has no `+++ b/` file header — is it a patch?"
    if not any((REPO / t).exists() for t in targets):
        pytest.skip(f"upstream patch; none of {targets} exists in this tree")
    repo, strip = owning_repo(targets[0])
    result = apply_check(repo, patch, strip, cached=False)
    if result.returncode == 0:
        return

    # ⛔⛔ "DOES NOT APPLY" HAS TWO CAUSES AND ONLY ONE OF THEM IS A DEFECT.
    #
    # A patch is stale when the COMMITTED tree moved under it — that is the
    # finding, and it means repair or withdraw. A patch also stops applying
    # while somebody has uncommitted edits to the same files, which says
    # nothing about the patch at all. ⚠ Found 2026-09-03: this test failed on
    # `ldtk-player-tileset-retarget-20260902.patch` with "the tree moved under
    # it or the patch was never test-applied" while the patch applied CLEANLY
    # to the index — five `.ldtk` worlds in the `game/ambition_map_assets`
    # submodule were dirty from another session's work in progress. Acting on
    # that reading would have withdrawn a correct patch, or regenerated it
    # against somebody else's unfinished edits and baked them in.
    #
    # ⇒ Ask the index before accusing the patch.
    against_index = apply_check(repo, patch, strip, cached=True)
    if against_index.returncode == 0:
        dirty = dirty_targets(repo, targets, strip)
        pytest.skip(
            f"{patch.name} applies to the committed tree but not the working "
            f"one: {len(dirty)} of its target(s) have uncommitted edits "
            f"({', '.join(dirty[:4])}{'…' if len(dirty) > 4 else ''}) in "
            f"{repo.name}. The patch is fine; this checkout is mid-edit."
        )

    assert result.returncode == 0, (
        f"{patch.name} no longer applies:\n{result.stderr.strip()}\n"
        f"⇒ And it does not apply to {repo.name}'s COMMITTED state either, so "
        "this is not somebody's work in progress: either the tree moved under "
        "it or the patch was never test-applied. Repair it or withdraw it; a "
        "decision row pointing at an unusable answer is worse than one "
        "pointing at nothing."
    )


def test_the_patches_that_cannot_be_checked_here_are_named():
    """⛔ ABSENT IS NOT ZERO. If every patch targeted an upstream path the test
    above would skip on all of them and the file would report green while
    checking nothing. Name them out loud instead.

    ⚠ THERE ARE NOW TWO WAYS TO SKIP, and the second one is reachable by
    accident: a checkout with uncommitted edits to every patched file would
    skip everything and still show green. Both are counted here, so the
    apply check cannot quietly stop checking anything.
    """
    upstream, mid_edit = [], []
    for patch in patch_files():
        targets = target_paths(patch)
        if not any((REPO / t).exists() for t in targets):
            upstream.append(patch.name)
            continue
        repo, strip = owning_repo(targets[0])
        if apply_check(repo, patch, strip, cached=False).returncode != 0 and \
                apply_check(repo, patch, strip, cached=True).returncode == 0:
            mid_edit.append(patch.name)
    checked = len(patch_files()) - len(upstream) - len(mid_edit)
    assert checked, (
        "NO patch in dev/patches/ was actually apply-checked, so this file "
        f"verified nothing. Upstream-only: {upstream}. Blocked by uncommitted "
        f"edits in this checkout: {mid_edit} — commit or stash that work and "
        "run again; those patches are unverified, not passing."
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
