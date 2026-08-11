# The character-template architecture, and deleting the enemy-archetype system

**JON'S DECISION AND JON'S BRIEF, 2026-08-10.** This answers queue **D48** —
*"is an enemy a CHARACTER, or an ARCHETYPE wearing one?"* — and answers it
larger than it was asked. The brief below is **his, reproduced verbatim** after
this header; ⛔ do not edit it, summarise it in place, or let a later reading
replace it. Everything above the rule is orientation added by the agent.

---

## The answer in one line

> **A character is a reusable authored template, not a singleton person.**
> `spawn Goblin` three times and `spawn Fretjaw` twice are the same engine
> operation: one `CharacterDefinition`, many runtime actors.

⇒ **the D48 fork is resolved as (a), and then goes past it.** (a) said *"an
enemy IS a character, and the brain is an override"*. Jon's answer keeps that
and adds the part the fork did not reach: there should be **one** character
authority. Not `CharacterCatalog` for half the facts and
`PreparedCharacterDefinition` for the other half, and certainly not
`ArchetypeSpec` for a third set selected through a field called `brain`.

⭐ **the load-bearing correction to the earlier scoping**: D48 framed this as a
content question about 93 authored spawns. It is that, but the spawns are the
*cheap* half. The expensive half is that `character_archetypes.ron` is a second
actor-definition system reached through `EnemySpawn.brain`, and it decides
health, movement, mass, abilities, death traits, mount class, respawn and
hostility — none of which is AI.

## Why the answer is bigger than the question

Jon's own emphasis, which belongs at the top rather than buried in the brief:

> **`character_archetypes.ron` is not merely an enemy tuning file to be
> renamed.** The fact that `respawn`, `attacks_player`, body capabilities, AI
> strategy, and mount semantics all coexist in one row is precisely the evidence
> that it should be decomposed and then deleted.

And the reason to do it now rather than later:

> The current tree is unusually favorable for doing this now because the generic
> ECS runtime underneath it has already been unified. The expensive part is the
> content/construction migration; the simulation core largely does not need to be
> reinvented.

## Baseline and standing state

```text
baseline commit   2fbda948e7461425b96f7bbf701328458201ea3f
```

⚠ **inspect live HEAD before changing anything.** If newer commits exist,
preserve their semantics and adapt.

Facts measured in this repo that the brief depends on, so a resumed session does
not re-derive them:

* **93 `EnemySpawn`s** across all four worlds (`intro` 16, `sandbox` 49,
  `mary_o` 24, `sanic_speedway` 4). **28** already author a `character` id;
  **65** do not, splitting **41** whose NAME is a catalog character and **24**
  whose name is a role (`Skirmisher`, sandbags, skitters, strikers, brutes,
  goblins, `Target`). Recount with a JSON walk of
  `game/ambition_map_assets/**/*.ldtk`; ⛔ `assets/` is not where the worlds live.
* **Two spawn paths, two behaviour authorities.** `NpcSpawn` →
  `features/npcs.rs` → `resolve_initial_brain` reads the catalog row's
  `default_brain`. `EnemySpawn` → `features/ecs/brain_builders.rs` →
  `enemy_default_brain(&ActorConfig)` reads `ArchetypeSpec`, and **never
  consults the catalog row at all**. That asymmetry is the whole bug behind
  Iron Mary's fireballs.
* **D56 has landed** (`2d327f455`): the renderer resolves art through the
  spawn's character id, then art identity, then display name. So authoring
  character ids no longer un-arts the spawns it is meant to fix — the deadlock
  that made D48 unlandable is gone in both directions.

## Phase status — UPDATE THIS AS PHASES LAND

The brief's own sequence, tracked. ⛔ this list is the resumption point after a
compact; a phase is `✔` only when its deletions have happened, not when its new
path works beside the old one.

| # | Phase | State |
|---|---|---|
| 1 | Establish final domain types (`CharacterId`, definition, prepared, registry, controller-profile identity) | ◐ **the TYPES ARE NAMED AND THE EXPRESSIVENESS HALF IS DONE.** ✔ `CharacterId` (entity_catalog, serde-transparent) · ✔ `BrainProfileRef` vs `BrainPresetId` (authored reference vs resolved key) · ✔ `CharacterDeathTraits` extracted below the runtime component · ✔ knockback weight and a default autonomous profile authorable and adopted. ▢ THE TYPE MOVE: `definition.rs` is down to ONE coupling (`build_actor_moveset`) and it is a design call, see "phase 1 progress" · ▢ `WornCharacter` → universal `CharacterIdentity`, blocked on the persona derive still resolving through the CATALOG |
| 2 | Migrate authored character data out of `character_archetypes.ron` | ▢ **mapped, and DELIBERATELY NOT STARTED** — appendix C reorders it after the constructor. `BUILDABLE_ONLY_CAST` is short-lived scaffolding, not architecture. Otherwise as mapped; the DOOR is open — see APPENDIX B. `BUILDABLE_ONLY_CAST` splits "can build" from "offers on the select grid", so a migrated character can be registered without becoming a portrait. Empty today; start with the mites |
| 3 | Unify character construction (`PreparedCharacterDefinition` + `CharacterSpawnPlan`) | ◐ **`CharacterSpawnPlan` EXISTS and the authored enemy lowers through it** (`spawn/character_spawn_plan.rs`) — it owns the character question and the placement context; `plan.definition(registry)` is the ONE place construction asks which character a body is. ▢ `controller` and the profile override are NOT on it yet: no current caller has either, and they arrive with the NPC and match paths. ▢ the encounter/programmatic paths still pass an empty registry; ▢ `PreparedMatch` still builds through `CharacterRoster` and is the appendix-D proving ground. Earlier: **the authored enemy reads its character from the PLACEMENT** — `adopt_character_intrinsics`, guarded end-to-end by `mod authored_enemy_reads_its_character`. ⛔ appendix C: that method is a PROBE SEAM; the next step is `CharacterSpawnPlan` (appendix E), not more fields through it. ▢ the programmatic and encounter-mob paths still pass an empty registry; ▢ `PreparedMatch` still builds through `CharacterRoster` and is the appendix-D proving ground |
| 4 | Migrate the 93 authored placements, encounters, summons | ▢ |
| 5 | Controller/provocation simplification; rollback becomes controller-only | ▢ |
| 6 | Remove legacy runtime projections (`ActorTuning`, `CharacterBrainSpec`, `sprite_character_id`) | ▢ |
| 7 | Remove legacy authored infrastructure (`ArchetypeSpec`, `CharacterRoster`, fragments, schema) | ▢ |
| 8 | Rename and document the final architecture | ▢ |

⚠ **the deletion target is the acceptance signal.** ~2,437 lines are obvious
legacy (`ArchetypeSpec` 319, roster/enemies module 1,198,
`character_archetypes.ron` 845, `enemy_roster.rs` 75), with `ActorTuning` (275)
and much of `autonomous_reconcile` (1,045) on top. A result of *+4000 new /
−2400 old* means the old model was wrapped rather than removed.

## ⛔⛔ READ APPENDICES C AND D BEFORE RESUMING

A **course correction** (appendix C, fourteen rulings) and the **Smash
addendum** (appendix D) landed at HEAD `735a1eafc575` and outrank the phase
table and the progress notes wherever they disagree. Two sentences carry most
of it:

> `adopt_character_intrinsics` is a **probe seam, not the final model** — the
> identity/domain seam and the common constructor come BEFORE group A's content
> migration, not after.

> **Smash is this campaign's proving ground, not a competing row.** Remove
> `PreparedMatch`'s `CharacterRoster` / `ArchetypeSpec` dependency and watch how
> much of `smash_fighter_kit()` stops being necessary.

⚠ **the phase table above is the resumption point.** These appendices say what
"done" means and which order to work in; they are not a task list.

## ⇥ Evidence for PHASE 5, found in the wild (2026-08-10)

The brief says provocation *"reconstructs the actor as another mechanical
creature"* and lists the machinery to delete. ⭐ **queue D74 found the same
machinery running on POSSESSION, with a measured consequence**, which is a
sharper witness than the provocation case because nobody was even provoking:

`brain_command::apply_catalog_mode` → `autonomous_reconcile::peaceful_config`
rebuilds a possessed body's `tuning` wholesale — `max_health: 1`, NPC patrol
speeds, default `CombatCapabilities`, the kit's action set, and **`is_aerial`
read off the CATALOG's `body_kind`**. It keys on `config.sprite_character_id`,
so which row it finds depends on what that id resolves to.

⛔ **I attributed queue D74's airborne PCA to this path and that attribution is
UNVERIFIED** — `apply_brain_commands` returns early for a player-driven body
(`brain_command.rs:255`), which a possessed one is, so the rebuild may not be
reached by that case at all. The struck claim stays in D74 with the correction.

⚠ **what stands on its own is the machinery, not that symptom.** A control
transfer that DOES reach `apply_catalog_mode` re-decides `max_health`, patrol
speeds, capabilities, the action set and `is_aerial` from the catalog row rather
than restoring what the body was — which is the brief's *"reconstructs the actor
as another mechanical creature"* with a file and a line. That is worth phase 5
whether or not it explains the PCA.

## ⇥ Phase 1 progress (agent, keep this current)

* ✔ **the field-ownership ledger** — appendix A, all 49 `ArchetypeSpec` fields
  classified against consumers, with seven judgement calls written up.
* ✔ **the multi-instance invariant is PINNED** —
  `one_character_definition_seats_two_independent_fighters`
  (`character_runtime/prepared_match/tests.rs`). A mirror match seats two bodies
  of one definition: same identity, different entity, seat, position and health
  pool. Falsified two ways — damaging both bodies reds the independence half,
  seating a second character reds the identity half.
* ✔ **the uniqueness audit the brief asks for is CLEAN.** Nothing in the
  workspace maps a character id to exactly one entity: the `String → Entity`
  maps that exist are keyed by sim id, encounter id or block name, and every
  `duplicate` guard in `ambition_characters` is about DEFINITIONS
  (`duplicate_character_ids_fail_with_stable_provider_names`,
  `duplicate_display_names_are_rejected_deterministically`), which is the
  correct place for one. ⇒ **instancing is not blocked by an existing
  assumption**, and `MatchSeat`'s own doc already anticipated the collision:
  *"the worn character id collides in a mirror match."*
* ✔ **death traits are authorable on a character** — `CharacterDefinition
  ::combat_capabilities`, carried through preparation and applied by the persona
  derive (`apply_worn_character_gameplay`), which is the ONE writer both a worn
  player and a seated fighter go through. Absence retracts, on the same rule as
  health, mass and the feel marker — see the retraction trap below, which is
  where the first attempt went wrong.
* ⛔⛔ **RETRACT BY RESETTING, NEVER BY REMOVING — cost sixteen integration
  tests, and it is a rule for every field this campaign moves.**
  `CombatCapabilities` is a REQUIRED member of `ActorClusterQueryData`, so
  `try_remove` took each seated fighter out of the actor cluster query entirely
  and it stopped being simulated as an actor. The symptom named nothing about
  components: *"player one swung twelve times in range and the other fighter is
  still on 52/52 HP."* ⇒ **an absent component is a different statement from a
  default one**, and for anything a body's construction owns, only the second is
  legal. ⚠ the reset is also conditional on the PREVIOUS persona having claimed
  the field, because `ActorClusterSeed::into_components` spawns every clustered
  actor with archetype capabilities — an unconditional reset would strip an
  exploding mite the moment anything wore a character on it.
* ✔ **knockback weight is authorable on a character** — `Vitals
  ::knockback_weight` → `PhysicalBaseline` → the seed's `CombatTuning.weight`
  at construction and the live component at a re-wear. It could be stated only
  on a roster ARCHETYPE before, so all three Smash fighters seat through
  `combatant` and weighed the same; they now spread 0.85 / 1.0 / 1.35 around the
  reference body, which is what makes D75's growth term mean something different
  per fighter.
  ⛔ **the first version of that test could not fail.** Its control asserted the
  unauthored character keeps its archetype's weight — but the fixture's
  archetype authored none, so it defaulted to the reference `1.0`, which is
  exactly what an unconditional `unwrap_or(1.0)` writes. Poisoning it passed.
  The fixture now authors a `1.4` archetype, which separates *"kept"* from
  *"overwritten with the ambient default"*, and the poison reds.
