"""The blocking-set guard must fire on a gate with no row, and not on a mention.

⛔ The load-bearing arm is the last one: the first version of this guard
accepted a question named ANYWHERE in the section, and its poison passed
because the section also names `Q127` in a paragraph recording an earlier
misfiling. A question named only by a sentence saying it is not a blocker
would have satisfied that check.
"""

from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_blocking_set_names_every_gate as guard  # noqa: E402


def test_the_shipped_pages_agree():
    # ⭐ THE RATCHET: it runs against the real queue and the real ruling page.
    assert guard.main() == 0


def test_every_gated_row_is_open_and_p0_or_p1():
    rows = guard.gated_rows()
    assert rows
    assert all(name.startswith(("P0 ", "P1 ")) for name in rows)


def test_a_row_row_is_recognised_and_a_prose_mention_is_not():
    """⛔⛤ THE DISTINCTION THE FIRST VERSION MISSED."""
    row = "| [`Q127`](#q127--are-difficulty) | **P0** `SETTINGS-ROLLBACK` | ... |"
    prose = "⚠ This sentence also named `Q127` until 2026-09-19 and that was wrong."
    assert guard.ROW.findall(row) == ["127"]
    assert guard.ROW.findall(prose) == []
    # ...while the looser rule the guard used to have cannot tell them apart.
    assert guard.QUESTION.findall(prose) == ["127"]


def test_the_shipped_section_covers_the_shipped_gates():
    named = guard.questions_named_in_the_section()
    gated = set()
    for questions in guard.gated_rows().values():
        gated |= questions
    assert gated <= named


def test_the_floor_is_below_the_shipped_count():
    """⛔ ANTI-VACUITY: a scan that lost the queue's convention must red."""
    assert 0 < guard.MIN_GATED_ROWS <= len(guard.gated_rows())


def test_the_field_ends_at_the_blank_line_not_after_n_lines():
    """⛔⛤ THE GUARD'S OWN VERSION OF THE DEFECT IT PUNISHES.

    It read a fixed three lines from `**Blocked by:**`. That is a LINE COUNT
    where the document has a structure, which is exactly why the blocking set
    was wrong twice. Two consequences, both observed on the real pages: a
    four-entry field was truncated to three (`A9` lost `Q97`), and a
    qualifying sentence under a short field was swallowed as part of it
    (`ID-PEER` gained `Q128`/`Q137`, both explicitly not gates).
    """
    lines = [
        "**Blocked by:** [Q100](x),",
        "[Q106](x),",
        "[Q108](x),",
        "and the admission policy in [Q97](x).",
        "",
        "⚠ Not a gate: Q999 is history.",
    ]
    para = guard.blocked_by_paragraph(lines, 0)
    found = set(guard.QUESTION.findall(para))
    assert found == {"100", "106", "108", "97"}
    assert "999" not in found


def test_a_field_at_end_of_file_terminates():
    assert guard.blocked_by_paragraph(["**Blocked by:** [Q1](x)"], 0).strip()
