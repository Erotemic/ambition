"""The two blindness pages must agree with their own tables, and with each other.

`docs/recipes/checks-that-did-not-run.md` numbers the checks that did NOT RUN;
`dev/journals/blind-checks-2026-09-03.md` numbers the ones that ran and could not
have failed. The recipe quotes the journal's total, and its own heading states
its row count in words.

⛔ BOTH DRIFTED ON 2026-09-03, the day they were written, and in the page whose
subject is counts that stop being true:

  * the journal's two tables each numbered 19-27, so "#22" had two answers;
  * a twelfth recipe entry was added as a second "11" while the heading still
    said "The eleven";
  * the recipe quoted "thirty-seven instances" of a journal that had grown to 46.

⇒ Each was found by a person counting rows by hand. This counts them.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
RECIPE = REPO / "docs/recipes/checks-that-did-not-run.md"
JOURNAL = REPO / "dev/journals/blind-checks-2026-09-03.md"

WORDS = {
    "one": 1, "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7,
    "eight": 8, "nine": 9, "ten": 10, "eleven": 11, "twelve": 12,
    "thirteen": 13, "fourteen": 14, "fifteen": 15, "sixteen": 16,
    "seventeen": 17, "eighteen": 18, "nineteen": 19, "twenty": 20,
}


def word_to_int(text: str) -> int | None:
    text = text.lower().replace("‑", "-")
    if text in WORDS:
        return WORDS[text]
    if "-" in text:
        tens, _, units = text.partition("-")
        base = {"twenty": 20, "thirty": 30, "forty": 40, "fifty": 50, "sixty": 60}
        if tens in base and units in WORDS:
            return base[tens] + WORDS[units]
    return None


def numbers(path: Path) -> list[int]:
    return [int(m) for m in re.findall(r"^\| (\d+) \|", path.read_text(), re.M)]


@pytest.mark.parametrize("page", [RECIPE, JOURNAL], ids=lambda p: p.name)
def test_the_row_numbers_are_unique_and_contiguous(page: Path):
    """⛔ Two rows numbered 19-27 in one file meant a citation had two answers."""
    got = numbers(page)
    assert got, f"no numbered rows parsed from {page.name}"
    dupes = sorted({n for n in got if got.count(n) > 1})
    assert not dupes, f"{page.name} has rows sharing a number: {dupes}"
    assert sorted(got) == list(range(1, len(got) + 1)), (
        f"{page.name} numbers are not 1..{len(got)}: {sorted(got)}"
    )


def test_the_recipe_heading_states_its_own_row_count():
    heading = re.search(r"## The ([a-z-]+), and what each one teaches", RECIPE.read_text())
    assert heading, "the section heading no longer states a count in words"
    stated = word_to_int(heading.group(1))
    assert stated == len(numbers(RECIPE)), (
        f"heading says {heading.group(1)} ({stated}); the table has "
        f"{len(numbers(RECIPE))} rows"
    )


def test_the_recipe_quotes_the_journals_real_total():
    quoted = re.search(r"([a-z-]+) instances, each with the commit", RECIPE.read_text())
    assert quoted, "the recipe no longer quotes the journal's total"
    stated = word_to_int(quoted.group(1))
    assert stated == len(numbers(JOURNAL)), (
        f"the recipe says {quoted.group(1)} ({stated}); the journal has "
        f"{len(numbers(JOURNAL))} rows"
    )


def test_the_two_pages_still_point_at_each_other():
    """They count DIFFERENT families and say so; the link is what keeps a reader
    from adding the totals."""
    assert "blind-checks-2026-09-03.md" in RECIPE.read_text()
    assert "checks-that-did-not-run.md" in JOURNAL.read_text()


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
