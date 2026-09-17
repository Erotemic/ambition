"""The name checker has to FIRE, and has to be right about what it reads.

⛔ A CHECKER WITH NO TEST IS A CHECK THAT MIGHT NOT FAIL. This one's whole value
is that it says "no such name" -- if its scan were wrong it would say that about
everything, or about nothing, and both read like a working tool.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_planning_test_citations import (  # noqa: E402
    DEFINED,
    cited_names,
    resolves,
)


def test_a_fabricated_name_does_not_resolve():
    # The half the tool exists for. Long enough to satisfy the pattern, and
    # deliberately absurd so no future arm can accidentally claim it.
    assert not resolves("a_test_that_nobody_has_ever_written_anywhere_at_all")


def test_the_definition_scan_found_a_corpus():
    # ⛔ THE CONTROL, AND IT IS A FLOOR RATHER THAN A NAME. A scan that matched
    # NOTHING passes the fabricated-name test forever while reporting every
    # citation in the repository as broken -- the loudest possible failure, but
    # one a reader could mistake for a real finding. Measured 2026-09-17: 18,054
    # names. The floor sits an order of magnitude below that and far above zero.
    assert len(DEFINED) > 1000


def test_this_very_test_resolves_by_the_python_road():
    # ⭐ SELF-REFERENTIAL ON PURPOSE. A control pinned to some other arm's name
    # dies the day that arm is renamed, and then the control is the thing that
    # needs repairing. This one cannot go stale while it exists: the file making
    # the assertion is the file defining the name.
    assert resolves("test_this_very_test_resolves_by_the_python_road")


def test_a_name_that_is_only_a_file_resolves():
    # ⛔⛤ THE THIRD ROAD, AND IT WAS A FALSE POSITIVE BEFORE IT EXISTED. Several
    # planning rows cite an integration test by its FILE -- the unit a reader
    # opens -- and the file's `#[test]` inside carries a different, longer name.
    # `fn NAME` alone called those citations imaginary.
    name = "a_move_keeps_its_occurrence_across_a_rewind"
    assert resolves(name)
    # ⛔ AND IT RESOLVES BY THAT ROAD ALONE. Without this, the arm goes green the
    # day somebody adds an `fn` of the same name and stops testing the road.
    assert name not in DEFINED


def test_it_reads_backticked_names_out_of_prose(tmp_path: Path):
    doc = tmp_path / "row.md"
    doc.write_text(
        "witnessed by `a_candidate_session_replaced_while_pending_is_discarded`,\n"
        "and by `the_shipped_apps_own_first_room_publishes`.\n",
        encoding="utf-8",
    )
    assert cited_names(doc)[0] == {
        "a_candidate_session_replaced_while_pending_is_discarded",
        "the_shipped_apps_own_first_room_publishes",
    }


def test_a_name_with_no_article_prefix_is_still_a_citation(tmp_path: Path):
    # ⛔⛤ THE REGRESSION THIS PINS COST THE CHECKER 61% OF ITS CORPUS. The first
    # version matched `(a|an|the)_…` because that is how this repo USUALLY names
    # an arm, so every `exactly_one_…`, `no_…`, `every_…`, `test_…` citation was
    # invisible -- and two dangling ones sat in the same paragraph as one it did
    # report. Re-anchoring on a prefix is the easy edit that would undo it.
    doc = tmp_path / "row.md"
    doc.write_text(
        "held by `no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves`\n"
        "and `every_presence_only_probe_is_named_with_its_reason`.\n",
        encoding="utf-8",
    )
    assert cited_names(doc)[0] == {
        "no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves",
        "every_presence_only_probe_is_named_with_its_reason",
    }


def test_a_short_backticked_name_is_not_a_citation(tmp_path: Path):
    # ⛔ THE MEASURED CUTOFF. `a_possible_morning` is a MUSIC SCORE, listed among
    # other score names, and no rename will ever make it resolve -- a checker with
    # a permanent false positive can never be wired into a gate. Across
    # `docs/planning`, the citations that resolve are six or more
    # underscore-separated words; this one is three.
    doc = tmp_path / "row.md"
    doc.write_text(
        "`scores/active` — `a_possible_morning`, `aether_severance`,\n",
        encoding="utf-8",
    )
    assert cited_names(doc)[0] == set()


def test_it_ignores_ordinary_backticked_prose(tmp_path: Path):
    # ⚠ THE PATTERN'S COST, AND IT IS LENGTH ALONE NOW. Anything under six
    # underscore-separated words is invisible -- deliberately, because the
    # alternative is reporting every backticked identifier in the planning tree.
    # These must NOT be reported.
    doc = tmp_path / "row.md"
    doc.write_text(
        "reads `the_id` and `a_flag`, installs `publish_candidate_session`, and\n"
        "the field `left_to_custodian` counts them.\n",
        encoding="utf-8",
    )
    assert cited_names(doc)[0] == set()
