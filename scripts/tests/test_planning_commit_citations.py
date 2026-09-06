"""A backticked commit name in a planning row must name a real commit.

`check_planning_citations.py` guarded symbols and file paths and never looked at
SHAs, so a fabricated one was invisible exactly the way a fabricated symbol used
to be — it looks like evidence and nobody types it into `git show`. The survey
that prompted this found 225 commit citations across `docs/planning` and ZERO
fabricated, which is the argument for guarding it NOW rather than after the
first one.

⛔ THE FALSE-POSITIVE CLASS IS THE HARD PART, and a first pass fell into it: a
`[0-9a-f]{7,40}` match reports twenty ROLLBACK CHECKSUMS from
`runtime-frame-history.md` (16 hex) and campaign hashes (32 hex) as unresolved
commits. Git abbreviations here are 7-12, or 40 in full, and nothing else may be
asked about.

⭐ AND SUBMODULE COMMITS ARE REAL CITATIONS THIS OBJECT STORE CANNOT SEE. Seven
of the eight the survey flagged were submodule commits, each saying so in its own
prose. The checker asks the submodule, which is a fact; reading the sentence to
guess would be a heuristic.
"""

from __future__ import annotations

import importlib.util
import re
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_planning_citations.py"


def load():
    spec = importlib.util.spec_from_file_location("citations_commits", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def module():
    return load()


@pytest.fixture
def doc(tmp_path):
    def write(text: str) -> Path:
        p = tmp_path / "row.md"
        p.write_text(text)
        return p
    return write


def test_a_real_commit_resolves(module, doc):
    head = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"], cwd=REPO,
        capture_output=True, text=True,
    ).stdout.strip()
    findings, _ = module.unresolved_commits(REPO, [doc(f"Landed at `{head}`.\n")])
    assert findings == []


def test_a_fabricated_commit_is_reported(module, doc):
    findings, _ = module.unresolved_commits(
        REPO, [doc("Landed at `deadbeef1234`.\n")]
    )
    assert len(findings) == 1 and "deadbeef1234" in findings[0][2]


@pytest.mark.parametrize(
    "value",
    [
        "c0312413be50c0312413be50c031",       # 28 hex — neither shape
        "30372c4b715e7f699aae56a3617407e0",   # 32 hex — a campaign hash
        "ce1a84c41b124c2d",                   # 16 hex — a ROLLBACK CHECKSUM
    ],
)
def test_a_hash_that_is_not_a_git_abbreviation_is_never_asked_about(module, doc, value):
    """⛔ This is the check that keeps the guard readable. Twenty of these were
    reported as unresolved commits by the first version."""
    findings, _ = module.unresolved_commits(REPO, [doc(f"checksum `{value}`\n")])
    assert findings == [], f"`{value}` is not a commit name and must not be asked about"


def test_the_marker_suppresses_a_deliberate_non_commit(module, doc):
    """The one real case in the tree: an md5 of a capture, labelled as an md5."""
    findings, _ = module.unresolved_commits(
        REPO, [doc(f"md5 `c0312413be50` {module.MARKER}\n")]
    )
    assert findings == []


def test_a_submodule_commit_resolves(module, doc):
    """⭐ The seven real ones. Uses a commit the submodule actually holds rather
    than a fixture, because the point is that THIS object store cannot see it."""
    subs = module.submodule_paths(REPO)
    if not subs:
        pytest.skip("no initialised submodules on this checkout")
    sub = subs[0]
    sha = subprocess.run(
        ["git", "-C", sub, "rev-parse", "--short", "HEAD"], cwd=REPO,
        capture_output=True, text=True,
    ).stdout.strip()
    here = subprocess.run(
        ["git", "cat-file", "-e", f"{sha}^{{commit}}"], cwd=REPO, capture_output=True
    )
    if here.returncode == 0:
        pytest.skip("this submodule's HEAD is also an object of the superproject")
    findings, _ = module.unresolved_commits(REPO, [doc(f"submodule commit `{sha}`\n")])
    assert findings == [], (
        f"`{sha}` is {sub}'s HEAD and must resolve there, not be reported"
    )


