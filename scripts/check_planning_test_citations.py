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
"never written". This asks `git grep` for a matching `fn`, which is cheap and
exact.

⚠ WHAT IT DOES NOT DO: it says a function with that name exists, not that the
function says what the prose claims. See
`reference_a_prose_location_claim_is_not_citation_checked`.

⛔⛤ **IT IS NOT WIRED INTO A GATE YET, AND EXITS 1 TODAY.** Its first run over
`docs/planning` found the A10 one above plus SEVEN more, in six documents owned by
other campaigns:

    engine/actor-monolith-work-frontier.md
        a_recipe_that_spawns_its_own_entity_escapes_the_candidate_isolation
    engine/agentic-character-runtime.md
        the_visual_follows_a_stored_set
    engine/authored-technique-admission.md
        a_verdict_no_move_claims_still_reaches_the_playback
    engine/checkpoint-restoration-protocol.md
        the_admitted_restore_carries_only_which_operation_and_whose
        the_admitted_restore_does_not_yet_outlive_its_own_frame
    engine/pickup-carve-checklist.md
        the_production_plugin_registers_the_custody_release
    engine/project-build-and-distribution.md
        a_possible_morning

Each is a rename or an arm that was described and never written, and deciding
which is which is the owning campaign's call rather than a mechanical fix -- a
name pointing at nothing may mean the EVIDENCE is missing, not just the label.
⇒ Add this to `run_tests.py --maintenance` once those SEVEN are resolved; until
then run it by hand. (`a_possible_morning` was the eighth and is gone: it is a
music score name, and `MIN_WORDS` below is the measured cutoff that excludes it
without excluding any of the 136 citations that do resolve.)
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs" / "planning"
# Anchored on the prefixes this repository's test names actually use.
CITED = re.compile(r"`((?:a|an|the)_[a-z0-9_]{12,})`")

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


def exists(name: str) -> bool:
    return (
        subprocess.run(
            ["git", "grep", "-l", "-E", rf"fn {name}\b"],
            cwd=REPO,
            capture_output=True,
            text=True,
        ).returncode
        == 0
    )


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
            if not exists(name):
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
    print(f"ok: all {checked} test names cited in docs/planning resolve to a `fn`")
    # ⛔ Printed on a GREEN run, not only when something fails: an exemption
    # nobody reads is how the list grows.
    for path, name in excused:
        print(f"  cite-ok (not checked): {path}: `{name}`")
    return 0


if __name__ == "__main__":
    sys.exit(main())
