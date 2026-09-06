"""The generated frame-history doc must match the ledger it claims to report.

⛔⛔ **IT DID NOT EXIST AT ALL** until 2026-09-01, while `perf_history.py`'s own
usage line had named it since the tool was written:

    scripts/perf_history.py report -o docs/planning/engine/runtime-frame-history.md

A generated document that is never generated is worse than a missing one: the
command in the header tells a reader it is authoritative, and there was nothing
to read.

⭐ AND ONCE GENERATED IT ROTS. Every ingested bundle changes it, and the file
carries `⛔ Do not hand-edit`. Without this test the first new capture makes the
committed copy a confident description of a ledger that has moved — the same
failure `test_modules_md_is_current` exists to prevent for crate maps.

⚠ THE COMPARISON IS OF CONTENT, NOT OF A TIMESTAMP. The report embeds an absolute
path to the ledger, which differs per checkout, so the check normalises that one
line rather than demanding byte equality it could never get on another machine.
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
REPORT = REPO / "docs/planning/engine/runtime-frame-history.md"


def normalise(text: str) -> list[str]:
    """Drop the generated-from path, which is absolute and per-checkout."""
    return [line for line in text.splitlines() if "runtime_frame_cost.jsonl" not in line]


def test_the_committed_report_matches_a_fresh_generation():
    assert REPORT.exists(), (
        f"{REPORT.relative_to(REPO)} is missing. Generate it with "
        f"`python3 scripts/perf_history.py report -o {REPORT.relative_to(REPO)}`"
    )
    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "fresh.md"
        result = subprocess.run(
            [sys.executable, "scripts/perf_history.py", "report", "-o", str(fresh)],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        assert result.returncode == 0, (
            f"the report generator failed:\n{result.stdout}{result.stderr}"
        )
        want = normalise(fresh.read_text())
        got = normalise(REPORT.read_text())

    assert got == want, (
        "the committed runtime-frame-history is stale. A bundle was ingested "
        "without regenerating it, and the file says `⛔ Do not hand-edit`. Run:\n"
        "  python3 scripts/perf_history.py report -o "
        "docs/planning/engine/runtime-frame-history.md"
    )


def test_the_report_actually_describes_records():
    """Premise guard: an empty report matches an empty generation.

    Without this, a generator that silently produced a header and no rows would
    satisfy the comparison above forever.
    """
    text = REPORT.read_text()
    assert "record(s)" in text, "the report lost its record count"
    assert text.count("|") > 40, (
        "the report has almost no table rows; it is not describing a ledger"
    )
