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

## ⇥ THE TWO FAILURE MODES ARE NOW COMPILER-ENFORCED (2026-08-13)

Jon named two. The first — *"do not migrate `ArchetypeSpec` into
`CharacterDefinition` wholesale; it holds THREE authorities and they must
separate"* — was guarded only by this document, and a document cannot fail.
Three exhaustive destructures now hold it, one per struct:

```text
  ArchetypeSpec        49 fields   machinery 2 · body 29 · controller 13 · placement 5
  ActorTuning          19 fields   body 13 · controller 0 · placement 4 · (see ⭐ below)
  CharacterDefinition  27 fields   identity 7 · body 15 · default-controller 3 · presentation 2
```

⇒ **adding a field to any of the three stops the crate COMPILING until somebody
files it under an authority.** Removing one does the same. There is no count to
edit and no census to redo — which matters, because this run has now corrected
stale counts in four ledger rows, six campaign rows, a decision index and this
file's own phase table.

⭐ **the first failure mode now requires an explicit lie rather than an
omission.** Carrying `aggro_radius` onto the body would not quietly widen a
struct; it would refuse to build until a controller fact was filed as a body one.

⭐⭐ **AND THE FIRST DELETION CAME OUT OF THE GUARD ITSELF.** Filing
`ActorTuning`'s fields under headings is what made its CONTROLLER column visible
as three entries — and then measurable: two (`patrol_speed`, `chase_speed`) were
already `BrainProfile`'s normalized efforts resolved against a body, a projection
rather than a second authority, and the third (`attack_cooldown_mult`) moved to
`BrainProfile` on 2026-08-13. **That column is now empty.**

⚠ **the migration was four edits** — add the field with a serde default, write it
at the archetype→profile seam, stop writing it at the archetype→tuning seam, read
it from `config.brain_profile`. 42 `BrainProfile` literals exist and none needed
touching, because they spread `..Default::default()`. Neither shipped archetype
row authors the value, so it is behaviour-neutral on today's content. ⇒ **that is
what a fact having one home buys, and it is the argument for doing the rest of
the split the same way.**

⚠ **the guards do not judge, they demand a decision** — and two are marked as
arguable rather than settled: `ArchetypeSpec::smash_heavy` (a weight class is a
body fact and `weight` already is one — check for a duplicate before migrating
it), and `CharacterDefinition`'s three default-controller fields, which are the
campaign's own design (*"one adopter does not earn the indirection"*) and are
watched for one thing only: **a default must stay replaceable.**

⚠ **the SECOND failure mode is not enforceable this way** — *"do not stop when
the new path works beside the old one"* is a claim about deletions, and what
holds it is a set of counting tests rather than a struct shape. **All of them
landed 2026-08-13; each replaces a number that had to be remembered with one
that checks itself:**

```text
  what_still_needs_an_archetype_row              4     the acceptance signal, exact set
  only_the_uncast_placements_..._fallback        2     item 15's deletion, exact set
  the_cast_that_still_needs_a_body_assist_...   14     item 9's deletion, CEILING
  the_cast_that_states_its_own_moves_...         8/36  P3.24's floor
  the_grid_fighters_that_state_..._moves         7/14  P3.26's floor
  the_cast_that_states_its_own_verbs_...         2     P3.25's floor (pre-existing)
```

⭐ **the exact-set ones ratchet in BOTH directions**, which a count cannot: they
fail when something new takes a dependency on the old model AND when the last
adopter leaves, and the second failure is the one that says a deletion is now
possible. ⇒ **the campaign no longer needs anybody to re-run a census to know
where it is.**

⛔⛔ **AND MOST OF THEM ARE NOT IN THE RUN'S GATE — worth saying about my own
work.** The goal file executes four things: `cargo check -p ambition_app
--all-targets`, the absence contracts, `cargo test -p ambition_app --test
app_it`, and a clean tree. So:

* ✔ **the four DESTRUCTURE guards are enforced continuously** — they are
  compile-time, and `--all-targets` compiles test targets.
* ✔ the app-level regressions are enforced (`app_it`): the forced puppy-slug
  seat, the grid-fighter split, the per-fighter frame data.
* ⛔ **the counting ratchets are NOT.** They live in `ambition_content`,
  `ambition_characters`, `ambition_platformer2d_actor_monolith` and
  `ambition_demo_mary_o` LIB suites, and nothing in the gate runs those. That
  includes `what_still_needs_an_archetype_row`, the acceptance signal's own
  countdown.

⇒ **they are correct where they are** — a ratchet belongs beside its subject —
but *"a test exists"* and *"a test runs on every turn"* are different claims and
this file should not blur them. ⚠ deliberately NOT fixed by editing the goal
file: changing the checks I am measured against, mid-run and unasked, is not
mine to do.

⚠ **two of them carry a control that says NOT to delete**, added after a
measurement changed the conclusion: the moveset ratchets no longer instruct
removing `DeclaredCombatRules::unarmed_melee`, because the fighters that state
no moves are peaceful on purpose — see *"Is `unarmed_melee` scaffolding, or is
it permanent?"* in `awaiting-maintainer-decision.md`.

## Phase status — UPDATE THIS AS PHASES LAND

The brief's own sequence, tracked. ⛔ this list is the resumption point after a
compact; a phase is `✔` only when its deletions have happened, not when its new
path works beside the old one.

