"""Two ways a planning page renders as something other than what was written.

⭐⭐ ONE GUARD, TWO RULES, AND THE CLASS IS WHAT MAKES IT ONE. Both defects are
purely STRUCTURAL — "does this `|` row have a separator above it", "are these two
adjacent headings really one" — and neither needs a judgment about what a sentence
MEANS. That is exactly the property that made the table rule gateable where the
deeper citation-content check could not be: a prose line names several things and
cites one, so any pairing is a guess. A second member arriving with the same
property is an argument for one file with two rules rather than two rival scripts.

── RULE 1 ────────────────────────────────────────────────────────────────────
A markdown table interrupted by prose stops being a table.

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
── RULE 2 ────────────────────────────────────────────────────────────────────
A heading wrapped onto a second `#` line is TWO headings.

⛔ FOUND BY A PEER READING THEIR OWN PAGE, 2026-09-06: one sentence an author
wrapped and prefixed twice renders as a heading plus a fragment with no subject.
Seven across the tree at the census; the sharpest was a split parenthetical whose
second half was a bare `CLOSED PROPERLY THE SAME DAY)` — a dangling `)` in a
heading, which is what the wrap does to a reader.

⚠ THE TWO HALVES OF THE TEST DESERVE DIFFERENT CONFIDENCE AND ARE KEPT APART. A
heading legitimately FOLLOWED BY a lowercase one is close to impossible; a heading
legitimately ENDING on "the" is merely unlikely, and the word list is the kind of
hand-kept thing that goes stale. `_wrapped_headings` returns which rule fired so a
future reader can retire the weak half without touching the strong one.

⛔⛔ AND ONE FILE IS EXCLUDED BY OWNERSHIP, WHICH IS NOT AN AMNESTY. Four real hits
live in `JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`, all inside the maintainer's own
preamble, and that file's first line says agents may edit it only to mark something
done. ⇒ Nobody here may fix them, so a gate over them would be permanently red for
a reason no agent can act on. The boundary is NOT a filename list: it is read from
the DECLARATION the document itself makes, so a file that stops declaring it stops
being excluded, and a new file that declares it is covered without an edit here.
And the hits are still COUNTED against a ceiling, because an agent adding a fifth
is exactly the thing the exclusion must not permit.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
PLANNING = REPO / "docs" / "planning"
SEPARATOR = re.compile(r"^\|[\s:|-]+\|$")
HEADING = re.compile(r"^(#{1,6})\s+(.*)$")

# The declaration a maintainer-owned page makes about itself, in its own first
# line. Read rather than listed — see the module doc.
MAINTAINER_DECLARATION = "Agents should only edit this file"

# Words a heading does not end on. The WEAK half of rule 2; see the module doc.
CONTINUATION = {
    "a", "an", "and", "are", "as", "be", "but", "by", "for", "from", "in", "is",
    "it", "its", "not", "of", "on", "or", "that", "the", "this", "to", "was",
    "which", "with",
}

# The four in the maintainer's preamble. A COUNT, never a list of line numbers:
# a list absorbs the guard's own weaknesses the moment somebody appends to it.
MAINTAINER_CEILING = 4


def _maintainer_owned(text: str) -> bool:
    """Does this page declare itself the maintainer's, in its own first line?"""
    first = text.split("\n", 1)[0]
    return MAINTAINER_DECLARATION in first


def _wrapped_headings(text: str) -> list[tuple[int, str, str]]:
    """Headings whose next line is a same-level heading that continues them.

    Returns `(line_no, rule, text)`, where `rule` is `"lowercase"` (the strong
    half) or `"continuation"` (the weak half) — kept apart deliberately.
    """
    lines = text.split("\n")
    found: list[tuple[int, str, str]] = []
    fenced = False
    for i in range(len(lines) - 1):
        if lines[i].lstrip().startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        first, second = HEADING.match(lines[i]), HEADING.match(lines[i + 1])
        if not (first and second and first.group(1) == second.group(1)):
            continue
        follower = second.group(2).strip().split()
        starts_lowercase = bool(follower) and follower[0][:1].islower() and follower[0][:1].isalpha()
        opener = first.group(2).strip().rstrip("*_`").split()
        ends_open = bool(opener) and opener[-1].lower().strip("*_`.,:;") in CONTINUATION
        if starts_lowercase:
            found.append((i + 1, "lowercase", lines[i][:70]))
        elif ends_open:
            found.append((i + 1, "continuation", lines[i][:70]))
    return found


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


