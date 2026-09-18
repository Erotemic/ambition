"""Every live maintainer question has one unique, canonical identity."""

from __future__ import annotations

import collections
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
DECISIONS = REPO / "docs" / "planning" / "awaiting-maintainer-decision.md"
QUESTION = re.compile(r"^## Q(\d+)\b(.*)$", re.MULTILINE)
ANY_Q_HEADING = re.compile(r"^#{1,6}\s+.*\bQ(\d+)\b.*$", re.MULTILINE)


def _questions() -> list[tuple[str, str]]:
    text = DECISIONS.read_text(encoding="utf-8")
    return [(number, rest.strip()) for number, rest in QUESTION.findall(text)]


def _non_canonical_q_headings(text: str) -> list[str]:
    """Every heading naming a Q number that is not a `## Q<n>` declaration.

    ⚠ **REPORTING THE OFFENDING HEADING IS THE WHOLE POINT.** The first version
    compared two lists of NUMBERS, so a real failure printed ninety numbers
    twice and left the reader to diff them by eye — and the number a heading
    repeats is never the number that identifies it. The line and the text are.
    """
    canonical = {m.group(0) for m in QUESTION.finditer(text)}
    return [
        f"{text[: m.start()].count(chr(10)) + 1}: {m.group(0)}"
        for m in ANY_Q_HEADING.finditer(text)
        if m.group(0) not in canonical
    ]


def test_live_questions_use_one_canonical_heading_shape() -> None:
    text = DECISIONS.read_text(encoding="utf-8")
    assert QUESTION.search(text), "no live maintainer questions were parsed"
    offenders = _non_canonical_q_headings(text)
    assert not offenders, (
        "these headings name a maintainer question number without being a "
        "`## Q<number> — ...` declaration, and there are two ways to arrive "
        "here:\n  "
        + "\n  ".join(offenders)
        + "\n\nEither this IS a question and it is written in a second "
        "dialect — mixed dialects made duplicate-number checks miss real "
        "collisions in the past, so give it the canonical shape; or it is a "
        "dated note inside a question's body, in which case DROP THE NUMBER "
        "FROM THE HEADING. It already sits under the question, and every other "
        "dated subsection in this file omits it."
    )


def test_no_two_questions_share_a_number() -> None:
    seen = collections.Counter(number for number, _ in _questions())
    clashes = {number: count for number, count in seen.items() if count > 1}
    assert not clashes, (
        "these live maintainer question numbers are ambiguous:\n  "
        + "\n  ".join(f"Q{number} x{count}" for number, count in sorted(clashes.items()))
    )


def test_the_matcher_is_not_vacuous() -> None:
    assert len(_questions()) >= 7


if __name__ == "__main__":
    test_live_questions_use_one_canonical_heading_shape()
    test_no_two_questions_share_a_number()
    test_the_matcher_is_not_vacuous()
    print(f"ok: {len(_questions())} live questions, all canonical and unique")
