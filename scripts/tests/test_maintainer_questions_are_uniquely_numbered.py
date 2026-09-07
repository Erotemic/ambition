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


def test_live_questions_use_one_canonical_heading_shape() -> None:
    text = DECISIONS.read_text(encoding="utf-8")
    canonical = [m.group(1) for m in QUESTION.finditer(text)]
    all_q_numbers = [m.group(1) for m in ANY_Q_HEADING.finditer(text)]
    assert canonical, "no live maintainer questions were parsed"
    assert all_q_numbers == canonical, (
        "maintainer questions must use `## Q<number> — ...`; mixed heading "
        "dialects made duplicate-number checks miss real collisions in the past. "
        f"canonical={canonical}, all={all_q_numbers}"
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
