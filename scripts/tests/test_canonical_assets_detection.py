"""The canonical-box detector must not turn ten ratchets on where they lie.

Ten asset ratchets were gated on `AMBITION_ASSETS_ARE_CANONICAL`, which ⛔
**nothing in the repository ever set** — so no lane had evaluated them while two
planning paragraphs called them ratchets. `scripts/lib/canonical_assets.py`
replaces the variable with a detection: a checkout whose sprite tree holds REAL
FILES generated them; one holding SYMLINKS is borrowing another checkout's via
`mirror_assets_for_worktree.py`.

⚠ **Both directions are dangerous and neither is the safe default:**

* a false YES turns ten guards red on a box whose `KNOWN_` lists never described
  it — the exact failure the original gate was added to prevent;
* a false NO is where this started: ten assertions that never run, and prose
  calling them a ratchet.

⛔ **THE ENABLED ARM IS UNEXERCISED WHERE THIS RUNS.** These tests build real and
symlinked trees under `tmp_path` and check the ANSWER; "the ten ratchets pass on
the canonical box" is a claim only that box can make, and nothing here should be
read as evidence for it.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts" / "lib"))

import canonical_assets as ca  # noqa: E402


def _tree(root: Path, *, files: int, linked: bool) -> Path:
    """A sprite tier tree, either generated here or mirrored from elsewhere."""
    tree = root / "crates/ambition_platformer2d_actor_monolith/assets/sprites"
    tree.mkdir(parents=True, exist_ok=True)
    source = root / "elsewhere"
    source.mkdir(exist_ok=True)
    for i in range(files):
        target = source / f"{i}.png"
        target.write_bytes(b"\x89PNG")
        png = tree / f"{i}.png"
        if linked:
            png.symlink_to(target)
        else:
            png.write_bytes(b"\x89PNG")
    return tree


#: A fixture repo has no `scripts/` directory, so the real freshness probe would
#: call it stale and these tests would be exercising the probe instead of the
#: file-ownership rule they mean to. Injected, never defaulted.
FRESH = lambda _repo: (True, "stubbed fresh")  # noqa: E731


def test_a_tree_of_real_files_is_canonical(tmp_path):
    """The box that generated the assets is the box whose lists describe them."""
    _tree(tmp_path, files=3, linked=False)
    assert ca.assets_are_canonical(tmp_path, env={}, fresh=FRESH) is True


def test_real_files_that_are_STALE_are_not_canonical(tmp_path):
    """⛔⛔ OWNING THE FILES IS NOT THE SAME AS THEM BEING CURRENT.

    On a box whose tier variants are stale build output, the size ratchet
    compares a fresh source page against an old reduced one and reports a SIZE
    finding that is really a REGENERATION-HISTORY finding. Observed
    2026-09-03 the hour this module landed: one box named four sheets that way
    with 82 stale files under it, while a freshly regenerated box reported
    zero. ⇒ A stale box must skip, not produce a false content finding.
    """
    _tree(tmp_path, files=3, linked=False)
    stale = lambda _repo: (False, "3 variant(s) older than their source.")  # noqa: E731
    assert ca.assets_are_canonical(tmp_path, env={}, fresh=stale) is False


def test_the_stale_skip_reason_names_the_freshness_check(tmp_path):
    """⭐ A skip a reader cannot act on is how the ten sat unevaluated.

    The reason must say WHICH check said stale, so the fix (regenerate) is one
    command away rather than a hunt.
    """
    _tree(tmp_path, files=3, linked=False)
    reason = ca.why_not(tmp_path)
    assert "STALE" in reason or "stale" in reason, reason
    assert "check_quality_variants_are_fresh" in reason, reason


def test_a_mirrored_tree_of_symlinks_is_not(tmp_path):
    """A worktree borrowing the main checkout's assets must not ratchet them —
    this is the case the live repo is in while these tests run."""
    _tree(tmp_path, files=3, linked=True)
    assert ca.assets_are_canonical(tmp_path, env={}) is False


def test_no_tree_at_all_is_not_canonical(tmp_path):
    """A fresh clone has never generated anything, so it has nothing of its own
    to check. ⛔ Absent must not read as clean."""
    assert ca.assets_are_canonical(tmp_path, env={}) is False


def test_one_regenerated_file_does_not_make_a_mirrored_tree_canonical(tmp_path):
    """⛔ THE WHOLE POINT OF THE PER-FILE MIRROR is that a regenerated sprite
    lands as a REAL file beside the links. If a single real file flipped the
    answer, regenerating one sprite in a worktree would switch ten ratchets on
    against a tree that is still 99% borrowed."""
    tree = _tree(tmp_path, files=20, linked=True)
    (tree / "regenerated.png").write_bytes(b"\x89PNG")
    assert ca.assets_are_canonical(tmp_path, env={}) is False


def test_the_env_variable_still_forces_it_on(tmp_path):
    """The documented opt-in predates the detection and stays: it is the only way
    to exercise the enabled path where the detection says no."""
    _tree(tmp_path, files=3, linked=True)
    assert ca.assets_are_canonical(tmp_path, env={ca.CANONICAL_ENV: "1"}) is True


def test_the_skip_reason_names_what_it_looked_at(tmp_path):
    """⛔ A SKIP THAT ONLY SAYS "set this variable" is how ten assertions sat
    unevaluated. The reason must distinguish the two NOs, because they call for
    opposite actions — regenerate, versus you are in a worktree and should not."""
    empty = ca.why_not(tmp_path)
    assert "never regenerated" in empty

    _tree(tmp_path, files=3, linked=True)
    mirrored = ca.why_not(tmp_path)
    assert "SYMLINKS" in mirrored and "mirror_assets_for_worktree" in mirrored
    assert mirrored != empty


def test_the_live_repo_answers_and_the_answer_matches_its_tree(tmp_path):
    """Run against the real checkout: whatever it says, it must AGREE with what
    the tree actually is. A detector that cannot be wrong here is one that
    hard-codes its answer."""
    trees = ca.sprite_trees(REPO)
    verdict = ca.assets_are_canonical(REPO, env={})
    if not trees:
        assert verdict is False
        return
    pngs = [p for tree in trees for p in list(tree.rglob("*.png"))[:5]]
    if not pngs:
        assert verdict is False
        return
    any_linked = any(p.is_symlink() for p in pngs)
    assert verdict is (not any_linked), (
        f"the detector says canonical={verdict} while the tree "
        f"{'holds symlinks' if any_linked else 'holds real files'}"
    )


def test_the_predicate_and_the_reason_share_one_freshness_run(tmp_path, monkeypatch):
    """⛔⛔ A REASON DERIVED FROM A DIFFERENT RUN THAN THE DECISION CAN CONTRADICT IT.

    `assets_are_canonical` and `why_not` both need the freshness answer, and the
    first version of each shelled out ITSELF — two executions of
    `check_quality_variants_are_fresh.py` over a MUTABLE generated tree, which
    can disagree if anything regenerates between them. ⚠ That is the same defect
    the symlink path already had and fixed by sharing `_sample_tree_files`,
    reintroduced one function later; a guard asserting the two agree is written
    against this property, so it is pinned rather than left as a comment.
    """
    _tree(tmp_path, files=3, linked=False)
    calls = []

    def counted(repo):
        calls.append(repo)
        return (True, "counted")

    ca._freshness.cache_clear()
    monkeypatch.setattr(ca, "_run_freshness_check", counted)
    try:
        ca.assets_are_canonical(tmp_path, env={})
        ca.why_not(tmp_path)
        assert len(calls) == 1, (
            f"the freshness checker ran {len(calls)} times for one question — "
            "the predicate and the reason can now disagree"
        )
    finally:
        ca._freshness.cache_clear()
