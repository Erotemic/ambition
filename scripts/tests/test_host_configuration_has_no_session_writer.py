"""The host-configuration guard must run in a lane, and must be able to FAIL.

A guard with no test beside it runs in no lane. Each arm below plants one
writer in a file of its own and asks for a red verdict that names the file.
"""

from __future__ import annotations

import pathlib
import sys

import pytest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import check_host_configuration_has_no_session_writer as guard  # noqa: E402
import multi_writer_resource_census as census  # noqa: E402


def test_no_production_code_writes_a_host_configuration_resource_today():
    assert guard.main() == 0


def _with(monkeypatch, tmp_path: pathlib.Path, name: str, source: str) -> str:
    """The real corpus plus one planted production file."""
    planted = tmp_path / name
    planted.write_text(source, encoding="utf-8")
    real = census.production_files()
    monkeypatch.setattr(census, "production_files", lambda *a, **k: [*real, str(planted)])
    return str(planted)


#: One planted file for each shape that takes `&mut` to a resource.
WRITERS = {
    "a_system_parameter": (
        "FriendlyFire",
        "fn turn_it_on(mut friendly_fire: ResMut<ambition_combat::targeting::FriendlyFire>) {\n"
        "    friendly_fire.enabled = true;\n}\n",
    ),
    "a_world_access": (
        "RegimePolicy",
        "fn enter_a_cutscene(world: &mut World) {\n"
        "    world.resource_mut::<ambition_time::time_control::RegimePolicy>().regime =\n"
        "        Regime::Cinematic;\n}\n",
    ),
    "an_optional_world_access": (
        "FriendlyFire",
        "fn turn_it_on(world: &mut World) {\n"
        "    if let Some(mut rule) = world.get_resource_mut::<FriendlyFire>() {\n"
        "        rule.enabled = true;\n    }\n}\n",
    ),
    "an_init_that_hands_out_the_value": (
        "RegimePolicy",
        "fn enter_a_cutscene(world: &mut World) {\n"
        "    world.get_resource_or_init::<RegimePolicy>().regime = Regime::Cinematic;\n}\n",
    ),
    "a_faction_table_writer": (
        "FactionRelations",
        "fn make_them_foes(mut relations: ResMut<ambition_combat::targeting::FactionRelations>) {\n"
        "    relations.set_mutual_hostile(ActorFaction::Npc, ActorFaction::Enemy, true);\n}\n",
    ),
    "a_resource_scope": (
        "FriendlyFire",
        "fn turn_it_on(world: &mut World) {\n"
        "    world.resource_scope::<FriendlyFire, _>(|_, mut rule| rule.enabled = true);\n}\n",
    ),
}


@pytest.mark.parametrize("shape", sorted(WRITERS))
def test_a_planted_writer_is_red(monkeypatch, tmp_path, capsys, shape):
    ty, source = WRITERS[shape]
    planted = _with(monkeypatch, tmp_path, f"{shape}.rs", source)
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert planted in out, out
    assert f"`{ty}` is host configuration" in out, out


def test_an_unnamed_install_site_is_red(monkeypatch, tmp_path, capsys):
    planted = _with(
        monkeypatch,
        tmp_path,
        "a_new_composer.rs",
        "fn compose(app: &mut App) {\n"
        "    app.insert_resource(ambition_time::time_control::RegimePolicy {\n"
        "        regime: Regime::RLDeterministic,\n    });\n}\n",
    )
    assert guard.main() == 1
    out = capsys.readouterr().out
    assert f"{planted} installs `RegimePolicy`" in out, out


def test_a_writer_in_a_test_module_is_not_counted(monkeypatch, tmp_path):
    """The control for the planted arms: the same line, in test-only code.

    `versus.rs` has such a line in the tree today (its `#[cfg(test)]` module
    sets the rule to prove a declaration plays over it), and the first arm is
    green with it there.
    """
    _with(
        monkeypatch,
        tmp_path,
        "a_fixture.rs",
        "#[cfg(test)]\nmod tests {\n"
        "    fn turn_it_on(mut friendly_fire: ResMut<FriendlyFire>) {\n"
        "        friendly_fire.enabled = true;\n    }\n}\n",
    )
    assert guard.main() == 0
    versus = pathlib.Path("game/ambition_app/src/app/versus.rs").read_text(encoding="utf-8")
    assert "resource_mut::<ambition_platformer2d::combat::targeting::FriendlyFire>()" in versus, (
        "the tree no longer holds the test-module writer this control names"
    )


def test_a_writer_in_a_comment_is_not_counted(monkeypatch, tmp_path):
    _with(
        monkeypatch,
        tmp_path,
        "prose.rs",
        "// This took `ResMut<FriendlyFire>` once, and does not now.\nfn nothing() {}\n",
    )
    assert guard.main() == 0


def test_a_named_install_site_that_left_is_red(monkeypatch, capsys):
    """A row cannot stay after its subject left."""
    stale = dict(guard.CONFIGURATION)
    why, sites = stale["FriendlyFire"]
    stale["FriendlyFire"] = (why, (*sites, "crates/ambition_combat/src/lib.rs"))
    monkeypatch.setattr(guard, "CONFIGURATION", stale)
    assert guard.main() == 1
    assert "installs it no more" in capsys.readouterr().out


def test_a_type_that_is_declared_nowhere_is_red(monkeypatch, capsys):
    """A rename must not leave the guard green over nothing."""
    renamed = dict(guard.CONFIGURATION)
    renamed["FriendlyFireRule"] = renamed.pop("FriendlyFire")
    monkeypatch.setattr(guard, "CONFIGURATION", renamed)
    assert guard.main() == 1
    assert "`FriendlyFireRule` is declared in no production file" in capsys.readouterr().out


def test_a_corpus_that_collapsed_refuses_a_verdict(monkeypatch, capsys):
    monkeypatch.setattr(census, "production_files", lambda *a, **k: ["crates/x.rs"])
    assert guard.main() == 1
    assert "production Rust file" in capsys.readouterr().out
