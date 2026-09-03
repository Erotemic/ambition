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


@pytest.fixture(scope="module")
def mixed_doc_output(tmp_path_factory) -> str:
    """ONE run over a document holding both cases.

    ⚠ Each invocation re-indexes the whole tree, so two tests asking two
    questions about one checker cost two full indexes. This is the same fix that
    took `test_planning_citations.py` from eight identical subprocesses to one --
    a lesson I applied that morning and then did not apply here.
    """
    doc = tmp_path_factory.mktemp("cites") / "row.md"
    doc.write_text(
        "an ambiguous one: `mod.rs:5`\n"
        "an unambiguous one: `scripts/check_planning_citations.py:1`\n"
    )
    return run_on(doc)[1]


def test_an_ambiguous_citation_is_reported_rather_than_guessed(mixed_doc_output):
    """⛔ `mod.rs` matches dozens of tracked files. Silently resolving against
    whichever came first is what made the checker nondeterministic; saying so is
    the honest answer, and it also tells the author to name the full path."""
    assert "AMBIGUOUS" in mixed_doc_output, mixed_doc_output


def test_an_unambiguous_citation_is_not_reported(mixed_doc_output):
    """⛔ THE PREMISE: the ambiguity report must not swallow ordinary rows. The
    unambiguous citation sits in the same document and must not appear."""
    assert "check_planning_citations.py:1" not in mixed_doc_output, mixed_doc_output


def test_the_live_planning_tree_has_no_ambiguous_citations():
    """The population, asserted rather than assumed. Four existed on
    2026-09-03; each was disambiguated by reading what the row SAYS and finding
    the file whose line matches it.

    ⚠ COMPUTED DIRECTLY RATHER THAN BY SPAWNING THE CHECKER. The full run also
    builds a 35,000-name symbol index this question does not use, and at 21 s it
    was the single slowest test in the repo-tooling lane -- more than a fifth of
    it -- for an answer that needs only the tracked-path list. Same shape as the
    eight identical subprocesses removed from `test_planning_citations.py`
    earlier the same day.
    """
    import importlib.util

    spec = importlib.util.spec_from_file_location("citations_ambig", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)

    by_suffix: dict[str, list[str]] = {}
    for rel in module.repo_files():
        by_suffix.setdefault(Path(rel).name, []).append(str(rel))

    ambiguous = []
    for doc in sorted((REPO / "docs/planning").rglob("*.md")):
        for lineno, line in enumerate(doc.read_text(errors="replace").splitlines(), 1):
            if module.MARKER in line:
                continue
            for m in module.FILE_LINE.finditer(line):
                path = m.group(1)
                hits = [h for h in by_suffix.get(Path(path).name, []) if h.endswith(path)]
                if len(hits) > 1:
                    ambiguous.append((str(doc.relative_to(REPO)), lineno, path, hits))
    assert not ambiguous, (
        "citations whose suffix matches more than one tracked file:\n"
        + "\n".join(f"  {d}:{n}  {p} -> {len(h)} candidates" for d, n, p, h in ambiguous)
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
