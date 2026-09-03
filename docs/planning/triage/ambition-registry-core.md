# `ambition_registry_core` — one protocol for canonical registries

> **State:** R2 + R3 LANDED 2026-09-03 (crate built, two pilots migrated); R4
> (evaluate before expansion) is next. Was TRIAGE — PROPOSED DIRECTION, 2026-07-22.
>
> ⭐ **What landed (`crates/ambition_registry_core`, dependency-free, ~150
> lines):** `RegistrationMeta { owner, source, schema_id }` validated by
> `require_non_empty` (a blank field is refused BY NAME); `classify(existing,
> incoming) -> New | Idempotent | Conflict { existing }` — deliberately no fourth
> answer, so a silent overwrite cannot be the accidental default;
> `canonical_row` (tab-joined, newline-terminated; a tab or newline in a field
> panics rather than being escaped — an escape would move every fingerprint the
> row enters) and `canonical_section(header, rows)`, which keeps the caller's
> order because a section's row kinds may be ordered by grammar. Nothing
> process-local can enter identity: the types carry strings.
>
> **Pilots:** `ConstructionRegistry` (its entries ARE `RegistrationMeta`; both
> registration roads classify; the dump is canonical rows — 70 construction
> tests green, the prepared-content fingerprint unchanged) and `RollbackRegistry`
> (blank-identity refusal and new/idempotent/conflict read from the core; its
> own wire-identity collision rule stays on top; both dumps are canonical rows —
> `rollback_schema_baseline.txt` byte-identical, which is the proof the
> migration moved no bytes). Workspace policy: member row +
> `engine.ambition_registry_core-dependency-free` (file-omits on the manifest,
> poison-verified). Capability footprint 44 → 45, declared in the baseline: a
> crate count, not bytes; the code it replaced was linked before.
>
> **R4, honestly:** source reduction is small (the two pilots lost ~40 lines and
> gained a dependency); the win is that the third protocol question ("does a
> conflict leave the old registry unchanged") is now answered by one function
> both read, and the next registry cannot answer it differently by accident.
> Candidates that already copy this protocol by hand and would migrate the same
> way: `PlacementLoweringRegistry`, `RoomContentStagingRegistry` (both carry
> their own `EmptyIdentity`). The seven SILENT-overwrite registries in the
> inventory are a different decision each — `classify` refuses to be their
> default, which is the point; each must say "replace" in place or adopt refusal.
> Fingerprint hashing was NOT extracted: rollback's hasher and version prefix
> stay its own, so no ledger moved.
>
> The direction is chosen: introduce a small, dependency-light
> `ambition_registry_core` crate for the protocol repeated across Ambition's
> registries. The exact Rust API remains an implementation decision and should
> be proved through one or two migrations before broad adoption.
>
> **Not a queue card:** this document records the direction only. Promotion into
> [`../tracks.md`](../tracks.md) should happen when the current construction and
> room-transaction campaign has a safe insertion point.
>
> **RE-MEASURED against `3e3c397f2` (2026-09-02). The premise did not decay — it
> GREW, and there is still no shared protocol.**
>
> - ✔ **`crates/ambition_registry_core` NOW EXISTS** — landed `479f9d3e4`,
>   after this re-measurement was taken. ⚠ The bullet below said it did not, and
>   was true at `3e3c397f2`; the drift is hours old, not weeks. ⭐ The crate
>   cites THIS PAGE in its own module docs as the inventory that justified it,
>   so the plan produced the crate and then went on saying the crate was absent.
>   ⇒ **What it is:** not a generic registry — domain crates keep their keys,
>   values, maps, dispatch and resources. It extracts the part that must not
>   drift: `RegistrationMeta`, `classify` (New / Idempotent / Conflict),
>   `require_non_empty`, and the `canonical_row`/`canonical_section` grammar a
>   deterministic dump and a fingerprint both read — `ConstructionRegistry`'s
>   answers, which the inventory found were the only ones deciding all four
>   questions on purpose.
>   ⇒ **Adoption, measured 2026-09-03: FOUR consumers**, which is the number the
>   remaining work is against rather than 31: `shared_tangle/construction/registry.rs`,
>   `platformer2d_runtime/rollback/registry.rs`,
>   `actor_monolith/features/ecs/spawn/content_staging.rs`, and
>   `platformer2d_world/placements.rs`. ⚠ So "there is still no shared protocol"
>   is spent; the live question is ADOPTION across the rest, and a registry whose
>   policy is genuinely different is expected to opt out by not calling
>   `classify` and to say why in place.
> - **31 distinct `pub struct *Registry` types** at HEAD, against **27** at
>   `159daa235` (2026-07-23, the day after this was written). Five arrived in
>   the interval: `ActionRegistry`, `BrainProfileRegistry`,
>   `FrontendAudioRegistry`, `PreparedCharacterRegistry`, `SchemaRegistry`.
> - The only registry-shaped trait in the workspace is
>   `RollbackRegistrar` (`ambition_platformer2d_core/src/snapshot.rs`), which is
>   one domain's registration hook and not the shared protocol proposed here.
>
> ⛔ **SO THE COST THIS DOC PREDICTED IS THE ONE THAT IS ACCRUING.** It argued
> the real expense is semantic drift rather than duplicate lines — registries
> disagreeing on what counts as identity, whether function addresses take part
> in equality, what enters a fingerprint, and whether a conflict leaves the old
> registry unchanged. Five new registries have since made those decisions
> independently, with no shared vocabulary to make them agree and no inventory
> saying which way each one went. A promotion to `tracks.md` should carry that
> inventory as its first deliverable: the four protocol decisions, per registry,
> as they stand — because the abstraction cannot be designed against 31
> registries whose current answers nobody has written down.

## The inventory, taken 0b69bf40b (2026-09-02)

This is the deliverable the note above named as a precondition for promotion:
how all 31 registries currently answer the protocol questions. **No abstraction
is designed here.**

⚠ **Method.** Cells are derived mechanically — the key type from the registry's
own map field, the conflict protocol from the return type of its registration
function. ⛔ "no register fn" means the type is built some other way (a builder,
`From`, deserialisation) and the question is not answered by a signature; it is
NOT a claim that conflicts are unhandled.

⭐ **Citations are `file:line`, and that is a deliberate reversal.** The first
draft of this table used bare paths, on the reasoning that line numbers rot —
they do, and three of them rotted inside a single merge earlier the same day.
The reasoning was still wrong. `scripts/check_planning_citations.py` recognises
exactly two citation forms: `` `path.rs:123` `` and `` `foo::bar` ``. **A bare  <!-- cite-ok -->
`` `path.rs` `` is not a citation to it at all**, so the "safer" form was simply
unchecked — 31 rows nothing would ever re-verify. With line numbers the table
adds 31 checked citations (410 → 441) and the checker's own MOVED/FABRICATED
triage catches the rot instead of me hoping it will not happen.

⇒ **A citation that rots under a checker beats one that cannot rot because
nothing reads it.** ⛔ And the intermediate claim this note used to carry — that
the checker "does not read markdown table cells" — was FALSE and is recorded
here because it was acted on: poisoning a cell with
`crates/ambition_input/src/semantic_NOPE.rs:9999` makes the checker report it  <!-- cite-ok -->
immediately. Tables were never the blind spot; the citation FORM was. ⛔ "no register fn" means
the type is built some other way (a builder, `From`, deserialisation) and the
question is not answered by a signature — it is NOT a claim that conflicts are
unhandled.

| registry | file | identity key | registration fn | conflict protocol |
|---|---|---|---|---|
| `ActionRegistry` | `crates/ambition_input/src/semantic.rs:61` | `SemanticActionId` | `register` | Result |
| `AdaptiveMusicCatalogRegistry` | `crates/ambition_audio/src/music/catalog.rs:353` | `String` | `register` | Result |
| `AudioCatalogRegistry` | `crates/ambition_audio/src/catalog.rs:92` | `String` | `register` | Result |
| `BossCatalogRegistry` | `crates/ambition_boss_encounter/src/catalog.rs:366` | `String` | `register` | Result |
| `CharacterCatalogRegistry` | `crates/ambition_characters/src/actor/character_catalog/registry.rs:201` | `String` | `register` | Result |
| `ConstructionRegistry` | `crates/ambition_platformer2d_shared_tangle/src/construction/registry.rs:228` | `RecipeId (+RelationKind)` | `try_register_recipe` | Result |
| `PlacementLoweringRegistry` | `crates/ambition_platformer2d_world/src/placements.rs:198` | `PlacementKind` | `try_register` | Result |
| `PlatformerAuthoredCatalogRegistry` | `crates/ambition_platformer2d_provider/src/authoring.rs:106` | `String` | `try_register` | Result |
| `RollbackRegistry` | `crates/ambition_platformer2d_runtime/src/rollback/registry.rs:237` | `String` | `try_register` | Result |
| `SchemaRegistry` | `crates/ambition_content_pack/src/schema.rs:253` | `SchemaId` | `register` | Result |
| `SfxBankRegistry` | `crates/ambition_audio/src/catalog.rs:236` | `String` | `register` | Result |
| `GameplaySessionRegistry` | `crates/ambition_game_shell/src/session.rs:71` | `ShellExperienceId` | `register` | bool/Option |
| `ShellExperienceRegistry` | `crates/ambition_game_shell/src/experience.rs:149` | `ShellExperienceId` | `register` | bool/Option |
| `BossEncounterRegistry` | `crates/ambition_boss_encounter/src/registry.rs:16` | `String` | `-` | no register fn |
| `BossProfileRegistry` | `crates/ambition_boss_encounter/src/pattern/profile.rs:213` | `String` | `-` | no register fn |
| `BossSheetRegistry` | `crates/ambition_sprite_sheet/src/boss.rs:103` | `String` | `-` | no register fn |
| `BrainProfileRegistry` | `crates/ambition_characters/src/actor/character_catalog/registry.rs:398` | `String` | `-` | no register fn |
| `CombatBanterRegistry` | `crates/ambition_conversation/src/banter.rs:18` | `String` | `-` | no register fn |
| `MusicRegistry` | `crates/ambition_audio/src/spec.rs:177` | `String` | `-` | no register fn |
| `PortraitSheetRegistry` | `crates/ambition_sprite_sheet/src/portrait.rs:142` | `String` | `-` | no register fn |
| `PreparedSessionRegistry` | `crates/ambition_game_shell/src/preparation.rs:206` | `LoadId` | `-` | no register fn |
| `QuestRegistry` | `crates/ambition_persistence/src/quest/registry.rs:18` | `String` | `-` | no register fn |
| `SfxRegistry` | `crates/ambition_audio/src/spec.rs:27` | `String` | `-` | no register fn |
| `SheetRegistry` | `crates/ambition_sprite_sheet/src/lib.rs:565` | `String` | `-` | no register fn |
| `EncounterRegistry` | `crates/ambition_encounter/src/registry.rs:18` | `String` | `insert` | silent |
| `FrontendAudioRegistry` | `crates/ambition_audio/src/selection.rs:181` | `String` | `declare_route` | silent |
| `GatePortalRegistry` | `crates/ambition_platformer2d_world/src/rooms/gate_portal.rs:77` | `String` | `register` | silent |
| `MovePrefabRegistry` | `crates/ambition_combat/src/moveset/prefab_registry.rs:21` | `String` | `register` | silent |
| `ParamSchemaRegistry` | `crates/ambition_entity_catalog/src/lib.rs:135` | `String` | `register` | silent |
| `PreparedCharacterRegistry` | `crates/ambition_characters/src/prepared.rs:1320` | `ambition_entity_catalog::CharacterId` | `insert_prepared` | silent |
| `RoomContentStagingRegistry` | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn/content_staging.rs:57` | `String` | `register` | silent |

### What the inventory says

⭐ **1. Identity: 23 of 31 key on a bare `String`.** Only eight use a typed id —
`SemanticActionId`, `RecipeId`, `PlacementKind`, `ShellExperienceId`, `LoadId`,
`CharacterId`, `SchemaId`. The workspace defines **42** `*Id` newtypes, so this
is not for want of a vocabulary. This is the drift the page predicted, measured.

⭐ **2. Conflict: three protocols coexist.** 11 return `Result<(), …Error>`, 7
overwrite silently, and 2 return a `bool`/`Option` the caller may discard —
`GameplaySessionRegistry::register -> bool` and
`ShellExperienceRegistry::register -> Option<ExperienceRegistration>`. A caller
moving between two registries cannot carry an expectation with it.

⛔⛔ **3. CORRECTED `1ec8cfb03` (2026-09-03): FUNCTION ADDRESSES DO PARTICIPATE, IN
EXACTLY ONE REGISTRY.** This row previously read "nowhere", and that was wrong.
`PlacementLoweringRegistry::try_register`
(`crates/ambition_platformer2d_world/src/placements.rs:271`) compares
`std::ptr::fn_addr_eq(existing.lower, f)` inside its identical-or-conflict test,
so re-registering one kind with the same owner/source/schema but a DIFFERENT
function is a conflict rather than an idempotent no-op.

⚠ **How the error was made, because the method is the lesson:** I checked whether
entry types DERIVE `PartialEq` and concluded no. The comparison is hand-written
inside the register function, where a derive scan cannot see it — I asked "is
equality derived" when the question was "is equality computed". A sweep for
`fn_addr_eq|ptr::eq` finds it in one command; the workspace's only other hits are
`ledge_grab` block identity. `ConstructionRegistry` stays correctly classified:
its `fn() -> D` is a `PhantomData` marker, not a stored address.

⛔ **AND IT CONSTRAINS THE R4 MIGRATION.** `ambition_registry_core::classify`
requires `E: PartialEq`. An entry holding `LoweringFn<C>` cannot simply derive
it — migrating this registry means writing that `PartialEq` by hand and saying
so, or the classification silently stops distinguishing two different lowering
functions under one key.

⭐ **4. And ONE registry already answers all four questions on purpose.**
`ConstructionRegistry` keys on `RecipeId` in a `BTreeMap` (deterministic order),
validates non-empty identity through `ConstructionRegistrationError::EmptyIdentity`,
documents its idempotence rule in prose — *"Re-registering byte-identical
ownership is idempotent; anything else conflicts"* — and states the fingerprint
constraint outright: its dump is hashed into the prepared-content fingerprint,
so *"a fingerprint sensitive to plugin insertion order would be unusable."*

⇒ **So the design input this page was waiting for is not "invent a protocol".
It is "generalise `ConstructionRegistry`'s, which is already written down, and
decide what to do about 23 String keys and three conflict conventions."**

## R4 evaluation — measured `65cd47e85` (2026-09-03)

### Every pilot ADDED lines, and that confirms this page rather than refuting it

| pilot | registry file | +/− | net |
|---|---|---|---|
| `ConstructionRegistry` (df) | `crates/ambition_platformer2d_shared_tangle/src/construction/registry.rs` | +74 / −68 | **+6** |
| `RollbackRegistry` (df) | `crates/ambition_platformer2d_runtime/src/rollback/registry.rs` | +50 / −36 | **+14** |
| `PlacementLoweringRegistry` | `crates/ambition_platformer2d_world/src/placements.rs` | +46 / −44 | **+2** |
| `RoomContentStagingRegistry` | `crates/ambition_platformer2d_actor_monolith/src/features/ecs/spawn/content_staging.rs` | +40 / −21 | **+19** |

⭐ **Four pilots, four net additions, zero lines saved.** That is exactly what
this page predicted — *"The cost is not primarily literal duplicate lines. The
larger cost is semantic drift"* — and it is worth stating as a measured result so
nobody promotes this campaign on a line-count argument it will not deliver. What
the pilots buy is that the protocol is written once and the DEVIATIONS become
visible; the prose that makes each deviation legible is most of the added lines.

### `classify` fits data, not behaviour

⛔ `ambition_registry_core::classify` requires `E: PartialEq`, and two of the
four pilots hold FUNCTIONS. They split:

- `PlacementLoweringRegistry` stores `LoweringFn<C>`, a plain fn pointer, and can
  compare it — its `PartialEq` is now hand-written so the address is visibly part
  of identity;
- `RoomContentStagingRegistry` stores `Arc<dyn Fn(..)>`, a closure, and **cannot**.
  `Arc::ptr_eq` would call two identical registrations different; comparing only
  the identity fields would call two different stagers the same. It has no
  Idempotent case to have, which is why "a duplicate source is an error" is
  correct rather than lazy.

⇒ **So the core has a boundary the other registries will meet: it fits a registry
whose value is DATA.** A registry whose value is behaviour can take
`RegistrationMeta` and `require_non_empty` and must stop there.

### The "7 silent overwrite" registries: at most 2 are accidents

⛔ **This page's own inventory called these seven "silent overwrite", which reads
as seven accidents. Reading every `register` fn, four state the replace and one
is not a production road:**

| registry | verdict |
|---|---|
| `ParamSchemaRegistry` (`crates/ambition_entity_catalog/src/lib.rs:142`) | ✔ **stated, with a reason** — "Last registration for a key wins (a re-register overrides — content install is the single caller)" |
| `EncounterRegistry` (`crates/ambition_encounter/src/registry.rs:42`) | ✔ **stated** — "Record (or replace) the live entity for an encounter id". It is a live-entity lookup, not an identity registry; replacing is the point |
| `MovePrefabRegistry` (`crates/ambition_combat/src/moveset/prefab_registry.rs:44`) | ✔ **stated** — "Register (or override) a prefab builder under `key`" |
| `FrontendAudioRegistry` (`crates/ambition_audio/src/selection.rs:201`) | ✔ **stated** — "Later declarations of the same route replace earlier ones" |
| `PreparedCharacterRegistry` (`crates/ambition_characters/src/prepared.rs:1427`) | ⚠ **not a production road** — `insert_prepared` is the test hatch its own comment discusses at length |
| `CombatBanterRegistry` (`crates/ambition_conversation/src/banter.rs`) | ⛔ **no stated reason** — "Bulk-register a set of hit-bark lines for one enemy name", nothing about a second registration |
| `GatePortalRegistry` (`crates/ambition_platformer2d_world/src/rooms/gate_portal.rs:82`) | ⛔ **no stated reason** — no doc comment at all; a bare `self.portals.insert(..)` |

⇒ **So the drift is smaller and better documented than the count suggested, and
the two candidates are candidates rather than defects.** Neither was changed:
whether a second gate-portal config for one zone, or a second bark set for one
enemy, should conflict is a question for whoever owns those domains. What the
inventory can say is that they are the only two where nobody wrote down the
answer.

## Problem

Ambition has several independently useful registries whose implementations keep
repeating the same protocol:

- stable registration keys;
- owner, source, and schema metadata;
- rejection of empty or malformed identity fields;
- idempotent re-registration;
- structured conflict reporting;
- deterministic ordering independent of provider registration order;
- canonical dumps for diagnostics and tests;
- stable contribution to prepared-content or snapshot fingerprints;
- tests for order independence, idempotence, and transactional conflict failure.

Examples include construction recipes and relations, rollback registration,
placement lowering, content staging, character catalog fragments, boss catalog
fragments, encounter registration, and quest registration. Their domain values
are different, but their surrounding protocol is increasingly the same.

The cost is not primarily literal duplicate lines. The larger cost is semantic
drift. Recent work repeatedly found registries that differed on what counted as
identity, whether function addresses participated in equality, which metadata
entered a fingerprint, and whether a conflict left the old registry unchanged.
Those are protocol decisions and should not be rediscovered per domain.

## Decision

Create a workspace crate named:

```text
crates/ambition_registry_core
```

Its job is to provide a small shared vocabulary and canonical mechanics. Domain
crates continue to own:

- their key and value types;
- their storage maps;
- their executable functions and runtime dispatch;
- their override or layering policy;
- their Bevy resources and `App` extension methods;
- their domain-specific diagnostics and validation;
- the decision about which registrations affect which content identity.

This is **not** a universal generic registry container. It is a shared protocol
for registries that remain domain-owned.

## Proposed responsibilities

The first version should be intentionally small. Candidate responsibilities are:

### Registration metadata

A common metadata value for stable declarations such as:

```rust
pub struct RegistrationMeta {
    pub owner: String,
    pub source: String,
    pub schema_id: String,
}
```

The exact storage types are not decided here. The important behavior is:

- fields have explicit meaning;
- required fields are validated consistently;
- equality is semantic and stable across builds;
- function addresses and process-local values never define registration identity;
- implementation behavior changes require an appropriate stable schema change.

### Registration outcomes and conflicts

A shared distinction between:

- a newly inserted registration;
- an idempotent equivalent registration;
- a conflicting registration for an already-owned key.

The conflict representation should retain both existing and incoming stable
metadata. Domain registries may wrap it with their own key and context.

### Canonical row emission

A small canonical row or section writer that makes it difficult to accidentally:

- depend on insertion order;
- omit a field from the diagnostic dump but include it in the fingerprint, or
  vice versa;
- use ambiguous separators or ad hoc formatting;
- fingerprint process-local function addresses.

The core should frame stable rows; domain registries decide the row vocabulary.
For example, construction may emit recipe and relation rows while rollback emits
component and resource rows.

### Fingerprint section framing

A shared way to turn a canonical registry dump or row stream into a named
fingerprint section. The crate does not own the application's complete content
fingerprint. It provides the stable section-level mechanics used by the owner.

### Small validation helpers

Only validation that is truly protocol-level belongs here, such as required
nonempty metadata fields and deterministic duplicate/conflict classification.
Domain-specific key and value validation stays with the domain.

## Explicit non-goals

`ambition_registry_core` must not become an `ambition_utils` grab bag.
Specifically, it should not:

- own one generic `Registry<K, V>` used by every subsystem;
- erase domain types behind `Any` merely to share storage;
- register executable provider callbacks for closed domains;
- infer schema changes from function pointer identity;
- depend on Bevy unless a later proof shows a genuinely shared Bevy adapter is
  worth the fanout;
- own complete prepared-content assembly;
- absorb stable-ID design, test fixtures, serialization helpers, or unrelated
  collection conveniences;
- force registries with intentionally different layering semantics into one
  policy.

A useful test for every proposed addition is: *does this encode a registry
protocol invariant, or is it merely code two registries happen to use?*

## Dependency boundary

The preferred crate is dependency-free or nearly dependency-free. It should sit
below the domain registries and change rarely.

Bevy-facing conveniences should initially stay in the owning crate. A broad,
frequently edited foundation crate would increase rebuild fanout across an
already large workspace and would defeat the purpose of a stable core.

The crate must not depend on registries that depend on it. Fingerprint integration
should use stable bytes or rows supplied upward to the lifecycle owner rather
than pulling domain registries downward into the core.

## First migration

Do not migrate every registry at once. Choose two registries that already share
the intended semantics but exercise different domains:

1. the construction recipe/relation registry;
2. the rollback registry or one provider-fragment registry.

The pilot should answer:

- Is `RegistrationMeta` sufficiently expressive without becoming domain-aware?
- Can canonical dump and fingerprint bytes be generated from the same rows?
- Can conflict errors remain more informative than they are today?
- Does the abstraction remove policy drift rather than merely move lines?
- Is source code still obvious to a maintainer or coding agent reading one file?
- Does the crate avoid dependency cycles and excessive rebuild fanout?

If the second registry requires awkward adapter code or weaker diagnostics, stop
and narrow the abstraction rather than forcing adoption.

## Migration phases

### Phase R1 — inventory and invariant table

Before code, inventory active registries and record for each:

- stable key;
- owner/source/schema metadata;
- idempotence policy;
- conflict policy;
- ordering rule;
- canonical dump format;
- fingerprint consumer;
- executable/runtime fields that must remain outside stable equality;
- provider layering or override rules.

This table is migration evidence, not a permanent source-of-truth document. It
may be deleted when all decisions are represented by code and tests.

### Phase R2 — build the narrow core

Implement only the metadata, outcome/conflict, canonical-row, and section-framing
pieces demonstrated by the pilot registries.

### Phase R3 — migrate two registries

Move the selected registries without changing their public domain behavior.
Retain or improve their existing poison tests:

- registration order does not change canonical output;
- identical registration is idempotent;
- conflicting registration is rejected transactionally;
- stable schema changes move the relevant fingerprint;
- runtime function addresses do not affect canonical identity.

### Phase R4 — evaluate before expansion

Measure source reduction, diagnostic quality, compile fanout, and API clarity.
Only then decide which additional registries should migrate. Some registries may
correctly remain independent because their policy is genuinely different.

## Acceptance criteria for promotion

Promote this direction into the execution queue only when a bounded card can
name:

- the two pilot registries;
- the exact shared invariants being extracted;
- the crate's dependency ceiling;
- tests that distinguish semantic identity from runtime function identity;
- a no-behavior-change migration strategy;
- an explicit list of registries not being migrated in the first pass.

The work is successful when registry implementations become smaller **and** it
becomes harder for them to disagree about canonical identity, conflict handling,
and fingerprinting. Line-count reduction alone is not sufficient.
