# Code smell backlog

Running log of smells noticed *opportunistically* while doing other work (Jon's
standing instruction, 2026-06-10). The rule: while focused on a big task, don't chase
smells — append them here so they aren't forgotten, and revisit later. Only fix inline
when the fix is very clear AND carries no risk of slowing the main task.

Append-only during runs; triage/prune during cleanup passes (move fixed items to the
Resolved section, condensed to a one-liner with the verdict/commit).

Entry format:

```
## YYYY-MM-DD <short title>
- **Where:** file:line (or module)
- **Smell:** what's wrong, one or two sentences
- **Noticed while:** the task being worked
- **Suggested fix / size:** sketch + rough effort (S/M/L)
```

---

## Open

## 2026-07-01 Portal TRANSIT-feel adapters still key on `PrimaryPlayer`, not the controlled body
- **Where:** `ambition_content/src/portal/ability_adapter.rs` (`suppress_ledge_grab_during_transit`, `warp_portal_input`) + `transit_body_adapter.rs::portal_player_input_adapter` — all `(With<PlayerEntity>, With<PrimaryPlayer>)`.
- **Smell:** the portal-gun USE path (input/fire/drop/pickup) now follows the controlled subject / holder (2026-07-01), but the transit-FEEL side effects (wall-ability suppression during transit, input warp on emergence, `PortalEmission`/`TrailContinuityBreak` traces) still only apply to the primary player. A possessed actor crossing a portal transits physically (transit_body_adapter tags every body generically ✓) but gets none of these input/feel guards, so possession-through-a-portal feels wrong. Not a gun-USE bug, so it was scoped out of the use-path pass.
- **Noticed while:** the controlled-body pass (portal gun → holder).
- **Suggested fix / size:** M — resolve `ControlledSubject` (fallback primary) and gate these on the transited body being the controlled subject, mirroring the input/fire adapters. Needs the transit event to carry / be matched against the controlled body.

## 2026-07-01 Docs reference a removed `CharacterArchetype` (né `EnemyArchetype`) ENUM — ✅ RESOLVED 2026-07-11
- **RESOLUTION:** the actual live stale refs (7, most of the original 8 had already been fixed by later refactors) reworded to the real path (`spec_for_brain`: brain key → `CharacterArchetypeSpec`): `spawn_actors.rs:75,882` + `entity_catalog/placements.rs:71` (Rust doc), and 4 RON comments (`character_catalog.ron` ×3, `character_archetypes.ron` ×1). `enemies/mod.rs:553` is CORRECT as-is (it names the *deleted* `CharacterArchetype::X.spec()` to explain what its test fixture replaced). Pure docs; no code change.
- **Where:** 8 sites — `features/ecs/spawn_actors.rs:75,641`, `features/enemies/mod.rs:527`, `ambition_characters/src/actor/mod.rs:186`, and comments in `character_catalog.ron` / `character_archetypes.ron`.
- **Smell:** doc comments call methods on a `CharacterArchetype` enum (`::from_brain`, `::attacks_player`, `::RangedSkirmisher`, `::X.spec()`) that no longer exists — the named-archetype enum was removed in an earlier refactor; only the `CharacterArchetypeSpec` STRUCT + a brain-key string map (`CharacterRoster`) remain. Classic docs-describe-dead-things (per the standing rule). Surfaced during the step-6 roster rename (the `Enemy*`→`Character*` sed renamed the dead refs in place, so they now read `CharacterArchetype::…`).
- **Noticed while:** unified-actors step-6 rename (character-roster vocabulary).
- **Suggested fix / size:** S — reword each comment to describe the real path (brain-key → `CharacterArchetypeSpec` via `CharacterRoster`), dropping the phantom-enum method calls. Pure docs; no code.

## 2026-07-02 STILL OPEN after control pass 3 — the two remaining forks — BOTH RESOLVED 2026-07-02 (see below)
- **Melee: two DRIVER systems remain. [RESOLVED 2026-07-02, commit 76b92010]** `attack_advance_system` (player) and `start_enemy_melee_from_brain_actions` + the `update_ecs_actors` inline active-edge (actor) are DELETED, replaced by ONE body-generic pair `combat::attack::{start_body_melee, advance_body_melee}` over `BodyClusterQueryData` (no `PlayerEntity` filter). The actor melee TIMER advance (`em.attack.tick`) is out of `em.update` — movement integration owns movement only; the AI reads `em.attack` as of the prior frame's advance (one consistent view). `start_attack`/`advance_attack` are body-generic (Option anim; effective-faction picks the damage channel the resolver already distinguishes; sprite_character_id picks the manifest box). Pinned by `unified_melee.rs` (player + hostile actor, same lifecycle), `possession_end_to_end.rs`, `enemy_attacks_player.rs`.
- **Boss possession = movement only. [RESOLVED 2026-07-02, commit bc6b0400]** Boss authored specials are now a persisted body CAPABILITY (`BossCapability`, derived from the pattern cfg at spawn, surviving the brain swap); the `Brain::Player` boss arm maps attack → primary strike / special → signature content special onto the SAME `BossAttackState`; `update_ecs_bosses` gates player-damage off for a player-controlled boss. Pinned by `boss_possession_specials.rs`.

## 2026-07-01 Player vs actor MELEE: two DRIVER systems around shared primitives (control fork resolved)
- **Where:** `combat/attack.rs` (`attack_advance_system`, player-cluster) vs `features/ecs/actors/update.rs` active-edge + `brain_effects.rs::start_enemy_melee_from_brain_actions` (actor-cluster).
- **Smell:** the melee is already ~80% converged — BOTH paths use the SAME `BodyMelee { swing: Option<MeleeSwing> }` component, the SAME spec pipeline (`resolve_attack_intent_from_view` → `attack_spec_from_view` → `into_world_frame`, see `begin_melee_attack`), and the SAME `spawn_melee_strike`/`emit_melee_slash`. The CONTROL fork is gone: `attack_advance_system` now consumes by `ControlledSubject` (not `PrimaryPlayerOnly`) and the actor consumer is keyed by `msg.actor` — neither is a primary-player identity gate. What remains is TWO thin DRIVER systems wrapped around the shared primitives (the player's has pogo + `PlayerAnimState` + self-impulse/commit; the actor's ticks inside the `update_ecs_actors` monolith).
- **Noticed while:** control-convergence pass 2.
- **Suggested fix / size:** L — the fighter-unification mega-project (memory `project_fighter_unification`, S4/S5/S6). Merge into ONE `advance_body_melee` querying `ae::BodyClusterQueryData + BodyMelee + ActionSet` (matches player AND actors — both carry the ancillary clusters), pulling melee timing/active-edge out of `update_ecs_actors`. Deliberately NOT done blind in the control pass: it changes the player's core combat feel (pogo/directional/timing) and can't be GUI-verified here — the exact "right shape first, verify feel after" work that needs Jon at the controls. Bosses stay separate (no ancillary clusters).

## 2026-07-01 Possessed BOSS moves but can't yet trigger its specials
- **Where:** `features/ecs/bosses/tick.rs` (`tick_boss_brains_system` `Brain::Player` arm).
- **Smell:** bosses are now architecturally possessable — the `unreachable!("bosses never Brain::Player")` and the `Without<BossConfig>` possession-candidate exclusion are gone, and a possessed boss reads slot input → moves via `velocity_target` through its float integrator. But boss ATTACKS are authored as scripted `BossPattern` profiles (`BossAttackState`), which the possession arm suspends — there's no mapping yet from player input → a chosen boss special.
- **Noticed while:** control-convergence pass 2 (bringing bosses into `Brain::Player`).
- **Suggested fix / size:** M — map player action input (attack/special press) to the boss's authored specials (e.g. cycle/select a `BossAttackProfile` and emit its `ActorActionMessage::Special`). Design question: which input picks which special. Progression-gating of WHICH boss is possessable is a separate targeting-policy layer above the body model.

## 2026-07-01 HUD / debug overlay still read the home avatar, not the controlled subject
- **Where:** ~~`ambition_render/src/hud.rs`~~ (player-facing status HUD — RESOLVED 2026-07-01), `ambition_app/src/app/hud.rs`, `ambition_app/src/dev/debug_overlay.rs` (dev surfaces — still `PrimaryPlayerOnly`).
- **Smell:** camera / portal viewer / nameplates now follow `ControlledSubject` (the body carrying `Brain::Player`), but HUD + debug overlay still key on the home avatar. While possessing, they show the home body's health/gizmos, not the driven body's.
- **PARTIAL RESOLUTION 2026-07-01:** the player-facing status HUD (`ambition_render/src/hud.rs`) now reads the **controlled subject** for EVERY stat — HP, mana, AND money — and refills the controlled body's mana. Economy is a body concern (an NPC / merchant carries its own `BodyWallet` + inventory; cf. Fork E pickup-on-`ControlledSubject`), so the wallet is just another body cluster the driven body may hold — `Option` only because not every body carries one yet (a wallet-less body reads `$0`). Possessing a body spends ITS purse, not the home avatar's. One body-generic query drives it all. Pinned by `hud::tests::hud_tracks_the_controlled_body_for_every_stat_including_money`.
- **Still open (dev surfaces, lower priority):** (1) `app/hud.rs`'s debug text HUD is *intentionally* primary-scoped (its own doc says co-op would get per-slot panels, not a generalization of this one) — arguably not a bug. (2) `debug_overlay.rs` mutably queries the player CLUSTER with `Without<FeatureSimEntity>` to dodge a B0001 conflict; repointing it at a possibly-`FeatureSimEntity` subject reintroduces that conflict, so it needs a read-only stat split from the mutable gizmo-preview query first.
- **Noticed while:** possession/control unification (this refactor).
- **Suggested fix / size:** S remaining (debug-overlay stat labels → controlled subject via a read-only sub-query); the player-facing win is done.

