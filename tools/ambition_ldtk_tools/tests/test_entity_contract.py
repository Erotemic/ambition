"""**The authoring loop, held against the runtime's own contract.**

The defect: `mary_o_1_3` was authored through `area create` + `repair` +
`validate` — three affirmative OKs — with six `EnemySpawn` entities carrying no
`character_id`. `convert_enemy_spawn` REFUSES that, so the room would have
panicked the game on load, and the whole Python loop had no way to say so.

⭐ **every test here observes BOTH terms.** A validator only ever shown the broken
world cannot demonstrate it is not simply shouting at everything, so each rule
appears twice: once with the poison and once with its complement, and the second
half must be silent. The `on_invalid: open` cases matter most for that — an
unrecognised `EnemySpawn.brain` is a PROVIDER EXTENSION with real consumers, and a
validator that flagged it would have broken the thing the fallthrough exists for.
"""

from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from ambition_ldtk_tools.ldtk.paths import default_sandbox_ldtk
from ambition_ldtk_tools.validate import validate_issues
from ambition_ldtk_tools.validate_rules.entity_contract import (
    contract_identifiers,
    entity_contract_issues,
    entity_contracts,
)


def _entity(identifier: str, iid: str, **fields):
    return {
        "__identifier": identifier,
        "iid": iid,
        "px": [0, 0],
        "width": 32,
        "height": 32,
        "fieldInstances": [
            {"__identifier": name, "__value": value} for name, value in fields.items()
        ],
    }


def _project(*entities, area: str = "lab", extra_level: dict | None = None):
    levels = [
        {
            "identifier": area,
            "fieldInstances": [{"__identifier": "activeArea", "__value": area}],
            "layerInstances": [
                {"__identifier": "Ambition", "entityInstances": list(entities)}
            ],
        }
    ]
    if extra_level:
        levels.append(extra_level)
    return {"levels": levels}


def _codes(project) -> list[str]:
    return [issue.code for issue in entity_contract_issues(project)]


def _errors(project) -> list[str]:
    return [
        issue.code
        for issue in entity_contract_issues(project)
        if issue.severity == "error"
    ]


# ---------------------------------------------------------------------------
# presence — the mary_o_1_3 defect itself


def test_an_enemy_spawn_without_character_id_is_refused():
    project = _project(_entity("EnemySpawn", "e1", name="Goblin"))
    issues = entity_contract_issues(project)
    assert [i.code for i in issues] == ["contract.required_field_missing"]
    issue = issues[0]
    assert issue.field == "character_id"
    assert issue.severity == "error"
    # The message has to NAME the field and say what to do about it.
    assert "character_id" in issue.message
    assert "character_id" in (issue.fix_hint or "")


def test_an_enemy_spawn_with_character_id_is_silent():
    project = _project(_entity("EnemySpawn", "e1", name="Goblin", character_id="goblin"))
    assert _codes(project) == []


def test_a_blank_character_id_reads_as_absent_exactly_as_the_converter_reads_it():
    # `field_string(...).filter(|v| !v.trim().is_empty())` — the empty string and
    # a whitespace run are both "no value" to the converter.
    for blank in ("", "   ", None):
        project = _project(_entity("EnemySpawn", "e1", character_id=blank))
        assert _errors(project) == ["contract.required_field_missing"], blank


def test_a_tolerated_field_warns_rather_than_erroring():
    # NpcSpawn.character_id is OPTIONAL at runtime while EnemySpawn's is refused.
    # That asymmetry is real, is pinned by the Rust prover, and must not be
    # flattened into "both are errors".
    project = _project(_entity("NpcSpawn", "n1", prompt="Talk"))
    issues = entity_contract_issues(project)
    assert [(i.severity, i.field) for i in issues] == [("warning", "character_id")]


# ---------------------------------------------------------------------------
# the three dispositions of an unrecognised value


