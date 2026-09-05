"""The gauntlet-prop guard has to FIRE, and has to tell "off" from "clean".

`check_gauntlet_props_are_rendered.py` is green against the live tree, so a
test that only ran it would prove nothing — the recurring lesson on this repo
is that a check which is green at minute zero guards nothing. These tests do
what the live run cannot:

* run it against the real tree, so a genuine divergence between the game's
  `GAUNTLET_PROP_IDS` and the renderer's `GAUNTLET_ICON_SPECS` reddens a lane
  instead of sitting in a script nobody invokes;
* SKIP rather than pass when the renderer submodule is absent. Exit 3 means
  the declarations went unexamined, and reporting that as success is exactly
  how `check_published_sheets_are_present.py` was off on every machine without
  a tool venv; and
* pin the ASYMMETRY, which is the part a reader would otherwise assume away:
  a declared prop with no drawing fails, a drawing with no declaration does
  not, because the renderer is a separately-pinned submodule where art
  legitimately lands before the game wires it.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
CHECK = REPO / "scripts" / "check_gauntlet_props_are_rendered.py"


def _run() -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CHECK)], capture_output=True, text=True, cwd=REPO
    )


def test_every_declared_gauntlet_prop_has_a_drawing() -> None:
    result = _run()
    if result.returncode == 3:
        pytest.skip("renderer submodule not importable; declarations unexamined")
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_check_fails_on_a_declared_prop_with_no_drawing(tmp_path: Path) -> None:
    """⭐ The poison lives here rather than in a comment claiming it was run."""
    import importlib.util

    spec = importlib.util.spec_from_file_location("gauntlet_check", CHECK)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    if module.drawn_ids() is None:
        pytest.skip("renderer submodule not importable")

    real = module.declared_ids
    try:
        module.declared_ids = lambda: [*real(), "poison_undrawn"]
        assert module.main() == 1
        # And the loose direction stays loose: a drawing the game ignores is a
        # note, not a failure.
        module.declared_ids = lambda: real()[:-1]
        assert module.main() == 0
    finally:
        module.declared_ids = real
