"""The documented demand-road vocabulary must match the call sites.

`asset-preparation-and-residency.md` lists the road names stamped on every
content-art demand — the vocabulary the `[image]` census reports and the
hall-entry hitch analysis is written in.

⛔ THE LIST HAS DRIFTED FOUR TIMES: it named SEVEN, a review said NINE and missed
`shrine-sheet`, a re-derivation on 2026-09-02 said TEN, and `entity-sprite`
made it ELEVEN within the day — added by `dde5547cb`, itself a commit about a
mis-stamped road. Every correction so far has been a hand-kept copy, and every
hand-kept copy has gone stale.

⇒ A road is added by passing a new string literal at a call site, and nothing
makes the prose follow. This is what makes the prose follow.
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
DOC = REPO / "docs/planning/engine/asset-preparation-and-residency.md"

#: The three functions that take a road name as their stamp.
STAMPERS = ("note_demand", "load_sheet_image", "load_sprite_pages")


def derived_roads() -> set[str]:
    """Every road literal passed to a stamper, read from the call sites."""
    files = subprocess.run(
        ["git", "grep", "-l", "-E", "|".join(STAMPERS), "--", "*.rs"],
        cwd=REPO, capture_output=True, text=True,
    ).stdout.split()
    call = re.compile(r"(?:%s)\s*\(" % "|".join(STAMPERS))
    roads: set[str] = set()
    for rel in files:
        text = (REPO / rel).read_text(errors="replace")
        for match in call.finditer(text):
            window = text[match.end() : match.end() + 400]
            for literal in re.findall(r'"([a-z][a-z0-9-]*)"', window):
                # the road is the first kebab-or-known literal in the argument
                # list; a path literal contains `/` or `.` and never matches.
                if "-" in literal or literal in {"portrait", "parallax"}:
                    roads.add(literal)
                    break
    return roads


def documented_roads() -> set[str]:
    """The fenced block immediately above the 'live roads' sentence."""
    text = DOC.read_text()
    anchor = text.index("live roads")
    block = text.rindex("```", 0, anchor)
    opening = text.rindex("```", 0, block)
    # ⛔ SKIP THE LANGUAGE TAG. The fence is ```text, and splitting from the
    # backticks puts the literal word "text" in the set -- a twelfth road that
    # does not exist. Content starts after the newline that ends the opener.
    body = text[opening + 3 : block]
    return set(body.split("\n", 1)[1].split())


def test_the_derivation_finds_something(  ):
    """⛔ THE PREMISE. A derivation that matched nothing would make the equality
    below pass against an empty documented list, which is the failure this whole
    file is about."""
    assert len(derived_roads()) >= 8, sorted(derived_roads())


def test_the_document_lists_exactly_the_roads_the_code_stamps():
    doc, code = documented_roads(), derived_roads()
    assert doc == code, (
        f"the road list has drifted again.\n"
        f"  in the doc, not in the code : {sorted(doc - code)}\n"
        f"  in the code, not in the doc : {sorted(code - doc)}\n"
        "⇒ A road is added by passing a new string literal at a call site. "
        "Update the fenced block in asset-preparation-and-residency.md and the "
        "count in the sentence under it."
    )


def test_the_sentence_states_the_same_count_as_the_block():
    """The prose says 'ELEVEN live roads'. A block and a count that disagree is
    how a reader ends up quoting the wrong one."""
    words = {
        "SEVEN": 7, "EIGHT": 8, "NINE": 9, "TEN": 10, "ELEVEN": 11,
        "TWELVE": 12, "THIRTEEN": 13, "FOURTEEN": 14,
    }
    text = DOC.read_text()
    stated = re.search(r"\*\*([A-Z]+) live roads", text)
    assert stated, "the sentence no longer states a count in words"
    assert words[stated.group(1)] == len(documented_roads()), (
        f"the sentence says {stated.group(1)} and the block lists "
        f"{len(documented_roads())}"
    )


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
