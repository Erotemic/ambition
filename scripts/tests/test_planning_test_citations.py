"""The name checker has to FIRE, and has to be right about what it reads.

⛔ A CHECKER WITH NO TEST IS A CHECK THAT MIGHT NOT FAIL. This one's whole value
is that it says "no such `fn`" -- if its grep were wrong it would say that about
everything, or about nothing, and both read like a working tool.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_planning_test_citations import cited_names, exists  # noqa: E402


def test_a_fabricated_name_does_not_resolve():
    # The half the tool exists for. Long enough to satisfy the pattern, and
    # deliberately absurd so no future arm can accidentally claim it.
    assert not exists("a_test_that_nobody_has_ever_written_anywhere_at_all")


def test_a_real_arm_resolves():
    # The control. Without it, a grep that never matches would pass the test
    # above forever while reporting every citation in the repository as broken.
    assert exists("a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction")


def test_it_reads_backticked_names_out_of_prose(tmp_path: Path):
    doc = tmp_path / "row.md"
    doc.write_text(
        "witnessed by `a_candidate_session_replaced_while_pending_is_discarded`,\n"
        "and by `the_shipped_apps_own_first_room_publishes`.\n",
        encoding="utf-8",
    )
    assert cited_names(doc) == {
        "a_candidate_session_replaced_while_pending_is_discarded",
        "the_shipped_apps_own_first_room_publishes",
    }


def test_it_ignores_ordinary_backticked_prose(tmp_path: Path):
    # ⚠ THE PATTERN'S COST. It keys on a prefix and a length, so short names and
    # anything not starting a/an/the are invisible to it -- deliberately, because
    # the alternative is reporting every backticked identifier in the planning
    # tree. These must NOT be reported.
    doc = tmp_path / "row.md"
    doc.write_text(
        "reads `the_id` and `a_flag`, installs `publish_candidate_session`, and\n"
        "the field `left_to_custodian` counts them.\n",
        encoding="utf-8",
    )
    assert cited_names(doc) == set()
