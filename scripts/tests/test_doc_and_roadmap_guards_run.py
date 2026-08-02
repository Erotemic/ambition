"""Two guards that existed and were called by nothing.

A sweep on 2026-08-01 cross-referenced every `scripts/*.py` with a non-zero exit
path against `run_tests.py`, `scripts/tests/` and the CI workflow.
`check_doc_links.py` and `check_roadmap_evidence.py` appeared in none of them —
so a broken documentation link or a roadmap claim with no evidence behind it
could land and nothing would say so.

⚠ this is the third and fourth instance of one pattern this run: `modules_md.py`
had a check mode nobody called, and the goal invoked `check_absence_contracts.py`
WITHOUT `--check`, which is the flag that makes it able to fail. **A repository
that writes its own tooling accumulates checks faster than it accumulates
callers**, and a check with no caller is indistinguishable from one that passes.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]


@pytest.mark.parametrize(
    ("script", "args", "what"),
    [
        (
            "check_doc_links.py",
            [],
            "a documentation link points at a file that does not exist",
        ),
        (
            # ⛔ **`--check` IS REQUIRED, and this test ran without it.**
            # `check_roadmap_evidence.py` ends `return 1 if (problems and
            # args.check) else 0`, so without the flag it reports problems on
            # stdout and exits 0 — and this assertion reads the exit code.
            # PROVED 2026-08-02 by injecting a problem: without the flag the
            # script exits 0 and this test PASSED; with it, exit 1.
            #
            # ⚠ the defect is in the file that documents the defect. This
            # module's own docstring records that the goal invoked
            # `check_absence_contracts.py` without `--check`, "which is the flag
            # that makes it able to fail" — and then called the sibling script
            # the same way. Knowing the shape is not the same as checking for it.
            "check_roadmap_evidence.py",
            ["--check"],
            "a roadmap claim has no evidence recorded behind it",
        ),
    ],
)
def test_the_guard_holds_against_the_live_tree(script: str, args: list[str], what: str):
    result = subprocess.run(
        [sys.executable, f"scripts/{script}", *args],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        f"{script} is RED — {what}:\n{result.stdout}{result.stderr}"
    )
