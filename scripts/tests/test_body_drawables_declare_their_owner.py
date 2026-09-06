"""The ownership census must find the population and reject an unclassified one.

Green against the tree, so a test that only ran it would prove nothing.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "check_body_drawables_declare_their_owner.py"


def _module():
    spec = importlib.util.spec_from_file_location("body_drawables", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree() -> None:
    proc = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stdout + proc.stderr


def test_a_new_body_naming_component_must_be_classified(monkeypatch) -> None:
    """⭐ THE CASE IT EXISTS FOR: a drawable added without declaring whose body it
    draws is one portal composition cannot see."""
    module = _module()
    monkeypatch.setattr(module, "CENSUS", {})
    assert module.main() == 1


def test_a_stale_census_entry_fails(monkeypatch) -> None:
    """The other direction, so the census cannot rot into names that no longer
    exist and make the guard look tighter than it is."""
    module = _module()
    census = dict(module.CENSUS)
    census["NoSuchVisual"] = "does not exist"
    monkeypatch.setattr(module, "CENSUS", census)
    assert module.main() == 1


def test_pub_crate_structs_are_found(tmp_path) -> None:
    """⛔ THE SCANNER'S OWN BLIND SPOT, caught by this guard on its first run.
    Matching only `pub struct` missed `pub(crate) struct SlashVisual`, and the
    guard then reported it as a STALE census entry — a scanner hole wearing the
    costume of a stale row."""
    module = _module()
    found = module.components_naming_a_body()
    assert "SlashVisual" in found, sorted(found)


def test_a_function_parameter_is_not_a_field() -> None:
    """⚠ `slash_visuals.rs` has `fn spawn_one(.., owner: Entity, ..)`. The first
    cut counted it as a second body-naming component, and I repeated that
    over-count in a message before this guard existed."""
    module = _module()
    found = module.components_naming_a_body()
    assert "spawn_one" not in found
    assert len(found) == 6, sorted(found)
