#!/usr/bin/env python3
"""Every open P0/P1 row's `Blocked by:` question appears in the blocking set.

⛔⛤ **THE DEFECT THIS EXISTS FOR, 2026-09-19: THE SAME TABLE WAS WRONG TWICE,
AND THE SECOND CAUSE WAS READING PROSE WHERE A FIELD EXISTS.**
`awaiting-maintainer-decision.md` opens with *What actually blocks architecture
work today*, and it is the page a maintainer reads to decide what to rule. It
was derived twice by scanning each queue row's prose for gate language. The
first pass used a 60-line window and missed two blockers stated further down.
The second used full row extents and produced six — while five open P0/P1 rows
carried an explicit `**Blocked by:**` line naming EIGHT questions, none of
which the table mentioned.

⇒ **Scanning prose for a fact a corpus states in a field is not conservative,
it is a different question.** It misses gates recorded only in the field, and
it invents ones: `ID-PEER` names `Q142` as history, three words from "blocked",
and a proximity match called it a gate.

⚠ **THE RULE IS ONE-DIRECTIONAL ON PURPOSE.** Every `Blocked by:` question must
appear in the section; the section may name MORE. Four live gates (`Q136`,
`Q122`, `Q144`, `Q146`) are stated in row prose and in no `Blocked by:` line at
all, so the true population is the union of both roads and an equality check
would red on the half this guard cannot see.

⚠ **AND A ROW THAT CLOSES IS NOT THIS GUARD'S BUSINESS.** Closed rows are
skipped by their heading, because a `Blocked by:` line under a `✅ DONE`
heading is a receipt rather than a gate.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
QUEUE = REPO / "docs/planning/queue.md"
RULINGS = REPO / "docs/planning/awaiting-maintainer-decision.md"

SECTION = "## What actually blocks architecture work today"
QUESTION = re.compile(r"\bQ(\d{2,3})\b")
#: A row in the blocking set's tables: `| [`Q136`](#...) | ... |`. Matching the
#: ROW rather than the question name is what makes this a check on the claim.
ROW = re.compile(r"^\|\s*\[`Q(\d{2,3})`\]", re.M)
BLOCKED_BY = "**Blocked by:**"


def blocked_by_paragraph(lines: list[str], index: int) -> str:
    """The `Blocked by:` field, to the blank line that ends it.

    ⛔⛤ **THIS WAS A FIXED THREE-LINE WINDOW UNTIL 2026-09-19, WHICH IS THE
    EXACT DEFECT THIS GUARD EXISTS TO PUNISH.** The blocking set was wrong
    twice because a derivation scoped a row by a LINE COUNT instead of by the
    structure it lives in; this guard then scoped the field the same way, and
    the first row to carry a qualifying sentence after its field had that
    sentence's question numbers read as gates. A paragraph ends at a blank
    line. That boundary is free and it is the one the document actually has.
    """
    out = []
    for line in lines[index:]:
        if not line.strip():
            break
        out.append(line)
    return " ".join(out)

#: ⛔ ANTI-VACUITY. If the section heading is renamed or the queue's convention
#: changes, this guard would compare an empty set against an empty set and pass
#: while the page it protects says nothing. MEASURED 2026-09-19: 5 gated rows.
MIN_GATED_ROWS = 3


def gated_rows() -> dict[str, set[str]]:
    """Open P0/P1 rows → the questions their `Blocked by:` field names."""
    lines = QUEUE.read_text(encoding="utf-8").split("\n")
    out: dict[str, set[str]] = {}
    priority: str | None = None
    row: str | None = None
    closed = True
    for index, line in enumerate(lines):
        if line.startswith("## "):
            priority = line[3:].split()[0]
        elif line.startswith("### "):
            row = line[4:].split(" —")[0].strip()
            # A heading states its own closure; this is the queue's convention.
            closed = any(mark in line for mark in ("✅", "CLOSED", "DONE"))
        elif line.startswith(BLOCKED_BY):
            if closed or priority not in ("P0", "P1") or row is None:
                continue
            found = set(QUESTION.findall(blocked_by_paragraph(lines, index)))
            if found:
                out.setdefault(f"{priority} {row}", set()).update(found)
    return out


def questions_named_in_the_section() -> set[str]:
    """Questions the section gives a TABLE ROW to, not merely a mention.

    ⛔⛤ **THE FIRST VERSION ACCEPTED A MENTION ANYWHERE IN THE SECTION, AND THE
    POISON THAT SHOULD HAVE CAUGHT IT PASSED.** Deleting `Q127` from its table
    row left the guard green, because the section also names `Q127` in a
    paragraph recording that an earlier draft had MISFILED it. ⇒ A question
    named only by a sentence explaining why it is not a blocker would satisfy
    a mention check, which is the opposite of what this guard is for. The row
    is the claim; prose around it is commentary.
    """
    text = RULINGS.read_text(encoding="utf-8")
    start = text.index(SECTION)
    rest = text[start + len(SECTION) :]
    end = rest.index("\n## ")
    return set(ROW.findall(rest[:end]))



#: A costed option in a ruling: `* **(a) …` or `1. **…`. Both spellings are in
#: use and neither is worth normalising for a checker's convenience.
OPTION = re.compile(r"^(?:\* \*\*\([a-z]\)|\d\. \*\*)", re.M)
#: ⛔ TWO, because a "decision" with one option is a statement.
MIN_OPTIONS = 2


def questions_without_options(named: set[str]) -> list[str]:
    """A blocker a maintainer cannot answer as written is a blocker twice.

    ⛔⛤ **MEASURED 2026-09-19: EIGHT OF THE SIXTEEN WERE FOUR-LINE STUBS** — a
    question plus one sentence, no measurement and no choices. Every one of
    them gated a P0 or P1 row. The maintainer's instruction was to return the
    set *"as actual decision questions with concrete options and
    consequences"*, and half the set could not be answered as written.
    """
    text = RULINGS.read_text(encoding="utf-8")
    out = []
    for question in sorted(named, key=int):
        marker = f"\n## Q{question} — "
        if marker not in text:
            out.append(f"`Q{question}` has a row in the blocking set and no section on this page")
            continue
        start = text.index(marker)
        rest = text[start + len(marker) :]
        end = rest.index("\n## ") if "\n## " in rest else len(rest)
        body = rest[:end]
        found = len(OPTION.findall(body))
        if found < MIN_OPTIONS:
            out.append(
                f"`Q{question}` blocks a P0/P1 row and states {found} option(s). A "
                "maintainer cannot answer it as written, so it blocks twice — give it "
                "the choices and what each costs."
            )
    return out


def main() -> int:
    try:
        named = questions_named_in_the_section()
    except ValueError:
        print(
            f"⛔⛔ {RULINGS.relative_to(REPO)} has no section titled "
            f"`{SECTION}`. A guard that cannot find its subject must refuse, "
            "not report clean."
        )
        return 1

    gated = gated_rows()
    if len(gated) < MIN_GATED_ROWS:
        print(
            f"⛔⛔ found {len(gated)} gated P0/P1 row(s), below the floor of "
            f"{MIN_GATED_ROWS}. That is a claim about this scan, not about the queue."
        )
        return 1

    missing = []
    for row, questions in sorted(gated.items()):
        for question in sorted(questions - named, key=int):
            missing.append(
                f"`{row}` is blocked by `Q{question}` and the blocking set gives it "
                "no table row. A maintainer reading that section to decide what to "
                "rule would leave this row blocked without knowing it. (A mention in "
                "the surrounding prose does not count — see `questions_named_in_the_"
                "section`.)"
            )

    if missing:
        print("the blocking set does not name every gate the queue states:")
        for line in missing:
            print(f"  ⛔ {line}")
        return 1

    thin = questions_without_options(named)
    if thin:
        print("the blocking set names questions that cannot be answered as written:")
        for line in thin:
            print(f"  ⛔ {line}")
        return 1

    total = sum(len(v) for v in gated.values())
    print(
        f"ok: {len(gated)} open P0/P1 row(s) state {total} `Blocked by:` gate(s), "
        f"and the blocking set gives each one a table row ({len(named)} row(s) "
        "there, every one carrying costed options)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