* ⭐ **the field that made this worth doing first**: `CombatCapabilities` had
  exactly ONE producer in the workspace — `ArchetypeSpecExt` — so a mite that
  splits on death could say so as an archetype and no registered character could
  say it at all. That is the incompleteness the brief describes, in its smallest
  reproducible form.
* ✔ **the default-autonomous-profile SEAM exists** —
  `resolve_initial_brain(catalog, id, authored_override, definition_default,
  ctx)`. Precedence, tested three ways: an authored placement override beats a
  definition's default beats the catalog row's. It lands on
  `BrainBinding::default_preset`, whose doc already says a `restore_default`
  rebuilds from that preset, so **no new `AutonomousSource` variant and no
  rollback shape change** — `CatalogDefault` still means *"the character's
  default"*, only who may state it widened. Qualified through
  `qualify_preset_like`, so a definition and a placement cannot mean different
  things by the same word.
  ✔ **and the NPC spawn path ADOPTS it** — `CharacterDefinition
  ::default_brain_profile` carries through preparation, and the registry now
  reaches `resolve_npc_brain` through `ActorConstructionContext::with_prepared`
  → `ActorPlacementContext` → `spawn_interactable_into` →
  `NpcActorSpawnPlan::peaceful`. Three tests on that path: the definition's
  profile beats the row's, a silent definition (and an empty registry) leaves
  the row in charge, and a placement override beats both. Poisoning the lookup
  reds the first and leaves the parity cases green, which is the shape a
  precedence test should have.
  ✔ **and both remaining SUPPLIERS are wired** — the room-transition loader and
  the session reset each take the registry as an `Option<Res<..>>` and call
  `.with_prepared`, so a rebuilt or re-entered room resolves an NPC's brain the
  same way a first-time staging does. ⇒ **every route that lowers an NPC now
  asks the character first.** ⚠ the transition's resource sits BESIDE its
  `construction_services` tuple rather than inside it: that tuple is already at
  seven and is read positionally (`construction_services.6`), so an eighth
  member would be one more number for a reader to decode.
* ⚠ **`a_definition_carries_no_controller_binding` is where the brief's ruling
  lands.** That test destructures `CharacterDefinition` exhaustively and says
  *"if `default_brain` is ever added, this stops compiling and the reviewer has
  to justify it against §4.7."* Jon has now justified it — a definition MAY name
  a default autonomous profile. ⇒ when phase 1 adds that field, rewrite this
  test's prose rather than deleting it; it is the structural guard that keeps
  the CURRENT controller off the definition, which is still the rule.
* ⭐ **the catalog fold is FOUR FIELDS, not a pervasive dependency** — measured
  in `definition.rs`'s finalization. `PreparedCharacterDefinition` consults the
  catalog for exactly `max_health`, `motion_model`, `movement_tuning`, and the
  kit (`action_set` / `moveset`). Everything else already carries through from
  the definition. ⇒ the brief's *"still finalized by consulting the catalog"* is
  a much smaller cut than it reads, and it closes the moment those four are
  authored on definitions — which is phase 2's output, not extra work.
* ⛔ **THE PHASE-2 ORDERING CONSTRAINT, in the code's own words.**
  `PLAYABLE_ROSTER` cannot stop gating buildability until definitions carry the
  archetype's intrinsic facts. `character_catalog.rs` records the measurement
  from when someone tried: registering every catalog row flipped ~100
  exploration NPCs off their archetype-built vitals onto defaults, caught by
  `a_player_death_reset_survives_the_rollback_window`, *"because the catalog row
  has no mass or health to fold back in — those come from the ARCHETYPE — so the
  blanket rule cannot be made behaviour-neutral, only narrower."* ⇒ **removing
  the workaround is the LAST step of phase 2, not the first**, and it becomes
  free once the facts have moved.
* ✔ **the default profile is TYPED** — `BrainPresetId`, not `String`, on the
  definition, the overrides, the prepared definition and the resolver parameter.
  ⚠ it is an AUTHORED LOCAL name wearing the id type, not an already-qualified
  catalog key; the resolver qualifies it exactly as it qualifies an authored
  placement override, and both docs say so.
* ✔✔ **THE INVERSION IS FIXED ON THE AUTHORED-ENEMY PATH** — appendix C's
  sharpest point. `spawn_enemy_with_faction_into` asked
  `config.sprite_character_id`, which `presentation_identity` →
  `id_for_authored_identity` produces WITH A DISPLAY-NAME FALLBACK. It now asks
  `authored.payload.gameplay_character_id()`, which has no fallback at all.
  `EnemySpawnSpec::art_identity` is renamed `presentation_identity` and its doc
  says what it may not answer; the *"what it LOOKS LIKE"* sentence is deleted.
  ⭐ **`None` is the honest answer and is left visible**: a placement that has
  not named a character falls back to its archetype, which is the transitional
  state phase 4 removes and which must not be papered over by a name match.
* ✔✔ **THE MISSING PHASE-3 INSTRUMENT NOW EXISTS** — `mod
  authored_enemy_reads_its_character` in `features/ecs/spawn/tests.rs` builds a
  real authored `EnemySpawn` against a populated `PreparedCharacterRegistry`
  and reads the health off the spawned body. It is the first harness in the tree
  that constructs an authored enemy with a character registered, and every
  later phase-3 field lands through it.
  ⭐ **its second test is the poison for the inversion.** A spawn named
  `"Busy Beaver"` authoring NO character id must keep its archetype's 3 HP even
  though the beaver character authors 9 — and it asserts
  `sprite_character_id == "npc_busy_beaver"` FIRST, so the gameplay assertion
  cannot pass merely because the name never resolved. Re-wiring the caller back
  through the sprite id reds it with `left: 9, right: 3`, verified by doing it.
  ⚠ the roster fixture gives `combatant` a DIFFERENT pool (42) from
  `medium_striker` (3), because `spec_for_brain` silently answers `combatant`
  for an unknown key and equal pools would hide a lookup that never landed.
* ⛔⛔ **`WornCharacter` IS NOT AN INERT TAG — attaching it ENROLLS a body in
  the persona derive, and that is the real cost of generalizing it.** Measured
  before starting the rename, because the rename is 59 files and the risk is
  not in the rename.
  - the render layer is **not** the obstacle I expected. `ensure_player_visual
    _sprite`'s `Without<WornCharacter>` looks like a class discriminator, but
    both it and `bind_worn_character_presentation` also require `PlayerVisual`,
    which an authored enemy does not carry. ⇒ **giving an enemy the identity
    does not reroute its presentation.** The discriminator is `PlayerVisual`.
  - the obstacle is `apply_worn_character_gameplay`. Its query is
    `Ref<WornCharacter>` plus `&mut ActionSet, &mut ActorMoveset, &mut
    IdentityKit, Ref<BodyAbilities>, &mut MotionModel` — so the moment an enemy
    wears a character, that system claims it and re-derives its action set,
    moveset, health, mass and knockback weight **through the CATALOG**. That is
    the target architecture arriving early, before phase 2 has moved the facts
    onto definitions, and it is the same failure the blanket-registration
    measurement already recorded — seen from the other side.
  - ⚠ **and it would adopt only SOME enemies, silently.** `ActorMoveset` is
    inserted conditionally (`if let Some(moveset)`) on the enemy and NPC paths,
    while `IdentityKit` arrives automatically via `WornCharacter`'s `#[require]`.
    A body with no authored moveset therefore fails the query and drops out with
    no diagnostic — a partial adoption that looks like a complete one.
  ⇒ **the identity component and the persona derive must be separated, or the
  derive must be made complete, BEFORE enemies can wear a character.** That is
  the same "one character-body constructor" the correction asks for, reached
  from the identity end, which is evidence the two items are one item.
* ⛔ **PREPARED COMPLETENESS (ruling 8) CANNOT LAND FOR DEATH TRAITS YET, and
  the reason is worth writing down before someone tries.** The ruling is right:
  `None` surviving preparation encodes *"ask the old body definition"*, which is
  the overlay ontology hiding inside the new type, and "no special death
  behaviour" is an ordinary resolved value rather than an absence.
  ⚠ **but flipping it to `CharacterDeathTraits::default()` today makes an
  exploding mite stop exploding.** `adopt_character_intrinsics` only overwrites
  the seed's capabilities when the definition SAYS something; a definition that
  always says something would reset every authored-enemy body to the default the
  moment its placement names a character — and the archetype is still where the
  mites' traits live. ⇒ **completeness for this field is gated on phase 2
  moving those traits**, not on anyone deciding to be stricter. The persona path
  is already correct (it retracts by resetting, conditional on the previous
  persona having claimed the field).
* ⭐ **`CharacterDeathTraits`'s FIVE FIELDS, inspected individually** — a
  reviewer asked for this before the type is declared final, and two of the five
  do not fit the name.
  - `explodes_on_death` / `divides_on_death` / `charge_crash_explodes` — clean
    on-death consequences, one consumer each in `damage::actor_hit`.
  - `never_dies` — **a MORTALITY policy, not an on-death consequence.** Its
    consumer is `damage_apply`, which decides whether a hit kills at all, so it
    sits one step BEFORE the other three. Left grouped: same kind of authored
    fact, same consumer family, and one misfit does not justify a second type.
    ▢ split it the moment a second mortality knob appears.
  - `drops_held_item: Option<HeldItemSpec>` — ⛔ **states WHICH item where it
    should state WHETHER.** It is populated from `ArchetypeSpecExt
    ::held_item_spec()`, i.e. the character's INTRINSIC weapon snapshotted at
    construction, so a body that swapped weapons at runtime drops the one it was
    born with. ⭐ **the witness is the code's own stated intent**:
    `ambition_combat::held_items`'s module doc says *"future item drops can read
    the same component without adding archetype-specific Rust branches"* — the
    live `HeldItem` component exists for exactly this and the drop path never
    adopted it. ⇒ target shape is a `bool` policy plus the live component at the
    drop site. ▢ NOT done: `damage::actor_hit` has no access to `HeldItem`, so
    it is a combat query change rather than a data change, and it touches D72's
    territory.
* ✔ **`CharacterId` IS TYPED** — `ambition_entity_catalog::CharacterId`,
  `#[serde(transparent)]` so authored world data encodes exactly as the bare
  string it always was. ⭐ **its home was decided by the dependency graph, not
  by taste**: `ambition_platformer2d_world` (which owns `EnemySpawnSpec`) does
  NOT depend on `ambition_characters`, and adding that edge fails the contracts
  job. Both crates already depend on `ambition_entity_catalog`, which is also
  where the placement schemas and `MovesetContract` live — so character identity
  sits with the rest of the content vocabulary. Adopted on
  `EnemySpawnSpec::character_id`, `gameplay_character_id()`, and
  **`WornCharacter`'s inner value** — so the component the brief names as the
  candidate universal `CharacterIdentity` already speaks the final vocabulary,
  and `WornCharacter::character()` hands the id onward without going through
  text. ▢ the prepared registry's keys and `CharacterSpawnPlan` follow.
* ✔ **death traits are AUTHORED DATA now, not a runtime component on the
  definition** — `ambition_characters::actor::CharacterDeathTraits`, lowered to
  `ambition_combat::CombatCapabilities` by one `From` impl at construction.
  ⭐ appendix C caught this: `CharacterDefinition` owning
  `crate::combat::CombatCapabilities` would have closed a CYCLE the moment the
  definition moved down, because `ambition_combat` already depends on
  `ambition_characters`. **The crate boundary was the design test and it
  answered** — an authored fact that needs a runtime type to say it was
  modelled at the wrong level. ⚠ the `From` destructures both sides
  exhaustively rather than deriving, so the day either grows a field the other
  lacks it stops compiling and someone has to say which layer owns it.
