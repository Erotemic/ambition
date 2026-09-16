"""The control-plane survival guard must be able to FAIL, not merely be run.

⛔⛤ **IT WAS IN `--maintenance` FROM THE DAY IT WAS WRITTEN AND HAD NO TEST BESIDE
IT.** That is one step better than the failure it records — a guard that runs in
no lane — but only one: a checker whose LOGIC nothing exercises is poisoned by
hand once, by its author, and never again. Its population grew from six documents
to eleven on 2026-09-16, which is exactly the kind of edit that can silently
neuter a floor.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_planning_docs_survive as guard  # noqa: E402


def test_the_live_control_plane_is_intact_today():
    assert guard.main() == 0


def test_every_guarded_document_exists_and_clears_its_floor():
    """⛔ ANTI-VACUITY: a floor above the real size would red forever; a floor of

    zero would pass over an emptied file. This asserts BOTH directions on the
    real corpus, which the exit code alone does not distinguish.
    """
    assert len(guard.LIVE_CONTROL_PLANE) >= 10, guard.LIVE_CONTROL_PLANE.keys()
    for rel, (floor, headings) in guard.LIVE_CONTROL_PLANE.items():
        path = guard.REPO / rel
        assert path.exists(), rel
        text = path.read_text(encoding="utf-8")
        lines = text.count("\n")
        assert lines > floor, f"{rel}: {lines} lines is at or under its floor {floor}"
        # ⚠ The floor must sit UNDER today's size with room, or compressing a
        # closed row to a receipt — which is the contract WORKING — reds it.
        assert floor < lines, rel
        for heading in headings:
            assert heading in text, f"{rel} lost {heading!r}"


def test_a_heading_rename_is_what_this_check_calls_a_lost_document():
    """⛔ THE SECOND RULE, which the line floor cannot see: a file can keep every

    line and stop being the document it claims to be.
    """
    rel, (_, headings) = next(
        (r, v) for r, v in guard.LIVE_CONTROL_PLANE.items() if v[1] and v[1][0] != "# "
    )
    assert headings[0] not in "", rel