def test_no_heading_was_wrapped_onto_a_second_line() -> None:
    """RULE 2 over the pages agents own."""
    offenders: list[str] = []
    for doc in sorted(PLANNING.rglob("*.md")):
        text = doc.read_text(encoding="utf-8", errors="ignore")
        if _maintainer_owned(text):
            continue
        for line_no, rule, opener in _wrapped_headings(text):
            offenders.append(f"{doc.relative_to(REPO)}:{line_no}  [{rule}]  {opener}")
    assert not offenders, (
        "each of these headings is immediately followed by a SAME-LEVEL heading "
        "that continues it — one sentence an author wrapped and prefixed twice, "
        "which renders as a heading plus a fragment with no subject:\n  "
        + "\n  ".join(offenders)
    )


def test_the_maintainer_owned_page_is_counted_and_not_forgiven() -> None:
    """⛔ THE EXCLUSION ABOVE IS AN OWNERSHIP BOUNDARY, NOT AN AMNESTY, and this
    is what keeps the difference visible. The four hits are inside Jon's own
    preamble in a file whose first line says agents may edit it only to mark
    something done — so no agent may fix them, and a gate over them would be
    permanently red for a reason nobody here can act on.

    ⇒ But an agent APPENDING a fifth is exactly what the exclusion must not
    permit, so they are still counted. A ceiling rather than a list of line
    numbers, for the same reason the plain-specials census uses one: a list
    absorbs the guard's own weaknesses the moment somebody appends to it."""
    owned = [
        doc
        for doc in sorted(PLANNING.rglob("*.md"))
        if _maintainer_owned(doc.read_text(encoding="utf-8", errors="ignore"))
    ]
    assert len(owned) == 1, (
        "the ownership boundary is read from a page's own first line, and the "
        f"number of pages declaring it changed (found {len(owned)}: {owned}). "
        "If the declaration was reworded, this guard silently widened or "
        "narrowed — which is the failure a filename list would have hidden."
    )
    hits = _wrapped_headings(owned[0].read_text(encoding="utf-8", errors="ignore"))
    assert len(hits) <= MAINTAINER_CEILING, (
        f"{owned[0].relative_to(REPO)} has {len(hits)} wrapped headings "
        f"(ceiling {MAINTAINER_CEILING}). The known four are the maintainer's "
        f"own preamble; a new one is an agent's:\n  {hits}"
    )


def test_the_wrapper_checker_can_see_both_rules() -> None:
    """The positive control, per rule. Without it both assertions above pass
    against a detector that never flags anything."""
    assert _wrapped_headings("## one heading\n\ntext\n") == []
    # A real heading followed by a real heading is not a wrap.
    assert _wrapped_headings("## First\n## Second\n") == []
    lower = _wrapped_headings("## The gate is complete, and the finding\n## is about the fact\n")
    assert [rule for _, rule, _ in lower] == ["lowercase"]
    cont = _wrapped_headings("# A5. Done (REOPENED AND\n# CLOSED THE SAME DAY)\n")
    assert [rule for _, rule, _ in cont] == ["continuation"]
    # Different levels are two headings, not a wrap.
    assert _wrapped_headings("## Opener and\n### closer\n") == []


def test_a_wrapped_heading_inside_a_fence_is_not_one() -> None:
    """The same correction rule 1 needed: a fenced block is not markdown."""
    assert _wrapped_headings("```md\n## Opener and\n## closer\n```\n") == []