* ✔ **the SILENT-DROP half of that is closed.** `apply_worn_character_gameplay`
  now takes `Option<&mut ActorMoveset>` and MINTS one when the body carries
  none, so membership in the persona derive is no longer decided by a component
  the enemy path inserts conditionally. Guarded by
  `a_worn_body_carrying_no_moveset_is_still_given_its_persona`; poisoned with
  the old skip semantics, it reds on the NAME (`left: "unset"`) rather than on
  the moveset — which is the point, because the old failure was never about
  moves.
* ⭐ **THE TYPE MOVE IS FOUR COUPLINGS, MEASURED — not a vague "someday".**
  Appendix C says begin moving `CharacterDefinition` to its proper low owner
  now. `definition.rs` (1,897 lines) reaches out of `ambition_characters`'s
  reach in exactly five places, and the death-traits split above already
  removed the one that would have been a CYCLE:
  1. ~~`crate::combat::moveset::{ATTACK_VERB, RANGED_VERB, SPECIAL_VERB,
     SMASH_VERB}`~~ — ✔ **DONE.** They live in `ambition_entity_catalog` beside
     `MovesetContract` now, re-exported from `ambition_combat::moveset` so all
     ~70 existing paths resolve unchanged, and `definition.rs` names the
     contract crate directly.
  2. `crate::combat::moveset::build_actor_moveset` — the kit fold. The one
     genuine question: either it follows the constants down, or the FOLD stays
     above and only the AUTHORED `CharacterDefinition` moves while
     `PreparedCharacterDefinition` stays. ⚠ the second is the smaller cut and
     probably the honest one — preparation resolves a kit, and resolving is a
     runtime concern.
  3. `motion_model_spec_for_character_id` (in `avatar/starting_character.rs`) —
     needs only `CharacterCatalog` + `ambition_platformer2d_core::
     MotionModelSpec`, both of which `ambition_characters` already has. ⇒ a
     pure catalog projection sitting in the wrong crate; it moves.
  4. `crate::features::Mass` — a DOC LINK only. Costs nothing.
  5. ~~`crate::combat::CombatCapabilities` on the definition~~ — **removed**,
     see the death-traits entry above. This was the blocking one.
  ⇒ **`definition.rs` now reaches into `ambition_combat` in exactly ONE place**
  — the `build_actor_moveset` call at the fold — plus two doc links that cost
  nothing. That single call is the whole remaining question, and it is a design
  call rather than a mechanical one: does the fold follow the constants down,
  or does only the AUTHORED definition move while `PreparedCharacterDefinition`
  stays above with the resolution? ⚠ **answer it before moving anything else**;
  the answer decides whether the move is one file or two crates.
* ▢ next: the rest of that separation — the derive still resolves through the
  CATALOG, so enrolling enemies before phase 2 has moved facts onto definitions
  would flip them onto catalog-derived kits. ⛔ do NOT open with the 59-file
  rename — the name is the cheap half and changing it first would make the
  risky half look done. It is already a component in `ambition_characters` holding a character
  id, already carried by non-player bodies (`sanic/badnik.rs`), and already the
  authority the renderer binds from — so the work is extending it to every spawn
  path and retiring `ActorConfig::sprite_character_id`, not inventing a type.

## What this decision also settles elsewhere

* **Jon's Iron Mary observation** — *"the instance of iron marry doesn't use her
  swordgun, she shoots fireballs"* — is this campaign's phase 4 acceptance test.
* **The goblin-lab heavy casting call** (`awaiting-maintainer-decision.md`) is
  reshaped, not answered: under this model a `large_brute` mob names a real
  reusable character definition, and borrowing another character's art is an
  explicit presentation override rather than a lie about identity.
* **`PLAYABLE_ROSTER` stops deciding which characters the engine can build.** The
  earlier blanket-registration failure (~100 exploration NPCs losing vitals) is
  re-read as *evidence the migration is incomplete*, not as a reason to keep the
  workaround.

---

# ⇥ JON'S BRIEF, VERBATIM (2026-08-10)

> Ok, after much deliberating with another agent, I have an answer to the
> question that identifies what the real - industry standard - elegant answer
> is. You should record this as a durable implementation plan in docs/planning
> in case we need to compact while implementing it. Here is the result of that
> conversation:

I dug through the construction, catalog, match, NPC, enemy, encounter, summon, provocation, rollback, mount, provider, and content-compiler paths. The direction is strongly confirmed.

Two refinements emerged from the code inspection:

* We should **not migrate `ArchetypeSpec` into `CharacterDefinition` wholesale**. It is a god-object containing three different authorities: intrinsic character/body facts, autonomous-controller policy, and placement/session policy. The refactor should split those facts and then delete the archetype system.
* We should go farther than D48. The clean endpoint is **one character-definition authority**, not `CharacterCatalog` for half the facts plus `PreparedCharacterDefinition` for another half. The current `PreparedCharacterDefinition` infrastructure is already close to the right endpoint, but it is still finalized by consulting the catalog and has a `HostCode` escape hatch.

The deletion payoff is credible. Just four obvious legacy artifacts are about **2,437 lines** (`ArchetypeSpec`, `CharacterRoster`, `character_archetypes.ron`, `enemy_roster.rs`). `ActorTuning` is another 275 lines, and the 1,045-line autonomous-reconcile module contains a large amount of provocation/archetype reconstruction that should disappear. Including roster fragments, schemas, compatibility branches, tests, and catalog-vs-definition folding, I would expect **several thousand net lines of legacy machinery to vanish**, even after adding the cleaner definitions.

Here is the implementation prompt I would give a fresh agent.

# Complete the Ambition character-template architecture and delete the legacy enemy-archetype system

You are implementing a substantial architecture refactor in Ambition.

Current repository baseline for this handoff:

```text
2fbda948e7461425b96f7bbf701328458201ea3f
```

Inspect current HEAD before changing anything; if newer commits exist, preserve their final semantics and adapt this plan to the live tree.

This is a **pre-release engine with zero external dependents**. Do not preserve legacy APIs, compatibility shims, duplicate registries, deprecated field names, or old authoring formats merely to reduce migration work.

`AGENTS.md` is explicit:

* unified actors;
* one body, one path;
* player/enemy/NPC/boss are data/controller distinctions, not separate actor ontologies;
* make the richer/general path universal and delete the alternatives;
* optimize for the clean engine architecture rather than preserving accidental current behavior.

The purpose of this task is to finish that migration for character identity and construction.

---

# Product decision

A **character is a reusable authored template**, not a singleton person.

These are the same engine operation:

```text
spawn Goblin
spawn Goblin
spawn Goblin

spawn Fretjaw
spawn Fretjaw
```

Each spawn receives the same authored character definition but creates a distinct runtime actor.

Conceptually:

```text
CharacterId::Fretjaw
    → reusable authored definition

SimId(100)
    → one runtime Fretjaw instance

SimId(205)
    → another runtime Fretjaw instance
```

The fact that Fretjaw is fictionally a particular named person does not make the character definition singleton-valued.

If the narrative needs one particular runtime Fretjaw to be “the canonical Fretjaw for this story role,” that is **instance/placement/narrative identity**, not `CharacterId`.

Likewise:

```text
Goblin
Fretjaw
Iron Mary
Puppy Slug
Exploding Mite
```

are all the same engine concept: reusable `CharacterDefinition`s.

A generic character and a named character differ in content semantics, not construction architecture.

---

# Target mental model

The professional endpoint is approximately:

```text
                  authored once
                       │
               CharacterDefinition
         ┌─────────────┼─────────────┐
         │             │             │
       body          capabilities   presentation
       vitals        actions        voice
       movement      moves          hurtboxes
       traits        equipment      metadata
         │             │             │
         └─────────────┴─────────────┘
                       │
             prepare / validate once
                       │
                       ▼
          PreparedCharacterDefinition
                       │
           ┌───────────┴───────────┐
           │                       │
   ControllerBinding           SpawnContext
 human / autonomous /       faction/disposition
 replay / policy            respawn/encounter/etc.
           │                       │
   autonomous profile?             │
           └───────────┬───────────┘
                       ▼
                 spawned actor
                CharacterId = X
                SimId = instance
```

A character definition may name a **default autonomous-controller profile** for authoring ergonomics.

That does **not** mean the controller is intrinsic identity.

The distinction is:

```text
CharacterDefinition
    may say:
    default_autonomous_profile = goblin_melee_ai

runtime actor
    may instead be driven by:
    Human
    another autonomous profile
    Replay
    RL/external policy
```

Possessing a Goblin changes who drives the Goblin. It does not change what a Goblin is.

This is the same broad model as Prefab/Blueprint/PackedScene-style general-purpose engines.

---

# Current architecture: confirmed problems

I inspected the relevant current source deeply.

There are currently **two competing actor-definition systems**.

## Newer system

`character_runtime::CharacterDefinition` and `PreparedCharacterDefinition` already own or resolve substantial character identity:

```text
presentation
body
hurtboxes
vitals
action set
moveset
motion model
movement tuning
voice
```

`PreparedCharacterDefinition` is explicitly intended to be flat, immutable, and complete.

This is the architecture to keep and finish.

## Older system

`character_archetypes.ron` → `ArchetypeSpec` → `CharacterRoster::spec_for_brain()` currently decides far more than AI.

A field called `brain` selects an archetype containing facts such as:

```text
movement physics
maximum health
run speed
mass
aerialness
surface walking
cling behavior

blink
fly
shield
dash

melee action
ranged action
held item
signature move

death explosion/division/crash traits
training-dummy behavior
knockback weight
death policy

mount class
pilot permissions
mount-death effect

projectile visual identity

brain template
patrol/chase effort
aggro radius
attack range
wall-turn behavior
Smash tactical policy
fighter level

hostility
contact damage

respawn
```

That means the current model effectively says:

```text
EnemySpawn.brain
    → what physical creature this is
    → what it can do
    → how healthy it is
    → how it moves
    → how it dies
    → how it mounts
    → how its AI thinks

EnemySpawn.character_id
    → costume/art
```

That ontology is the thing to delete.

Do not rename `ArchetypeSpec` and keep it.

Do not copy all its fields into another god-object.

Split its facts according to ownership and eliminate the competing actor definition.

---

# A particularly important confirmed defect: provocation currently changes the body

The current peaceful-NPC → hostile transition demonstrates how badly the two authorities are entangled.

A peaceful catalog NPC currently starts from a reduced generic configuration, including roughly:

```text
1 HP
peaceful/reduced tuning
reduced combat configuration
```

When provoked, code uses string heuristics over character id/name/dialogue to choose a hostile archetype.

Examples currently map names/id patterns to things such as:

```text
combatant
pirate_heavy
pirate_raider
cellular_automaton_fighter
...
```

The provocation path then overwrites:

```text
health-related configuration
movement tuning
gravity/aerial state
combat capabilities
brain configuration
action set
live brain
sprite/config read models
```

In other words:

> becoming angry currently reconstructs the actor as another mechanical creature.

Delete that architecture.

A peaceful Fretjaw is already Fretjaw.

A hostile Fretjaw is still Fretjaw.

Provocation should primarily change:

```text
disposition/aggression
+
autonomous controller selection/state
```

It should **not** change Fretjaw's intrinsic body, vitals, abilities, movement identity, or action repertoire.

This should allow substantial deletion from:

```text
features/ecs/actors/conversion.rs
features/ecs/autonomous_reconcile.rs
character_catalog/binding.rs
```

including the hostile-archetype reconstruction machinery.

---

# A second confirmed signal: blanket character registration failed because the migration is incomplete

The repository previously tried registering the whole character catalog into `PreparedCharacterRegistry`.

That caused roughly 100 exploration NPCs to lose their expected vitals/behavior.

The current comments interpret this as a reason not to register the whole cast.

For this refactor, interpret it correctly:

> It proves `PreparedCharacterDefinition` is not yet the complete actor definition because health/body/kit facts still come from the archetype path.

