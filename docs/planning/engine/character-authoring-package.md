# Character authoring package — current migration frontier

**State:** OPEN, narrowed 2026-08-30.

This plan owns the remaining character-authoring boundary. It no longer carries
the full history of the migration.

## Goal

A character's authored facts should be discoverable as one coherent package,
while engine/ruleset capabilities own the schemas and runtime meaning of the
facets they understand.

The rule is:

> Character-specific values live with the character. Reusable semantics live in
> engine/ruleset capabilities. A game selects supported facets; it does not
> rewrite the character after registration.

This is an ownership and authoring boundary, not a universal character ontology.

## Current architecture

The repository already has important pieces of the target:

- `PreparedCharacterDefinition` is the resolved immutable runtime-facing
  character definition. It carries body facts, hurtboxes, abilities, autonomous
  policy, authored movesets, presentation references and validation results.
- canonical character height is authored as a shared character fact; art quality
  scales presentation rather than changing declared gameplay height — ⚠ the
  SECOND clause is guarded (`quality_change_keeps_each_character.rs`), the first
  is only half true, see the height residual under A1;
- ordinary Smash move repertoires are character-authored `MovesetContract`
  values rather than one shared fighter kit;
- `ambition_characters::smash_fighter::SmashFighterFacet` is a typed,
  `deny_unknown_fields` platform-fighter facet with content-pack validation and
  runtime lowering;
- the Smash content pack selects a character's fighter facet without making the
  engine own that named fighter;
- prepared-content validation records which vocabularies were checked and which
  authored references failed, rather than dropping that information after
  preparation.

The first facet is intentionally incomplete as a universal representation. Its
module explicitly does not absorb the ordinary authored repertoire merely to
make every fighter fact use one file format.

## Ownership model

### Character authoring owns

- stable character identity and authoring context;
- character-specific body/presentation source;
- character-specific authored moves and geometry;
- character-specific VFX/SFX references or recipes where they are genuinely
  part of the character's identity;
- ruleset-specific facet values such as platform-fighter capture/body policy;
- source provenance and review artifacts.

### Engine/ruleset capabilities own

- schemas and semantic meaning;
- validation and lowering;
- runtime types and simulation behavior;
- content compatibility/fingerprint rules;
- generic authoring primitives and diagnostics.

### Game/provider composition owns

- which characters/facets are admitted;
- which capabilities are installed;
- match/world policy and participant assignment;
- bindings between provider content and the current experience.

A game should not become a second character database.

## Current execution work

### A1 — re-measure bypasses before migrating another field

The old plan listed a broad M0–M8 migration. That is no longer the right unit of
work. Before moving another fact, find a concrete character value that still has
one authoring source but is being re-authored or patched at game/runtime
composition.

Promote only a slice that can name:

1. the current source of truth;
2. the duplicate/override road to delete;
3. the target character/facet owner;
4. the preparation/lowering path;
5. the acceptance test proving the old authority is gone.

This is queue row D166.

**Censused 2026-08-31, and one slice closed.** Ten sites were examined against
the five-part test. Most are two *legitimate* authors — a demo mechanic keyed on
identity, or a match rule composed through `MatchRules` — and are explicitly not
targets. Three residuals name all five parts:

- ✔ **Knockback weight — CLOSED 2026-08-31.** `smash_reading_of_character` in
  `ambition_demo_smash` was a `match definition.id` writing `Vitals::knockback_
  weight`, an ordinary character fact the engine already owns, for a character
  the demo does not own — the falsifier below in as many words. George now states
  `knockback_weight: Some(1.35)` in his own `smash_fighter.ron`; the two
  stand-ins state theirs where they are constructed, which is authoring rather
  than override. Guard: `george_carries_the_knockback_weight_his_own_facet_
  authors` in `game/ambition_app/tests/smash_in_the_host.rs`, red under both
  poisons (strip the RON field; cut the facet out of registration).
