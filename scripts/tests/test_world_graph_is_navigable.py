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


def _world(levels: list[dict]) -> str:
    import json

    return json.dumps({"levels": levels})


def _level(identifier: str, area: str | None, zones: list[tuple[str, str, str, bool]]) -> dict:
    """A level whose doors each name their ARRIVAL ZONE explicitly.

    ⚠ The arrival zone used to be hardcoded to `"back"` here, and the third arm
    caught it the moment it existed: two fixtures were quietly authoring doors
    into a zone nothing declared. A helper that fills in a field the test is
    about makes every fixture agree with itself and with nothing else.
    """
    fields = []
    if area is not None:
        fields.append({"__identifier": "activeArea", "__type": "String", "__value": area})
    entities = [
        {
            "__identifier": "LoadingZone",
            "fieldInstances": [
                {"__identifier": "id", "__value": zid},
                {"__identifier": "target_room", "__value": target},
                {"__identifier": "target_zone", "__value": target_zone},
                {"__identifier": "bidirectional", "__value": both},
            ],
        }
        # `target=None` is an ARRIVAL-ONLY zone: a real place to land that
        # authors no door of its own.
        for zid, target, target_zone, both in zones
    ]
    return {
        "identifier": identifier,
        "fieldInstances": fields,
        "layerInstances": [{"entityInstances": entities}],
    }


def test_an_area_you_can_enter_and_not_leave_is_reported(tmp_path: Path) -> None:
    """⛔⛔ THE SECOND FAILURE ARM, which the dangling-door poison cannot reach.

    A guard with two arms needs two poisons: the shipped-world poison I ran
    proved the DANGLING arm fires and says nothing about this one. `vault` is a
    real area, so nothing dangles — it simply has no way out.
    """
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "trap.ldtk").write_text(
        _world(
            [
                _level("hub_level", "hub", [("to_vault", "vault", "vault_door", False)]),
                # A real area with no outgoing zone at all.
                _level("vault_level", "vault", []),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 1


def test_a_one_way_door_becomes_two_way_when_authored_bidirectional(
    tmp_path: Path,
) -> None:
    """⚠ THE CONTROL for the arm above: the SAME shape with `bidirectional`
    authored must PASS. Without it, the test above would also pass on a check
    that called every world a trap."""
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "ok.ldtk").write_text(
        _world(
            [
                _level("hub_level", "hub", [("to_vault", "vault", "arrival", True)]),
                # ⚠ An ARRIVAL-ONLY zone: it exists so the door has somewhere to
                # land, and authors no target of its own, so `vault` still has no
                # way out except the `bidirectional` reverse edge. That is the
                # real authoring pattern, and hardcoding a shared zone name hid
                # it until the arrival arm existed.
                _level("vault_level", "vault", [("arrival", None, None, False)]),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 0


def test_a_door_naming_an_arrival_zone_that_does_not_exist_is_reported(
    tmp_path: Path,
) -> None:
    """⛔ THE THIRD ARM: the room resolves and the ZONE does not.

    A door can name a real area and a zone in it that was never authored, and the
    room-level check cannot see that -- it only asks whether the target area
    exists. Nothing in the engine checks the arrival side either.
    """
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "lost.ldtk").write_text(
        _world(
            [
                _level("hub_level", "hub", [("to_vault", "vault", "no_such_zone", True)]),
                # `vault` exists; its only zone is `back_to_hub`, and the door
                # above arrives at `no_such_zone`, which nothing authors.
                _level("vault_level", "vault", [("back_to_hub", "hub", "to_vault", True)]),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 1


def test_the_same_world_passes_when_the_arrival_zone_exists(tmp_path: Path) -> None:
    """⚠ Its control: rename the vault's zone to the one the door names and the
    identical world must PASS, or the arm above would also fire on a healthy
    world."""
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "found.ldtk").write_text(
        _world(
            [
                _level("hub_level", "hub", [("to_vault", "vault", "back", True)]),
                _level("vault_level", "vault", [("back", "hub", "to_vault", True)]),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 0


def test_a_door_naming_a_room_that_is_no_area_is_reported(tmp_path: Path) -> None:
    """⛔⛔ THE FIRST ARM, AND IT HAD NO FIXTURE TEST UNTIL NOW.

    I poisoned this one ONCE, against a real shipped world, by repointing a
    `target_room` at a nonexistent area -- a one-off that proved the arm fires
    and left nothing behind. MEASURED: disabling the `dangling` computation left
    all seven of the other tests GREEN.

    ⇒ Three arms, and poisoning them one at a time is the only way to find the
    one nobody covers. A one-off poison proves the CODE works today; a fixture
    proves it still will.
    """
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "dangling.ldtk").write_text(
        _world(
            [
                _level("hub_level", "hub", [("to_nowhere", "no_such_area", "arrival", True)]),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 1


def _portal(link: str) -> dict:
    return {
        "__identifier": "Portal",
        "fieldInstances": [{"__identifier": "link_id", "__value": link}],
    }


def _level_with(identifier: str, area: str, zones, portals) -> dict:
    level = _level(identifier, area, zones)
    level["layerInstances"][0]["entityInstances"].extend(portals)
    return level


def test_a_portal_pair_spanning_two_areas_is_refused(tmp_path: Path) -> None:
    """⛔⛔ THE CHECK'S OWN PREDICATE, ASSERTED.

    It models a door as a `LoadingZone` and knows nothing about portals. All
    seven authored portal groups stay inside one area today, so that is complete
    — but a cross-area portal would make its verdict wrong BOTH ways at once: an
    area whose only exit is that portal reads as a trap, and a real connection is
    missing from the graph.

    ⇒ Asserted rather than commented, because "portals do not cross areas today"
    is a fact that rots in silence.
    """
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "spanning.ldtk").write_text(
        _world(
            [
                _level_with("hub_level", "hub", [("to_vault", "vault", "arrival", True)], [_portal("gate")]),
                _level_with("vault_level", "vault", [("arrival", None, None, False)], [_portal("gate")]),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 1


def test_a_portal_pair_inside_one_area_is_fine(tmp_path: Path) -> None:
    """⚠ Its control — which is the SHIPPED shape. Without it, the arm above
    would also fire on every world that authors a portal at all, and the four
    worlds do."""
    module = _module()
    module.WORLDS = tmp_path
    (tmp_path / "contained.ldtk").write_text(
        _world(
            [
                _level_with("hub_level", "hub", [("to_vault", "vault", "arrival", True)], [_portal("gate"), _portal("gate")]),
                _level_with("vault_level", "vault", [("arrival", None, None, False)], []),
            ]
        ),
        encoding="utf-8",
    )
    assert module.main() == 0