The desired invariant is:

```text
every declared character
    → one complete PreparedCharacterDefinition
    → constructible through every ordinary spawn path
```

The current workaround where `PLAYABLE_ROSTER` effectively determines which catalog rows become buildable definitions should disappear.

`PLAYABLE_ROSTER` may remain a UI/content decision about which characters appear in a selection screen.

It must **not** define which characters the engine is capable of constructing.

---

# One authority for character definitions

The current split between:

```text
CharacterCatalogEntry
CharacterDefinition
ArchetypeSpec
```

must end.

Today `CharacterCatalogEntry` itself already owns substantial gameplay facts:

```text
display/presentation
default brain
default action set
playable-kit source
motion model / momentum
abilities
movement tuning
max health
voice/barks
etc.
```

`CharacterDefinition` owns another overlapping set.

`ArchetypeSpec` owns yet another.

The final runtime must not consult multiple registries to answer what one character is.

## Desired source/preparation model

Use a clear pipeline such as:

```text
authored CharacterDefinition
        │
        ├── may reference reusable named profiles/documents
        │
        ▼
prepare_character(...)
        │
        ▼
PreparedCharacterDefinition
        │
        ▼
one CharacterRegistry / PreparedCharacterRegistry
```

The exact serialized source type may be called something like:

```text
CharacterDefinition
CharacterDefinitionDoc
CharacterSource
```

depending on what best fits the current compiler.

The semantic requirement is more important than the spelling:

> one authored character definition becomes one complete prepared character definition.

Do not retain a parallel gameplay `CharacterCatalog` that independently answers:

```text
health
abilities
movement
default action kit
default autonomous behavior
```

If a catalog-like projection remains useful for tooling/presentation, it must be **derived from the same character source/prepared definition** and must not be a second gameplay authority.

Prefer moving `CharacterDefinition`, `PreparedCharacterDefinition`, their stable ID, and their registry into the lowest natural character-domain crate—likely `ambition_characters` if the live crate graph permits it.

Use:

```text
.agent/index/crates/graph-resolved.json
```

from the live repository before changing dependency direction.

Do not create a new crate merely for aesthetic symmetry if the existing `ambition_characters` ownership is clean.

---

# Character identity needs a real name

The current runtime has concepts such as:

```text
WornCharacter
ActorConfig.sprite_character_id
ActorConfig.sprite_override_npc_name
```

These preserve the history where character identity meant primarily “which art is this body wearing?”

That is no longer the model.

Introduce or adopt a sensible stable template identity, for example:

```text
CharacterId
```

and a runtime component conceptually like:

```text
CharacterIdentity(CharacterId)
```

Use the existing terminology if an equivalent type already exists and genuinely has these semantics.

Requirements:

```text
CharacterId
    = reusable character-definition identity

SimId / FeatureId
    = one runtime instance

display_name
    = presentation only
```

A body should not need separate:

```text
sprite_character_id
```

once the body already knows which character definition it instantiates.

Presentation, voice, body metadata, combat geometry, etc. should derive from `CharacterIdentity`.

Remove display-name matching as an authoritative spawn/identity fallback.

Display-name lookup may remain a tooling/search convenience if useful.

---

# Do not make `CharacterDefinition` a new god-object

Move each `ArchetypeSpec` fact to the authority it actually belongs to.

Use this classification as the starting point, then verify every field's consumers before migrating it.

## Character/body intrinsic facts

These belong in the character definition or in reusable profiles referenced by the character definition and flattened during preparation:

```text
body geometry / sprite-authored body
standing/body physical dimensions
mass
movement model
movement tuning / run capability
aerial body capability
surface/crawler capability
cling behavior where intrinsic

maximum health / intrinsic vitals
knockback weight if genuinely body-owned

intrinsic abilities:
    blink
    fly
    shield
    dash
    etc.

action set
moveset/signature moves
melee/ranged capabilities
default/intrinsic equipment or loadout
contact-damage trait where genuinely body-owned

death traits:
    explode-on-death
    divide-on-death
    charge-crash behavior
    intrinsic immortality/training-dummy trait
    only where these really are properties of the character

mount capability
pilot capability
intrinsic mount-death behavior

presentation traits:
    projectile visual where it cannot instead belong directly to the projectile/action
    dream visual metadata
```

Prefer existing generic components and typed definitions over creating another monolithic “character tuning” bag.

For example, a ranged projectile's visual should ideally be carried by the ranged action/projectile specification rather than redundantly copied onto a top-level character when that is the real owner.

## Autonomous-controller policy

These do **not** define the character body.

Move them into a narrow reusable autonomous-controller / brain profile:

```text
brain template
patrol/chase effort
aggro radius
preferred attack distance
wall-turn behavior
Smash tactical policy
Smash heavy/duelist/dash-to-close preferences
fighter AI level
provocation controller policy
```

There is already substantial reusable machinery:

```text
BrainPreset
BrainPresetId
Brain
BrainBinding
ControllerBinding
```

Prefer evolving that vocabulary to inventing a parallel system.

Choose clean names.

For example, depending on the resulting code:

```text
BrainProfile
BrainProfileId
AutonomousControllerProfile
AutonomousControllerBinding
```

may be clearer than `CharacterBrainSpec`.

Do not retain the word `archetype` for autonomous-controller policy.

## Spawn/session/ruleset facts

These should not move into the character definition:

```text
respawn policy
initial faction/team
initial hostility/disposition
encounter membership
ruleset-owned death behavior
story/placement role
current controller
```

A Goblin can be:

```text
hostile room enemy
friendly NPC
human-controlled body
Smash fighter
summoned minion
training instance
```

without ceasing to be Goblin.

The existing code has already discovered this distinction accidentally:

`ActorTuning::adopting_archetype()` has to preserve `respawn` because blindly applying an archetype's respawn policy to a named NPC caused a real bug.

That is direct evidence that `respawn` belongs to placement/session lifetime, not actor identity.

Similarly, do not preserve:

```text
attacks_player
```

as a character-template fact.

Ambition has no privileged “player” engine identity.

Use factions, disposition, aggression, ruleset, and controller state.

---

# Autonomous default versus current controller

Preserve this important distinction:

```text
character identity
    ≠
current controller
```

But improve the authoring ergonomics.

A character definition may have:

```text
default_autonomous_profile: Option<BrainProfileId>
```

or an equivalent concept.

Then:

```text
spawn Goblin with no controller override
→ Goblin's normal autonomous controller

spawn Goblin with Human(...)
→ human drives the same Goblin body

spawn Goblin with brain_override = guard
→ guard policy drives the same Goblin body
```

This means:

> character owns its normal default behavior

without saying:

> the AI brain is intrinsic body identity.

Update the existing design documentation that currently states too absolutely that a character definition cannot carry a default brain.

The correct invariant is:

```text
current controller is session/runtime authority

character definition may provide
the default autonomous-controller choice
when no stronger context overrides it
```

---

# Remove `PlayableKitSource::HostCode`

The player robot's `HostCode` branch is another protagonist-era exception.

Current architecture already has the correct generic layers:

```text
character's intrinsic/base abilities
+
runtime body-owned progression/equipment/grants
+
session restrictions
=
effective abilities
```

Use those.

Every character definition—including `player_robot_v3`—should contain its intrinsic baseline:

```text
abilities
action set
moveset
movement/body identity
```

Progression, equipment, debug grants, possession, and session rules then modify the runtime body through the generic capability/inventory system.

Delete:

```text
PlayableKitSource
PreparedKit::HostCode
host-code fallback branches
special protagonist-kit finalization logic
```

Do not replace them with a differently named protagonist exception.

The final `PreparedCharacterDefinition` should contain one resolved intrinsic kit representation.

---

# Lower every spawn path through one character construction plan

Different authoring surfaces may remain different because they express different contextual concerns.

Do **not** create one enormous authoring struct containing every field any subsystem might ever need.

But they must all lower into the same actor-construction contract.

Conceptually:

```text
NpcSpawn -------------------\
HostileCharacterSpawn -------\
EncounterMobSpec -------------\
SummonSpec --------------------> CharacterSpawnPlan
MatchParticipant -------------/
Programmatic spawn ----------/
```

A conceptual `CharacterSpawnPlan` contains only real shared construction facts, such as:

```text
character_id
controller selection / autonomous-profile override
spawn transform
disposition/faction
lifecycle/respawn context
other genuinely contextual grants
```

Then one body-construction implementation:

```text
PreparedCharacterDefinition
+
CharacterSpawnPlan
→
generic actor ECS components
```

No alternate peaceful-NPC body builder.

No enemy archetype body builder.

No match-specific physical body reconstruction.

One body, one path.

---

# Refactor `EnemySpawnSpec`

Its current documentation explicitly says:

```text
brain        = what it DOES
character_id = what it LOOKS LIKE
```

Delete those semantics.

The normal authored form should be approximately:

```text
HostileCharacterSpawnSpec {
    character_id: CharacterId,
    brain_override: Option<BrainProfileId>,
    ...
}
```

The exact type name may remain `EnemySpawnSpec` if it clearly means the **placement role** “starts as a hostile enemy” rather than a distinct actor ontology.

But:

```text
character_id
```

must mean:

> which reusable character definition this actor instantiates.

Make it required for normal shipped visible actors after migrating the content.

Do not keep the display-name fallback as an authoritative compatibility path.

Rename:

```text
brain
```

to something that truthfully expresses its new semantics, likely:

```text
brain_override
controller_profile_override
```

depending on final vocabulary.

The ordinary case should need no redundant behavior field:

```text
character_id: "iron_mary"
brain_override: None
```

means:

> instantiate Iron Mary and use her normal autonomous behavior.

An unusual scene can say explicitly:

```text
character_id: "iron_mary"
brain_override: Some("berserk")
```

---

# Refactor NPC spawning

The current NPC path is already closer to the desired controller model:

```text
explicit brain override
→ otherwise character default brain
```

Keep that idea.

But NPC physical construction must use the exact same complete `PreparedCharacterDefinition` as an enemy, match fighter, summon, or possessed body.

Delete the current peaceful generic body reconstruction.

A peaceful NPC is:

```text
CharacterDefinition
+
peaceful disposition/context
+
an autonomous controller that does not initiate combat
```

not a body with arbitrarily reduced health and capability.

NPC and enemy editor schemas may remain separate conveniences.

They must lower into the same character spawn plan.

---

# Refactor provocation

Provocation is a major deletion target.

After the migration:

```text
before:
Fretjaw body + peaceful disposition/controller

provoke

after:
same Fretjaw body
same intrinsic vitals
same movement/body capabilities
same action repertoire
hostile disposition/aggression
combat autonomous controller
```

No archetype reconstruction.

No health rewrite.

No kit rewrite.

No gravity/body rewrite.

No name/dialogue string heuristics.

Delete concepts such as:

```text
HostileArchetypeId
AutonomousSource::Provoked { archetype }
project_provoked_archetype
hostile_spec_for_actor
hostile_brain_id_for_actor
ActorTuning::adopting_archetype
```

or their current equivalents.

If a character has a special controller profile when provoked, author that fact explicitly as controller-policy data.

Do not infer it from display names, dialogue ids, string prefixes, or art identity.

---

# Simplify rollback accordingly

Current rollback state contains legacy reconstruction facts because a restored actor may need to recreate the old archetype-derived body configuration.

That should become unnecessary.

`BrainBinding` currently snapshots variants including:

```text
CatalogDefault
CatalogPreset
Provoked { archetype }
Boss { ... }
```

After the refactor, a normal autonomous body should need only enough rollback-owned state to reconstruct **controller selection**, not physical character identity.

Conceptually:

```text
AutonomousControllerBinding {
    character_default
    or explicit profile override
    or boss-specific external ownership where genuinely necessary
}
```

`CharacterIdentity` itself is rollback-owned if it is runtime-mutable through transformation/re-wear.

Its snapshot value is merely the stable `CharacterId`.

Runtime body state restores through the ordinary rollback components.