def test_a_silently_defaulted_value_is_an_authoring_error():
    """A value the converter quietly substitutes is still a mistake.

    ⛔ THE SUBJECT MOVED, and that is the good news. This used to use
    `LoadingZone.activation` — *"falls through to Door, so `edgeexit` IS a door
    and nothing anywhere says so"* — and the converter was FIXED: it refuses an
    unrecognised spelling now and says so by name
    (`ldtk/src/lib.rs`, *"rather than silently becoming a Door"*), so the
    contract calls that field `refused` and this test had been asserting the old
    world. It was red from that day, invisibly, because nothing runs this suite.

    ⭐ nine fields still declare `silent_default`, so the disposition is alive and
    worth guarding; `KinematicPath.mode` is one, substituting `PingPong`.
    """
    # ⚠ `points` and `speed` are supplied so the ONLY issue is the one under
    # test: a bare `KinematicPath` also trips `required_field_missing`, which
    # would make this assertion about the fixture rather than the disposition.
    project = _project(
        _entity("KinematicPath", "k1", points="0,0;100,0", speed=110, mode="sideways")
    )
    issues = entity_contract_issues(project)
    assert [i.code for i in issues] == ["contract.value_silently_defaulted"]
    assert "PingPong" in issues[0].message


def test_a_legal_activation_is_silent():
    for activation in ("Door", "EdgeExit", "Walk"):
        project = _project(_entity("LoadingZone", "z1", id="exit", activation=activation))
        assert _codes(project) == [], activation


def test_a_typoed_pickup_kind_is_an_error():
    # `currancy:1` parses to PickupKindSpec::Custom, and `collect_pickups` has no
    # arm for Custom — the pickup vanishes on touch and grants nothing.
    project = _project(_entity("PickupSpawn", "p1", kind="currancy:1"))
    assert _errors(project) == ["contract.value_silently_defaulted"]


def test_every_pickup_grammar_the_engine_parses_is_accepted():
    for kind in ("health:1", "currency:25", "ability:test_key", "flag:opened_it"):
        project = _project(_entity("PickupSpawn", "p1", kind=kind))
        assert _codes(project) == [], kind


def test_an_unrecognised_brain_is_an_extension_and_stays_silent():
    # the half a stricter validator would have broken. `mary_o_snake` and
    # `sanic_badnik` are CharacterBrain::Custom keys their providers match on.
    for brain in ("mary_o_snake", "sanic_badnik", "combatant", "Passive", "Guard:96"):
        project = _project(_entity("EnemySpawn", "e1", character_id="x", brain=brain))
        assert _codes(project) == [], brain


def test_a_retired_brain_spelling_is_refused_out_loud():
    project = _project(
        _entity("EnemySpawn", "e1", character_id="x", brain="Patrol:lab_line")
    )
    assert _errors(project) == ["contract.retired_spelling"]


def test_a_refused_enum_value_is_an_error_and_a_legal_one_is_not():
    bad = _project(_entity("EnemySpawn", "e1", character_id="x", respawn="OnRoomReentry"))
    assert _errors(bad) == ["contract.value_refused"]
    for respawn in ("DeadStaysDead", "OnRoomReenter", "OnRest", "InPlace(0.85)"):
        good = _project(_entity("EnemySpawn", "e1", character_id="x", respawn=respawn))
        assert _codes(good) == [], respawn


# ---------------------------------------------------------------------------
# cross-field rules


def test_a_brain_beside_a_path_ref_is_two_answers_to_one_question():
    path = _entity("KinematicPath", "k1", points="0,0;100,0", speed=110)
    spawn = _entity(
        "EnemySpawn",
        "e1",
        character_id="x",
        brain="combatant",
        path_ref={"entityIid": "k1"},
    )
    assert "contract.conflicting_fields" in _errors(_project(path, spawn))


def test_a_path_ref_alone_resolves_and_is_silent():
    path = _entity("KinematicPath", "k1", points="0,0;100,0", speed=110)
    spawn = _entity("EnemySpawn", "e1", character_id="x", path_ref={"entityIid": "k1"})
    assert _codes(_project(path, spawn)) == []


