"""The anchor guard must run in a lane, and must be able to FAIL."""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_planning_anchors_resolve as guard  # noqa: E402


def test_every_planning_pointer_resolves_today():
    assert guard.main() == 0


def test_the_slug_matches_github_for_a_heading_with_an_emoji_and_a_dash():
    """⛔ THE EXACT SHAPE THAT BROKE. `— ✅ DONE` collapses to `--done`: the em

    dash and the emoji vanish, the spaces around them become hyphens. A guard
    that models this wrongly reports a false RED on every decorated heading, and
    a false red is obeyed faster than a false green is questioned.
    """
    assert (
        guard.slug("### A10 — candidate world / last-good-world publication — ✅ DONE")
        == "a10--candidate-world--last-good-world-publication---done"
    )


def test_a_plain_heading_slugs_the_obvious_way():
    assert guard.slug("## Priority table") == "priority-table"


def test_anchors_are_read_from_headings_only():
    """⚠ A line mentioning `#` mid-sentence is not a heading."""
    tmp = pathlib.Path(guard.PLANNING / "README.md")
    found = guard.anchors(tmp)
    assert found, "the planning README declares no headings; the reader is broken"
    assert all(" " not in a for a in found)
