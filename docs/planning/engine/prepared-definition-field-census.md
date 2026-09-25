# Prepared character definitions — per-field dependency census

This page is the field/use census that A6's hold asked for ("make a field/use
census before moving types"). It does not claim that a type move has landed, and
it proposes no boundary. The selected uses at the end guide the independent
authoring boundary (I1/I2); they do not authorize a wholesale character-domain
split.

## Instrument

```bash
python3 scripts/measure_field_readers_by_seal.py PreparedCharacterDefinition \
    --diff dev/prepared_definition_members.json
```

The instrument is a deprecation seal, not a text scan. The field names that
matter (`id`, `body`, `kit`, `mount`, `vitals`, `sheet`, `provider`) appear on
many types, so attributing an access needs type inference. `#[deprecated]` makes
the compiler report every use site in one pass, including reads through a codec,
a `Reflect` path, a macro or a helper. A visibility seal does not work: a private
field is an error, so the build stops at the nearest dependent. A text scan
undercounts, which is the dangerous direction because a field then looks cheap to
move.

`dev/prepared_definition_members.json` holds the last member list and the commit
it was taken against. Diff the members, not the total: a stable total can hide a
moved member.

Limits the script prints on every run: a consumer carrying `#[allow(deprecated)]`;
a `derive` on the struct warns inside the owning crate; a consumer behind a
feature this build does not enable; a workspace already red for an unrelated
reason. The seal does not distinguish a read from a write (use
`scripts/measure_state_writers.py` for writers). It tags only `pub` fields:
`PreparedCharacterDefinition` has 32 fields, and `voice`, `cue_dependencies`,
`vfx_dependencies`, `checked` and `unresolved` are private and never counted.

A production/test split of the sites is a second instrument over the same
population; different split methods disagree by a few sites. Compare per-site
member lists, not split totals.

## Who reads what (production)

Last run: 216 sites across 27 `pub` fields, at `9cf7cec50` (`--workspace
--all-targets`, so tests are included). Production field counts per crate:

| crate | fields | role |
|---|---:|---|
| `ambition_characters` | 27 | the definition's own home |
| `ambition_platformer2d_actor_monolith` | 15 | live runtime |
| `ambition_platformer2d_actor_spawn` | 13 | materialization / construction |
| `ambition_app_tools` | 11 | tool binaries |
| `ambition_combat` | 3 | action execution: `authored_moveset`, `kit`, `ranged_execution` |
| `ambition_body_seed` | 3 | body seeding: `body`, `locomotion`, `vitals` |
| `ambition_match` | 3 | match activation |
| `ambition_demo_smash` | 2 | ruleset |
| `ambition_sim_harness` | 1 | harness |
| `ambition_content` | 1 | `portrait`; its other reads are tests, including the content guard `a_character_states_its_policy_in_one_place` |

`ambition_app`, `ambition_demo_mary_o` and `ambition_platformer2d_provider` read
the definition only from tests.

`ambition_body_seed` and `ambition_combat` are the clean slices: each reads a
coherent, non-overlapping group and nothing else.

`ambition_app_tools` reads eleven fields, but only `kit` (in `moveset_export.rs`,
`moveset_takes.rs` and `ko_envelope.rs`) and `portrait` drive logic. The other
nine are serialized by `character_json` into the moveset bundle. That bundle
carries a schema id (`ambition.moveset_inspector.v2`) with a bump rule, and its
only consumer is `tools/ambition_moveset_inspector` in this repository, so moving
one of those nine changes a versioned JSON schema, not tool behaviour.

## Dual-read fields

Nine fields are read by both the spawn road and the runtime, so preparation and
materialization are not already separated:

```
both          autonomous_profile  death_traits  id  kit  motion_model
              mount  movement_tuning  provider  sheet
spawn only    body  held_item  hurtboxes  vitals
runtime only  display_name  locomotion  portrait  provoked_profile
              provoked_profile_id  ranged_execution
```

`kit` ranks first by sites and by consumer crates, by a wide margin, and it is the
one semantic tool dependency. `motion_model` has the fewest sites but one per
consumer crate, so it has no cheap side. Run the instrument for current counts.

Fields with at most one consumer outside the owning crate are already clean and
need no work: `contact_damage`, `dream_seed`, `lineage`,
`preserves_mirror_symmetry`, `ranged_vfx` (owner only), `hurtboxes`
(`actor_spawn`), `practice_target` (`ambition_content`), `provoked_profile` and
`provoked_profile_id` (`actor_monolith`).

## The authority axis

