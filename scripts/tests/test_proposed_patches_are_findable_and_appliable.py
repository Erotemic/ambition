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
    result = subprocess.run(
        ["git", "apply", "--check", str(patch)], cwd=REPO, capture_output=True, text=True
    )
    assert result.returncode == 0, (
        f"{patch.name} no longer applies:\n{result.stderr.strip()}\n"
        "⇒ Either the tree moved under it or the patch was never test-applied. "
        "Repair it or withdraw it; a decision row pointing at an unusable answer "
        "is worse than one pointing at nothing."
    )


def test_the_patches_that_cannot_be_checked_here_are_named():
    """⛔ ABSENT IS NOT ZERO. If every patch targeted an upstream path the test
    above would skip on all of them and the file would report green while
    checking nothing. Name them out loud instead."""
    upstream = [
        patch.name for patch in patch_files()
        if not any((REPO / t).exists() for t in target_paths(patch))
    ]
    checked = len(patch_files()) - len(upstream)
    assert checked, (
        "NO patch in dev/patches/ targets a path in this tree, so the "
        f"apply check verified nothing. Upstream-only: {upstream}"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