## 2026-06-26 Characters are defined by named `EnemyArchetype` rows, not by their movement kit
- **PARTIAL 2026-07-15 (commit 6875aeaea):** the PLAYER-ability axis of this now composes — `AbilitySet` gained `union`/`intersect`, `AbilityGrant` is a composable vocabulary, and a catalog row lists grants that union into `AbilityBase` (see the RESOLVED item ~line 386). The ENEMY-archetype-row bundle (HP + brain + `smash_can_*` caps as one frozen row) is STILL open — that is the bigger fish this entry is really about.
- **Where:** `ambition_content/assets/data/enemy_archetypes.ron` + `EnemyArchetypeSpec` (`ambition_actors/src/features/enemies/mod.rs`); the spawn path resolves a string brain-key → a fixed archetype row that bundles HP + tuning + brain template + the capability flags (`smash_can_blink`, `smash_can_fly`, melee/ranged specs, …).
- **Smell (Jon, 2026-06-26):** "There really shouldn't be archetypes; characters should be defined by what movements they have available to them." An archetype is a frozen bundle; the elegant model is a character = a **capability/kit set** (which verbs its body has: blink, fly, shield, dash, ledge, melee/ranged shapes, tilts, special) + tuning, composed freely, not picked from a closed roster of named rows. The S3 capability work is incrementally pushing this way (each verb is now a per-body `CombatCapabilities` flag projected from the spec into the body AND the brain), but the *source* is still a named archetype row rather than a kit the body simply HAS. The closed-archetype shape is the body-side analogue of the closed-`SpecialActionSpec`-enum tension already noted for the engine-for-other-games goal.
- **Noticed while:** wiring blink (S3a) + fly (S3b) as body capabilities for the PCA — each verb needed a `smash_can_*` field threaded through the archetype row → spec → (brain cfg + body caps), which is the seam a kit-first model would make unnecessary.
- **Suggested fix / size:** L, NOT now (explicitly deferred by Jon — "just a smell to log"). Direction: let a character author its capability set + tuning directly (data), drop the named-archetype indirection; the brain reads the kit, the body enforces it. Dovetails with the fighter-unification roadmap's "per-body capability set" and the engine-for-other-games keystone.

## 2026-06-26 `BrainSnapshot.wall_contact` is defined + read but NEVER populated in production
- **Where:** field `ambition_characters/src/brain/snapshot.rs` (`wall_contact: Option<WallContact>`); read by `Wanderer` (`state_machine/mod.rs::tick_wanderer`); the ONLY `Some` constructions are in tests (grep `wall_contact: Some` → test files only).
- **Smell:** an aspirational seam. The Wanderer's climb-vs-reverse + chatter-pause logic keys on `wall_contact`, but every production snapshot builder sets it to `None`, so the puppy-slug's wall reactions never fire in-engine (it relies on a separate integrator-side facing flip, making the brain branch dead). It also looked like the natural seam for AI anti-corner during the duelist work — I deliberately did NOT use it precisely because it's unpopulated (would have been inert in-game).
- **Noticed while:** looking for a production-faithful wall-awareness signal for the duelist neutral game (rejected it, used target-relative footsies instead).
- **Suggested fix / size:** M — either populate `wall_contact` in the enemy snapshot builder from the integrator's already-computed wall-stop state (the `perp` side-speed stall check in `integration.rs`), or delete the field + the Wanderer branch if no consumer will wire it. Right now it's the worst of both: present, read, and dead.

## 2026-06-26 `ObservationFrame` flat-struct field additions ripple to 3 test literals
- **Where:** `ambition_characters/src/brain/smash/{action,mode,emit}.rs` each have an `obs_at(...)` test helper that builds a full `ObservationFrame { .. }` literal.
- **Smell:** the same shape as the 2026-06-23 `BrainSnapshot` entry, one level down. Adding `self_aerial` forced edits to all three. There is no `ObservationFrame::idle()`-style constructor to `..` from.
- **Noticed while:** adding the `self_aerial` field for the aerial brain.
- **Suggested fix / size:** S — add a `#[cfg(test)] ObservationFrame::at(distance_x)` (or a `Default`) in `observation.rs` and have the three `obs_at` helpers delegate, so a new field touches one place.

## 2026-06-21 Dead `landed`/`killed` scaffold in `advance_attack` (+ possibly-broken pogo-off-enemy)
- **Where:** game/ambition_app/src/app/world_flow/attack.rs ~316-347 (`advance_attack`)
- **Smell:** `let landed = false; let killed = false;` are hardcoded (synchronous hit resolution moved to the ECS damage queue), so every block gated on them is dead: the connect-sound `SfxMessage::Hit` (line ~321) AND — more worryingly — the pogo-impulse-on-landing block (`if landed && abilities.pogo && spec.can_pogo ...`, ~329-347). Pogo off the *orb* is a separate live path (~260-285); pogo off a *landed enemy hit* via this block can never fire. Either it's genuinely broken (needs migrating to the ECS damage queue like the connect sound was) or it's residue to delete. Found while answering "is there an attack-connect sound?" — answer: yes, the generic `SfxMessage::Hit` from `features/ecs/damage/mod.rs:307`, NOT this dead site; there is no *distinct* hit-confirm cue.
- **Noticed while:** investigating the mockingbird "swing doesn't fire / doesn't register while overlapping the boss" bug.
- **Suggested fix / size:** S to delete the dead connect-sound lines; M to decide+fix the pogo-off-enemy path (verify in-game whether enemy pogo-bounce currently works, then either migrate to the ECS queue or remove). Don't blind-delete — the pogo block may be a latent bug, not just dead code.

