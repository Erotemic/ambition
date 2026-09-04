"""`docs/planning` prose has no sentence cut in half by a later insertion.

⛔⛔ THREE INSTANCES IN `queue.md` ON ONE DAY, AND NO EXISTING GATE COULD SEE ANY
OF THEM. An edit anchored on a line that ends mid-clause inserts its block
between that line and the one finishing the sentence. The reader gets a clause
with no ending; the continuation sits tens of lines below, after unrelated text,
reading as a new paragraph. Both halves are valid Markdown, so the link gate, the
citation gate and `git diff` all stay green.

⭐ Two of the three were reunited with text still in the file, displaced rather
than deleted -- which is why `check_severed_sentences.py`'s failure message
hands over `git log -S` instead of suggesting an ending be written.

⚠ The population floor matters here for the same reason it does in the zone-name
ratchet: this check's healthy answer is ZERO findings, and a checker that parsed
nothing also reports zero. So the test below asserts the scanner SEES the corpus
before it believes a clean result, and asserts it can still fail.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SPEC = importlib.util.spec_from_file_location(
    "check_severed_sentences", REPO / "scripts" / "check_severed_sentences.py"
)
mod = importlib.util.module_from_spec(SPEC)
sys.modules["check_severed_sentences"] = mod
SPEC.loader.exec_module(mod)


def _planning_docs():
    return sorted((REPO / "docs" / "planning").rglob("*.md"))


def test_the_scanner_actually_reads_the_corpus():
    """A clean result from a scanner that read nothing is not a clean result."""
    docs = _planning_docs()
    assert len(docs) > 50, f"only {len(docs)} planning documents found"
    assert (REPO / "docs" / "planning" / "queue.md") in docs


def test_no_planning_sentence_is_severed_from_its_continuation():
    offenders = []
    for path in _planning_docs():
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
