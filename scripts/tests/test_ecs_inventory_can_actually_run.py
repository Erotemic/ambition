"""The navigation-packet generator has to be able to RUN.

⛔⛔ IT DID NOT, AND NOTHING SAID SO. Measured 2026-09-06: the tool venv held
`tree-sitter 0.26.0` against `tree-sitter-rust 0.24.2` and
`ecs_inventory.py --crate <anything>` DUMPED CORE — on small crates too, so it was
not size or memory. `scripts/setup/python_tools.sh` installed `tree_sitter` BARE
while the script's own PEP-723 header declares `tree-sitter>=0.25,<0.26`, so a new
upstream release silently killed it.

⭐ THE CONSEQUENCE IS QUIET, WHICH IS WHY IT WANTS A TEST. `ecs_inventory.py`
regenerates the `.agent/ecs_inventory` packets an agent navigates by. A crash
means the committed data stops being regenerable and simply goes stale; nothing
reads its exit code, and a segfault prints no finding at all.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "ecs_inventory.py"


def test_the_declared_tree_sitter_range_is_what_is_installed() -> None:
    """The script DECLARES a range; the environment must satisfy it.

    ⚠ Asserted against the script's own metadata rather than a number written
    here, so the two cannot drift: the header is the authority.
    """
    header = SCRIPT.read_text(encoding="utf-8")
    assert "tree-sitter>=0.25,<0.26" in header, (
        "the script's declared range moved; update this test's subject deliberately"
    )
    pytest.importorskip("tree_sitter")
    # ⚠ Ask the DISTRIBUTION, not the module: `tree_sitter` exposes no
    # `__version__`, and reaching for one is an AttributeError rather than a
    # version check — a test that would have "failed" on every machine for a
    # reason unrelated to its subject.
    import importlib.metadata as metadata

    version = tuple(int(p) for p in metadata.version("tree-sitter").split(".")[:2])
    assert (0, 25) <= version < (0, 26), (
        f"tree-sitter {metadata.version('tree-sitter')} is outside the range "
        "`ecs_inventory.py` declares. Installing it BARE is what let 0.26 in, and "
        "0.26 against tree-sitter-rust 0.24 SEGFAULTS the tool.\n"
        "  fix: re-run scripts/setup/python_tools.sh (it pins the range now)"
    )


def test_it_produces_an_inventory_rather_than_crashing() -> None:
    """A positive control: the run must SUCCEED, not merely not-error."""
    pytest.importorskip("tree_sitter")
    pytest.importorskip("tree_sitter_rust")
    proc = subprocess.run(
        [sys.executable, str(SCRIPT), "--crate", "crates/ambition_load"],
        cwd=REPO, capture_output=True, text=True,
    )
    assert proc.returncode == 0, (
        f"ecs_inventory exited {proc.returncode}; a NEGATIVE code is a SIGNAL, "
        "and it dumped core here on 2026-09-06.\n"
        + proc.stdout[-400:]
        + proc.stderr[-400:]
    )
    assert "wrote" in proc.stdout, proc.stdout[-400:]
