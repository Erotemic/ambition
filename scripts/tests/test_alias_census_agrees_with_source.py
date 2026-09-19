"""The alias census guard must fire on drift, and must not confuse two methods.

⛔ This guard reports by staying silent, so these arms pin it from outside. The
last arm is the one the defect actually needs: the bare name and the parameter
form are DIFFERENT counts of the same tree, and reading one as the other is
what made `183` and `193` look like a contradiction when both were right.
"""

from __future__ import annotations

import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_alias_census_agrees_with_source as guard  # noqa: E402


def test_the_shipped_documents_agree_with_the_tree():
    # ⭐ THE RATCHET: it runs against the real tree and the real markers.
    assert guard.main() == 0


def test_the_parameter_form_does_not_match_an_import():
    """⛔⛤ THE WHOLE DEFECT IN ONE ASSERT. A `use` line mentions the alias and
    does not instantiate it; ten of them are the entire difference between the
    census's 183 and the number source carried."""
    line = "use ambition_platformer2d::lifecycle::SessionWorldRef;\n"
    assert not guard.SPELLINGS["SessionWorldRef"].search(line)
    assert re.search(r"\bSessionWorldRef\b", line)


def test_the_parameter_form_matches_a_system_parameter():
    body = "fn s(room: SessionWorldRef<CurrentRoom>) {}\n"
    assert len(guard.SPELLINGS["SessionWorldRef"].findall(body)) == 1


def test_the_parameter_form_matches_across_the_generic_break():
    """Shipped signatures wrap, and `Name <` with a space is the same use."""
    assert guard.SPELLINGS["SessionWorldMut"].search("x: SessionWorldMut <T>")


def test_a_helper_is_counted_as_a_call_not_a_mention():
    """⚠ The plan's old table counted mentions for these two rows while its
    header claimed uses, which is how a four-row table held two methods."""
    assert not guard.SPELLINGS["session_root_for_scope"].search(
        "/// see [`session_root_for_scope`]\n"
    )
    assert guard.SPELLINGS["session_root_for_scope"].search(
        "let root = session_root_for_scope(world, scope);"
    )


def test_the_census_marker_is_required_to_exist():
    assert guard.CENSUS_MARKER.search("<!-- alias-census: parameter_form=183 files=95 -->")
    assert not guard.CENSUS_MARKER.search("<!-- alias-census: 183 in 95 files -->")


def test_the_split_marker_parses_every_spelling():
    marker = (
        "<!-- alias-split: SessionWorldRef=167/91 SessionWorldMut=16/13 "
        "live_session_world_root=3/1 session_root_for_scope=2/2 -->"
    )
    hit = guard.PLAN_MARKER.search(marker)
    assert hit
    split = dict(
        (name, (int(c), int(f))) for name, c, f in guard.SPLIT_ENTRY.findall(hit.group(1))
    )
    assert set(split) == set(guard.SPELLINGS)
    assert sum(split[n][0] for n in guard.ALIASES) == 183


def test_the_floor_is_below_the_shipped_count_and_above_zero():
    """⛔ ANTI-VACUITY, pinned: a scan that lost its corpus must red rather than
    compare two zeroes."""
    live = guard.measure()
    total, _ = guard.alias_total(live)
    assert 0 < guard.FLOOR < total