## 2026-06-13 Docs reference deleted RON-based levels
- **Where:** docs that predate the LDtk-only world source (ADR 0009's "Consequences" implies RON-world-authoring docs remain unswept). NOTE: `check_doc_links.py` passes (links resolve); this is about stale *prose* describing removed RON room/world authoring, not broken links.
- **Smell:** RON-shaped room/world levels were fully removed (LDtk is the only world source), but some docs still describe them as if extant. Jon's standing rule: a doc describing something that no longer exists is a smell.
- **Suggested fix / size:** S — grep docs for "RON room|world|manifest|level" prose, archive or rewrite.

## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit
- **Where:** crates/ambition_actors/src/mechanics/combat/events.rs (FeatureVisualKind)
- **Smell:** a named-ish variant in kit vocabulary (excluded from the combat-kit guard word list).
- **Suggested fix / size:** S — rename to TrainingDummy, BUT it touches LDtk/content mapping, so do it with `ambition_ldtk_tools`, not a blind rename.

## 2026-06-10 Special-attack EFFECTS consumers are half-vocabulary (post de-name)
- **Where:** crates/ambition_actors/src/features/ecs/brain_effects.rs (spawn_gnu_apple_rain_*, spawn_overfit_volley_*, LockOnBeam/PitTrap/RotatingCross/MinionCascade consumers); SpecialActionSpec docs in ambition_characters/src/brain/action_set.rs
- **Smell:** the BossAttackProfile de-name is honest at the key/schedule/geometry/param layers, but the consumer impls still bake content (apple art identity, gnu-named fns, "GNU-ton boss:" spec docs).
- **Suggested fix / size:** M — lift baked constants + projectile-art identity into RON spec fields; rename consumers to the vocabulary. The active target of the Technique/Effects framework design (2026-06-13).

## 2026-06-15 Gravity-inversion residual design questions
Found via the headless `gravity_symmetry.rs` harness. The input-frame gates (crouch,
drop-through, attack-pogo, fast-fall, possession, ladder-jump, ledge, patrol wall-stop)
are now gravity-relative. These four remain world-Y-locked — each is a DESIGN question
(should it be gravity-relative?), cheaply verifiable by adding a symmetry case:
- **Directional attack hitbox offset** — `ambition_combat/src/lib.rs:446` (`view.pos + spec.hitbox_offset`): down/up/forward offsets are world-locked, so directional attacks are screen-relative.
- **`ground_gap_below_feet`** — `ambition_app/src/app/world_flow.rs:63` probes world-down for landing feedback.
- **Thrown ground-item physics** — `ambition_actors/src/items/pickup/mod.rs:169` (`GROUND_ITEM_GRAVITY`): thrown items fall world-down regardless of the gravity field.
- **Player knockback** — `apply_player_hit_events` builds `editable_tuning.as_engine()` without `apply_gravity_dir`; UNTESTED under a flip.

## 2026-06-21 Sprite-renderer path helpers duplicated + generated dir scattered
- **Where:** `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/cli.py` (`package_dir`/`repo_root`/`sandbox_sprites_dir`/`generated_dir`) vs the now-deleted `paths.py`.
- `paths.py` (`package_root`/`tool_root`/`repo_root`/`generated_root`/`sandbox_sprites_dir`) was the *better-factored* version — repo_root searches upward for `crates/`+`tools/` instead of hardcoding `parents[3]` — but it was **orphaned** (zero importers). Deleted it as dead code 2026-06-21; cli.py keeps its working copies.
- **TODO:** extract cli.py's path helpers into `registry/paths.py` (one home, the upward-search impl) and have cli import them. NOT done in the org pass because `cli.generated_dir(name)` = `DEFAULT_ASSET_DIR / name` is *semantically different* from `paths.generated_root()` = `tool_root()/generated` — they're not 1:1, so the dedup needs care, and the pixel parity harness does not assert output *paths* (pytest's draw_all/install tests do, partially).
- Related: generated output lands in **three** dirs — `generated/`, `targets/generated/`, and the tool-root `generated/` (all gitignored now, so a consistency smell, not a git-hygiene problem). Pick one canonical generated root when doing the path dedup.

## 2026-06-23 `BrainSnapshot` test helpers rebuild full literals across crates (leave for now)
- **Where:** per-module test helpers `snap_at` (`ambition_characters/src/brain/state_machine/tests.rs`), `snap_with_target_at_x` (`ambition_characters/src/brain/smash/mod.rs`), plus full `BrainSnapshot { .. }` literals in `ambition_actors` tests (`features/conversion_tests.rs`, `features/ecs/spawn/tests.rs`).
- **Smell:** each rebuilds the whole `BrainSnapshot` literal instead of starting from the existing `BrainSnapshot::idle()` constructor + overriding the one or two fields each test cares about. A new field means touching all of them.
- **Why left (not merged):** they span two crates and each helper parameterizes different fields (`target_x` vs `pos_x + target_x`); a single cross-crate shared fixture would be premature generalization. The cheaper, in-scope win is to have each helper build on `..BrainSnapshot::idle()` rather than a full literal — but `idle()`'s defaults may not match every test's intent, so it needs a per-helper check, not a blind sweep. Noticed during the boss de-bloat sweep (the boss fixtures WERE consolidated; these are the actor/brain analog).
- **Suggested fix / size:** S per helper — reduce each to `BrainSnapshot { field: x, ..BrainSnapshot::idle() }` after confirming `idle()` matches its baseline. Production `BrainSnapshot { .. }` constructions (`actors/update.rs`, `player/systems.rs`) are distinct real builds — leave them.

## 2026-06-23 Boss phase→music mapping looked duplicated but is single-source (verified, leave)
- **Where:** `boss_encounter/systems.rs::phase_music_track` vs `boss_encounter/events.rs::publish_events`.
- **Finding:** NOT a duplicate. `phase_music_track` is the only place that maps a `BossEncounterPhase` to its `BossEncounterSpec` music field; it's called twice within `systems.rs` (the active-track collector + `phase_event_to_encounter_events`). `publish_events` only *consumes* an already-resolved `MusicRequested { track }` string — it never re-derives the phase→track mapping. (`publish_events` DOES own the sole phase→banner-*text* mapping, also single-source.) Recorded so the next reader doesn't re-chase it as a dedup target.

## 2026-06-23 Settings: 3 IR options absent from the pause-menu SettingsItem surface (gap, behavior-change to fix)
- **Where:** `persistence/settings/model/mod.rs` (`SettingsItem` enum + `rows_for` + `shared_option_id`) vs `menu/ir/settings/mod.rs` (`SettingsOptionId`) + `menu/ir/system/mod.rs` (curated_options) + settings `apply`/`build`.
- **Smell (already-diverged parallel list):** `FramePacing`, `PortalReverseFacing`, `InputFrameMode` exist in the IR, the System-cube curated_options, and `apply_settings_option`, but have NO `SettingsItem` variant / `rows_for` entry, so the pause menu can't surface them. The comment at `model/mod.rs:181-183` claims the "stage 3b" IR-bridge migration is complete, which contradicts this.
- **Why NOT fixed here:** resolving it ADDS user-facing pause-menu rows = a behavior change, out of scope for a behavior-preserving de-bloat pass. Also may be an intentional subset (pause menu < cube menu) rather than a bug. Needs Jon's call.
- **Suggested fix / size:** M — if the pause menu should mirror the cube: add the 3 `SettingsItem` variants + `rows_for` + `shared_option_id` mappings; then update the `:181-183` comment. If intentional, add a one-line note that the pause surface is deliberately a subset. The whole class would benefit from a test asserting every `SettingsOptionId` reachable via the cube is either in `SettingsItem` or on an explicit "pause-omitted" allow-list.

## 2026-06-23 Manual `const ALL` lists shadow exhaustive enum matches (drift-prone, add a guard)
- **Where:** `assets/game_assets/entity_sprite.rs` (`EntitySprite` enum + `const ALL` at ~:104 + `relative_path`/`entity_sprite_asset_id` matches); `assets/game_assets/mod.rs` (`ParallaxTheme`/`ParallaxLayerAsset` enums + their `const ALL` + `key()`/`from_key()`).
- **Smell:** the `match` arms are exhaustive (compiler catches a missing variant), but the hand-maintained `const ALL: [_; N]` is NOT — adding a variant without updating `ALL` silently drops it from every `ALL`-driven iteration (asset preload, round-trip checks). No current mismatch.
- **Why not fixed:** the only robust fix is a guard test, and `ALL.len() == <variant count>` needs `std::mem::variant_count` (nightly) or a strum-style derive; a plain test that round-trips every `ALL` entry through `key()`/`from_key()` is the pragmatic option. Adding tests is lower-value than dedup and preventive only.
- **Suggested fix / size:** S each — a round-trip test per enum (every `ALL` entry resolves through its match and back), or adopt a derive that generates `ALL`.

## 2026-06-23 Diverging test fixtures NOT consolidated (verified, leave)
- **`EncounterSpec` builders** — `encounter/state.rs::spec()`, `encounter/rewards.rs::spec_with_trigger()`, `encounter/tests.rs::lab_spec()` each rebuild the 9-field `EncounterSpec`, but with different values (camera_zoom 1.2/1.0/1.5, reward `Health{2}` vs `default_encounter_reward()`) and different parameterization (waves vs trigger vs fixed). A shared builder would need a base+override shape; values already differ per test, so forcing it risks changing fixtures tests depend on. Left.
- **`LdtkProject` synthetic fixtures** — `world/ldtk_world/tests/kinematic_paths.rs` has 6 `LdtkProject` literals. The world-audit agent rated them ~95% identical, but that held only for the first two: across all six the level dimensions diverge (only 2 use 640×480 / 40×30; one project has multiple levels). A single `synthetic_project(id, entities)` helper won't fit without a wide builder (px_wid/hei/c_wid/hei params), at which point the call sites aren't much shorter. Left.

## 2026-06-23 Portal LDtk-emission field vs helper feature-gate mismatch
- **Where:** `world/ldtk_world/conversion/mod.rs` — `RuntimeEntityEmission` fields `portal_gun_spawns`/`portals` are `#[cfg(feature = "portal")]` but their helper methods `portal_gun_spawn()`/`portal()` are `#[cfg(feature = "portal_ldtk")]`.
- **Smell:** two gates for one concern. `portal_ldtk` implies `portal` (per Cargo.toml) so it compiles, but a reader checking the field gate would mis-predict the helper gate.
- **Why not fixed:** changing a cfg gate can shift what compiles under each feature combo — needs a per-feature build check, not a blind edit; not behavior-preserving by inspection alone.
- **Suggested fix / size:** S — align the helper gates to the field gate (or add a one-line comment explaining the asymmetry), then build under `--features portal` and `--features portal_ldtk` to confirm.

## 2026-06-23 `dialog_lint` fixed-arity command table is hand-synced + untested
- **Where:** `dialog_lint.rs` (`FIXED_ARITY_COMMANDS` ~:19-31) must match the `In<...>` arities of the `cmd_*` fns in `dialog/yarn_bindings.rs`; the comment says "MUST match" but nothing tests it.
- **Smell:** a new dialog command added without updating the table is silently un-linted. Manual parallel list, no guard.
- **Suggested fix / size:** M — a test that scrapes the `cmd_*` signatures (or a registry the bindings already build) and cross-checks the table.

## 2026-06-26 Glob-import seam map (from `clippy::wildcard_imports` sweep)
- **Context:** `cargo clippy --fix -W clippy::wildcard_imports` over the workspace. Preludes are correctly exempt; only **4** named globs auto-expanded losslessly (committed `dbe143c4`). The rest are the smells.
- **~93 named globs clippy *refuses* to auto-expand** — these are the seam-y ones: re-export façades, enum-variant globs (`use PortalChannelColor::*;`, `use Enum::*`), and names fed into macros. clippy marks them `MaybeIncorrect`, so they need manual judgement. This refusal set *is* the untangle worklist; expanding one often reveals a façade module that should be a curated `pub use`.
- **Production `use super::*` is pervasive (143 non-test sites).** clippy's default only exempts `super::*` *inside test modules*, so it would expand all 143 production ones (out of scope for the named-only pass). Worst case observed: `ambition_content/src/bosses/specials/gradient_sentinel.rs` — `use super::*` expands to a 27-name grab-bag including std `vec`/`format`/`ToString`, i.e. `specials/mod.rs` is re-exporting a kitchen-sink prelude. That re-export hub is the real smell to break up.
- **Tooling note:** `cargo clippy --fix` applies *every* active lint, not just the `-W` one — the broad run also silently did `derivable_impls` (lossy: collapsed a `#[cfg(target_os="android")]` `MenuTapMode::default()` into `#[derive(Default)]`, dropping the Android branch), `explicit_auto_deref`, and a `PHI_FRAC` float-precision rewrite. `-A warnings -W clippy::wildcard_imports` over-suppresses and fixes nothing. So: run broad, then revert every hunk that isn't a glob expansion. No clean single-lint `--fix` incantation found.
- **Suggested fix / size:** L, incremental. Per façade module: expand the glob, see what leaks, convert the hub to an explicit curated `pub use`. Start with `bosses/specials/mod.rs`.

## Resolved

- **2026-07-01 Control convergence pass 2: body SEMANTICS follow the controlled body** — the first pass moved `Brain::Player`; this pass converged what "controlled" MEANS. (1) Body locomotion CAPABILITY vs AI POLICY: peaceful NPC `max_run_speed` is now the body's physical top speed (player's `MAX_RUN_SPEED`), `patrol_speed` stays policy expressed as normalized `locomotion_for` intent — a driven NPC sprints, an autonomous one ambles. (2) Body attack CAPABILITY vs policy: peaceful NPC `ActionSet` derives from its authored combat kit (not empty `peaceful()`), so a driven NPC throws its authored punch while its peaceful brain never presses attack autonomously. (3) Movement-verb taxonomy: flight-mode bodies no longer consume the buffered grounded jump (gated in the one engine jump handler), so a possessed flyer steers vertically instead of leaping. (4) Ability ORIGIN = `ControlledSubject`: blink (+ its in-game preview reticle), grapple, dive, mark/recall, beam, volley, vortex, meteor, puppy-slug, held-weapon fire key on the driven body, not `PrimaryPlayer`. (5) Effective allegiance: `combat::targeting::effective_faction` (a `Brain::Player` body fights as `Player`) replaces the possession faction FLIP — authored `ActorFaction` is never mutated (targeting + incoming-damage + outgoing-stamp all resolve through it). (6) Bosses are architecturally possessable (see Open note). (7) `resolve_controlled_subject` asserts exactly-one `Brain::Player(PRIMARY)`. (8) `PrimaryPlayer`/`PrimaryPlayerOnly` docs now say "home avatar identity, NOT the controlled body". Commit `727ba5d1` + this pass. Remaining (Open): the two melee DRIVER systems (fighter-unification), boss-specials-under-possession, HUD/debug subject.
- **2026-07-01 Bifurcated control/combat: possession was input-copy, not control transfer** — the home avatar and a possessed actor ran PARALLEL control paths (movement via a `Possessed { control }` input-copy override in `update_ecs_actors` + `sync_possession_input`; the home body suppressed by a `not_possessing` run-condition), while attack still consumed from the home body (`attack_advance_system` on `PrimaryPlayerOnly`) — so a possessed attack came from the vacated body. Collapsed to ONE model: control authority = the entity carrying `Brain::Player(slot)`. Added the slot-input model (`SlotControls`, `PlayerSlot -> ControlFrame`); possession is now brain TRANSFER (move `Brain::Player(PRIMARY)` off the home body onto the target, restore on release); `update_ecs_actors` drives any `Brain::Player` body through the same brain tick via slot input (deleted the possessed override + `POSSESSED_MOVE_SPEED`); `attack_advance_system` consumes for the `ControlledSubject`; camera/portal/nameplates derive from `ControlledSubject`; effective allegiance via a faction flip at transfer (not a `Possessed` marker); melee/ranged effect consumers gate on ActionSet CAPABILITY, not `disposition.is_peaceful()` AI policy. Deleted `Possessed`, `sync_possession_input`, `possession_active`, `not_possessing`. Pinned by the possession test module (brain transfer / restore / exactly-one-player-brain / attack-only-from-target / target-lost). Remaining forks logged Open above (two melee state machines; HUD/debug subject).
- **2026-06-26 Enemy `BrainSnapshot.sim_time` hardcoded to 0.0 → reaction latency inert** — `update_ecs_actors` now threads a real accumulating sim clock (`GameplayElapsed`, summed from `WorldTime::scaled_dt`) into `build_enemy_brain_snapshot` instead of `0.0`, so the Smash brain's `obs_history` reaction latency works in-game. `518838df`; pinned by `gameplay_clock_accumulates_scaled_dt`.
- **2026-06-17 Patrol wall-stop read screen-vel.x** — under sideways gravity the patrol "reverse facing" detection watched the zeroed gravity axis and never fired (enemy ground into the wall). Now watches the gravity-perpendicular side velocity in both grounded integrators. `5c29c4a9`; pinned by `patrol_enemy_reverses_facing_at_a_wall_under_sideways_gravity`.
- **2026-06-17 Vestigial `PlayerPlatformRideState`** — write-only after riding became emergent; removed across the chain. `2a5aafde`.
- **2026-06-15 Dual inventory bags** — `ItemKind`/`PlayerInventory` deleted, collapsed onto `OwnedItems`/`Item`; dialogue can now grant any of the 24 items.
- **2026-06-15 Boss sprite assets** — 7 named `GameAssets` fields + per-boss loaders collapsed to one `boss_sprites: HashMap<&str, _>` + a data table; renderer names no boss. Replay bit-identical.
- **2026-06-10 ItemKind/Item enum split** — dual-bag half resolved (above); the 24-row `Item` enum INTENTIONALLY kept (type-level equip/ability wiring; narrow closed enum preferred over a wide registry). Won't-do-by-analysis.
- **2026-06-10 BossAttackProfile brain enum** — LEAVE the enum: the named melee variants are SHARED attack-shape vocabulary across many bosses; content-specific specials route through `Special(String)` (consumers already in `ambition_content::bosses::specials`). Won't-do-by-analysis.
- **2026-06-10 audio/music runtime extraction** — music-cue catalog moved to `ambition_content::music`; 2026-07-06 E1b later moved the reusable SFX-bank loader/drain into `ambition_audio` and deleted the old `audio/runtime.rs` fallback. Remaining gameplay-core audio/music files are sandbox adapters, not foundational crate material.
- **2026-06-10 check_doc_links red** — links now resolve (`check_doc_links.py` passes). Residual suggestion: add it to CI to prevent future drift.
- **2026-06-15 Gravity-relative input frame** — crouch / drop-through / attack-pogo / fast-fall / possession / ladder-jump / ledge route their vertical "descend" gate through `ae::movement::gravity_descend`. One-way landing already gravity-symmetric.
- **2026-06-14 Cube edge-button (page-turn) duplicated per face** — extracted one `edge_button_nav(...)` consumed by both callers; the only per-face difference is the `EdgeInward` param.
- **2026-06-13 EnemyConfig.archetype tuning hub** — `EnemyArchetype` enum deleted; roster lifted to `ambition_content`, enemies resolve by brain-key against an installed `EnemyRoster`. Guarded by `architecture_boundaries_enemy_config_is_archetype_free`.
- **2026-06-15 audio/mod.rs `use bevy::prelude::*` "unused"** — load-bearing false positive (child modules re-glob it via `use super::*`). DO NOT remove.
- **2026-06-21 Alpha-clobber audit surface (sprite renderer)** — drawing a translucent fill straight onto an RGBA image with `ImageDraw.Draw(img)` *replaces* the destination alpha (clobbers underlying content) instead of blending; correct path is a scratch layer + `Image.alpha_composite` (the "gnu_ton rule"). Flagged by Jon (recurring agent mistake; likely latent bugs exist). Added the canonical primitive `core/draw.overlay_draw` (+ `composite_polygon`), pinned by `tests/test_core_overlay.py`. TODO: (a) unify the 3 existing scratch-layer copies onto `overlay_draw` — `generic_explosions._overlay_draw` DONE (delegates to core, parity-clean, since core uses the same `"RGBA"` scratch mode); `skeleton.composite_polygon` uses PLAIN `Draw` (not `"RGBA"`) so its unification would shift overlapping-translucent pixels → needs a parity-checked bless; rigdoc painter still TODO. (b) audit the ~139 plain `ImageDraw.Draw(img)` sites for translucent-over-content clobbers — **the pixel parity harness CANNOT catch these** (they render consistently wrong, so there's no before/after drift). Needs eyeball/heuristic, not the harness.
- **2026-06-25 `DebugOverlayLabels` leaks a `pub(crate)` type through a `pub` field** — `dev/debug_overlay/prims.rs:198` `pub struct DebugOverlayLabels(pub Vec<DebugLabel>)` exposes `DebugLabel` (only `pub(crate)`) at `pub` visibility → `private_interfaces` warning. Pre-existing; surfaced when the app-module blanket `#![allow(unused_imports)]` came off (2b2b88f1) but unrelated to it. Fix: make the field `pub(crate)` (or `DebugLabel` `pub`). Left unaddressed — out of scope for the glob untangle.
- **2026-06-26 [RESOLVED 2026-06-28, commit cc3e972e] precision-blink test was stale, not a real bug** — the test (`player.rs`) asserted quick-blink `blink_quick_dir == (-1,0)` (locomotion-framed) under sideways gravity, but `InputFrameMode::DEFAULT_MOVEMENT` is `ScreenRelative`, so quick blink IS screen-relative by default (got `(0,-1)`), exactly like precision blink — matching in-game behavior. Rewrote the test to pin BOTH the screen-relative default AND the seam (flipping only the locomotion mode to body-relative rotates quick blink while precision stays screen-directed).
- **2026-06-28 Duel "Player" robot renders ~21% smaller than the controlled player** — NOT a stale/duplicate character (same `player_robot` sheet, same `player_robot` archetype → same Hadouken+Swipe VFX). Two render-sizing paths diverge: the player uses `player_placeholder_render_size` = `sprite_render_size_scaled(spec, 30×48, 1.16)` (the `PLAYER_PLACEHOLDER_VISUAL_SCALE` "heroic" boost) while the generic actor path (`sprite_render_size_for_name` → `sprite_body_collision_for_character_id`) uses `sprite_render_size(spec, ldtk_box)` with no heroic scale and the duel's 28×46 box (vs the player's 30×48). Net: 48·1.35·1.16 = 75.2 px tall (player) vs 46·1.35 = 62.1 px (duel robot). Elegant fix = promote the 1.16 from the player ENTITY to a character-catalog `render_scale` (display-only — must NOT leak into the body-metrics-derived hitbox in `sprite_body_collision_for_character_id`), so every spawn of the `player` character (player or duel copy) draws identically; then the duel robot also wants the player's authored 30×48 body box. Deferred: blind visual change (touches the PLAYER's render path + catalog schema) needing GUI confirmation of the target look — offered to Jon as a confirmable follow-up.
- **2026-06-27 `enemy_archetypes.ron` / `EnemyArchetypeSpec` / `EnemyRoster` / `EnemyBrain` are misnomers** — now that the protagonist is authored as the `player_robot` archetype (and the roadmap makes player/enemy just controller+capability DATA, not types), these are *character* archetypes, not "enemy" ones. Flagged by Jon. The rename (file + `EnemyArchetypeSpec`→`CharacterArchetypeSpec`, `enemy_roster`, `EnemyBrain`, `ALL_BRAIN_KEYS`, etc.) is a mechanical pass touching many refs and is INDEPENDENT of the movement-tuning/unification work — deferred as its own commit, not bundled. Jon also noted he's "not sure archetypes is a great design anyway" — so don't over-invest in the archetype concept; the rename is the cheap win, a deeper archetype rethink is separate.
- **2026-06-27 Stale `docs/planning/<oldfile>.md` breadcrumbs after the planning rewrite** — the `docs/planning/` tree was consolidated/renamed (see `docs/planning/MIGRATION.md`): old flat filenames (e.g. `fighter-capability-and-motor-unification.md`, `non-player-centric-actor-unification.md`, `restructuring-blueprint.md`) became `engine/unified-actors.md` / `engine/architecture.md` / etc. ~13 code-comment breadcrumbs in crates/ (and a few root TODO/*.md) still point at the OLD filenames. They're comments, not functional, so left for a mechanical sweep: `grep -rln 'docs/planning/[a-z-]*\.md' --include=*.rs crates/` then repoint to the consolidated doc (the MIGRATION table is the map). AGENTS.md is fine — it references the directory, which survived.
- **2026-06-27 Planning docs lag the body-vocab de-player-casing** — after the keystone moved the movement/economy vocabulary onto `crate::actor` (commits f3c8dff8 → 59653267), `docs/planning/engine/architecture.md`'s Bucket-2 component plan still names now-renamed/moved types: `PlayerWallet` (→`BodyWallet`), `PlayerShieldState`/`PlayerEnvironmentContact`/etc. (→`Body*`, all 18 clusters renamed), and `unified-actors.md`'s step-3 "(historical)" prose names dead symbols `ActorBody`/`PlayerClustersMut`/`integrate_grounded_body`/`integrate_aerial_body` (now `AncillaryMovementBundle` real components / `BodyClustersMut` / `ActorMut::integrate_body`). Also `ActorStatus.shield_raised` (retired, commit e0f65a78) may be referenced as a live bucket item. These are planning prose, not code, so left for a doc-refresh pass (deferred-intent additions in unified-actors.md ARE current). When refreshing: the Bucket-2 economy/movement slices are largely DONE; what remains is interaction-consumer + attack-state + safety/respawn. Don't trust a planning doc's component name without grepping the code first.
- **BIFURCATION: 2026-06-28 player vs actor MELEE attack pipeline (two state machines + two damage paths)** — the player and a brain-driven actor run PARALLEL melee pipelines, the keystone fork behind the recent "actor melee doesn't connect / no slash / not the same attack" bugs. Player: `PlayerAttackState` + `AttackSpec`, driven by `attack_advance_system` (`PrimaryPlayerOnly`, EXCLUDED from `update_ecs_actors` via `Without<PlayerEntity>`); damage = a per-frame Volume `HitEvent` (`HitSource::PlayerSlash`) each active frame, deduped via `hit_targets`. Actor: `ActorAttackState` (windup/active/cooldown + `pending_axis`, NO spec), driven by `update_ecs_actors`; damage = a persistent `Hitbox` ENTITY spawned at the windup→active edge, resolved by `apply_hitbox_damage`, deduped via `HitboxHits`. The `Hitbox`-entity path is the canonical one (bosses, actors, player AOE all use it). DONE so far (one-definition, not yet one-path): the SLASH visual now has ONE emitter `combat::attack::emit_melee_slash` that both call (commit after this entry). REMAINING merge (the real "ONE BODY ONE PATH"), per the explore map, in order: (1) player melee SPAWNS a `Hitbox` entity via `spawn_melee_hitbox` (Player faction) and DELETE the per-frame `HitEvent` loop — needs `apply_hitbox_damage`'s Player-faction branch to carry knockback (`knock_x`) + the damage multiplier (currently hardcodes `knock_x: 0.0`); keep pogo in the player path (player-only physics). (2) Merge `PlayerAttackState`+`ActorAttackState` into ONE body attack-state component (add `spec: Option<AttackSpec>` to the actor state; compute the actor's spec at `begin_melee_attack` instead of deferring geometry to the edge), and ONE driver that ticks it + spawns the hitbox + slash at the active edge for every body. (3) Fold the player into `update_ecs_actors` (drop `Without<PlayerEntity>`) OR have both call the one shared strike-spawn system; delete `attack_advance_system`'s bespoke damage/slash. RISK: (1) and (2) change the player's WORKING core melee feel (knockback/dedup/timing) and are BLIND (no GUI here) — drive with headless tests (damage connects, knockback preserved, one-hit-per-target, one slash) and have Jon verify feel; do NOT add any new player/actor-specific attack code in the meantime (route through the shared seam).
  - **UPDATE 2026-06-28 (mostly RESOLVED):** the slash visual, the swing MODEL, and the STATE COMPONENT are now unified. (a) `emit_melee_slash` is the one slash emitter; (b) the actor swing is resolved through the player's `attack_spec_from_view` + `AttackSpec` and stored on the shared state (commit `actor melee adopts the player's AttackSpec`); (c) step (2) is DONE — `PlayerAttackState`/`ActivePlayerAttack` AND the timer-based `ActorAttackState` are all DELETED, replaced by ONE `BodyMelee { swing: Option<MeleeSwing>, cooldown, ranged_cooldown, pending_axis }` carried by the player and every actor, built on the player's spec+elapsed `MeleeSwing` model (commits `merge actor melee state onto ONE BodyMelee` + `fold the player onto BodyMelee`). REMAINING (the last sliver): step (1)/(3) — the player still PRODUCES its melee damage as a per-frame Volume `HitEvent` from `advance_attack`/`attack_advance_system`, while actors produce it as a `Hitbox` ENTITY via `update_ecs_actors`+`apply_hitbox_damage`. Both feed the SAME resolver (`apply_feature_hit_events`), so this is two PRODUCERS of one event, not two resolvers. Collapsing it cleanly wants the player's per-swing string-key dedup (`MeleeSwing.hit_targets`, fed back by the universal resolver) reconciled with the hitbox's entity-key dedup (`HitboxHits`) — and the player's universal target coverage (breakables/orbs/bosses via `HitTarget::Volume`) preserved. Tracked; lower-risk to do once a body-melee DRIVER unifies the player+actor systems (currently still two systems gated by the not-yet-unified movement architecture).
  - **UPDATE 2026-06-28 (RESOLVED — producer collapsed):** the player melee now spawns a Player-faction `Hitbox` ENTITY through the SAME `combat::hitbox::spawn_melee_strike` every actor uses; the per-frame Volume `HitEvent` loop in `advance_attack` + the separate `start_attack` slash emit are DELETED. `spawn_melee_strike` derives BOTH the damage hitbox AND the slash from ONE gravity-resolved `world_box`, fixing the player's hitbox-vs-vfx divergence under C4 gravity (the player box now gates the screen-axis manifest box to upright, else the gravity-rotated spec box — the actor's rule). `Hitbox` gained `knock_x`; the Player FollowOwner branch emits the `PlayerSlash` Volume each active tick (deduped per-swing via `MeleeSwing.hit_targets`), World-anchored Player hitboxes (shockwave AOE) keep the once-only sentinel; `apply_hitbox_damage` resolves owner-pos via `CenteredAabb` OR `BodyKinematics` (player has no `CenteredAabb`). NOT a fork anymore — the ONLY remaining seam is the two DRIVER systems (`attack_advance_system` vs `update_ecs_actors`), which is the movement-driver question. Next: the unified action/ability timeline on this strike seam.
- **2026-06-30 Prop / arena animators skip `with_render_basis` → trimmed-sheet props misalign** — `spawn_room_prop` (`crates/ambition_render/src/rendering/world.rs:201`) and the cut-rope arena prop swap (`game/ambition_content/src/bosses/cut_rope/arena.rs:348`) build `CharacterAnimator::new(asset)` WITHOUT `.with_render_basis(basis_size, basis_anchor)`, unlike the actor/NPC paths (`rendering/actors/mod.rs:329/407`) and player (`scene_setup.rs:332`). For an alpha-trimmed sheet (now the default packing policy), a missing basis makes `current_render()` return `None`, so the renderer keeps the full-logical-frame size while the atlas index points at a SMALLER trimmed rect → the frame draws stretched/offset and wobbles per-frame (same failure class as the just-fixed `sprite_target` atlas-borrow). Latent because most props may be near-square / lightly trimmed so the drift is subtle, and some prop sheets opt out of trim. FIX when touched: compute `basis_size`/`basis_anchor` from the built sprite + feet anchor (mirror the actor path) and chain `.with_render_basis(...)`; or factor a shared `spawn_character_animator(asset, sprite, anchor)` helper so no spawn site can forget. Found while fixing the pirate/ninja-leader misalignment (commit bb98144a).
- **2026-07-01 [RESOLVED same day] Dedicated pogo button was dead (`gravity_symmetry` pogo failed under BOTH orientations)** — VERIFIED it was a real pre-existing regression, not a stale test (failed identically at the parent commit). Root cause: `action_set::resolve` emitted the Melee request only on `melee_pressed`, ignoring `pogo_pressed`, so a dedicated pogo press produced no melee message → `start_body_melee` never began the pogo swing. The melee-unification dropped the button trigger even though `control.rs` documents pogo as sandbox-hitbox-owned (button AND air-down). Fix: `resolve` emits the swing on `melee_pressed || pogo_pressed` (AirDown/`can_pogo` intent set downstream in `start_attack`; AI brains never set `pogo_pressed`); also folded `pogo_pressed` into `wants_any_action`. Pinned by a `resolve`-level unit test + the now-green `gravity_symmetry` pogo test.

## 2026-07-02 Slot-board steering (assign_slots → holding-ring) is a latent no-op
- **Where:** `features/ecs/actors/update.rs` — `tick_actor_brains` still calls `crate::combat::slots::assign_slots(...)` and `compute_holding_positions` exists, but the per-actor `slot_pos` they fed was already discarded (`let _ = slot_pos`) BEFORE the monolith split. Actor spacing is driven by the brain's `crowding` signal, not the slot board.
- **Smell:** the whole combat-slot *steering* path (board assignment + holding-ring fallback) produces a value nothing consumes. `compute_holding_positions` is now behind a `#[allow(dead_code)]` (test-covered only).
- **Why not fixed here:** ripping out slot-board steering is its own change (touches `CombatSlotsRes`, `assign_slots`, the slot tests) and is independent of the brain/movement/read-model split. Left as a focused follow-up.
- **Suggested fix / size:** M — decide whether the slot board earns its keep (arbitrating which enemy commits to an attack) vs. the crowding signal; if not, delete `assign_slots`/holding/`compute_holding_positions` + `CombatSlotsRes`.


## 2026-07-02 Music renderer audit — findings noted but NOT fixed
- **Where:** `tools/ambition_music_renderer` (a 4-agent code audit fixed ~35 findings across render/bundle/audit/CLI; these are the survivors).
- **`audit/reference_audio_audit.py` onset proxy is self-referential** — the onset threshold is the 85th percentile of the flux values themselves, so ~15% of frames always exceed it and `onset_proxy_per_second` is nearly content-independent. Fix wants a robust absolute threshold (median + k·MAD) and a finer hop than 0.5 s, or real peak-picking. Size S-M.
- **`audit/transition_audit.py` runtime-preview end-of-window step** — the `ambition_runtime` preview freezes the incoming gain and hard-cuts the outgoing at the context-window end, a discontinuity the real runtime (which keeps converging) does not have; the preview can show a click that is not real. Needs both exponentials continued through the post-window region. Size S.
- **CLAP is discoverable but not hostable** — `plugins list_clap` finds plugins and the validator warns, but no backend can run them; either add a CLAP host adapter or stop discovering them. Size M.
- **`backends/sfizz_backend.py` VST3-hosted sfizz path shares little with the CLI path** — parameter setting/probing logic partially duplicates `pedalboard_backend`; low value until the sfizz-VST3 path is actually exercised.
- **`agent/` studio scripts predate the audit fixes** — `agent/song_studio/studio.py` etc. still hand-roll analysis the packaged audits now expose (e.g. `audit mix_balance` CLI); worth folding the studio loop onto the packaged surfaces next time a song-studio pass happens.

## 2026-07-02 `cargo test -p ambition_content` silently SKIPS the portal tests
- **Where:** `ambition_content/Cargo.toml` has `default = []`; the `portal` module (and its 36 tests, incl. the transit conservation audits) compiles only under `--features portal`. The app crate enables the feature, so `-p ambition_app` integration tests cover portals, but the content crate's own suite skips them with no signal (53 pass, portal ones simply absent).
- **Why it bit:** the carried-momentum drift regression (F13, fixed 81fd5016) shipped while `cargo test -p ambition_content` looked green — the failing coverage lived in the feature-gated module + an app test nobody re-ran. "N passed" reads as "covered" when a whole module was compiled out.
- **Suggested fix / size:** S — either make the content crate's dev-profile tests always enable `portal` (`[dev-dependencies]` self-reference with features, or default the feature on since gameplay_core pins it anyway per the Cargo comment "the sandbox lib was never built fully bare"), or add a CI/test-runner note that content must be tested with `--features portal`.

## 2026-07-02 Music audit gaps exposed by ear-testing gradient_ascent
- **Where:** `tools/ambition_music_renderer` audit suite vs Jon's timestamped listen notes.
- **Gap 1 - effects-induced pitch instability is invisible to every audit.** A pedalboard chorus at depth 0.15 / 0.8 Hz produced an audible "bend-down" percept on long sustained lead notes (heard at 0:06); nothing analyzes the RENDERED audio for pitch stability. A cheap detector: per-stem YIN/pYIN pitch track over sustained notes, flag cyclic deviation > ~25 cents. Same detector would catch authored bends that start detuned (the 2:45 issue - a note authored to START -150 cents flat).
- **Gap 2 - foreground-melody collision salience.** The dissonance audit DID flag the 1:17 clash (bar 40: whistle answer vs hook hang) but ranked it among pad-voicing add9s; nothing distinguishes "two simultaneous foreground melodies overlapping in the same register" (perceptually severe) from "chord-tone seconds inside a pad voicing" (fine). A lead-vs-lead overlap check - motif-kind layers sounding simultaneously within an octave - would have surfaced it as its own category.
- **Why not fixed now:** mid-composition session; both are S-M sized audit modules. The mix_balance/dissonance surfaces to extend are in `ambition_music_renderer/audit/`.
- 2026-07-02 (between_objectives): audit/lead_collision derived a note's bar via int(start_beat // beats_per_bar) on float-accumulated beats; a bar-boundary note arriving as 223.99999999999997 was floored into the PREVIOUS bar and flagged as a false exposed tension against that bar's chord. Fixed with a +1e-6 nudge before flooring (all three floor sites). Underlying smell: build_score events carry nominal_bar/nominal_beat that audits could use instead of re-deriving from floats.
- 2026-07-02 (pitch_stability tool limitation): `audit/pitch_stability` runs a monophonic pitch tracker over the rendered lead STEM, so it merges reverb-connected legato phrases into one "note" (observed note_seconds of 8.8s where the longest authored note is 2.9s) and reads the melody's own motion + sample vibrato as pitch "wobble". A verified-accepted cue (broken_transmitter, Sonatina violin lead) scores 23 wobbles/6 onsets up to 210 cents; between_objectives' flute scores 38/10 up to 400 cents at a fast melodic peak — same order of magnitude, i.e. normal for an expressive sustained SFZ lead, not a tuning defect. Real fix: segment the analysis on the MIDI note boundaries that `build_score` already emits (`_ambition_note_events` carry nominal_bar/nominal_beat/nominal_duration_beats) instead of re-detecting onsets from a reverberant stem. Until then, treat wobble_count as a guitar-scoop detector (sustained SINGLE notes bending), not a verdict on melodic leads.

## 2026-07-03 (fable-review T1 fallout) — entity-sprite generator emits dead `FeatureVisualKind::Npc/Boss` labels
- **Where:** `tools/ambition_sprite2d_renderer/.../targets/props/entities.py` generates `crates/ambition_actors/assets/sprites/**/entity_manifest.yaml` with `category: FeatureVisualKind::Npc` / `::Boss`. After AD1-T1 (8cef2245) those variants no longer exist — the enum is `{ Actor, Hazard, Breakable, Chest, Pickup, Switch }`.
- **Why not fixed here:** the YAML is GENERATED (never hand-edit generated output), and the `category:` label is tooling metadata that no Rust code parses, so it doesn't break the build — but it names a dead variant. Fix belongs in the generator, not the output.
- **Suggested fix / size:** XS — update `entities.py` to emit `FeatureVisualKind::Actor` for the actor rows (or drop the `FeatureVisualKind::`-prefixed label entirely, since nothing consumes it), then regen. Related: [fable-review E35].

## 2026-07-03 — `unified_body_movement` chase test fails (pre-existing; smells of query-order non-determinism)
- **Where:** `game/ambition_app/tests/unified_body_movement.rs::home_body_and_actor_body_move_through_the_same_integration_phase` (rl_sim). A `cellular_automaton_fighter` enemy spawned 160px RIGHT of the player should chase LEFT >5px over 40 fixed-60hz steps.
- **Symptom:** it doesn't. Built+ran at the session-start commit 7c0872a7 (isolated worktree) → enemy moved x 1110→1134.8 (+25px, WRONG direction); at a later HEAD → 1110→1109.4 (−0.6px, right direction but far too small). A ~25px swing between two builds of a DETERMINISTIC fixed-60hz sim is the tell: an order-sensitive query somewhere in the chase pipeline (brain → ActorControl → `integrate_sim_bodies`) is iterating in `Entity`/HashMap order, not a stable id. [[feedback_query_order_determinism]]
- **Why not fixed here:** pre-existing (fails at session start too — not a regression from the read-model/taxonomy work), and it's a focused gameplay/determinism debug, not a stale-test fix. Possibly tied to the PAUSED PCA encounter (`cellular_automaton_fighter` is the Perfect Cellular Automaton archetype).
- **Suggested fix / size:** M — trace the chase pipeline for an unsorted query/iteration; sort by a stable feature id. Confirm determinism by running the test across two clean builds and asserting identical enemy_x.

## 2026-07-03 — `control_frame_modes_from_settings` is an unwired settings→control-frame consumer
- **Where:** `crates/ambition_actors/src/items/pickup/mod.rs:660` — `pub(crate) fn control_frame_modes_from_settings(settings) -> ae::ControlFrameModes` reads `UserSettings.gameplay.control_frame_modes()`, but has ZERO callers (a `never used` warning).
- **Why it matters:** it's the read point for the user's control-frame preference (gravity-relative vs screen-relative joystick mapping) — an OPEN design (frame-of-reference.md). Either a dropped consumer (the setting isn't applied — a feature gap) or a pre-wiring point awaiting the reference-frame decision. Not just dead code to delete.
- **Why not fixed here:** wiring it needs the open reference-frame design call; deleting it would remove the wiring point. Left in place, flagged. [[project_reference_frames]]
- **Suggested fix / size:** S once the reference-frame design lands — wire it into the input→control-frame bridge; until then, keep it (or `#[allow(dead_code)]` with a pointer to frame-of-reference.md).

## 2026-07-05 (G4) — fused `gnu_ton` boss profile + split-overlay render — ✅ RESOLVED 2026-07-10
Torn down in `refactor-chain.md` R2 (the E6 deferred teardown). The fused profile,
its encounter spec, its sheet, and the whole split-layer render are deleted; the
referencing tests were retargeted onto the ADR-0020 linked pair FIRST, which is
how the retarget caught a real fact: the rider authors a G5 `possessed_verbs` map
the fused profile never had, so a possessed boss's Attack now commands
`hand_sweep`, not capability slot 0. Detail + the honest LOC accounting (it did
NOT shrink `boss_encounter/`) in `docs/planning/engine/refactor-chain.md` §R2.

## 2026-07-06 (E5 step 5) — `apply_room_replay_request_system` hard-codes cut-rope content in the APP
- **Where:** `game/ambition_app/src/app/sim_systems.rs::apply_room_replay_request_system` — calls `ambition_content::bosses::{is_cut_rope_boss, reset_cut_rope_boss_attempt}` inline before the generic room replay.
- **Context:** the E5-finish step-4 de-weave made the MESSAGE generic (`session::reset::RoomReplayRequested`, content emits from `ContentDialogueFollowupSet`), but the CONSUMER still hard-names cut-rope resets. That's why this system had to stay app-side in the E5 step-5 carve (the app may name content; the engine/host may not) — the readiness brief's "generic, de-woven" claim was only half true.
- **Suggested fix / size:** S — mint a `ContentRoomReplayResetSet` slot (same pattern as `ContentRoomResetSet`): content registers a `reset_cut_rope_attempt_on_replay` system reading the same `RoomReplayRequested` message; the app consumer drops the `ambition_content` import. Then the replay consumer is host-generic and can move wherever the reset/world-flow concern lands.

## 2026-07-09 — sprite_sheet lib-test never compiled (carve dropped test placeholders)

`ambition_sprite_sheet` shipped two dangling `#[cfg(test)] mod tests;`
declarations (`boss.rs`, `game_assets/mod.rs`) added by the F1.5 carve
(cdf21e0b) that pointed at test files which were NEVER created — so the crate's
lib-test target failed to compile from that commit onward (only surfaced under
`cargo test --all-targets`, which the default gate doesn't run). No coverage was
lost: the boss-sprite tests still live at
`ambition_actors::boss_encounter::sprites::tests` and the game-assets tests at
`ambition_actors::assets::game_assets::tests` (the config/profile/sandbox half
went to `ambition_asset_manager`). Removed the orphan declarations. OPPORTUNITY:
sprite_sheet owns real logic (boss sprite-metric derivation, entity-sprite
resolvers) with no crate-local unit tests — worth adding sprite_sheet-side
coverage rather than only testing through the actors adapter. Lesson reinforces
F7: a carve that adds `mod tests;` must MOVE the fixture in the same commit.

## 2026-07-09 — `ambition_actors::features::conversion_tests` is misnamed

`crates/ambition_actors/src/features/conversion_tests.rs` (+ its inner
`mod conversion_tests`) contains "headless movement + collision tests for the
actor simulation" (NPC patrol/gravity/possession, enemy AI, archetype tuning) —
NOT LDtk conversion tests. The name misled the fable audit F5.4 into listing it
as a test-travel candidate for `ambition_ldtk_map`; on inspection it is
correctly actor-side and stays. RENAME opportunity: `actor_movement_tests.rs`
(and drop the redundant inner `mod conversion_tests` wrapper). Low-risk; deferred
to avoid churn during the F9.2 arc.

## 2026-07-10 (R1) — the `ambition` umbrella re-exports `bevy`, but not its DERIVES

A downstream game crate whose manifest names ONLY `ambition` can use bevy
*types* through `ambition::bevy::…`, but it cannot `#[derive(Component)]` /
`#[derive(Resource)]`. Bevy's derive macros resolve the `bevy_ecs` path through
the CONSUMER's `Cargo.toml` (`BevyManifest`), and a re-export does not satisfy
that lookup — the expansion emits a bare `::bevy_ecs::…` and fails with
"unresolved module or unlinked crate `bevy_ecs`". Hit while writing the D-C
hosting oracle in `game/ambition_demo_sanic` (which keeps a one-dependency
manifest on purpose); the test now evaluates the run condition with
`RunSystemOnce` instead of gating a bespoke marker resource.

**So the E9 "author a game through the umbrella alone" claim has an asterisk:**
any content crate that defines its own components/resources must ALSO list
`bevy` in its manifest. That is probably fine (it is one line, and the version
is pinned by the workspace), but it is not what the umbrella's doc comment
implies. Options if we want the claim literal: re-export the derives from
`ambition` under different names (ugly), or document the `bevy` line as expected
in the umbrella's docs + `docs/planning/demos/README.md` (cheap, honest).
Size: S. Not blocking — `game/ambition_demo_sanic` authors rooms, not components.

## 2026-07-10 (R3) — feature-gated test targets rot: `portal_render` had not compiled in weeks

`game/ambition_content/src/portal/tests.rs`'s `partial_render_keeps_the_sprite_and_adds_the_exit_copy`
(gated `#[cfg(feature = "portal_render")]`) imported `ambition_portal::{sync_portal_world_frame,
tag_portal_scene_bodies, PortalWorldFrame}`. Those symbols live in
`ambition_host::portal` and `ambition_portal_presentation` — a crate ABOVE
content and a crate content only gets under `portal_render`. An E-track carve
moved them and never updated this test, so `cargo check -p ambition_content
--features portal_render` had failed since. Nothing in the standing gate builds
that feature, so nobody saw it. Fixed while landing R3: the two five-line host
bridges are restated as local test fixtures (the host owns testing them; what
this test exercises is presentation's `sync_portal_body_pieces`).

**The class, not the instance.** CC6 already taught this once — the content
suite silently skipped its portal tests until `--features portal` joined the
gate. `portal_render` is the same hole one feature over. Candidates the standing
gate still never builds: `portal_render`, `bevy_ui_menu`, `kaleidoscope_menu`,
`mobile_touch`, `physics_debris`, `static_map`. A periodic
`cargo check --workspace --all-targets --all-features` (or a per-feature matrix
in CI) would catch the next one. Size: S for the check, unknown for what it finds.

## 2026-07-10 (R6b) — a gate script that greps only for success is silent on failure

My R6b gate ran `cargo test -p ambition_actors --lib 2>&1 | grep -E "^test result" | tail -1`
and printed NOTHING. Nothing is what a clean pass looks like to a careless reader,
so a `grep`-for-success gate reports "green" when the crate's lib-TEST target does
not compile. It had not: the slot-0 filter annotations deleted `use
crate::actor::{PlayerEntity, PrimaryPlayer};` from six modules, and their inline
`#[cfg(test)]` blocks reached those names through `use super::*`. The lib compiled;
`--lib` tests did not.

Caught only because the missing line looked odd next to its siblings. **Any gate
filter must match the FAILURE signatures too**, not just the success marker:
`grep -E "^test result|^error|FAILED|panicked"`. The other gate legs in this chain
happened to be safe (`grep -E "FAILED|^error"`), which is the same lesson from the
other side. Same class as the Monitor doctrine: *silence is not success.*

## 2026-07-15 — Mary-O follow-up: outstanding items Jon flagged (multi-game host UX)

Batch logged from a Mary-O demo session (sprite/sfx/kit/tiles/cycle landed
separately). Jon's steer: *"start logging these to smells instead of taking care
of them directly … take care of what we can and log anything we don't fix."* The
shell-UX cluster (menus, exit-to-title, vanity/title timing, control hints) is
going to Fable to plan; the two bugs below are the ones worth fixing directly.

**1. The in-game Ambition inventory menu opens on the TITLE SCREEN.** Big smell,
two ways: (a) it is *default* behavior nothing opted into, and (b) it should not
be *possible* at all outside a live gameplay session. The inventory-open input /
system is almost certainly registered globally rather than gated on an
`ActiveSessionScope` / gameplay-mode `run_if`. Same family as the session-scope
work: menu chrome that belongs to a session is leaking into the host shell. Fix
direction: gate the inventory toggle (and any gameplay-only menu) on "a live
session exists AND it is Ambition's mode", not on the process being up. Size: S–M.

**2. Attack fires the wrong direction after turning.** Move LEFT, press attack →
the attack comes out to the RIGHT (or reads as a back-attack). Smells like the
attack samples a facing/aim latched when movement *started* (or the pre-turn
facing) instead of the body's current facing at the swing. Needs diagnosis —
likely in the melee/aim resolution reading a stale `facing`/`ResolvedMotionFrame`
rather than the live one. Look for where the swing's direction is chosen vs. where
facing is updated in the same tick (ordering). Find the elegant fix (single source
of truth for "which way am I facing NOW"). Size: S once located.

**3. Vanity / "powered by Ambition" card + title presentation timing.** The
vanity card is too short and has no fade in / fade out — it needs both, and a
fade-*out* specifically. The title screen has no opening animation: the menu
snaps in after a dramatic pause instead of fading in. And the soundtrack is tied
to the BOOT sequence when it should be tied to the TITLE SCREEN. Presentation/feel
cluster; part of the Fable shell-UX plan. Size: M.

**4. "Exit to title screen" is missing from the Ambition pause menu.** Today you
cannot leave a live Ambition session back to the multi-game title without quitting
the process. The session-scope teardown already exists (launch/quit/relaunch is
leak-free); this is a menu entry that fires the existing "retire session → return
to launcher" path. Part of the Fable shell-UX plan. Size: S–M.

**5. Sanic and Mary-O have no pause menus.** Each hosted experience needs a small,
minimal menu: Pause, Quit to Desktop, Quit to Title. Jon: *"this should be elegant
to do"* — the seam should be one per-experience shell-menu primitive the host
offers, not a bespoke menu per demo. This is the core of the Fable plan (a
per-experience shell-chrome seam), and it composes with #3/#4. Size: M.

**6. Rename `smb1` → `mary_o` across the code.** The mode string is already
`"mary_o"`, but the crate (`ambition_demo_smb1`, `ambition_demo_smb1_app`), the
`Smb1*` types, `SMB1_MODE`/`SMB1_CATALOG_RON` consts, and `smb1_*` fns still carry
the working title. Mechanical but wide (crate rename = build churn); do it as its
own commit. See [[feedback_entity_id_matches_label]]. Size: M (churn).

**7. ✅ RESOLVED 2026-07-15 (commit 6875aeaea) — ability sets compose onto
characters.** The preset picker (`AbilityKitSpec`) is gone. `AbilitySet` gained
the algebra (`NONE`/`union`/`intersect`); `AbilityGrant` is the composable
vocabulary; a catalog row lists grants (`abilities: Option<Vec<AbilityGrant>>`)
and `AbilitySet::compose` unions them into the body's `AbilityBase`. New verbs
land as new grants a character appends, never as a preset the roster forks — the
additive model Jon asked for, and the ability slice of the 2026-06-26
closed-archetype smell (line ~62). The same landing fixed a still-live bug: the
F3 dev-sync WHOLESALE-REPLACED the primary player's abilities with the global
`EditableAbilitySet` (default `sandbox_all`), so Mary-O's authored run+jump was
clobbered back to the full kit every frame. The editable is now a session MASK
(`effective = base ∩ editable`) — a mask only removes, never adds, so the
restricted base survives. Gear/upgrade UNION reuses the same `union` seam when a
powerup that grants a movement verb lands (the milk/mushroom is the first
candidate). ORIGINAL: this session added `AbilityKitSpec` (a preset picker) as a
floor; Jon: *"ability sets should probably compose onto characters"*.

**8. Design Q for Fable — on-screen control hints are hardcoded to the Ambition
player.** The context-sensitive button icons/text appear pinned to the single
Ambition protagonist's verbs. There needs to be a hook so whatever is currently
CONTROLLED (worn character) or whatever UI is active drives the control-hint
labels/icons. Same relativity principle as the rest of the engine: the prompt row
is a function of the active control context, not a global constant.

**9. `WorldItem` visual is a tinted quad, not the real sprite (2026-07-15).**
`sync_world_item_visuals` (ambition_render) draws a colored `Sprite::from_color`
per world item, tinted by row id. A real `super_mary_o_milk_carton` sheet is
generated in the renderer submodule; wiring it needs the PROP-SHEET render path
(the item visual draws one image, not a manifest frame rect). Follow-up: give
`WorldItemFact` an optional sprite/anchor the render can bind, reusing the
prop-sprite pipeline. Until then the quad is the shipped (draw-blind) visual.

**10. `WorldItem` has no locomotion (2026-07-15).** A resting collectible only —
a classic sliding mushroom wants a free-body integrator like `ground_item_physics`.
Deliberately NOT abstracted at N=2 (design-balance): unify a `settle_free_body`
helper across `GroundItem`/`WorldItem` when a THIRD moving pickup lands.

**11. Goomba squash bypasses the engine death path (2026-07-15).** `bounce_squash_goombas`
zeroes health + despawns directly, skipping the shared death/score/drop pipeline.
Fine for a 1-HP walker with no drops; revisit if goombas ever drop or score.

**12. Reactive-block reuse unproven — brick-break is the missing second consumer
(2026-07-15).** `ContactSource::Block` now carries `GeoId` and the ?-block powerup
is its first consumer. A brick-break (Head/Support vs a breakable `GeoId` → remove
the block) would prove the primitive generalizes, but removing a block mid-run is a
`World`-mutation slice, deferred. The engine-for-other-games oracle wants a second
consumer eventually.
