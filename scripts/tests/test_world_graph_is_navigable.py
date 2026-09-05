"""The world-graph check has to FIRE, and SKIP rather than pass when blind.

It is green against the shipped worlds, so a test that only ran it would prove
nothing. These do what the live run cannot:

* run it, so a genuine dangling door or one-way trap reddens a lane instead of
  sitting in an stderr warning nobody reads during a build;
* SKIP on exit 3. The worlds are SYMLINKS into `game/ambition_map_assets`; a
  checkout without that submodule examined no doors, and calling that success is
  how a gate goes quiet on every machine that has not run a setup step. ⛔ It is
  also why this gate cannot be evidence between two machines: two boxes at the
  same commit can hold different worlds; and
* pin the ANTI-VACUITY and the KEY, which are the two ways this script could
  pass while measuring nothing.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
CHECK = REPO / "scripts" / "check_world_graph_is_navigable.py"


def _module():
    spec = importlib.util.spec_from_file_location("world_graph_check", CHECK)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def test_every_authored_door_leads_somewhere_and_no_area_is_a_trap() -> None:
    result = subprocess.run(
        [sys.executable, str(CHECK)], capture_output=True, text=True, cwd=REPO
    )
    if result.returncode == 3:
        pytest.skip("map submodule absent; no doors examined")
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_area_key_is_the_one_the_engine_reads() -> None:
    """⛔ The whole check rests on ONE camelCase string.

    Keying on the level identifier invents areas; keying on `active_area`
    (snake_case) matches nothing and falls back to identifiers everywhere, which
    looks identical to working. Both produced confident false findings before
    this script existed, which is why the premise is verified rather than
    commented.
    """
    module = _module()
    source = (REPO / "crates/ambition_platformer2d_ldtk/src/project.rs").read_text(
        encoding="utf-8"
    )
    assert f'field_string("{module.AREA_FIELD}")' in source


def test_a_world_with_no_doors_is_a_failure_not_a_pass(tmp_path: Path) -> None:
    """⛔ ANTI-VACUITY: an empty corpus must not print a clean bill."""
    module = _module()
    module.WORLDS = tmp_path
    empty = tmp_path / "empty.ldtk"
    empty.write_text('{"levels": []}', encoding="utf-8")
    assert module.main() == 1