def test_a_path_ref_into_another_active_area_is_refused():
    # AREA-scoped, matching `LdtkEntityCtx::kinematic_path_ref` exactly. A
    # wider index here would call a level healthy that the converter rejects.
    elsewhere = {
        "identifier": "other",
        "fieldInstances": [{"__identifier": "activeArea", "__value": "other"}],
        "layerInstances": [
            {
                "__identifier": "Ambition",
                "entityInstances": [
                    _entity("KinematicPath", "k1", points="0,0;100,0", speed=110)
                ],
            }
        ],
    }
    spawn = _entity("EnemySpawn", "e1", character_id="x", path_ref={"entityIid": "k1"})
    project = _project(spawn, extra_level=elsewhere)
    assert _errors(project) == ["contract.entity_ref_unresolved"]


def test_a_dangling_path_ref_is_refused():
    spawn = _entity("EnemySpawn", "e1", character_id="x", path_ref={"entityIid": "nope"})
    assert _errors(_project(spawn)) == ["contract.entity_ref_unresolved"]


def test_a_platform_authoring_two_motions_is_refused_and_one_is_not():
    both = _entity("MovingPlatform", "m1", sweep_dx=240, loop_dy=-640)
    assert "contract.conflicting_fields" in _errors(_project(both))
    assert _codes(_project(_entity("MovingPlatform", "m1", sweep_dx=240, speed=120))) == []


def test_a_loop_anchor_without_a_span_describes_no_motion():
    lonely = _entity("MovingPlatform", "m1", loop_min_y=-96.0)
    assert _errors(_project(lonely)) == ["contract.companion_field_missing"]
    paired = _entity("MovingPlatform", "m1", loop_min_y=-96.0, loop_dy=-640.0)
    assert _codes(_project(paired)) == []


def test_a_conditional_fields_grammar_is_conditional_too():
    # nine real `Breakable*` placements in sandbox.ldtk carry `respawn_seconds: 0` beside `respawn:
    # OnRoomReload`, and `parse_breakable_respawn` never looks at the number unless `respawn` is
    # exactly `AfterSeconds`.
    inert = _entity(
        "BreakablePlatform", "b1", respawn="OnRoomReload", respawn_seconds=0
    )
    assert _codes(_project(inert)) == []
    live = _entity("BreakablePlatform", "b2", respawn="AfterSeconds", respawn_seconds=0)
    assert _errors(_project(live)) == ["contract.value_refused"]
    missing = _entity("BreakablePlatform", "b3", respawn="AfterSeconds")
    assert _errors(_project(missing)) == ["contract.conditional_field_missing"]


def test_a_zero_radius_loop_is_refused_and_a_real_one_is_not():
    assert _errors(_project(_entity("SurfaceLoop", "s1", radius=0))) == [
        "contract.value_refused"
    ]
    assert _codes(_project(_entity("SurfaceLoop", "s1", radius=200))) == []


def test_a_one_point_path_is_refused_and_a_two_point_one_is_not():
    assert "contract.value_refused" in _errors(
        _project(_entity("KinematicPath", "k1", points="0,0", speed=110))
    )
    assert _codes(_project(_entity("KinematicPath", "k1", points="0,0;100,0", speed=110))) == []


# ---------------------------------------------------------------------------
# the vocabulary itself


def test_the_identifier_list_comes_from_the_contract_and_includes_surface_ramp():
    # the drift that was already live: `SurfaceRamp` is registered in
    # `standard_converters()` and was missing from the hand-typed KNOWN_ENTITIES,
    # so authoring the engine's own fillet failed a green check.
    identifiers = contract_identifiers()
    assert "SurfaceRamp" in identifiers
    assert {"EnemySpawn", "Solid", "LoadingZone", "Portal"} <= identifiers


#: Exactly the keys `contract::FieldContract` deserializes. a key outside
#: this set is silently ignored by BOTH readers — `refused_paterns` would
#: neither parse in Rust nor validate in Python, and the contract would quietly
#: claim less than it means to. This is the only place that can catch it.
RUST_FIELD_KEYS = {
    "name", "presence", "on_invalid", "values", "patterns", "refused_patterns",
    "refused_samples", "normalize", "min_points", "positive", "nonzero",
    "requires_value_of", "requires_fields", "conflicts_with", "entity_ref_target",
    "entity_ref_scope", "default", "sample", "poison", "note",
}
RUST_ENTITY_KEYS = {
    "identifier", "probe_size", "feature", "consumed_elsewhere", "note", "fields",
}


