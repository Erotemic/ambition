"""The plugin-build census has to be able to go wrong.

⛔ Its own matcher was wrong twice while it was written, and the failure direction
is the dangerous one: under-matching reports the CLEANEST possible answer about
the dirtiest crate.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "demo_resources_inserted_at_plugin_build.py"


def _module():
    spec = importlib.util.spec_from_file_location("demo_build_resources", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree(capsys) -> None:
    module = _module()
    assert module.main() == 0
    out = capsys.readouterr().out
    assert "ambition_demo_smash" in out


def test_both_spellings_of_the_build_signature_are_matched() -> None:
    """⛔⛔ THE BUG THAT REPORTED SMASH AS ZERO. This repo writes the plugin hook
    both ways, and `demo_smash` — the demo with the most insertions and every
    known instance of the leak — uses the qualified one."""
    module = _module()
    short = "impl Plugin for P { fn build(&self, app: &mut App) { app.insert_resource(A); } }"
    qualified = (
        "impl Plugin for P { fn build(&self, app: &mut bevy::prelude::App) "
        "{ app.insert_resource(B); } }"
    )
    for src, want in ((short, "A"), (qualified, "B")):
        start = module.BUILD.search(src).end() - 1
        assert [m.group(1) for m in module.INSERT.finditer(module.body_of(src, start))] == [want]


def test_an_empty_sweep_fails_rather_than_reporting_a_clean_tree(monkeypatch) -> None:
    module = _module()
    monkeypatch.setattr(module, "inserted_at_build", dict)
    assert module.main() == 1


def test_the_body_stops_at_its_own_closing_brace() -> None:
    """An insertion AFTER the plugin's build block is not inside it."""
    module = _module()
    src = "fn build(&self, app: &mut App) { app.init_resource::<Inside>(); }\n" \
          "fn other() { app.init_resource::<Outside>(); }"
    start = module.BUILD.search(src).end() - 1
    names = [m.group(1) for m in module.INSERT.finditer(module.body_of(src, start))]
    assert names == ["Inside"], names
