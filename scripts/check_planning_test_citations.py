#!/usr/bin/env python3
"""Every test NAME a planning document cites must exist.

⛔⛤ **THE CITATION GATE CHECKS PATHS, NOT TEST NAMES.** `check_planning_citations.py`
verifies that a cited file exists and that a symbol resolves; a backticked
`a_something_something` inside prose is neither, so a renamed or imagined arm can
sit in a document indefinitely. MEASURED 2026-09-15: the A10 owner document cited
`a_hidden_candidate_session_root_is_invisible_to_the_live_lookup_and_visible_to_its_transaction`
as the unit-level witness for the whole candidate-visibility design. No such
function has ever existed -- the real one is
`a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`
-- and the only occurrence of the cited name in the repository was the citation.

⇒ A reader following that name finds nothing and cannot tell "renamed" from
"never written". This asks whether the name is DEFINED anywhere, which is cheap
and exact.

⚠ WHAT IT DOES NOT DO: it says something with that name exists, not that it says
what the prose claims. See
`reference_a_prose_location_claim_is_not_citation_checked`.

⛔⛤ **AND THE FIRST VERSION OF THIS CHECKER WAS KEYED ON THE AFFIXES ITS AUTHOR
HAD SEEN, WHICH IS THE ERROR IT EXISTS TO CATCH, ONE LEVEL UP.** It matched only
`` `(a|an|the)_…` `` because that is how this repo USUALLY names an arm. Measured
2026-09-17, dropping the prefix takes the population from **181 names to 296**
and the broken list from 1 to 8 -- so it was reading 61% of its own corpus, and
two of the names it missed sat in the same paragraph as one it reported. The
resolver grew the same way: `fn NAME` alone called twelve of this repo's own
Python `def test_…` arms imaginary, called an integration test cited by the FILE
a reader opens imaginary, and called `you_have_to_cut_the_rope` -- a shipped
`.ldtk` level -- imaginary. ⇒ The question is what a READER finds, so the roads
are `fn`, `def`, and a tracked file with that stem.

⚠ `_NAME_BODY` is shared by the citation regex and the definition scan ON
PURPOSE. If they drift, a name becomes citable and undefinable at the same time
and every citation of it reads as broken.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs" / "planning"
# ⛔ NOT anchored on a prefix; the module docstring says what that cost. The
# spellings it used to miss: `exactly_one_…`, `no_…`, `every_…`, `test_…`,
# `possession_…`, `f9_…`.
_NAME_BODY = r"[a-z][a-z0-9_]{12,}"
CITED = re.compile(rf"`({_NAME_BODY})`")

# ⛔⛤ **A MEASURED THRESHOLD, NOT A GUESSED ONE.** The prefix and length alone
# also match backticked CONTENT names — `a_possible_morning` is a music score, in
# a list of scores, and no rename will ever make it resolve. A checker with a
# permanent false positive can never be wired into a gate, so the cutoff was
# derived from the corpus rather than chosen: across `docs/planning`, **136
# citations resolve to a `fn` and the shortest is SIX underscore-separated
# words**; the false positive is three. Every genuinely broken name found is six
# or more, so this separates them exactly.
#
# ⚠ It is a heuristic and it has a cost: a real test named in four or five words
# is invisible to this checker. That is the trade for being gateable, and the
# number to revisit if the convention changes.
MIN_WORDS = 6


# A line may name something that DELIBERATELY does not exist -- a deleted API, a
# superseded const, a pre-cut path kept as the record -- and for those a
# resolvable citation would mean the deletion did not happen. Such a line carries
# `<!-- cite-ok: <reason> -->`.
#
# ⛔⛤ **THE PREDICATE IS IMPORTED, NOT RESPELLED, AND THAT IS THE WHOLE POINT.**
# `check_planning_citations.py` already owns `marker_suppresses` and documents
# the scope as THE CITATION'S OWN LINE OR THE ONE BELOW IT, because a citation
# often ends a wrapped sentence and the marker will not fit beside it at 80
# columns. That module records having been respelled three times in itself and a
# FOURTH time, differently and more narrowly, in a test -- and that the narrow
# copy reddened a guard on a marker that was legal by the documented rule.
# MEASURED 2026-09-16: my first version here was a FIFTH spelling, same-line
# only, which is the same trap one more time. ⇒ Import the keeper.
#
# ⛔⛔ AND IT IS AN AMNESTY, SO IT IS SCOPED AND PRINTED. Five planning documents
# were writing `cite-ok` before this checker read them at all -- the convention
# looked honoured and did nothing, so the gate went red on a name whose exemption
# had been written months earlier on the same line. The marker must carry a
# NON-EMPTY reason, and every name it excuses is REPORTED on a green run. An
# amnesty nobody can see is a way to hide what it exempts.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from check_planning_citations import MARKER, marker_suppresses  # noqa: E402

EXEMPT_REASON = re.compile(re.escape(MARKER) + r":\s*(?P<reason>[^>]*?)\s*-->")


def cited_names(path: Path) -> tuple[set[str], set[str]]:
    """Return (names to check, names excused by a `cite-ok` marker)."""
    lines = path.read_text(encoding="utf-8").splitlines()
    checkable: set[str] = set()
    excused: set[str] = set()
    for lineno, line in enumerate(lines, start=1):
        names = {
            name for name in CITED.findall(line) if len(name.split("_")) >= MIN_WORDS
        }
        if not names:
            continue
        # ⚠ `marker_suppresses` owns the SCOPE (this line or the next). The
        # reason is this checker's own extra requirement, so it is searched
        # across exactly the same two lines the keeper looks at.
        window = "\n".join(lines[lineno - 1 : lineno + 1])
        reason = EXEMPT_REASON.search(window)
        # ⛔ `.search()` TRUTHY IS NOT A REASON. `<!-- cite-ok: -->` matches this
        # pattern with an EMPTY capture, so testing the match object excuses the
        # name on a marker that gives no reason at all. MEASURED: the poison for
        # this went green, and only that poison found it -- the first version of
        # this function tested the GROUP and the regression came in when I
        # collapsed onto the shared predicate.
        if marker_suppresses(lines, lineno) and reason and reason.group("reason").strip():
            excused |= names
        else:
            checkable |= names
    return checkable, excused


def _tracked_stems() -> dict[str, str]:
    """Every tracked file's basename without its extension -> the first path."""
    listing = subprocess.run(
        ["git", "ls-files"], cwd=REPO, capture_output=True, text=True
    ).stdout.split()
    stems: dict[str, str] = {}
    for line in listing:
        stems.setdefault(PurePosixPath(line).stem, line)
    return stems


