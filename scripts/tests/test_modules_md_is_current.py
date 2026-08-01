"""Every crate's `MODULES.md` matches the tree it describes.

`scripts/modules_md.py` has had a check mode the whole time and **nothing ever
called it**, so the D-B navigability standard was maintained by whoever
remembered. On 2026-08-01, regenerating found nineteen crates stale and three
with no map at all — a map is the first thing an agent reads about a crate, and a
stale one is a confident description of a shape that has moved.

⚠ this lives in `scripts/tests/` rather than as a new `run_tests.py` job because
that suite already runs FIRST and cheaply in the backbone, for exactly the reason
this check exists: *"a guard nobody executes is not a guard."*
"""

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
