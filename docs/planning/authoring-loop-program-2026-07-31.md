# The authoring loop — content packs, participant actions, causal inspection

**Armed:** 2026-07-31, on Jon's instruction. This file is the SPINE for the
three-program architecture campaign; rows land here and are mirrored as `▢` in
[queue-72h-2026-07-31.md](queue-72h-2026-07-31.md), which the guard reads.

**The objective, in Jon's words:**

> An agent should be able to create a mechanically and aesthetically distinct
> game from the public SDK and content documentation, validate it quickly
> without rebuilding the engine, control it consistently across gameplay and UI
> contexts, and diagnose why any important simulation result occurred without
> reading engine implementation code.

One loop, not three subsystems:

```text
author → validate → control → simulate → explain
```

---

## What already exists — read this before building anything

This program is **not** starting from zero, and the largest risk to it is
rebuilding a substrate that is already here under a different name. Surveyed
2026-07-31 against source, not against docs.

| Program | Already built | Where |
|---|---|---|
| A | authored RON catalog + parser + App-local fragment registry that MERGES across providers | `ambition_characters::actor::character_catalog` |
| A | a 698-line cross-content validator (LDtk links, dialogue ids, quest conditions, encounter/boss ids, music refs) | `game/ambition_content/src/content_validation.rs` |
| A | `UnresolvedRef` + `explain(id, declared_by)` — the resolution-diagnostic shape, twice | `ambition_platformer2d_shared_tangle/src/binding.rs:451`, `ambition_render/.../item_visuals.rs:139` |
| A | `PreparedCharacterDefinition`, `PhysicalRetraction`, `DisplacedPhysicals` — preparation/lowering already exists for ONE family | `ambition_platformer2d_actor_monolith/src/character_runtime/` |
| B | `ParticipantId`, `InputParticipant`, `InputContextId`, `ContextClaim`, `ParticipantContexts`, `resolve_active_input_context` | `crates/ambition_input/src/participant.rs` |
| B | `SeatMenuFrames` — seat-keyed menu input with per-seat repeat state | `crates/ambition_input/src/menu.rs` |
| B | `UiCue`/`ActiveUiCues` — context-keyed prompt projection | `crates/ambition_input/src/cues.rs` |
| B | `KeyboardPreset::input_map()` → `InputMap<Platformer2dInputActionMonolith>`, `action_label()` | `crates/ambition_input/src/presets.rs` |
| C | a rolling flight recorder: frames + discrete events + markdown/JSON dump | `crates/ambition_gameplay_trace/` |
| C | `AMBITION_FIGHTER_TRACE=1`, `ladder_probe`, the rollback observatory | fighter brain, `app_it` |

**So the work is not "build three systems". It is: find where each substrate
stops being an authority, and move the boundary.** The four places it stops are
below, each with the evidence that says so.

---

## The four load-bearing gaps, with evidence

### ⛔ A-gap 1 — content reaches the runtime through `include_str!`

`game/ambition_content/src/character_catalog.rs:12`:

```rust
pub const CHARACTER_CATALOG_RON: &str = include_str!("../assets/data/character_catalog.ron");
```

and the public facade's content seam takes the same shape —
`CharacterContent::Ron(&'static str)` in `crates/ambition_platformer2d/src/app.rs`.

**Editing a character therefore requires a Rust rebuild (~10 min in this repo).**
The program's headline acceptance criterion — *"valid character addition →
validates without rebuilding Rust"* — is false today, and it is false **at the
public API**, which is the stated priority trigger *"the public API is about to
harden around the wrong concept"*.

⚠ do not read this as "delete `include_str!`". A shipped binary embedding its own
content is correct for distribution. What is wrong is that it is the ONLY path,
so the authoring loop inherits the distribution constraint.

### ⛔ A-gap 2 — validation is a fixed-arity function, not a registry

`validate_content_graph(&music, &project, &character_catalog)` is a function of
three hardcoded content families. A capability cannot register a schema; adding
a content family means editing this signature and its body. There is no pack
identity, no fingerprint, no canonical ordering, no capability-requirement
check, no source manifest, no alias/symlink dedup, and **no CLI** — the only
caller is a `#[cfg(test)]` block at line 655.

