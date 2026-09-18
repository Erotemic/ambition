"""The facade's closure, and the two halves of the claim that drifted apart.

⛔⛤ The number went stale on four pages and the RENDER half went FALSE on
three, which are different failures: a stale number is a number, and a
mandatory path that no longer exists describes the architecture backwards.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_facade_dependency_closure as guard  # noqa: E402


def test_the_closure_is_the_recorded_number():
    reached = len(guard.closure()) - 1
    assert reached == guard.CLOSURE, (
        f"the facade's mandatory closure is {reached}, recorded as {guard.CLOSURE}. "
        "A fall is progress; either way move the constant and every page in one commit."
    )


def test_render_is_outside_the_mandatory_graph():
    """⛔ THE ARCHITECTURAL PROPERTY, and the one a single manifest edit undoes."""
    paths = guard.closure()
    assert guard.MUST_STAY_OUT not in paths, (
        f"`{guard.MUST_STAY_OUT}` is mandatory again, by "
        f"{' -> '.join(paths.get(guard.MUST_STAY_OUT, []))}"
    )


def test_the_facade_itself_is_reachable_which_is_the_non_vacuity():
    """⚠ WITHOUT THIS, A TRAVERSAL THAT FOUND NOTHING PASSES BOTH ARMS ABOVE.

    A closure of zero would fail the first arm today, but only because 48 is
    non-zero — invert the constant to 0 and the render arm passes serenely on an
    empty graph. This says the graph was walked.
    """
    paths = guard.closure()
    assert guard.FACADE in paths
    assert len(paths) > 10, f"the traversal reached only {len(paths)} package(s)"


def test_every_page_restating_the_number_agrees():
    reached = len(guard.closure()) - 1
    disagreeing = [
        f"{page}:{line} says {value}"
        for page, rows in guard.restatements().items()
        for line, value in rows
        if value != reached
    ]
    assert not disagreeing, f"measured {reached}; these disagree: {disagreeing}"


def test_the_restatement_scan_is_not_vacuous():
    """⛔ A PATTERN THAT MATCHES NOTHING MAKES THE ARM ABOVE MEANINGLESS.

    The agreement arm is a claim about a population, and the population is
    whatever this regex finds in `docs/planning`. If a page rephrases the noun
    the scan goes quiet and the six owners come back without a failure.
    """
    pages = guard.restatements()
    assert len(pages) >= 3, f"only {len(pages)} page(s) matched the restatement pattern"


# ── the history exemption, which is where a stale number can still hide ─────


def test_a_historical_value_needs_the_marker(tmp_path, monkeypatch):
    """⛔ THE EXEMPTION IS NARROW: the `HISTORICAL` table alone is not enough.

    Without the marker requirement, any page could state 51 forever and this
    check would call it history.
    """
    page = tmp_path / "docs" / "planning" / "p.md"
    page.parent.mkdir(parents=True)
    page.write_text("the facade reaches 51 other workspace packages today\n")
    monkeypatch.setattr(guard, "REPO", tmp_path)
    assert guard.restatements() == {"docs/planning/p.md": [(1, 51)]}
    assert guard.history() == []


def test_the_marker_moves_it_from_live_to_history(tmp_path, monkeypatch):
    page = tmp_path / "docs" / "planning" / "p.md"
    page.parent.mkdir(parents=True)
    page.write_text(
        "at the review baseline it reached 51 other workspace packages "
        "<!-- cite-ok: the baseline record -->\n"
    )
    monkeypatch.setattr(guard, "REPO", tmp_path)
    assert guard.restatements() == {}
    assert guard.history() == [("docs/planning/p.md", 1, 51)]


def test_an_UNDECLARED_value_is_never_history_however_marked(tmp_path, monkeypatch):
    """⭐ THE CONTROL THAT KEEPS THE MARKER FROM BEING A BLANKET WAIVER.

    `cite-ok` silences a value the `HISTORICAL` table vouches for, with its
    reference point. A number this closure never had is a mistake whatever the
    author wrote beside it.
    """
    page = tmp_path / "docs" / "planning" / "p.md"
    page.parent.mkdir(parents=True)
    page.write_text("it reached 44 other workspace packages <!-- cite-ok: honest -->\n")
    monkeypatch.setattr(guard, "REPO", tmp_path)
    assert guard.restatements() == {"docs/planning/p.md": [(1, 44)]}
    assert guard.history() == []


def test_every_historical_value_states_its_reference_point():
    for value, why in guard.HISTORICAL.items():
        assert len(why) > 40, f"{value}'s provenance is too short to be a reference point"