STEMS = _tracked_stems()


def _defined_names() -> set[str]:
    """Every `fn NAME` / `def NAME` in the tree, from ONE `git grep`.

    ⚠ The per-name form was one `git grep` per citation -- 592 subprocesses for
    296 names, and about three minutes. That is affordable by hand and not
    affordable in a gate, which is the state this checker is trying to leave.
    """
    out = subprocess.run(
        ["git", "grep", "-h", "-E", rf"\b(fn|def) {_NAME_BODY}"],
        cwd=REPO,
        capture_output=True,
        text=True,
    ).stdout
    return set(re.findall(rf"\b(?:fn|def) ({_NAME_BODY})", out))


DEFINED = _defined_names()


def resolves(name: str) -> bool:
    """Does a reader following this name find anything?

    ⛔⛤ **THREE ROADS, BECAUSE THE QUESTION IS WHAT A READER FINDS, NOT WHAT
    LANGUAGE WROTE IT.** Asking only for `fn NAME` reported a Python checker's
    own `def test_…` arms as imaginary (12 of them), reported an integration
    test cited by its FILE — the unit a reader opens — as imaginary, and
    reported `you_have_to_cut_the_rope`, a shipped `.ldtk` level, as imaginary.
    None of those three is a broken citation; each is a name that resolves by a
    road this checker was not looking down.
    """
    return name in DEFINED or name in STEMS


def main() -> int:
    broken: list[tuple[str, str]] = []
    excused: list[tuple[str, str]] = []
    checked = 0
    for path in sorted(DOCS.rglob("*.md")):
        names, skipped = cited_names(path)
        for name in sorted(skipped):
            excused.append((str(path.relative_to(REPO)), name))
        for name in sorted(names):
            checked += 1
            if not resolves(name):
                broken.append((str(path.relative_to(REPO)), name))
    if broken:
        print("planning docs cite test names that do not exist:")
        for path, name in broken:
            print(f"  {path}: `{name}`")
        print(
            "A reader following one of these finds nothing and cannot tell a "
            "rename from a name that was never written."
        )
        return 1
    print(
        f"ok: all {checked} names cited in docs/planning resolve to a `fn`, "
        "a `def` or a tracked file"
    )
    # ⛔ Printed on a GREEN run, not only when something fails: an exemption
    # nobody reads is how the list grows.
    for path, name in excused:
        print(f"  cite-ok (not checked): {path}: `{name}`")
    return 0


if __name__ == "__main__":
    sys.exit(main())