Do not restore health/movement/capability state by rerunning a character/archetype constructor during rollback unless that state is explicitly defined as derived.

Remove roster/archetype access from rollback reconciliation.

Keep boss-specific logic separate where boss behavior architecture genuinely requires it; do not broaden this task into an unrelated boss rewrite.

When renaming rollback components/types, update the rollback schema deliberately rather than leaving compatibility aliases.

This is pre-release.

---

# Delete `CharacterBrain`

The placement enum:

```text
CharacterBrain::Passive
CharacterBrain::Patrol { ... }
CharacterBrain::Guard { ... }
CharacterBrain::Custom(String)
```

is another conflation.

`Custom(String)` currently means:

> look up an entire creature archetype.

Patrol/guard also mix controller selection with placement context.

Delete this representation.

Use:

```text
BrainProfileId / AutonomousControllerProfileId
```

for controller policy.

Keep contextual information such as:

```text
spawn anchor
patrol path
patrol radius
guard leash
```

in an explicit controller build context / placement context.

The current `AuthoredBrainContext` is already a useful model.

---

# Refactor encounters

Current `EncounterMobSpec` repeats the old ontology explicitly:

```text
kind      = what it DOES
character = what it LOOKS LIKE
```

Delete it.

Target something like:

```text
EncounterMobSpec {
    character_id: CharacterId,
    brain_override: Option<BrainProfileId>,
    spawn: ...,
    delay: ...,
    size_override: Option<...>, // only if genuinely needed
}
```

The character definition determines normal body geometry.

A wave-specific body-size override should be visibly an override, not a required parallel identity.

The generated/minted encounter id remains the **runtime body identity**.

Do not use it as character identity.

---

# Refactor summons and programmatic spawns

Current constructs such as:

```text
SummonedMinionParams.archetype_id
SpawnActorKind::Enemy { brain }
```

must disappear.

Programmatic character spawning should be character-first.

Prefer a generic request shaped around:

```text
character_id
controller/autonomous override
faction/disposition
spawn context
```

Do not preserve a separate runtime `Enemy` construction family if enemy-ness is only contextual data.

`SpawnActorKind::Boss` may remain temporarily if the boss encounter architecture has genuinely different construction requirements.

Do not force unrelated boss work into this campaign merely for visual uniformity.

---

# Refactor matches

The match architecture already demonstrates the right separation:

```text
MatchParticipant {
    character,
    controller,
    team,
    ...
}
```

Keep that.

But `PreparedMatch` currently constructs a hybrid:

```text
PreparedCharacterDefinition
+
old CharacterRoster/ArchetypeSpec
```

and patches character physical facts afterward.

Delete the hybrid.

A match fighter's physical/intrinsic actor state must come directly from the same `PreparedCharacterDefinition` used everywhere else.

Then attach:

```text
ControllerBinding
ruleset state
team
match-specific action override if intentionally authored
```

afterward.

A human Fretjaw and CPU Fretjaw should have the same underlying Fretjaw body definition.

The controller alone differs.

---

# Remove the old enemy-archetype database

Once every consumer is migrated, delete the system completely.

Expected deletion targets include, as applicable in current HEAD:

```text
crates/ambition_combat/src/archetype_spec.rs

game/ambition_content/assets/data/character_archetypes.ron
game/ambition_content/src/enemy_roster.rs

CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
CharacterRosterAssemblyError
spec_for_brain
movement-inheritance assembly specific to archetypes

ArchetypeSpec
ArchetypeSpecExt

old character_archetypes content schema
ARCHETYPES_SOURCE_PATH
pack.ron character_archetypes entry

provider-specific CharacterRosterFragment installers
```

Current provider fragments include areas such as:

```text
Mary-O AI Slop / Snake
Sanic Badnik
Smash
Ambition content
```

Migrate their real facts into:

```text
character definitions
brain/controller profiles
spawn context
```

and delete the fragment mechanism.

Do not leave an empty legacy registry for tests.

Do not leave a `LegacyCharacterRoster`.

Do not provide compatibility re-exports.

---

# Split or delete `ActorTuning`

`ActorTuning` currently remains another projection of the old god-profile and mixes:

```text
body movement
health
AI distances
contact behavior
hostility
respawn
death policy
aerialness
sandbag role
visual identifiers
```

Do not preserve this shape.

Move consumers to the actual authoritative generic components.

Examples:

```text
movement
→ character movement/body components

health
→ BodyHealth/vitals

body capabilities
→ AbilityBase / BodyAbilities / appropriate body traits

AI distances/efforts
→ Brain/BrainProfile configuration

respawn
→ spawn/lifecycle component

faction/hostility
→ disposition/aggression/faction

death policy
→ ruleset/body-health authority as appropriate

visual identity
→ CharacterIdentity / action/projectile presentation data
```

If a small coherent actor-runtime tuning component remains after the migration, name it for exactly what it owns.

Do not retain `ActorTuning` as a dumping ground merely because many systems already read it.

---

# Split or delete `CharacterBrainSpec`

This type contains actual AI policy mixed with duplicated body capability flags.

Examples such as:

```text
smash_heavy
smash_duelist
smash_dash_to_close
fighter_level
```

may legitimately belong to a brain/controller profile.

But flags such as:

```text
smash_can_blink
smash_can_fly
smash_can_shield
```

duplicate capabilities already represented on the body.

A brain should decide using the capabilities of the body it currently controls.

Do not author capability twice as:

```text
brain may attempt blink
body may enforce blink
```

when the controller can inspect the body's actual capability state.

Keep decision-policy facts in the controller profile.

Use body capability state as the enforce/availability authority.

---

# Mounts reinforce this model

ADR 0020 already says:

```text
mount and rider are ordinary actors
mountability/pilotability are body capabilities
controller can be rider/player/AI
```

Honor that.

Move:

```text
mount_class
pilotable_mount_classes
intrinsic mount-death behavior
```

into character/body capability definition where those facts are intrinsic.

The actual mount/rider pairing is a runtime relation/placement fact.

It must not depend on which AI brain happens to drive either body.

---

# Character source-data migration

There are currently 93 authored `EnemySpawn`s across the world files.

Measured population:

```text
93 total

28 already author character_id

65 currently do not
    41 have names which correspond to catalog characters
    24 have role/generic names which do not
```

Migrate them explicitly.

## The 41 named characters

Give them stable `CharacterId`s.

Do not continue inferring identity from display name.

Review actual behavior rather than mechanically assuming the current catalog defaults are correct.

Important examples:

### Iron Mary

Her character row already declares approximately:

```text
default brain = melee brute
action set = brute lunge
```

The sky placement currently gets shark-rider behavior because the enemy path ignores the character kit.

After migration, spawning Iron Mary should get Iron Mary's definition unless the placement explicitly overrides it.

### Burning Flying Shark

Its current catalog default is not necessarily the combat behavior supplied by its old archetype.

This is exactly the kind of row which must be **semantically migrated**, not blindly trusted.

Decide what the character's canonical normal autonomous profile is.

If a particular placement intentionally behaves differently, author an explicit override.

### Exploding / Dividing Mites

Their intrinsic death behavior currently lives in archetype rows.

Move those traits into their character definitions.

### Giant GNU

Current comments explicitly acknowledge that HP/rideability or related physical facts live in the old archetype file.

Move those facts into the character/body definition.

### Fretjaw and peaceful named characters

Do not encode peacefulness by stripping combat capabilities from the body.

Fretjaw's definition should describe what Fretjaw can do.

An ambient/NPC placement chooses peaceful disposition/autonomous policy.

The same Fretjaw definition can later be human-controlled, provoked, or spawned into combat without reconstructing a different creature.

## The remaining 24 generic/role names

Classify each one.

If it represents a real reusable visible thing, create a proper character definition:

```text
generic goblin
heavy goblin
training target
generic skirmisher
etc.
```

if that is what the content genuinely means.

If it is a fixture/debug/structural actor that does not deserve normal character authoring, give it an explicit low-level fixture/dev construction API.

Do not pollute the shipped character registry with fake definitions solely to satisfy uniformity.

If an unfinished character temporarily borrows another character's art, represent that as an explicit presentation reference/override—not by lying about its character identity.

---

# One definition may be instantiated arbitrarily many times

Add regression coverage proving this explicitly.

Example:

```text
spawn Fretjaw twice
```

Verify:

```text
both:
    CharacterIdentity == Fretjaw

different:
    SimId / FeatureId
    health state
    position
    brain state
    inventory
```

No uniqueness guard should reject it.

Character identity is template identity, not runtime entity identity.

Audit any existing lookup that assumes:

```text
CharacterId → exactly one Entity
```

and replace it with proper runtime/placement/story identity.

---

# Game-specific story metadata must not pollute the generic engine definition

The current character catalog also contains Ambition-specific authoring metadata such as Hall/gallery concerns.

While consolidating character authority, distinguish:

```text
generic reusable character definition
```

from:

```text
Ambition-game-specific editorial/gallery metadata
```

Do not put concepts such as:

```text
MainHall
Basement
```

into the generic engine character model merely because they currently share a RON row.

Game-specific metadata may remain in the game content layer keyed by `CharacterId`, provided it does **not** become a parallel gameplay-construction authority.

Presentation fields that are genuinely generic—sheet, portrait, voice, display name—can remain part of the reusable character definition.

---

# Sensible naming is part of the task

Do not leave terminology which preserves the old ontology.

At the end, there should be no concept where “archetype” means:

> a hidden second actor definition selected through a brain field.

Prefer vocabulary in these semantic families:

```text
CharacterId
CharacterDefinition
PreparedCharacterDefinition
CharacterIdentity
CharacterRegistry / PreparedCharacterRegistry

BrainProfile / AutonomousControllerProfile
BrainProfileId
AutonomousControllerBinding

ControllerBinding

CharacterSpawnPlan
SpawnContext
```

These names are guidance, not mandatory exact spellings.

Choose names after inspecting live usages.

Requirements:

* the name says what the object actually owns;
* do not use `brain` to mean body identity;
* do not use `sprite_*` to mean character identity;
* do not use `player` terminology for generic actor concepts;
* do not leave `catalog` in a name if it has become the authoritative character-definition registry;
* do not leave `archetype` in a type which now means AI policy;
* do not create “V2”, “New”, “Unified”, or “Legacy” names as the final API.

Rename production comments/documentation to describe the final invariants, not the migration history.

---

# Content compiler / authored-source cleanup

The old `character_archetypes` schema must disappear when its data has been migrated.

Update the content pack accordingly.

Delete:

```text
character_archetypes.ron
its schema registration
its pack entry
its loader/fragment plumbing
its validation path
```

Character definitions and autonomous-controller profiles must have proper content schemas and `deny_unknown_fields` where appropriate.

Do not duplicate validation through separate runtime parsers.

Preserve the repository's good rule:

```text
authored content
→ compiler/lowering
→ runtime authority
```

not:

```text
same RON parsed independently by compiler and game
```

---

# Provider architecture

Providers currently publish character-roster fragments.

Delete that interface.

A provider should publish things like:

```text
CharacterDefinition(s)
BrainProfile(s)
game-specific metadata
```

through the same generic registration/preparation seams.

A new provider character should require:

```text
author character
choose/default controller profile
register it
```

and **zero core actor-engine edits**.

That is an acceptance criterion.

---

# Preserve the good runtime engine

Do not rewrite systems which already consume generic actor components correctly.

The current architecture is favorable because most per-tick systems already operate on things such as:

```text
BodyKinematics
BodyHealth
BodyAbilities
AbilityBase
ActionSet
Moveset
Brain
CombatCapabilities
ActorDisposition
ActorAggression
ActorFaction
mount relations
```

The main refactor is:

```text
OLD authored/construction authority
        ↓
generic runtime components
```

to:

```text
CharacterDefinition
+ Controller/Profile
+ SpawnContext
        ↓
generic runtime components
```

Preserve the generic bottom half.

Delete the alternate constructors feeding it.

---

# Recommended implementation sequence

This is an end-to-end task. Use intermediate commits if useful, but do not stop with both architectures still alive.

