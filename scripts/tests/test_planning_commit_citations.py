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


def test_the_live_planning_tree_has_no_fabricated_commit():
    """The population this guard was built on, asserted rather than remembered."""
    module = load()
    docs = sorted((REPO / "docs/planning").rglob("*.md"))
    findings, _ = module.unresolved_commits(REPO, docs)
    assert findings == [], (
        "a planning row cites a commit no repository holds:\n"
        + "\n".join(f"  {f}:{n}  {c}" for f, n, c in findings)
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
