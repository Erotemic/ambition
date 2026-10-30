"""A Bool field is never null: bevy_ecs_ldtk refuses a whole world file over
one, and then paints none of its levels' tiles (pvn, sandbox, noodle and
dinosaur did, 2026-10-30). The validator names it; `repair` writes false,
which the game's parser already read null as."""

from ambition_ldtk_tools.validate import (
    fill_null_bools,
    normalize_field_def,
    validate_bool_is_not_null,
)


def a_switch(value):
    return {
        "levels": [
            {
                "fieldInstances": [],
                "layerInstances": [
                    {
                        "entityInstances": [
                            {"fieldInstances": [{"__identifier": "requires_facing", "__type": "Bool", "__value": value}]}
                        ]
                    }
                ],
            }
        ]
    }


def the_field(project):
    return project["levels"][0]["layerInstances"][0]["entityInstances"][0]["fieldInstances"][0]


def test_a_null_bool_is_an_error_and_repair_writes_false():
    project = a_switch(None)
    errors = []
    validate_bool_is_not_null(errors, "switch", the_field(project))
    assert len(errors) == 1 and "null Bool" in errors[0]
    assert fill_null_bools(project) == 1
    assert the_field(project)["__value"] is False
    errors = []
    validate_bool_is_not_null(errors, "switch", the_field(project))
    assert errors == []


def test_a_set_bool_is_left_alone():
    project = a_switch(True)
    assert fill_null_bools(project) == 0
    assert the_field(project)["__value"] is True


def test_a_bool_definition_is_not_nullable_and_defaults_to_false():
    field_def = {"identifier": "opened", "__type": "Bool", "type": "F_Bool", "canBeNull": True, "defaultOverride": None}
    normalize_field_def(field_def)
    assert field_def["canBeNull"] is False
    assert field_def["defaultOverride"] == {"id": "V_Bool", "params": [False]}
