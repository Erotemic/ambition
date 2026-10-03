# Character authoring package

**State:** OPEN. This page owns the remaining character-authoring boundary.

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
- standing height is a catalog fact of the character; art quality scales
  presentation and never changes gameplay height
  (`quality_change_keeps_each_character.rs`);
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

[Composable actor resources](composable-actor-resources.md) applies this same
ownership split to resources. Character authoring can state which optional
resources a character has. A capability owns the meaning of a resource role and
its fill/spend policy. Composition binds the capability role to the character's
resource. The final prepared actor/resource composition carries the resolved
layout and bindings; runtime systems do not search authored names again.

## Current execution work

### A1 — migrate a field only when it has a named bypass

Before moving another fact, find a concrete character value that has one
authoring source but is re-authored, patched or re-derived at game or runtime
composition. Promote a slice only when it names:

1. the current source of truth;
2. the duplicate or override road to delete;
3. the target character or facet owner;
4. the preparation and lowering path;
5. the acceptance test proving the old authority is gone.

This is queue row D166. Search two ways: *who writes a character fact they do
not own*, and *which character facts does construction re-derive instead of
reading from the character*. A missing author and a redundant derivation are the
same defect seen from two ends. A geometry fact is three components (collision
size, render quad, quad offset); closing one of them looks like closing the seam.

**Open residuals:**

- **The stand-in table** (`smash_duelist_a.ron`). The source is a hand-written
  verb list in `game/ambition_demo_smash/src/moveset.rs`; the target owner is
  `SmashRepertoire` -> `into_contract()`, with `borrows:` as the established
  derive. `the_stand_in_is_george_s_genre_shape_with_the_special_button_removed`
  measures the surplus that a correct migration takes to zero. It waits on the
  product question of how thin a stand-in is (Q89 in
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md)).
- **Two display-name keys remain.** `CharacterCatalog::id_for_authored_identity`
  falls back to a display name; over `app_it` and the four demo test binaries
  it was never taken (2026-10-03). The sprite table keys a sheet by its display
  name as well as its id (`CharacterSpriteAssets::declare`). Delete each only
  after measuring that nothing reads it.

**Closed slices and their guards:**

- Knockback weight is authored in George's `smash_fighter.ron`, not patched by
  the demo (`george_carries_the_knockback_weight_his_own_facet_authors`).
- Mary-O's transformation beat follows the catalog row's sheet
  (`every_mary_o_form_resolves_a_real_sheet_in_the_shipped_demo`).
- A sprite-authored body is constructed from its sheet through the same
  `posed_body_geometry` call the pose pass uses
  (`a_sprite_authored_body_is_constructed_from_its_sheet`). `ActorClusterSeed`
  carries the resolved geometry and `render_size`; spawn sites no longer look a
  quad up by placement name (`a_skirmisher_is_drawn_at_the_quad_its_character_resolves`).
- The display-name join of the character demand is deleted. Room staging
  demands a character by the id its placement or request names
  (`EnemySpawnSpec::character_id`, `SpawnActorKind::Enemy { character }`), not
  by its placement name, and the demand's `canonical_character_id` fallback is
  gone. Measured over `app_it` (2026-10-03): 28 demanded tokens reached it, and
  21 of them were captions or placement ids that named no character; the demos
  demanded none. Guard: `every_shipped_room_demands_its_characters_by_id`
  (poison: demand `enemy.name`, and 38 (room, token) demands are named).
- `Vitals::canonical_height` is deleted; height comes from the catalog's <!-- cite-ok: a deleted name -->
  standing height.

**Not residuals:** a demo mechanic keyed on identity (Mary-O's power tier), a
character setting its own facts where it is constructed, and a grid fighter that
authors no fighter body (it plays on the ruleset body
`SMASH_FIGHTER_BODY`; a missing author is a product call). `CharacterDefinition`
has no `with_vitals` builder; that is ergonomics, not a duplicate authority.

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

The second clause is a goal, not an invariant: the display-name keys under A1
are known residuals with no guard.

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

Standing height has enough evidence to be a shared character fact. Other facts
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

## Definitions, preparation and live actors are different responsibilities

[Packet A6](actor-monolith-work-frontier.md) retains this document's field census
but rejects a rename-only `characters -> actors` migration. Authoring schemas and
validated immutable definitions can be consumed without owning live actor query
and mutation. Materialization combines prepared definitions with body builders;
brain policy and moveset execution consume them during simulation.

For each migrated field name its author, validated/prepared representation,
activation boundary, runtime projection and persistent identity. Do not split
one value across a registry and a second runtime default that can disagree.
Mechanical geometry must not derive from resident device texture quality.

The maintainer's iteration cost now supplies a concrete extension-boundary
customer, independently of moveset vocabulary size. I1-I3 in
[fast iteration](fast-iteration-implementation.md) move the pure helper closure
into the existing value owner, emit a portable artifact and reuse prepared cast
admission. They do not move live actor construction into the compiler or serialize
PreparedCharacterDefinition wholesale. A11/A12 remain the installed-technique
and move-flow authority. Preserve one structural body road across controlled
actors, enemies, bosses and summons; shape/profile differences are policy, not
a reason to add a separate player actor authority.