- ✔ **Mary-O `sheet_target` — CLOSED 2026-08-31.** It re-derived a character's
  sheet from its id at runtime when the same pairing was already authored three
  other ways. `clip_seconds` now asks
  `ambition_sprite_sheet::character::catalog_join::sheet_for_character_id_from_data`,
  and the transformation beat follows the catalog row: point the spark form's
  row at another manifest and the beat changes, which is the assertion the old
  table could not have moved. ⚠ the system's catalog and sheets are
  `Option<Res<_>>` — the absence means *"no sheet to read"*, the case the beat
  already answered with its fallback, and
  `every_mary_o_form_resolves_a_real_sheet_in_the_shipped_demo` is what keeps
  that from becoming a silent veto in the shipped demo.
- ▢ **The display-name join** (`character_runtime/mod.rs`'s
  `canonical_character_id` falling through to `id_for_display_name`) is a real
  A3 residual, but content deliberately rides it and room/roster tokens
  legitimately arrive as display names. Not a one-slice promotion — see A3.

⛔ **NOT a residual, and worth stating so it is not re-filed**: eleven of the
fourteen grid fighters play on the actor baseline because they author no fighter
body. That is a *missing author*, not a duplicate authority, and it is a product
call.

### Re-censused 2026-09-02 — NO NEW CANDIDATE, and here is what was searched

D166 said *"re-census before migrating another field"*. Done, and the answer is
that the boundary has no fifth slice waiting: nothing found writes an
engine-owned character fact for a character it does not author. What was swept,
so the next person does not repeat it:

