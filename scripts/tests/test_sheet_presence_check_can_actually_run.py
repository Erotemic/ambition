"""The published-sheet presence check must be able to RUN, not just to refuse.

`check_published_sheets_are_present.py` asks whether every rostered sprite
target published anything at all. It answers through
`scripts/lib/sprite_install_names.claimed_install_names`, which imports the
renderer package to learn what each target DECLARES it installs — never
guessing `<target>_spritesheet.png`, because four targets disprove that guess.

⛔⛔ THE IMPORT WAS BARE, SO THE CHECK DISABLED ITSELF. It needed the renderer
already on `sys.path` — i.e. the tool venv. On an ordinary checkout that HAS the
submodule but no venv it raised `ModuleNotFoundError`, `claimed_install_names`
returned `None`, and the check printed "cannot check: the sprite renderer is not
importable here" and **exited 0**. A guard that exits clean on every machine
that has not run a setup step is a guard nobody notices is off, and this one is
in `scripts/setup/generated_content.sh`'s neighbourhood where a 0 reads as
"fine".

⭐ THE SIBLING HAD IT RIGHT ALL ALONG. `generate_visual_quality_variants.py`
inserts the submodule root itself and has always reached the registry. Same
repo, same package, two different answers to "is the renderer importable".

⚠ REFUSING IS STILL CORRECT WHEN THE SUBMODULE IS ABSENT — that is a real state
(`git submodule update --init` not run), and "cannot check" beats inventing a
verdict. The fix only stops the refusal happening when the code IS there.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
LIB = REPO / "scripts/lib/sprite_install_names.py"
RENDERER = REPO / "tools/ambition_sprite2d_renderer"


def load():
    spec = importlib.util.spec_from_file_location("sprite_install_names", LIB)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_module_puts_the_renderer_within_reach_when_it_is_checked_out():
    """⭐ THE FIX ITSELF. Without this the import is bare and fails on any
    checkout without the tool venv."""
    if not RENDERER.is_dir():
        pytest.skip("the sprite renderer submodule is not checked out")
    load()
    assert str(RENDERER.resolve()) in sys.path, (
        "the renderer submodule must be reachable, or claimed_install_names "
        "returns None and the presence check exits 0 without checking anything"
    )


def test_it_answers_with_real_install_names_rather_than_none():
    """⛔ POSITIVE CONTROL, AND THE WHOLE POINT. `None` is the 'cannot check'
    signal; a checkout with the submodule must get an answer instead."""
    if not RENDERER.is_dir():
        pytest.skip("the sprite renderer submodule is not checked out")
    module = load()
    claimed = module.claimed_install_names(["alice"])
    assert claimed is not None, (
        "the renderer is checked out, so this must not report 'cannot check'"
    )
    names = claimed.get("alice")
    assert names, "alice is a rostered target and declares install names"
    assert any(n.endswith("_spritesheet.png") for n in names)


def test_an_unknown_target_is_omitted_rather_than_reported_empty():
    """`an unregistered name and a target that installs nothing are different
    facts` — the module's own words, and the distinction the caller relies on
    to avoid reporting a typo as a publishing failure."""
    if not RENDERER.is_dir():
        pytest.skip("the sprite renderer submodule is not checked out")
    module = load()
    claimed = module.claimed_install_names(["a_target_that_does_not_exist"])
    assert claimed is not None
    assert "a_target_that_does_not_exist" not in claimed


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
