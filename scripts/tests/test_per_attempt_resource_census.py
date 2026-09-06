"""The per-attempt census has to be able to go wrong.

It reports rather than enforces, on purpose -- "not `AttemptScoped`" is not a
finding, because most collection-holding content resources are catalogs and
caches that MUST survive a death. So what is left to protect is the measurement:

* an empty corpus must FAIL, not report a calm zero;
* a known per-attempt resource losing its impl must FAIL by NAME, because that is
  the shipped Sanic bug coming back; and
* the sweep must actually find the collection field, not just the derive.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "per_attempt_resource_census.py"


def _module():
    spec = importlib.util.spec_from_file_location("per_attempt_resource_census", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_it_is_green_against_the_tree(capsys) -> None:
    module = _module()
    assert module.main() == 0
    out = capsys.readouterr().out
    assert "retracted through `AttemptScoped`: 3" in out


def test_an_empty_population_fails_rather_than_reporting_zero(monkeypatch) -> None:
    """⛔⛔ A grep-shaped census over a moved directory reports zero and every
    claim built on it becomes trivially true."""
    module = _module()
    monkeypatch.setattr(module, "collection_resources", lambda: [])
    assert module.main() == 1


def test_a_known_per_attempt_resource_losing_its_impl_fails_by_name(monkeypatch, capsys) -> None:
    """The Sanic bug was one resource that looked like its retracting sibling.
    Dropping an impl must name the resource, not just lower a count."""
    module = _module()
    monkeypatch.setattr(module, "implementors", lambda: {"BrokenBricks", "SpentPowerBlocks"})
    assert module.main() == 1
    assert "SpentMonitors" in capsys.readouterr().err


def test_the_sweep_requires_a_collection_field_not_just_the_derive(tmp_path, monkeypatch) -> None:
    """⚠ ANTI-VACUITY FROM THE OTHER SIDE. A sweep that matched every `Resource`
    would report a huge population and hide the three that matter in it."""
    module = _module()
    crate = tmp_path / "game" / "demo" / "src"
    crate.mkdir(parents=True)
    (crate / "lib.rs").write_text(
        "#[derive(Resource, Default)]\n"
        "pub struct HasNoCollection(pub u32);\n\n"
        "#[derive(Resource, Default)]\n"
        "pub struct HasOne(pub Vec<String>);\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(module, "REPO", tmp_path)
    names = [row[2] for row in module.collection_resources()]
    assert names == ["HasOne"], names


def test_an_untriaged_resource_fails_rather_than_landing_in_the_everything_else_pile(
    monkeypatch, capsys
) -> None:
    """⭐ THE ROW'S REAL OPEN HALF, made structural: 'state that is never cleared
    by ANY room signal is invisible'. It is invisible only while nobody has to
    answer. A collection-holding content resource in neither list now fails."""
    module = _module()
    real = module.collection_resources
    monkeypatch.setattr(
        module,
        "collection_resources",
        lambda: real() + [("game/demo/src/lib.rs", 7, "SpentSomethingNew")],
    )
    assert module.main() == 1
    assert "SpentSomethingNew" in capsys.readouterr().err


def test_a_triage_entry_for_a_resource_that_is_gone_fails(monkeypatch, capsys) -> None:
    """⛔ A hand-kept list rots toward its author's memory of the tree. This
    caught two of my own entries within the hour: `AttemptsSeen` and `Forced`
    were tuple structs that only ever entered the population through the sweep
    bug this file fixed."""
    module = _module()
    monkeypatch.setattr(
        module,
        "NOT_PER_ATTEMPT",
        dict(module.NOT_PER_ATTEMPT, AResourceThatWasDeleted="stale"),
    )
    assert module.main() == 1
    assert "AResourceThatWasDeleted" in capsys.readouterr().err
