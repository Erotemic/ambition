"""Every crate's generated `MODULES.md` block must match its module tree.

The test runs the same check mode used by `scripts/modules_md.py`, ensuring module
maps and module `//!` summaries cannot drift merely because regeneration was
forgotten."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def test_every_module_map_matches_its_crate():
    result = subprocess.run(
        [sys.executable, "scripts/modules_md.py"],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, (
        "a crate's MODULES.md no longer describes its modules:\n"
        f"{result.stdout}{result.stderr}\n"
        "Regenerate with `python scripts/modules_md.py --write` and commit the "
        "result — the map is the first thing an agent reads about a crate, and a "
        "stale one is a confident description of a shape that has moved."
    )
