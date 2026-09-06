"""Two agents can each measure the next free question number and still collide.

⛔⛔ IT HAPPENED ON 2026-09-06 AND NEITHER OF US WAS CARELESS. A peer needed a
number for a new maintainer question, checked the file, found `Q70` free, and took
it. It *was* free when they looked — my `Q70` had not landed yet. ⇒ **In a shared
numbered file the next number is a measurement, and the measurement expires
between reading it and writing it.** Both of us did the right thing and the file
ended with two `Q70`s pointing at unrelated questions.

⭐ THE FIX IS NOT "check harder". It is to make the collision impossible to push,
which is what this is: a maintainer reading `Q70` in a commit message, a campaign
row or a message from either of us must land on exactly one question.

⚠ AND IT CANNOT KEY ON ONE HEADING SHAPE. Mine was `## ⊙ Q71 — …` and the peer's
`## Q70 — …`; a grep for `^## ⊙ Q` finds one and reports the file clean, which is
how I missed the collision for an hour after being told about it. The marker is
optional, the level varies, and the number is the only thing both forms share.
"""

from __future__ import annotations

import collections
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
DECISIONS = REPO / "docs" / "planning" / "awaiting-maintainer-decision.md"

#  the ⊙/marker is optional and the heading level varies; anchor on the
# number and let everything between `#` and `Q` be anything.
# ⛔⛔ `\S*` ATE THE `Q`. The first version was `^#{1,4}\s+\S*\s*Q(\d+)` — and
# for the UNMARKED form `## Q69 …` the greedy `\S*` consumes `Q69` itself, so
# only the marked rows matched. It parsed ONE question out of a file holding
# dozens and would have reported no collisions forever. ⇒ The optional marker is
# a whole token followed by whitespace, or nothing at all.
QUESTION = re.compile(r"^#{1,4}\s+(?:\S+\s+)?Q(\d+)\b(.*)$", re.MULTILINE)


def _questions() -> list[tuple[str, str]]:
    text = DECISIONS.read_text(encoding="utf-8")
    return [(number, rest.strip()) for number, rest in QUESTION.findall(text)]


def test_no_two_questions_share_a_number() -> None:
    """⛔ THE CONTRACT. A number cited in a commit, a campaign row or a message
    between agents must resolve to exactly one question."""
    seen = collections.Counter(number for number, _ in _questions())
    clashes = {number: count for number, count in seen.items() if count > 1}
    assert not clashes, (
        "these question numbers are used more than once, so a citation of one "
        "resolves to two different questions:\n  "
        + "\n  ".join(
            f"Q{number} x{count}:\n    "
            + "\n    ".join(rest[:80] for n, rest in _questions() if n == number)
            for number, count in sorted(clashes.items())
        )
    )


def test_the_matcher_sees_both_heading_shapes() -> None:
    """⭐⭐ THE POSITIVE CONTROL, and it is the whole reason this file exists in
    this form. The collision survived an hour of looking because my grep was
    `^## ⊙ Q70` — it matched my row, missed the peer's unmarked one, and reported
    the file clean. A matcher that sees one dialect of a shared file is worse than
    none, because it answers confidently."""
    numbers = [number for number, _ in _questions()]
    # ⛔ SEVEN, MEASURED — and my first draft of this floor said 20 because I
    # guessed at "far more". A floor is a claim about the corpus and deserves the
    # same measurement as anything else; an over-high one fails on a healthy tree
    # and teaches the next reader to delete it.
    assert len(numbers) >= 7, (
        f"only {len(numbers)} questions parsed; there were 7 when this was "
        "written, so the matcher has stopped recognising a heading shape rather "
        "than the file having shrunk"
    )
    text = DECISIONS.read_text(encoding="utf-8")
    marked = re.findall(r"^#{1,4}\s+⊙\s*Q\d+", text, re.MULTILINE)
    plain = re.findall(r"^#{1,4}\s+Q\d+", text, re.MULTILINE)
    assert marked and plain, (
        "the file no longer contains BOTH a marked (`## ⊙ Qn`) and an unmarked "
        f"(`## Qn`) heading — marked {len(marked)}, plain {len(plain)}. If one "
        "dialect is gone the guard above is only being tested against the other"
    )
