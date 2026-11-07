"""Test the sentence detector and optionally lint the planning corpus.

Set AMBITION_PLANNING_PROSE_LINT=1 to run the heuristic corpus audit.
The detector's positive and negative controls run in the normal pytest lane.
"""

from __future__ import annotations

import importlib.util
import os
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_severed_sentences", REPO / "scripts" / "checks/check_severed_sentences.py"
)
mod = importlib.util.module_from_spec(SPEC)
sys.modules["check_severed_sentences"] = mod
SPEC.loader.exec_module(mod)


def _planning_docs():
    return sorted((REPO / "docs" / "planning").rglob("*.md"))


@pytest.mark.skipif(
    os.environ.get("AMBITION_PLANNING_PROSE_LINT") != "1",
    reason="optional planning prose audit (AMBITION_PLANNING_PROSE_LINT=1)",
)
def test_no_planning_sentence_is_severed_from_its_continuation():
    docs = _planning_docs()
    assert (REPO / "docs/planning/README.md") in docs
    offenders = []
    for path in docs:
        stranded, _ = mod.scan(path)
        offenders += [(path.relative_to(REPO), *hit) for hit in stranded]
    assert not offenders, "\n".join(
        f"{p}:{line} — '{cur}' continues the sentence above, which already ended"
        for p, line, _prev, cur in offenders
    )


def test_the_detector_can_still_fail(tmp_path):
    """⭐ Poison arm. A guard whose red has never been seen is not a guard.

    The fixture is the real defect's shape: a finished sentence, then a block
    inserted after it, then the orphaned tail of an earlier sentence.
    """
    doc = tmp_path / "severed.md"
    doc.write_text(
        "Some prose that runs on and ends properly here.\n"
        "  ⛔ **AN INSERTED BLOCK.** It ends properly too.\n"
        "  somewhere other than the scores.\n",
        encoding="utf-8",
    )
    stranded, _ = mod.scan(doc)
    assert stranded, "the detector did not see a severed sentence in a poisoned file"

    clean = tmp_path / "clean.md"
    clean.write_text(
        "Some prose that runs on and\n"
        "  ends properly here.\n"
        "  ⛔ **AN INSERTED BLOCK.** It ends properly too.\n",
        encoding="utf-8",
    )
    assert not mod.scan(clean)[0], "a correctly wrapped sentence must not report"
