"""A markdown table interrupted by prose stops being a table.

⛔⛔ FOUND LIVE FOUR TIMES ON 2026-09-06, all in tables a reader is meant to scan,
and nothing catches it — the source reads fine and only a rendered view shows the
damage:

* the campaign's proof-move inventory — an aside between rows 4 and 5, so **9 of
  13 rows** rendered as literal pipes;
* the parity inventory's primitives table — two asides between `P10` and `P11`;
* `fighter-brain.md`'s dead-vs-restrained table — a ten-line aside between `the
  tell` and `what it means`;
* `world-facts-observations-and-memory.md` — an eight-row block whose header was
  never written at all.

⚠ THE LAST ONE IS A DIFFERENT DEFECT WEARING THE SAME SYMPTOM, and this checker
cannot tell them apart: an interrupted table and a headerless one both present as
"a row with no separator above it". ⇒ Diagnose per instance. The cheap tell is the
PIPE COUNT — uniform across the block means a header is simply missing; ragged, or
a non-row line in the middle, means something interrupted it — but it is a hint
and not an answer: one of the four was mis-diagnosed twice from the counts alone
and only reading the surrounding twenty lines settled it.

⭐ WHY THIS ONE CAN BE A GATE WHERE THE CITATION-CONTENT CHECK COULD NOT: it is
purely structural. "Does this line start a table without a separator row" needs no
judgment about what a sentence MEANS, which is what killed the deeper citation
check (a prose line names several things and cites one, so any pairing is a guess).

⚠ FENCES MUST BE SKIPPED. The first run of this sweep reported 29 hits, almost all
ASCII diagrams inside ``` blocks. Skipping fences leaves three. A checker that
over-reports is as much a claim about the instrument as one that under-reports.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
PLANNING = REPO / "docs" / "planning"
SEPARATOR = re.compile(r"^\|[\s:|-]+\|$")


def _broken_tables(text: str) -> list[tuple[int, str]]:
    """Rows that begin a table with no separator line under them."""
    lines = text.split("\n")
    found: list[tuple[int, str]] = []
    fenced = False
    i = 0
    while i < len(lines):
        if lines[i].lstrip().startswith("```"):
            fenced = not fenced
            i += 1
            continue
        stripped = lines[i].strip()
        # Three pipes is the smallest real row; a bare `|` is a diagram edge.
        if not fenced and stripped.startswith("|") and stripped.endswith("|") and stripped.count("|") >= 3:
            after = lines[i + 1].strip() if i + 1 < len(lines) else ""
            before = lines[i - 1].strip() if i else ""
            if not before.startswith("|") and not SEPARATOR.match(after):
                found.append((i + 1, stripped[:70]))
            while i < len(lines) and lines[i].strip().startswith("|"):
                i += 1
            continue
        i += 1
    return found


def test_every_planning_table_has_a_header() -> None:
    offenders: list[str] = []
    for doc in sorted(PLANNING.rglob("*.md")):
        for line_no, text in _broken_tables(doc.read_text(encoding="utf-8", errors="ignore")):
            rel = doc.relative_to(REPO)
            offenders.append(f"{rel}:{line_no}  {text}")
    assert not offenders, (
        "these rows start a table with no `|---|` separator, which usually means a "
        "prose line was written INSIDE a table and ended it — every row after that "
        "point renders as literal pipes:\n  " + "\n  ".join(offenders)
    )


def test_the_checker_can_see_a_break() -> None:
    """The positive control. Without it the test above passes against a checker
    that never flags anything, which is the failure mode it exists to prevent."""
    good = "| a | b |\n|---|---|\n| 1 | 2 |\n"
    assert _broken_tables(good) == []
    interrupted = "| a | b |\n|---|---|\n| 1 | 2 |\nprose\n| 3 | 4 |\n"
    assert len(_broken_tables(interrupted)) == 1


def test_fenced_diagrams_are_not_tables() -> None:
    """The correction the first sweep needed: ASCII art inside a fence is not a
    table, and counting it produced 29 hits where there were three."""
    fenced = "```text\n|  |\n| x |\n```\n"
    assert _broken_tables(fenced) == []
