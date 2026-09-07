"""Cheap structural guards for planning Markdown.

These checks deliberately avoid semantic judgment. They catch two renderer-level
mistakes that have occurred repeatedly in planning prose:

* a table row appearing after prose interrupted the table; and
* one heading sentence being wrapped into two same-level headings.

All planning files are subject to the same rules. There is no maintainer-owned
scratchpad exemption.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
PLANNING = REPO / "docs" / "planning"
SEPARATOR = re.compile(r"^\|[\s:|-]+\|$")
HEADING = re.compile(r"^(#{1,6})\s+(.*)$")
CONTINUATION = {
    "a", "an", "and", "are", "as", "be", "but", "by", "for", "from", "in", "is",
    "it", "its", "not", "of", "on", "or", "that", "the", "this", "to", "was",
    "which", "with",
}


def _wrapped_headings(text: str) -> list[tuple[int, str, str]]:
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
            offenders.append(f"{doc.relative_to(REPO)}:{line_no}  {text}")
    assert not offenders, (
        "these rows begin a Markdown table without a separator/header, usually "
        "because prose interrupted the table:\n  " + "\n  ".join(offenders)
    )


def test_table_checker_positive_controls() -> None:
    good = "| a | b |\n|---|---|\n| 1 | 2 |\n"
    assert _broken_tables(good) == []
    interrupted = "| a | b |\n|---|---|\n| 1 | 2 |\nprose\n| 3 | 4 |\n"
    assert len(_broken_tables(interrupted)) == 1
    assert _broken_tables("```text\n| a | b |\n| c | d |\n```\n") == []


def test_no_heading_was_wrapped_onto_a_second_line() -> None:
    offenders: list[str] = []
    for doc in sorted(PLANNING.rglob("*.md")):
        text = doc.read_text(encoding="utf-8", errors="ignore")
        for line_no, rule, opener in _wrapped_headings(text):
            offenders.append(f"{doc.relative_to(REPO)}:{line_no} [{rule}] {opener}")
    assert not offenders, (
        "same-level adjacent headings appear to be one wrapped heading sentence:\n  "
        + "\n  ".join(offenders)
    )


def test_heading_checker_positive_controls() -> None:
    assert _wrapped_headings("## First\n## Second\n") == []
    lower = _wrapped_headings("## The gate is complete, and the finding\n## is about the fact\n")
    assert [rule for _, rule, _ in lower] == ["lowercase"]
    cont = _wrapped_headings("# A5. Done (REOPENED AND\n# CLOSED THE SAME DAY)\n")
    assert [rule for _, rule, _ in cont] == ["continuation"]
    assert _wrapped_headings("```md\n## Opener and\n## closer\n```\n") == []