## Phase 1 — establish final domain types

Create/move the final:

```text
CharacterId
CharacterDefinition
PreparedCharacterDefinition
character registry
autonomous controller profile identity
```

into their proper domain owner.

Extend the character source/prepared definition so it can express every intrinsic character fact currently required from the old archetype path.

Add default autonomous-controller-profile semantics.

Remove the conceptual need for catalog fallback during runtime body construction.

## Phase 2 — migrate authored character data

Move intrinsic facts out of:

```text
character_archetypes.ron
```

into character definitions.

Move autonomous-policy facts into brain/controller profiles.

Move placement/session facts out to their actual contexts.

Do this for Ambition plus provider-local demo roster fragments.

Make every normal declared character produce a complete prepared definition.

Remove the `PLAYABLE_ROSTER`-as-buildable-cast workaround.

## Phase 3 — unify character construction

Create one generic character-body construction path from:

```text
PreparedCharacterDefinition + CharacterSpawnPlan
```

Route NPC, enemy, match, encounter, summon, and programmatic character construction through it.

Delete the separate physical body/kit construction paths.

## Phase 4 — migrate authored placements

Migrate all 93 `EnemySpawn`s to explicit stable character identity.

Migrate encounter wave data.

Migrate summons/programmatic requests.

Remove display-name identity fallbacks.

## Phase 5 — simplify controller/provocation architecture

Replace old `CharacterBrain`/archetype selection with autonomous profile selection.

Make provocation a disposition/controller transition.

Delete hostile-archetype reconstruction.

Update rollback reconciliation to controller-only restoration.

## Phase 6 — remove legacy runtime projections

Delete/split:

```text
ActorTuning
CharacterBrainSpec
ActorConfig fields that mirror character/archetype identity
sprite_character_id
sprite_override_npc_name
```

where their facts now have proper owners.

Do not keep read models merely because deleting them touches many systems.

## Phase 7 — remove legacy authored infrastructure

Delete:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
character_archetypes.ron
enemy_roster.rs
old content schema
provider roster fragments
legacy exports
legacy tests
compatibility constructors
```

The old vocabulary should disappear from production code.

## Phase 8 — rename and document the final architecture

Search production code and authored content for stale concepts.

Update:

```text
AGENTS-adjacent architecture docs
character-definition design docs
ADRs whose ownership claims changed
MODULES.md
public API docs
authoring docs
```

Keep comments about current ownership/invariants.

Remove investigation chronology and obsolete migration narratives.

---

# Explicit deletion goal

There should be a large deletion payoff.

Current obvious legacy artifacts include approximately:

```text
ArchetypeSpec                             319 lines
CharacterRoster/enemies module          1198 lines
character_archetypes.ron                 845 lines
enemy_roster.rs                           75 lines
---------------------------------------------
obvious core legacy                    ~2437 lines
```

Additionally:

```text
ActorTuning                              275 lines
autonomous_reconcile                   1045 lines
BrainBinding/catalog compatibility      substantial
provider roster fragments               additional
content-schema plumbing                  additional
legacy tests                             additional
```

Not every line in those latter files should vanish, but large parts should.

A successful implementation should not result in:

```text
+4000 new abstraction lines
-2400 legacy lines
```

because that likely means the old model was wrapped instead of removed.

Several thousand **net deleted legacy lines** is a realistic target, although correctness and ownership matter more than hitting a numeric quota.

Report the actual before/after LOC and list every deleted legacy type/file.

---

# Required behavioral/architectural tests

Add focused tests proving the new ontology.

## Same definition, multiple instances

```text
spawn Fretjaw twice
```

Prove same character identity, independent runtime identities/state.

## Same character through different contexts

Spawn the same Goblin definition through:

```text
NPC placement
hostile room placement
encounter
programmatic spawn
match CPU
match human
```

Where contexts are applicable, verify the **intrinsic body facts agree**:

```text
body geometry
vitals
movement identity
intrinsic abilities
action/moveset baseline
mount capabilities
character identity
```

Only contextual/controller facts should differ.

Do not require every route in one giant test if smaller focused tests prove the invariant more clearly.

## Controller independence

Use one character with:

```text
Human
CPU
Replay or policy where practical
```

Prove changing controller does not change intrinsic character/body definition.

## Provocation

For a named peaceful character:

```text
before provoke:
    character X
    intrinsic kit K
    vitals V

after provoke:
    character X
    intrinsic kit K
    vitals V
    hostile disposition
    different autonomous controller
```

No body reconstruction.

## Iron Mary

Prove an ordinary Iron Mary enemy obtains Iron Mary's authored kit rather than the old shark-rider/archetype kit.

Also prove an explicit brain/controller override works when intentionally authored.

## Provider character

A provider-defined character that does not exist in Ambition's built-in content must construct correctly through the generic character path without a core code edit.

## Complete buildable cast

Every declared normal character intended for runtime spawning should have a complete prepared definition.

Do not define buildability via `PLAYABLE_ROSTER`.

## No authoritative display-name lookup

Authoritative spawn tests should use stable character IDs.

## Rollback

Prove:

```text
CharacterIdentity
controller selection
provocation/controller transition
```

restore correctly without `CharacterRoster` or archetype reconstruction.

---

# Required absence checks at the end

Use direct source searches—not a new policy framework—to verify the migration is complete.

Production code should no longer contain the old architecture concepts except in historical migration documents where explicitly retained:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
HostileArchetypeId
ProvokedArchetype
character_archetypes.ron
PlayableKitSource
PreparedKit::HostCode
sprite_character_id
```

`CharacterBrain` should also be gone from normal actor placement/construction unless inspection proves a genuinely unrelated meaning remains.

`ActorTuning` and `CharacterBrainSpec` should either be gone or reduced/renamed into narrowly coherent concepts; do not leave them with their current mixed authority.

Do not create an automated grep ratchet solely for these names unless an existing repository mechanism naturally owns that assertion.

---

# Validation

Follow current repository instructions.

At minimum:

```text
cargo check -p ambition_app
```

is the integration compile gate, not merely checking a leaf crate.

Run focused tests for every touched actor/construction/content domain.

Because authored schemas and `.ron` fields change, search and migrate **both Rust and authored RON**, including ignored/generated authoring files where repository instructions require filesystem search rather than Git-only search.

Run the relevant content compiler/validation tests.

Run targeted demo tests for at least:

```text
Ambition
Mary-O
Sanic
Smash
```

where their provider character definitions/rosters were migrated.

Do not spend the task running enormous unrelated suites repeatedly; use focused tests during the migration and the app integration gate at the end.

---

# Definition of done

This task is complete only when all of the following are true:

1. **CharacterDefinition is the one reusable actor-template authority.**

2. **PreparedCharacterDefinition is complete.**
   Runtime character construction does not ask another gameplay registry what the character really is.

3. **A character can be instantiated arbitrarily many times.**
   Character identity and runtime entity identity are explicitly separate.

4. **Every ordinary actor construction path is character-first.**

5. **Current controller is separate from character identity.**
   A character may provide a default autonomous profile, but human/CPU/replay/policy control does not change the body definition.

6. **Intrinsic character abilities/body/vitals/moves come from the character definition.**

7. **Respawn/faction/disposition/encounter lifecycle come from spawn/session/ruleset context.**

8. **Provocation does not morph the body.**
   It changes hostility/controller state.

9. **The old enemy-archetype authority is deleted.**

10. **`character_archetypes.ron` is deleted.**

11. **`CharacterRoster` and provider roster fragments are deleted.**

12. **No `brain` field secretly selects health/body/capabilities.**

13. **No `character_id` field means merely “costume.”**

14. **No display string is authoritative character identity.**

15. **The player robot no longer requires a HostCode character-kit exception.**

16. **Match, NPC, enemy, encounter, summon, and programmatic paths agree on what one character is.**

17. **New names describe current semantics without Legacy/New/V2 compatibility vocabulary.**

18. **The final change deletes substantially more legacy machinery than it adds adapter machinery.**

---

# Guiding question

For every field you migrate, ask:

```text
Is this a fact about
    the reusable character,
    the current controller,
    or this particular spawn/session?
```

Put it with that owner.

Then delete the old place.

The intended final experience for an engine user is extremely simple:

```text
author Goblin once
spawn Goblin anywhere
→ it is a Goblin

author Fretjaw once
spawn Fretjaw twice
→ two independent Fretjaw actors

change who controls either body
→ same character, different controller
```

Adding a new character should ultimately mean:

```text
author the character's body/capabilities/presentation
choose its normal autonomous profile
register the content
```

with **zero actor-engine code edits**.

Do not stop when the new path works beside the old one.

Finish the migration, delete the old authority, and leave the repository with one obvious way to answer:

> What is this actor?

The answer should be:

> It is an instance of this CharacterDefinition.

---

> **Jon's closing emphasis, verbatim:** One code-level point I would emphasize to
> the implementing agent in conversation, if it asks: **`character_archetypes.ron`
> is not merely an enemy tuning file to be renamed.** The fact that `respawn`,
> `attacks_player`, body capabilities, AI strategy, and mount semantics all
> coexist in one row is precisely the evidence that it should be decomposed and
> then deleted.
>
> The current tree is unusually favorable for doing this now because the generic
> ECS runtime underneath it has already been unified. The expensive part is the
> content/construction migration; the simulation core largely does not need to be
> reinvented.

---

# ⇥ APPENDIX B (agent, 2026-08-10) — the PHASE 2 migration map

⚠ **agent-added, below Jon's brief.** Phase 2 moves authored facts out of
`character_archetypes.ron` into character definitions. This is the map that
work needs, measured rather than guessed, plus the three structural facts that
decide its shape.

## ⛔ FACT 1 — the two authorities share NO ids at all

```text
archetype rows in ambition_content    24
catalog rows                         133
ids present in BOTH                    0
```

An archetype is keyed by a BRAIN name (`cellular_automaton_fighter`); a
character by a character id (`perfect_cellular_automaton`). ⇒ **there is no
derivable mapping between them.** The join exists only as authoring convention
in the world files, so phase 2 cannot be a mechanical rename — every pairing is
a decision somebody has to make once.

## ⭐ FACT 2 — the DEMOS already did this migration; Ambition's content did not

Only four archetypes author a `character_id` on their spawns, and all four
belong to provider demos:

```text
mary_o_ai_slop                     → ai_slop                        (14 spawns)
mary_o_snake                       → solid_snake                     (6)
mary_o_snakes_on_a_cartesian_plane → npc_snakes_on_a_cartesian_plane (2)
mary_o_snakes_on_a_paper_plane     → npc_snakes_on_a_paper_plane     (2)
sanic_badnik                       → sanic_badnik                    (4)
```

Every Ambition-content archetype authors `character_id: None`. ⇒ the pattern the
brief asks for is already shipped by the newer content, which is a useful
existence proof and a reason to expect the shape to work.

## FACT 3 — one character already wears FOUR different brains

`Puppy Slug` is spawned under `puppy_slug` ×10, `Guard:96` ×1,
`Patrol:lab_patrol_line` ×1, and `medium_striker` ×1. ⭐ **that is the campaign's
worked example in one row**: under the new model it is one `puppy_slug`
definition spawned four times, three of them with an explicit controller
override — and `Guard:96` / `Patrol:lab_patrol_line` are exactly the
`CharacterBrain::{Guard, Patrol}` conflation the brief says to delete, caught in
authored content rather than argued from the type.

## The map — 21 archetypes, three groups

⚠ **`✓` means the spawn's NAME is a catalog display name**, i.e. a character
already exists to migrate onto. `—` means it is a role name.

### Group A — clean 1:1, migrate first (9 archetypes, 36 spawns)

| archetype | character | spawns |
|---|---|---:|
| `puppy_slug` | Puppy Slug | 10 |
| `burning_flying_shark` | Burning Flying Shark | 7 |
| `pirate_shark_rider` | Pirate Raider | 6 |
| `exploding_mite` | Exploding Mite | 5 |
| `dividing_mite` | Dividing Mite | 3 |
| `sky_parrot` | Stochastic Parrot | 2 |
| `giant_gnu` | Giant GNU | 1 |
| `pirate_heavy_shark_rider` | Iron Mary | 1 |
| `ai_slop` | Ai Slop | 1 |