def test_a_commit_reachable_from_no_ref_is_reported(module, doc):
    """⛔⛔ EXISTING IS NOT FINDABLE. A REBASED commit stays in the local object
    store, so `cat-file -e` says yes while nobody else can fetch it. That is not
    hypothetical: `a3924b2b2` was cited in queue.md as the fix for a union
    failure and is that commit's pre-rebase name; a reviewer read it as
    fabricated.

    ⚠ The dangling commit is BUILT here rather than borrowed from the reflog, so
    the test does not depend on this checkout's history. `commit-tree` writes an
    object with no ref pointing at it -- harmless, and collected by the next
    `git gc`.
    """
    tree = subprocess.run(
        ["git", "rev-parse", "HEAD^{tree}"], cwd=REPO,
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    dangling = subprocess.run(
        ["git", "commit-tree", tree, "-m", "unreferenced probe"], cwd=REPO,
        capture_output=True, text=True, check=True,
    ).stdout.strip()
    findings, _ = module.unresolved_commits(REPO, [doc(f"citing `{dangling[:9]}`\n")])
    assert len(findings) == 1, f"a commit on no ref must be reported: {findings}"
    assert "reachable from NO ref" in findings[0][2], findings[0][2]


def grandfathered() -> set[str]:
    """Commit citations that existed in `docs/planning` when the epoch was cut.

    ⭐⭐ DERIVED, NOT STORED — 2026-09-06. This read
    `dev/epoch-grandfathered-citations.txt`, a committed 381-line list. Measured:
    the list and this derivation agree EXACTLY, 381 = 381, zero difference in
    either direction. ⇒ the file was a second copy of a fact the repository
    already holds, and the epoch tree is the copy that cannot drift.

    ⛔⛔ AND THE STORED FORM HAD A HOLE THE DERIVED FORM CANNOT HAVE. This set is
    an AMNESTY: a name in it is exempt from "resolves nowhere ⇒ fabrication". With
    a file, the way to silence a real fabrication was to append one line to it —
    the guard would then certify its own blind spot, and the diff would look like
    housekeeping. You cannot append to the epoch's tree.

    ⚠ It answers about the DOCUMENTS, so it is identical on every checkout — which
    is the property the whole guard was rewritten for (a count over the object
    store is not an invariant; a shallow clone answers differently). One batched
    `git cat-file`, ~86 ms.
    """
    epoch = subprocess.run(
        ["git", "rev-list", "--max-parents=0", "HEAD"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout.split()[0]
    files = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", epoch, "docs/planning"],
        cwd=REPO, capture_output=True, text=True, check=True,
    ).stdout.split()
    blobs = subprocess.run(
        ["git", "cat-file", "--batch"],
        cwd=REPO, input="".join(f"{epoch}:{f}\n" for f in files),
        capture_output=True, text=True, check=True,
    ).stdout
    module = load()
    names: set[str] = set()
    for line in blobs.splitlines():
        if module.MARKER in line:
            continue
        for match in module.COMMIT.finditer(line):
            names.add(match.group(1))
    assert names, (
        "the epoch tree yielded NO citations, so every unresolved sha would read "
        "as a fabrication.\n  Either `docs/planning` did not exist at the epoch "
        "root, or this checkout cannot read it."
    )
    return names


def test_no_unresolvable_citation_that_the_epoch_did_not_grandfather():
    """Every sha that fails to resolve must be one the epoch left behind.

    ⛔⛔ THIS WAS A COUNT AND THE COUNT WAS NOT AN INVARIANT. It asserted
    `len(findings) == 576`, which is what THIS checkout happens to fail to
    resolve; a reviewer with deliberately shallow submodule history measured 615
    from the same source. ⇒ A number derived from the OBJECT STORE cannot gate a
    property of the DOCUMENTS.

    ⭐ MEMBERSHIP INSTEAD, derived from the documents and therefore identical on
    every checkout: a citation written before the epoch is grandfathered (and
    readable in the history store `.git-epoch.yaml` names); one written after
    must resolve normally. An unresolved sha that is NOT grandfathered is new,
    and is exactly the fabrication this guard exists for.
    ⚠ It also cannot be defeated the way a count could -- by deleting one stale
    citation while adding one fabricated one.
    """
    module = load()
    docs = sorted((REPO / "docs/planning").rglob("*.md"))
    findings, _ = module.unresolved_commits(REPO, docs)
    allowed = grandfathered()
    # ⚠ Findings carry the citation as it appears in prose, backticks included;
    # the grandfathered set holds bare names. Compare the NAMES.
    #
    # ⛔⛔ AND `strip("`")` WAS NOT ENOUGH, WHICH MADE THIS GUARD REPORT 365
    # FINDINGS WHERE THERE WAS ONE. `unresolved_commits` appends a DIAGNOSIS to
    # the citation when the object is present but unreachable — "`00030e603`
    # (exists locally, reachable from NO ref — rebased away or on a deleted
    # branch; push it or cite the commit that landed)" — so stripping backticks
    # leaves a SENTENCE, and a sentence never matches a bare sha. Every
    # annotated finding was therefore reported as ungrandfathered.
    #
    # ⚠ IT IS INVISIBLE ON A CLONE THAT LACKS THE PRE-EPOCH OBJECTS, which is why
    # it was green on one machine and red on another from the same source: the
    # annotation only appears for objects the store HAS. A checkout that predates
    # the truncation carries them and sees 365; a fresh one carries none and sees
    # the bare form that happens to compare correctly. ⇒ The guard's own note
    # about a reviewer measuring a different number from the same documents was
    # about the FINDINGS; this is the same hazard reaching the COMPARISON.
    #
    # ⇒ Take the sha, not the prose around it. `unresolved_commits` guarantees a
    # citation starts with its hex run; anything after it is commentary this
    # comparison must not read.
    def sha_name(cite: str) -> str:
        match = re.match(r"[0-9a-f]+", cite.strip("`"))
        assert match, f"a finding that does not start with a sha: {cite!r}"
        return match.group(0)

    ungrandfathered = sorted({sha_name(cite) for _, _, cite in findings} - allowed)
    assert not ungrandfathered, (
        f"{len(ungrandfathered)} commit citation(s) resolve nowhere and were NOT "
        f"cited at the epoch:\n  " + "\n  ".join(ungrandfathered[:10]) + "\n\n"
        "  Each is either a FABRICATION or a commit that exists only on the "
        "machine that wrote it.\n"
        "  A pre-epoch citation is readable in the history store named by "
        "`.git-epoch.yaml`.\n"
    )


def _retired_absolute_assertion():
    module = load()
    docs = sorted((REPO / "docs/planning").rglob("*.md"))
    findings, _ = module.unresolved_commits(REPO, docs)
    assert findings == [], (
        # ⛔⛔ THE FIRST LINE USED TO SAY "no repository holds", WHICH IS A CLAIM
        # THAT CAN BE FALSE. A superproject `git fetch` does NOT fetch submodule
        # branches, so a commit that exists on the machine that wrote the row is
        # simply absent here — measured 2026-09-04, where one
        # `git -C tools/ambition_music_renderer fetch --all` took this arm from
        # 1 failed to 9 passed with nothing edited. ⇒ The old wording sent a
        # reader to correct a citation that was already correct, and cost about
        # forty minutes.
        # ⚠ It is also the wording a REWRITTEN sha deserves, and those two look
        # identical here — same assertion, same message, opposite remedies. So
        # the message now names both roads and the one command that tells them
        # apart.
        "a planning row cites a commit this checkout cannot resolve:\n"
        + "\n".join(f"  {f}:{n}  {c}" for f, n, c in findings)
        + "\n\n  ⚠ THIS MAY BE YOUR CHECKOUT RATHER THAN THE CITATION."
        "\n  A superproject fetch does not fetch submodule branches, so a"
        "\n  commit written on another machine can be absent here."
        "\n    1. `git submodule foreach git fetch` and re-run — if it clears,"
        "\n       nothing was ever wrong with the row."
        "\n    2. Still missing? `git cat-file -t <sha>` in the submodule."
        "\n       Resolves there  -> foreign commit: name its repo for readers."
        "\n       Resolves nowhere -> rewritten by a rebase: replace it with"
        "\n       the surviving commit, do not merely qualify it."
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