def test_no_contract_key_is_silently_ignored_by_both_readers():
    for identifier, entity in entity_contracts().items():
        assert set(entity) <= RUST_ENTITY_KEYS, identifier
        for field in entity.get("fields") or []:
            unknown = set(field) - RUST_FIELD_KEYS
            assert not unknown, f"{identifier}.{field.get('name')}: {sorted(unknown)}"


def test_a_normalizing_field_uses_a_rule_both_readers_know():
    for identifier, entity in entity_contracts().items():
        for field in entity.get("fields") or []:
            assert field.get("normalize") in {
                None,
                "lowercase",
                "lowercase_underscore",
            }, f"{identifier}.{field['name']}"


def test_normalization_follows_the_parser_rather_than_a_notion_of_case():
    # `CameraClampMode::from_author_value` lowercases AND folds `-` to `_`.
    for clamp in ("zone_bounds", "Zone-Bounds", "ROOM_BOUNDS"):
        project = _project(_entity("CameraZone", "c1", clamp_mode=clamp))
        assert _codes(project) == [], clamp
    # `PortalChannelColorSpec::from_name` only lowercases, so `c-1` is refused.
    assert _codes(_project(_entity("Portal", "p1", color="PURPLE"))) == []
    assert _codes(_project(_entity("Portal", "p1", color="c7"))) == []
    assert _errors(_project(_entity("Portal", "p1", color="c-1"))) == [
        "contract.value_refused"
    ]


def test_every_declared_field_states_a_disposition():
    for identifier, entity in entity_contracts().items():
        for field in entity.get("fields") or []:
            assert field.get("on_invalid") in {"refused", "silent_default", "open"}, (
                f"{identifier}.{field['name']}"
            )
            assert field.get("presence", "optional") in {
                "required",
                "recommended",
                "optional",
            }, f"{identifier}.{field['name']}"
            if field.get("presence") == "required":
                assert field.get("sample"), (
                    f"{identifier}.{field['name']} is required but carries no sample, "
                    "so the Rust prover cannot build a minimal instance with it"
                )


# ---------------------------------------------------------------------------
# the whole loop, on real content


@pytest.mark.skipif(
    not default_sandbox_ldtk().is_file(),
    reason="sandbox.ldtk lives in the ambition_map_assets submodule",
)
def test_the_validate_command_goes_red_on_real_content_missing_a_required_field(tmp_path):
    """Both terms, on the world the game actually ships.

    The `validate` entry point — not the rule module — must produce exactly one
    NEW error when a required field is cleared, and none when it is restored.
    Comparing the two runs rather than asserting an absolute count keeps the
    world's own unrelated diagnostics out of the verdict.
    """
    project = json.loads(default_sandbox_ldtk().read_text())

    healthy = tmp_path / "healthy.ldtk"
    healthy.write_text(json.dumps(project))
    before = {
        (i.severity, i.code, i.entity_iid, i.field) for i in validate_issues(healthy)
    }
    assert not any(code.startswith("contract.") for _, code, _, _ in before), (
        "the shipped world already violates the contract; fix the world, not the test"
    )

    poisoned_project = copy.deepcopy(project)
    victim = None
    for level in poisoned_project["levels"]:
        for layer in level.get("layerInstances") or []:
            for instance in layer.get("entityInstances") or []:
                if instance.get("__identifier") != "EnemySpawn":
                    continue
                for field in instance.get("fieldInstances") or []:
                    if field.get("__identifier") == "character_id" and field.get(
                        "__value"
                    ):
                        field["__value"] = ""
                        victim = instance["iid"]
                        break
                if victim:
                    break
            if victim:
                break
        if victim:
            break
    assert victim, "sandbox.ldtk has no EnemySpawn authoring a character_id to clear"

    poisoned = tmp_path / "poisoned.ldtk"
    poisoned.write_text(json.dumps(poisoned_project))
    after = {
        (i.severity, i.code, i.entity_iid, i.field) for i in validate_issues(poisoned)
    }

    assert after - before == {
        ("error", "contract.required_field_missing", victim, "character_id")
    }
    assert before - after == set()
