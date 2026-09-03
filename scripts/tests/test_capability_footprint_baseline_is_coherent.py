"""The capability-footprint baseline must agree with ITSELF.

`scripts/baselines/capability-footprint-baseline.json` is the ratchet a D33
carve moves: `closure_size` and `never_asked_for_count` change when a crate
enters or leaves the movement-only sentinel's closure, and `queue.md`'s
post-carve checklist (item 5) requires the carve to move them in its own commit.

⛔ THAT CHECKLIST ONLY COVERS A CRATE ENTERING. Nothing covered a crate LEAVING,
and the sub-lists drifted: on 2026-09-03 five names were listed as reachable --
`ambition_platformer2d_ldtk`, `ambition_portal2d`, `ambition_inventory_ui`,
`ambition_portal2d_presentation`, `ambition_touch_input` -- while not being in
`ambition_closure` at all. The file's OWN narrative records the first one
leaving ("ldtk_left_the_closure_2026_08_22: 44 -> 43"), so the departure was
known and the lists were simply not pruned with it.

⚠ WHY IT MATTERS RATHER THAN BEING UNTIDY: `reachable_only_through_the_facade`
is the "can this be closed by a manifest change?" list, and a carve decision is
taken off it. Three of its four entries had already left the closure, so the
"170 call sites, not a quick win" analysis was about crates that are no longer
in the footprint being analysed.

These are pure-JSON invariants: no cargo, no tree walk, milliseconds.
"""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
BASELINE = REPO / "scripts/baselines/capability-footprint-baseline.json"

#: The lists whose members must all be crates the sentinel actually links.
SUBSET_KEYS = (
    "never_asked_for",
    "reachable_via_ambition_platformer2d_actor_monolith_alone",
    "reachable_only_through_the_facade",
)


@pytest.fixture(scope="module")
def baseline() -> dict:
    return json.loads(BASELINE.read_text())


def test_the_counts_match_the_lists_they_count(baseline):
    """A count and its list are two statements of one fact, and the count is
    what gets quoted into planning rows -- so it is the one that goes stale
    invisibly."""
    assert baseline["closure_size"] == len(baseline["ambition_closure"])
    assert baseline["never_asked_for_count"] == len(baseline["never_asked_for"])


@pytest.mark.parametrize("key", SUBSET_KEYS)
def test_every_named_crate_is_in_the_closure(baseline, key):
    """⛔ A crate that LEFT the closure cannot still be 'reachable' inside it.
    This is the check that was missing: the checklist covers entering only."""
    stray = sorted(set(baseline[key]) - set(baseline["ambition_closure"]))
    assert not stray, (
        f"`{key}` names {len(stray)} crate(s) that are not in `ambition_closure`: "
        f"{stray}. They left the footprint and the list was not pruned with them."
    )


def test_the_closure_names_real_crates(baseline):
    """⛔ AND THE NAMES MUST BE REAL. A renamed crate leaves a baseline that
    still ratchets -- on a set of names nothing in the workspace answers to,
    which passes every count check above while measuring nothing."""
    declared = set()
    for rel in subprocess.run(
        ["git", "ls-files", "*Cargo.toml"], cwd=REPO, capture_output=True, text=True
    ).stdout.split():
        for line in (REPO / rel).read_text(errors="replace").splitlines():
            if line.startswith("name = "):
                declared.add(line.split('"')[1])
                break
    unknown = sorted(set(baseline["ambition_closure"]) - declared)
    assert not unknown, f"the baseline names crates the workspace does not: {unknown}"


def test_the_composition_doc_quotes_the_baselines_real_count(baseline):
    """⛔ A NUMBER IN PROSE GOES STALE WITH EVERY CARVE, and this one did twice
    in a day: `engine/capability-and-runtime-composition.md` said "All 19 that a
    movement-only game never asked for arrive through the monolith alone" while
    the baseline read 21, then 23.

    ⚠ This test WILL go red on the carve that moves the number, and that is the
    point -- the same trade as the coverage footer's gated-test count. The
    EQUALITY of the two lists is the doc's real claim and is guarded above; this
    guards the figure a reader will quote.
    """
    doc = (REPO / "docs/planning/engine/capability-and-runtime-composition.md").read_text()
    stated = re.search(
        r"\*\*All (\d+) that a movement-only game never asked for", doc
    )
    assert stated, "the composition doc no longer states that count"
    assert int(stated.group(1)) == len(baseline["never_asked_for"]), (
        f"the doc says All {stated.group(1)}; the baseline's never_asked_for has "
        f"{len(baseline['never_asked_for'])}. Re-quote the doc in the carve's "
        "own commit."
    )


def test_the_two_reachability_lists_do_not_overlap(baseline):
    """`why_the_split_matters` rests on them being different questions: one is
    closable by a manifest change and the other is not. A crate in both makes
    that sentence false."""
    both = set(baseline["reachable_via_ambition_platformer2d_actor_monolith_alone"]) & set(
        baseline["reachable_only_through_the_facade"]
    )
    assert not both, f"a crate cannot be in both reachability lists: {sorted(both)}"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
