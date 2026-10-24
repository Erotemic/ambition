"""An entity reference's editor value is its target's iid.

The editor reads `realEditorValues`, not `__value`. A reference written as
`V_String` of the whole ref object (a Python dict repr) resolved to nothing in
the editor, and the next editor save nulled the reference.
"""

from ambition_ldtk_tools.validate import editor_value_for, real_editor_values_are_acceptable

REF = {"entityIid": "EnemySpawn-6806", "layerIid": "Ambition-4477", "levelIid": "lookout-4475", "worldIid": "w"}


def test_a_reference_is_stored_as_its_target_iid() -> None:
    assert editor_value_for(REF, "EntityRef") == [{"id": "V_String", "params": ["EnemySpawn-6806"]}]


def test_the_dict_repr_shape_is_not_acceptable() -> None:
    stale = {"__type": "EntityRef", "__value": REF, "realEditorValues": [{"id": "V_String", "params": [str(REF)]}]}
    assert not real_editor_values_are_acceptable(stale)
