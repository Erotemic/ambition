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


#: Four placements of `cite-ok` around a citation on line 3. Two suppress and
#: two must not.
#:
#: ⛔⛔ THE "NOT HONOURED" ARMS ARE THE ONES THAT MATTER. "Honours the next line"
#: and "honours anything anywhere" print the same green on the first two arms
#: alone, so a change that simply stopped checking placement would pass every
#: test one would naturally write.
_MARKER_SCOPE_CASES = [
    ("on the citation's own line", 3, True),
    ("on the line directly below", 4, True),
    ("two lines below", 5, False),
    ("on the line above", 2, False),
]


@pytest.mark.parametrize("where, marker_line, suppressed", _MARKER_SCOPE_CASES)
def test_the_marker_scope_is_the_citation_line_and_the_one_below(
    where, marker_line, suppressed
):
    """⭐ ONE KEEPER, ASSERTED. The checker documents this scope, and the live-tree
    guard above now asks the checker rather than re-spelling it.

    MEASURED 2026-09-10: the guard used to accept the marker only on the SAME
    line, so a marker placed where the documentation allows passed
    `check_planning_citations.py --strict` and reddened the repo-tooling lane.
    **An author who followed the documentation got a red.**
    """
    import importlib.util

    spec = importlib.util.spec_from_file_location("citations_scope", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)

    lines = ["intro", "context", "a citation: `some/file.rs:1`", "tail", "further"]
    lines[marker_line - 1] += f"  <!-- {module.MARKER} -->"
    assert module.marker_suppresses(lines, 3) is suppressed, (
        f"a marker {where} should "
        f"{'suppress' if suppressed else 'NOT suppress'} the citation on line 3"
    )


def test_the_marker_scope_holds_at_the_end_of_a_document():
    """⚠ THE OFF-BY-ONE THE NEXT-LINE RULE INVITES. A citation on the LAST line
    has no line below it, and the predicate must answer without reading past the
    end."""
    import importlib.util

    spec = importlib.util.spec_from_file_location("citations_scope_eof", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)

    assert module.marker_suppresses(["a citation: `x.rs:1`"], 1) is False
    assert module.marker_suppresses(
        [f"a citation: `x.rs:1`  <!-- {module.MARKER} -->"], 1
    ) is True


def _checker_module():
    import importlib.util

    spec = importlib.util.spec_from_file_location("citations_lane", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize("marker_offset, reported", [(1, False), (2, True)])
def test_the_vanished_lane_honours_the_marker_scope_the_predicate_documents(
    tmp_path, marker_offset, reported, capsys
):
    """⛔⛔ THE PREDICATE HAVING ONE KEEPER IS NOT THE SAME CLAIM AS THE LANES
    CALLING IT. `marker_suppresses` was tested directly and passed, while the
    `--vanished` and commit lanes each re-spelled `MARKER in line` and accepted
    the marker ONLY on the citation's own line. MEASURED 2026-09-11: a marker
    placed exactly where the documentation says silenced the resolution lane and
    left `--vanished` red on the same row, so an author following the docs got a
    red from one lane and a green from the other.

    ⇒ This exercises the LANE, not the predicate: the defect lived entirely in
    whether the lane asked.

    ⭐ THE VANISHED NAME IS DERIVED, never hard-coded — a name that comes back
    would make this fixture pass while testing nothing.
    """
    module = _checker_module()
    baseline = re.search(
        r'PLANNING_VANISHED_BASELINE = "([0-9a-f]+)"',
        (REPO / "scripts/run_tests.py").read_text(),
    ).group(1)
    gone = sorted(
        module.item_names(module.source_text_at(baseline))
        - module.item_names(module.source_text())
        - module.crate_names()
    )
    assert gone, (
        f"no name defined at {baseline} is missing at HEAD, so this fixture "
        "cannot cite a vanished one and would certify nothing"
    )
    name = gone[0]

    doc = tmp_path / "row.md"
    lines = [f"A row citing `{name}` on purpose."]
    lines += [""] * (marker_offset - 1)
    lines.append("<!-- cite-ok: recorded deliberately -->")
    doc.write_text("\n".join(lines) + "\n")

    module.vanished_report([doc], baseline, set())
    out = capsys.readouterr().out
    hit = name in out
    assert hit is reported, (
        f"a marker {marker_offset} line(s) below its citation should "
        f"{'not suppress' if reported else 'suppress'} it in the --vanished "
        f"lane, but it {'reported' if hit else 'did not report'}:\n{out}"
    )


@pytest.mark.parametrize("marker_offset, reported", [(1, False), (2, True)])
def test_the_commit_lane_honours_the_marker_scope(tmp_path, marker_offset, reported):
    """The same claim for the third reader of the marker."""
    module = _checker_module()
    doc = tmp_path / "row.md"
    sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    lines = [f"A row citing commit `{sha}` on purpose."]
    lines += [""] * (marker_offset - 1)
    lines.append("<!-- cite-ok: recorded deliberately -->")
    doc.write_text("\n".join(lines) + "\n")

    findings, _ = module.unresolved_commits(REPO, [doc])
    hit = any(sha in str(f) for f in findings)
    assert hit is reported, (
        f"a marker {marker_offset} line(s) below its citation should "
        f"{'not suppress' if reported else 'suppress'} it in the commit lane, "
        f"but it {'reported' if hit else 'did not report'}: {findings}"
    )


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
        doc_lines = doc.read_text(errors="replace").splitlines()
        for lineno, line in enumerate(doc_lines, 1):
            # ⛔⛔ THE CHECKER'S OWN PREDICATE, NOT A SECOND SPELLING OF IT.
            # This read the marker only on the citation's OWN line while
            # `check_planning_citations.py` documents the line below as legal
            # too — so a marker placed where the documentation allows passed
            # `--strict` and reddened THIS test (2026-09-10). One keeper now.
            if module.marker_suppresses(doc_lines, lineno):
                continue
            for m in module.FILE_LINE.finditer(line):
                path = m.group(1)
                hits = [h for h in by_suffix.get(Path(path).name, []) if h.endswith(path)]
                if len(hits) > 1:
                    ambiguous.append((str(doc.relative_to(REPO)), lineno, path, hits))
    assert not ambiguous, (
        "citations whose suffix matches more than one tracked file:\n"
        + "\n".join(f"  {d}:{n}  {p} -> {len(h)} candidates" for d, n, p, h in ambiguous)
        + "\n\nName the full path. If the ambiguous form is deliberate -- a row"
        + f"\nquoting it AS the example -- put `{module.MARKER}` on the citation's own"
        + "\nline or on the line directly below it. Those two placements are the"
        + "\nwhole scope: two lines below, or the line above, do not suppress."
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