| # | Phase | State |
|---|---|---|
| 1 | Establish final domain types (`CharacterId`, definition, prepared, registry, controller-profile identity) | ◐ **the TYPES ARE NAMED AND THE EXPRESSIVENESS HALF IS DONE.** ✔ `CharacterId` (entity_catalog, serde-transparent) · ✔ `BrainProfileRef` vs `BrainPresetId` (authored reference vs resolved key) · ✔ `CharacterDeathTraits` extracted below the runtime component · ✔ knockback weight and a default autonomous profile authorable and adopted. ✔ **THE TYPE MOVE IS DONE** (landed 2026-08-12, still marked ▢ here on 08-13): the authored `CharacterDefinition` lives in `ambition_characters` and `PreparedCharacterDefinition` stayed above, which is the design call this row was waiting on. ⭐ the coupling INVERTED rather than moved — `ambition_characters` does not depend on `ambition_combat` at all, and `ambition_combat` re-exports the derivation · ▢ `WornCharacter` → universal `CharacterIdentity`, blocked on the persona derive still resolving through the CATALOG |
| 2 | Migrate authored character data out of `character_archetypes.ron` | ◐ **NEARLY DONE, AND THE DISTANCE IS FOUR CASTING DECISIONS** (re-measured 2026-08-13). `character_archetypes.ron` is **263 lines and TWO rows** — `combatant`, which exists only as the row three `OPEN_CASTING` waivers borrow, and `medium_striker`, which exists only because one placement names it. ⭐ `worlds::tests::what_still_needs_an_archetype_row` rebuilds every placement's resolution against an EMPTY roster and reports exactly four things that stop resolving; **every other enemy placement in all four worlds builds as a character with no archetype table at all.** ⇒ all four are content decisions with options, evidence and a recommendation written up in `awaiting-maintainer-decision.md`, so what is left of this phase is agreement rather than migration. ⇥ AS WRITTEN: ◐ ◐ **STARTED — the first two characters are off the roster.** `npc_exploding_mite` and `npc_dividing_mite` author their death traits and health pools as CHARACTERS, their 8 sandbox placements name them, and the two `*_on_death` lines are DELETED from `character_archetypes.ron`. The diff is negative in the legacy file for the first time. ▢ their controller facts (patrol/chase/aggro/attack_range/brain_template) still need a `BrainProfile` type before those rows can go entirely. Formerly: mapped, and deliberately not started — appendix C reorders it after the constructor. `BUILDABLE_ONLY_CAST` is short-lived scaffolding, not architecture. Otherwise as mapped; the DOOR is open — see APPENDIX B. `BUILDABLE_ONLY_CAST` splits "can build" from "offers on the select grid", so a migrated character can be registered without becoming a portrait. Empty today; start with the mites |
| 3 | Unify character construction (`PreparedCharacterDefinition` + `CharacterSpawnPlan`) | ◐ **RE-MEASURED 2026-08-13 — two of the three ▢s below were STALE.** ✔ `PreparedMatch` no longer builds through `CharacterRoster`: all three mentions left in `prepared_match.rs` are historical comments recording its removal (*"that arm is deleted; preparation no longer takes the roster at all"*), and this row carried the claim TWICE, from two eras. ✔ the ENCOUNTER path takes `Res<PreparedCharacterRegistry>` **required**, documented as *"a wave that names a migrated character builds that character rather than an archetype wearing its face"*. ▢ what actually remains is the PROGRAMMATIC pair — `damage_drops` (the mite's split) and `puppy_slug_gun` (the summoned ally) — which take `Option<Res<..>>` and fall back to an `empty_cast`. ⚠ **and that pair is NOT a defect — I filed it as one and was wrong.** `prepared.rs` states the contract deliberately: *"`PreparedCharacterRegistry` is absent rather than empty, and absent already means 'no registered characters' to every consumer"*. When the resource is missing there are no characters to resolve, so the archetype road is the only available answer — and it is instrumented, not silent: the open-casting arm warns with the identifier, the declaring provider, the borrowed row and the reason. ⇒ P0.1's conflation is about an EXPLICIT `CharacterId` missing from a PUBLISHED registry, which is a different fact from no registry at all. ▢ `controller` and the profile override are still not on the plan, and that one is deliberate and documented — no current caller has either. ⇥ AS WRITTEN, and preserved because its first half is still right: **`CharacterSpawnPlan` EXISTS and DISTINGUISHES a missing registration from an unmigrated placement** (`Result<Option<..>, &CharacterId>`) — an authored character that is not prepared is a fault, warned today and a hard error once phase 4 makes the field required. Earlier: **it EXISTS and the authored enemy lowers through it** (`spawn/character_spawn_plan.rs`) — it owns the character question and the placement context; `plan.definition(registry)` is the ONE place construction asks which character a body is. ▢ `controller` and the profile override are NOT on it yet: no current caller has either, and they arrive with the NPC and match paths. ▢ the encounter/programmatic paths still pass an empty registry; ▢ `PreparedMatch` still builds through `CharacterRoster` and is the appendix-D proving ground. Earlier: **the authored enemy reads its character from the PLACEMENT** — `adopt_character_intrinsics`, guarded end-to-end by `mod authored_enemy_reads_its_character`. ⛔ appendix C: that method is a PROBE SEAM; the next step is `CharacterSpawnPlan` (appendix E), not more fields through it. ▢ the programmatic and encounter-mob paths still pass an empty registry; ▢ `PreparedMatch` still builds through `CharacterRoster` and is the appendix-D proving ground | |
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

## ⇥⇥ REMAINING WORK — THE ONE CHECKLIST (agent, 2026-08-10)

### ⇥ WHERE THE 23 STAND — tallied 2026-08-13, so a fresh session need not count

```text
  ✔ DONE     10   1, 4, 5, 8, 10, 11, 12, 13, 17, 19
  ◐ PARTIAL   8   2, 3, 14, 15, 16, 18, 20, 21
  ▢ OPEN      5   6, 7, 9, 22, 23
```

⭐ **and the shape matters more than the tally: almost every remaining ▢ and ◐
is gated on the SAME four casting decisions.** 14, 15, 16, 20, 21, 22 all wait
on `character_archetypes.ron` losing its last two rows; 9 waits on the fourteen
body-incomplete characters; 23 is a rename that comes last by design. **6 and 7
are the only ones that are engineering and not waiting on Jon.**

⚠ **the four decisions are written up with options and a recommendation each** —
`awaiting-maintainer-decision.md`, which now indexes ELEVEN open questions
(it had been showing five while six sat un-indexed in its own body).


⛔ **this is the resumption list. Everything below the appendices is context;
this is the work.** Ordered by dependency: an item's blockers are above it. Each
carries the measurement that sizes it, so no step begins with a survey.

⚠ **status: the replacement architecture EXISTS and the deletion has BEGUN.**
Two characters are off the roster (the mites), `character_archetypes.ron` is two
lines smaller. The legacy population still standing:

```text
ArchetypeSpec                        319
features/enemies / CharacterRoster  1198
character_archetypes.ron             843
enemy_roster.rs                       75
ActorTuning                          275
autonomous_reconcile                1045
                                    ----
                                    3755
```

### A. Unblock the rest (small, ordered, no content)

1. ✔ **ANSWERED 2026-08-12, and the answer is the one this item guessed.** The
   second option: only the AUTHORED `CharacterDefinition` moves;
   `PreparedCharacterDefinition` stays above the cut. The evidence is the file
   itself rather than a preference — `derive_moveset`, the single reach into
   `ambition_combat`, is a private PREPARATION function, not a method on the
   authored type, and the authored type's only other mentions of that crate are
   two doc links. Resolving a kit is runtime work, exactly as written here.
   ⇥ as written:
   ✔ **Decide `build_actor_moveset`'s home.** `definition.rs` reaches into
   `ambition_combat` in exactly ONE place. …
   ⇥ ✔✔ **DECIDED AND MOVED (2026-08-12).** The answer written above was right
   and the code now agrees: `moveset/prefabs.rs` is
   `ambition_characters::moveset_prefabs`, and preparation calls it directly.
   ⇥ **it cost three separations, each measured rather than assumed**: the pogo
   technique's SCHEMA went down while its executor stayed
   (`ambition_characters::technique`); `MovePrefabRegistry` — the half that
   validates presentation ids through `ambition_vfx` — became its own module,
   because building a move from a spec and expanding an authored prefab key are
   different jobs and only the second needs to know what a renderer can draw; and
   a `use super::*` glob had to go first, because a bulk move cannot be planned
   against a glob.
   ⛔ **one thing travelled that should not have, and it is back**: the canonical
   robot's slash SFX overlay and its three cue constants moved because they were
   ADJACENT in the file, not because preparation calls them (it does not — the
   overlay's only production caller is the protagonist road). Returned to
   `ambition_combat::moveset::player_robot_slash`, with a 26th absence contract
   holding the line. The test is *does preparation call it*, not *was it next to
   something preparation calls*.
2. ◐ **The compiler was asked, and it named exactly one blocker — which is now
   gone** (2026-08-12). Every field on `CharacterDefinition` is an
   `ambition_characters`, `ambition_platformer2d_core` or
   `ambition_entity_catalog` type except `ranged_execution`, which was a MONOLITH
   type (`avatar::RangedExecution`) and so pinned the whole struct. It has moved
   to `ambition_characters::brain`, beside the `ActionSet` whose `ranged`/
   `special` folding it decides — all 42 sites repointed, no re-export left
   behind. ⇥ ⭐⭐ **AND THE STRUCT HAS MOVED** (same day): `CharacterDefinition`,
   `Lineage`, `Vitals` and `BodySource` are
   `ambition_characters::actor::definition` — 602 lines out of the monolith.
   ⚠ **the authored half had ZERO `crate::` references**, which is how a
   600-line move became a cut rather than a refactor, and is itself the evidence
   that item 1's answer was right: everything monolith-shaped in that file was on
   the PREPARATION side of the line.
   ⇥ ▢ the type is re-exported from `character_runtime`, where the 277 callers
   still name it. That is an import convenience over ONE definition rather than a
   second one — but the re-export is what a later slice removes, and until then
   the crate boundary is stated in only one place.
   ⇥ as written:
   ✔ **Move `CharacterDefinition` into `ambition_characters`** per (1). This is
   appendix C ruling 4, and the compiler is the instrument: let it expose every
   remaining wrongly-owned field rather than predicting them.
   ⇥ ✔✔ **AND THE PREPARATION HALF FOLLOWED IT (2026-08-12).**
   `character_runtime/definition.rs` was 2,061 lines and is **356**: the authored
   model, the whole preparation pipeline, `PreparedCharacterRegistry` and
   `CharacterRegistrationError` are `ambition_characters::prepared`. What stayed
   is what is genuinely an App's — `try_register_character`,
   `StagedCharacterOverrides`, `CharacterPreparationPlugin` and its barrier.
   ⚠ **the hard part was not the imports, it was the PRIVACY.**
   `prepare_character` and `finalize_character` were private module functions,
   and that privacy IS the finalization barrier. Publishing them to span the
   crate boundary would have put the ordering hazard the barrier exists to remove
   back on the production surface. ⇒ the barrier is a TYPE now:
   `prepared::StagedCharacter` is minted only by `prepare_for_registration` and
   consumed only by `finalize_cast` — the `Bound<N>` pattern from the binding
   boundary, where folding early is not prevented but UNSPELLABLE.
   ⚠ eighteen monolith fixtures needed the barrier-bypassing test seams from one
   crate up; they got `ambition_characters`'s `test-support` feature, enabled as
   a DEV-dependency only, so neither seam exists in a production build. No
   fixture was rewritten and no test was lost.
3. ◐ **A `BrainProfile` type — THE TYPE LANDED 2026-08-11**
   (`ambition_characters::brain::profile`). It replaced `CharacterBrainSpec`
   outright rather than joining it, and took `aggro_radius`, `attack_range` and
   `turns_at_walls` off `ActorTuning` on the way — those are decisions a DRIVER
   makes. It is authorable with `deny_unknown_fields`.

   ⇥ ⭐⭐ **ADOPTION IS DONE — 16 CHARACTERS, re-measured 2026-08-13**, and this
   item's *"no character names one yet"* is its most stale sentence. Sixteen of
   the authored cast carry a profile: **2 by NAME** (`goblin` and
   `npc_lab_raider`, both pointing at the shared `medium_striker`, which is what
   makes it a ROLE rather than one creature's private policy) and **14 INLINE**
   — the mites, both sandbags, the slug, the parrot, the shark riders, the gnu
   and its hands, the salvage guard, the PCA, the AI slop, the paper-plane swarm.

   ⚠ **inline is not a shortfall**, and the campaign says so itself: *"one
   adopter does not earn the indirection, and publishing a shared policy nobody
   shares leaves a second empty role behind exactly like the one being removed."*

   ⇒ what actually remains of this item is the OTHER half of its sentence: **the
   archetype still projects one.** That is the archetype road, gated on the
   castings — not on adoption.

   ⚠ **and the `smash_can_*` mirror this item forward-references as "item 21's
   deletion" is ALREADY DELETED** — zero mentions workspace-wide, removed
   2026-08-11 on the rule that *a capability copied onto a controller policy
   makes the policy unreusable, because the copy describes ONE body*. Item 21
   records it; this sentence still reads as though it were pending.

   ⇥ AS WRITTEN: ▢ what remains is
   ADOPTION: the archetype still projects one, and no character names one yet.
   The `smash_can_blink/fly/shield` mirror of the body's capabilities came
   across deliberately and is item 21's deletion. Originally:
   a reusable autonomous-controller profile with
   the controller-policy subset of `ArchetypeSpec` (patrol/chase effort, aggro
   radius, attack range, brain template, smash hit band, turn-at-wall). ⭐ this
   is the single biggest unblocker left: the mites' rows cannot disappear
   without it, group B is entirely it, and `CharacterSpawnPlan`'s
   `autonomous_profile_override` has no reader without it.
4. ✔ **Free preparation from the catalog — DONE 2026-08-11.** The two id spaces
   are the same one and it is now CHECKED rather than assumed:
   `app_it -- character_provider_namespace` asserts every registered
   definition's provider is a provider the catalog registry assembled under,
   carries a poison so membership can say no, and was probed RED by giving one
   registration site a made-up provider. Preparation qualifies with
   `qualify_in_provider(&definition.provider, ..)` and reads no catalog row.
   ⛔ the `test::patrol_peaceful` warning above is DISCHARGED, and the fixture
   that produced it was the fault: it parsed its catalog instead of ASSEMBLING
   it, and production namespaces every preset `provider::name`.
5. ✔ **DONE 2026-08-12 (campaign P0.3).** `MatchParticipant::character`,
   `PreparedSeat::character_id` and `StartingCharacter::character_id` are
   `CharacterId`; the prepared registry was already keyed on it. ⚠ residue
   recorded rather than pretended away: `PreparedCharacterOverrides::id`
   (module-private staging) and the AUTHORING schemas (`EnemySpawnSpec`,
   `InteractionKind::Npc`, `EncounterSpec`) are still `String` — content-facing
   surfaces that belong with the authoring pass. ⇥ as written:
   ▢ **Finish typed identity**: `PreparedSeat::character_id`, and the `&str`
   several runtime accessors return. `CharacterId` should survive from authoring
   to runtime, converting only at serialization/presentation/debug boundaries.

### B. One construction path (phase 3's remainder)

6. ▢ **Route the NPC path through `CharacterSpawnPlan`.** The second of six
   authoring surfaces, and the second is where a shared contract either proves
   general or turns out to be the enemy path renamed. It is also what returns
   `controller` and `autonomous_profile_override` to the plan WITH readers.
   ⚠ measured prerequisite: `character_id` is `Option<String>` on both
   `ambition_interaction::InteractionKind::Npc` and its `InteractionKindSpec`
   mirror, ~37 construction/match sites. Wide, mechanical, wants its own slice.

   ⇥ ⚠ **re-measured 2026-08-13: it is 29 sites, not ~37** — still wide, still
   mechanical, and the field is still `Option<String>` on the variant
   (`lib.rs:56`). ⭐ **and item 7's census makes these ONE piece of work**: the
   common constructor already serves match, enemy, summon and encounter, so the
   NPC road is the last authoring surface outside it. This is the biggest
   unblocked item in the checklist.
7. ◐ **THE CONSTRUCTOR EXISTS AND FOUR OF THE SIX ROADS USE IT** (census
   2026-08-13, by call site rather than by claim). `ActorClusterSeed::
   new_character_in` is called by the MATCH seat (`prepared_match.rs`), the
   AUTHORED ENEMY (`spawn_actors.rs:1668`), the SUMMON (`:1389`) and the
   ENCOUNTER MOB (`:2498`).

   ⇥ ✔ **and this item's second sentence is satisfied where it can be**: the
   encounter road takes `Res<PreparedCharacterRegistry>` REQUIRED, and the summon
   road resolves against the prepared cast first and falls back loudly. ⚠ the two
   PROGRAMMATIC sites (`damage_drops`, `puppy_slug_gun`) still take
   `Option<Res<..>>` — and that is the DOCUMENTED contract, not an empty-registry
   gap: *"`PreparedCharacterRegistry` is absent rather than empty, and absent
   already means 'no registered characters' to every consumer."*

   ⇥ ▢ **what is left is the NPC road, which is item 6** — so these two are one
   piece of work, not two.

   ⇥ AS WRITTEN: ▢ **The real common body constructor** —
   `PreparedCharacterDefinition` + `CharacterSpawnPlan` → one actor. Then
   encounter, programmatic and summon paths, none of which may keep passing an
   empty prepared registry.
8. ✔ **`PreparedMatch` drops `CharacterRoster` / `ArchetypeSpec`** — appendix
   D's proving ground, and the point where `smash_fighter_kit()` and
   `roster.fighter_abilities` should stop being necessary. ⭐ take this EARLY
   once (7) lands; it is the strongest evidence the architecture composes.

   ⇥ ✔ **DONE 2026-08-13 (campaign P2.18).** `prepare_match` and
   `prepare_the_match` no longer take a roster at all — it was a REQUIRED `Res`,
   so every host that prepared a match had to install an enemy archetype table to
   seat a fighter. `prepared_match.rs` now names `CharacterRoster` and
   `ArchetypeSpec` only inside ⛔ history blocks; no code reaches either.
   `seat_brain_profile` lost its archetype arm, and with it `CharacterRoster`
   stopped being a controller-policy authority — one of the three fused
   authorities, out of the type.

   ⇥ ✔ **`smash_fighter_kit()` is gone too** — grepped for the definition, not
   the name: every surviving mention is a comment about what it used to do.

   ⇥ ⛔⛔ **BUT `roster.fighter_abilities` IS NOT SCAFFOLDING, and this line's
   premise is wrong.** It was measured end to end on 2026-08-13
   (`a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines`,
   campaign P4.29/30/32): dropping `shield` from a character turns the test red,
   and adding `fly` to a character ALONE does not — the mask intersects it away.
   Both are needed for a verb to reach a live body. ⇒ that is `AbilitySet`
   INTERSECTION, and the mask is the SESSION RULESET's half of it: a stage states
   what its mode permits, and authoring a capability onto a character must not
   smuggle it in. Removing it would delete an authority Jon's own three-way split
   names, not a crutch. It stays, and item 25's "ruleset restricts, never grants"
   is the rule it implements.
9. ▢ **Delete `adopt_character_intrinsics`** once (7) replaces the precedence
   it performs. It is migration scaffolding, not a destination.

   ⇥ ⭐ **SIZED 2026-08-13, and it had no size before.** One production caller
   (`spawn_actors.rs`), whose comment says it serves *"the shrinking population
   of half-migrated characters"* without saying how many that is. It is
   **14 of 36**, pinned as a CEILING by
   `the_cast_that_still_needs_a_body_assist_only_shrinks`:

   ```text
     6  pirates          D96 item 8 — "a pirate quartermaster's vitals: six
                         REGISTERED_WITHOUT_A_BODY entries". It is exactly these six.
     1  npc_carl_stargan D96 item 5
     4  Hall NPCs        npc_alice, npc_bob, npc_noether, npc_oiler
     3  moveset authors  npc_pirate_admiral, npc_ninja_shadow_oni_leader,
                         special_patent_clerk — they state their own MOVES and
                         still not their own BODY
   ```

   ⇒ **two thirds of that population is already filed as content decisions.**

   ⛔⛔ **BUT THE POPULATION IS NOT THE CALLER COUNT, and my first sizing of this
   item confused them** (corrected 2026-08-13, same day). The enemy road reaches
   `adopt_character_intrinsics` only on the FALL-THROUGH — when a placement names
   a character whose `body_blueprint()` is `Err`. **Zero shipped placements do**:
   `what_still_needs_an_archetype_row` reports one placement resolving to no
   character at all and none resolving to an incomplete one, and the seam has no
   other production caller.

   ⇒ **the seam is already dead in shipped content** — deleting it today changes
   nothing observable.

   ⛔⛔ **AND "not blocked" DOES NOT FOLLOW FROM THAT — I wrote it and it is
   wrong** (re-corrected within the hour). The seam's job is to let a character
   that cannot build a body still CORRECT one, and **14 such characters exist**.
   Deleting it is a no-op the day it happens and a silent regression the day
   somebody places one of the 14 as an enemy: that body would take the archetype
   whole, with none of the facts its half-migrated character authored.

   ⇒ so item 9 IS gated on those 14 becoming complete, which is D96 — the
   original framing was right and my correction over-reached. ⚠ **the measurement
   was sound both times; what was wrong was the sentence after it.** The same
   thing happened to `ActorTuning`'s field count today, and both times the error
   was one inference beyond the evidence.

   ⚠ **body-incomplete is NOT unseatable.** The admiral and the oni leader are on
   the Smash grid and fight. It means *"cannot build a body from the character
   alone"* — item 9 deletes the ASSIST, not the character.

### C. Content migration (phase 2 + 4)

10. ✔ **DONE — all seven, and their rows are deleted.** Verified 2026-08-12:
    `character_archetypes.ron` contains no `puppy_slug`, `burning_flying_shark`,
    `pirate_shark_rider`, `sky_parrot`, `giant_gnu`, `pirate_heavy_shark_rider`
    or `ai_slop` row. ⭐ Iron Mary — this item's own acceptance test for Jon's
    original observation — is `npc_pirate_heavy_iron_mary`, authored and named by
    its placement. ⇥ as written:
    ▢ **Group A's remaining seven** — `puppy_slug` (10 spawns),
    `burning_flying_shark` (7), `pirate_shark_rider` (6), `sky_parrot` (2),
    `giant_gnu` (1), `pirate_heavy_shark_rider`/Iron Mary (1), `ai_slop` (1).
    Follow the mite recipe: author on the definition, register, name the
    placement, delete from the row IN THE SAME CHANGE.
    ⚠ **Iron Mary is the acceptance test for Jon's original observation.**
11. ✔ **DONE.** Both mite rows are gone, and the homeless facts this item named
    have an intrinsic home: `CharacterDefinition::contact_damage`
    (`ContactDamage { strength, amount }`), authored by every contact-damaging
    character that has migrated. ⇥ as written:
    ▢ **The mites' rows disappear** once (3) gives their controller facts a
    home. Also still homeless: `contact_strength` / `damage_amount` (body
    contact damage) — they need an intrinsic home before any contact-damaging
    character fully migrates.
12. ✔ **DONE, and by this item's own test.** `medium_striker` is a reusable
    `autonomous_profiles` entry with TWO adopters — the goblin band and
    `npc_lab_raider` — which is the success condition stated here; it did not
    survive as a renamed whole-body archetype. ⚠ its ARCHETYPE row still exists
    for unmigrated placements, and that is item 7's business, not this one's.
    ⇥ as written:
    ▢ **Group B — shared behaviour profiles.** `medium_striker` becoming a
    reusable `BrainProfile` is success; surviving as a renamed whole-body
    archetype is failure.
13. ✔ **DONE — all five classified, and all five rows are deleted.** Verified
    2026-08-13 against the shipped worlds rather than against the group's own
    description, which is what changed three of the verdicts:

    ```text
      ranged_skirmisher   REAL CHARACTER   the world casts "Skirmisher" as
                                           npc_pirate_raider — not a role at all
      sandbag_finite      REAL CHARACTERS  `sandbag` / `sandbag_infinite`, with
      sandbag_infinite                     practice_target and a policy that
                                           notices nobody (0px aggro, 0px reach)
      large_brute         SPLIT            the proving-grounds placement is cast
                                           (npc_pirate_heavy_iron_mary); the
                                           goblin encounter WAVE is still open
      small_skitter       SPLIT            the proving-grounds placement is cast
                                           (npc_puppy_slug); `SmallSkitter`, the
                                           mite's split, is still open
    ```

    ⭐ **the "no character exists" premise was wrong for four of the five.** This
    group was written as *pure roles* — and the migration resolved them by
    casting real creatures, which is the outcome the brief wanted and the
    opposite of what the group predicted. ⚠ the two SPLIT rows are not partial
    work: a placement and a code path are different populations, and both
    residues are provider-declared in `OPEN_CASTING` with the row they borrow,
    tracked as D96/D102. Nothing here is blocked on engineering.

    ⇥ as written:
    ▢ **Group C — generic roles.** Classify each: real character, fixture-only
    low-level API, or presentation borrowing. ⛔ do not force test entities into
    the character catalog for type uniformity.
14. ◐ **SIXTY-FIVE IS NOW TWO** (re-measured 2026-08-12 by walking the LDtk
    JSON, not by regex). Across all four shipped worlds every `NpcSpawn` and
    every `EnemySpawn` carries a `character_id` except **two**: `intro.ldtk`'s
    `under_town_pipes` / `under_town_skitter` and `sandbox.ldtk`'s `dive_drill`
    / "Target". Both are CONTENT DECISIONS (ledger D96 items 3 and 4), not
    migration work — a skitter is a place plus a movement style, and a thing
    called "Target" in a dive-drill room is plausibly the sandbag but that
    changes the drill. ⚠ `BossSpawn` is a separate population this item never
    counted: 11 of them carry no `character_id` field at all, and they resolve
    through `PhaseScript` boss profiles rather than the archetype road.
    ⇥ as written:
    ▢ **The 65 unmigrated placements.** ⚠ measured: `intro.ldtk` (16) and
    `sandbox.ldtk` (49) entity instances carry NO `character_id` field instance
    at all — this is "add the field to 65 entities", not "fill in a value".
    ⛔ surgical, formatting-preserving edits only (never `json.dumps`), and
    count `EntityRef`s before and after every session.
    ⛔ the file is in the `game/ambition_map_assets` SUBMODULE: a deletion here
    and an authoring there must land together or bodies silently lose facts.

    ⇥ ⛔⛔ **AND A THIRD COPY THIS ITEM NEVER COUNTED — the room GENERATORS.**
    Fixed 2026-08-13. A room is not authored by hand; it is generated from
    `tools/ambition_ldtk_tools/specs/*_area.ron`, which keeps its own copy of
    every placement's fields. The worlds were fully migrated while **eleven spec
    placements still named deleted rows and no character**, so regenerating any
    of those rooms would have written the pre-migration placement back over the
    migrated one. Each spec now mirrors the world's values, and
    `tests/test_spec_brains_resolve.py` holds the condition: a spec's `brain:`
    must name something that still resolves, or the placement must name its
    character.

    ⚠ **match by ROOM AND ENTITY NAME, never by key.** The three placements the
    spec calls `pirate_on_shark` are named *Burning Flying Shark* and the world
    casts them as `npc_burning_flying_shark` — the key would have cast them as
    the pirate raider.
15. ◐ **MEASURED AND PINNED — two placements away, and both are Jon's.**
    `worlds::tests::only_the_uncast_placements_still_ride_the_display_name_fallback`
    asserts the EXACT set of shipped `EnemySpawn`s that author no character:
    `dive_drill/EnemySpawn-6126` ("Target") and
    `under_town_pipes/EnemySpawn-104875` (the skitter) — D96 items 3 and 4, the
    same pair item 14 counted.

    ⭐ **an exact set rather than a count, so it ratchets in both directions.** A
    new unnamed placement fails it, and so does casting the last one — at which
    point this item stops being a survey and becomes a deletion, announced by a
    red test rather than by somebody re-measuring.

    ⚠ the split underneath is already right: `gameplay_character_id` has NO
    fallback by design, because a display name that happens to match a character
    is a coincidence the engine must not act on. What item 15 deletes is the ART
    road, which is tolerable precisely because a wrong sheet is visible.

    ⇥ as written:
    ▢ **Make `EnemySpawnSpec::character_id` required** and delete the
    display-name fallback from `presentation_identity`, once (14) is complete.
16. ◐ **Code-only archetype users.** `spec_for_brain`, `CharacterBrain::Custom`,
    `ArchetypeSpec`, `CharacterRoster` in construction, matches, summons,
    provocation, rollback, giant limbs, demos, fixtures. Each ends as a real
    character, a real brain profile, spawn policy, a fixture-only API, or
    deleted. ⛔ no fifth bucket.

    ⇥ **THE BUCKETS, ASSIGNED 2026-08-13 — and five of the nine roads are
    settled.** Each verdict below is the compiler's, not a grep's
    (`probe_dead_public_fns.py`, D105), after four hand-rolled censuses were
    wrong in this run alone.

    ```text
      spec_for_brain     ✔ FIXTURE-ONLY API — `#[cfg(test)]`, renamed
                           `generic_body_for_a_test_fixture`; production has no
                           equivalent because an unresolved id is an error
      matches            ✔ DELETED — `prepare_match` takes no roster; the seat
                           policy arm went with it (P2.18)
      provocation        ✔ DELETED — `hostile_brain_id_for_actor` and its twin
      rollback           ✔ DELETED — `Provoked { archetype }` is payloadless
      giant limbs        ✔ DELETED — `planned_giant_host_ids` had zero callers;
                           the limbed QUESTION stays with `is_limbed_host`
      summons            ▢ `damage_drops` → `SmallSkitter`, a casting decision
      construction       ▢ body facts, and the last road in
      demos              ▢ mary_o's one row-pair; the outlander fixture's own
      CharacterBrain::Custom  ▢ the placement vocabulary itself — outlives the
                           roster, and is not archetype-shaped
    ```

    ⇥ ⚠ **and the three dead helpers found on the way say something about the
    shape of this item**: `planned_authored_enemy_ids` still DEMANDED a
    `&CharacterRoster` it had stopped using, carrying it as `_roster` *"so
    roster/diagnostic call sites keep one authority"* — for call sites that no
    longer existed. A parameter kept for symmetry is how a roster reaches code
    with no question for it, and a census that counts TYPE MENTIONS rather than
    live reads will keep over-reporting this item's remainder.

### D. Collapse the old authorities (phases 5–8)

17. ✔ **Provocation becomes disposition + controller** — same identity, same
    body, same capabilities. Delete the name/dialogue heuristic and
    `HostileArchetypeId`; rollback preserves character id + controller binding +
    disposition, never a second body-definition id.

    ⇥ ✔ **ALL THREE NAMED DELETIONS ARE DONE, the last of them 2026-08-13.**
    The name/dialogue heuristic went arm by arm as its creatures took their own
    `provoked_profile_ref` (D84 pirates, D89 automatons), leaving
    `hostile_brain_id_for_actor()` returning the literal `"combatant"` — deleted
    in campaign P2.20, along with `provoked_projection`'s `archetype: &str`
    parameter and the test-only `hostile_spec_for_actor` twin. `HostileArchetypeId`
    is gone with `AutonomousSource::Provoked { archetype }`, which is the
    payloadless `ProvokedDefault` now and encodes as a bare tag on the wire.

    ⇥ ✔ **and the rollback clause holds, checked variant by variant.**
    `AutonomousSource` has five: `CatalogDefault`, `CatalogPreset(BrainPresetId)`,
    `ProvokedDefault`, `ProvokedProfile { profile }` and `CharacterProfile` —
    none of which names a body definition. ⚠ the sixth, `Boss { archetype:
    BossAutonomyId }`, is a different population and not a counterexample: it
    names an autonomous MODE rebuilt from the boss catalog as a `BossPattern`,
    which exists so a boss has a reconstructible mind when possession masks its
    live brain. A boss's BODY does not come from it.

    ⇥ ⚠ **what provocation still writes is a read-model, and it is derived**:
    `config.brain` comes from `config_brain_for(&brain)` exactly as every other
    road derives it, and `provocation_changes_the_mind…` pins that it may never
    be `Custom(_)`. That assertion used to say `!= Passive`, which was the
    archetype name's fingerprint — it could not have passed without the roster
    key it was supposed to be guarding against.
18. ◐ **Prepared completeness** (ruling 8): `None` must stop meaning "ask the
    archetype". ⚠ gated on (10)–(11): flipping death traits to
    `CharacterDeathTraits::default()` today makes an unmigrated mite stop
    exploding.

    ⇥ ✔ **THE GATE IS SPENT — the mites are migrated** (measured 2026-08-13).
    `npc_exploding_mite` authors `explodes_on_death` and `npc_dividing_mite`
    authors `divides_on_death`, both `with_death_traits` on their own
    definitions; four characters author them in total (the two mites, the burning
    shark, `sandbag_infinite`). There is no unmigrated mite left to stop
    exploding.

    ⇥ ✔ **AND THE RULING IS ALREADY IN FORCE WHERE IT DECIDES ANYTHING.** The
    character-first road reads `definition.death_traits.clone().unwrap_or_default()`
    — `None` means *the character said nothing* and takes the default, which is
    exactly ruling 8. The `if let Some(..)` that still reads "ask the archetype"
    is `adopt_character_intrinsics`, on the LEGACY road, and it is correct there:
    a HALF-migrated character's unstated facts have no other source, so patching
    rather than defaulting is what keeps a partial migration honest.

    ⇒ **so this item does not need a flip; it completes when the legacy road
    goes**, which is item (9) behind item (7). ⚠ and that road already carries
    **zero shipped placements**: every `EnemySpawn` in the four worlds names a
    complete character except `under_town_pipes` and `dive_drill`, and neither
    names a character at all, so `adopt_character_intrinsics` has live code and no
    live data (see `every_shipped_enemy_placement_can_be_built`).
19. ✔ **Remove `PreparedKit::HostCode`** — the protagonist is a definition plus
    runtime body state like everything else.

    ⇥ ✔ **DONE, verified 2026-08-13 by reading both enums rather than the name.**
    `PlayableKitSource` has exactly ONE variant, `Authored`; the row-level
    selector that could choose a code-side kit is deleted and every shipped row
    is authored. `PreparedKit`'s second arm is `Unauthored { authored_moveset }`
    and its doc records the rename with the reason: *"this was called `HostCode`,
    after a `PlayableKitSource` variant that NO LONGER EXISTS … what reaches this
    arm now is only the absence"*.

    ⭐ **the rename is the substance, not cosmetics.** `HostCode` named a
    SELECTOR — a row choosing the host's kit — and `Unauthored` names an ABSENCE:
    an id the catalog does not know, or no catalog at all. A body reaching that
    arm is not asking for the protagonist's kit; nobody authored one for it.
    ⚠ `authored_moveset` survives on that arm because a character may bring
    timelines without an action set.

    ⚠ one comment in `starting_character.rs` still said an authored charger would
    *"keep charging once `HostCode` is gone"* — it is gone; corrected the same
    day.
20. ◐ **Remove the scaffolding**: `BUILDABLE_ONLY_CAST` (a registered complete
    definition should simply BE buildable), the definition→catalog→archetype
    precedence bridges, `PLAYABLE_ROSTER` as a buildability gate.

    ⇥ ✔ **`BUILDABLE_ONLY_CAST` IS GONE, and gone the way this item asks rather
    than by renaming** (verified 2026-08-13: the only surviving mention is one
    comment recording it as history). `buildable_only_cast()` is DERIVED —
    `authored_ids()` off `AUTHORED_CAST`, minus anything already on
    `PLAYABLE_ROSTER`. So a character that authors a body IS buildable, with no
    second list to remember, which is exactly *"a registered complete definition
    should simply BE buildable"*.

    ⭐ **the exclusion became structural too.** The old hand list carried a note
    — *"the parrot is NOT here and must not be… listing it twice would register
    it twice"* — a rule enforced by a reader noticing a comment. Deriving the cast
    surfaced FIVE characters that author a body and appear on the select grid at
    once, and the `.filter(|id| !PLAYABLE_ROSTER.contains(id))` is that note
    turned into code.

    ⇥ ▢ **what is left is `REGISTERED_WITHOUT_A_BODY`, and it is D96 item 8.**
    Six pirates plus Stargan: characters registered so their POLICY reaches a
    body (D98) while their vitals stay unauthored, because *how tough a pirate
    quartermaster is* is a content decision and authoring a number to empty the
    list would be inventing one. Its own doc already says it should shrink.
    ⚠ it cannot be derived from the pirate RULE that gives them their policy —
    `starts_with("npc_pirate_")` covers a pirate added tomorrow, which is the
    property that rule exists for, while this list answers the different question
    of which ids to REGISTER, and that needs names.

    ⇥ ▢ **the archetype precedence bridge** is `adopt_character_intrinsics`,
    which item (9) deletes behind item (7) — and it already has zero shipped
    placements to serve (see item 18).
21. ◐ **Split or delete `ActorTuning` and `CharacterBrainSpec`** by their actual
    remaining responsibilities. ⚠ the capability-authored-twice set is exactly
    `can_blink`/`can_fly`/`can_shield` vs `smash_can_*`.

    ⇥ ✔ **`CharacterBrainSpec` IS DELETED** — zero mentions in the workspace
    (verified 2026-08-13). Half of this item resolved by removal rather than by
    splitting.

    ⇥ ✔ **AND THE ⚠ POINTS AT NOTHING: the capability duplication is gone.**
    `smash_can_blink` / `smash_can_fly` / `smash_can_shield` have zero mentions;
    they were mirrored onto the controller profile and deleted 2026-08-11 (Jon's
    redirect §7) on the rule that *a capability copied onto a controller policy
    makes the policy unreusable, because the copy describes ONE body*. The three
    `can_*` survive on `ArchetypeSpec` as an archetype row's authored VERBS, and
    they feed exactly one port: `movement_kit()` → the body's `AbilitySet`. The
    Smash brain reads the BODY (`can_fly: body.fly || body.fly_toggle`), never
    the row. One authored source, two readers, no copy.

    ⇥ ▢ **what is left is `ActorTuning` the TYPE.** Its fields are already divided
    by authority (campaign P2.19: 13 body, 3 controller, 4 placement) and two of
    those divisions have landed as behaviour — the notice radius and patrol
    decision read `BrainProfile` rather than a silhouette read-model, and
    `attacks_player` is `is_hostile`/`hostile_by_default`. The struct itself goes
    with `ArchetypeSpec` in item (22), because every remaining field has a live
    reader and the projection that fills them is the archetype road.
22. ▢ **Delete** `ArchetypeSpec`, `CharacterRoster`, the roster fragments and
    registry, `spec_for_brain`, `character_archetypes.ron`, `enemy_roster.rs`.
23. ▢ **Rename and document** the final architecture; retire `archetype`,
    `sprite_character_id` and `art_identity`-as-gameplay from the vocabulary.

### Deferred by decision, not forgotten

* `never_dies` sits in `CharacterDeathTraits` though it is a MORTALITY policy
  rather than an on-death consequence. Split it when a second mortality knob
  appears, not before.
* `a_definition_carries_no_controller_binding` must have its PROSE rewritten
  (Jon justified a default profile), not be deleted — it still guards the rule
  that the CURRENT controller stays off the definition.


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

⚠ **the phase table is the STATE and the REMAINING WORK checklist above is the
WORK.** These appendices say what "done" means and why; they are not a task
list, and nothing in them should be worked directly.

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

⚠ **condensed 2026-08-10** from a 295-line chronology to what a resuming
session needs: what LANDED, what is OPEN with its measured blocker, and the
traps that became RULES. The narrative of how each was found is in the commits.

### Landed

* **Field-ownership ledger** — appendix A, all 49 `ArchetypeSpec` fields.
* **Multi-instance invariant PINNED** —
  `one_character_definition_seats_two_independent_fighters`. The uniqueness
  audit is clean: nothing maps a character id to exactly one entity.
* **Authorable on a character**: death traits (as
  `ambition_characters::actor::CharacterDeathTraits`, plain data lowered into
  `CombatCapabilities`), knockback weight, and a default autonomous profile.
* **Typed identities** — `ambition_entity_catalog::CharacterId`
  (`#[serde(transparent)]`) on `EnemySpawnSpec`, `CharacterDefinition::id`,
  `PreparedCharacterDefinition::id`, the prepared registry KEY, and
  `WornCharacter`'s inner value. `impl Borrow<str>` keeps `get(&str)` working,
  which is why the key change cost nine call sites and not a sweep.
* **`BrainProfileRef` (authored, provider-relative) vs `BrainPresetId`
  (resolved)**, and preparation now RESOLVES it, so `resolve_initial_brain`
  uses a canonical key verbatim instead of re-qualifying per spawn.
* **Gameplay identity no longer comes from presentation** —
  `gameplay_character_id()` has no display-name fallback;
  `presentation_identity()` keeps one, for art only.
* **`CharacterSpawnPlan` + `SpawnContext`** (appendix E), with the authored
  enemy lowering through them, and `plan.definition()` distinguishing
  *unmigrated* from *authored-but-not-prepared*.
* **The phase-3 harness** — `mod authored_enemy_reads_its_character` builds an
  authored enemy against a populated registry. Its second test is the
  inversion's poison: a spawn named `"Busy Beaver"` that claims no character
  keeps its archetype's HP though the beaver character authors more, and it
  asserts the sprite DID resolve first so the gameplay assertion cannot be
  vacuous.
* **Persona-derive membership** — `apply_worn_character_gameplay` takes
  `Option<&mut ActorMoveset>` and mints at most one per body.

### Open, with the measured blocker

* ✔ **The type move — DONE, and the design call was answered the way this bullet
  leaned** (landed 2026-08-12, found still marked ▢ on 2026-08-13). The authored
  `CharacterDefinition` lives in `ambition_characters/src/actor/definition.rs`
  and `PreparedCharacterDefinition` stayed above, *"because resolving a kit is
  runtime work"* — the file's own module doc records the reasoning.

  ⭐ **and the coupling did not merely move, it INVERTED.** `derive_moveset` no
  longer reaches into `ambition_combat` at all: the derivation lives in
  `ambition_characters::moveset_prefabs` and `ambition_combat` RE-EXPORTS it, so
  its call sites are unchanged. `ambition_characters` does not depend on
  `ambition_combat` — verified against both `Cargo.toml`s, not by grep. That was
  *"the last thing keeping the authoritative character model from following the
  model down"*.

  ⇥ AS WRITTEN: ▢ *"`definition.rs` reaches into `ambition_combat` in exactly ONE
  place now — `build_actor_moveset` at the fold... ⛔ answer it before moving
  anything."*
* ▢ **`WornCharacter` → universal `CharacterIdentity`.** Blocked, and not by the
  rename: attaching it ENROLS a body in `apply_worn_character_gameplay`, which
  re-derives its kit THROUGH THE CATALOG — phase 2's endpoint arriving early.

  ⇥ ⚠ **THE BLOCKER SHRANK AND DID NOT DISAPPEAR** (re-measured 2026-08-13, and
  worth stating precisely because the difference decides the size of the job).
  `apply_worn_character_overlay` now says *"the REGISTRY first, then the catalog,
  then the id"* and does it — the catalog is a FALLBACK, not the resolver. So a
  body whose character is registered and body-complete would be enrolled and get
  the RIGHT answer.

  ⇒ **what is still exposed is the 14 characters that cannot build a body from
  their own definition** (`the_cast_that_still_needs_a_body_assist_only_shrinks`)
  — for those the fallback is still what answers, so enrolling everybody hands
  the catalog's kit to exactly the population D96 items 5 and 8 are about.
  ⚠ `catalog: Res<CharacterCatalog>` is also still a REQUIRED resource on the
  system, which is a second, smaller thread.
  ⚠ the render layer is NOT the obstacle; both render systems also gate on
  `PlayerVisual`, which an enemy lacks.
* ✔ **Preparation no longer consults the catalog** for the profile's namespace
  (2026-08-11). The definition's own `provider` qualifies it, and the equality
  it rested on is a guarded fact now — see checklist item A4.
* ▢ **Prepared completeness (appendix C ruling 8) for death traits.** Right in
  principle, and flipping it today makes an exploding mite stop exploding:
  `adopt_character_intrinsics` only overwrites when the definition speaks, so a
  definition that always speaks resets every authored enemy to the default while
  the mites' traits still live on the archetype. Gated on phase 2.
* ▢ Still string-typed: `PreparedSeat::character_id` and several `&str`
  accessors.

### Rules this phase paid for

* ⛔⛔ **RETRACT BY RESETTING, NEVER BY REMOVING.** `try_remove` of a component
  that is a REQUIRED query member took seated fighters out of the actor cluster
  entirely — sixteen integration tests, and the symptom named nothing about
  components (*"player one swung twelve times and the other fighter is still on
  52/52"*). ⚠ the reset is conditional on the PREVIOUS persona having claimed
  the field, or wearing a character strips an exploding mite.
* ⛔ **A required component in a query is a filter nobody wrote down.** Same
  family, found twice: the persona derive required `ActorMoveset`, and the new
  cluster member for the live held item is `Option` for exactly this reason.
* ⛔ **A test whose control cannot fail.** The knockback-weight control asserted
  an unauthored character keeps its archetype's weight — but the fixture
  authored none, so it defaulted to the reference `1.0`, which is what the bug
  writes. Fixtures must separate *kept* from *overwritten with the ambient
  default*.
* ⛔ **Do not synthesise a namespace.** Qualifying the profile with the
  definition's `provider` looked equivalent to the catalog's — the two are
  assumed equal and never checked, and a catalog-less fixture produced
  `test::patrol_peaceful`, a key that exists nowhere.
* ⭐ **The catalog fold is FOUR FIELDS** — `max_health`, `motion_model`,
  `movement_tuning`, and the kit. Not a pervasive dependency, and it closes when
  phase 2 authors those on definitions.
* ⛔ **`PLAYABLE_ROSTER` cannot stop gating buildability until definitions carry
  the archetype's intrinsic facts** — registering every catalog row flipped ~100
  exploration NPCs onto defaults. Removing the workaround is the LAST step of
  phase 2, not the first.
* ⚠ **`a_definition_carries_no_controller_binding`** guards the rule that the
  CURRENT controller stays off the definition. Jon has justified a DEFAULT
  profile; rewrite that test's prose rather than deleting it.



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
  intrinsic, HAS A HOME    contact_strength · damage_amount — since 2026-08-12 they are
                           `CharacterDefinition::contact_damage` (`ContactDamage { strength,
                           amount }`), authored by every contact-damaging character that has
                           migrated. The line below read "NO HOME YET"; see checklist item 11.
  CONTROLLER               patrol_effort · chase_effort · aggro_radius
                           attack_range · brain_template · smash_hit_band
  PLACEMENT                respawn: OnRoomReenter
```

~~⚠ **and the deeper problem is the CONSUMER, not the home.** A mite only ever
appears as an `EnemySpawn`, and that path builds its body from
`ActorClusterSeed::new_in` → `spec.combat_capabilities()` — **the archetype**.
An enemy body carries no `WornCharacter`, so the persona derive never runs on
it. ⇒ authoring a mite's death traits on its definition today would state them
in a place nothing on its own spawn path reads, and deleting them from the
archetype would simply turn them off.~~ ⛔ **SUPERSEDED THE SAME DAY — do not
act on the paragraph above**; it is kept because its diagnosis of WHY the
consumer was missing is still the clearest one.

### ⇥ WHAT GROUP A ACTUALLY COSTS — measured 2026-08-10, and it is LDtk work

The eight mite spawns are all in `sandbox.ldtk`, all carry
`brain: exploding_mite | dividing_mite`, and **none carries a `character_id`**.

```text
world                 EnemySpawns   carry character_id
intro.ldtk                    16                    0
sandbox.ldtk                  49                    0
mary_o.ldtk                   24                   24
sanic_speedway.ldtk            4                    4
                    ------------   ------------------
                              93                   28
```

⇒ the brief's *"28 already author a character id"* is CONFIRMED, and it is
entirely Mary-O plus Sanic. **Every one of the 65 that do not are in `intro` or
`sandbox`, whose entity instances carry no `character_id` field instance at
all** — so phase 4 there is not "fill in a value", it is "add the field
instance to 65 entities".

⛔ **that makes group A an LDtk edit, with this repo's two standing hazards:**
never `json.dumps` a `.ldtk` (the formatting does not survive, and `repair`
does not restore it), and the LDtk EDITOR nulls `EntityRef`s world-wide — count
refs before and after any session. ⇒ the mite migration wants a surgical,
formatting-preserving edit and a ref count on both sides, not a JSON round-trip.

⚠ **I got this census wrong once before getting it right**, and the way it
failed is worth copying: my first pass keyed on a field called `character` and
reported `authored=0` everywhere, which would have been filed as a dramatic
correction to the brief. The LDtk fieldDef is `character_id`. **Checking the
field NAME against `defs.entities[].fieldDefs` is what turned a false alarm into
a real measurement** — a census that reads zero everywhere is usually asking the
wrong question.

### ⇥ UNBLOCKED — the authored enemy path reads its character

✔ `spawn_enemy_with_faction_into` lowers the placement to a
`CharacterSpawnPlan` and asks `plan.definition(registry)`, so an authored
`EnemySpawn` whose character is REGISTERED takes that character's health,
knockback weight and death traits over its archetype's. Both callers that
matter reach it: the construction executor (`construct_authored_enemy`) and the
giant host.

⛔ **NOT "the spawn's art identity", and an earlier version of this paragraph
said so.** The lookup goes through the placement's `gameplay_character_id()`,
which has no display-name fallback; the art road is a separate accessor. A
spawn that has not named a character resolves nothing and keeps its archetype.

⇒ **group A can proceed mechanically** — author the mite's facts on its
definition, register it, delete them from the archetype. ⚠ but appendix C
REORDERS it behind the common constructor, so "can" is not "should": the
constructor now exists (`CharacterSpawnPlan`), and group A resumes once the NPC
path lowers through it too. ⚠ `BUILDABLE_ONLY_CAST` is short-lived migration
scaffolding, not the registration mechanism to build on.

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

⭐⭐ **THIS CLASSIFICATION IS NOW ENFORCED BY THE COMPILER** (2026-08-13):
`archetype_spec::authority_split_tests::every_archetype_field_has_a_destination`
exhaustively destructures the type with every field filed under one of four
headings. **Add a field to the archetype schema and `ambition_combat` stops
compiling until somebody says where it goes**; remove one and the same. ⇒ where
the appendix below and that function disagree, THE FUNCTION IS THE ANSWER — it
cannot be true of a tree it does not compile against.

⚠ **the count held.** Re-asked by the destructure rather than by counting, it is
still 49 — on the same day a hand grep undercounted the much smaller
`ActorTuning` by six and a correction had to be retracted. ⇒ a `grep -A N` window
truncates silently; a destructure cannot.

⚠ **two judgement calls are marked in the code rather than hidden**: the four
`smash_*` fields are filed as CONTROLLER policy wearing a mode's name, with
`smash_heavy` **checked and confirmed policy** — it reads like a weight class but selects
`SmashCfg::BRUTE_DEFAULT` over `STRIKER_DEFAULT`, a whole AI preset, so it is not a duplicate of `weight`, and
`dream_seed` / `ranged_visual` are presentation projected FROM the body rather
than a fourth authority.

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

1. ✔ **`is_aerial`** — was a live TWO-SOURCE CONFLICT, already documented on the field,
   ⛔ **and I cited queue D74 as it breaking in the wild — WRONGLY, retracted
   2026-08-10.** The probe measured `gravity_scale = 1.0` on the registered PCA,
   so nothing floated it; that symptom is a movement divergence during
   possession and is not an instance of this conflict. The conflict itself is
   still real and still unresolved — two authorities state one body's
   aerial-ness — but it has no witness yet, which is exactly what it had before.

   ⇒ ✔✔ **RESOLVED 2026-08-13, and structurally rather than by a decision.**
   Traced through all three construction roads with the PCA finally registered
   (ledger D74), each has exactly ONE authority and no body can see two:

   ```text
     new_character_in      prepared `baseline_free_flight`  — catalog folded at
                                                              PREPARATION
     new_peaceful_npc_in   the character first; the catalog only for a
                           character with NO PREPARED ENTRY
     new_in (archetype)    `spec.is_aerial` — reached only by a placement that
                           names no complete character
   ```

   ⭐ **the fold is what closes it.** `finalize_character` resolves
   `baseline_free_flight: None` to `Some(false)`, so a prepared character is
   never MUTE about flight — and "complete" (the test that sends a placement down
   the character-first road) requires locomotion, which carries that field. A body
   whose character states flight therefore cannot reach the archetype road, and a
   body on the archetype road has no character answer to contradict.

   ⚠ **and D89 cut the catalog edge rather than demoting it**: `finalize_character`
   does not read `body_kind` at all. `Floating` still answers a real question —
   it supplies no `default_standing_height`, which is why the PCA is 68px and not
   `Standard`'s 48 — so geometry and locomotion were decoupled through the one
   enum and only the locomotion edge was cut. ⚠ a stale ⚠ line above that fix
   still claimed *"the catalog's answer may fill it"*; corrected the same day.

   ⇒ what REMAINS is not a conflict but an `unwrap_or` arm: the peaceful road's
   catalog fallback, which answers for ~150 unprepared NPC placements and becomes
   unreachable when the registry holds everything. `Option<bool>` must still
   survive the move for the reason below.
   `new_peaceful_npc_in` reads the catalog's `body_kind: Floating`; the hostile
   `EnemySpawn` path reads this. **The Perfect Cellular Automaton is `Floating`
   in its catalog row and played grounded by the shipped duel.** Unifying the two
   authorities forces that disagreement to resolve, and resolving it changes how
   a shipped fight plays. ⛔ do not fold it silently — this is exactly the class
   of thing the brief means by *"semantically migrated, not blindly trusted"*.
   `Option<bool>` must survive the move: `None` ≠ `Some(false)` is why the
   conflict is expressible at all.
2. ✔ **`is_sandbag`** — read as a character fact and behaved as a placement role.
   It reaches the RENDER read model (`ActorRenderView.is_sandbag`, a
   sprite-upgrade fallback), `save_sync`, and cluster pathing. A sandbag is a
   training instance of some body, which argues placement; but three consumers
   treat it as identity. Decide once, and move all three.

   ⇒ ✔ **ANSWERED BY MEASUREMENT 2026-08-13: it is a CHARACTER fact, and there
   is nothing left to move.** No shipped archetype row sets `is_sandbag` — the
   only two that do are ENGINE FIXTURE rows, kept (by their own note) for *"tests
   about the archetype machinery itself … about the SHAPE, not about Ambition's
   dummy"*, so they retire with `ArchetypeSpec` rather than before it. Every
   shipped practice target is a CHARACTER authoring `as_practice_target()`, and
   all three consumers read `ActorTuning::is_sandbag`, a projection fed from
   `CharacterDefinition::practice_target` on the character-first road.

   ⭐ **and the one behaviour that reads the archetype field DIRECTLY already
   agrees across both roads**: `new_in` suppresses patrol-path attachment for a
   sandbag row, and `new_character_in` suppresses it for `practice_target` — with
   a note saying why the second was added (*"a dummy on a patrol path is a dummy
   that walks away from the player practising on it"*). That is the fork this
   appendix worried about, already unified, and unified in the direction this
   ruling would have chosen.

   ⚠ the invariant that used to guard it, `CharacterRoster::sandbags_are_passive`,
   went vacuous when its subject migrated and is deleted; the claim lives on the
   characters now (`practice_target_characters_do_not_strike_back`), and it
   asserts the POLICY rather than the old rule's `melee: None` proxy — both
   sandbags carry a real `PunchWeak` on purpose.
3. ✔ **`never_dies`** — same shape, cleaner answer: `damage_apply` uses it to make
   a body take no health damage. That is either an intrinsic trait (an immortal
   creature) or a training-mode ruleset fact. The brief allows the intrinsic
   reading *"only where these really are properties of the character"* — the
   shipped users are sandbags, which suggests it travels with `is_sandbag`.

   ⇒ ✔ **AND IT TRAVELLED, 2026-08-13's measurement finding it already there.**
   No shipped archetype row sets `never_dies`; `sandbag_infinite` authors it on
   its own definition, and that file's doc already argues the intrinsic reading —
   it is *"a separate creature rather than a flag on the sandbag"*. So the
   appendix's suggested answer is the shipped one, reached by the content rather
   than by this ruling.

   ⚠ its own note records the residue worth keeping: `9999` health AND
   `never_dies` is *"one fact stated twice"*, with each half read by a different
   consumer (the number by damage readouts, the flag by the death check). That is
   a smaller duplication than this appendix is about, and it belongs to whoever
   unifies mortality.
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
* **`context`** — ⛔ **NARROWED 2026-08-10, and the first draft of this bullet
  was the trap.** It listed feature identity, faction, disposition, respawn,
  encounter membership and patrol paths, on the rule *"every member must be a
  decision the PLACEMENT made"*. That rule is necessary and NOT sufficient: a
  placement decides plenty that belongs to one authoring surface rather than to
  the shared constructor. A match seat, a summon and a programmatic spawn should
  not have to manufacture a room-style display name or an empty path list to use
  the common constructor.
  ⇒ `SpawnContext` is `{ feature_id, aabb }` — runtime identity and where the
  body goes. The display NAME (presentation/debug label), FACTION (relationship
  policy) and room PATHS (autonomous-controller input) went back to the enemy
  call site until a SECOND caller shows they are shared, at which point they
  want their own contextual types — `InitialRelations`,
  `AutonomousControllerContext`, presentation attachments — rather than more
  members here.

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

1. ✔ `SpawnContext`, from the enemy path only, carrying today's contextual
   fields verbatim — feature id/name, aabb, faction, paths.
2. ✔ `CharacterSpawnPlan` around it. ⚠ **`controller` did NOT survive contact**:
   written as the two-variant enum this list proposed, then removed the same
   hour because the compiler pointed out nothing reads it — an authored enemy
   authors no override and is always autonomous. *"Resist widening it before a
   third caller needs it"* turned out to understate the rule: resist adding it
   before the FIRST caller needs it. It returns with the NPC path.
3. ✔ The authored enemy lowers through it, `EnemyActorSpawnPlan` still the
   lowered result. The harness was already there:
   `mod authored_enemy_reads_its_character`.
4. ▢ **NEXT: the NPC path**, then encounter/programmatic/summon, then
   `PreparedMatch` — which appendix D names as the proving ground and which is
   where the `CharacterRoster` dependency finally comes out.
   ⚠ the NPC path's prerequisite is typing `character_id` on BOTH
   `InteractionKind::Npc` and its `InteractionKindSpec` mirror, ~37 sites; wide
   and mechanical, wants its own slice. See the D73 ledger row.
