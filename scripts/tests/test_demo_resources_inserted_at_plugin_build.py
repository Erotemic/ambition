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


def test_an_overwriting_insert_of_a_foreign_type_is_flagged_and_a_variable_is_not() -> None:
    """⭐ `insert_resource` OVERWRITES; `init_resource` is a no-op when the
    resource is already there. On a type the demo does NOT define, that
    difference is Jon's `99ab15e32` ("Smash was deleting another plugin's
    resources on the way out") arriving from the other direction.

    ⚠ And the false positive that arm shipped with first: `insert_resource(goal_pole)`
    passes a VARIABLE, not a type, and lowercase is how you tell.
    """
    module = _module()
    out = []
    for _crate, hits in module.inserted_at_build().items():
        out += [label for _f, label in hits]
    flagged = [o for o in out if "OVERWRITING" in o]
    assert any(o.startswith("RespawnInterval") for o in flagged), flagged
    assert not any("goal_pole" in o for o in flagged), flagged


def test_a_broken_signature_matcher_is_caught_by_an_INDEPENDENT_witness() -> None:
    """⛔⛔ THE COUNT FLOOR COULD NOT CATCH THIS FILE'S OWN BUG. `MIN_DEMOS = 3`
    against five demos passes when exactly one drops out — and exactly one
    dropping out is what happened, when `BUILD` matched only `&mut App` and lost
    `ambition_demo_smash`.

    ⛔ AND THE FIRST MEMBERSHIP CHECK COULD NOT EITHER, because it derived its
    expectation with `BUILD` too: poisoning the matcher removed the demo from the
    findings AND from the expectation, and the check stayed green. A membership
    assertion whose subject is its own filter's output proves nothing.
    """
    import re

    module = _module()
    real = module.BUILD
    try:
        module.BUILD = re.compile(r"fn build\(\s*&self,\s*app:\s*&mut\s*App\s*\)\s*\{")
        assert module.main() == 1
    finally:
        module.BUILD = real


def test_the_plugin_witness_matches_both_trait_spellings() -> None:
    """⚠ THE SAME BLINDNESS, HIT TWICE IN ONE FILE. `impl Plugin for` alone misses
    `ambition_demo_smash`, which writes `impl bevy::prelude::Plugin for` — the
    identical qualified-vs-short problem that hid it from `BUILD`, met again while
    writing the witness meant to catch that."""
    module = _module()
    assert module.PLUGIN_IMPL.search("impl Plugin for P {")
    assert module.PLUGIN_IMPL.search("impl bevy::prelude::Plugin for SmashRulesPlugin {")
    assert not module.PLUGIN_IMPL.search("struct NotAPlugin;")


def test_a_demo_that_inserts_nothing_is_reported_as_zero_not_omitted(capsys) -> None:
    """`ambition_demo_pocket` has a `Plugin::build` and inserts NOTHING in it. A
    reader seeing four rows concludes there are four demos; it was invisible until
    the membership check replaced the count floor."""
    module = _module()
    assert module.main() == 0
    assert "ambition_demo_pocket: 0" in capsys.readouterr().out