- **id-keyed branches** (`match character_id`, `match definition.id`,
  `match worn.id()`, `match id.as_str()`) across `game/` and `crates/`. The two
  that touch character facts are legitimate by this document's own rule:
  `mary_o::powerups::power_tier` is a demo mechanic keyed on identity (the power
  ladder is Mary-O's product concept, not an engine fact), and
  `attack_hitbox::authored_attack_volume_resolver` matches `Option`, not an id —
  `Some(cid)` vs the player fallback, which is dispatch rather than a table.
- **writes to `definition.vitals` / `.locomotion` / `.movement_tuning`** outside
  `authored/`. Every hit is a character setting its OWN facts where it is
  constructed — `sanic::badnik`, `mary_o::plane`, `mary_o::snake`,
  `player_robot_lineage` — which is authoring. The one demo write to a fact it
  does not own (`smash_reading_of_character`) was the slice that closed on
  2026-08-31, and the crate now says so in as many words at its old site.
- **`ambition_demo_smash/src/lib.rs`'s `definition.movement_tuning = DEFAULT_TUNING`**
  is the nearest thing to a residual and is already adjudicated above: which
  baseline a platform fighter's body uses is the product call, and eleven of
  fourteen fighters not authoring one is a missing author.

⚠ **ONE ASYMMETRY FOUND, AND IT IS NOT A D166 SLICE.** `CharacterDefinition` has
twenty-two `with_*` builders — abilities, locomotion, mount, contact damage,
moveset, sheet, hurtboxes, canonical height — and **none for `vitals`**, so every
character that wants health assigns the public field after `new`. That is an
ergonomic gap, not a duplicate authority: there is no second road to delete, and
the five-part test needs one. Recorded here so it is not re-filed as a residual;
if it is ever worth closing, it closes as a builder addition and not as a
migration.

⛔⛔ **A THIRD residual — RETRACTED IN PLACE 2026-08-31, THE SAME DAY IT WAS
FILED.** It read *"two fields hold one fact… two independent live truths for one
character fact is the second falsifier below"*. **That was wrong, and the
correction is more useful than the claim.** `Vitals::canonical_height` and the
catalog row's `standing_height` have **DISJOINT POPULATIONS**: 18 catalog rows
author a standing height, and neither caller of `with_canonical_height`
(`player_robot_lineage`, Mary-O's three forms) is among them. Two mechanisms used
by different characters, not two truths about one.

⭐ **What IS true, and it is smaller:** `canonical_height` is read by nothing in
gameplay — measured, the only non-test reader in the tree is `moveset_export`'s
JSON dump. The scaling its doc claimed happens at AUTHORING time through
`world_per_pixel_for_height`, whose OUTPUT is what gets stored
(`BodySource::SpriteAuthored`). So it is a record of an authoring input, and the
fix was to make its doc say so rather than to delete a field a tool reports.
⛔ do not re-file this as a migration; the falsifier does not fire.

⛔ **Two names cited in comments do not exist.** `character_id_for_display_name`
(cited at `game/ambition_content/src/duel_arena.rs:61,81`) is really
`id_for_display_name`; `smash_fighter_kit()` is cited as a live generic floor in
five places and no such function exists — only the const `SMASH_FIGHTER_KIT`
survives. `select.rs`'s "adopter count is supposed to be FALLING" note is
therefore measuring something already at zero.

### A2 — keep the first fighter facet load-bearing

The current platform-fighter facet is evidence that character-owned typed facets
can lower through a capability-owned schema. Extend it only when a real
platform-fighter value still requires a game-owned patch or central closed table.

Do not move the ordinary repertoire into the facet merely for representation
symmetry. The current `MovesetContract` authoring already has one character
owner. Migrate it only if the package/validation/review workflow gains a concrete
benefit and the old source is deleted.

### A3 — preserve one preparation authority

Legacy adapters may feed the same preparation boundary during migration, but
there must be one published `PreparedCharacterDefinition` and no downstream
re-derivation from parent/patch/name-search state.

⚠ **The second clause is a GOAL, not a standing invariant — measured 2026-08-31.**
`canonical_character_id` resolves an identity by searching display names, and
`game/ambition_content/src/duel_arena.rs` depends on it on purpose. Recording it
as a known residual is honest; leaving it written as an invariant implies a guard
that does not exist.

A new serialized facet must define its schema/version and content compatibility
behavior before it becomes a stable public format.

### A4 — make authoring inspection useful before building a large editor

The minimum authoring loop is:

```text
inspect character package
    -> edit character-specific source
    -> validate references/semantics
    -> generate compact review products
    -> prepare/run the relevant experience
    -> observe and iterate
```

Move authoring data toward a graphical workbench only when the semantic model is
stable enough that the frontend is exposing real contracts rather than inventing
a second one.

## Shared versus ruleset-specific facts

Canonical height has enough evidence to be a shared character fact. Other facts
remain ruleset-specific until multiple consumers prove shared semantics:

- physical mass/weight;
- locomotion hull policy;
- default movement tuning;
- intrinsic capabilities versus ruleset grants.

Do not generalize those merely because several games use the same character.

## Action-authoring residuals

The completed character-actions campaign is folded here. Two trigger-based
questions remain:

- only generalize prompt layout when a real repertoire exceeds the current
  control surface;
- expose cooldown/charge availability separately from repertoire presence only
  when a real UI/agent consumer needs that distinction.

Input/seat/provider-action routing remains owned by
[`participant-action-system.md`](participant-action-system.md).

## Do not pre-generalize

Do not introduce, without a concrete customer:

- one universal character file;
- one universal rig or animation model;
- one hurtbox model every game must consume;
- one physical `mass` with universal gameplay meaning;
- one repository per character;
- runtime Python dependencies;
- a general sampled vector-field attack representation;
- a full-roster flag-day migration;
- a graphical editor that duplicates unresolved semantics.

## Falsifiers

The boundary is wrong if any of these become necessary:

- a game-owned table rewrites ordinary character facts by character id;
- authoring tooling must define runtime simulation semantics;
- importing a character forces installation of every facet the character has;
- authoring one ordinary use of an existing mechanic requires editing unrelated
  engine registries;
- migration leaves two independent live truths for the same character fact;
- abstraction reduces expressive character-specific mechanics in order to make
  the data look uniform.

## Exit

This program can leave active planning when:

1. character-specific source is discoverable coherently;
2. prepared character values are the only runtime-facing authority;
3. at least one real ruleset facet is authored, validated, selected and lowered
   without a game-owned character table;
4. ordinary edits to migrated facts require no unrelated engine registry edit;
5. another experience can consume the same character identity without pulling
   irrelevant facets;
6. remaining moves are opportunistic authoring/tooling improvements rather than
   an unresolved ownership boundary.
