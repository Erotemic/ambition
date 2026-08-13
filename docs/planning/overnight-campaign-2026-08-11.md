# Overnight campaign: finish the character architecture and make Smash feel like a real platform fighter

⛔⛔ **SUPERSEDED IN PART, 2026-08-11 (later the same day):
`redirect-2026-08-11-finish-the-architecture.md` OUTRANKS THIS FILE.** Jon's
redirect off checkpoint `853d9a66b5ed` replaces the P0.1–P5.42 execution order
below with its own P0–P4, and its first instruction is **pause broad character
migration**: D78 is not a rollback bug, it is the last two-phase seam in ordinary
character construction, and fixing that root comes before any further migration
rows. Everything this file says about the two coupled goals, the three
authorities, deleting legacy AS EACH MIGRATION LANDS and the prohibitions still
stands. Read the redirect first; read the RESUME block below for measured state.

⛔ **Jon's brief, verbatim, given 2026-08-11T03:0xZ.** Recorded here at his
instruction ("You might want to record this in docs planning so it survives
compaction"). Nothing below this banner is agent prose; the only agent addition
is the status block immediately after it. Where this file and
`character-template-architecture-2026-08-10.md` disagree, **this file wins** —
it is newer and it is his, and its "Overnight execution order" (P0–P5) replaces
that file's 23-item checklist as the ordering authority.

> **Baseline check, done at arming:** the brief's handoff baseline
> `6079f2233d7244e64a8c87123f92aac0da809b22` ("wip") IS an ancestor of HEAD, which
> is **11 commits newer**. Per the brief, that newer work is inspected before
> anything it names is redone. The measured state of it lives in
> `character-template-architecture-2026-08-10.md` ("REMAINING WORK — THE ONE
> CHECKLIST" and "Phase 1 progress"), which is where the per-item measurements
> are; use that file as the survey and this file as the order.

## ⇥ RESUME HERE (agent, keep current)

**Rewritten 2026-08-11 after a long session; everything above the previous
version was stale within hours, so trust THIS block and the progress table, not
memory.**

* ⭐ **THE LEDGER: `character_archetypes.ron` is 600 lines / 13 rows, down from
  843.** Deleted today, each with its migration in the same change: both mites,
  the puppy slug, the sky parrot, AI Slop, the burning flying shark, the GIANT
  GNU, the giant's HANDS, both SHARK RIDERS (`pirate_shark_rider`,
  `pirate_heavy_shark_rider`) and the FINITE SANDBAG. Two provider fragments
  went too — Sanic's entirely, Mary-O's down to the plane swarms alone.
* ⭐ **the three authorities are all AUTHORABLE now.** A character states its
  body (`vitals`, `locomotion`, `contact_damage`, `abilities`, `action_set`,
  `held_item`, `mount`, `practice_target`, `dream_seed`, `death_traits`); a
  `BrainProfile` states policy (`template`, distances, `patrol_effort`,
  `chase_effort`, `attacks_player`, smash tactics) and can now be SHARED by name
  through `CharacterCatalogData.autonomous_profiles` +
  `CharacterDefinition::autonomous_profile_ref`; a placement states
  `character_id`, `disposition` and `respawn` (every world's `EnemySpawn` has
  the `respawn` fieldDef now).
* ⛔⛔ **THE ONE BLOCKER THAT MATTERS: ledger D78.** Any character-first ENEMY
  body in a room the rollback oracle simulates DESYNCS. Twelve probes bisected
  it to the persona projection WRITING `ActionSet` mid-run; granting the same
  value at construction instead is GREEN. It is NOT hostility, the crawler, the
  slug, global ordering, an unaccounted component, an unregistered persona
  output, or a cross-schedule derive — all measured, all in the row. Two fixes
  were implemented and reverted. ⇒ this blocks the three intro "Puppy Slug"
  placements and is the reason to be careful about migrating anything into a
  rollback-tested room. **Read D78 before touching the persona seam.**
* ⇥ **WHAT IS LEFT OF THE ROWS**, and it is mostly placement work now: four
  Group-C roles (`small_skitter`, `large_brute`, `gradient_seeker`,
  `ranged_skirmisher`) take the same recipe the goblin proved — lift the row's
  controller half into a shared `autonomous_profiles` entry, give each placement
  the character it actually is. `medium_striker` has 4 placements left,
  `sandbag_infinite` needs a decision (a second catalog row, which would also
  put a second dummy in the Hall), and `pirate_raider`/`pirate_heavy` are the
  PROVOCATION path (P2.20), not a migration.
* ⚠ **ONE PRODUCT QUESTION ONLY JON CAN ANSWER**: the sandbox `0140-0146` block
  is a DEMO ROW of one-of-each archetype — `patrol cutter`, `small skitter`,
  `guard striker`, `medium striker`, `gradient seeker`, `large brute` — with no
  creature identity at all. Either characters get invented for them or the demo
  row goes.
* ⭐ **the recurring trap, hit FOUR times today**: a row authors
  `patrol_effort` (0.4783, 0.5116, 0.6774, 1.0×3) and `BrainProfile` defaults to
  0.5, so deleting a row without moving that number silently retunes the
  creature. Same shape for `attacks_player`, `respawn`, `held_item` and
  `is_sandbag`. **Diff the row's fields against the character's before deleting,
  every time.**
* ⚠ **pre-existing red, not from this campaign**, re-confirmed on a fully
  stashed tree including the submodule:
  `ambition_demo_mary_o_app::level_1_acceptance::a_small_mary_o_dies_to_one_hit_and_the_level_restarts`
  ("a side contact with no armor left must kill her").
* **Gate state**: `cargo check -p ambition_app --all-targets` clean,
  `cargo test -p ambition_app --test app_it` 327 passed / 0 failed, workspace
  `--all-targets` clean, nothing uncommitted.

## Campaign progress (live — update as slices land)

| P | Item | State |
|---|---|---|
| P0.1 | Explicit CharacterId missing from prepared registry must be an error | ✔ **DONE — all three roads refuse, and they refuse by ONE rule.** `CharacterSpawnPlan::definition` has distinguished the three outcomes for a while (`Ok(None)` unmigrated / `Ok(Some)` prepared / `Err(id)` authored-but-unprepared); what was open was the CALLER, and the honest gate was that the enemy road refused while the NPC and encounter roads still said `.and_then(|c| prepared.get(c))` — the same silent answer to *"names nobody"* and *"names somebody nobody registered"*. ⭐ the enemy road's policy now lives beside the plan as `report_unprepared_character` and all three call it: **no cast at all** → one warning about the COMPOSITION, **a fallback can build it** → warn (a BORROWED character is a shipping arrangement), **nothing can build it** → REFUSE. ⚠ each road supplies its own fallback because they differ honestly — the enemy's is its brain key's archetype, the NPC's is its catalog row (an NPC's body comes from the row, so only the KIT is borrowed, which is the D84 pirate defect), the wave's is `new_in`'s roster spec. ⛔ the refusal is poison-verified: removing it turns exactly ONE test red, and the two must-not-refuse cases stayed green. ⇥ the 28 art claims that made a hard error refuse the tree were already gone; the original design note (**the TYPE says it**) is unchanged. |
| P0.2 | Resolve character-owned autonomous profile refs during preparation | ✔ **DONE.** Preparation resolved the ref already; what was open is the half Jon names — *"should not need a parallel catalog row merely to know its namespace"*. It now qualifies with the DEFINITION's own provider (`qualify_in_provider`) and consults no catalog. The two id spaces were "assumed equal, never checked"; `character_provider_namespace` checks them on the shipped composition, and was probed RED by poisoning one registration site. The npc fixture that had argued against this change was repaired to ASSEMBLE its catalog (production namespaces every preset `provider::name`) rather than parse it raw |
| P0.3 | Complete typed CharacterId through prepared registry/runtime/match seams | ✔ **DONE — all three seams named in the item.** ⚠ the REGISTRY half had already landed and the row did not say so: `PreparedCharacterRegistry` is a `BTreeMap<CharacterId, _>` and both `CharacterDefinition::id` and `PreparedCharacterDefinition::id` were typed, with `Borrow<str>` keeping every `&str` lookup working. What was left was the other two. ⇥ **MATCH**: `MatchParticipant::character` and `PreparedSeat::character_id`, which is where a DISPLAY NAME could still be handed in where an id belongs. ⇥ **RUNTIME**: `StartingCharacter::character_id`, plus the staging map `StagedCharacterOverrides::by_id`. ⚠ empty still means *the content default* — `CharacterId` carries an empty string as happily as `String` did, and `is_content_default` is the one reader of that emptiness. ⭐ `#[serde(transparent)]` means every authored file encodes byte-identically, so this is a compile-time change only. ⇥ ▢ residue, deliberately not chased here: `PreparedCharacterOverrides::id` (module-private staging) and the authored-placement/interaction fields (`EnemySpawnSpec`, `InteractionKind::Npc`, `EncounterSpec`) are still `String`/`Option<String>` — those are AUTHORING surfaces and belong with the authoring pass, not the runtime one. |
| P0.4 | Inspect/narrow SpawnContext before adding more callers | ✔ **ALREADY NARROW** — two members, `feature_id` and `aabb`. The display name, faction and room kinematic paths were on it and were taken OFF for exactly Jon's reason (*"a Match participant should not need dummy room paths"*); the type's doc states the rule and names the three evictions. Re-inspected, nothing to remove |
| P0.5 | Fix current-held-item death ownership | ✔ **ALREADY DONE in the tree** — `CharacterDeathTraits::drops_held_item` is a `bool` policy and the drop path reads the body's LIVE held item (`actor_hit.rs`, `held_at_death`). Its own doc records the bug Jon describes: it used to be `Option<HeldItemSpec>`, so a body that picked up a different weapon dropped the one it was authored with. Not redone |
| P1.6 | Finish CharacterIdentity | ◐ **THE SUBSTANCE IS DONE; WHAT IS LEFT IS THE RENAME, AND IT IS JON'S** (measured 2026-08-12). ⛔ `grep CharacterIdentity` returns **0 matches** — the type this row names does not exist, and the row is about EVOLVING `WornCharacter` into it (§*Generalize WornCharacter into the universal character identity*). ⭐ `WornCharacter(pub CharacterId)` is already the typed universal fact: one component, no second runtime component claiming which character a body is, and `#[require(IdentityKit)]` beside it. ⇥ **the census that actually sizes this** — 50 non-test readers of `ActorConfig::sprite_character_id` across 20 files, which is the OTHER thing that could claim the fact. Classified: ~30 are in the presentation crates (`ambition_render`, `ambition_sim_view`, `ambition_character_sprites`) and are right to key on sprite identity; the authored-volume road (`authored_volumes` / `combat_schedule` / `attack`) resolves polygons the SPRITE authors, so sprite-first is correct and it already falls back to `WornCharacter`; bark lines are voice. ⭐ **the two gameplay seams both already prefer `WornCharacter`**: `moveset/mod.rs:508` (`worn.map(..).or_else(|| config…sprite_character_id)`) and the manifest hitbox at 666. ⇥ ✔ **and the census found one real defect, fixed at `e16b968c3`**: `peaceful_config` decided flight from `body_kind: Floating` alone — the one rule the invariant list forbids by name, and the PCA's exact disagreement. Unreachable today (every prepared Floating character authors a profile, so `apply_catalog_mode` returns first), which is why it survived; fixed at the rule rather than left resting on that. ⇒ ▢ **the open half is a NAME, and Jon's brief made it conditional** — *"evolve/rename it into something like `CharacterIdentity(CharacterId)` **if that name best describes the final semantics**"*. ⚠ the argument against: `ActorIdentity` already exists (the label read-model), so `CharacterIdentity` beside it invites exactly the confusion the rename is meant to remove, while `WornCharacter` names the RELATION precisely — a body WEARS a character, and possession / mounting / recharacterization swap it. Recommend keeping the name; the rename is a one-commit mechanical change whenever Jon says otherwise. |
| P1.7 | Move/finalize character domain types into the appropriate low crate | ✔ **DONE 2026-08-12 — see the ⇒ at the end of this row. THE GRAPH CHECK JON'S BRIEF ASKS FOR, DONE 2026-08-12 — and it makes the row a SPLIT rather than a move.** ⇥ `character_runtime/` is 11,083 lines; `definition.rs` alone is 2,061 and holds **three layers**, only two of which can go down: the authored model + namespaces (`SheetTarget`/`PortraitTarget`/`MoveId`/…, `PreparedCharacterDefinition`, `CharacterBindings`, `PreparedKit`, `CharacterBodyBlueprint`), the preparation functions (`prepare_character`, `finalize_character`, `derive_moveset`), and a Bevy **`App` extension** (`CharacterDefinitionAppExt::try_register_character`, `CharacterPreparationPlugin`, `StagedCharacterOverrides`) which is high by definition. ⇥ **the four cross-crate reach-ins, measured from the file rather than guessed** — 43 `ambition_characters`, 16 `ambition_entity_catalog`, 8 `ambition_platformer2d_core` are all things `ambition_characters` ALREADY depends on, so they are free. What remains: **(1) ✔ NOT A BLOCKER — `ambition_platformer2d_shared_tangle::binding`** (the `Resolver`/`UnresolvedRef` boundary). shared_tangle depends on `projectile_spec` + `core` and **not** on `ambition_characters`, so `ambition_characters → shared_tangle` is a legal downward edge. **(2) ⛔ A HARD CYCLE — `ambition_sprite_sheet`.** That crate DEPENDS ON `ambition_characters`, so any direct call from moved code is a cycle. Two call sites, both in `CharacterBindings::with_engine_{sheet,portrait}_vocabulary`, and **both already documented as belonging elsewhere**: *"kept OUT of `prepare_character`… this is the registration seam's job, because registration is where the engine is"*. ⇒ they travel UP with the App extension, not down with the model, and `with_available_sheets` / `with_available_portraits` are the public setters that already express the inversion. ⚠ inherent methods must live in the defining crate, so once `CharacterBindings` moves they become free functions at the seam — the ORPHAN RULE adjudicating placement, exactly as it has before. **(3) ◐ TWO REAL DESIGN OBSTACLES — ONE IS GONE.** `prepare_character` reached into `crate::avatar` and `crate::combat` from INSIDE preparation. ⛔ Jon's brief: *"do not solve dependency cycles by leaving the authoritative character model inside the actor monolith"*, and it names the method — extract the lower FACT, then lower it, as `CombatCapabilities` was. ⇥ ✔ **`motion_model_spec_for_character_id` IS LOWERED.** Its body was eighteen lines reading nothing but `CharacterCatalog` and `ambition_platformer2d_core`, both visible from `ambition_characters` — a catalog question written next to its first caller, with nothing avatar-domain about it. It is `CharacterCatalog::motion_model_spec` now; preparation asks the catalog directly and `definition.rs` names `crate::avatar` **zero** times. The monolith keeps the old free function as a one-line delegate so its six call sites and their tests are untouched. ⇥ ▢ **what is left is `crate::combat::moveset::build_actor_moveset`**, and `ambition_combat` DEPENDS ON `ambition_characters`, so the call cannot follow the model down. ⭐ **but it is lowerable, and the residue is THREE LINES** — measured 2026-08-12 rather than estimated. Its inputs are `ambition_characters::brain`'s `Melee`/`Ranged`/`SpecialActionSpec` and its output is `ambition_entity_catalog::MovesetContract`: every type is already visible from `ambition_characters`. Its home, `moveset/prefabs.rs`, is 893 lines whose ONLY non-lowerable references are `crate::on_hit::{POGO_BOUNCE_KEY, set_pogo_sfx}` (3 uses, combat-internal effect keys) and one `ambition_vfx::move_vfx_kind` — and that one sits in `presentation_problems`, a VALIDATION helper outside `build_actor_moveset`'s path, while `ambition_vfx` depends on geometry/projectile_spec/sfx and not on `ambition_characters`, so even that edge would point downward. ⇒ the decision was what to do with the pogo keys. ⇥ ✔ **AND IT IS MADE: the technique's SCHEMA is lowered** (2026-08-12). `POGO_BOUNCE_KEY`, `PogoBounceParams` and its three accessors sat in `ambition_combat::on_hit` beside the Bevy system that executes the rebound; the lower FACT is *what a `pogo_bounce` effect SAYS* and it is `ambition_characters::technique` now, while the queries, policies and message stay where the bodies are — the `CombatCapabilities` → `CharacterDeathTraits` split Jon's brief names as the precedent. ⚠ the cue comes back as a `String` rather than an `SfxId`: wrapping it would buy an `ambition_characters → ambition_sfx` edge for one newtype, and the low crate owning the authored TEXT is better layering. `ambition_combat::on_hit` keeps `pogo_sfx_from` as that adapter and re-exports the rest, so its call sites are untouched. ⇒ **`moveset/prefabs.rs` names ZERO `crate::` paths.** ⇥ ✔ **and the `ambition_vfx` use is gone too** (2026-08-12): it sat in `MovePrefabRegistry::expand`, which validates an authored move's presentation ids against what renderers can draw — so the REGISTRY moved to its own `moveset/prefab_registry.rs` rather than the validation being dropped. Building a move from a spec and EXPANDING an authored prefab key are different jobs, and only the second needs to know what a renderer can draw. `prefabs.rs` now references `ambition_characters`, `ambition_entity_catalog` and `super::*`, and the one surviving `ambition_vfx` mention is a doc comment. ⇥ ✔ **AND THE GLOB IS GONE, so the coupling is MEASURED** (2026-08-12). `prefabs.rs` said `use super::*`, which is how a module's real coupling stays unknown — a bulk move cannot be planned against a glob. Made explicit, what it needs is three groups: `ambition_entity_catalog` (the whole `MoveSpec` vocabulary plus the four VERB constants, which live there and are free), `ambition_characters::brain::action_set` (the three action specs), and **SIX `&str` constants from this crate** — `SWING_SFX_CUE`, `SLASH_ARC_VFX`, `SLASH_POKE_VFX` and three `PLAYER_ROBOT_*` cues. ⇒ **that is the entire remaining coupling**, and it is the same class as `POGO_BOUNCE_KEY` before it was lowered: plain presentation ids that travel with the builders. ⚠ the cleanup also emptied four imports out of `moveset/mod.rs` that existed ONLY to feed the glob, and moved two into `tests.rs` where they are used. ⇒ ✔✔ **AND IT IS DONE (2026-08-12): `prefabs.rs` IS `ambition_characters::moveset_prefabs`.** The six presentation constants travelled with it; their compile-time hash pins against `ambition_sfx::ids` did NOT and must not — that crate is invisible from here — so the low crate owns the authored text and `ambition_combat` keeps the pin, the same split as `pogo_sfx_cue_from`. `ambition_combat::moveset` re-exports the whole module, so every `moveset::<builder>` path in the workspace is unchanged. ⇒ **`character_runtime/definition.rs` now reaches into the monolith ZERO times** — the last line, `crate::combat::moveset::build_actor_moveset`, calls `ambition_characters::moveset_prefabs` directly. ⇥ ◐ **AND THE MOVE OF `definition.rs` HAS STARTED.** ✔ the crate EDGE it needs is in and verified: `ambition_characters → ambition_platformer2d_shared_tangle`, for the `Resolver`/`BindingReport`/`Namespace` boundary preparation is written in terms of. ⛔ **checked, not assumed** — `check_absence_contracts.py`'s `ambition_characters` contract is TRANSITIVE, all 25 contracts hold, and the capability-footprint ratchet is unchanged at 41 crates because shared_tangle was already in the closure. ⚠ three satellite lockfiles were stale and `cargo tree --locked` was failing before it; `fixtures/external_consumer/Cargo.lock` is GITIGNORED and never shows in `git status`, which is the one that hides. ✔ **first slice landed**: the seven `Namespace` marker types (`SfxCueId`, `MoveId`, `VerbId`, `SheetTarget`, `PortraitTarget`, `RangedPayload`, `VfxTag`) are `ambition_characters::binding_namespaces`, re-exported so every `character_runtime::{..}` path is unchanged. ⇥ ⛔⛔ **GPT 5.6 REDIRECT, 2026-08-12: THE BULK MOVE BLOATED THE LOW CRATE.** Lowering `prefabs.rs` solved the cycle AND dragged canonical PLAYER-ROBOT PRESENTATION POLICY into `ambition_characters` with it — `apply_player_robot_slash_sfx` plus the three `PLAYER_ROBOT_*` cue constants. ⇥ **measured, and GPT is right**: that overlay has exactly ONE production caller, `avatar/starting_character.rs:311` (the protagonist path), and `prepare_character` never reaches it. It did not need to move; it moved because it was in the file. ⇒ **the fix is to send it back up** — the low crate should hold only what preparation actually calls (`attack_move_from_melee`, `directional_attack_variants`, `special_move_from_spec`, `build_actor_moveset` and the `simple_*` builders); one character's sound policy belongs with the protagonist road. ⚠ **and the same question is owed to the rest of the move**: `SWING_SFX_CUE` / `SLASH_ARC_VFX` / `SLASH_POKE_VFX` are generic and are genuinely used by the builders, so they stay — but every other item that travelled should be re-checked against *does preparation call it*, not *was it adjacent*. ⛔ do NOT continue the `definition.rs` split until this pass is done; the same failure would land ten times larger. ⇥ ✔✔ **THE PASS IS DONE, 2026-08-12, and the audit answered ONE WRONG and TWO RIGHT.** **(a) the robot overlay: WRONG, and reverted** — `apply_player_robot_slash_sfx` and the three `PLAYER_ROBOT_*` cues are back in `ambition_combat::moveset::player_robot_slash`, beside the compile-time hash pins that correctly never left, so the text and its proof are one crate again. A 26th absence contract, `the-character-domain-is-not-named-after-a-character`, holds the line: no `fn` / `const` / `struct` in `ambition_characters` may be NAMED after one creature. It runs comment-stripped, so the crate may still EXPLAIN itself with a concrete example (`entry.rs` cites `player_robot_v3`) and TEST with one (`npc_puppy_slug` fixtures). ⚠ FALSIFIED: re-adding one `pub const PLAYER_ROBOT_SWING_SFX_CUE` turns it red. **(b) `technique.rs`: RIGHT, and it is preparation that calls it** — `moveset_prefabs.rs:618` authors `EffectRef::new(POGO_BOUNCE_KEY)` onto the down-air volume, so the key is builder vocabulary. Its param accessors travel with it because a schema split across two crates is the shape just repaired, not a fix for it. **(c) `binding_namespaces.rs`: RIGHT** — the seven markers ARE the vocabulary preparation resolves against, and they need nothing but `Namespace`. ⇥ ⛔⛔ **AND THAT EDGE WAS RE-DECIDED TOO (GPT finding 3): `ambition_characters` no longer names `shared_tangle` at all.** The note above argued the edge safe because *"the contracts are TRANSITIVE and they hold"* — legality, which is not the same question as FLOOR. shared_tangle is ~18k lines across 51 files of platformer lifecycle, camera, transit, schedules and hotkeys, and the character domain used ONE name from it, so every edit to any of those files invalidated it. The boundary is `ambition_binding` now, whose entire dependency list is `tracing` (ledger D109). ⇒ **the split is unblocked**, with sub-case (b) below corrected first so the mechanical relocation cannot decide the public architecture. ⇥ ✔ **sub-case (a) IS DONE** (`ca6e1e400`): `with_engine_{sheet,portrait}_vocabulary` are one free function `with_engine_vocabularies` at the registration seam, and `CharacterBindings` keeps only the QUESTION (`has_sheet_vocabulary` / `has_portrait_vocabulary`) — a query, not a policy. Two content registrations stopped passing the sheet vocabulary by hand and gained the portrait check they never asked for. D106's regression still drives the real seam and still asserts BOTH halves. ⇥ ⛔⛔ **AND THE CUT IS NOT WHERE THIS ROW SAID IT WAS — measured 2026-08-12, and it changes the design rather than the line number.** The row planned to split at the `App` layer (~1829) and leave preparation below it. That does not work, and the reason is the good news: **`prepare_character` and `finalize_character` are PRIVATE module functions**, and `PreparedCharacterDefinition::voice` plus every `CharacterBindings` field are private with them. That privacy IS the barrier — it is what makes an early fold unreachable. Splitting the module at any line either widens those to `pub` (the ordering hazard back on the production surface, the exact shape sub-case (b) was just corrected for) or widens the field access instead. ⭐ **the measurement that makes the whole thing cheap**: lines 1..1506 of `definition.rs` — the authored model AND the preparation pipeline — contain **ZERO `crate::` reach-ins** (one comment mentions `crate::combat::moveset`; no code does), **ZERO `super::` uses**, **ZERO bevy** (the first derive is `PreparedCharacterRegistry`'s `Resource` at 1541), and exactly FIVE import lines, all naming crates `ambition_characters` already has. It is a clean cut with nothing to untangle. ⇒ **so move MORE, not less, and make the barrier a TYPE.** `PreparedCharacterRegistry` goes down too (`ambition_characters` already derives `Resource` in a dozen files, so bevy is not the obstacle), `prepare_character` / `finalize_character` stay PRIVATE, and the App extension gets two public seams instead of two public functions: a `stage` that mints an OPAQUE staged value and a `finalize_cast` that consumes it. ⭐⭐ **an opaque type minted by one function and consumed by another is the `Bound<N>` pattern this repository already runs and documents** — `Bound` has no public constructor, so a consumer cannot hold one it did not resolve. Applied here, folding early stops being prevented by module privacy and starts being IMPOSSIBLE TO SPELL, which survives a crate boundary where privacy does not. That is strictly stronger than what the monolith has today. ⇥ ▢ **and sub-case (b)'s 18 fixture call sites get the sanctioned road**: an explicit `test-support` feature on `ambition_characters` + a monolith `dev-dependency` enabling it, which is what GPT's own correction allows and what keeps `prepare_and_finalize_for_test` off the production surface. ⛔ do NOT rewrite those 18 fixtures to drive the App instead — `definition_tests.rs` (25 uses) tests PREPARATION and moves down with it; the other seven files test COMPOSITION and only want a prepared value to hand in, which the feature gives them without a lifecycle they are not testing.

  ⇥ ▢ **the rest, with its sub-cases enumerated so the next step is not a survey**: `definition.rs` splits at its `App` layer (line ~1829 — `CharacterDefinitionAppExt`, `CharacterPreparationPlugin`, the `StagedCharacterOverrides` impl, `close_preparation_barrier`, `finalize_prepared_cast`), which STAYS. Three sub-cases to handle rather than discover: (a) `CharacterBindings::with_engine_{sheet,portrait}_vocabulary` are inherent methods that must become FREE functions at the seam once `CharacterBindings` is below (orphan rule); (b) ⛔⛔ **`prepare_and_finalize_for_test` and `FinalizedCharacter` MUST NOT BECOME PLAIN `pub` — this line said they must, and it was wrong** (GPT 5.6 review of `1579ab3`, finding 2; corrected 2026-08-12 before any of it ran). They are `#[cfg(test)] pub(crate)`, `#[cfg(test)]` items do not cross a crate boundary, and the mechanical fix is to widen them — which is exactly how a crate move gets to DECIDE the public architecture. The function's own doc says why it is sealed: it BYPASSES the Bevy finalization barrier and folds a character against whatever catalog happens to exist at that moment, which is the ordering hazard `CharacterPreparationPlugin::finish` was built to remove. Production access would recreate it, and *"the tests live in another crate now"* is not a reason to reopen an invariant. ⇒ **split the TESTS by what they test instead**: preparation/finalization tests move DOWN with the implementation and keep using private `cfg(test)` support; monolith-COMPOSITION tests stay up and drive the real registration/finalization lifecycle. If a genuinely cross-crate fixture still needs the private machinery after that split, it gets an explicit test-support feature + dev-dependency — never an ordinary production API. ⚠ this has to be settled BEFORE the rest of `definition.rs` moves, because the easiest mechanical relocation would otherwise become the architecture; (c) `StagedCharacterOverrides`'s STRUCT lives in `character_runtime/mod.rs`, only its impl is in this file. **(4)** one doc link to `ambition_platformer2d_runtime::finalize`, which costs nothing.  ⇥ ✔✔ **AND IT IS DONE (2026-08-12): `definition.rs` IS 2,061 → 356 LINES.** `ambition_characters::prepared` holds the authored model, the preparation pipeline, `PreparedCharacterRegistry` and `CharacterRegistrationError` — 1,856 lines, of which ~1,750 are a DELETION from the monolith. What stayed is what is genuinely an App's: `try_register_character`, `StagedCharacterOverrides`, `with_engine_vocabularies`, `CharacterPreparationPlugin` and its barrier. ⭐ **the barrier is a TYPE, exactly as designed**: `prepare_character` and `finalize_character` are still PRIVATE, in their new crate, and the App layer holds `StagedCharacter` — minted only by `prepare_for_registration`, consumed only by `finalize_cast`, with three accessors that read identity and nothing that folds. ⚠ the staging resource's doc used to say *"nothing downstream can READ one, because `PreparedCharacterOverrides` does not escape this module"*; it now says nothing downstream can FOLD one, which is weaker in what it hides and stronger in what it prevents. ⭐ **the 18 fixtures took the sanctioned road**: `ambition_characters`'s `test-support` feature, enabled by the monolith as a DEV-dependency only, so `prepare_and_finalize_for_test` and `insert_prepared` are absent from every production build. Not one fixture was rewritten and not one test was lost — monolith 1,267, characters 482, app 337, 26/26 contracts. ⚠ **`rollback_coverage` went red and it was RIGHT to**: its resource waiver is keyed on a full type path, and `PreparedCharacterRegistry` changed crates. The path was corrected and the reason left alone — a rename-tracking guard catching a rename is the guard working. ⇥ ▢ **what remains of P1.7**: `definition_tests.rs` (25 uses) tests PREPARATION and should follow it down; it works today through the `test-support` feature, which is correct but leaves preparation's own tests one crate above preparation. ⇥ ✔ **THE REMAINDER LANDED (2026-08-12): `definition_tests.rs` SPLIT BY WHAT EACH TEST TESTS.** 19 preparation tests + the fixture builders (824 lines) are `ambition_characters::prepared_tests`; the 8 that drive an `App` through `try_register_character` + `finalize` stayed, because those test COMPOSITION. The three builders would otherwise have been copied into both crates and drifted into two different characters sharing a name, so they have one home behind the same `test-support` feature — they build authored data and have no barrier to bypass, which `prepared_fixtures.rs` says out loud. characters 483 → 502. ⇥ ⛔⛔ **AND THE BARRIER THIS ROW DESIGNED DID NOT HOLD — the claim was measured on the FIELD and made about the OPERATION** (GPT 5.6 review, priority 2). `StagedCharacter` was opaque, minted by a `pub fn prepare_for_registration` and consumed by a `pub fn finalize_cast`; an opaque field prevents nothing when both ends of the pipe are public, and `finalize_cast([prepare(..).staged], whatever_catalog_exists_right_now, ..)` is ordinary safe code — exactly the early fold `CharacterPreparationPlugin` exists to remove. ⇒ **the lifecycle went DOWN beside the fold rather than the fold coming up**: `stage_authored_character`, `StagedCharacterOverrides`, `CharacterPreparationPlugin` and its barrier are `ambition_characters::prepared` now, with `StagedCharacter`, `StagedRegistration`, `prepare_for_registration` and `finalize_cast` all PRIVATE. What crosses the crate boundary is a CONTRIBUTION and a finished READ; there is no route to a staged value outside that module at all. The monolith keeps what `ambition_characters` must not know — the engine's baked sheet/portrait vocabularies, which come from `ambition_sprite_sheet`, which depends on it. `definition.rs` 357 → 169 lines. A 27th absence contract holds the line, falsified by re-publishing `finalize_cast`; `test-support` is untouched and unnamed by it. ⇒ ✔ **P1.7 IS DONE.** |
| P1.8 | Make PreparedCharacterDefinition complete for intrinsic construction | ◐ **capabilities, LOCOMOTION and CONTACT DAMAGE all authorable now** — `abilities` (verbs), `locomotion` (run speed, gait, surface cling, cling-breaks) and `contact_damage` (strength, amount), each `deny_unknown_fields`. A character can finally state how fast it is and whether touching it hurts, which is what a body needed the enemy archetype for. ▢ remaining before an enemy can be built character-first: melee/ranged action specs, mass, held item, respawn (placement) |
| P1.9 | Route authored enemy through character-first construction | ◐ **the road EXISTS and two characters take it.** A placement naming a COMPLETE character (one that states its locomotion) is built by `new_character_in` with no archetype, and wears itself so the persona derive writes its kit. ▢ every other enemy is still half-migrated and takes the legacy road with `adopt_character_intrinsics` patching over it — which is now that seam's only remaining job |
| P1.10 | Route NPC through the same body constructor | ✔ **the NPC road asks the CHARACTER whether it flies now, which is the first fact it ever took back from the catalog** (2026-08-12). ⛔ the defect was named in `ArchetypeSpec::flies`'s own doc and nobody had closed it: *two* spawn paths decided aerial-ness and NEITHER asked the character — this one read `body_kind: Floating`, the hostile path read the archetype — so D89's ruling (`body_kind` describes a SHAPE and stopped deciding flight) had reached only half the game. `CharacterLocomotion::baseline_free_flight` is `Option<bool>` so a character can refuse flight OUT LOUD, which a body kind cannot express; when it speaks it wins, and silence leaves the catalog rule exactly where it was. Poison-verified in both directions — a silent character re-deciding ~150 unmigrated placements would have looked identical to correct. ⇥ **▢ the rest, SIZED: 163 NPC placements name 129 distinct characters and TWELVE of them name a migrated one** (`goblin`, `npc_ai_slop`, both mites, the shark, the giant + its hands, `npc_lab_raider`, `npc_puppy_slug`, `npc_salvage_guard`, `sandbag`, `stochastic_parrot`). Those twelve still get `max_health: 1`, `NPC_PATROL_SPEED` and `MAX_RUN_SPEED` from the seed's hardcoded tuning rather than their own blueprint — that is the next slice, and it changes twelve bodies, not a hundred. ⇥ ⭐⭐ **AND THE TWELVE NOW GET THEIR OWN BODY** (2026-08-12): `max_health` and `max_run_speed` come from the character's blueprint when it has one. Every NPC used to spawn at 1 HP with the shared player `MAX_RUN_SPEED` — an exploding mite and a burning flying shark standing in a room with one hit point and the protagonist's legs, because nothing on this road knew who they were. ⛔⛔ the health POOL was a SECOND literal `1`, written independently of the tuning's; they agreed by coincidence, and teaching only the tuning to ask would have left a body claiming a maximum of nine and holding one. Poison-verified. ⛔ `patrol_speed`/`chase_speed` stay put (controller policy — how fast a body CAN move is its own fact, how fast it AMBLES is not) and so does `respawn: DeadStaysDead` (placement policy, ADR 0022). ▢ what remains on this row is the ROAD itself: an NPC naming a complete character should be built by `new_character_in` like the enemy road, not by a peaceful seed that patches two fields. ⇥ ⭐⭐ **AND THE ROAD ITSELF, 2026-08-12 — this row is DONE.** A placement naming a body-complete character is now built by `new_character_in`, the SAME constructor the authored-enemy road and the match seat use; what stays behind is only what is genuinely about the PLACEMENT (peaceful hostility, the interactable's patrol path, the sheet id, hurt feedback). ⛔ patching two fields was never the finish line and the fields it did NOT patch say why: this road handed every body `AbilitySet::NONE`, `CombatCapabilities::default()`, no contact damage, no `surface_walker`, no `ranged_visual` and a default brain profile — so an exploding mite standing in a room could not explode, a crawler did not cling, and an authored projectile came out unadorned. ⚠ the twelve now amble at `run_speed × patrol_effort` instead of the shared `NPC_PATROL_SPEED`; my own regression caught that and its EXPECTATION was the stale half, not its principle — a fixed constant is what a body gets when nobody knows who it is. |
| P1.11 | Route PreparedMatch through it immediately after | ✔ **A SEAT'S BODY IS BUILT FROM ITS CHARACTER.** `ActorClusterSeed::new_fighter_in` takes no roster: size and art from the character's sprite, health and weight from its definition, aerial-ness from its catalog body kind, abilities from the ruleset mask, and the CPU's `BrainProfile` handed in as a VALUE (`CharacterRoster::brain_profile_for`) rather than resolved by building a creature. Every fighter on the grid used to be physically a `combatant` wearing a character |
| P1.12 | Route encounter, summon, programmatic paths | ◐ **the ENCOUNTER road is character-first**: a wave naming a complete character builds from it and wears it, and the prepared cast is threaded through the encounter system to get there. Forced by a deletion — Mary-O's stomp fixtures spawn through this road, and a snake whose row was gone came out as a `combatant`. ▢ summons. **Programmatic staged actors are WIRED and can NAME a character** (`SpawnActorKind::Enemy { character }`) — that seam passed `&Default::default()` with a note calling it a gap, and it behaved as a lie: every runtime-staged actor was told no character authors anything, so a migrated creature spawned at runtime came out as its archetype |
| P2.13 | Migrate clean Group-A character/archetype cases | ◐ **eleven migrated: both mites, the PUPPY SLUG, the SKY PARROT, Mary-O's SOLID SNAKE and AI SLOP, Sanic's BADNIK, both PLANE SWARMS, the Hall's AI SLOP, and the BURNING FLYING SHARK — the first MOUNT** (the swarms are Ambition's characters registered by Ambition — registering them from the demo that PLACES them made the registry and the catalog's owners map disagree about who authored them, which a guard refuses by name) (twenty placements that already named them, so no LDtk work — their roster rows were the whole gap) (its ten placements across `sandbox.ldtk` and `intro.ldtk` now author `character_id` + `disposition: Peaceful`, and its six archetype pins MOVED to a test beside the definition rather than being deleted) — health, run speed, gait, contact damage, the swipe, the death blast and the Smash policy are all on their definitions, split across the three authorities. ▢ the remaining seven Group-A characters ⛔ **that census was STALE — re-measured 2026-08-11 across every `.ldtk`**: puppy_slug (10), burning_flying_shark (7), sky_parrot (2) and ai_slop (1) ALREADY name their `character_id`. What is actually left unnamed is `pirate_shark_rider` (6), `pirate_heavy_shark_rider`/Iron Mary (1) and `giant_gnu` (1). ⇥ **THE GIANT IS MIGRATED AND ITS ROW IS DELETED** (`35a80b485`, 707 → 678 lines) — it took three layers learning *ask the character, then the archetype*: the limbed-host predicate, the activation path's construction context (which never handed planning a cast: `prepared=None` → `Some(35)`), and `mount_capabilities_of`. ⇥ **AND BOTH SHARK RIDERS FOLLOWED** (`a5c3812fe`): `npc_pirate_raider` + `npc_pirate_heavy_iron_mary`, seven placements named, both rows deleted — they needed `BrainProfile.patrol_effort` (0.4783/0.5116 are TUNED), `CharacterDefinition.held_item` (a raider without its gun-sword is not a raider) and a placement-authored `respawn`. ⇥ **AND THE GIANT'S HANDS** (`659699b82`) — one character, two bodies, spawned by the rig from a single definition; `populate_giant_hand_into` was still on the archetype road while the host beside it was character-first. ⭐ **`character_archetypes.ron`: 843 → 601 lines, 13 rows left** (the FINITE SANDBAG went too — it had been a playable character and an enemy archetype row at the same time). Group A has NO unnamed placements left; the next migration is sized in ledger **D77** (the sandbags, blocked only on `is_sandbag` having no authoring surface) ⇥ ⭐ **THE CENSUS IS A TEST NOW, AND MY ONE-OFF MEASUREMENT WAS WRONG BY SIX** (2026-08-12). `the_body_complete_cast_only_grows` builds every id in `buildable_cast()` through `authored_intrinsics` and counts the ones that state their own locomotion: **NINETEEN**. A regex over the match arms had said thirteen — it cannot see nested braces, so it reported both shark riders, the giant's hands, the salvage guard and the lab raider as unmigrated, every one of them done. ⇒ the test asserts the count only moves UP (a migration does not remove completeness) with a CONTROL that it is not yet everybody, so the criterion cannot silently start answering true for all. ⚠ it asks *does this character state its locomotion* — the campaign's own words — rather than mirroring `body_blueprint`, whose test entry point is `#[cfg(test)]` inside the monolith and invisible from content. |
| P2.14 | Delete each migrated legacy row as it becomes unnecessary | ◐ **136 lines out of `character_archetypes.ron`** (843 → 707), and a guard asserts the three migrated creatures have no row left — with a control so it cannot pass on an empty file the moment the mites could carry their own bodies. The rule held: the deletion landed in the same change as the migration |
| P2.15 | Extract Group-B shared AI behavior into real BrainProfiles | ◐ **the TYPE exists** — `ambition_characters::brain::BrainProfile`, authorable, `deny_unknown_fields`, replacing `CharacterBrainSpec` outright and taking `aggro_radius`/`attack_range`/`turns_at_walls` off `ActorTuning`. ⇥ **A CHARACTER CAN NAME ONE NOW** (`8d112cf99`): `CharacterCatalogData.autonomous_profiles`, namespaced per provider, referenced by `CharacterDefinition::autonomous_profile_ref` and resolved at preparation into the existing value (inline wins). ⭐ FIRST ADOPTER: the GOBLIN (`787165763`) — `medium_striker`'s controller half is a shared policy and its five sandbox placements are goblins with their own bodies instead of generic strikers. ⚠ the `BrainPreset` fork is deliberately untouched: a preset authors ABSOLUTE speeds and a profile authors effort, so merging them needs the body a preset does not know. ▢ the remaining Group-C roles (`small_skitter`, `large_brute`, `gradient_seeker`, `ranged_skirmisher`) are the same recipe, now placement work rather than architecture ⇥ ⭐ **SECOND ADOPTER, 2026-08-12: the LAB RAIDER.** `medium_striker` was a shared policy with ONE creature pointing at it, which is that creature's private profile wearing a general name — `npc_lab_raider` authors its body (5 HP, 170 run, 0.70/1 contact; the goblin's numbers, because the archetype gave both the same ones) and NAMES the policy, so the indirection is now doing the job it was introduced for. Its intro placement names the character. |
| P2.16 | Classify Group-C generic roles | ◐ **the classification is DECIDED and demonstrated**: a generic role splits into a shared `autonomous_profiles` entry (the reusable POLICY) plus whichever character the placement actually is (the BODY). Proven on `medium_striker` → the goblin. ▢ four roles left, plus a product question only Jon can answer: the sandbox `0140-0146` block is a DEMO ROW of one-of-each archetype (`patrol cutter`, `small skitter`, `guard striker`, `medium striker`, `gradient seeker`, `large brute`) with NO creature identity — either they get characters invented for them or the demo row goes ⇥ ⭐ **`gradient_seeker` IS DELETED** (2026-08-12) — and it taught the rule a third time: an archetype whose ENTIRE population is one placement was never a role, it was that creature's body filed under a different name. Its one spawn is literally called *Salvage Guard*, so `npc_salvage_guard` authors the body (4 HP, 225 run, 0.80/1 contact, the swipe), an INLINE `BrainProfile` carries the controller half, and the placement names the character. ⚠ inline and NOT a shared `autonomous_profiles` entry, which is the P2.16 rule rather than an inconsistency: one adopter does not earn the indirection, and publishing a shared policy nobody shares leaves a second empty role behind exactly like the one being removed. ⭐ **`character_archetypes.ron`: 284 → 264 lines, THREE rows left** (`combatant`, `medium_striker`, `sandbag_infinite`). ▢ three roles left, and the sandbox `0140-0146` demo row is still Jon's product question. ⇥ **THE REMAINING ROWS ARE SIZED, AND IT IS FOUR PLACEMENTS** (measured 2026-08-12 across every `.ldtk`, deduped through the symlinks): of the 20 placements still wearing the three surviving rows, **16 already name a character** — their key is read only for what the character does not state. The four that do not: `Lab Raider` ✔ (done, above), `under_town_skitter` (`medium_striker`), and the two `sandbag_infinite` dummies. ⛔ so `medium_striker` is blocked on ONE spawn whose creature identity is a product question — `under_town_skitter` is a place plus a movement style, not a creature — and `sandbag_infinite` is blocked on `is_sandbag` having no authoring surface (ledger D77). Neither is migration work; both are decisions. ⇥ ⭐ **`sandbag_infinite` IS DELETED TOO** (2026-08-12), and D77's stated blocker was STALE — it said `is_sandbag` had no authoring surface, and `CharacterDefinition::practice_target` (doc-aliased `is_sandbag`) has been that surface for a while, with `ActorClusterSeed` already writing `is_sandbag: practice_target`. **Grepping for the thing the row said was missing is what found it.** The immortal dummy is a SECOND CREATURE rather than a flag on `sandbag`, because `never_dies` is a `CharacterDeathTraits` field: *the same dummy, invincible in this room* is not expressible, and faking it would put a mortality policy on a spawn point. ⇥ its shipped row moved into the ENGINE's fixture, the same answer the pirates and the shark got — the machinery tests need the SHAPE, not Ambition's dummy. ⭐ **`character_archetypes.ron`: 264 → 244 lines, TWO rows left** (`combatant`, `medium_striker`), and `medium_striker` is blocked on ONE spawn (`under_town_skitter`) whose creature identity is Jon's call. |
| P2.15b | Group-B pacing + hostility become PROFILE facts | ✔ **`BrainProfile` gained `patrol_effort`, `chase_effort` and `attacks_player`**, and `new_character_in` consumes them — it wrote `run_speed * 0.5`, `run_speed` and `true` as LITERALS, so every character-first body ambled at half pace and hunted the player whatever its row said. That is why `pirate_shark_rider` (0.4783), `medium_striker` (0.44) and `giant_gnu` (`attacks_player: false`) could not migrate. The archetype projection hands them over, so unmigrated creatures are unchanged; the defaults are the old literals, so fighter seats are unchanged. Two more bugs fell out: a migrated character's `attacks_player` was re-clobbered from the `combatant` fallback one line after construction, and an authored `run_speed: 0.0` was read as *said nothing* and answered with the stage's sprinter default |
| P2.17 | Migrate provider roster fragments | ◐ **SANIC'S IS GONE and MARY-O'S IS DOWN TO ONE ROW-PAIR.** The badnik, Solid Snake and AI Slop are complete characters whose placements name them; their fragments are deleted and their `respawn` moved to the placements (each world's `EnemySpawn` gained the `respawn` fieldDef the parser has read all along). ⚠ each deletion had the SAME one-field trap — the rows authored `patrol_effort: 1.0` and `BrainProfile` defaults to a half-speed amble, so deleting without moving that number would have silently halved every enemy in both demos. ⇥ **AND THE LAST FRAGMENT IS JON'S DOCUMENTED EXCEPTION, not an open item.** Mary-O's plane-swarm rows survive because the swarms are genuinely SHARED: Ambition's Hall EXHIBITS both (`hall_of_characters.ldtk` places `npc_snakes_on_a_paper_plane` and `npc_snakes_on_a_cartesian_plane`) and Mary-O PLACES both, so their catalog rows cannot move to Mary-O without emptying two Hall pedestals, and Mary-O cannot register them itself without the registry and the catalog owners map disagreeing about who authored them (`the_shipped_cast_has_one_authority_per_character` refuses it by name). That is exactly the brief's clause: *"do not force deletion if the standalone packaging boundary genuinely cannot import the content provider cleanly, but document the exact dependency reason"* — the reason is a shared character with one catalog home and two consumers, and the fragment is the standalone fallback |
| P2.18 | Delete CharacterRoster/ArchetypeSpec infrastructure | ◐ **the census is now the COMPILER's, and my published number was wrong by more than half.** P2.22's row said `spec_for_brain` has *"production callers in `autonomous_reconcile` (2), `actor_clusters` (2), `brain_builders` (1) and `spawn_actors` (2)"* — seven. That was a regex over Rust, which counts prose and misses call sites, and [[feedback_ask_the_tool_dont_model_it]] names exactly this failure. Marking the function `#[deprecated]` and reading `cargo check --lib` back gives the real list: **17**, in `construction` (6), `spawn_actors` (5), `autonomous_reconcile` (2), `actor_clusters` (1 — `new_in`, the archetype road itself), `brain_builders` (1), `actors/conversion` (1) and `enemies/mod` (1). ⇥ ⭐ **three are gone or converted so far** (2026-08-12): the peaceful-NPC seed's fabricated inert `spec` (which also took the `roster` parameter off four functions and a `Res<CharacterRoster>` off a content system); the staged-actor mount plan, which now asks the CHARACTER exactly as the authored arm does; and `enemy_spawn_is_sandbag`, a `pub fn` re-exported from `features/mod.rs` with **zero callers anywhere in the workspace** — dead public API holding a live reference to the roster. ⇥ ▢ the remainder splits cleanly by who is blocking: `construction`+`spawn_actors`+`enemies` ask PREDICATES (limbed host, mount class, sandbag) that all have a character-first form already, so they convert as their placements migrate; `autonomous_reconcile`+`conversion` are the PROVOCATION road, which is P2.20 and sized at ~144 Hall characters; and `actor_clusters::new_in` is the archetype road itself, which goes last and takes the two remaining `character_archetypes.ron` rows with it ⇥ ✔ **THE CONTROLLER-POLICY AUTHORITY IS OUT OF THE VERSUS STAGE (2026-08-13).** `VERSUS_ROSTER_RON` is DELETED — one `ArchetypeSpec` row registered as a `CharacterRosterFragment` for exactly one lookup: a CPU seat naming `versus_duelist`, answered through an ENEMY ARCHETYPE TABLE, so the controller half of `character + controller + team` arrived by way of a body definition. Its controller half is an `autonomous_profiles` entry in `VERSUS_CATALOG_RON` now, the same migration ledger D87 made for Smash's six rows. ⚠ **its body half went nowhere because nothing read it**: `max_health`, `run_speed`, `melee`, `move_style` and `respawn` stopped being read the day a seat was built from its CHARACTER (P1.11) — the `fighter_abilities` note in `versus.rs` is the record of exactly that, since the row's authored `melee` *"reached the body regardless of what the match said the body could do"*, and removing it is what exposed this stage's missing `attack` verb. ⇥ ⛔⛔ **AND IT WENT RED IN THE SDK GUARD, WHICH WAS THE INSTRUMENT'S BLIND SPOT RATHER THAN THE CHANGE'S FAULT.** `the_versus_cpu_roster_is_satisfiable_by_the_sdk_composition` called `unsatisfiable_seats(&archetypes, None)` — asking only the archetype table, when that function's own doc says a seat's answer can live in either of two authorities. It reported a seat unsatisfiable that the shipped composition satisfies perfectly. The CLAIM is unchanged and still the one that shipped a statue; it is now asked of both places. ⚠ falsified by putting the original defect back (`VERSUS_CPU_BRAIN = "medium_striker"`), which turns it red with its own message. ⇥ ▢ **what this leaves**: `seat_brain_profile`'s archetype arm has NO production caller — Smash resolves `smash::duelist*`, versus resolves `ambition_versus::versus_duelist`, and the only remaining `brain_profile: Some(<archetype key>)` seats in the workspace are TEST fixtures (`"combatant"`, `"medium_striker"`, `"aggressive"`, `"no_such_archetype"` in `staging.rs`, `prepared_match/tests.rs`, `input_systems.rs`, `rollback_match_activation.rs`). Deleting the arm removes CONTROLLER POLICY from `CharacterRoster` entirely — one of the three fused authorities — and costs the repair of those fixtures, which is the next step of this row. ⇥ ✔✔ **AND THE ARM IS DELETED (2026-08-13): `CharacterRoster` IS NO LONGER A CONTROLLER-POLICY AUTHORITY.** `seat_brain_profile` lost its `archetypes` parameter; a CPU seat's policy comes from the published registry or the seat is refused. That is one of the THREE fused authorities Jon's brief names, gone from the roster type — the remaining two are intrinsic body and placement policy. ⇥ ⚠ **34 monolith tests went red and every one was a FIXTURE seating through a road production no longer has**, which is what the arm was hiding: they named `combatant`, an enemy archetype row. Repaired by publishing a policy the way a composition publishes one — `fixture_policies()` keys `provider::cpu_policy` for each of the four providers those fixtures register characters under, with `CONTENT_FREE_ROSTER_RON`'s `combatant` controller half verbatim (StandStill, every radius and effort zero) so migrating the AUTHORITY does not quietly retune what the fixtures seat. ⚠ a fifth provider seats zero bodies and says so loudly — `four_fighters_on_two_teams` caught exactly that on `arena`, which is the failure this list is allowed to have and not a silent one. ⇥ ⚠ `puppy_slug_forced_seat` was the same defect in `ambition_content` and took the same repair: it names `medium_striker`, Ambition's own PUBLISHED policy from the catalog it registers, instead of an archetype row. ⇥ ⚠ **the preference test lost its subject and gained a clause**: `a_cpu_seat_prefers_a_published_policy_over_an_archetype_of_the_same_name` asserted *"the legacy road is still open, which is what makes the preference above a preference rather than a replacement"* — it is a replacement now, so it is `a_cpu_seats_policy_resolves_in_a_provider_or_not_at_all`, keeping the provider-resolution claim and its bare-key poison and adding the one the arm used to hide: an archetype key resolves to NOTHING. ⚠ falsified by naming `combatant` again — 34 red. ⇥ ✔ **AND THE MATCH ROAD NO LONGER TAKES A ROSTER AT ALL (2026-08-13).** Deleting `seat_brain_profile`'s arm left FOUR readers still consulting the archetype table about a controller question, and each was now a validator MORE PERMISSIVE than the thing it validates — the shape `content_schema` already names: *"THE COMPILER MUST NOT APPROVE WHAT THE RUNTIME REFUSES"*. ⇥ (1) `unsatisfiable_seats` returned `None` — satisfiable — for any seat whose key was in the roster, and seating would then refuse it; its `archetypes` parameter is gone, and the diagnostic lists published policies instead of *"Known archetype keys"*. (2) `activate_if_seatable` lost the parameter with it, and versus's activation collapsed from two arms to one: it branched on whether an archetype table EXISTED and activated UNVALIDATED when it did not, which was defensible while a policy had two possible homes ("no archetype table" ≠ "no answer") and is not now. (3) `prepare_match` and `prepare_the_match` lost the roster entirely — it was a REQUIRED `Res`, so every host that prepared a match had to install an enemy archetype table to seat a fighter. (4) the seat-refusal message stopped printing *"Archetype keys: […]. ⚠ the archetype table is the LEGACY half — a seat should name a published policy"*, which now sends a reader to add a row that would change nothing. ⇥ ⚠ **and two GUARDS were reading the dead authority**, which is the more useful finding: `the_stage_kills`'s ladder check was `archetypes.has_brain_key(profile) || published.get(..)`, and that first term made it unfalsifiable in the direction that matters — an archetype table could answer for a rung whose policy was never published, which is exactly the state D87's deletion was supposed to have ended. The staging fixtures built a `CharacterRoster` to validate seats and now build a `BrainProfileRegistry`. |
| P2.19 | Split/delete ActorTuning | ◐ **RE-MEASURED 2026-08-12, and both numbers this row inherited are wrong.** The acceptance list sizes `ActorTuning` at 275 lines; it is **199**. D73 item 21 says *"the capability-authored-twice set is exactly `can_blink`/`can_fly`/`can_shield` vs `smash_can_*`"*; **none of those fields is on `ActorTuning` any more** — `BrainProfile` took the controller half and `AbilitySet` the capability half, so the duplication that item names is already gone and its ⚠ now points at nothing. ⇥ ⛔ **there is no free deletion left in it: every one of its twenty fields has at least one reader.** ⚠ and getting that right took two censuses — a `tuning.field` regex reported `contact_strength` as dead, and it is READ, at `integration.rs:436`, split across four lines by rustfmt. [[feedback_ask_the_tool_dont_model_it]] for the third time this run: **a regex over Rust cannot see a field access rustfmt wrapped.** ⇥ ▢ so this row is a real SPLIT, not a cleanup — the twenty fields divide as body (13), controller (3: `patrol_speed`, `chase_speed`, `attack_cooldown_mult`) and placement/session (4: `respawn`, `death_policy`, `is_sandbag`, `attacks_player`) — and it lands when the three authorities can each hold their share ⇥ ✔ **AND THE SPLIT'S FIRST REAL SLICE LANDED 2026-08-13 — a BODY half that two roads answered differently.** `peaceful_config` is the projection a body takes when it is released back to peaceful, and its own doc claims it installs *"the same undescribed-body pool the seed this mirrors installs"*. It did not: `new_peaceful_npc_in` reads the PREPARED character's blueprint for `max_health` and `max_run_speed` (P1.10) and falls back to the constants only for a body nobody authored, while this hard-coded both. So a character-first body that calmed down would have been handed the shared PLAYER top speed and the undescribed pool — the §2 `ProvokedArchetype` defect running in reverse, a silent body downgrade wearing a controller change. ⇥ ⚠ **UNREACHABLE, and measured that way rather than argued** — the same shape as the `is_aerial` trap this module already documents ten lines above, which is what made it worth looking for. `apply_catalog_mode` returns before the call when a character states its own POLICY, and all seventeen `ambition_content` characters that author locomotion also author a profile, so the population is EMPTY today. ⛔ but the guard keys on the character's POLICY while the downgrade is to its BODY, so a character authoring locomotion and no profile falls straight through it — an ordinary thing to author. Closed at the PROJECTION, which is the layer that can see the character. ⇥ ⚠ `patrol_speed`/`chase_speed` stay flat in both roads deliberately and the note now says why: how fast a body AMBLES is the controller's fact. ⚠ two terms in the regression — the character's numbers survive AND a body nobody authored still gets the shared defaults, so "reads the blueprint" cannot be satisfied by a projection that reads it for everything. ⚠ and `apply_catalog_mode`'s own guard comment still said `max_health: 1`, a premise D101 deleted. ⇥ ✔ **AND THE PLACEMENT FOUR LOST THEIR PLAYER-CENTRIC ONE (2026-08-13).** `attacks_player` is `hostile_by_default` on `ArchetypeSpec`, `is_hostile` on `ActorTuning`, and `SpawnDisposition::attacks_player()` is `is_hostile()`. ⭐ **the same name Jon deleted from `BrainProfile` on 2026-08-11** as *"player-centric vocabulary in the one type that must never be"* — it survived there because it is a PLACEMENT fact rather than a policy, and a placement may perfectly well say *this creature is hostile*; what it may not say is who it is hostile TO. Who a hostile body attacks has its own authority (`ActorFaction`, `MatchTeam`, `damage_lands_between`), and a CPU-versus-CPU match is full of bodies that attack on sight with no player in it. ⚠ no authored row sets the field — the only mention in `character_archetypes.ron` is a doc comment — and a serde `alias` keeps an out-of-repo provider's roster parsing. ⇥ ⛔⛔ **TWO CENSUS FAILURES IN ONE SLICE, BOTH CAUGHT BY SOMETHING OTHER THAN ME.** (a) a grep filtered on `attacks_player[:=]` reported the `ArchetypeSpec` field as having only a test reader; the COMPILER found `attacks_player: self.attacks_player` in `enemies/mod.rs`, the projection that makes it live. [[feedback_ask_the_tool_dont_model_it]], third time this run. (b) renaming `attacks_player:` → `is_hostile:` across `*.rs` rewrote SIX RON rows inside `r#"…"#` literals, and 237 tests went red on `Unexpected field named `is_hostile``. [[reference_ron_in_rust_literals_escape_type_changes]] names exactly this; the fix was to classify every occurrence by whether it fell inside a raw string before touching it. |
| P2.20 | Remove hostile/provocation body reconstruction | ◐ **SIZED, and the design is already half-built** (2026-08-12). Provocation has TWO arms: a character that states a provoked policy records `AutonomousSource::ProvokedProfile` and keeps its body; everything else takes `hostile_brain_id_for_actor()` + `roster.spec_for_brain` and is REBUILT as an archetype. ⇥ **eleven characters state one** — the nine pirates (by a rule, reproducing the deleted matcher's own heavy/light split) and both cellular automatons. Every other provokable body — the Hall cast and the exploration NPCs — takes the legacy arm. ⇥ ⭐ **what deletes it**: `BrainProfile::default()` already exists and is already what a character-first body with no policy is paced against, so the provoked BRAIN needs no roster. The open question is the KIT: `enemy_combat_kit_for_spec(&hostile_spec)` reads the archetype for it, and the character's own `ActionSet` is the answer that road already uses elsewhere. ⛔ NOT attempted blind: it changes what provocation produces for ~144 Hall characters, and the ~100-NPC regression this campaign already paid for came from exactly that shape of blanket change. ⇥ ⭐⭐ **THIS ROW AND P2.20 ARE THE SAME MISSING AUTHORITY** (found 2026-08-12 by measuring both). `smash_fighter_kit()` grants one generic swipe to a seated fighter whose character says nothing; the PROVOCATION path hands a peaceful body a whole archetype for the same reason — a Hall NPC authors `peaceful`, so a provoked one would have nothing to swing. Two spellings of *a default fighting kit*, and neither could be deleted while the concept had no name. ⇒ it has one: `brain_builders::default_fighting_kit()`, with a test asserting it EQUALS what the provocation fallback builds from `combatant` (plus a control that the row authors a melee at all, so the comparison cannot be vacuous). The NPC spawn road consumes it instead of the roster. ⇥ ⛔⛔ **AND THEY MUST NOT BE MERGED — the numbers decided it** (2026-08-12). Side by side: the stage's floor is `0.22/0.08/0.26`, 4 damage, 34 reach; exploration's is `0.28/0.08/0.32`, 1 damage, 28 reach. Faster, harder, longer — a platform fighter's floor against an exploration provoke, and correct that they differ. Unifying them would RETUNE a mode while wearing a refactor's commit. ⇒ ⭐ **what the pair proves is where this default belongs: the SESSION RULESET**, the campaign's third authority. A stage states what an unarmed fighter swings for; a room states something else; neither is a fact about a body. Naming it in the engine made the question askable and is not its final home. Both functions now carry the other's numbers in their docs, so a reader of either sees the pair. ⇥ ⭐⭐ **AND THE REMAINING WORK IS NOT ~144 CHARACTER EDITS** (re-sized 2026-08-12). The Hall has **129** `NpcSpawn` placements and this row has been carrying "author a provoked policy on each" as its shape. It is not: the pirates prove the pattern is a RULE (`id.starts_with("npc_pirate_")` → one of two published profiles, nine characters, zero arms), and everything that matches no rule falls to `hostile_brain_id_for_actor() → "combatant"`, which is **one fallback in one function**. ⇒ the work is to replace that ONE fallback with a declared default profile — the same move `unarmed_melee` just made into `DeclaredCombatRules` — after which the provocation road stops naming an archetype at all and `AutonomousSource::Provoked { archetype }` becomes `ProvokedProfile { profile }` for the whole cast. **That is also P2.21.** ⇥ ⛔ **one product question blocks it and it is Jon's**: the archetype road gives a provoked villager `combatant`'s BODY — 4 HP, 155 px/s — while the profile road leaves the body alone and changes only the mind, which is Jon's own model but means an unmigrated Hall NPC is provoked at **1 HP** and dies to one hit. Either that pool is a SESSION fact (the third authority, exactly like `unarmed_melee`) or the Hall cast needs vitals. ⇒ **ledger D96 item 7.** ⇥ ⭐⭐ **AND THE BLOCKER THIS ROW ENDS ON IS STALE — measured 2026-08-13.** The product question it defers to Jon is *"an unmigrated Hall NPC is provoked at 1 HP and dies to one hit"*. **D101 answered it**: the `1` was the defect, the number moved up to `ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH` (4) at the two SPAWN seeds, and provocation stopped writing health at all. An undescribed body is undescribed before anybody hits it. ⇥ ✔ **and the live provoke path already stopped asking the roster** — it builds from `brain_builders::default_provoked_policy()`. ⇒ **what actually remains is ONE read-model write**, and it is not decoration: provocation sets `config.brain = CharacterBrain::Custom("combatant")`, and `evaluate_enemy_ai_output` branched `Passive => aggro 0.0`, so the archetype NAME was standing in for *"this body is hostile now"*. A body that took the honest `Passive` read-model would have been provoked into never noticing anybody. ⇥ ✔ **HALF OF THAT IS DONE (2026-08-13): the notice radius has ONE authority.** The `Passive` arm is deleted — how far a body notices from is `BrainProfile::aggro_radius`, and the arm was a second answer through a SILHOUETTE read-model (written `Passive` for anything that is not a patrol brain, including a boss whose real mind is a `BossPattern`). ⚠ **inert by measurement rather than argument**: every production site that writes `Passive` writes `BrainProfile::default()` beside it, whose `aggro_radius` is `0.0` — the peaceful NPC seed, the boss config and the reconcile projection all do. Falsified by restoring the arm. ⇥ ▢ **the other half is `patrol_enabled`**, `!is_sandbag && !matches!(brain, Passive)` in the same function — the same co-authority, one flag over, and the reason the test asserts `!= Chase` rather than a named idle mode (asserting `Patrol` would have pinned the coupling it is about removing). ⇒ when that flag reads a fact instead of the silhouette, provocation writes `config_brain_for(brain)` like the reconcile road already does, `hostile_brain_id_for_actor()` and its test-only `hostile_spec_for_actor` twin are deleted, and the provoke road names no roster key anywhere. ⇥ ✔✔ **DONE 2026-08-13: THE PROVOKE ROAD NAMES NO ROSTER KEY.** `hostile_brain_id_for_actor()` and its test-only twin `hostile_spec_for_actor` are deleted; `provoked_projection` lost its `archetype: &str` parameter and derives the read-model with `config_brain_for` like every other road. ⛔⛔ **AND I HAD THE CONSEQUENCE WRONG IN THE FIRST HALF OF THIS WORK, which the consumer census corrected**: `evaluate_enemy_ai_output`'s output reaches `ActorStatus::ai_mode` and NOTHING ELSE, and `ai_mode`'s only readers are `ActorIntent` — whose own doc says it exists *"so rendering and HUD systems can branch on actor state"* — and the rollback snapshot. `is_dangerous()` has no gameplay caller. So the `Passive` arm never stopped a provoked body chasing; the BRAIN decides that, from the same `BrainProfile`. It made the HUD say `Idle` about a body its brain was chasing with. ⇒ still the same defect (two authorities over one number, the presentation copy free to disagree) and still worth deleting — but a presentation bug, and the census is what turned a feared *"~144 NPC behaviour change"* into a change that alters no creature's behaviour at all. ⇥ ✔ `patrol_enabled` went the same way: `!matches!(brain, Passive)` → `profile.patrol_effort > 0.0`, which is the fact the field's own doc names (*"has a path or a non-zero patrol speed"*). `is_sandbag` stays — a practice target holds still because of what its BODY is. ⇥ ⚠ **and one test was PINNING the defect**: `provocation_changes_the_mind_and_leaves_every_body_fact_alone` asserted the provoked read-model is `!= Passive`, which was the archetype name's fingerprint — it could not have passed without the roster key. Inverted to the invariant that now matters: the read-model may never be `Custom(_)`, and must equal what `config_brain_for` derives from the actual brain. ⇒ **P2.20 is ✔ for its archetype dependence.** What the row still names beyond that is the ~129 Hall placements' provoked POLICY, which is content authoring, not this coupling. |
| P2.21 | Remove rollback dependence on archetype identity | ✔ **DONE — the variant is payloadless; see the ⇒ at the end of this row. It was ONE enum variant, and it is P2.20's** (measured 2026-08-12). The dependence is `AutonomousSource::Provoked { archetype: HostileArchetypeId }` — a roster key in the rollback codec, rebuilt by rerunning archetype construction. Its character-first twin `ProvokedProfile { profile }` **already exists, is already snapshotted, and is already what nine pirates and the PCA use**; it names a published profile instead of a creature and rebuilds only the mind. ⇒ this row needs no design of its own: it lands when P2.20 replaces the `combatant` provocation fallback, and the variant is deleted with the last body that stores it ⇥ ✔✔ **AND IT HAS ALREADY LANDED — grepped 2026-08-13 before working it, which is the rule this row would otherwise have cost another session to.** `AutonomousSource::Provoked { archetype: HostileArchetypeId }` does not exist: the variant is `ProvokedDefault`, PAYLOADLESS, and its own doc explains why — *"the id was always the same string… carrying it made the rollback road resolve a roster the live road had stopped consulting"*. The rollback codec encodes it as a bare tag (`snapshot_impls.rs`, `put_u8(out, 2)`), so no roster key crosses the wire. ⇒ **P2.21 is ✔**, and it did not wait for P2.20 as this row predicted — it landed with D89/D84 when the last matcher arms went. |
| P2.22 | Delete `character_archetypes.ron` | ◐ **THE CONTENT SIDE IS TWO PLACEMENTS** (measured 2026-08-12 against the CENSUS, not a regex). Across all four of Ambition's worlds, exactly **two** `EnemySpawn`s still need the archetype road: `under_town_pipes`/`under_town_skitter` (`medium_striker`, no character) and `dive_drill`/"Target" (`brain: Passive`, no character). Every other enemy placement in the game names one of the nineteen body-complete characters and is built character-first. ⛔ both are CONTENT DECISIONS, not migration work — a skitter is a place plus a movement style, and a thing called "Target" in a dive-drill room is plausibly the `sandbag` character but that changes the drill. ⇥ ▢ **the CODE side is the larger half and it is sized**: `spec_for_brain` still has production callers in `autonomous_reconcile` (2), `actor_clusters` (2 — including the peaceful-NPC seed's inert `spec`), `brain_builders` (1) and `spawn_actors` (2). `combatant` cannot go until those do, and they are P2.18/P2.19/P2.20 rather than this row. |
| P3.23 | Move Robot v3 off HostCode to normal character data | ✔ **DONE, and most of it was ALREADY DONE when this row was read** (2026-08-12). ⛔ grepping first: `playable_kit: HostCode` has ZERO adopters in the shipped catalog — the only occurrence is a COMMENT — and `PlayableKitSource` no longer HAS a `HostCode` variant. The content half and the selector deletion had both landed; the ▢ was on finished work, which is the fourth time this campaign has found one. ⇒ what was left was a NAME: `PreparedKit::HostCode` was still called after the deleted selector, while what actually reaches it is "an id the catalog does not know, or no catalog at all" — nobody authored a kit. It is `PreparedKit::Unauthored` now, and the error message that told an author they had *"taken the host-code kit"* says they authored no action set instead. ⭐ a name that asserts a vanished selector sends the next reader looking for the row that chose it, and there is none — the stale-citation trap, in a type name rather than a comment. |
| P3.24 | Remove `smash_fighter_kit()` as the universal replacement | ◐ **it no longer replaces a real fighter's repertoire** — it is the action-set grant that lets a borrowed peaceful Hall NPC attack at all, and a character with authored moves now keeps them. ▢ the grant itself goes when those characters author their own ⇥ ⭐ **THE ADOPTER COUNT FELL FOR THE FIRST TIME** (2026-08-12): the GOBLIN authors eleven timelines — the third character in the game to state a table and the FIRST ENEMY to — so it no longer takes the floor. ⚠ the set is computed from `authored_moveset.is_some()`, so authoring is the whole wiring: nothing lists adopters by hand. ⚠ and it is NOT the robot's table renumbered — shorter reach, faster jab, softer kill, pinned by a test that compares the two, because a copied table would pass every other check in the file. ⇥ ⭐ **AND THE ADMIRAL, same day** — its row already said what its moves are (`default_action_set: "pirate_pistol"`, the roster comment "pistol + cutlass", `collision_scale: 1.6`), so the table is a long slow blade. ⭐ THE ORDERING IS NOW TESTED ACROSS ALL THREE: reach and startup order admiral > robot > goblin and the kill damage follows, asserted against the OTHER TABLES rather than literals — a comparative claim needs a comparative test, and pinning one table's numbers alone would go green on a table that had quietly become somebody else's. ⇥ ⭐ **AND THE PATENT CLERK, READ BACK OFF ITS OWN ROW.** Its `gameplay_description` already said *"a high-mastery heavyweight controller … turns careful observation into unusually strong parries and finishers"* — heavyweight, controller, finishers — and those three words ARE the table: slowest startups in the game, tilts that set up rather than kill, the hardest smashes. The design was written down and nobody had read it back. ⛔ the CLASSIFICATION mechanic (MASS/ENERGY/MOVING/AT REST, reference frames, the elevator recovery) is deliberately NOT in the moveset: those are systems, not swings. ⇥ ⛔⛔ **STARGAN IS NOT NEXT, AND THAT IS A PRODUCT ANSWER RATHER THAN A GAP.** His row says `default_action_set: "peaceful"`, `default_brain: "stand_still"`, and `tags: […, "peaceful"]` — a science communicator who does not fight. The floor grant is EXACTLY what this row exists to describe (*"the action-set grant that lets a borrowed peaceful Hall NPC attack at all"*), so his adoption of it is CORRECT until Jon decides whether Carl Stargan fights and how. Writing him a repertoire would be inventing combat for a pacifist. ⚠ **an earlier count of "two adopters left" stood here and was wrong by five** — it was written before anything measured the set, and the measured census further down this cell says SEVEN. A hand-written count beside a computed one is the hand-written one rotting; the census below is the only number in this row anybody should quote. ⇥ ⭐⭐ **THIS ROW AND P2.20 ARE THE SAME MISSING AUTHORITY** (found 2026-08-12 by measuring both). `smash_fighter_kit()` grants one generic swipe to a seated fighter whose character says nothing; the PROVOCATION path hands a peaceful body a whole archetype for the same reason — a Hall NPC authors `peaceful`, so a provoked one would have nothing to swing. Two spellings of *a default fighting kit*, and neither could be deleted while the concept had no name. ⇒ it has one: `brain_builders::default_fighting_kit()`, with a test asserting it EQUALS what the provocation fallback builds from `combatant` (plus a control that the row authors a melee at all, so the comparison cannot be vacuous). The NPC spawn road consumes it instead of the roster. ⇥ ⛔⛔ **AND THEY MUST NOT BE MERGED — the numbers decided it** (2026-08-12). Side by side: the stage's floor is `0.22/0.08/0.26`, 4 damage, 34 reach; exploration's is `0.28/0.08/0.32`, 1 damage, 28 reach. Faster, harder, longer — a platform fighter's floor against an exploration provoke, and correct that they differ. Unifying them would RETUNE a mode while wearing a refactor's commit. ⇒ ⭐ **what the pair proves is where this default belongs: the SESSION RULESET**, the campaign's third authority. A stage states what an unarmed fighter swings for; a room states something else; neither is a fact about a body. Naming it in the engine made the question askable and is not its final home. Both functions now carry the other's numbers in their docs, so a reader of either sees the pair. ⇥ ⭐⭐⭐ **`smash_fighter_kit()` IS DELETED** (2026-08-12) — 41 lines, and the concept did not die with it, it MOVED. `DeclaredCombatRules::unarmed_melee` is where it lives: the same field family as `knockback_growth` one line up, whose doc already said *"what a stage says when its moves author none"*. Smash declares the swipe verbatim (0.22/0.08/0.26, 4 damage, 34 reach); the versus route declares `None` and says why (its cast all author their own). ⚠ two fixtures went red and BOTH were right: a stage that declares no floor seats a kit-less character unarmed, so the fixtures had stopped modelling how a stage is built — repaired by declaring, never by loosening the assertion. ⇥ ⭐ **and the demo's own fighter came off the shared table** (2026-08-12): all THREE registered fighters in `ambition_demo_smash` took `fighter_moveset()` verbatim, so on the standalone stage George Booul the heavy swung identically to Robot v2 the lightweight and differed only by `knockback_weight`. `george_booul_moveset()` is his — eleven moves built on the law of the excluded middle his own row quotes (*"Either you are on the stage or you are not"*): three pokes at 0.05–0.07s for 3–4 damage, eight commitments at 0.16–0.40s for 11–21, and **nothing at all in between, not even a tilt**. The band is asserted at the builder as well as in the tests, and the poison is that the SHARED table must still HAVE a middle — otherwise the threshold, not George, is what the test describes. ⚠ the two robot stand-ins keep the shared table deliberately: their canonical repertoire lives on the real Robot provider (redirect §15) and a third robot table here is the copy that redirect forbids. ⇥ ⭐⭐ **AND THE COUNT IS MEASURED NOW, not remembered** (2026-08-12). `the_grid_fighters_with_a_real_repertoire_only_grow` asks the shipped host which grid fighters carry an `authored_moveset`: **SEVEN of fourteen** — `player_robot_v3`, `goblin`, `npc_pirate_admiral`, `special_patent_clerk`, `smash_george_booul`, `perfect_cellular_automaton` and `npc_ninja_shadow_oni_leader`. The seven on the generic floor are Mary-O and Sanic (other demos' protagonists, who bring bodies but no smash table) and five Hall NPCs authored to stand in a room and talk. ⇥ ⭐ **the oni leader is the first table written from a character's BARKS**, his row having no `gameplay_description`: *"the shadow answers"* → the fastest startups on the grid, *"one breath left"* → the shortest active windows, *"a leader's hardest order is the one obeyed instantly"* → every move recovers for more than THREE TIMES its own active window. ⚠ that ratio is a different AXIS rather than a fifth set of numbers — the four earlier tables slide reach/damage/speed together, and a table that only slides further along the same axis is the previous one renumbered. Its poison is that the GOBLIN must FAIL the ratio, or the property belongs to `strike`'s shape rather than to him. ⚠ **the test corrected the list I wrote from memory, by two** — I had four and there are six, because George's table reaches the HOST grid through the smash provider and the PCA's Cellular Pulse has been a real `MovesetContract` since D89. A ratchet written from what its author believes is a memory with an assertion around it. ⇥ ⚠ **and it measures MOVESETS, which is what this row is about** — the file's older census measures ACTION SETS (can this body swing) and its title has asked the moveset question since August; it was unanswerable while everyone was empty. ⇥ ▢ the scaffold is now one authored field a stage can stop declaring, rather than a helper a crate has to delete. ⚠ **and this clause used to say the adopter count was "unchanged in NUMBER", in the same commit that took it from eight to seven** — it was written about the deletion and left standing beside the census it contradicted. ⇒ the number lives in ONE place in this cell now (the measured seven), because a row that states its own count twice states it wrong once. |
| P3.25 | Remove universal `fighter_abilities` replacement | ◐ **it is a MASK, not a grant** — `seat_abilities` = character's authored verbs ∩ the mode's declared set; a ruleset may FORBID and may never hand a body a verb it lacks. Regression `a_match_cannot_grant_a_verb_the_character_does_not_have`, probed RED by swapping intersect→union. ▢ the bridge remains for characters that author nothing (almost all of them), and that is what deletes the field ⇥ ⭐ **THE NUMBER IS TWO** (2026-08-12, `the_cast_that_states_its_own_verbs_only_grows`). Only `perfect_cellular_automaton` and `imperfect_cellular_automaton` state an `AbilitySet`, so `effective_abilities`'s third arm — `(None, mode) => mode`, the GRANT — is what every other character takes. That arm is the scaffold, and it dies when the count reaches the cast; the test is a ratchet with a control that says to DELETE it rather than keep it once that happens. ⇥ ✔ **and the fly ▢ from Jon's handoff is SATISFIED**: Smash's declared set omits `fly` and `fly_toggle` entirely, so intersection grounds a flying character on the stage while leaving its intrinsic capability alone — which is exactly what he asked for, already covered by `a_match_cannot_grant_a_verb_the_character_does_not_have`. ⇥ ⛔⛔ **AND AUTHORING VERBS IS A NERF, WHICH IS WHY THE COUNT IS STUCK AT TWO.** The mask INTERSECTS, so a character that authors `basic() + attack` — everything its archetype row actually states — LOSES `shield`, `dodge`, `ledge_grab`, `double_jump` and `dash` on the smash stage, because it currently receives those from the grant. Migrating the goblin's verbs faithfully would ground it. ⇒ the remaining work is not a migration at all: to stop needing the bridge a character must author the FULL platform-fighter set, and whether a goblin can double-jump and ledge-grab is a design decision per creature — Jon's, like the other five this campaign has surfaced. The two automatons are the only characters whose rows already authored their verbs (`can_blink`/`can_fly`/`can_shield`/dash), which is exactly why they are the only two in the list. |
| P3.26 | Make Smash consume each character's actual body/capabilities/moves | ◐ **capabilities and MOVES both reach the seat now.** The blocker was invisible: a match's borrowed action-set grant regenerated the moveset from itself, so eleven authored move timelines lost to one derived swipe on the only path that seats a fighter. A grant covers the action set (*may this body attack*), never the moves (*what the attack IS*) — `authored_moveset` on the prepared value is what tells the two apart. ▢ the other roster characters still author no moves |
| P3.27 | Add Puppy Slug forced-seat regression | ◐ **the SEAM is pinned**: `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` seats a crawler that authors 36px/s, Slither and surface cling beside a character that authors none, and asserts the crawler keeps its own body while the unmigrated one still gets the stage's fighter default. Probed RED. ▢ the full end-to-end version — actually forcing `npc_puppy_slug` onto the Smash stage and pressing Attack/Jump. ⛔⛔ **THIS ROW SAID IT WAS BLOCKED, AND BOTH HALVES OF THE BLOCK ARE FALSE** (re-measured 2026-08-12): `character_archetypes.ron` holds `combatant` and `medium_striker` and nothing else — puppy_slug's archetype row is GONE — and its character authors `run_speed: 80.0`, `MoveStyleSpec::Slither`, `surface_walker` and `cling_breaks_on_hit`, pinned by its own definition test. ⇒ this is UNBLOCKED and the ▢ is the work, not the wait. ⇥ ✔ **AND THE WORK IS HALF DONE, with the half that is not measured rather than guessed** (2026-08-12). `puppy_slug_forced_seat.rs` seats the SHIPPED `npc_puppy_slug` through the real registration seam beside `npc_carl_stargan`, who authors nothing: the slug keeps 80.0 top speed, its surface cling and its contact damage 1, and Stargan gets the stage's default and no cling — the control is what stops the first three assertions passing on a stage that gives everybody a cling. ⇥ ⛔⛔ **THE SECOND HALF OF JON'S CRITERION FAILS, AND THE NUMBERS ARE NOW ON THE TABLE.** *"Jump → no jump if its body cannot jump … no generic humanoid jump, no generic dash"* — measured through that seam, `npc_puppy_slug` and `npc_carl_stargan` come out with the IDENTICAL mask: `jump=true double_jump=true attack=true`. A slithering wall-crawler double-jumps on the Smash stage, because the slug authors locomotion and contact damage but no `AbilitySet`, and the seat's mask INTERSECTS — with nothing to intersect against, the stage's fighter default wins whole. ⇒ this was **D96 item 9 / P3.25** with a concrete creature attached — and ✔ **JON ANSWERED THE SAME DAY**: *"If the slug does not have a double jump ability it should not be able to double jump. The point of a slug is that it shows that it is spawned happily even though it basically has no moves."* The slug authors `move_horizontal` and nothing else; the intersection already refused to GRANT a verb a character lacks, so authoring was the whole fix, and it corrects the EXPLORATION road too (`ActorBody::locomotion_abilities()` grants jump/variable/double to any body that authors none). ⚠ `attack: false` deliberately: its damage is CONTACT damage, so a swipe would be the "generic swipe" clause of the same criterion. ⭐ and the criterion Jon actually named is now its own test — `a_creature_with_one_verb_still_seats_and_simulates` runs 120 ticks and asserts both bodies are still seated and still on the number line, because "spawned happily with basically no moves" is a claim about the ARCHITECTURE rather than about the slug. ⚠ Stargan still receives the stage's humanoid mask and that is correct: the claim is not "no fighter gets defaults", it is "a character that SAYS what it can do is believed". ⚠ the test does NOT pin those masks as correct — it pins that the slug authors no mask, so the day one is authored the assertion to add is already written down in that function's doc. ⚠ a ▢ whose stated reason has since been deleted is the failure mode this campaign has already paid for four times — the reason has to be re-read, not the box. ⇥ ✔✔ **AND THE ENGINE HALF OF JON'S CRITERION IS PINNED NOW (2026-08-12), WHICH IT WAS NOT.** Asserting the seated slug's mask says `jump: false` is only worth having if the engine HONOURS the mask — and measured, **the base `jump` flag was the one ability gate nothing tested.** `double_jump`, `double_dash` and `wall_climb` each have one; the plainest capability in the set had none, and its gate is a single `&&` in `apply_intent`, exactly the shape a refactor drops with no compiler error. ⇒ `movement::tests::ability_gates::jump_ability_controls_the_ground_jump`: press Jump on a grounded body with the flag off and NEITHER the `MovementOp::Jump` nor the rise happens; the same fixture with the flag on does both. ⚠ **the op AND the velocity, because asserting only the op passes on an engine that emits nothing and launches the body anyway**, and the grounded control is what stops a gate that refuses every jump from reading as a pass. ⚠ FALSIFIED: dropping `&& abilities.abilities.jump` from `apply_intent` turns it red. ⭐ the two tests are cross-referenced in each other's docs, because neither is the claim alone — one says the shipped creature asks for no jump, the other says asking is what decides. |
| P4.28 | 3–2–1–GO opening countdown | ✔ **LANDED.** `MatchRules::opening_countdown_ticks` + `OpeningPhase`, DERIVED from `now - activated_on` so there is no timer in the rollback window (`activated_on` is snapshotted — omitting it would restart the ceremony mid-match after a rewind). The release moved OUT of the Smash stage into match flow and frees every seat in one flush; the stage only says the numbers. Test `a_declared_countdown_holds_every_seat_until_it_ends` asserts BOTH states were observed and was probed RED twice |
| P4.29 | Wire shields/parry for appropriate fighters | ◐ **authored, reaching the seat, and it immediately exposed a CPU defect worth the whole exercise.** Giving the fighters `shield` turned the stage into two statues: `Disadvantage` covers CORNERED as well as hitstun, Shield outscored Retreat, and guarding does not un-corner anybody — an absorbing state reached in the opening second, per fighter, forever. Fixed where the genre says: a shield is a reaction to a SWING (gated on a hostile mid-attack), and a cornered fighter with nothing incoming retreats. ▢ still unverified in play: does the bubble block, does the parry window read ⇥ ✔ **VERIFIED IN PLAY 2026-08-13** — `a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines` boots the real smash stage through `build_demo_app`, decides a two-fighter match, runs 120 ticks and reads the LIVE `BodyAbilities` off both seated bodies. ⭐ **the gap these three ▢ marks named was never a capability, it was a MEASUREMENT**: the verbs were authored and the engine machinery has been there all along, but nothing checked the DISTANCE between them — definition → preparation → seating → the match's ability MASK, each step of which had its own test while the chain did not. ⇥ ⛔⛔ **and falsifying it corrected the claim.** Dropping `shield` from the character turns it red (the character is NECESSARY). Adding `fly` to the character alone does NOT — the match mask intersects it away — so the poison proves the MASK, not "the character decides"; adding `fly` to BOTH turns it red. ⇒ what the test actually measures, end to end through a live stage, is `AbilitySet` INTERSECTION: a character states what its body can do, a ruleset states what the mode permits, and a verb needs both. The middle row is the one worth keeping — authoring a capability onto a character is not enough to smuggle it into a mode, which is what makes P3.25's mask a real restriction rather than decoration. |
| P4.30 | Wire grounded dodge | ◐ capability authored on the smash fighters; ▢ unverified in play. ⇥ ⭐ **THE ENGINE HALF IS COVERED — measured 2026-08-12, so the ▢ is narrower than it reads.** `available_dodge` gates on `abilities.dodge`, and `movement/abilities.rs` carries a dedicated test module for the resolver: which maneuver a body performs from one buffered press, with a body owning BOTH verbs as the interesting case. What is unverified is the COMPOSITION half — that a fighter's authored `dodge: true` is what its seated body carries — and that is the same claim as P4.29's shield and P4.32's ledge grab, because all three arrived in one `with_abilities` call for one reason. ONE test closes all three. ⇥ ⛔⛔ **AND THE ATTEMPT MEASURED WHY IT IS STILL OPEN (2026-08-12), so the next run does not rediscover it.** A seating test in `ambition_demo_smash` composing `MinimalShellPlugins + AmbitionLoadPlugin + SmashExperiencePlugin` — the composition the demo's own routing tests use — **PANICS on the first update after a roster exists**: `sync_live_player_dev_edits_system` takes `EditableAbilitySet` as a plain `Res`, and no shell-only composition publishes it. Adding `add_headless_foundation` + `PlatformerEnginePlugins::fixed_tick()` fixes that and then seats **nothing**, because this demo enters through CHARACTER SELECT: a roster inserted by hand never reaches a stage, so there is no seated body to read. ⇒ **the test needs a live stage, not a composed app** — the route has to be driven, which is what `smash_in_the_host.rs` does behind the `input` feature. That is the work, and it is fixture engineering rather than a missing capability. ⚠ recorded rather than left as a red test: a failing test in the tree is a worse instrument than an honest ▢. ⇥ ✔ **VERIFIED IN PLAY 2026-08-13** — `a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines` boots the real smash stage through `build_demo_app`, decides a two-fighter match, runs 120 ticks and reads the LIVE `BodyAbilities` off both seated bodies. ⭐ **the gap these three ▢ marks named was never a capability, it was a MEASUREMENT**: the verbs were authored and the engine machinery has been there all along, but nothing checked the DISTANCE between them — definition → preparation → seating → the match's ability MASK, each step of which had its own test while the chain did not. ⇥ ⛔⛔ **and falsifying it corrected the claim.** Dropping `shield` from the character turns it red (the character is NECESSARY). Adding `fly` to the character alone does NOT — the match mask intersects it away — so the poison proves the MASK, not "the character decides"; adding `fly` to BOTH turns it red. ⇒ what the test actually measures, end to end through a live stage, is `AbilitySet` INTERSECTION: a character states what its body can do, a ruleset states what the mode permits, and a verb needs both. The middle row is the one worth keeping — authoring a capability onto a character is not enough to smuggle it into a mode, which is what makes P3.25's mask a real restriction rather than decoration. |
| P4.31 | Implement true air dodge | ✔ **LANDED as its own maneuver.** `apply_dodge` was gated on `on_ground`, so an airborne dash press fell through to the air dash. The air dodge now carries its own state — `air_dodge_timer` (invulnerable window), `air_dodge_endlag_timer` (committed but VULNERABLE), `air_dodge_spent` (one per airtime) — aims along the full stick including DOWN, and refunds on landing through `refresh_movement_resources_clusters` rather than beside its 18 call sites. ⭐ `body_vulnerable` now takes ONE `evading` term (`BodyMotionFacts::evading()`) instead of a per-maneuver `dodge_rolling` argument threaded through six emit sites, so the next evade extends a method rather than auditing six callers. ⛔ the window is AUTHORED, not universal: `DEFAULT_TUNING.air_dodge_time` is 0.0 because an airborne dash press already means the air dash for every exploration body; the Smash fighter authors it next to its jump squat. Five tests, each poisoned red first |
| P4.32 | Enable and tune existing ledge mechanics in Smash | ◐ `ledge_grab` authored on the smash fighters — Jon's diagnosis was exactly right, *"the generic fighter capability set did not grant ledge_grab"*. ▢ verify grab/hang/climb/roll/getup-attack/jump/drop on the real stage, and fix what the first real adopter exposes ⇥ ✔ **VERIFIED IN PLAY 2026-08-13** — `a_seated_fighter_carries_the_verbs_its_character_authored_and_not_the_engines` boots the real smash stage through `build_demo_app`, decides a two-fighter match, runs 120 ticks and reads the LIVE `BodyAbilities` off both seated bodies. ⭐ **the gap these three ▢ marks named was never a capability, it was a MEASUREMENT**: the verbs were authored and the engine machinery has been there all along, but nothing checked the DISTANCE between them — definition → preparation → seating → the match's ability MASK, each step of which had its own test while the chain did not. ⇥ ⛔⛔ **and falsifying it corrected the claim.** Dropping `shield` from the character turns it red (the character is NECESSARY). Adding `fly` to the character alone does NOT — the match mask intersects it away — so the poison proves the MASK, not "the character decides"; adding `fly` to BOTH turns it red. ⇒ what the test actually measures, end to end through a live stage, is `AbilitySet` INTERSECTION: a character states what its body can do, a ruleset states what the mode permits, and a verb needs both. The middle row is the one worth keeping — authoring a capability onto a character is not enough to smuggle it into a mode, which is what makes P3.25's mask a real restriction rather than decoration. |
| P4.33 | Author landing lag/autocancel on real aerials | ◐ **all five aerials author BOTH halves** (n/f/b/u/d-air, lag 0.10–0.28s, autocancel windows inside each move's duration) — `MoveSpec` has carried the pair for a while with no adopter, and the guard asserts both are present because an autocancel with no lag is silently inert. ▢ unverified in play: does landing mid-aerial actually lock control for the authored time |
| P4.34 | Add at least one real strong/Smash attack to Robot v3 | ✔ **F-smash, U-smash and D-smash, authored as MOVES.** No resolver change: the runtime already read a Smash-strength gesture off a directional flick and already resolved `smash_forward → attack_forward → attack`. The F-smash is 18 frames of startup, 15 damage, 150 base launch with 1.3 growth and a 1.7× charge payoff, against the jab's 3 frames / 3 damage / 55 launch — a different move by every measure that makes it one |
| P4.35 | Add tumble/knockdown/tech/getup state and animation slots | ✔ **the four states landed** in `movement/knockdown.rs`: tumble (TWO fields — `tumble_timer` for the helpless part, `tumble_until_landing` for the part that outlives it; jump/attack ACT OUT), knockdown (0.55s prone, no control), tech (evade press in a 20-frame window skips the knockdown with i-frames; a guess expires into a 40-frame lockout), getup (roll / getup-attack / stand / timeout, all invulnerable). ⭐ the ENTRY is the kernel's `pending_launch` drain, so the combat side changed by zero lines — 'was that launch big enough to tumble' is the model's question. ⛔ the TICK had to move to the CONTROL phase: in the sim phase a tech attempt came out as an AIR DASH that stalled the body mid-flight, and a buffered dash fired `[DodgeRoll, Knockdown]` on one tick. `tumble_speed` authored (Smash: 500 px/s), engine default 0.0. Six poisons, six reds. ✔ **ANIMATION SLOTS TOO, with no new sheet rows**: knockdown → `LandHard`, getup → `LandRecovery`, tumble → `Hit`, and the AIR DODGE → `Roll` (whose own fallback is `DodgeRoll`, so one curl still animates and two sheets show two maneuvers) — which is the animation half of Jon's "distinguish it from a ground roll". ⛔ the ordering is load-bearing: hitstun outlives the landing, so reading `hit` first made the whole floor game invisible |
| P4.36 | Add stock-respawn protection | ✔ **LANDED.** Two seconds of the engine's generic `Empowered`/`UNTOUCHABLE` grant — the same timed untouchable a star pickup uses, already rollback-registered — inserted by the RULESET on a stock spend, never on an elimination. ⛔ the test found immediately that nothing in Smash ticked empowerments: the grant read `remaining: 2.0` five seconds later, permanent. `run_empowerments` is per-GAME registration (Mary-O and Sanic each schedule it) and Smash had never had an empowerment; registered, and noted as a footgun worth an engine-side fix |
| P4.37 | Tune hit feedback using existing generic hooks | ✔ **AUDITED AND FIXED 2026-08-12 — and the row's own instruction (*"do not rewrite the underlying hitlag/hitstun systems unless the measurement shows a defect"*) is why this is a measurement and not a rewrite.** ⭐ **HITLAG ALREADY SCALES WITH THE HIT.** `ae::hit_response::hitlag_duration = hitlag_time × reaction_scale(knockback)`, floored at half so the weakest connect is still a readable beat and sharing `hitstun_duration`'s ceiling; its own doc says *"a jab taps and a smash lands"*. Both sides freeze for the SAME duration, which is what makes a connect read as one event. Nothing to tune there. ⛔⛔ **THE DEFECT IS THE CAMERA: a landed hit never shakes the screen, at any severity.** `CameraShakeState::kick` has exactly TWO production call sites in the whole workspace — a boss phase change (`boss_encounter/systems.rs`) and a hard-fall LANDING (`app/phases.rs`). A smash that sends a fighter to the blast zone and a jab move the camera identically, which is exactly the *"strong hit should feel materially different from a weak poke"* the row names. ⇥ ✔ **FIXED at `720812aa6`, and it needed no new severity concept**: the hit has already resolved its severity into `combat.hitstop_timer` through `reaction_scale`, and the pattern to mirror is right there — a PURE amplitude function plus one kick site. `camera_ease::hit_shake_amplitude(hitstop, reference)` mirrors `hard_fall_shake_amplitude` exactly: dead zone, gain, unit-testable away from the bevy plumbing. The dead zone IS the reference, so a standard connect shakes nothing and only a hit HARDER than standard moves the camera — the same shape as the hard-fall floor being a jump-height landing. The hardest connect the 4x band allows reaches ~10px, under the 14px cap a hard fall already reaches, so both reasons to shake sit on ONE scale. ⚠ **the reference is a PARAMETER**: restating `0.070` in `shared_tangle` would be a second literal agreeing with `Platformer2dFeelTuningMonolith::hitlag_time` by coincidence — the shape that already cost this campaign a health pool — and a route that retuned its hitlag would silently retune its camera the wrong way. ⚠ kicked every frame the freeze is live, deliberately: `kick` is strongest-wins, so re-asserting HOLDS the shake for exactly the freeze and releases into the decay, with no edge-detection state (a `Local` remembering last frame's timer would be cross-frame state in a rollback schedule, for an effect that is already idempotent). ⭐ **and the SEAM is tested, not just the law** — a pure function nobody calls shakes nothing, which is the D106 failure one layer up. `hit_shakes_the_camera` drives the real sim: a 4x connect must move the camera and a reference connect must not. A camera that shakes on everything passes the first; a disconnected one passes the second. ⚠ scaling off the resolved hitstop rather than off a move id is what keeps this body-generic; a move-name special case is the thing the brief forbids. ⚠ the shake CEILING is already the ROUTE's statement (`CameraShakeTuning`, D14), so a stage that wants a calmer camera already has the dial. ⛔⛔ **REOPENED AND RE-FIXED 2026-08-12 (GPT 5.6 review of `1579ab3`), and the first fix was the defect wearing the fix's clothes.** The shake landed in `ambition_app`'s `sync_player_presentation` — a system whose query is `With<PlayerEntity>` and whose kick was gated again on `PrimaryPlayer`. THREE independent reasons that could not serve P4.37, each fatal alone: (1) `PrimaryPlayer` names the HOME AVATAR, and `time_control` already carries Jon's 2026-08-07 freeze as standing proof — *"start a CPU-versus-CPU match. There is no `PrimaryPlayer` in it"*; a match under `InitialBodyPolicy::NoInitialBody` legitimately has ZERO. (2) **that system is registered by `ambition_app` ALONE** — measured, not inferred: `grep` finds its only `add_systems` in `app/plugins.rs:224`, and `ambition_demo_smash_app` composes `PlatformerEnginePlugins` + `PlatformerHostPlugins` and never installs it. The feature could not fire in the PROVING-GROUND BINARY at all. (3) it read ONE body's `hitstop_timer`. ⛔ **and the test agreed with the bug**: it booted the Hall, ASSIGNED `combat.hitstop_timer` on the home avatar by hand and stepped one frame — proving that manually arming the one body that still worked can shake the exploration camera, which is not the claim. ⇥ ✔ **the fix is a body-generic system in the ENGINE**: `features::ecs::hit_camera_shake::shake_camera_on_landed_hits`, scheduled in `CombatSet::Settle` by `CombatSchedulePlugin` — the group every host composes. It reads EVERY `BodyCombat` and folds with `max` (order-independent, so query iteration order cannot change the frame). No player marker, no move ids, no per-character table. ⛔⛔ **and the MEASUREMENT found a second defect the first fix could not have survived: THE DEAD ZONE WAS ABOVE THE WHOLE GAME.** Probing `duel_arena` — a real authored fight — the hardest connect either fighter produced was **0.0595 s against a 0.070 s reference, 0.85x**. The dead zone sat at the full reference, so EVERY hit in Ambition's own combat landed under it and the camera could never move in the shipped game; only a Smash-style growth knockback could ever have cleared it (the smash demo authors real `knockback_growth`; every prefab-derived swing authors `0.0`). ⇒ the dead zone is now the WEAKEST connect the hitlag law admits, named ONCE as `ae::hit_response::MIN_HITLAG_SCALE` and used by both `hitlag_duration`'s floor and the camera — not a second `0.5` agreeing by coincidence. The duel's ordinary trade buys ~1.2px, the hardest smash ~11.8px: a tenfold spread under the 14px hard-fall cap, instead of a cliff nothing reached. ⭐ **the regression EARNS its hit and REMOVES the home avatar**: `hit_shakes_the_camera` boots `duel_arena`, strips `PrimaryPlayer` from every body (the exact `NoInitialBody` shape), and watches 600 frames of a fight it did not arrange. Four clauses, none droppable: zero home avatars for the duration · hits actually landed · they cleared the weakest-connect floor · the camera moved. ⚠ **FALSIFIED, not just passed**: re-adding `With<PrimaryPlayer>` to the system's query drops `peak_shake_px` to exactly `0.0` and the test fails with its own message. ⇥ ✔✔ **AND A THIRD DEFECT, FOUND BY THE NEXT REVIEW AND REAL (GPT 5.6, priority 1): IT READ TWO CLOCKS.** Making it body-generic put it in the SIMULATION schedule, which is right for reading the frame's resolved damage and wrong for writing `CameraShakeState` — presentation state that is NOT rollback-registered, so a rollback host re-executing historical frames observed one landed hit again on every resimulation and kicked the PRESENT camera each time: a ghost shake with no hit under it, arriving whenever the network hiccups. ⇒ the guard is a PARAMETER, not a `run_if` at the registration — this is the one system in the combat schedule that writes non-rollback presentation state, so "authoritative passes only" is a property of the SYSTEM rather than of one call site, and a second registration that forgot the condition would bring the ghost back silently. ⚠ **and the opposite policy is right one module over**, so the note says which case this is rather than citing doctrine flatly: `dev/trace`'s recorders were once gated this way and it was WRONG, because rows keyed by `(generation, frame)` are REPLACED by a resimulation while a camera kick replaces nothing — a monotone `max` onto live state, so a replayed pass can only ADD. ⭐ the regression drives the real system over ONE unchanged armed hit twice, differing only in `replaying_history`, and asserts both terms; falsified by dropping the guard. |
| P5.38 | CPU AI chooses from actual character movesets/capabilities | ◐ **MOST OF THIS WAS ALREADY TRUE and the row did not say so** (grepped 2026-08-12 before touching anything). The Smash CPU seats run `template: Fighter`, and that brain does ask: `attack_kit_of` enumerates the body's REAL moveset by asking `move_for_directional_verb` for basic/smash/special × five directions, so a CPU throws the character's own `smash_forward` — `press_the_chosen_attack` sets `melee_strong_hint` for a Smash binding. Movement verbs are capability-gated on the body (`can_blink`/`can_shield`/`can_dash`, jump on the real air-jump budget). ⛔ **the ONE thing it got wrong was the evade, and it got it wrong in three places at once**: `apply_dodge` claims the dash buffer BEFORE `apply_dash` can see it, so a body owning `dodge` never dashes — and the Smash fighters author `dash: true` and `dodge: true` together (P4.30). Every burst a CPU chose on that stage was named `Dash` by the brain, modelled as `ShadowIntent::Dash` by the rollout, and performed as a ROLL by the body; and a body authoring only `dodge` was offered no burst at all, because the question asked was `can_dash`. ⇒ `SelfView::can_dodge`, one `evade` selection that names the maneuver the press will really produce, a direction rule (roll AWAY from a swing, INTO everything else — perceivable, so a human could make the same read), and `Dodge` added to the rollout's unmodelled list, which REMOVES a lie rather than adding a gap. ⇥ ▢ remaining: no ledge verb (`ledge_grab` is authored but `Recovery` still walks toward centre and jumps — that is P4.32's). ⇥ ⛔⛔ **AND THE SECOND CLAUSE WAS WRONG — re-measured 2026-08-12.** It said the `template: Smash` brain *"builds an EMPTY `attack_kit` by design, so Ambition's own enemies do not choose from their movesets"*. The premise is true and the conclusion does not follow: `attack_kit_of` does return `Vec::new()` for a non-`Fighter` brain, deliberately, because a scored kit is a per-actor per-tick `Vec` no other brain reads. But choosing from a moveset does not require a scored kit. The Smash brain AIMS: `smash::action` picks up / down / forward from the target's real offset, `emit` turns that into `ActorControlFrame::attack_axis`, and `resolve_attack_gestures` resolves the axis against the body's OWN `MovesetContract` — so a goblin that authors an up-tilt throws it at a target overhead, and one that authors none falls back to its base attack. ⇒ what Ambition's enemies genuinely do NOT choose is the VERB (basic vs smash vs special) and they never weigh frame data; that is the real gap, and it is a smaller and more specific one than the row claimed. ⇥ ✔ **AND THE HALF THAT IS TRUE IS PINNED NOW, because it was not**: `choose_action`'s direction pick — the entire mechanism — had ONE test asserting a melee comes out at all. `an_engaged_swing_aims_at_where_the_target_actually_is` asserts all four reads: level → sideways toward the foe, overhead → up, airborne-above-foe → down, and ⭐ **GROUNDED-above-foe → NOT down**, because a body standing on a platform over its foe is not throwing a down-air and a rule reading the vertical offset alone would answer identically. ⚠ FALSIFIED: dropping `&& !obs.self_on_ground` turns it red on exactly that clause. ⚠ gravity-framed throughout — `down` is the observation's, so a rotated-gravity room reads the same (I10). |
| P5.39 | Remove obsolete Smash stand-ins | ✔ **ANSWERED, AND THE ANSWER IS THE DEPENDENCY REASON JON'S BRIEF ASKED FOR** (2026-08-12). The brief allows keeping them *"if the standalone packaging boundary genuinely cannot import the content provider cleanly"*, and it cannot: `game/ambition_demo_smash/Cargo.toml` depends on **`ambition_platformer2d` + `bevy` and nothing else** — the E9 oracle rule, whose whole point is that a stocks match must be expressible through the ENGINE facade. `player_robot_v3`/`v2` are authored in `ambition_content`, a GAME crate; depending on it would delete the property the demo exists to prove. ⇒ the two copies are the packaging boundary, not a leftover of the pre-registry architecture. ⭐ **and the duplication is CONDITIONAL, which is what makes it harmless**: `SmashRoster::assemble` drops each copy the moment the character it stands in for resolves, so no host shows two robots with one wearing a made-up name. ⛔ **three test files relied on that in a COMMENT and nothing asserted it** — a stand-in that stopped stepping aside would surface as a duplicate portrait nobody was looking for. `the_demos_robot_copies_step_aside_for_the_real_lineage` asserts it against the composed host, poisoned by the standalone default (which must still declare them, or the copies were simply deleted and the drop rule had stopped running). |
| P5.40 | Rerun PCA as an unconditionally registered character | ▢ **BLOCKED ON D74, WHICH JON'S OWN ROW ALLOWS SEPARATING** (*"keep D74 separate if the timing bug remains"*). Re-verified 2026-08-12: it remains. ⭐ the state is already LOCATED and written at the exclusion itself in `character_catalog.rs` — registering the PCA reds `possession_end_to_end::attack_while_possessing_…`, the per-step trail is identical through step 3 and parts at step 4 on `vel.x` (baseline zeroes it on a 4-step cadence, the registered build accumulates at −10.83/step, both falling), with the same hp, same `Brain::Player(0)`, same collision size and same gravity — so the fault is UPSTREAM of combat, in a movement or contact decision on a falling body, and the possessed body ends 580 px away and airborne. ⛔ **four wrong mechanisms have been written down for this already; the standing rule at that comment is DO NOT ADD A FIFTH WITHOUT OUTPUT**, so this row stays ▢ rather than accumulating another guess. The next step is the one the comment names: bisect the movement kernel across the deterministic step-4 divergence. ⚠ the cost of the exclusion is stated and unchanged — the PCA is on `SMASH_ROSTER`, so the grid is one portrait shorter. |
| P5.41 | Clean architecture docs and stale comments | ◐ **THE EIGHT THIS SESSION BROKE ARE REPAIRED**, and repairing them found the real shape of the row. Deleting `project_provoked_archetype`, `HostileArchetypeId`, `AutonomousSource::Provoked`, `reconstruct_provoked` and `BUILDABLE_ONLY_CAST` left doc links pointing at items that no longer exist — and two of them were worse than broken, they were WRONG: `autonomous_reconcile`'s module doc still described a rewind "rerunning the roster archetype construction … tuning / capabilities from the archetype id the binding retained" (all three nouns gone), and `reconstruct_provoked_profile`'s note still called its counterpart the one that "rebuilds a whole body because an archetype IS the creature" — which was the DEFECT, fixed the same day. Each repair keeps a ⚠ line saying what the sentence used to claim, because a reader who knew the old text needs to know it was retired rather than lost. ⇥ ⛔⛔ **AND `cargo check` CANNOT SEE ANY OF THIS.** The gate is `check --all-targets`; broken intra-doc links are a RUSTDOC lint, so this whole class rots silently between the two. Measured 2026-08-12 with `cargo doc --no-deps`: **199 unresolved-link / private-item-link warnings** across four crates — monolith 122, `ambition_characters` 39, `ambition_platformer2d_core` 21, `ambition_combat` 17 — and only eight of them were mine. ⇒ ✔ **the RATCHET exists and CI runs it** — `scripts/check_doc_link_ratchet.py --check`, baseline in `dev/doc_link_ratchet_baseline.json`, in the `rustfmt + clippy` job. Counts may fall and must not rise; it also fails when a crate emits NO rustdoc output, because zero warnings from a build that did not happen is not a score. Poisoned with one bogus link: red. ▢ the 199 itself is untouched on purpose — the ratchet is what makes lowering it stick, and a sweep with nothing behind it is 199 links again by September. Ledger **D103**. |
| P5.42 | Measure deletion payoff | ◐ **THE FIRST ACTUAL MEASUREMENT, 2026-08-12 — and it reframes the acceptance signal.** Jon's baseline is *~3,755 lines of standing legacy*. Measured today, the same six items are **3,059 lines — of which 1,600 are CODE and 1,459 (48%) are PROSE.** ⛔ **STATE THE UNITS: a line count over this repository is not a code count.** This campaign writes long ⛔/⭐ notes recording what left and why, and those notes live in the files they are about, so the legacy shrinks in behaviour faster than it shrinks in bytes. A deletion ledger read in raw lines will understate the payoff and — worse — can read a file as GROWING while its code halves. ⇥ **per item, total / code:** `features/enemies/mod.rs` 1,786 / **1,107** · `autonomous_reconcile` 416 / 204 · `ArchetypeSpec` 320 / 119 · `ActorTuning` 197 / 80 · `character_archetypes.ron` 264 / **45** (two rows) · `enemy_roster.rs` 76 / 45. ⭐⭐ **`features/enemies/mod.rs` IS 69% OF ALL REMAINING LEGACY CODE**, and everything else together is under 500 lines. That is the lever, and it is one file. ⚠ **its raw line count is ABOVE the baseline's 1,198 and that is not necessarily growth** — the baseline read *"features/enemies + CharacterRoster"* and it is not recorded which files that spanned, so the two numbers are not comparable and this row will not pretend they are. What IS comparable is the code figure going forward, which is why it is recorded here. ⇥ **the three items that have visibly moved**: `character_archetypes.ron` 843 → 264 (45 code, two rows), `autonomous_reconcile` 1,045 → 416, `ActorTuning` 275 → 197. ⇥ **the two that have not moved at all**: `ArchetypeSpec` (319 → 320) and `enemy_roster.rs` (75 → 76). Both are downstream of the same content decisions D96 holds, which is the honest reason and not an excuse — an item that cannot move until Jon answers should say so rather than sit unmarked. |

---

Work from the live repository.

The handoff baseline was:

```text
6079f2233d7244e64a8c87123f92aac0da809b22
```

If HEAD is newer, inspect the newer work first and preserve anything that already satisfies this brief. Do not redo completed work simply because this prompt names it.

This is an **overnight execution campaign**, not an investigation-only assignment.

Keep working through independent tasks rather than stopping after the first architectural seam. Make coherent commits as major slices land. If one task becomes blocked by an unrelated defect, record the reproduction and move to the next independent item unless that defect truly blocks the architecture.

The two goals are deliberately coupled:

1. **Finish D73**: move Ambition toward the normal professional game-engine model where a character is a reusable authored template that can be instantiated arbitrarily many times, and delete the old enemy-archetype ontology rather than wrapping it.
2. **Use Smash as the proving ground**: by the next playtest, the Smash demo should feel materially more like a real platform fighter—native character moves, real defensive/recovery options, proper round opening, meaningful aerial/landing states, stocks/percent/knockback rules, and less “generic characters walking around an arena.”

Do not treat these as competing projects.

Smash is one of the strongest tests of whether the character architecture is actually compositional.

---

# Product model

The engine should ultimately have this simple story:

```text
CharacterDefinition("Goblin")
    ↓ instantiate
Goblin actor A
Goblin actor B
Goblin actor C

CharacterDefinition("Fretjaw")
    ↓ instantiate
Fretjaw actor A
Fretjaw actor B
```

Character identity is **template identity**, not singleton runtime identity.

```text
CharacterId
    = what reusable character this actor instantiates

SimId / FeatureId
    = this particular runtime actor

controller
    = who or what currently drives this body

spawn/session context
    = why this instance exists and what contextual rules apply
```

A fictionally unique named character may be spawned twice.

A generic Goblin and a named Fretjaw are the same kind of engine definition.

What differs is authored content, not actor ontology.

This is the model associated with reusable Prefab/Blueprint/PackedScene-style authoring in mainstream general-purpose engines.

---

# The three authorities

Every fact migrated out of the legacy model must end in exactly one of these categories.

## Character definition — what this character IS and CAN DO

Examples:

```text
body geometry
body/movement model
mass / knockback weight
vitals baseline
intrinsic movement capabilities
intrinsic abilities
action repertoire
moveset
attack volumes
hurtboxes
intrinsic equipment/loadout
mount/pilot body capabilities
intrinsic death traits
presentation
voice
```

Reusable low-level profiles are fine if they help author these facts, but the **prepared character** must resolve them into one complete answer.

## Controller / autonomous profile — how an autonomous participant chooses actions

Examples:

```text
brain strategy
patrol/chase policy
aggro distance
spacing preference
attack-selection policy
fighter difficulty
wall-turn behavior
Smash tactical preferences
```

A controller supplies intent.

It does not manufacture body capabilities.

## Spawn/session/ruleset context — what is true about this instance here

Examples:

```text
current controller
team/faction
disposition
encounter membership
respawn policy
stocks
match rules
story/placement identity
patrol route
```

A Goblin can be friendly, hostile, human-controlled, CPU-controlled, summoned, or seated in Smash without becoming a different character.

---

# Hard architectural rule

Do not migrate `ArchetypeSpec` wholesale into another struct.

The legacy archetype is a god-object because it currently combines all three categories.

The successful endpoint is not:

```text
ArchetypeSpec
    renamed to CharacterProfile
```

The successful endpoint is:

```text
CharacterDefinition
+
BrainProfile
+
SpawnContext / ruleset state
```

with the legacy god-object deleted.

---

# Current state at the handoff

Important work has already landed.

Preserve it unless current HEAD supersedes it.

## Already good

* typed `CharacterId` exists;
* `WornCharacter` now stores typed character identity;
* gameplay identity for an authored enemy no longer comes through `sprite_character_id` / display-name presentation lookup;
* `BrainProfileRef` and resolved `BrainPresetId` are separate concepts;
* authored death traits were moved below the runtime combat crate as `CharacterDeathTraits`;
* the optional-`ActorMoveset` query-membership bug was fixed;
* the double moveset-mint bug was subsequently fixed;
* a real upstream `CharacterSpawnPlan` exists rather than merely merging the already-lowered NPC/enemy plans;
* an authored enemy can consult a prepared character before spawning;
* duplicate instances of one character definition are already understood/tested as legal.

## Still transitional

The legacy architecture is still heavily present.

At the handoff, source use remained widespread:

```text
ArchetypeSpec         → many production files
CharacterRoster       → many production files
spec_for_brain        → many production files
character_archetypes.ron still exists
PlayableKitSource / HostCode still exists
PreparedMatch still uses CharacterRoster
Smash still injects smash_fighter_kit()
Smash still replaces fighter abilities with one generic set
```

Do not declare D73 finished while those facts remain.

---

# FIRST: repair the current CharacterSpawnPlan correctness hole

Current `CharacterSpawnPlan::definition()` effectively returns `None` for two different cases:

```text
A. placement has no character id yet
B. placement explicitly names CharacterId X but X is absent from PreparedCharacterRegistry
```

Those must not be equivalent.

During migration:

```text
no character authored
    → temporary legacy fallback is allowed and visible

explicit character authored but not prepared
    → construction ERROR
```

An authored:

```text
character_id = IronMary
```

must never silently produce a shark-rider body because Iron Mary was accidentally omitted from some registration list.

Implement a typed failure/result.

Conceptually:

```text
resolve_character(...)
    -> Result<Option<&PreparedCharacterDefinition>, MissingPreparedCharacter>
```

where `Ok(None)` means specifically “this legacy placement has not yet been migrated.”

Add a poison regression proving that an explicit missing character fails loudly.

At the final D73 endpoint the `Option` disappears entirely for normal character spawns.

---

# Resolve character-owned brain references during preparation

The authored form may carry a provider-relative controller reference:

```text
BrainProfileRef
```

but a `PreparedCharacterDefinition` should be prepared.

It should contain the canonical resolved autonomous-profile identity:

```text
BrainProfileId / BrainPresetId
```

not an unresolved local reference which requires `CharacterCatalog` again at spawn time.

Target:

```text
CharacterDefinition {
    provider = foo
    default_autonomous_profile = "fighter"
}

        ↓ prepare

PreparedCharacterDefinition {
    default_autonomous_profile = foo::fighter
}
```

A placement override may remain provider-relative until the placement itself is lowered.

The character's own prepared default should not need a parallel catalog row merely to know its namespace.

---

# Finish typed CharacterId propagation

`CharacterId` should stay typed through:

```text
authored character source
PreparedCharacterDefinition
PreparedCharacterRegistry key
CharacterSpawnPlan
runtime CharacterIdentity
match participant preparation
encounter/summon requests
```

Do not repeatedly convert authoritative character identity back to `String` and then reparse/recompare it.

Strings are appropriate at:

```text
RON/serialization boundaries
UI/debug rendering
external authoring text
```

not between engine authorities.

---

# Generalize WornCharacter into the universal character identity

Do not introduce a second runtime component saying which character an enemy is.

The current `WornCharacter` already contains the right fundamental fact.

Evolve/rename it into something like:

```text
CharacterIdentity(CharacterId)
```

if that name best describes the final semantics.

A body carrying it means:

> this runtime actor is an instance of this CharacterDefinition.

It does **not** mean:

> this is specifically the controlled protagonist or a temporary costume.

Audit the current persona-derive side effects before widening it.

The final architecture should not rely on:

```text
insert CharacterIdentity
    ↓ later update system notices it
    ↓ reconstructs half the body from CharacterCatalog
```

Ordinary construction should produce the complete body directly from the prepared definition.

A later runtime character transformation/re-template may legitimately use a separate reconciliation operation.

Do not let that special dynamic operation dictate normal spawn construction.

---

# Finish the common character constructor

This is the central D73 milestone.

All ordinary character spawn surfaces must lower to one semantic construction operation.

Keep separate authoring schemas where they make sense:

```text
NpcSpawn
EnemySpawn
EncounterMobSpec
SummonSpec
MatchParticipant
programmatic character request
```

but lower them into a common input roughly like:

```text
CharacterSpawnPlan {
    character: CharacterId,
    controller selection / autonomous override,
    minimal shared spawn facts,
}
```

paired with:

```text
PreparedCharacterDefinition
```

then build generic runtime components once.

Do not make `SpawnContext` a giant collection of everything an `EnemySpawn` currently happens to know.

Current fields such as:

```text
room kinematic paths
feature display name
faction
```

may need to move into narrower contextual/controller/relationship inputs as the second and third callers arrive.

A Match participant should not need dummy room paths.

A summon should not need to invent an LDtk feature name.

Let additional callers expose what is truly shared.

---

# Construction acceptance condition

The final ordinary path must resemble:

```text
PreparedCharacterDefinition
+
CharacterSpawnPlan
        ↓
CharacterIdentity
BodyHealth
BodyKinematics / movement model
BodyAbilities
ActionSet
ActorMoveset
hurtboxes
combat traits
presentation identity
etc.
```

Then independently:

```text
ControllerBinding
BrainProfile
team/faction
respawn/lifecycle
interaction
encounter state
```

attach their contextual facts.

No ordinary constructor should first build:

```text
ArchetypeSpec creature
```

and then patch the character over it.

`ActorClusterSeed::adopt_character_intrinsics` is a temporary probe seam.

Do not grow it until it applies every legacy archetype field.

Replace that pattern with character-first construction.

---

# Move CharacterDefinition and prepared domain types to their proper crate

Use:

```text
.agent/index/crates/graph-resolved.json
```

before changing crate dependencies.

The low character-domain types should live in the lowest natural reusable character crate, probably `ambition_characters` if the live graph still supports that direction.

Use the crate boundary as a design test.

The earlier `CombatCapabilities` problem was handled correctly:

```text
runtime CombatCapabilities
    did not belong on authored CharacterDefinition
```

so a lower `CharacterDeathTraits` fact was extracted and then lowered into the runtime component.

Repeat that reasoning for every dependency obstacle.

Do not solve dependency cycles by leaving the authoritative character model inside the actor monolith.

---

# PreparedCharacterDefinition must become COMPLETE

Source authoring may remain compact and optional.

Prepared data should not mean:

```text
None
→ leave whatever the old archetype happened to say
```

for intrinsic character facts.

Preparation should resolve:

```text
explicit character values
+
reusable profiles/defaults
+
provider defaults where appropriate
```

into a complete immutable character template.

Normal construction then needs no second gameplay registry to answer what the character is.

For facts whose ordinary value is “none”:

```text
death traits = default/no special behavior
```

prefer an explicit resolved default in prepared data rather than optionality whose hidden meaning is “ask another authority.”

Keep `Option` only where absence is itself meaningful.

---

# Fix held-item death ownership before migrating it broadly

`CharacterDeathTraits` currently includes something like:

```text
drops_held_item: Option<HeldItemSpec>
```

but Ambition's architecture says the runtime body owns its currently held item.

If the intended rule is:

> when this kind of character dies, drop whatever they are currently holding

then author:

```text
drop_current_held_item_on_death: bool / policy
```

and inspect the live held item when death happens.

Do not snapshot an authored weapon into character death metadata and then drop that stale item after the actor has changed equipment.

If some character instead always produces a specific loot item, model that explicitly as death loot.

Do not conflate the two.

---

# Route every construction surface through the character path

Once the constructor exists, migrate callers in an order that continuously increases proof.

Recommended order:

## 1. Authored enemy

Already partially there.

Make it fully character-first.

An authored character's:

```text
health
body
movement
abilities
actions
moves
death traits
mountability
```

must not come from `spec_for_brain`.

## 2. NPC

The same character in peaceful NPC form and hostile enemy form must have the same intrinsic body and repertoire.

Only:

```text
controller
disposition
interaction
placement context
```

differ.

## 3. PreparedMatch / Smash

Do this **early**, not at the very end.

It is the strongest architecture test.

Remove `PreparedMatch`'s dependency on:

```text
CharacterRoster
ArchetypeSpec
```

The exact same Fretjaw definition should be seatable under:

```text
Human controller
CPU controller
```

without reconstructing another fighter body underneath.

## 4. Encounter mobs

Replace:

```text
kind = what it does
character = what it looks like
```

with real character identity plus optional controller override.

## 5. Summons / programmatic spawns

Replace `archetype_id` / `SpawnActorKind::Enemy { brain }`-style actor construction with character-first requests.

## 6. Provider characters

A provider must be able to add a complete new character plus autonomous profile with **zero actor-engine source edits**.

---

# Start deleting legacy rows immediately after each migration

Do not postpone all deletion until a final cleanup phase.

For each migrated character/profile group:

```text
move intrinsic facts
move autonomous policy
move contextual facts
update consumers
DELETE migrated legacy fields/rows
```

When a legacy row has no remaining semantic owner, delete it immediately.

The diff should begin trending negative.

---

# Migrate character_archetypes.ron semantically

There is intentionally **no universal archetype → character mapping**.

The ids are different namespaces.

That is correct.

Use the existing one-time migration ledger as evidence, then delete it when migration is complete.

## Group A — clear character/profile pairings

Start with characters where the intended mapping is obvious, such as the mite family and other one-character/one-profile cases.

For each:

```text
intrinsic body facts → CharacterDefinition
AI policy            → BrainProfile
respawn/faction      → placement/context
```

Delete the old row afterward.

## Group B — shared behavior profiles

Cases like `medium_striker` are proof that the shared entity is **AI/controller policy**, not actor identity.

Multiple real characters may reference the same `BrainProfile`.

Their bodies remain distinct definitions.

## Group C — generic role names

Classify them.

If a visible recurring thing genuinely exists in the world:

```text
Goblin
GoblinHeavy
TrainingTarget
...
```

give it a real reusable CharacterDefinition.

If it is a true fixture/debug-only construct, use an explicit fixture API.

Do not create fake character identities solely to satisfy uniformity.

If temporary art is borrowed, use a presentation override rather than lying about CharacterId.

---

# Delete the old roster authority

The final production tree should no longer need:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
character_archetypes.ron
enemy_roster.rs
```

Provider roster fragments should become normal registration of:

```text
CharacterDefinition
BrainProfile
provider/game metadata
```

No compatibility re-export.

No `LegacyCharacterRoster`.

No empty dead registry left because tests used to instantiate it.

---

# Split/delete ActorTuning

`ActorTuning` currently mixes too many authorities.

Move its surviving facts to the components that actually own them:

```text
movement tuning      → character/body movement
health                → BodyHealth/vitals
AI distances          → BrainProfile/controller policy
contact behavior      → body/combat trait where intrinsic
faction/hostility     → relationship/disposition
respawn               → lifecycle context
death policy          → body/ruleset authority
visual character id   → CharacterIdentity/presentation
```

If a small coherent runtime tuning component survives, rename it for exactly that responsibility.

Do not retain `ActorTuning` as a miscellaneous compatibility projection.

---

# Provocation: delete body reconstruction

A peaceful named actor is already the complete character.

Example:

```text
Fretjaw in Hall
    CharacterIdentity = Fretjaw
    Fretjaw vitals
    Fretjaw movement
    Fretjaw actions/moves
    peaceful disposition
    ambient controller
```

Provoking Fretjaw should produce:

```text
same CharacterIdentity
same vitals
same movement
same abilities
same moves

different disposition
different autonomous controller
```

Delete name/dialogue-string heuristics which select a hostile body archetype.

Delete:

```text
HostileArchetypeId
provoked archetype reconstruction
health/body/kit rewrites on provocation
```

once rollback no longer needs them.

---

# Rollback: preserve character identity and controller identity separately

Rollback should snapshot/restore:

```text
CharacterId
runtime mutable body state
controller/profile binding
disposition/context state
```

It should not need:

```text
legacy archetype id
→ rebuild what this character physically is
```

Update schema/checksum/registration coherently whenever rollback-owned state changes.

Do not create a newly named replacement for `HostileArchetypeId`.

---

# Remove protagonist HostCode special treatment

`player_robot_v3` must become a normal complete CharacterDefinition.

Delete the final need for:

```text
PlayableKitSource::HostCode
PreparedKit::HostCode
```

The character definition owns its real move repertoire.

The runtime body owns:

```text
progression unlocks
equipment
temporary grants
current inventory
session restrictions
```

The protagonist should not need a separate “ask application code what moves I have” branch.

This is especially important for Smash.

---

# SMASH: the character refactor is the foundation, not a separate project

The Smash demo currently has several good engine systems underneath it:

* stocks;
* blast-zone deaths;
* unbounded damage percent;
* percentage HUD;
* match winner flow;
* damage-scaled knockback;
* DI;
* hitlag/hitstun;
* body-generic combat resolution;
* body-generic movement;
* jump squat;
* landing-lag/autocancel support;
* shield/parry infrastructure;
* dodge infrastructure;
* substantial ledge-grab/getup machinery;
* real ActorMoveset / directional attack infrastructure.

The reason the demo still feels generic is largely that many of these systems are either not adopted by the fighters or are being overwritten by historical leveling hacks.

Use D73 to expose them properly.

---

# Smash principle: same character, different rules

For Player Robot v3 in particular, the target is:

```text
same CharacterDefinition
same moveset
same attack timings
same attack volumes
same authored damage
same authored base launch

Ambition interpretation:
    HP depletion
    low/flat knockback growth
    little/no DI
    exploration progression/context

Smash interpretation:
    percent accumulation
    stocks/blast zones
    damage-scaled knockback
    DI
    match lifecycle
```

Do not make:

```text
PlayerRobotV3Ambition
PlayerRobotV3Smash
```

or maintain two copies of its attacks.

The ruleset changes the interpretation, not the character identity.

---

# Remove Smash's generic fighter leveling hacks

Current Smash currently does roughly:

```text
every selected character
    .with_action_set(smash_fighter_kit())

roster.fighter_abilities =
    same move/jump/double-jump/dash/attack set for everybody
```

These were compensating for the old broken character model.

Remove them as soon as the characters are complete enough to survive without them.

The curated Smash roster may require a usable fighter kit.

The engine must **not manufacture one for arbitrary characters**.

---

# Puppy Slug is an explicit compositional acceptance test

Force a Puppy Slug into Smash through a test/debug setup even though it is not normally selectable.

Expected behavior:

```text
CharacterIdentity = PuppySlug

movement input
    → uses Puppy Slug's actual authored locomotion

Attack
    → no action if Puppy Slug has no attack

Special
    → no action if none is authored

Jump
    → no jump if its body cannot jump

stocks / damage / knockback / HUD / blast zones
    → still work normally
```

Smash must not silently give it:

```text
generic swipe
generic humanoid jump
generic dash
generic shield
```

This proves:

```text
controller ≠ capability
ruleset ≠ moveset
buildable ≠ Smash-selectable
```

Keep Puppy Slug off the normal roster unless there is a deliberate product decision to make it a fighter.

---

# Player Robot v3: make it the first polished real fighter

Once HostCode is removed, make Robot v3 the strongest end-to-end proof.

Its authored repertoire should be usable both in Ambition and Smash.

Do not create duplicate Smash move definitions.

At minimum make sure the shared moveset meaningfully supports:

```text
grounded basic attack(s)
up/down directional grounded attacks where authored
neutral/forward aerial
back air
up air
down air / pogo where appropriate
ranged/special where genuinely part of the robot
```

Use the existing directional `ActorMoveset` architecture rather than adding Smash-only combat dispatch.

---

# Add real strong / Smash attacks

The engine already has:

```text
SMASH_VERB
AttackVariant::FSmash
AttackVariant::DSmash
AttackVariant::USmash
```

but these are not yet a meaningful part of play.

Land a body-generic strong-attack gesture.

A reasonable implementation may use:

```text
held attack / dedicated strong input / existing Smash verb
```

according to the current input architecture.

Do not hardcode Robot behavior into the resolver.

Author the actual moves on the character's moveset.

At minimum give Robot v3 one satisfying forward Smash attack and, if the existing architecture permits cleanly, Up Smash and Down Smash as well.

Strong moves should differ materially in:

```text
startup/commitment
damage
base launch
hitlag/feedback
recovery
```

rather than merely naming the normal swipe differently.

---

# Turn on landing lag and autocancel for real aerials

The mechanics already exist.

At least the principal Smash fighter's real aerial moves should author meaningful:

```text
landing lag
autocancel windows
```

so landing during an aerial has the expected platform-fighter commitment.

Do not build a second Smash-specific landing-lag subsystem.

Exercise the generic one.

Add focused tests for:

```text
landing during active/non-autocancel aerial → landing lag
landing inside autocancel window            → little/no landing lag
```

---

# Wire shields and parry into Smash-capable characters

The body-generic shield/parry infrastructure already exists.

Once generic `fighter_abilities` is removed, appropriate characters should explicitly author:

```text
shield capability
```

and the control affordance should expose it.

Verify in actual match construction:

```text
shield raises
incoming hit is blocked
parry/rising-edge behavior works if currently designed that way
shield does not appear on a body that lacks the capability
```

Do not create `SmashShield`.

Use the shared combat system.

If the shield currently lacks genre-important feedback such as clear block hitlag/stun, measure and tune using the generic shield path.

---

# Ground dodge and real air dodge

The core already has a grounded dodge/roll path with invulnerability.

Make sure appropriate Smash fighters actually receive/use it through their character capability data.

Then inspect aerial Dodge behavior carefully.

The UI/affordance language has implied an aerial dodge in some contexts, but the existing `apply_dodge` implementation is grounded.

Implement a **real body-generic air dodge** if it is still absent.

Requirements:

```text
directional input
finite invulnerability
clear travel/velocity behavior
recovery/end lag
cannot be spammed infinitely in one airtime
refreshes according to an explicit landing/ledge/lifecycle rule
```

Model its semantic state explicitly enough that animation/debugging can distinguish it from a ground roll.

Do not merely reinterpret aerial dash as “close enough” if the gameplay state is different.

---

# Adopt the existing ledge system in Smash

There is already extensive ledge machinery:

```text
grab
hang
climb
roll
getup attack
ledge jump
drop
regrab cooldown
ledge invulnerability
momentum carry
```

The Smash fighters currently do not appear to be exercising it because the generic fighter capability set did not grant `ledge_grab`.

Give appropriate character definitions the capability.

Then verify in the real Smash stage:

```text
fall past edge
grab ledge
hang
jump from ledge
neutral getup/climb
roll getup
getup attack
drop
regrab cooldown
```

Use the generic implementation.

Fix integration/tuning defects that the first real adopter exposes.

This should have a very large next-play payoff without needing to invent a new mechanic.

---

# Add tumble / knockdown / tech / getup as body-generic combat states

This is one of the largest remaining “doesn't feel like a platform fighter” systems.

Build it generically enough that Ambition can reuse the mechanics where appropriate.

A sufficiently launched/hitstunned actor making a relevant collision should have an explicit reaction state.

Conceptually:

```text
launched / tumble
    ↓ contact during tech window
successful tech
    → tech in place
    → directional tech roll

failed tech
    → knockdown
        → neutral/slow getup
        → getup roll
        → getup attack
```

Consider walls/ceilings only if they fit cleanly after ground tech; do not block ground tech on implementing every surface at once.

Important architectural requirements:

* this is body/combat state, not a Smash-only entity marker;
* expose tuning values rather than scattering magic constants;
* rollback-register new authoritative state coherently;
* do not use `dodge_roll_timer` as the only semantic representation merely because it already provides invulnerability;
* share invulnerability projection where appropriate, but preserve distinct maneuver/reaction identity;
* add character animation slots/fallback mappings even if bespoke art is not available yet.

The user has explicitly wanted:

```text
knocked-down animation
slow/neutral getup
tech
getup attack
```

so create the architecture slots even where existing sprites must temporarily fall back.

---

# Opening countdown: 3 – 2 – 1 – GO

Smash already opens its roster suspended.

Currently the stage removes `ScriptedControl` as soon as `ActiveMatch` exists because there is no ceremony.

Replace that with an actual opening countdown.

Use the existing:

```text
opens_suspended
ScriptedControl
```

contract rather than creating a parallel input-lock mechanism.

Expected:

```text
fighters spawn
camera frames cast
3
2
1
GO
all active seats release atomically
```

Use existing HUD/banner/audio infrastructure where practical.

The countdown must be ruleset/match flow, not participant-specific hacks.

Add a deterministic test proving bodies remain held before GO and release at the transition.

This is a high-priority next-play feature.

---

# Respawn protection

After losing a stock, a returning fighter should not be immediately vulnerable during the first instant of materialization.

Add explicit Smash/ruleset-owned respawn protection.

Prefer a generic temporary invulnerability/intangibility mechanism already present in the body system.

Make the policy configurable.

Reasonable behavior:

```text
respawn
→ temporary protection
→ expires after duration
and/or
→ clears when the fighter commits an attack
```

depending on what fits the engine cleanly.

Do not bake the rule into `CharacterDefinition`.

The same character in Ambition need not receive Smash stock-respawn protection.

A full angel platform is optional; clear lifecycle protection is the important mechanic.

---

# Keep knockback policy where it is

Do not duplicate knockback formulas into character moves.

The architecture currently has a strong seam:

```text
move:
    authored base launch
    optional intrinsic growth if genuinely special

ruleset:
    default knockback growth
    DI policy
```

Smash already declares percent-scaled growth.

Ambition remains flatter by default.

Preserve this.

Use the same Robot v3 attack in both games as a regression:

```text
same attack at low/high accumulated damage

Ambition:
    approximately stable launch

Smash:
    high-damage target launches substantially farther
```

This is exactly the intended Hollow-Knight/Smash blend.

---

# Hitlag, hitstun, strong-hit feedback

Audit the actual play result once native moves land.

Do not rewrite the underlying hitlag/hitstun systems unless the measurement shows a defect.

Tune existing generic policies so strong platform-fighter hits have readable impact.

Use existing hooks for:

```text
hit freeze
camera shake
launch VFX
SFX
trails
```

where available.

Prefer scaling feedback from actual resolved hit/launch severity rather than move-name special cases.

The goal for the next playtest is for a strong hit to **feel** materially different from a weak poke.

---

# CPU AI must use the real character

After D73, Smash CPU policy should not carry a hidden whole-body archetype.

The controller profile chooses actions.

The character body exposes the available actions/capabilities.

The fighter brain already has machinery for inspecting the actual moveset and distinguishing attack/smash/special choices.

Use that.

A CPU controlling Puppy Slug should not invent a melee move.

A CPU controlling Robot v3 should understand the robot's real attacks.

CPU difficulty/strategy remains a controller-profile fact.

---

# Curated roster versus forced character

Keep:

```text
SMASH_ROSTER
```

as an explicit product/content selection if useful.

But its semantics should become simply:

> these are the characters the normal select screen offers.

It should not control whether the engine can build those characters.

Likewise:

```text
BUILDABLE_ONLY_CAST
```

must disappear before D73 closes.

Final invariant:

```text
complete registered CharacterDefinition
    → buildable

SMASH_ROSTER
    → normally selectable in this mode
```

If a test/dev caller explicitly seats an unlisted character, construction should still work.

---

# Remove stand-in/copy characters where the new provider model makes them unnecessary

The standalone Smash demo currently contains some stand-in copies of robot-lineage characters so it can run without the Ambition composition.

Once provider/character registration is clean, inspect whether the standalone demo can consume the real reusable definitions without duplicating character identity.

Prefer:

```text
same real CharacterDefinition provider
```

over:

```text
smash_duelist_a wearing Robot art
```

Do not force deletion if the standalone packaging boundary genuinely cannot import the content provider cleanly, but document the exact dependency reason.

Copies should not survive merely because they predate the new registry architecture.

---

# Character-select validity

Once D73 is complete:

```text
CharacterDefinition registered
→ buildable
```

The select screen may then validate its curated roster against that registry.

Do not allow:

```text
portrait selectable
match cannot construct it
```

but also do not make the UI roster the source of buildability.

This should permanently remove the old PCA registration class of failure.

After character registration becomes unconditional, rerun the PCA reproduction.

If the old movement/load-timing divergence still exists, keep it as a separate D74 bug.

Do not “fix” it by un-registering PCA again.

---

# Player Robot v3 and progression

Move Robot v3's actual combat definitions into normal authored character data.

Keep the distinction:

```text
character repertoire
    = what moves exist for this character

runtime progression/grants
    = which capabilities this body currently has unlocked
```

Do not duplicate move definitions per progression stage.

If a locked action is unavailable, input simply cannot execute it.

For Smash, use the character's same move definitions.

If the mode intentionally grants a canonical fighter loadout, represent that as an explicit runtime grant/policy referencing existing character capabilities.

It must not synthesize an attack that does not exist.

This is the same compositional rule demonstrated by Puppy Slug.

---

# Ambition must benefit from the Smash work

Do not create platform-fighter mechanics in a separate Smash-only physics stack.

Features such as:

```text
better hit reactions
tumble/tech/getup architecture
move-specific landing lag
directional attacks
strong attacks
shield/parry
body-generic ledges
better knockback reactions
```

belong in the reusable engine/body/combat layers where sensible.

Ambition may configure or adopt a subset.

The user's intended Ambition feel is:

> closer to Hollow Knight, but intentionally influenced by Smash.

In practice that means it is reasonable for Ambition to retain:

```text
HP-based combat
tighter/smaller rooms
lower/flat knockback growth
little/no DI
different death/checkpoint rules
exploration progression
```

while sharing the same:

```text
moves
hitboxes
hitlag
hit reactions
body capabilities
combat geometry
movement primitives
```

with Smash.

Do not fork the character for each game.

---

# Lower-priority Smash work if the primary list lands

Do these only after the character constructor, native moves, countdown, defense/recovery adoption, and reaction-state work are in good shape.

## Grab / throw foundation

Platform fighters eventually need grabs/throws.

If time remains, establish a generic contact/state model and one simple throw.

Do not create a giant throw subsystem that prevents higher-impact tasks from completing.

## Additional stages

Not a priority tonight.

One stage with good combat is more useful than three stages with weak fundamentals.

## Extensive roster balancing

Also not a priority.

Make Robot v3 and a small number of existing characters feel distinct and correct first.

---

# Tests that should prove the architecture

Do not rely only on unit tests of helpers.

Add focused end-to-end tests for the actual ontology.

## Duplicate character

```text
spawn Fretjaw twice
```

Both:

```text
CharacterIdentity == Fretjaw
```

but distinct:

```text
SimId
position
health
brain state
inventory
```

## One character through multiple contexts

Use one character through applicable paths:

```text
NPC
hostile room spawn
encounter
match CPU
match human
```

Assert intrinsic character facts agree.

Only controller/context facts differ.

## Missing explicit character

Explicit CharacterId absent from registry must error, never fall back to legacy archetype.

## Iron Mary

Ordinary Iron Mary spawn must receive Iron Mary's authored body/kit.

No shark-rider fireball kit unless an explicit override says so.

## Puppy Slug in Smash

Forced seat:

```text
can be controlled
keeps crawler body
keeps own repertoire
Attack does nothing when no attack exists
stocks/percent/blast zones still work
```

## Robot v3 cross-mode

Ambition and Smash instances share:

```text
CharacterId
moveset definitions
attack timing
attack geometry
authored damage/base launch
```

while rules differ in:

```text
death policy
knockback growth
DI
stocks
```

## Controller independence

Same character under human vs CPU:

```text
same intrinsic character
different controller
```

## Provocation

Before and after:

```text
same CharacterId
same body
same intrinsic kit
different disposition/controller
```

## Smash round flow

```text
spawn suspended
3
2
1
GO
release

stock loss
respawn protection
eventual vulnerability

last side standing
winner
```

## Ledge

Real Smash fighter can:

```text
grab
hang
ledge jump
getup
roll
getup attack
drop
```

## Air dodge

One use in airtime according to policy; no infinite spam; refresh occurs on defined lifecycle boundary.

## Tech / knockdown

Successful tech and failed-tech knockdown produce distinct authoritative state and outcome.

---

# Regression philosophy

Whenever possible, poison the old path.

Tests should distinguish:

```text
new authority actually won
```

from:

```text
both old and new happened to contain the same value
```

Use deliberately different test values.

Do not add architecture grep-policy infrastructure merely to assert one historical name disappeared.

Direct absence searches at the end are sufficient.

---

# Legacy absence checklist

Before D73 can be called complete, inspect production source and remove the old concepts where they represented this architecture:

```text
ArchetypeSpec
CharacterRoster
CharacterRosterFragment
CharacterRosterRegistry
spec_for_brain
character_archetypes.ron
HostileArchetypeId
PlayableKitSource::HostCode
PreparedKit::HostCode
BUILDABLE_ONLY_CAST
adopt_character_intrinsics
sprite_character_id as gameplay identity
display-name character identity fallback
provoked-archetype reconstruction
```

Also inspect:

```text
ActorTuning
CharacterBrainSpec
CharacterBrain
```

and either delete them or reduce/rename them to narrowly coherent responsibilities.

Do not retain the old whole-body model under a new name.

---

# Naming quality

Leave final production APIs with names that say what they own.

Good vocabulary families:

```text
CharacterId
CharacterIdentity
CharacterDefinition
PreparedCharacterDefinition
CharacterRegistry

BrainProfileId
BrainProfile
AutonomousControllerBinding

CharacterSpawnPlan
SpawnContext
LifecyclePolicy
Relationship/Disposition
```

Exact names may differ based on the final code.

Avoid final names such as:

```text
Legacy*
New*
V2*
Unified*
sprite_character_id
brain_as_character_type
```

Do not use `player` for generic body/controller concepts.

---

# Documentation hygiene

The D73 planning file became very large during investigation.

As the architecture lands, shrink it.

Keep:

```text
final architecture
field ownership
current phase
migration census still needed
remaining blockers
acceptance tests
deletion checklist
```

Remove:

```text
conversation transcripts
dated incident narratives
old hypotheses
verbatim course-correction messages
progress claims superseded by later code
```

Production comments should describe current invariants, not the history of discovering them.

---

# Overnight execution order

Use this order unless the live tree has already completed an item.

## P0 — make the new authority safe

1. Explicit CharacterId missing from prepared registry must be an error.
2. Resolve character-owned autonomous profile refs during character preparation.
3. Complete typed CharacterId through prepared registry/runtime/match seams.
4. Inspect/narrow SpawnContext before adding more callers.
5. Fix current-held-item death ownership.

## P1 — finish one common character body constructor

6. Finish CharacterIdentity.
7. Move/finalize character domain types into the appropriate low crate.
8. Make PreparedCharacterDefinition complete for intrinsic construction.
9. Route authored enemy through character-first construction.
10. Route NPC through the same body constructor.
11. Route PreparedMatch through it immediately after.
12. Route encounter, summon, programmatic paths.

At this milestone there should be **one physical character-construction path**.

## P2 — turn migration into deletion

13. Migrate clean Group-A character/archetype cases.
14. Delete each migrated legacy row as it becomes unnecessary.
15. Extract Group-B shared AI behavior into real BrainProfiles.
16. Classify Group-C generic roles.
17. Migrate provider roster fragments.
18. Delete CharacterRoster/ArchetypeSpec infrastructure.
19. Split/delete ActorTuning.
20. Remove hostile/provocation body reconstruction.
21. Remove rollback dependence on archetype identity.
22. Delete `character_archetypes.ron`.

## P3 — remove protagonist and Smash leveling exceptions

23. Move Robot v3 off HostCode to normal character data.
24. Remove `smash_fighter_kit()` as the universal replacement.
25. Remove universal `fighter_abilities` replacement.
26. Make Smash consume each character's actual body/capabilities/moves.
27. Add Puppy Slug forced-seat regression.

## P4 — next-play Smash feel

28. 3–2–1–GO opening countdown.
29. Wire shields/parry for appropriate fighters.
30. Wire grounded dodge.
31. Implement true air dodge.
32. Enable and tune existing ledge mechanics in Smash.
33. Author landing lag/autocancel on real aerials.
34. Add at least one real strong/Smash attack to Robot v3; preferably F/Up/Down if clean.
35. Add tumble/knockdown/tech/getup state and animation slots.
36. Add stock-respawn protection.
37. Tune hit feedback using existing generic hitlag/VFX/SFX/camera hooks.

## P5 — polish/integration

38. CPU AI chooses from actual character movesets/capabilities.
39. Remove obsolete Smash stand-ins where provider architecture now makes them unnecessary.
40. Rerun PCA as an unconditionally registered character; keep D74 separate if the timing bug remains.
41. Clean architecture docs and stale comments.
42. Measure deletion payoff.

---

# Do not get derailed

Do not stop D73 to investigate unrelated issues such as D74 unless they block the current integration.

If a separate defect is found:

```text
write focused reproduction
record it in planning/queue
continue the overnight campaign
```

Likewise, do not spend hours on new art assets.

Use existing animation fallbacks/slots where necessary and make the mechanical architecture correct.

Do not add more broad architecture guard frameworks merely because this is a large refactor.

The code structure and focused regressions should carry the invariant.

---

# Validation discipline

Use targeted checks as each slice lands.

Important affected packages include at least:

```text
ambition_characters
ambition_combat
ambition_platformer2d_actor_monolith
ambition_demo_smash
```

plus whichever encounter/content/provider crates the live migration touches.

Run focused tests for changed mechanics and content compiler validation.

At meaningful integration milestones run:

```text
cargo check -p ambition_app
```

and relevant runnable/demo checks available in the repository.

Do not repeatedly run giant unrelated suites while iterating.

Do not weaken existing regression tests just to make the migration compile.

Do not use `cargo fmt` as part of this handoff.

---

# End-of-night report

At the end, leave a concise report containing:

## Architecture

* which character-construction paths now use the common constructor;
* which legacy paths remain;
* whether `CharacterRoster` / `ArchetypeSpec` are gone;
* where character, controller, and spawn/session facts now live.

## Deletion payoff

Report:

```text
files deleted
legacy types deleted
legacy rows deleted
approximate lines added/deleted
```

The expected direction is substantial net legacy deletion.

If the implementation adds thousands of adapter lines while leaving all the old authority alive, consider the architecture unfinished.

## Smash

List exactly which of these are now playable:

```text
native per-character moves
Robot v3 shared Ambition/Smash moveset
strong attacks
landing lag/autocancel
shield/parry
ground dodge
air dodge
ledge options
tumble/knockdown
tech
getup options
3-2-1-GO
respawn protection
damage-scaled knockback
DI
```

Do not claim mechanics merely because infrastructure exists; distinguish:

```text
implemented and adopted in Smash
```

from:

```text
engine capability still awaiting content/adoption
```

## Tests

List the targeted tests and integration checks run.

## Remaining blockers

Only include genuine remaining work after this campaign.

Do not preserve stale TODOs that the night's work made obsolete.

---

# Definition of success

A successful morning tree should make these statements true or substantially closer to true:

> A character is authored once and can be instantiated anywhere.

> Who controls a body does not decide what body it is.

> A brain profile does not secretly choose health, movement, moves, art, or mount identity.

> A display name or sprite never determines gameplay character identity.

> Smash seats the same real characters used elsewhere rather than constructing hidden fighter archetypes underneath them.

> Forcing Puppy Slug into Smash gives you Puppy Slug, even if Puppy Slug is a terrible fighter.

> Player Robot v3 uses the same authored combat repertoire in Ambition and Smash.

> Ambition and Smash create different combat feel primarily through rules such as HP versus percent/stocks, knockback growth, DI, lifecycle, and progression—not duplicate move definitions.

> Smash begins with 3–2–1–GO and exposes real platform-fighter defensive, aerial, ledge, reaction, and recovery mechanics.

> The old `character_archetypes.ron` / `CharacterRoster` ontology is gone or visibly on its last shrinking remnants rather than hidden behind a new abstraction.

The goal is not merely to make the current tests green.

The goal is to leave Ambition with a character architecture an engine user would find obvious, and a Smash playtest that finally feels like the engine is becoming a real platform fighter.
