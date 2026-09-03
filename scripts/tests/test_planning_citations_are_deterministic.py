"""The citation checker's answer must be a function of its input.

⛔⛔ IT WAS NOT. `file.rs:123` is matched by SUFFIX, so `game_assets/mod.rs`
matches two tracked files — and the line-number check read `hits[0]`, an
ARBITRARY candidate. The same citation came back "1 unresolved" and
"all resolved" on alternate runs over an unchanged tree.

⚠ THAT COST AN HOUR AND A WRONG CONCLUSION. I observed the flip four times,
always just after a `git fetch`, and wrote it up as a probable race between the
checker's `git ls-files` and another agent's index write — a plausible mechanism,
recorded honestly as unconfirmed, and WRONG. Racing my own git reproduced it, but
the tracked-file COUNT was identical (3293) in the clean and failing runs, and
the failing citation was always the same line. The tell was in the data I already
had.

⇒ A checker whose answer depends on `git ls-files` ordering is worse than no
checker: it teaches its reader that failures are noise.
"""

from __future__ import annotations

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


def run_on(doc: Path) -> tuple[int, str]:
    proc = subprocess.run(
        [sys.executable, str(SCRIPT), str(doc)],
        cwd=REPO, capture_output=True, text=True,
    )
    return proc.returncode, proc.stdout


def test_the_same_document_gives_the_same_answer_twice(tmp_path):
    """⭐ THE PROPERTY. Two runs over one unchanged document must agree."""
    doc = tmp_path / "row.md"
    doc.write_text(
        "A citation whose suffix matches more than one file: `mod.rs:5`.\n"
        "And one that matches none: `no_such_file_anywhere.rs:5`.\n"
    )
    first, second = run_on(doc), run_on(doc)
    assert first == second, (
        "the checker gave two different answers for one input:\n"
        f"--- first ---\n{first[1]}\n--- second ---\n{second[1]}"
    )


def test_an_ambiguous_citation_is_reported_rather_than_guessed(tmp_path):
    """⛔ `mod.rs` matches dozens of tracked files. Silently resolving against
    whichever came first is what made the checker nondeterministic; saying so is
    the honest answer, and it also tells the author to name the full path."""
    doc = tmp_path / "row.md"
    doc.write_text("see `mod.rs:5`\n")
    _code, out = run_on(doc)
    assert "AMBIGUOUS" in out, out


def test_an_unambiguous_citation_still_passes(tmp_path):
    """⛔ THE PREMISE: the ambiguity report must not swallow ordinary rows."""
    doc = tmp_path / "row.md"
    doc.write_text("see `scripts/check_planning_citations.py:1`\n")
    _code, out = run_on(doc)
    assert "all resolved" in out, out


def test_the_live_planning_tree_has_no_ambiguous_citations():
    """The population, asserted rather than assumed. Four existed on
    2026-09-03; each was disambiguated by reading what the row SAYS and finding
    the file whose line matches it."""
    proc = subprocess.run(
        [sys.executable, str(SCRIPT)], cwd=REPO, capture_output=True, text=True
    )
    assert "AMBIGUOUS" not in proc.stdout, proc.stdout


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