### ⛔ B-gap — `ActiveInputContext` is a global resource resolved from seat ZERO

`crates/ambition_input/src/participant.rs:167` and `:190`:

```rust
#[derive(Resource, ...)]
pub struct ActiveInputContext { open: Vec<InputContextId> }

// resolve_active_input_context:
participants.iter().find(|(p, _)| p.id == ParticipantId::PRIMARY)
```

The claims are per-participant and correct. **The resolved answer is not.** One
global context stack, computed from player one, governs everyone. Consequences
that are already reachable:

* seat 2 opening a menu is invisible to the router — its actions keep flowing to
  gameplay;
* seat 1 opening a menu silently takes gameplay away from seat 2;
* a participant with no controlled body cannot own a context distinct from the
  primary's.

This is the same defect class the repo already fixed one layer down: `SeatMenuFrames`
exists precisely because `MenuControlFrame` was "ONE global answer". The context
resolution never got the same treatment.

⚠ **two authorities currently disagree** — the second stated priority trigger.
`ParticipantContexts` says input ownership is per-seat; `ActiveInputContext` says
it is global. Both ship.

### ⛔ B-gap 2 / C-gap — the action vocabulary is closed, and causality is TEXT

`Platformer2dInputActionMonolith` (`crates/ambition_input/src/actions.rs`) is a closed
`#[derive(Actionlike)]` enum behind the `input` feature. A capability cannot add
a semantic action; gameplay and menu actions share the enum, split by
convention and by which system reads them.

And the causal substrate is a **player-shaped** flight recorder:
`GameplayTraceFrame` carries `PlayerTraceState`. That is player-centrism in the
observation layer — the one thing Ambition's core values name first. There is no
`explain_tick`; `git grep 'fn explain'` returns two unrelated ref-resolution
helpers. The fighter's decisions are observable only as **printed lines** under
`AMBITION_FIGHTER_TRACE=1`, which is the exact thing Program C forbids
("do not build the inspector by parsing text logs").

---

## Where this lands architecturally — the hole is already reserved

`crates/ambition_platformer2d/src/app.rs:435`, written before this program existed:

> Slice A holds only what host assembly consumes. **`ContentPackDraft` and
> everything under it is slice B**; a content method here would be a method
> whose input nothing can yet validate.

ADR 0032 already named `ContentPackDraft` and deferred it for the stated reason
that nothing could validate it yet. Program A **is** that validator. This is not
a new layer bolted beside the facade; it is the facade's own declared next slice.

---

## Sequencing — three connected vertical slices

Each slice ends with: the displaced path deleted, one positive test, several
negative tests, and the API leaks recorded. No slice waits on another's polish.

### ▢ P1. Slice 1 — one compiled character, its actions, one explanation

The narrowest end-to-end cut of all three programs at once.

