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
  scales presentation rather than changing declared gameplay height;
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

The old `character-actions.md` campaign is folded here. Two trigger-based
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
