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
⇒ Add this to `run_tests.py --maintenance` once those eight are resolved; until
then run it by hand. (`a_possible_morning` is the likeliest false positive of the
pattern: prose in backticks long enough to look like a test name.)
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DOCS = REPO / "docs" / "planning"
# Long enough that ordinary prose in backticks does not qualify, and anchored on
# the two prefixes this repository's test names actually use.
CITED = re.compile(r"`((?:a|an|the)_[a-z0-9_]{12,})`")


def cited_names(path: Path) -> set[str]:
    return set(CITED.findall(path.read_text(encoding="utf-8")))


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
    checked = 0
    for path in sorted(DOCS.rglob("*.md")):
        for name in sorted(cited_names(path)):
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