> **✔ STATUS 2026-08-03 — P1a and P1b are DONE, and the blocker they left is
> CLOSED.** `crates/ambition_content_pack` (the compiler), `crates/ambition_content_cli`
> (the front door) and `ambition_characters`'s `character_catalog` schema all
> ship. The honest half recorded on 07-31 — *"the RUNTIME still reads
> `include_str!`; nothing consumes a `PreparedContentPack` yet"* — no longer
> holds: `game/ambition_content/src/character_catalog.rs` compiles the pack and
> lowers it, in `load_catalog` AND in `register`, so the running game and the
> validator are one read of one file.
>
> **What is actually left is scope, not a blocker**, and it is tracked in
> [the migration ledger](#the-migration-ledger--which-families-are-in-the-pack).
> One family of thirteen is in. See that section before writing anything here.

**P1a — `PreparedContentPack` is a value, and a character validates off disk.**
New engine crate `ambition_content_pack`:

```text
ContentPackDraft
  ↓ parse            (source manifest: every file read, canonicalised, deduped)
  ↓ schema resolution (SchemaId → installed handler, or UnknownSchema)
  ↓ reference resolution (UnresolvedContentRef<T> → ResolvedContentRef<T>)
  ↓ capability validation (declared requirement → installed capability)
  ↓ conflict detection   (duplicate canonical identity, ambiguous ownership)
  ↓ canonical ordering
  ↓ fingerprint
PreparedContentPack
```

Carries: pack id + version, module namespace, source manifest, declared
capability requirements, schema versions, content identities, asset identities
+ provenance, canonical ordering, resolved references, diagnostics, fingerprint.

First registered schema: the character catalog, owned by the characters
capability. First migration target: `game/ambition_content`. **Do not redesign
every content family before one pack works end to end.**

**Guard:** `cargo test -p ambition_content_pack` green, and a `content_pack`
module in `app_it`.

**P1b — the validator has ONE implementation and a CLI front door.**
The same `validate` used by the standard test, CI, the CLI, dev reload and
packaging. The CLI is a diagnostic door, not the enforcement.

**Acceptance, and it is a measurement not a claim:** build the validator once,
edit a character RON, re-run — **no cargo invocation** — and get the new verdict.
Record the latency. Then probe each hard error red: unknown schema, missing
preset, missing asset, unresolved character/role/action/world ref, duplicate
identity, uninstalled required capability, an unknown field (which must NOT be
silently ignored), and the same file reached through a symlink.

**Guard:** a `validate_without_rebuilding` test that shells the built binary,
and the eight negative cases named above.

**P1c — the participant's semantic gameplay actions route to that character.**
Depends on P2a below only for the multi-seat case; the single-seat route can
land first.

**P1d — explain one movement and one attack decision.**
`harness.explain_tick(tick, actor_id)` answering *why did this body move* and
*which authored move was scored vs which began playback*. Headless API first;
the panel is later and smaller.

**Guard:** an `explain_tick` module in `app_it`.

### The migration ledger — which families are in the pack

**Read this before claiming the compiler is or is not "done".** The compiler is
built; what remains is moving families through it. `pack.ron` says so itself:
*"the only migrated family is the character catalog… This grows as families
migrate."*

Every ▢ row below still reaches the runtime through `include_str!` **and its own
parser** — which is the two-readers-of-one-file shape the compiler exists to
remove. Inventory taken 2026-08-03 by grepping `include_str!` in
`game/ambition_content/src`, not from memory.

⭐ **migrating a family finds bugs the old reader could not see, and that is the
argument for doing the rest.** `items.ron` is POSITIONAL — the slot index binds
a row to its `Item` discriminant and there is no key in the file. Deleting one
row therefore does not remove one item, it re-authors twenty-three: every later
row shifts up a slot, and the short tail falls back to built-in defaults so the
grid still *looks* full. No parse error, no missing reference; `from_ron`
accepted it. Look for the invariant a family's own reader cannot check.

| family | authored source | in the pack? |
|---|---|---|
| character catalog | `data/character_catalog.ron` | ✅ `character_catalog`, owned by `ambition_characters` |
| items | `data/items.ron` | ✅ `item_catalog`, owned by `ambition_items` (2026-08-03) |
| boss seeds | `data/boss_seeds.ron` | ✅ `boss_seed_library`, owned by `ambition_characters` (2026-08-03) |
| boss calibration | `data/boss_validator_bands.ron` | ✅ `boss_validator_bands`, same owner (2026-08-03) |
| enemy roster | `data/character_archetypes.ron` | ✅ `character_archetypes`, owned by `ambition_combat` (2026-08-03) |
| boss profiles | `data/boss_profiles.ron` | ✅ `boss_profiles`, owned by `ambition_characters` (2026-08-03) |
| boss sheets | `data/boss_sheets.ron` | ▢ owner `ambition_sprite_sheet`, which pulls `bevy_render` — needs the placement analysis the profile vocabulary got |
| boss sprite/anim maps | Rust `BTreeMap`s in `bosses/mod.rs` | ▢ content authored as CODE; must become data before a schema can own it |
| boss encounters | 9 × `data/boss_encounters/*.ron` | ✅ `boss_encounter`, `ambition_characters` — validated 2026-08-03, **LOADED 2026-08-04** once the schema could say how nine sources merge |
| encounter waves | `data/encounters/goblin_encounter.ron` | ✅ `encounter_waves`, owned by `ambition_encounter` (2026-08-04). ⭐ found: an empty wave list means what OMITTING the key means |
| music registry | `assets/audio/music_registry.ron` | ✅ `music_registry`, owned by `ambition_audio` (2026-08-03). ⚠ GENERATED — a refusal is a generator bug |
| sfx registry | `assets/audio/sfx_registry.ron` | ✅ `sfx_registry`, same owner (2026-08-03) |
| dialogue | 7 × `assets/dialogue/sandbox/*.yarn` | ▢ not RON; needs a handler that parses Yarn |
| worlds | the LDtk projects | ▢ |
| vanity cards | `data/vanity_card{,_made_this_meme}.ron` | ▢ |
| fighter brain ladder | `data/fighter_brain_ladder.ron` | ◐ `fighter_brain_ladder`, owned by `ambition_characters` (2026-08-07) — declared, validated and LOWERED. ⛔ **but not yet CONSUMED**, which for this family is the actual defect: it had zero production readers, not two. See below. |

### ⛔ The fighter brain ladder is not a MIGRATION — it is a bug (2026-08-07)

Every other ▢ in the table above is the two-readers-of-one-file shape: the
runtime reaches the data through `include_str!` and its own parser, and the
compiler exists to make that one read. **The fighter brain ladder is not that.**
`fighter_brain_ladder.ron` is parsed in exactly one place — a content TEST
(`game/ambition_content/tests/fighter_brain_ladder.rs`) — and by nothing else in
the workspace. Ambition authored a nine-rung difficulty ladder and the running
game has never read a row of it.

⭐ **the engine says this is exactly what should not happen.**
`FighterBrainProfile::for_level`'s own doc calls itself *"a FLOOR, not the
ladder. A game that cares ships its own nine rows
(`FighterBrainLadder::from_ron`) and this is never consulted"*. Ambition ships
the nine rows. The floor is consulted anyway, at both production call sites
(`brain_builders.rs:86`, `character_catalog/resolver.rs:154`).

**What the game actually plays, measured against what is authored:**

* ⛔⛔ **every difficulty scores moves with the LEVEL-9 weight set.**
  `for_level` hands every rung `UtilityWeights::default()`, and `default()` is
  `v1()` — `reach_fit 1.0, frame_advantage 0.6, kill_potential 0.4, stage_risk
  -0.8, expected_payoff 0.5`, which is the ladder's level 9 verbatim. The
  ladder's whole second axis is that a low rung does NOT notice things: L1
  authors `kill_potential: 0.00` and `expected_payoff: 0.00`, and its comment
  says why — *"not noticing which move hits harder IS a difficulty statement"*.
  That axis does not exist in the shipped game. A level-1 CPU prices a smash
  exactly as a level-9 does, and the only thing separating difficulties is
  reaction speed and noise.
* **the numbers differ on the reachable rung.** `fighter_level` defaults to 5.
  Authored L5 is `reaction 300ms / apm 200 / noise 0.20 / read 0.2`; the floor
  computes `325 / 270 / 0.275 / 0.3`. So the shipped level-5 fighter acts 35%
  faster than authored, and is noisier.
* ⚠ **the L3 divergence is NOT reachable, and saying so matters.** `for_level`
  sets `rollout_depth: 12, rollout_k: 4` for level ≥ 6 while the ladder authors
  ZERO on every row deliberately (*"§12.7 gates authoring nonzero rows on FB6e's
  instruments"*). Nothing authors a level ≥ 6 fighter today — the default is 5
  and smash's fragment says 1 — so the ungated rollout layer is latent rather
  than live. It becomes live the day anyone authors a hard CPU.

⭐ **so this row is worth more than its position in the table suggests.** Loading
it is not a cleanup that leaves behaviour identical, which is what migrating the
other families was; it is a change to how every CPU in the game evaluates a move.
Whoever takes it should expect the fighter-brain tests to move, and should treat a
difficulty-feel change as the POINT rather than as a regression.

**The shape, scoped 2026-08-07 so the next session does not re-derive it.** Three
pieces, and the third is the whole cost:

1. **the schema** — `ambition_characters` already owns four
   (`character_catalog`, `boss_seed_library`, `boss_validator_bands`,
   `boss_profiles`), so this is a fifth in the same crate with no placement
   question. `crates/ambition_encounter/src/content_schema.rs` is the template at
   ~180 lines. ⭐ the "what can this family say that its parser accepts and the
   runtime cannot use" question — which the template says is the point of
   migrating — is ALREADY answered here: `FighterBrainLadder::problems()` exists
   and the content test asserts it empty (monotone in reaction/APM/noise, never a
   zero reaction). The handler wires that up rather than inventing it.
2. **registration + lowering** — mechanical, mirrors the four siblings.
3. ⛔ **the consumption, which is where the work actually is.** Both production
   call sites are PURE FUNCTIONS with no access to a resource:
   `enemy_default_brain(enemy: &ActorConfig) -> Brain` (`brain_builders.rs:86`)
   and the catalog resolver (`resolver.rs:154`). A loaded ladder has to reach
   both, and the tempting fix — read it at each site — reintroduces the two-sites
   shape this program exists to remove.
   ⭐ **the right seam is ONE function**: `profile_for_level(level, ladder:
   Option<&FighterBrainLadder>)` that falls back to `for_level` when no game
   shipped rows, with both call sites routed through it. That also states the
   engine's own rule — floor unless the game overrides — in code instead of in a
   doc comment, which is why `for_level` was consulted anyway.
   ⚠ threading the ladder to those two sites is a signature change through their
   callers. That is the estimate: the schema is a morning, the threading is the
   slice.
   ⭐ **the carrier already exists, found 2026-08-07 and it lowers the estimate**:
   `BrainBuildContext` (`character_catalog/binding.rs`) is a per-spawn context
   already threaded into `brain_from_preset_with_context`, carrying
   `spawn_world_x` and `patrol_radius`. The ladder is the same KIND of thing — a
   per-spawn fact the resolver needs and cannot look up — so it belongs there
   rather than in a new parameter on four functions. ⚠ the catch: today the
   `Fighter` arm lives in the CONTEXT-FREE `brain_from_preset`, and the context
   variant delegates to it for every non-patrol preset. So the Fighter arm has to
   move into the context-aware path first, which is the same shape the patrol arm
   already has.

**✔ the seam and the LOAD are both landed** (2026-08-07): `profile_for_level` in
`ambition_characters`, and a `fighter_brain_ladder` schema declared in `pack.ron`,
registered in both registries, and lowered into the prepared pack — asserted by
`the_prepared_pack_lowers_the_shipped_ladder`, which reads the PACK rather than
the file so it fails if the manifest, either registry, or lowering breaks.

▢ **what is left is the consumption, and it has a layering constraint the sketch
above missed.** `BrainBuildContext` is the right carrier for the CATALOG path,
which lives in `ambition_characters`. It cannot serve the other site:
`enemy_default_brain` is in the MONOLITH, and the prepared pack lives in
`game/ambition_content`, which is above the monolith — so the monolith cannot
read the pack and the ladder cannot be fetched where it is needed.
⭐ **so the ladder has to arrive as a RESOURCE the game inserts**, lowered from
the pack by `AmbitionContentPlugin` and read at brain construction. That is the
same shape every other migrated family uses to cross this boundary, and it is
also why the two sites cannot simply share a function argument.
⚠ it is config, not state: a rollback restores a brain, never the ladder it was
built from, so this resource wants no rollback registration — see the
`PROVENANCE_ONLY` argument for the same reasoning about write-once data.

**The call chain, MEASURED 2026-08-07 rather than estimated.** It is wider than
"two call sites" — `for_level` has two, but the functions carrying them are
reached from three system entry points:

```text
FighterBrainProfile::for_level
├── character_catalog/resolver.rs:154   brain_from_preset  (ambition_characters)
│     └── carrier: BrainBuildContext  ← already threaded, per above
└── brain_builders.rs:86               enemy_default_brain (monolith)
      ├── prepared_match.rs:606  realize_seat  ← activate_the_prepared_match (a system)
      └── brain_builders.rs:173  …_brain_and_action_set
            ├── autonomous_reconcile.rs:99
            └── mount/mod.rs:567   (dismounted rider)
```

So the monolith side is **three** functions to widen and **three** systems to give
the resource to, not one. ⚠ each of those systems gains a param, and two of them
are already close to Bevy's 16-param ceiling — the reason both backends in this
repo bundle `SystemParam` structs. Budget a bundle, not a parameter.
⛔ **and do not fix only the match path.** Seating a fighter and reconciling one
into hostility would then read different ladders, which is the fork this repo's
first rule exists to prevent — `enemy_default_brain` must take the ladder, so
every path through it is the same path.

### ✔ RESOLVED — the placement blocker, and what it actually was

This section used to say boss profiles and the enemy roster were BLOCKED on a
crate-placement decision, deferred to the §4 carve. **That was wrong, and the
correction is the useful part.**

A schema must be registered by the crate owning its type, and the validator has
to link that crate. Both families' types lived in
`ambition_platformer2d_actor_monolith` — 708 crates and a renderer, against the
validator's 242 — so migrating them looked like it would triple the tool.

⭐ **But the coupling was LOCATIONAL, not real.** Nothing in either vocabulary
needed the actor crate: every field resolves against
`ambition_platformer2d_core`, `ambition_entity_catalog`, `ambition_characters`
and `ambition_combat`. Moved, `cargo check` passed first try, unchanged. **The
validator went 242 → 252 crates and still links no renderer** — the families
that were "blocked on tripling the tool" cost it ten crates, because the tool was
never the problem. The lesson generalises: *measure whether the owner crate's
weight is a dependency of the TYPE or an accident of where the type sits.*

Three consequences worth knowing before the next move:

* **The orphan rule bills you at the seam.** `impl BossBehaviorProfile { … }`
  and `impl CharacterArchetypeSpec { … }` could not follow their types, because
  they return actor-crate shapes (`BossCatalog`, `ActorTuning`). Both became
  extension traits in the actor crate — `BossBehaviorProfileExt`,
  `CharacterArchetypeSpecExt`. Call sites keep their syntax and gain a `use`.
* **A move can need a move under it.** `PickupKind` had to go DOWN to
  `ambition_entity_catalog` first (`BossRewardProfile` names it, and its old home
  `ambition_interaction` depends on `ambition_characters` — a cycle);
  `CharacterBrainTemplate` went to `ambition_characters::brain`, which its own
  doc comment had already argued for.
* **Re-export paths lie about ownership.** `audio_registries.rs` imports
  `MusicRegistry` from the monolith; it is DEFINED in `ambition_audio`. Checking
  cost one grep and unblocked a family that looked as stuck as the others.

### ✔ RESOLVED — the THIRD limit: a schema lowered by MANY sources

This section used to say a schema may be lowered by only ONE source. `compile`
refused a second, on purpose — merge semantics is the handler's question and a
generic merge would be the compiler guessing — and Ambition's nine
boss-encounter files hit it exactly, so that family was `AuthoringOnly`:
validated, and then **parsed a second time by the runtime**.

**Closed 2026-08-04** by the contract this section asked for — *parse each
source, merge the schema's own fragments, validate the aggregate, lower once* —
as `ContentSchemaHandler::aggregate`, a trait method with a default.

⭐ **the return value IS the declaration** (`Aggregation::{Undefined, Defined}`).
The alternative was a field on `SchemaRegistration`, and then a schema could
claim it aggregates without implementing the merge, or implement it and forget
to say so — two facts that can disagree about one thing.

⛔ **it is called for ONE source too.** The tempting rule is "aggregate only at
two or more", and under it a pack declaring a single file lowers a FRAGMENT
where the runtime expects the COLLECTION: the artifact's type would depend on
how many files an author happened to write. Fine at nine, broken the day someone
deletes the eighth. ⚠ the probe for it only works because the toy's fragment
type and artifact type DIFFER — a `Vec<String>` merged into a `Vec<String>`
passes under the broken rule too.

⚠ **fragments arrive in DECLARED order** (the manifest, top to bottom), which is
the order an author can see and diff — not map order, not filesystem order.

⭐ **and it answers half of the SECOND limit below**: aggregation is the first
point at which a schema can see all of its own sources, so a cross-source
invariant within one family now has somewhere to live. "Two of these nine files
claim one encounter id" is refused there. Cross-SCHEMA invariants still are not
expressible; that is the part of the second limit that stands.

`CompileStage::Aggregation` sits between facet validation and reference
resolution, so a refusal there reports what never ran.

### ◐ A second limit: a handler sees ONE facet — of ONE schema

`compile` calls `handler.check(&facet, …)` per source, and there is no whole-pack
hook. So a handler can express "this reference resolves" (the compiler's
reference-resolution stage does that across sources) but NOT a cross-source
invariant like *exact cover* — "every attack the roster plays is claimed by
exactly one seed". The one-source half of that check moved into the
`boss_seed_library` schema; the other half is still a test. Worth knowing before
promising that a migration dissolves a given check.

⭐ **half of this fell to the aggregation contract above (2026-08-04):** a
schema's `aggregate` sees every one of ITS OWN sources at once, which is where
"two encounter files claim one id" now lives. What remains is genuinely
cross-SCHEMA — the exact-cover check spans the seed library and the roster, two
different schemas, and no hook sees both.

### ⚠ The migration checklist a family is NOT done without

Three review rounds found the same defect shape three times: the principal table
was validated and lowered, the family was called migrated, and runtime-significant
state AROUND the table stayed outside validation or the fingerprint. Ask all four
before marking a row ✅:

1. **Is any ORDER significant?** The fingerprint sorts definitions by content id,
   so anything positional is invisible unless the position is IN the canonical
   form. Caught: music track order (radio next/prev), item slot index (the grid
   is positional — the index binds a row to its `Item` variant).
2. **Is any REGISTRY-LEVEL field outside the rows?** Per-row `define` calls miss
   it entirely. Caught: `default_track`, `sample_rate`.
3. **Does any field REFERENCE another family already in the pack?** If both sides
   are in, it belongs in reference resolution, not in a startup validator.
   Caught: boss↔encounter (both ways), encounter→music.
4. **Does the runtime REFUSE anything the compiler accepts?** A validator that
   says yes where the game says no has moved the failure to the worst place.
   Caught: archetype inheritance cycles, blank brain keys.

⚠ and check the canonical form's own containers: a `HashMap` under derived
`Debug` randomises per instance, so it is not canonical at all.

⛔ **and the second authority is `content_validation.rs`, not `include_str!`.**
The 698-line cross-content validator still runs at app startup
(`game/ambition_app/src/app/resources.rs:182`) over LDtk room links, dialogue
ids, quest conditions, encounter/boss ids and music refs. That is *exactly*
what the compiler's reference-resolution stage is built to own. Migrating a
family is only half a row; the other half is the cross-reference it lets
`content_validation.rs` stop doing. A family that moves without shrinking that
file has added a third reader, not removed a second.

### ▢ P2. Slice 2 — the multi-context participant flow

**P2a — `ActiveInputContext` becomes per-participant.** The B-gap above. This is
the single highest-payoff row in Program B because it is a *contradiction between
two shipped authorities*, not a missing feature, and every later context (select,
pause, dialogue, inventory, cutscene, debug) inherits the wrong answer until it
moves.

⚠ **the primary's global resource cannot simply be deleted** — it has readers
that legitimately want "the shell's answer". Give it an owner (the same shape as
`MatchParticipantRoster::published_by`) rather than two meanings.

**P2b — the real contexts exist as first-class ids**, not route-specific
branches: `select`, `pause`, `dialogue`, `inventory`, `cutscene`, `debug` beside
today's `shell.startup_acknowledge` / `shell.launcher` / `gameplay`.

**P2c — bindings are ONE authority** for routing, remapping, glyphs, touch
affordances, prompts and help. `action_label()` and `ActiveUiCues` are the two
halves that exist; they must read the same binding the router reads.

**P2d — the flow:** launcher → character select → match → pause → return, with
**two local participants** joining independently, navigating, selecting, playing,
and pausing **without stealing each other's input**. Smash's select screen is the
consumer and it already exists — this proves the seam on a real screen.

**Guard:** a `participant_contexts` module in `app_it`; the two-seat
no-input-stealing case probed red.

### ▢ P3-blocker. The semantic action layer — designed, not started

P3 asks a capability to contribute a semantic ACTION. It cannot today, and the
reason is precise: `Platformer2dInputActionMonolith` is a closed `#[derive(Actionlike)]` enum in
`ambition_input`, behind the `input` feature. leafwing needs a concrete
`Actionlike` type, so a capability cannot add a variant without editing the
engine — the exact "one central closed enum" the content compiler was built to
avoid, one layer over.

**Two shapes, and the cheap one is not enough.**

* *Bind a capability action to an existing `Platformer2dInputActionMonolith` slot.* Works today
  with no engine change, and is genuinely useful (a `Grapple` bound to
  `Utility`). But it cannot express an action the device vocabulary lacks, so it
  does not close the row — it just makes the row's absence tolerable.
* *Make the action identity open.* `Actionlike` requires
  `Debug + Clone + Eq + Hash + Send + Sync + Reflect + TypePath + 'static` plus
  `input_control_kind()`. **An interned newtype can satisfy all of them**, so
  `InputMap<SemanticAction>` is possible and a capability registers actions
  freely. `input_control_kind` comes from the registration rather than a `match`.

**The migration is the risk, not the design.** `Platformer2dInputActionMonolith` has hundreds of
call sites; a half-migration would leave two action vocabularies, which is worse
than one closed one. So the order matters:

1. `SemanticActionId` + a registry (id, owning capability, control kind, doc,
   default binding) — additive, nothing migrates.
2. `Platformer2dInputActionMonolith`'s variants become REGISTERED entries in that registry rather
   than the vocabulary itself, with the enum kept as the engine's own constants
   so existing call sites read unchanged.
3. `InputMap<SemanticAction>` replaces `InputMap<Platformer2dInputActionMonolith>` at the seam;
   `SeatBindings` already projects from whatever map the router reads, so it
   follows for free.
4. Only then can a capability add one — and P3's test is that it does so
   without editing `ambition_input`.

⚠ **do not start this while the two-participant flow is unfinished.** The review
was explicit about not expanding defensively, and an action vocabulary with one
speculative consumer is exactly that. The row exists so that when a capability
genuinely needs an action, the design is not re-derived from scratch.

### ▢ P3. Slice 3 — a capability-owned schema, action, and causal fact

One capability contributes the full set: behavior + authored schema + semantic
action + rollback registration + schedule participation + causal facts —
**without editing a central character enum or actor monolith**. That last clause
is the test.

**Guard:** the capability lives outside `crates/ambition_platformer2d_actor_monolith`, its schema is
authored from content, and `explain_tick` reports its fact.

---

## Shared contracts these three must not duplicate

* **Prepared identities.** The compiler assigns the stable resolved identity;
  actions, characters, moves, roles, capabilities, causal facts and diagnostics
  all quote it. The inspector DISPLAYS those identities — it never reconstructs a
  name from runtime internals.
* **Determinism.** Anything authoritative registers rollback state, participates
  in deterministic scheduling, appears in the schema fingerprint, resets on
  lifecycle replacement, runs headless. ⚠ this repo has been bitten repeatedly by
  the opposite: a derive's memo, a change tick, an unordered reader. Instrumentation
  is observer-only or it is simulation.
* **Cost.** Retained causal history is bounded and the expensive domains are
  feature-gated. CI must be able to run the inspector without rendering.

## What is deliberately NOT in scope yet

* A capability dependency/conflict solver (deferred on the same evidence as the
  API campaign deferred it — no consumer blocked).
* Full multi-experience asset virtualization.
* Every rebind UI and every device backend.
* Migrating every content family. One pack, end to end, first.

## Sentinel consumers — used progressively, not as a gate

1. `game/ambition_content` (content-heavy pack) — P1's migration target.
2. `fixtures/minimal_game` (minimal external platformer).
3. Smash select → gameplay, two participants — P2's consumer.
4. Smash fighter decision + stock lifecycle — P1d's and P3's consumer.
5. A traversal-focused actor (Sanic or Mary-O) — proves the inspector tolerates
   missing combat domains.
6. One custom movement/mechanic capability — P3.

**The program is not complete until the API has survived several genuinely
different consumers.** Do not declare it on the strength of one.

## Measurements to record as we go

Engine files opened · compile iterations · validation latency · internal imports
attempted · missing documentation · unresolved diagnostics · context consumed ·
whether the result is mechanically distinct. These guide refinement; they are not
a reason to stop product work for universal proofs.