⭐ **start here, and start with the MITES**: their `explodes_on_death` /
`divides_on_death` are already expressible on a definition (landed 2026-08-10),
so they are the only group whose facts have somewhere to go today.

#### ⛔⛔ CORRECTION (same day, before starting): GROUP A IS BLOCKED ON PHASE 3

Read the mite rows to begin, and the migration does not terminate. Their facts
split three ways and only one third has both a home and a reader:

```text
exploding_mite / dividing_mite
  intrinsic, EXPRESSIBLE   max_health · run_speed (MovementTuning::max_run_speed)
                           melee Swipe · move_style · explodes/divides_on_death
  intrinsic, NO HOME YET   contact_strength · damage_amount (body contact damage)
  CONTROLLER               patrol_effort · chase_effort · aggro_radius
                           attack_range · brain_template · smash_hit_band
  PLACEMENT                respawn: OnRoomReenter
```

⚠ **and the deeper problem is the CONSUMER, not the home.** A mite only ever
appears as an `EnemySpawn`, and that path builds its body from
`ActorClusterSeed::new_in` → `spec.combat_capabilities()` — **the archetype**.
An enemy body carries no `WornCharacter`, so the persona derive never runs on
it. ⇒ authoring a mite's death traits on its definition today would state them
in a place nothing on its own spawn path reads, and deleting them from the
archetype would simply turn them off.

### ⇥ UNBLOCKED the same day — the authored enemy path now reads its character

✔ `spawn_enemy_with_faction_into` resolves the spawn's art identity against the
prepared registry and calls `ActorClusterSeed::adopt_character_intrinsics`, so
an authored `EnemySpawn` whose character is REGISTERED takes that character's
health, knockback weight and death traits over its archetype's. Both callers
that matter reach it: the construction executor (`construct_authored_enemy`)
and the giant host. ⇒ **group A can proceed** — author the mite's facts on its
definition, add it to `BUILDABLE_ONLY_CAST`, delete them from the archetype.

⚠ two paths still pass an empty registry and are named at their call sites: the
programmatic `spawn_staged_actor_into` (a runtime-minted body, no registry in
scope) and the encounter mob. Neither is group A.

⇒ the paragraph below stands as the reasoning that FOUND this, not as the
current state:

⇒ ⭐⭐ **phase 2 cannot complete before phase 3 for any enemy-spawned character.**
The order in the brief is right as a sequence of AUTHORITY changes and
misleading as a sequence of work: what unblocks group A is phase 3's single
construction path (`PreparedCharacterDefinition` + `CharacterSpawnPlan`), because
that is what makes an enemy body read its character at all.

⇒ **the tractable phase-2 work that does NOT wait on phase 3** is the part with
a live reader today: the NPC/catalog path (which already consults definitions —
that is why the ~100-NPC regression was measurable) and the demo providers,
which have already migrated. ⛔ do not open group A with a dual statement in two
authorities hoping phase 3 arrives; that is exactly the *"works beside the old
one"* state the brief forbids stopping at.

### Group B — one archetype, MANY characters (the ontology problem, stated)

`medium_striker`, 9 spawns: **Lab Raider ✓, Puppy Slug ✓**, and seven role
names — `under_town_skitter`, `medium striker`, `annex_goblin_a/b`,
`pg_goblin_a/b/c`. ⇒ one behaviour rented by two real characters and seven
unnamed things. `gradient_seeker` is the same shape at smaller scale
(Salvage Guard ✓ + `gradient seeker` —).

⛔ **this group cannot be migrated by moving facts.** Its archetype is a
CONTROLLER PROFILE that several characters share, which is precisely the split
the brief prescribes: the profile survives as a `BrainProfile`, and each spawn
names its own character.

### Group C — pure roles, no character exists (5 archetypes, 12 spawns)

`ranged_skirmisher` (4 · "Skirmisher"), `sandbag_finite` (3),
`sandbag_infinite` (2), `large_brute` (2), `small_skitter` (2).

⇒ the brief's own instruction applies literally: *"if it is a fixture/debug/
structural actor that does not deserve normal character authoring, give it an
explicit low-level fixture/dev construction API. Do not pollute the shipped
character registry with fake definitions solely to satisfy uniformity."* The
sandbags are the clearest case; `large_brute` is the goblin-heavy casting call
already sitting with Jon.

⚠ **`combatant` and the other three archetypes with no spawns at all**
(`large_colossus`, `pirate_heavy`, `player_robot`, `small_lurker`,
`giant_gnu_hands`, `cellular_automaton_fighter`) are reached by CODE rather than
by an authored placement — a fixture default, a boss part, a duel. Count them
separately; a spawn census does not see them.

---

# ⇥ APPENDIX A (agent, 2026-08-10) — the field-ownership ledger

⚠ **added by the agent, below Jon's brief and outside it.** His brief says
*"use this classification as the starting point, then verify every field's
consumers before migrating it."* This is that verification, banked so phase 2
does not re-derive it.

⛔ **the reference counts are a NAME-BASED UPPER BOUND**, not consumer counts. A
grep for `\.melee` matches every unrelated `.melee` in the workspace, which is
why `melee` reads 105 refs in 40 files. Use the count to rank the work, never to
claim a field is nearly unused — ⭐ but the SMALL numbers are trustworthy in the
direction that matters: `mount_death_splash` at 1 really is one site.

`ArchetypeSpec` has **49 fields**. Their owners under the new model:

## Pure assembly machinery — deleted, migrates nowhere (2)

| field | note |
|---|---|
| `inherits` | archetype-to-archetype inheritance; character definitions reference reusable profiles instead |
| `movement_resolved` | `#[serde(skip)]`, filled by the roster's inheritance pass — it exists only because the roster exists |

## Character/body intrinsic (26)

`movement` · `max_health` · `run_speed` · `mass` · `surface_walker` ·
`cling_breaks_on_hit` · `is_aerial` · `explodes_on_death` · `divides_on_death` ·
`charge_crash_explodes` · `weight` · `mount_class` ·
`pilotable_mount_classes` · `mount_death_splash` · `default_size` · `melee` ·
`ranged` · `held_item` · `can_blink` · `can_fly` · `can_shield` · `can_dash` ·
`body_contact_damage` · `contact_strength` · `signature_move` · `move_style`

## Autonomous-controller policy (11)

`patrol_effort` · `chase_effort` · `aggro_radius` · `attack_range` ·
`attack_cooldown_mult` · `turns_at_walls` · `brain_template` · `fighter_level` ·
`smash_hit_band` · `smash_heavy` · `smash_dash_to_close` · `smash_duelist`
(and `provoke_forced_brute_min_aggro`, below)

⭐ **`turns_at_walls` is not in Jon's starting list and its own doc already
classifies it**: *"this is control policy consumed by Patrol/Wanderer brains, not
movement/collision policy."* The field had the answer written on it.

## Spawn / session / ruleset (2)

`respawn` · `attacks_player` — both named in the brief. `attacks_player` is
**deleted**, not moved: there is no privileged player identity.

## ⭐ THE SEVEN JUDGEMENT CALLS — read these before phase 2

These are the fields where the three-way split does not answer itself. Each one
is a decision the migration must make deliberately, with what is known:

1. **`is_aerial`** — a live TWO-SOURCE CONFLICT, already documented on the field,
   ⛔ **and I cited queue D74 as it breaking in the wild — WRONGLY, retracted
   2026-08-10.** The probe measured `gravity_scale = 1.0` on the registered PCA,
   so nothing floated it; that symptom is a movement divergence during
   possession and is not an instance of this conflict. The conflict itself is
   still real and still unresolved — two authorities state one body's
   aerial-ness — but it has no witness yet, which is exactly what it had before.
   `new_peaceful_npc_in` reads the catalog's `body_kind: Floating`; the hostile
   `EnemySpawn` path reads this. **The Perfect Cellular Automaton is `Floating`
   in its catalog row and played grounded by the shipped duel.** Unifying the two
   authorities forces that disagreement to resolve, and resolving it changes how
   a shipped fight plays. ⛔ do not fold it silently — this is exactly the class
   of thing the brief means by *"semantically migrated, not blindly trusted"*.
   `Option<bool>` must survive the move: `None` ≠ `Some(false)` is why the
   conflict is expressible at all.
2. **`is_sandbag`** — reads as a character fact and behaves as a placement role.
   It reaches the RENDER read model (`ActorRenderView.is_sandbag`, a
   sprite-upgrade fallback), `save_sync`, and cluster pathing. A sandbag is a
   training instance of some body, which argues placement; but three consumers
   treat it as identity. Decide once, and move all three.
3. **`never_dies`** — same shape, cleaner answer: `damage_apply` uses it to make
   a body take no health damage. That is either an intrinsic trait (an immortal
   creature) or a training-mode ruleset fact. The brief allows the intrinsic
   reading *"only where these really are properties of the character"* — the
   shipped users are sandbags, which suggests it travels with `is_sandbag`.
4. **`death_policy`** — the brief puts ruleset-owned death behaviour in session
   context, and `HpDepleted` vs `Unbounded` is precisely a ruleset fact (Ambition
   has health, a platform fighter has stocks and a blast zone). ⚠ but it is
   authored per-body today and a mixed roster is expressible. Recommend: ruleset
   owns the default, a character may not override it.
5. **`provoke_forced_brute_min_aggro`** — provocation controller policy, which the
   brief wants authored explicitly rather than inferred. It is a *controller
   profile selected on a transition*, not a number on the body; the cleanest form
   is a named provoked-profile reference, and this f32 becomes a field of that
   profile.
6. **`ranged_visual`** — the brief's own hint applies: *"a ranged projectile's
   visual should ideally be carried by the ranged action/projectile specification
   rather than redundantly copied onto a top-level character."* Move it into
   `RangedActionSpec`, do not carry it on the character.
7. **`dream_seed`** — presentation metadata for the psychedelic shader pass.
   Generic-enough to ride the character definition (like sheet/portrait/voice),
   but check whether it is Ambition-specific editorial metadata first; the brief
   forbids Hall/gallery-class concepts entering the generic model.

## The capability-authored-twice set

`can_blink` / `can_fly` / `can_shield` / `can_dash` are body capabilities, and
`CharacterBrainSpec` carries `smash_can_blink` / `smash_can_fly` /
`smash_can_shield` alongside them. ⇒ **the duplication the brief calls out is
real and is exactly these three pairs.** The controller reads the body's
capability state; only the DECISION flags (`smash_heavy`, `smash_duelist`,
`smash_dash_to_close`, `fighter_level`) stay on the profile.

---

# ⇥ APPENDIX C — THE COURSE CORRECTION (relayed 2026-08-10 at HEAD `735a1eafc575`)

⛔ **this outranks my own progress notes wherever they disagree.** Condensed to
its RULINGS on 2026-08-10 at a reviewer's instruction — *"the planning file
should preserve the authoritative target, field-ownership decisions, migration
census, current phase and acceptance tests; it should not become the archive of
every conversation that produced those conclusions."* The full relayed text of
this appendix and of appendix D is in git history at `5f5cbd1bb` and
`e67468819`; nothing below is a paraphrase of a decision, only of its argument.

## The thesis

`adopt_character_intrinsics` is a **probe seam, not the final model**. It proved
character-owned data can outrank the archetype before the entity exists. Keep
the regression coverage; do NOT migrate dozens of fields by growing it — that
moves the god-object's precedence logic into a patch function.

Today: `EnemySpawn.brain → ArchetypeSpec → build almost the whole actor →
resolve sprite id → patch three fields`. Target: `character id → prepared
definition → build the actor`, with the controller chosen separately.

## The rulings