The seal measures readers of one struct, so no pair of its sites can be two
authorities. All production runtime reads resolve through
`PreparedCharacterRegistry` or a `&PreparedCharacterDefinition` handed to them.
The real second authority is `CharacterCatalog`, which
`CharacterAuthorityConflict` (`character_runtime/audit.rs`) already names. The
decidable question per field is whether the fact has a second home in the catalog
and, if so, whether a disagreement is watched.

| field | second home | reading |
|---|---|---|
| `kit`, `death_traits`, `mount` | none | two readers of one authority; nothing to do |
| `id` | the key both maps use | nothing to do |
| `provider` | `entry.provider` + `CharacterCatalogOwners` | two authorities, watched (`ProviderDisagreement`) |
| `sheet` | `entry.spritesheet` / `entry.manifest` | two authorities, watched (`SheetDisagreement`) |
| `autonomous_profile` | the catalog's named-profile library | one resolution (`resolve_autonomous_profile` in `prepared.rs`); the catalog's `default_brain` names a `BrainPreset`, a different vocabulary |
| `movement_tuning`, `motion_model` | `catalog.axis_tuning(id)` / `catalog.motion_model_spec(id)` | one authority: the preparation barrier folds the catalog row in (`prepared.rs`) |

A value read at spawn and during simulation can legitimately be one immutable
definition; two reads do not require two authorities or two copies. Adding the
folded fields to `CharacterAuthorityConflict` would be a variant that cannot fail.

No generic resolver, no request bus, no new trait to unify registry and catalog,
and no type moves.

### Open: what an unprepared id inherits at wear time

⭐ DECIDED 2026-09-25 (AP30): a catalog row IS a character, so the barrier
prepares every row nobody authored as a bare definition, and the fold gives it
its row and its provider's declarations. The shipped host now has no unprepared
catalog id (147 of 147 prepared, where it was 58), so the read-time fold below
answers only for an id no catalog knows, or where no barrier ran. Deleting it is
AP30's second half. The paragraph below is the measurement that led here.

The fold is spelled twice: `avatar/starting_character.rs` re-performs it at read
time for ids the registry does not hold. Measured in every composition (the
shipped host through the launcher and directly, and all four demos), no catalog
row that authors feel is orphaned, and in the demos the read-time fold is never
reached. Deleting it anyway turned six tests red on the wear/re-wear road, one of
which asserts that an unauthored character inherits its catalog row. So the
deletion is a design decision, not a cleanup: rule on what an unprepared id should
inherit at wear time. Until then the fold stays spelled twice, and
`game/ambition_app/tests/authored_feel_reaches_the_prepared_cast.rs` keeps the
orphan case from arising in real compositions.

Three smash fighters author `Some(DEFAULT_TUNING)` against a silent catalog row
where the character is constructed in `ambition_demo_smash`. That is the
authoring road, not a disagreement; its comment files the remaining fighters'
tuning for a later slice.

### Open: the autonomous profile reference is not retained

`finalize_character` resolves both policy fields from an inline value or a named
reference, but keeps only the provoked one: `provoked_profile_ref` survives as
`provoked_profile_id`, and `autonomous_profile_ref` survives as nothing. After
preparation, "authored inline" and "named a profile" are indistinguishable for the
autonomous policy. It blocks nothing today; a re-stage, a hot revision or a save
codec would each need the distinction.

## Selected uses for the independent authoring boundary

This census guides I1/I2; it does not require copying PreparedCharacterDefinition
wholesale into a portable schema or moving all its readers to one new crate.

| Field family | Selected responsibility | Migration rule |
| --- | --- | --- |
| id/provider and logical references | Pure identity where already appropriate | Preserve canonical meaning; no parallel identity algebra |
| authored moves, kit, motion model and tuning | Immutable mechanical definition with owner validation | Portable only through pure owned values; live application remains with the body/action owner |
| body/hurtboxes/vitals | Authored facts plus typed construction input | Separate from live body health/materialization; do not serialize ECS state as content |
| autonomous/provoked policy | Prepared controller policy and referenced identity | Preserve the same resolver at spawn, live selection and rewind; do not move brain execution into the compiler |
| sheet/portrait/voice | Logical asset/presentation references with declared mechanical dependencies | Collision-bearing metadata is mechanical even if an art tool produced it |
| checked/dependency inventories | Derived preparation/discovery evidence | Recompute from one validator; do not trust a serialized 'checked' bit as installed admission |

For each moved field, record preparation input, runtime reader, invalidation and
reload policy. Test a nondefault value through the consumer. Follow
[generation/reload](content-generation-and-reload.md).
