"""`scripts/check_stale_marks.py` validates the SHAPE of a ⌛ STALE mark.

Convention: `docs/planning/STALENESS.md`. Jon, 2026-08-08 — mark staleness with
evidence where it is found, sweep later.

⛔ **the checker must never claim to judge truth.** It cannot know whether a doc
is stale; it can only tell whether the mark is reviewable. These tests pin that
boundary as much as the parsing.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "check_stale_marks", REPO / "scripts" / "check_stale_marks.py")
sm = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(sm)

GOOD = ("⌛ **STALE 2026-08-08** — the rename in c74246de9 left every crate name "
        "here wrong; `grep -c ambition_platformer_ crates/` returns 0.")


def _marks(tmp_path: Path, text: str):
    (tmp_path / "doc.md").write_text(text, encoding="utf8")
    return sm.collect(tmp_path)


def test_a_well_formed_mark_has_no_problems(tmp_path):
    marks = _marks(tmp_path, f"# T\n\n{GOOD}\n")
    assert len(marks) == 1 and marks[0].problems() == []
    assert marks[0].date == "2026-08-08"


def test_a_mark_with_no_date_is_a_problem(tmp_path):
    marks = _marks(tmp_path, "# T\n\n⌛ **STALE** — " + "x" * 60 + "\n")
    assert any("no ISO date" in p for p in marks[0].problems())


def test_a_mark_with_no_evidence_is_a_problem(tmp_path):
    """⛔ the whole point of the convention: a flag without evidence makes the
    eventual sweep either blind trust or repeated work."""
    marks = _marks(tmp_path, "# T\n\n⌛ **STALE 2026-08-08** — old.\n")
    assert any("no evidence" in p for p in marks[0].problems())


def test_a_mark_inside_a_code_fence_is_an_EXAMPLE_not_a_claim(tmp_path):
    """⛔ found by running the first version: it reported the convention doc's own
    examples, which is a checker nobody runs twice."""
    text = f"# T\n\n```markdown\n{GOOD}\n```\n"
    assert _marks(tmp_path, text) == []


def test_the_convention_document_itself_is_skipped(tmp_path):
    (tmp_path / sm.SPEC_DOC).write_text(f"# Spec\n\n{GOOD}\n", encoding="utf8")
    assert sm.collect(tmp_path) == []


def test_evidence_may_continue_on_the_following_lines(tmp_path):
    """A document-level mark is a blockquote across several lines; the evidence
    is usually on line two, so a one-line reader would call every one of them
    unevidenced."""
    text = ("# T\n\n> ⌛ **STALE 2026-08-08 — read with suspicion.**\n"
            "> The crate names predate the rename; `grep -c ambition_platformer_ "
            "crates/` returns 0.\n")
    marks = _marks(tmp_path, text)
    assert len(marks) == 1 and marks[0].problems() == []


def test_the_checker_does_not_pretend_to_judge_staleness(tmp_path):
    """A mark whose claim is FALSE is still well-formed. That is deliberate —
    see the module docstring."""
    text = ("# T\n\n⌛ **STALE 2026-08-08** — this document is perfectly current "
            "and this mark is wrong, but it is shaped correctly.\n")
    assert _marks(tmp_path, text)[0].problems() == []


def test_the_convention_document_exists():
    assert (REPO / "docs" / "planning" / "STALENESS.md").is_file()
