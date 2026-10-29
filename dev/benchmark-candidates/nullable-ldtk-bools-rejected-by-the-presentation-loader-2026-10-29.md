# Nullable LDtk booleans fail the presentation loader

The prepared world and the Bevy LDtk asset loader read the same world files.
A capture can render the world through the prepared IR while the asset server
reports a failed LDtk load. Inspect the asset log as well as the image.

The committed `ambition_map_assets` sandbox contains 108 Boolean field values
with `__value: null`. The `NpcSpawn.requires_facing` definition sets
`canBeNull: true` and has no default override. The edited working file had 107
such values. The failure is present without the working edits.

`bevy_ecs_ldtk` 0.15.0 deserializes `FieldValue::Bool` as `bool`, unlike nullable
numeric and string fields. It rejects the first null Boolean. The capture log
reported this at `requires_facing` on the sandbox guide.

Resolve the field contract at the authoring and parser boundary. Do not replace
null values with false without checking inheritance semantics. Do not add a
per-map filter that hides the disagreement. Keep the prepared world and the
presentation import on one authored schema.

Source owners:

- `tools/ambition_ldtk_tools/`
- `crates/ambition_platformer2d_ldtk/src/`
- `bevy_ecs_ldtk` 0.15.0, `src/ldtk/field_instance.rs`