1. **There is NO archetype→character map, and there must never be one.** 24
   archetype rows, 133 catalog characters, 0 shared ids — expected, because
   `medium_striker` is a BEHAVIOUR identity and `fretjaw` is a CHARACTER
   identity. The migration census is one-time working material. ⛔ no
   `ArchetypeCharacterMap` / `LegacyArchetypeToCharacter` / `CharacterForBrain`.
2. **The census is evidence, not a completeness proof.** Audit every code-only
   user of `spec_for_brain`, `CharacterBrain::Custom`, `ArchetypeSpec`,
   `CharacterRoster` — construction, matches, summons, provocation, rollback,
   giant limbs, demos, fixtures. Each ends as a `CharacterDefinition`, a brain
   profile, spawn/session policy, a fixture-only API, or deleted. **No fifth
   bucket called "legacy archetype we still need."**
3. **Gameplay identity must never come through presentation identity.**
   ✔ done for the authored enemy; `character_id` means *which definition is
   instantiated*, and the display-name fallback is presentation-only migration
   compatibility that must not survive D73.
4. **Move the domain types down NOW**, and let the compiler expose wrongly
   owned fields rather than predicting them. When an authored fact needs a
   runtime type to state it, extract the lower semantic fact. ✔ demonstrated by
   `CharacterDeathTraits`.
5. **Use typed identities from the start** — `CharacterId`, and a profile
   identity that distinguishes an authored REFERENCE from a resolved ID.
   ⛔ do not build the replacement API on `String` and promise types later.
6. **An empty `PreparedCharacterRegistry` is a test seam, not production
   semantics.** Once a route claims to spawn a character, its prepared
   definition is required. A fixture wanting a bodiless generic actor gets an
   explicit low-level API instead.
7. **Build `CharacterSpawnPlan` BEFORE migrating group A content.** Authoring
   surfaces stay distinct and lower into one semantic plan. ⛔ not a bag of
   every placement field; ⛔ do not preserve two body constructors by making
   their internals look similar.
8. **The prepared definition must be COMPLETE.** `None → ask the archetype` is
   the overlay ontology surviving inside the new type. Source data may stay
   partly authored during migration; the PREPARED representation may not.
9. **Keep the three-owner split** (character template / autonomous-controller
   profile / spawn-session context). Copying all 49 `ArchetypeSpec` fields onto
   `CharacterDefinition` is the failure mode, not the goal.
10. **Group B is the valuable one.** `medium_striker` becoming a reusable brain
    profile is success; surviving as a renamed whole-body archetype is not.
11. **Delete the scaffolding**: `BUILDABLE_ONLY_CAST`, the display-name
    fallback, `adopt_character_intrinsics`, the definition→catalog→archetype
    precedence bridges. A registered complete definition IS buildable; the
    select grid owns a separate curated roster.
12. **Remove `PreparedKit::HostCode`.** The protagonist is a definition plus
    runtime body state, like everything else.
13. **Provocation becomes a disposition + controller transition** — same
    identity, same body, same capabilities. Delete `HostileArchetypeId` and the
    name/dialogue heuristic. Rollback preserves character id + controller
    binding + disposition, never a second body-definition id.
14. **Name it finally as it lands.** ⛔ no new uses of `archetype`,
    `sprite_character_id`, `art_identity`-as-gameplay, `brain: String`,
    `player` in generic actor APIs, `catalog default` as final authority; and
    no `New` / `V2` / `Unified` / `Legacy`.

## The two questions to ask before every change

> Does this bridge make it easier to DELETE `character_archetypes.ron`, or does
> it create another permanent way to consult it?

> If `CharacterRoster` disappeared immediately after this commit, would the new
> owner have enough information to construct the same character correctly?

## The acceptance signal

⛔ **not** "an authored enemy reads three character fields", **not** "all
authored enemies have character ids". D73 is done when this describes
production:

> Every normal spawned actor is an instance of one prepared
> `CharacterDefinition`. Its current controller is a separate binding, and its
> spawn/session policy is separate context.

At which point *"which archetype is this character really based on?"* is not a
question that exists. The large deletion is the evidence, not a side effect.

---

# ⇥ APPENDIX D — THE SMASH ADDENDUM (relayed 2026-08-10)

⭐ **this is the link between the run's two top rows.** D72 and D73 are the same
work from two ends: Smash is D73's largest beneficiary and its proving ground.
Condensed with C; full text at `e67468819`.

## The requirement it adds

> Once the common character constructor exists, use `PreparedMatch` as one of
> the proving grounds. Remove its `CharacterRoster` / `ArchetypeSpec`
> dependency and see how much of the generic `smash_fighter_kit()` /
> `fighter_abilities` leveling machinery becomes unnecessary.

⇒ **this supersedes my earlier reading that the authored room enemy is phase 3's
proving ground.** The enemy is where the seam was PROVED; the match is where it
must be PAID OFF, because the match already states the target model at its API
level (`MatchParticipant { character, controller, team }`) and contradicts it
underneath — it validates CPU profiles through `CharacterRoster`, then calls
`ActorClusterSeed::new_in(.. archetypes ..)`.

## The root cause this campaign owns

Every Smash fighter is overwritten with `.with_action_set(smash_fighter_kit())`
because seven of twelve selected characters are Hall NPCs whose catalog rows say
`peaceful` — *"eight looks and one game."*

⭐ **that is a CONTROLLER fact recorded as a BODY fact**, the same error as
`EnemySpawn.brain` deciding health, pointing the other way. A peaceful Alice
means *Alice + a peaceful controller*, not *Alice has no fighting repertoire*.
Once definitions carry what a character actually knows, Smash gets PCA's moves,
Mary-O's moves, Sanic's moves — instead of one generic kit under eight sprites.
`arena_duelist_long` / `arena_duelist_close` already author their own vitals,
movesets and hurtboxes, and are the model.

## What each layer owns, after D73

* **Character** — body/geometry, weight and vitals baseline, intrinsic
  movement, intrinsic capabilities, attacks/specials/moveset, hurtboxes,
  presentation.
* **Seat/controller** — human vs CPU vs replay; CPU strategy and difficulty.
  ⇒ `Sanic CPU L1 / L5 / human / replay` without four Sanics.
* **Match** — teams, stocks, blast-zone rules, respawn between stocks, global
  balance, and **capability RESTRICTIONS**.
* **Smash presentation** — select grid, HUD, stage framing, match UI.

⚠ **rules may still constrain a character, and that is not the same as
replacing its kit.** The model is `character intrinsic capability ∩ match
capability policy`, NOT `Smash replaces abilities with a generic fighter set`.
`roster.fighter_abilities = Some(..)` was repairing a construction divergence
between seats; once every seat instantiates the same definition through the
same path, that rationale is gone.

## Two things that become free

* **Mirror matches.** `CharacterId ≠ SimId` engine-wide, so `Fretjaw vs
  Fretjaw` stops being something match code is careful about.
* **Character select.** *Registered complete definition ⇒ constructible*, so
  the grid answers one content question instead of coordinating catalog rows,
  playable rosters, buildable rosters, archetypes and sprite identities.

## The end-to-end demo

> The same Fretjaw definition works in the Hall, in a normal hostile encounter,
> under possession, and in Smash, with only controller and contextual rules
> changing.

---

# ⇥ APPENDIX E (agent, 2026-08-10) — the `CharacterSpawnPlan` shape, DERIVED

Appendix C's ruling 7 says build the common construction contract before
migrating group A content, and warns: *"do not make this one giant bag of every
possible placement field"* and *"do not preserve two physical-body constructors
merely by making their internals look similar."* This appendix derives the
shape from what the two existing plans actually hold, so the next session
implements rather than invents.

## ⭐ THE LOAD-BEARING DISTINCTION: this is an UPSTREAM layer, not a merge

`EnemyActorSpawnPlan` and `NpcActorSpawnPlan` (both in
`features/ecs/spawn_actors.rs`) are **already-lowered** plans. By the time
either exists, the brain is BUILT, the action set is RESOLVED, the combat kit is
COMPUTED and the `ActorClusterSeed` is CONSTRUCTED FROM THE ARCHETYPE. Merging
them would produce one struct that still asks the archetype what the body is —
appendix C's *"archetype-built creature + character overlay"* with fewer types.

`CharacterSpawnPlan` sits ABOVE both. It is what a placement lowers TO and what
construction reads FROM, before any of the above has been decided.

```text
NpcSpawn / EnemySpawn / EncounterMobSpec / SummonSpec / MatchParticipant
        ↓                       (each authoring surface stays distinct)
CharacterSpawnPlan  { character, controller, context }
        +
PreparedCharacterDefinition
        ↓                       (ONE character-body construction)
generic runtime ECS components
```

## What the two current plans hold, classified

Measured, not recalled — `spawn_actors.rs:323` and `:498`.

**Shared by both (9)** — `entity_name`, `feature_id`, `feature_name`,
`feature_aabb`, the `ActorClusterSeed`, `brain`, `action_set`, `combat_kit`,
`aggression`.

**Enemy only (3)** — `faction`, `held_item`, `moveset`.

**NPC only (3)** — `render_size`, `interactable`, `brain_binding`.

⇒ **the overlap is not evidence of a shared plan; it is evidence of a shared
CONSTRUCTOR.** Nine of twelve fields agree because both paths build the same
kind of body, and every one of the nine is an OUTPUT of resolution rather than
an authored input. The three-and-three that differ are the genuine contextual
additions the correction says to keep out of the common plan.

## The proposed shape

```text
CharacterSpawnPlan {
    character: CharacterId,                        // which template is instantiated
    controller: ControllerBinding,                 // human seat / autonomous / replay
    autonomous_profile_override: Option<BrainProfileId>,
    context: SpawnContext,                         // where, whose, under what rules
}
```

* **`character`** — from `EnemySpawnSpec::gameplay_character_id()` (landed),
  `NpcSpawn.character_id`, `MatchParticipant.character`, `EncounterMobSpec`'s
  migrated field. ⛔ never from a display name and never from a sprite id.
* **`controller`** — the axis Smash already states correctly
  (appendix D §5): *"Character = Sanic, Controller = Cpu { brain_profile }"*.
  A placement that says nothing is autonomous.
* **`autonomous_profile_override`** — the placement's `brain_override`. Its
  precedence against the definition's default is ALREADY IMPLEMENTED in
  `resolve_initial_brain`; this field is just where the override travels.
* **`context`** — `SpawnContext` carries what the placement decided that is not
  the character: the spawn `aabb`, the feature identity (`feature_id` /
  `feature_name` / `entity_name`), faction, disposition/aggression, respawn
  policy, encounter membership, patrol paths. ⚠ **this is the field that will
  rot into the giant bag** if it is allowed to accept anything; the rule that
  keeps it honest is that every member must be a decision the PLACEMENT made,
  never a fact the CHARACTER states.

## Where the three current outputs go

* `brain` / `brain_binding` — DERIVED inside construction from `controller` +
  `autonomous_profile_override` + the definition's default. Not plan fields.
* `action_set` / `moveset` / `combat_kit` / `held_item` — DERIVED from the
  prepared definition. Not plan fields. These are exactly the facts phase 2
  moves off the archetype, which is why the plan cannot be finished before
  phase 2 starts and phase 2 cannot be finished before the plan exists — they
  interleave per character rather than sequencing.
* `render_size` / `interactable` — presentation and interaction attachments,
  contextual additions on the NPC path.

## The order to build it in

1. `SpawnContext` first, from the enemy path only, carrying today's contextual
   fields verbatim. It is the piece with no ambiguity.
2. `CharacterSpawnPlan` around it, with `character` populated from the accessor
   that already exists and `controller` a two-variant enum (human seat /
   autonomous) — resist widening it before a third caller needs it.
3. Route the authored enemy through it, keeping `EnemyActorSpawnPlan` as the
   lowered result. ⭐ the harness for this already exists:
   `mod authored_enemy_reads_its_character`.
4. Then the NPC path, then encounter/programmatic/summon, then `PreparedMatch`
   — which appendix D names as the proving ground and which is where the
   `CharacterRoster` dependency finally comes out.
