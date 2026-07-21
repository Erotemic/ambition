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

## 2026-07-20 bfs-side sand plumbing is vestigial after the FS2/FS3 sand slice
- **Where:** `game/ambition_content/src/falling_sand.rs` — the `MaterialKind::Sand` arms, `project_sand`, `dense_sand`, `SAND_THRESHOLD`, and the `TallyLedger.sand` column.
- **Smell:** sand no longer enters `bevy_falling_sand` (it runs on `falling_sand_sim`'s deterministic grid, `574550a6d`), so this code sees zero sand particles forever. It stays only because deleting it churns the water/oil path the sand directive explicitly fenced off.
- **Noticed while:** landing the FS2/FS3 sand slice.
- **Suggested fix / size:** M — delete WITH the water/oil correctness slice (which replaces the whole bfs bridge); doing it standalone would rewrite `tally_particles`/`project_liquid` twice.

## 2026-07-20 Falling-sand step scans the full grid width per occupied row
- **Where:** `falling_sand_sim/sand_grid.rs` — `step`/`settle_into` iterate every x in rows whose `row_loose > 0`.
- **Smell:** fine at current scale (rows skip in O(1) when empty; 1024-wide room), but a fully flooded room walks ~650k cells/tick. Per-row min/max x spans would bound it.
- **Noticed while:** landing the slice; deliberately NOT optimized (no evidence it matters yet).
- **Suggested fix / size:** S if profiling ever names it.

## 2026-07-19 Simulation-originated presentation effects have no seam and no confirmed-frame release
- **Where:** 27 direct `MessageWriter<VfxMessage>` sites, plus FOUR more request families with their own direct writers — `EffectRequest` (6), `DebrisBurstMessage` (6), `ExplosionRequest` (3), `FireworksRequest` (1). Audio has exactly one producer (`ambition_sfx::SfxWriter`), which is why `010c84369` could add its guard in one place.
- **Smell:** every one of these re-fires when a rollback re-simulates a frame, and there is no single place to intervene. Counting only `VfxMessage` UNDER-scopes the problem by four families.
- **Do NOT** "add a VfxWriter then copy the audio one-liner." That gate answers *this frame ran before*, which is not *this frame is confirmed* — under predicted remote input it keeps the phantom effect and suppresses the corrected one (see `SfxEmissionGate`'s doc comment). Copying it would spread a known-wrong policy to five more families.
- **Suggested fix / size:** L, staged. (1) Centralize the dominant `VfxMessage` production seam. (2) Inventory which of the five families are simulation-originated vs presentation-originated — UI/menu effects stay outside entirely. (3) Frame-stamp the simulation-originated intents. (4) Release accepted intents at the host's confirmed boundary, discarding abandoned predictions. Do one confirmed-frame vertical slice end-to-end before generalizing, and never gate 27+ call sites individually.

## 2026-07-19 BIFURCATION: projectile hit-detection runs TWO victim loops — ✅ RESOLVED 2026-07-19
- **RESOLUTION (same day, tracks #8):** collapsed to ONE victim loop over every body, mirroring `hitbox/mod.rs:203`. `Has<PlayerEntity>` now picks only payload policy (routing stamp + the player's parry heal). Three real drifts died with the fork: the actor side got knockback (it passed `None`, so an actor hit by the very bolt that launched the player just absorbed it), the player side got the grudge term (`damage_lands` instead of `can_damage`), and vulnerability became FEEDBACK-only for both (§A2: the event always flows, i-frames resolve at consume time). The vulnerability cluster is `Option` in the unified query on purpose — requiring it would silently drop simple feature bodies from the query (the required-components-skip trap, which is exactly how the 4 test failures during this change presented). `can_damage` is now unused by projectiles.
- **(historical)**
- **Where:** `crates/ambition_actors/src/projectile/systems.rs:577-665` (player loop) and `:679-729` (actor loop); the unified pattern to copy is `crates/ambition_combat/src/hitbox/mod.rs:203-322`.
- **Smell:** the melee hitbox path collapsed to ONE victim loop over every body with `Has<PlayerEntity>` picking only payload policy; projectiles still run the pre-unification shape — separate `strict_intersects`, separate vulnerability checks, `HitTarget::Player` vs `HitTarget::Actor`, and a knockback asymmetry (player gets `FeelScale(0.85)`, actor gets `None`). The code self-documents the drift (`:617-619` "This site had drifted (it dropped the parry term)"). This is the single clearest unmerged combat fork left.
- **Noticed while:** 2026-07-19 deep review (bifurcation sweep).
- **Suggested fix / size:** M — one victims query with `Has<PlayerEntity>` selecting payload policy, exactly like `hitbox/mod.rs:203`; delete both loops.

## 2026-07-19 BIFURCATION: "body was struck" feedback keyed on `is_player` at TWO attacker-side emit sites
- **Where:** `crates/ambition_combat/src/hitbox/mod.rs:241-268` and `crates/ambition_actors/src/features/ecs/actors/update.rs:1113-1142` (byte-identical payload: `PLAYER_DAMAGE` sfx + `Burst{14,300,[1.0,0.34,0.28,0.88],Shard}` + `DebrisBurst{Impact}`); non-player victims get their richer feedback on the CONSUMER side instead (`damage/actor_hit.rs:271`, `:207-213`). Death feedback is likewise per-victim-kind at three sites (`actor_hit.rs:377-388`, `boss_hit.rs:205-215`, `damage_apply.rs` `death_respawn_player`).
- **Smell:** hit RESOLUTION is unified (`resolve_body_hit`), but hit FEEDBACK forks by `is_player` and by layer. Duplicated payload constants at two emit sites will drift.
- **Noticed while:** 2026-07-19 deep review. **Jon's fix note asks for per-attack VFX/SFX binding ("the same one should not generically be used for all attacks") — the elegant resolution is ONE victim-side feedback seam keyed on the attack/volume spec + the victim's feel profile, which retires both `is_player` branches AND gives moves authored feedback in the same stroke.**
- **Suggested fix / size:** M — single victim-side reaction system consuming the resolved hit event; attack spec contributes the effect identity; delete both attacker-side emit blocks.

## 2026-07-19 Portal gun visuals read sim `BodyKinematics` directly (read-model leak) — ✅ RESOLVED 2026-07-19
- **Where:** `crates/ambition_portal_presentation/src/gun_visuals.rs:62-63` (`Query<(&BodyKinematics, &PortalGun, Option<&PortalTransit>)>`); `lib.rs:128` documents attaching runtime `BodyKinematics` to the presentation entity.
- **Smell:** presentation crate queries the sim-side kinematics component instead of a pose view (`BodyPoseView` exists and the rest of render uses it). Portal-domain component reads are fine; the body-kinematics read is the leak.
- **Resolution:** the crate now reads a host-published `PortalBodyView` (pos/size/facing) on two seams — `PortalSceneBody` and the new `PortalAffordanceBody` — and names `BodyKinematics`/`PlayerEntity`/`PrimaryPlayer` nowhere. **The suggested fix ("read the pose from `ambition_sim_view`") was NOT taken and should not be:** `ambition_sim_view` depends on `ambition_actors`, so consuming `BodyPoseView` would add exactly the host-crate edge this crate's manifest forbids ("never a host crate"). Used the crate's own host-seam idiom instead (same shape as `PortalCameraContinuityHostView`). **The leak was hiding a bug:** the affordance body is tagged from `ControlledSubject`, so while possessing, the held gun and disorientation indicator drew on the HOME AVATAR while the fire adapter already resolved the shot from the controlled body holding the gun (`portal_fire_origin_comes_from_the_holding_controlled_body`, "no fallback" to primary). Pinned by two `ambition_host` tests; the untag half poison-verified.

## 2026-07-19 `ambition_menu`/`ambition_settings_menu` reimplement navigation beside `ambition_ui_nav`
- **Where:** `crates/ambition_menu/src/render/bevy_ui/mod.rs:165` (`focus_key_for`/`MenuFocusKey` own focus model; no `ui_nav` dep), `crates/ambition_settings_menu` (same), plus the shell pause menu, `ambition_app/src/menu/grid_backend.rs`, and `kaleidoscope_app.rs` stacks.
- **Smell:** `ambition_ui_nav` is the declared shared seam (windowed list math, scroll-to-row, pointer activation, focus state; consumed by inventory_ui/dialog/render/touch_input) but the two menu crates and three app-side stacks hand-roll focus/list logic.
- **Suggested fix / size:** M — adopt `ui_nav::{list, pointer}` in `ambition_menu` first (it feeds launcher/pause/kaleidoscope), then settings_menu. Blocked on nothing; pure adoption.

## 2026-07-19 Cutscene and encounter each hand-roll an input-freeze beside `GameMode`
- **Where:** `crates/ambition_cutscene/src/lib.rs:211` (`freezes_player_input`; `touch_input/src/menu_bridge.rs:64` confirms "Cutscenes don't change GameMode") and `crates/ambition_encounter/src/lifecycle.rs:42` (own lock).
- **Smell:** dialogue and pause already suspend input through the shared `GameMode`; cutscene and encounter carry private mechanisms for the same effect. M22 (no sequence DSL) is not implicated — this is one duplicated *mechanism*, not a domain merge.
- **Suggested fix / size:** S/M — a shared "gameplay input suspended" gate derived from `GameMode` + domain locks, consumed by the one input-folding seam.

## 2026-07-19 `spawn_actors.rs` is a second spawn-side monolith
- **Where:** `crates/ambition_actors/src/features/ecs/spawn_actors.rs:120` (`apply_spawn_actor_requests`, ~513 lines, 23 branch constructs) and `:633` (`boss_actor_cluster`, ~355 lines); 7 commits since 2026-07-16.
- **Smell:** the ledger's monolith attention is all on `update_ecs_actors`; the spawn dispatcher grew to the same shape unnoticed.
- **Suggested fix / size:** M — extract per-`SpawnActorKind` spawn helpers so the dispatcher is a thin match.

## 2026-07-19 `ambition_actors` compat-facade debris (73 `pub use` lines, two dead modules)
- **Where:** `crates/ambition_actors/src/effects/mod.rs` (10-line `pub use ambition_vfx::*`, ZERO consumers), `src/debug_label.rs` (6-line re-export, ZERO consumers), `src/host/` (46-line re-export of `ambition_persistence::host::windowing`), plus ~73 `pub use ambition_*` compat lines total; doubled `#[cfg(test)]` attribute at `lib.rs:51-52`.
- **Smell:** migration residue that makes actors' coupling look wider than it is (decomposition.md:93-96 already notes this distorts the touch-input question). AGENTS.md: delete compat shims on sight.
- **Suggested fix / size:** S per facade — repoint consumers at canonical homes (`ambition_platformer_primitives` for `PrimaryPlayer`/`GravityField` etc.), delete the re-exports. Mechanical, sonnet-gradable.

## 2026-07-19 `enforce_session_contract` re-fingerprints the whole rollback registry every frame
- **Where:** `crates/ambition_runtime/src/rollback/session.rs:282-300`.
- **Smell:** clones the entire `RollbackRegistry` (~165 String-heavy descriptors) and recomputes a blake3 schema fingerprint in `PreUpdate` while any session is active — every frame of every dev build. Correct but wasteful.
- **Suggested fix / size:** S — compute the fingerprint once at session start (registry is immutable while a session is active) and compare the cached value.

## 2026-07-19 14 duplicate `fn test_app()` fixtures; `combat/util.rs` grab-bag
- **Where:** 14 separate `fn test_app()` fixtures in 14 files (11 in `ambition_actors` — `abilities/ranged/*`, `boss_encounter/`, …); `crates/ambition_combat/src/util.rs` (256 lines) self-describes as "grab-bag … no shared theme" with ad-hoc collision predicates (`player_is_standing_on`, ±2.0/8.0 tolerances).
- **Suggested fix / size:** S/M — one shared minimal-app test fixture per crate; classify util.rs items to their owning modules.

## 2026-07-19 No `[workspace.dependencies]`; real version drift
- **Where:** root `Cargo.toml` (no workspace-deps table); bevy pinned 46×; `ambition_items` ron 0.12 vs 0.11 elsewhere; thiserror 1 in 3 crates vs 2 elsewhere.
- **Suggested fix / size:** S/M mechanical — adopt `[workspace.dependencies]` for bevy/serde/ron/thiserror at least; aligns versions and makes the next bevy bump one-line.

## 2026-07-19 Python tooling: repo-root resolution duplicated ~14×; artifact scripts missing the `file://` link convention; LDtk CLI advertises dead subcommands
- **Where:** `scripts/*.py` each define `ROOT`/`REPO_ROOT`/`REPO`/`repo_root()` independently (`generate_agent_index.py:15`, `agent_query.py:30`, `run_tests.py:41`, ~11 more); `scripts/{generate_background_assets,ecs_inventory,modules_md,sweep_runtime_diagnostics,render_line_profiles,non_ecs_inventory,generate_visual_quality_variants}.py` write artifacts without the AGENTS.md-required rich `file://` output link (only `module_graph.py` complies); `tools/ambition_ldtk_tools/.../cli.py:53-55,687` registers `link add/remove/check` subcommands that are empty `[TODO]` stubs.
- **Suggested fix / size:** S each — one `scripts/_paths.py` helper; add the link prints; wire or drop the `link` subcommands.

## 2026-07-16 `ProjectileView.visual_id` allocates a `String` per projectile per sim tick
- **Where:** `crates/ambition_sim_view/src/facts.rs:341` (`rebuild_projectile_views`)
- **Smell:** the projectile-visual eviction (224dba72f) turned `ProjectileView.kind`
  (`Copy` enum) into `visual_id: String`; the read-model is rebuilt from scratch every
  tick by construction, so every live projectile now clones an id `String` per tick
  even though the id is immutable after spawn. Pure alloc churn — worst under
  bullet-hell volleys.
- **Noticed while:** fable review of the opus structural-eviction commits.
- **Suggested fix / size:** S–M. Either intern the id (`Arc<str>` in both
  `ProjectileVisualId` and the view — clone becomes a refcount bump) or restructure
  the rebuild to reuse rows keyed by entity. The `Arc<str>` route is smaller and keeps
  the "rebuilt every tick" `declare_derived` claim intact.

## 2026-07-01 Portal TRANSIT-feel adapters still key on `PrimaryPlayer` — ✅ RESOLVED 2026-07-15
- **RESOLUTION (commit 00d249292):** `warp_portal_input` + `portal_player_input_adapter` resolve `ControlledSubject` (fallback primary) exactly like the use-path adapters, so a possessed body's emergence gets `PortalEmission`/`PortalInputWarp`/trace seams. Wall-ability suppression went further: BODY-GENERIC over `With<PortalTransit>` (the aperture-edge hazard is a property of transiting), with a paired `restore_wall_abilities_after_transit` that restores the four verbs from the body's own `AbilityBase` on latch removal — needed because the F3 per-frame re-sync only covers the primary player. Poison-tested both ways.

## 2026-07-01 Docs reference a removed `CharacterArchetype` (né `EnemyArchetype`) ENUM — ✅ RESOLVED 2026-07-11
- **RESOLUTION:** the actual live stale refs (7, most of the original 8 had already been fixed by later refactors) reworded to the real path (`spec_for_brain`: brain key → `CharacterArchetypeSpec`): `spawn_actors.rs:75,882` + `entity_catalog/placements.rs:71` (Rust doc), and 4 RON comments (`character_catalog.ron` ×3, `character_archetypes.ron` ×1). `enemies/mod.rs:553` is CORRECT as-is (it names the *deleted* `CharacterArchetype::X.spec()` to explain what its test fixture replaced). Pure docs; no code change.
- **Where:** 8 sites — `features/ecs/spawn_actors.rs:75,641`, `features/enemies/mod.rs:527`, `ambition_characters/src/actor/mod.rs:186`, and comments in `character_catalog.ron` / `character_archetypes.ron`.
- **Smell:** doc comments call methods on a `CharacterArchetype` enum (`::from_brain`, `::attacks_player`, `::RangedSkirmisher`, `::X.spec()`) that no longer exists — the named-archetype enum was removed in an earlier refactor; only the `CharacterArchetypeSpec` STRUCT + a brain-key string map (`CharacterRoster`) remain. Classic docs-describe-dead-things (per the standing rule). Surfaced during the step-6 roster rename (the `Enemy*`→`Character*` sed renamed the dead refs in place, so they now read `CharacterArchetype::…`).
- **Noticed while:** unified-actors step-6 rename (character-roster vocabulary).
- **Suggested fix / size:** S — reword each comment to describe the real path (brain-key → `CharacterArchetypeSpec` via `CharacterRoster`), dropping the phantom-enum method calls. Pure docs; no code.

## 2026-07-02 STILL OPEN after control pass 3 — the two remaining forks — BOTH RESOLVED 2026-07-02 (see below)
- **Melee: two DRIVER systems remain. [RESOLVED 2026-07-02, commit 76b92010]** `attack_advance_system` (player) and `start_enemy_melee_from_brain_actions` + the `update_ecs_actors` inline active-edge (actor) are DELETED, replaced by ONE body-generic pair `combat::attack::{start_body_melee, advance_body_melee}` over `BodyClusterQueryData` (no `PlayerEntity` filter). The actor melee TIMER advance (`em.attack.tick`) is out of `em.update` — movement integration owns movement only; the AI reads `em.attack` as of the prior frame's advance (one consistent view). `start_attack`/`advance_attack` are body-generic (Option anim; effective-faction picks the damage channel the resolver already distinguishes; sprite_character_id picks the manifest box). Pinned by `unified_melee.rs` (player + hostile actor, same lifecycle), `possession_end_to_end.rs`, `enemy_attacks_player.rs`.
- **Boss possession = movement only. [RESOLVED 2026-07-02, commit bc6b0400]** Boss authored specials are now a persisted body CAPABILITY (`BossCapability`, derived from the pattern cfg at spawn, surviving the brain swap); the `Brain::Player` boss arm maps attack → primary strike / special → signature content special onto the SAME `BossAttackState`; `update_ecs_bosses` gates player-damage off for a player-controlled boss. Pinned by `boss_possession_specials.rs`.

## 2026-07-01 Player vs actor MELEE: two DRIVER systems around shared primitives — ✅ RESOLVED-BY-DRIFT 2026-07-19
- **RESOLUTION (2026-07-19 deep-review triage):** the melee moveset unification (2026-07-15) deleted BOTH drivers — `attack_advance_system`, `start_body_melee`, `advance_body_melee`, `start_enemy_melee_from_brain_actions` all have zero hits at HEAD. Melee is one `"attack"`-verb moveset move for every body (see the BIFURCATION "FULLY RESOLVED — one path" note further down this file, which this stale Open entry predated).
- **Where:** `combat/attack.rs` (`attack_advance_system`, player-cluster) vs `features/ecs/actors/update.rs` active-edge + `brain_effects.rs::start_enemy_melee_from_brain_actions` (actor-cluster).
- **Smell:** the melee is already ~80% converged — BOTH paths use the SAME `BodyMelee { swing: Option<MeleeSwing> }` component, the SAME spec pipeline (`resolve_attack_intent_from_view` → `attack_spec_from_view` → `into_world_frame`, see `begin_melee_attack`), and the SAME `spawn_melee_strike`/`emit_melee_slash`. The CONTROL fork is gone: `attack_advance_system` now consumes by `ControlledSubject` (not `PrimaryPlayerOnly`) and the actor consumer is keyed by `msg.actor` — neither is a primary-player identity gate. What remains is TWO thin DRIVER systems wrapped around the shared primitives (the player's has pogo + `PlayerAnimState` + self-impulse/commit; the actor's ticks inside the `update_ecs_actors` monolith).
- **Noticed while:** control-convergence pass 2.
- **Suggested fix / size:** L — the fighter-unification mega-project (memory `project_fighter_unification`, S4/S5/S6). Merge into ONE `advance_body_melee` querying `ae::BodyClusterQueryData + BodyMelee + ActionSet` (matches player AND actors — both carry the ancillary clusters), pulling melee timing/active-edge out of `update_ecs_actors`. Deliberately NOT done blind in the control pass: it changes the player's core combat feel (pogo/directional/timing) and can't be GUI-verified here — the exact "right shape first, verify feel after" work that needs Jon at the controls. Bosses stay separate (no ancillary clusters).

## 2026-07-01 Possessed BOSS moves but can't yet trigger its specials — ✅ RESOLVED (G5), verified 2026-07-15
- **RESOLUTION:** the G5 possession-verb work landed `possessed_attack_choice` in `bosses/tick.rs`: a melee press reduces the controller aim to an `AttackDir` and walks `directional_verb_chain` over the profile's authored `possessed_verbs`; the special button resolves the `"special"` verb; a boss authoring no verbs keeps the legacy deterministic mapping (melee → slot(0) strike, special → signature content special). Pinned by `possession_verb_map_tests`. This entry predated G5 and was never closed.

## 2026-07-01 HUD / debug overlay still read the home avatar, not the controlled subject
- **Where:** ~~`ambition_render/src/hud.rs`~~ (player-facing status HUD — RESOLVED 2026-07-01), `ambition_app/src/app/hud.rs`, `ambition_app/src/dev/debug_overlay.rs` (dev surfaces — still `PrimaryPlayerOnly`).
- **Smell:** camera / portal viewer / nameplates now follow `ControlledSubject` (the body carrying `Brain::Player`), but HUD + debug overlay still key on the home avatar. While possessing, they show the home body's health/gizmos, not the driven body's.
- **PARTIAL RESOLUTION 2026-07-01:** the player-facing status HUD (`ambition_render/src/hud.rs`) now reads the **controlled subject** for EVERY stat — HP, mana, AND money — and refills the controlled body's mana. Economy is a body concern (an NPC / merchant carries its own `BodyWallet` + inventory; cf. Fork E pickup-on-`ControlledSubject`), so the wallet is just another body cluster the driven body may hold — `Option` only because not every body carries one yet (a wallet-less body reads `$0`). Possessing a body spends ITS purse, not the home avatar's. One body-generic query drives it all. Pinned by `hud::tests::hud_tracks_the_controlled_body_for_every_stat_including_money`.
- **Still open (dev surfaces, lower priority):** (1) `app/hud.rs`'s debug text HUD is *intentionally* primary-scoped (its own doc says co-op would get per-slot panels, not a generalization of this one) — arguably not a bug. (2) `debug_overlay.rs` mutably queries the player CLUSTER with `Without<FeatureSimEntity>` to dodge a B0001 conflict; repointing it at a possibly-`FeatureSimEntity` subject reintroduces that conflict, so it needs a read-only stat split from the mutable gizmo-preview query first.
- **Noticed while:** possession/control unification (this refactor).
- **Suggested fix / size:** S remaining (debug-overlay stat labels → controlled subject via a read-only sub-query); the player-facing win is done.

## 2026-06-26 Characters are defined by named `EnemyArchetype` rows, not by their movement kit — ✅ RESOLVED 2026-07-15 (commit 60179c706)
- **RESOLUTION 2026-07-15 (commit 60179c706):** the three sub-smells this entry bundled are all closed. (1) **Named-enum roster** — GONE (resolved-by-drift before this pass): the `CharacterArchetype`/`EnemyArchetype` enum no longer exists; resolution is a pure string brain-key → `CharacterArchetypeSpec` lookup (`CharacterRoster.by_brain: HashMap<String, _>`) assembled from per-App provider fragments (`CharacterRosterFragment`/`Registry`), and movement tuning already COMPOSES via `inherits` (`resolve_movement_inheritance` folds `BASELINE ← parent ← this row's patch`). (2) **Player-ability composition** — `AbilitySet` union/intersect + `AbilityGrant` bundles → `AbilityBase` (commit 6875aeaea). (3) **Enemy movement kit as a frozen bundle of loose bools** — THIS pass. A character's movement verbs (blink/fly/shield/dash) were authored as `smash_can_*` bools on the flat row, projected to the brain-attempt (`SmashCfg.can_*`) AND stored a THIRD time as `CombatCapabilities.can_*` — a redundant mirror of the body's own `AbilitySet` (which `from_caps` already rebuilt from them). Collapsed to ONE authority: `CombatCapabilities` now carries only combat-CONSEQUENCE traits (death behaviors + weapon drop); the movement verbs lose the misleading `smash_` prefix (they're body capabilities, not Smash-template tuning) and feed one `CharacterArchetypeSpec::movement_kit() -> ae::AbilitySet` that BOTH ports read — the body unions it in at spawn (`ActorBody::from_kit`, enforce) and the Smash brain reads the same verbs (`brain_spec`, attempt). The body's `AbilitySet` (`BodyAbilities`/`AbilityBase`) is now the single movement-capability authority for player AND enemy — the exact "characters are defined by the movements they have, in the player's own vocabulary" the smell asked for. Behavior-preserving; workspace all-targets green, 783+370+105 unit tests + boss-possession app test green. NOTE (deliberately NOT chased — separate, lower-value polish): the movement kit is still authored as four sibling RON bools rather than a `Vec<AbilityGrant>` list (would need new Blink/Fly/Shield/Dash grant variants in engine-core; only 2 rows author the kit, so the ceremony isn't worth it yet); and HP/brain-template/melee/ranged still live as sibling fields on the (now string-keyed, inheritance-composing) spec row, which is fine — the "frozen NAMED bundle" was the smell, not "a struct with fields."
- **(historical) PARTIAL 2026-07-15 (commit 6875aeaea):** the PLAYER-ability axis of this now composes — `AbilitySet` gained `union`/`intersect`, `AbilityGrant` is a composable vocabulary, and a catalog row lists grants that union into `AbilityBase` (see the RESOLVED item ~line 386). The ENEMY-archetype-row bundle (HP + brain + `smash_can_*` caps as one frozen row) is STILL open — that is the bigger fish this entry is really about. *(Superseded by the resolution above.)*
- **Where:** `ambition_content/assets/data/enemy_archetypes.ron` + `EnemyArchetypeSpec` (`ambition_actors/src/features/enemies/mod.rs`); the spawn path resolves a string brain-key → a fixed archetype row that bundles HP + tuning + brain template + the capability flags (`smash_can_blink`, `smash_can_fly`, melee/ranged specs, …).
- **Smell (Jon, 2026-06-26):** "There really shouldn't be archetypes; characters should be defined by what movements they have available to them." An archetype is a frozen bundle; the elegant model is a character = a **capability/kit set** (which verbs its body has: blink, fly, shield, dash, ledge, melee/ranged shapes, tilts, special) + tuning, composed freely, not picked from a closed roster of named rows. The S3 capability work is incrementally pushing this way (each verb is now a per-body `CombatCapabilities` flag projected from the spec into the body AND the brain), but the *source* is still a named archetype row rather than a kit the body simply HAS. The closed-archetype shape is the body-side analogue of the closed-`SpecialActionSpec`-enum tension already noted for the engine-for-other-games goal.
- **Noticed while:** wiring blink (S3a) + fly (S3b) as body capabilities for the PCA — each verb needed a `smash_can_*` field threaded through the archetype row → spec → (brain cfg + body caps), which is the seam a kit-first model would make unnecessary.
- **Suggested fix / size:** L, NOT now (explicitly deferred by Jon — "just a smell to log"). Direction: let a character author its capability set + tuning directly (data), drop the named-archetype indirection; the brain reads the kit, the body enforces it. Dovetails with the fighter-unification roadmap's "per-body capability set" and the engine-for-other-games keystone.

## 2026-06-26 `BrainSnapshot.wall_contact` defined + read but NEVER populated — ✅ RESOLVED 2026-07-15 (DELETED)
- **RESOLUTION (commit aad8f81e1):** investigated populate-vs-delete; DELETE won. The puppy slug (the only Wanderer) is a `surface_walker` whose wall/surface response is the AdhesiveCrawler MOTION MODEL (kernel wraps corners) — the brain-level climb decision is superseded, and its `climbing` flag had no reader; the reverse arm is owned by the grounded integrator's patrol wall-stop (populating would double-flip against it). Deleted `WallContact`, the field, `WandererState`, and the `climb_walls`/`chatter_*` knobs through the catalog schema. Behavior-identical (the branch was unreachable).

## 2026-06-26 `ObservationFrame` flat-struct field additions ripple to 3 test literals
- **Where:** `ambition_characters/src/brain/smash/{action,mode,emit}.rs` each have an `obs_at(...)` test helper that builds a full `ObservationFrame { .. }` literal.
- **Smell:** the same shape as the 2026-06-23 `BrainSnapshot` entry, one level down. Adding `self_aerial` forced edits to all three. There is no `ObservationFrame::idle()`-style constructor to `..` from.
- **Noticed while:** adding the `self_aerial` field for the aerial brain.
- **Suggested fix / size:** S — add a `#[cfg(test)] ObservationFrame::at(distance_x)` (or a `Default`) in `observation.rs` and have the three `obs_at` helpers delegate, so a new field touches one place.

## 2026-06-21 Dead `landed`/`killed` scaffold in `advance_attack` — ✅ RESOLVED-BY-DRIFT 2026-07-15
- **RESOLUTION:** stale — the melee-unification collapsed `advance_attack`/`attack_advance_system` into the body-generic `combat::attack::{start_body_melee, advance_body_melee}` (now `features/ecs/attack.rs`) and `game/ambition_app/src/app/world_flow/attack.rs` no longer exists. `grep 'landed = false|killed = false'` is empty across the tree — the hardcoded-false gates (dead connect-sound + dead pogo-on-landing block) are gone with the old system. Nothing to delete. NOTE: the *pogo-off-a-landed-enemy* QUESTION the entry raised (does an enemy pogo-bounce fire?) was never separately answered; if pogo-off-enemy is a wanted feel, it's a fresh gameplay item, not this dead scaffold.

## 2026-06-13 Docs reference deleted RON-based levels — ✅ RESOLVED 2026-07-15 (already correct)
- **RESOLUTION:** swept it. The canonical world-authoring docs already flag RON room manifests as historical (`docs/concepts/ldtk-world-composition.md:25` "LDtk is the current world/level authoring source. Old RON room manifests are historical."; `docs/systems/ldtk-world-composition.md` lists "Old RON room manifests are historical" + "Treating old RON room docs as current" as an anti-pattern). Grepping the current-GUIDANCE doc set (`docs/systems`, `docs/recipes`, `docs/concepts`) for present-tense RON room/level authoring turned up only `sprite-rendering-surface.md:103` `assets/sprites/*.ron` (a live sprite manifest, not a level). The remaining 81 `.ron`+room/world/level hits are in brainstorms/ADRs (legit historical snapshots — must NOT be rewritten) or name RON's LIVE roles (tuning/audio/catalogs). No live-guidance doc misleads. No change.

## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit — ✅ RESOLVED-BY-DRIFT 2026-07-19
- **RESOLUTION (2026-07-19 deep-review triage):** the enum moved to `crates/ambition_platformer_primitives/src/feature_kind.rs` and the `Sandbag` variant is GONE — collapsed into `Actor` ("one actor kind covers enemy/NPC/boss/sandbag"). Remaining variants: `Actor, Hazard, Breakable, Chest, Pickup, Switch`. Residue: an inert `FeatureVisualKind::Sandbag` line in the gitignored `entity_manifest.yaml` (nothing parses it) and legitimate content display-names.

## 2026-06-10 Special-attack EFFECTS consumers are half-vocabulary (post de-name) — ✅ RESOLVED 2026-07-15 (commit d7aa8f2c7)
- **RESOLUTION:** mostly resolved-by-drift, closed the residual. The consumers MOVED off `ambition_actors/brain_effects.rs` into content Techniques (`game/ambition_content/src/bosses/specials/*.rs`) and the projectile ART became data-driven (`ProjectileVisualKind::Apple.to_tag()`, no owner-id substring read) — both since 2026-06-15. The ENGINE layer was already honest (`SpecialActionSpec::Special(String)`; "the engine names no boss special"). What REMAINED was content-crate vocabulary drift, now fixed: (1) `spawn_gnu_apple_rain_*` → `spawn_apple_rain_*` — the lone fn-name outlier among 10 technique-keyed siblings; (2) owner-id prefixes de-named off the wielding boss onto the technique (`gnu_ton_apple` → `apple_rain`, `smirking_behemoth_eye_beam` → `eye_beam`) — behavior-safe because projectile self/friendly-fire filtering is ENTITY-based (`hitbox.owner`), the owner_id string is trace/nameplate-only; (3) doc comments migrated from DEAD enum names (`MemorizedVolley`/`PitTrap`/`RotatingCross`/`MinionCascade`/`DebrisRain`) to the live technique keys (`overfit_volley`/`minima_trap`/`saddle_point`/`gradient_cascade`/`apple_rain`); (4) a stray GNU-ton reference dropped from a generic-brain comment. LEFT (legitimate, NOT the smell): content-module docstrings that name which boss a Technique kit belongs to ("Gradient Sentinel kit", "Smirking Behemoth eye-beam"); the enum's historical migration note listing the collapsed variant names; encounter-specific test names (`gnu_ton_apple_rain_*` test the gnu_ton pattern); and the GNU-ton art asset PATH (`sprites/gnu_ton_boss/gnu_ton_apple.png` — real boss art). NOTE: the smell's original "lift baked constants into RON spec fields" ambition is deferred — the APPLE_RAIN_*/OVERFIT_VOLLEY_* tuning consts still live in code (content-owned, "just numbers"), not RON; that's a data-authoring nicety, not a vocabulary smell, and a separate item if wanted. ambition_content all-targets clean, 9 specials tests green, no rustfmt cascade.

## 2026-06-15 Gravity-inversion residual design questions — ✅ RESOLVED 2026-07-15 (Jon: all frame-relative, no exceptions)
Jon's call: every one should be frame-relative, always, everywhere. On inspection three of the four were already done by later refactors; only the thrown-item LAUNCH was a live gameplay gap.
- **Directional attack hitbox offset** — ALREADY frame-relative: `AttackSpec::into_world_frame(frame)` (attack.rs:287) rotates `hitbox_offset`/`half_size`/`self_impulse`/`knockback` through `frame.to_world` before the consumer adds it to `view.pos`. The only raw (un-rotated) `attack_hitbox_from_view(view, attack_spec_from_view(...))` calls are `#[cfg(test)]` helpers.
- **Player knockback** — ALREADY frame-relative: the `as_engine()`-without-`apply_gravity_dir` path was replaced by `resolved_body_knockback_velocity` → `frame.to_world(local)` (damage_apply.rs:505) off the VICTIM's `ResolvedMotionFrame.down()` (:759). Launch is side-away-from-source + rise-against-gravity, rotated by the body's frame.
- **Thrown ground-item physics** — the free-fall INTEGRATION was already gravity-relative (`gravity.dir_for` per-position + `apply_world_forces`; `GROUND_ITEM_GRAVITY` is just the magnitude). FIXED the remaining screen-locked bits: the throw LAUNCH velocity + ahead-offset are now authored body-local (forward/up) and rotated via `AccelerationFrame::new(gravity.dir_for(...)).to_world(...)` (identity under normal gravity), and the OOB despawn-guard generalized from world-down-only to all four world bounds so a side-flung item under a flip still parks.
- **`ground_gap_below_feet`** (room-transition landing diagnostic, log-only) — now probes along the transiting body's `ResolvedMotionFrame.down()` instead of world-down: `feet_coord`/`head_coord`/`gravity_half` project blocks onto the gravity + side axes. Behavior-identical under normal gravity. (`TransitBodies` gained a read-only frame query, read before the mutable cluster borrow.)
Verified: actors --lib 783 green (existing throw tests pin the normal-gravity parity); the throw/knockback both ride the same tested `to_world` seam as aim — no new symmetry test added (identity-guaranteed change, no bureaucratic bloat).

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

## 2026-06-23 Settings: IR options absent from the pause-menu SettingsItem surface — ✅ RESOLVED 2026-07-15 (Jon: it's a bug)
- **RESOLUTION:** Jon's call — the pause menu should mirror the cube. The gap was actually FIVE cube-curated options with no pause row: `VisualQuality` + `FramePacing` (Video) and `MovementFrameMode` + `AimFrameMode` + `PortalReverseFacing` (Gameplay). (The smell's "InputFrameMode" is the movement/aim split; it also missed `VisualQuality`.) Added each as a `SettingsItem` variant + `rows_for` row (in cube order) + `shared_option_id` mapping + the two exhaustive-match arms (apply + label); label/apply flow through the shared IR automatically, so the pause menu and cube can't drift. Verified all 5 are genuinely wired to gameplay (frame modes → `control/input_systems.rs`; portal → `PortalTuning::reorient_facing`), so this surfaces LIVE settings desktop/pause users couldn't reach. Updated the stale `shared_option_id` doc comment. menu 90 + settings 11 tests green. NOTE (still true): there is no pause↔cube parity guard — the pause menu was unguarded, which is why this stayed silent. A test asserting every cube-curated `SettingsOptionId` is either a `SettingsItem` or on an explicit pause-omitted allow-list would catch the next one (deferred — not adding a guard test in this pass per the no-bloat steer).

## 2026-06-23 Manual `const ALL` lists shadow exhaustive enum matches (drift-prone, add a guard)
- **Where:** `assets/game_assets/entity_sprite.rs` (`EntitySprite` enum + `const ALL` at ~:104 + `relative_path`/`entity_sprite_asset_id` matches); `assets/game_assets/mod.rs` (`ParallaxTheme`/`ParallaxLayerAsset` enums + their `const ALL` + `key()`/`from_key()`).
- **Smell:** the `match` arms are exhaustive (compiler catches a missing variant), but the hand-maintained `const ALL: [_; N]` is NOT — adding a variant without updating `ALL` silently drops it from every `ALL`-driven iteration (asset preload, round-trip checks). No current mismatch.
- **Why not fixed:** the only robust fix is a guard test, and `ALL.len() == <variant count>` needs `std::mem::variant_count` (nightly) or a strum-style derive; a plain test that round-trips every `ALL` entry through `key()`/`from_key()` is the pragmatic option. Adding tests is lower-value than dedup and preventive only.
- **Suggested fix / size:** S each — a round-trip test per enum (every `ALL` entry resolves through its match and back), or adopt a derive that generates `ALL`.

## 2026-06-23 Diverging test fixtures NOT consolidated (verified, leave)
- **`EncounterSpec` builders** — `encounter/state.rs::spec()`, `encounter/rewards.rs::spec_with_trigger()`, `encounter/tests.rs::lab_spec()` each rebuild the 9-field `EncounterSpec`, but with different values (camera_zoom 1.2/1.0/1.5, reward `Health{2}` vs `default_encounter_reward()`) and different parameterization (waves vs trigger vs fixed). A shared builder would need a base+override shape; values already differ per test, so forcing it risks changing fixtures tests depend on. Left.
- **`LdtkProject` synthetic fixtures** — `world/ldtk_world/tests/kinematic_paths.rs` has 6 `LdtkProject` literals. The world-audit agent rated them ~95% identical, but that held only for the first two: across all six the level dimensions diverge (only 2 use 640×480 / 40×30; one project has multiple levels). A single `synthetic_project(id, entities)` helper won't fit without a wide builder (px_wid/hei/c_wid/hei params), at which point the call sites aren't much shorter. Left.

## 2026-06-23 Portal LDtk-emission field vs helper feature-gate mismatch — ✅ RESOLVED-BY-DRIFT 2026-07-15
- **RESOLUTION:** stale — a later refactor already fixed it. In `ambition_ldtk_map/src/conversion/mod.rs` the `portal_gun_spawns` field is now UNGATED (always present, an empty Vec when the feature is off), and the only asymmetry left is intentional + consistent: the builder `portal_gun_spawn()` + the `convert_portal_gun_spawn`/`convert_portal` converters carry BOTH a real `#[cfg(feature = "portal_ldtk")]` impl AND an explicit `#[cfg(not(...))]` fail-loud stub (`portal_compiled_out`). Verified `cargo check -p ambition_ldtk_map --no-default-features --features "ldtk_runtime,portal"` builds clean. No `#[cfg(feature = "portal")]`-vs-`portal_ldtk` field/helper split remains. No code change.

## 2026-06-23 `dialog_lint` fixed-arity command table is hand-synced + untested
- **Where:** `dialog_lint.rs` (`FIXED_ARITY_COMMANDS` ~:19-31) must match the `In<...>` arities of the `cmd_*` fns in `dialog/yarn_bindings.rs`; the comment says "MUST match" but nothing tests it.
- **Smell:** a new dialog command added without updating the table is silently un-linted. Manual parallel list, no guard.
- **Suggested fix / size:** M — a test that scrapes the `cmd_*` signatures (or a registry the bindings already build) and cross-checks the table.

## 2026-06-26 Glob-import seam map (from `clippy::wildcard_imports` sweep) — PARTIAL 2026-07-15 (3 façades landed)
- **PARTIAL 2026-07-15 (round 2):** two more cleanly-separable façades untangled the same way. **`ambition_render/rendering/actors`** — animation/boss/overlays each dropped `use super::*` for explicit imports (bevy prelude stays; externals from canonical crates); all their "parent" refs turned out to be comment mentions, so zero coupling. Dropping the hub's `#![allow(unused_imports)]` also surfaced 2 genuinely-dead hub imports (`AabbExt`, the boss-sprite trio now living in `boss.rs`) — removed. Forward `pub use x::*` kept (real public render API). **`ambition_world/rooms`** — the 5 leaf spec modules (camera/gate_portal/loading_zone/metadata/specs) + `spawn` went explicit (each used only 1–2 external names + at most 2 sibling spec types); dropping the `allow` revealed the hub's `Resource` import existed ONLY to feed submodules (never used by mod.rs) — removed. **Key judgment:** `room_graph.rs` and `graph.rs` are AGGREGATORS that legitimately consume the whole sibling spec vocabulary (RoomSpec, PropSpec, LoadingZoneActivation, …) — for them `use super::*` is the right idiom, NOT the kitchen-sink smell, so they keep it. A module ending up half-explicit (leaves) / half-glob (aggregators) is the principled end state, not inconsistency. render 44 + world 36 tests green, both crates warning-clean. **`ambition_runtime/snapshot` RECLASSIFIED → coupled (skip):** codecs.rs (1402 lines), registry.rs, restore.rs each heavily use the parent's shared serialization API (`put_f32`/`SnapshotState`/`Reader`/…) — the mod.rs comment documents that shared-core intent deliberately. Full explicit imports = re-listing the module's own API (high churn, re-break-prone), same as `cut_rope`. Left as-is.
- **PARTIAL 2026-07-15 (round 1):** the named worst offender — `bosses/specials/mod.rs` — is fully untangled. The 7 Technique submodules each dropped `use super::*` for explicit imports (`bevy::prelude::*` stays as the one canonical glob; everything else named); the hub's 7 `pub use x::*` re-exports became one curated `pub use x::{State, spawn_fn}` per submodule (nothing outside the module consumes them, but they're the Techniques' real public API); the `#![allow(unused_imports)]` mask is gone; the test modules' `super::super::*` grandparent-glob is gone (only `gradient_nova`'s needed anything — `WorldTime` — now explicit). ambition_content lib + tests compile warning-clean; 9 specials tests green. **REMAINING back-glob worklist (30 sites, 10 crates)** — a submodule that does `use super::*` under a `pub use submod::*` hub: `ambition_world/rooms` (6), `ambition_combat` (5: lib + components + events + hazard_runtime + path_motion + moveset/prefabs), `ambition_runtime/snapshot` (3), `ambition_render/rendering/actors` (3), `ambition_sprite_sheet` (4: character/sheets + game_assets), `ambition_actors` (4: features/ecs/actors conversion+update, boss_encounter/attack_geometry/aabb, features/ecs/bosses), `ambition_dev_tools` (2), `ambition_characters/brain/boss_pattern/tick`, `ambition_engine_core/ledge_grab/runtime`, `ambition_content/bosses/cut_rope` (arena+victory). NOT all are equal: `cut_rope` and `actors/update` are deeply-coupled flat-modules whose submodules legitimately share the parent's PRIVATE const namespace — full explicit imports there are high-churn with little gain; the real win is curating the `pub use *` re-export (which over-exposes) + dropping the `allow`, keeping the intra-module glob. Judge per-case; don't blind-sweep. Whole-crate re-exports (`pub use ambition_vfx::*` etc.) are deliberate façades, NOT this smell.
- **Context:** `cargo clippy --fix -W clippy::wildcard_imports` over the workspace. Preludes are correctly exempt; only **4** named globs auto-expanded losslessly (committed `dbe143c4`). The rest are the smells.
- **~93 named globs clippy *refuses* to auto-expand** — these are the seam-y ones: re-export façades, enum-variant globs (`use PortalChannelColor::*;`, `use Enum::*`), and names fed into macros. clippy marks them `MaybeIncorrect`, so they need manual judgement. This refusal set *is* the untangle worklist; expanding one often reveals a façade module that should be a curated `pub use`.
- **Production `use super::*` is pervasive (143 non-test sites).** clippy's default only exempts `super::*` *inside test modules*, so it would expand all 143 production ones (out of scope for the named-only pass). Worst case observed: `ambition_content/src/bosses/specials/gradient_sentinel.rs` — `use super::*` expands to a 27-name grab-bag including std `vec`/`format`/`ToString`, i.e. `specials/mod.rs` is re-exporting a kitchen-sink prelude. That re-export hub is the real smell to break up.
- **Tooling note:** `cargo clippy --fix` applies *every* active lint, not just the `-W` one — the broad run also silently did `derivable_impls` (lossy: collapsed a `#[cfg(target_os="android")]` `MenuTapMode::default()` into `#[derive(Default)]`, dropping the Android branch), `explicit_auto_deref`, and a `PHI_FRAC` float-precision rewrite. `-A warnings -W clippy::wildcard_imports` over-suppresses and fixes nothing. So: run broad, then revert every hunk that isn't a glob expansion. No clean single-lint `--fix` incantation found.
- **Suggested fix / size:** L, incremental. Per façade module: expand the glob, see what leaks, convert the hub to an explicit curated `pub use`. DONE 2026-07-15: `bosses/specials`, `render/rendering/actors`, `world/rooms` (leaves). Skipped as coupled (aggregator/shared-core, glob is correct): `runtime/snapshot`, `cut_rope`, `actors/update`, world `room_graph`/`graph`. The cleanly-separable high-value ones are now done; remaining sites (`ambition_combat`, `ambition_sprite_sheet`, `ambition_dev_tools`, `boss_pattern/tick`, `ledge_grab/runtime`, `attack_geometry/aabb`) are unaudited — assess coupling per-case before touching, most are likely aggregators.

## Resolved

- **2026-07-19 A `portal_render` host composition without `PortalPlugin` panicked, and the gate could not see it** — ✅ RESOLVED 2026-07-19 (`e4edd4acb`). The composition decision: `ambition_host/portal` FORWARDS `ambition_runtime/portal` (the facade already co-forwards the pair at the composition root; `PlatformerEnginePlugins` installs `PortalPlugin` under that feature), so a host-standalone graph composes a complete sim — `demo_shell_smoke` went 5-red → 6/6 green under `portal_render` with zero test edits. Both halves closed in order: composition first, then `ambition_host` dropped from `SKIP_FEATURE_JOB` (its "gates no test code" claim had gone false when the feature-gated portal seam tests landed), with the same-commit rule now written at the skip-list site.

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
- **2026-06-21 Alpha-clobber audit surface (sprite renderer)** — [2026-07-19 re-audit: raw `ImageDraw.Draw(` sites have GROWN to **373** (was ~139); the eyeball audit for translucent-over-content clobbers remains undone and the parity harness still cannot catch them] — drawing a translucent fill straight onto an RGBA image with `ImageDraw.Draw(img)` *replaces* the destination alpha (clobbers underlying content) instead of blending; correct path is a scratch layer + `Image.alpha_composite` (the "gnu_ton rule"). Flagged by Jon (recurring agent mistake; likely latent bugs exist). Added the canonical primitive `core/draw.overlay_draw` (+ `composite_polygon`), pinned by `tests/test_core_overlay.py`. TODO: (a) unify the 3 existing scratch-layer copies onto `overlay_draw` — `generic_explosions._overlay_draw` DONE (delegates to core, parity-clean, since core uses the same `"RGBA"` scratch mode); `skeleton.composite_polygon` uses PLAIN `Draw` (not `"RGBA"`) so its unification would shift overlapping-translucent pixels → needs a parity-checked bless; rigdoc painter still TODO. (b) audit the ~139 plain `ImageDraw.Draw(img)` sites for translucent-over-content clobbers — **the pixel parity harness CANNOT catch these** (they render consistently wrong, so there's no before/after drift). Needs eyeball/heuristic, not the harness.
- **2026-06-25 `DebugOverlayLabels` leaks a `pub(crate)` type through a `pub` field** — `dev/debug_overlay/prims.rs:198` `pub struct DebugOverlayLabels(pub Vec<DebugLabel>)` exposes `DebugLabel` (only `pub(crate)`) at `pub` visibility → `private_interfaces` warning. Pre-existing; surfaced when the app-module blanket `#![allow(unused_imports)]` came off (2b2b88f1) but unrelated to it. Fix: make the field `pub(crate)` (or `DebugLabel` `pub`). Left unaddressed — out of scope for the glob untangle.
- **2026-06-26 [RESOLVED 2026-06-28, commit cc3e972e] precision-blink test was stale, not a real bug** — the test (`player.rs`) asserted quick-blink `blink_quick_dir == (-1,0)` (locomotion-framed) under sideways gravity, but `InputFrameMode::DEFAULT_MOVEMENT` is `ScreenRelative`, so quick blink IS screen-relative by default (got `(0,-1)`), exactly like precision blink — matching in-game behavior. Rewrote the test to pin BOTH the screen-relative default AND the seam (flipping only the locomotion mode to body-relative rotates quick blink while precision stays screen-directed).
- **2026-06-28 Duel "Player" robot renders ~21% smaller than the controlled player** — NOT a stale/duplicate character (same `player_robot` sheet, same `player_robot` archetype → same Hadouken+Swipe VFX). Two render-sizing paths diverge: the player uses `player_placeholder_render_size` = `sprite_render_size_scaled(spec, 30×48, 1.16)` (the `PLAYER_PLACEHOLDER_VISUAL_SCALE` "heroic" boost) while the generic actor path (`sprite_render_size_for_name` → `sprite_body_collision_for_character_id`) uses `sprite_render_size(spec, ldtk_box)` with no heroic scale and the duel's 28×46 box (vs the player's 30×48). Net: 48·1.35·1.16 = 75.2 px tall (player) vs 46·1.35 = 62.1 px (duel robot). Elegant fix = promote the 1.16 from the player ENTITY to a character-catalog `render_scale` (display-only — must NOT leak into the body-metrics-derived hitbox in `sprite_body_collision_for_character_id`), so every spawn of the `player` character (player or duel copy) draws identically; then the duel robot also wants the player's authored 30×48 body box. Deferred: blind visual change (touches the PLAYER's render path + catalog schema) needing GUI confirmation of the target look — offered to Jon as a confirmable follow-up.
- **2026-06-27 `enemy_archetypes.ron` / `EnemyArchetypeSpec` / `EnemyRoster` / `EnemyBrain` are misnomers** — now that the protagonist is authored as the `player_robot` archetype (and the roadmap makes player/enemy just controller+capability DATA, not types), these are *character* archetypes, not "enemy" ones. Flagged by Jon. The rename (file + `EnemyArchetypeSpec`→`CharacterArchetypeSpec`, `enemy_roster`, `EnemyBrain`, `ALL_BRAIN_KEYS`, etc.) is a mechanical pass touching many refs and is INDEPENDENT of the movement-tuning/unification work — deferred as its own commit, not bundled. Jon also noted he's "not sure archetypes is a great design anyway" — so don't over-invest in the archetype concept; the rename is the cheap win, a deeper archetype rethink is separate.
- **2026-06-27 Stale `docs/planning/<oldfile>.md` breadcrumbs after the planning rewrite** — the `docs/planning/` tree was consolidated/renamed (see `docs/planning/MIGRATION.md`): old flat filenames (e.g. `fighter-capability-and-motor-unification.md`, `non-player-centric-actor-unification.md`, `restructuring-blueprint.md`) became `engine/unified-actors.md` / `engine/architecture.md` / etc. ~13 code-comment breadcrumbs in crates/ (and a few root TODO/*.md) still point at the OLD filenames. They're comments, not functional, so left for a mechanical sweep: `grep -rln 'docs/planning/[a-z-]*\.md' --include=*.rs crates/` then repoint to the consolidated doc (the MIGRATION table is the map). AGENTS.md is fine — it references the directory, which survived.
- **2026-06-27 Planning docs lag the body-vocab de-player-casing** — after the keystone moved the movement/economy vocabulary onto `crate::actor` (commits f3c8dff8 → 59653267), `docs/planning/engine/architecture.md`'s Bucket-2 component plan still names now-renamed/moved types: `PlayerWallet` (→`BodyWallet`), `PlayerShieldState`/`PlayerEnvironmentContact`/etc. (→`Body*`, all 18 clusters renamed), and `unified-actors.md`'s step-3 "(historical)" prose names dead symbols `ActorBody`/`PlayerClustersMut`/`integrate_grounded_body`/`integrate_aerial_body` (now `AncillaryMovementBundle` real components / `BodyClustersMut` / `ActorMut::integrate_body`). Also `ActorStatus.shield_raised` (retired, commit e0f65a78) may be referenced as a live bucket item. These are planning prose, not code, so left for a doc-refresh pass (deferred-intent additions in unified-actors.md ARE current). When refreshing: the Bucket-2 economy/movement slices are largely DONE; what remains is interaction-consumer + attack-state + safety/respawn. Don't trust a planning doc's component name without grepping the code first.
- **BIFURCATION: 2026-06-28 player vs actor MELEE attack pipeline (two state machines + two damage paths)** — the player and a brain-driven actor run PARALLEL melee pipelines, the keystone fork behind the recent "actor melee doesn't connect / no slash / not the same attack" bugs. Player: `PlayerAttackState` + `AttackSpec`, driven by `attack_advance_system` (`PrimaryPlayerOnly`, EXCLUDED from `update_ecs_actors` via `Without<PlayerEntity>`); damage = a per-frame Volume `HitEvent` (`HitSource::PlayerSlash`) each active frame, deduped via `hit_targets`. Actor: `ActorAttackState` (windup/active/cooldown + `pending_axis`, NO spec), driven by `update_ecs_actors`; damage = a persistent `Hitbox` ENTITY spawned at the windup→active edge, resolved by `apply_hitbox_damage`, deduped via `HitboxHits`. The `Hitbox`-entity path is the canonical one (bosses, actors, player AOE all use it). DONE so far (one-definition, not yet one-path): the SLASH visual now has ONE emitter `combat::attack::emit_melee_slash` that both call (commit after this entry). REMAINING merge (the real "ONE BODY ONE PATH"), per the explore map, in order: (1) player melee SPAWNS a `Hitbox` entity via `spawn_melee_hitbox` (Player faction) and DELETE the per-frame `HitEvent` loop — needs `apply_hitbox_damage`'s Player-faction branch to carry knockback (`knock_x`) + the damage multiplier (currently hardcodes `knock_x: 0.0`); keep pogo in the player path (player-only physics). (2) Merge `PlayerAttackState`+`ActorAttackState` into ONE body attack-state component (add `spec: Option<AttackSpec>` to the actor state; compute the actor's spec at `begin_melee_attack` instead of deferring geometry to the edge), and ONE driver that ticks it + spawns the hitbox + slash at the active edge for every body. (3) Fold the player into `update_ecs_actors` (drop `Without<PlayerEntity>`) OR have both call the one shared strike-spawn system; delete `attack_advance_system`'s bespoke damage/slash. RISK: (1) and (2) change the player's WORKING core melee feel (knockback/dedup/timing) and are BLIND (no GUI here) — drive with headless tests (damage connects, knockback preserved, one-hit-per-target, one slash) and have Jon verify feel; do NOT add any new player/actor-specific attack code in the meantime (route through the shared seam).
  - **UPDATE 2026-06-28 (mostly RESOLVED):** the slash visual, the swing MODEL, and the STATE COMPONENT are now unified. (a) `emit_melee_slash` is the one slash emitter; (b) the actor swing is resolved through the player's `attack_spec_from_view` + `AttackSpec` and stored on the shared state (commit `actor melee adopts the player's AttackSpec`); (c) step (2) is DONE — `PlayerAttackState`/`ActivePlayerAttack` AND the timer-based `ActorAttackState` are all DELETED, replaced by ONE `BodyMelee { swing: Option<MeleeSwing>, cooldown, ranged_cooldown, pending_axis }` carried by the player and every actor, built on the player's spec+elapsed `MeleeSwing` model (commits `merge actor melee state onto ONE BodyMelee` + `fold the player onto BodyMelee`). REMAINING (the last sliver): step (1)/(3) — the player still PRODUCES its melee damage as a per-frame Volume `HitEvent` from `advance_attack`/`attack_advance_system`, while actors produce it as a `Hitbox` ENTITY via `update_ecs_actors`+`apply_hitbox_damage`. Both feed the SAME resolver (`apply_feature_hit_events`), so this is two PRODUCERS of one event, not two resolvers. Collapsing it cleanly wants the player's per-swing string-key dedup (`MeleeSwing.hit_targets`, fed back by the universal resolver) reconciled with the hitbox's entity-key dedup (`HitboxHits`) — and the player's universal target coverage (breakables/orbs/bosses via `HitTarget::Volume`) preserved. Tracked; lower-risk to do once a body-melee DRIVER unifies the player+actor systems (currently still two systems gated by the not-yet-unified movement architecture).
  - **UPDATE 2026-06-28 (RESOLVED — producer collapsed):** the player melee now spawns a Player-faction `Hitbox` ENTITY through the SAME `combat::hitbox::spawn_melee_strike` every actor uses; the per-frame Volume `HitEvent` loop in `advance_attack` + the separate `start_attack` slash emit are DELETED. `spawn_melee_strike` derives BOTH the damage hitbox AND the slash from ONE gravity-resolved `world_box`, fixing the player's hitbox-vs-vfx divergence under C4 gravity (the player box now gates the screen-axis manifest box to upright, else the gravity-rotated spec box — the actor's rule). `Hitbox` gained `knock_x`; the Player FollowOwner branch emits the `PlayerSlash` Volume each active tick (deduped per-swing via `MeleeSwing.hit_targets`), World-anchored Player hitboxes (shockwave AOE) keep the once-only sentinel; `apply_hitbox_damage` resolves owner-pos via `CenteredAabb` OR `BodyKinematics` (player has no `CenteredAabb`). NOT a fork anymore — the ONLY remaining seam is the two DRIVER systems (`attack_advance_system` vs `update_ecs_actors`), which is the movement-driver question. Next: the unified action/ability timeline on this strike seam.
  - **UPDATE 2026-07-02 (driver seam RESOLVED):** the player/actor DRIVER split closed — `attack_advance_system` + `start_enemy_melee_from_brain_actions` + the inline `update_ecs_actors` edge collapsed into ONE body-generic pair `combat::attack::{start_body_melee, advance_body_melee}` over `BodyClusterQueryData` (commit `76b92010`). This left ONE last melee duality: two STATE MACHINES — the flat `BodyMelee`/`MeleeSwing` driver vs the `MovePlayback` moveset — bridged by `project_moveset_melee_to_body_melee`, selected per-body by the `MovesetMelee` marker (whichever a body had, only one ran).
  - **UPDATE 2026-07-15 (FULLY RESOLVED — one path):** melee is now a `"attack"`-verb MOVESET move for EVERY body; the flat driver is deleted outright. Gone: `start_body_melee`, `advance_body_melee`, `start_attack`, `advance_attack`, `slash_kind`, and the flat single-hitbox spawn `combat::hitbox::spawn_melee_strike` (its only caller was `advance_attack`). A body's swing is triggered by `combat::moveset::trigger_moveset_moves`, struck (window-scoped hitbox + slash, convex authored blades, charge scaling, on-hit techniques) by `advance_move_playback`, and projected back into `BodyMelee` for the anim/HUD/telegraph read-model by `project_moveset_melee_to_body_melee`. The enemy spawn now builds its moveset from `action_set.melee`/`ranged` (the SAME capability the brain's emit-gate reads, not the raw `spec.melee`), so every meleeing body is `MovesetMelee` — closing the flat-only gap (e.g. a held-weapon melee) definitionally. The surviving `BodyMelee` cooldown floors (ranged refire I3 + the legacy melee-recovery floor) tick in the trimmed `tick_body_melee_cooldowns`. Melee runs on the owner's PROPER time now (the moveset clock), so it dilates/pauses correctly for every body — proper time collapses to sim time for an undilated body, so normal play is byte-identical. Generic attacks do NOT pogo; pogo is a composed on-hit technique on the down-air move (`pogo_moveset_off_world_orbs` + `dispatch_hitbox_on_hit`). Behavior-preserving for normal play; workspace all-targets green, 105 combat + 370 characters + 782 actors + 32 moveset unit tests + `unified_melee`/`enemy_attacks_player`/`possession_end_to_end`/`boss_possession_specials` app tests green. NOTE (follow-up, PRUNED 2026-07-15 in the same session): the two production-dead melee leftovers are gone. (a) `combat::hitbox::spawn_melee_hitbox` (the flat single-hitbox primitive — no production caller after `spawn_melee_strike` went) deleted with its two re-exports + its unit test. (b) The `Hitbox.knock_x` field (the flat player's signed slash impulse, always `0.0` from every production spawn now that the moveset drives knockback via `volume.knockback`/`launch_dir`) removed from the `ambition_vfx::Hitbox` struct + all literals; its two `apply_hitbox_damage` read sites now emit `PlayerSlash { knock_x: 0.0 }` literally. IMPORTANT — the SEPARATE `HitSource::PlayerSlash { knock_x }` EVENT channel stays LIVE: `abilities/traversal/dive.rs` emits it non-zero (`local_dir.x * DIVE_KNOCKBACK`), consumed in `damage/actor_hit.rs` (`if *knock_x != 0.0`). Only the `Hitbox` component FIELD was dead, not the event variant.
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

## 2026-07-02 `cargo test -p ambition_content` silently SKIPS the portal tests — ✅ RESOLVED 2026-07-15
- **RESOLUTION:** added the `[dev-dependencies]` self-reference `ambition_content = { path = ".", features = ["portal"] }`, so the crate's OWN test build always enables `portal` (39 portal tests now run under a bare `cargo test -p ambition_content`; they were compiled out before). Chose the self-dev-dep over `default = ["portal"]` to keep zero blast radius on downstream crates — only content's dev/test profile gets the feature.

## 2026-07-02 Music audit gaps exposed by ear-testing gradient_ascent
- **Where:** `tools/ambition_music_renderer` audit suite vs Jon's timestamped listen notes.
- **Gap 1 - effects-induced pitch instability is invisible to every audit.** A pedalboard chorus at depth 0.15 / 0.8 Hz produced an audible "bend-down" percept on long sustained lead notes (heard at 0:06); nothing analyzes the RENDERED audio for pitch stability. A cheap detector: per-stem YIN/pYIN pitch track over sustained notes, flag cyclic deviation > ~25 cents. Same detector would catch authored bends that start detuned (the 2:45 issue - a note authored to START -150 cents flat).
- **Gap 2 - foreground-melody collision salience.** The dissonance audit DID flag the 1:17 clash (bar 40: whistle answer vs hook hang) but ranked it among pad-voicing add9s; nothing distinguishes "two simultaneous foreground melodies overlapping in the same register" (perceptually severe) from "chord-tone seconds inside a pad voicing" (fine). A lead-vs-lead overlap check - motif-kind layers sounding simultaneously within an octave - would have surfaced it as its own category.
- **Why not fixed now:** mid-composition session; both are S-M sized audit modules. The mix_balance/dissonance surfaces to extend are in `ambition_music_renderer/audit/`.
- 2026-07-02 (between_objectives): audit/lead_collision derived a note's bar via int(start_beat // beats_per_bar) on float-accumulated beats; a bar-boundary note arriving as 223.99999999999997 was floored into the PREVIOUS bar and flagged as a false exposed tension against that bar's chord. Fixed with a +1e-6 nudge before flooring (all three floor sites). Underlying smell: build_score events carry nominal_bar/nominal_beat that audits could use instead of re-deriving from floats.
- 2026-07-02 (pitch_stability tool limitation): `audit/pitch_stability` runs a monophonic pitch tracker over the rendered lead STEM, so it merges reverb-connected legato phrases into one "note" (observed note_seconds of 8.8s where the longest authored note is 2.9s) and reads the melody's own motion + sample vibrato as pitch "wobble". A verified-accepted cue (broken_transmitter, Sonatina violin lead) scores 23 wobbles/6 onsets up to 210 cents; between_objectives' flute scores 38/10 up to 400 cents at a fast melodic peak — same order of magnitude, i.e. normal for an expressive sustained SFZ lead, not a tuning defect. Real fix: segment the analysis on the MIDI note boundaries that `build_score` already emits (`_ambition_note_events` carry nominal_bar/nominal_beat/nominal_duration_beats) instead of re-detecting onsets from a reverberant stem. Until then, treat wobble_count as a guitar-scoop detector (sustained SINGLE notes bending), not a verdict on melodic leads.

## 2026-07-03 (fable-review T1 fallout) — entity-sprite generator emits dead `FeatureVisualKind::Npc/Boss` labels — ✅ RESOLVED 2026-07-15
- **RESOLUTION:** fixed the two dead rows in `entities.py` (`npc_terminal` + `boss_core`) to emit `FeatureVisualKind::Actor` (the live variant those actors map to). The generator is now correct on a fresh clone. No regen commit needed: the `entity_manifest.yaml` output is gitignored (0 tracked) AND no Rust parses the `category:` label, so the on-disk stale copy is inert. Left `sandbag_dummy` → `FeatureVisualKind::Sandbag` alone — that variant still EXISTS; its rename is the separate 2026-06-10 Sandbag→TrainingDummy item (needs `ambition_ldtk_tools`, not a blind edit).

## 2026-07-03 — `unified_body_movement` chase test fails — ✅ GREEN as of 2026-07-15 (root cause never pinned)
- **UPDATE 2026-07-15:** the test PASSES at HEAD (all 3 in the file), stable across repeated runs of the same build. Note the original evidence compared two DIFFERENT commits, so "25px swing between two builds" may have been legitimate code drift, not build nondeterminism. Leaving the entry as a breadcrumb: if it flips red again across otherwise-identical builds, suspect TypeId-keyed hash iteration (varies per compilation) in the chase pipeline. Original entry below.
## (original) 2026-07-03 — `unified_body_movement` chase test fails (pre-existing; smells of query-order non-determinism)
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

## 2026-07-06 (E5 step 5) — `apply_room_replay_request_system` hard-codes cut-rope content in the APP — ✅ RESOLVED 2026-07-15
- **RESOLUTION:** minted `ContentRoomReplayResetSet` (in `session/reset`, same pattern as `ContentRoomResetSet`/`ContentDialogueFollowupSet`). Content registers `reset_cut_rope_attempt_on_replay` (cut_rope/mod.rs) in that slot — it reads the same generic `RoomReplayRequested`, collects cut-rope placement ids, and calls the existing `reset_cut_rope_boss_attempt`. The app anchors the slot `.before(apply_room_replay_request_system)`, and the consumer dropped its 4 content-only params (`boss_registry`/`save`/`boss_music`/`cut_rope_bosses`), the cut-rope block, and both `ambition_content::bosses::*` references — so the host replay consumer now resets the player+world only and names no content. (Both systems read the message via independent reader cursors, so it lands the same frame.) Original entry below.
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

## 2026-07-09 — `ambition_actors::features::conversion_tests` is misnamed — ✅ RESOLVED 2026-07-15

Renamed `conversion_tests.rs` → `actor_movement_tests.rs` (`git mv` + the
`#[cfg(test)] mod` decl in `features/mod.rs` + the `features::ecs` re-export
comment). Dropped the redundant inner `#[cfg(test)] mod conversion_tests { }`
wrapper — the file decl already scopes the module as test-only — dedenting the
854-line body one level (14 tests still green). The name now matches the content
(headless NPC/enemy movement + collision, archetype tuning); no more fable-audit
mislisting as an `ambition_ldtk_map` test-travel candidate.

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

**UPDATE 2026-07-15 (another instance, fixed):** `cargo test -p
ambition_game_shell` (bare features) NEVER compiled — `input.rs` reads
KeyCode/Gamepad unconditionally but the crate's bare bevy dep carried no input
features; it only built when a co-built sibling unified them in. Fixed by adding
`bevy_input_focus` + `gamepad` to the base dep (commit 8e492f1a3). The class
stands: a lone `-p` build is the only thing that exposes these.

**UPDATE 2026-07-15 (the check was run):** ran the suggested
`cargo check --workspace --all-targets --all-features` — it came back CLEAN
(exit 0), no compile rot in any feature-gated target. The union of all features
happens to be coherent right now (no mutually-exclusive pair tripped a false
failure). Only survivor was a dead `#[cfg(test)] use ambition_sfx::SfxMessage`
in `features/ecs/mod.rs` (unused once its sole test consumer was cfg'd out under
the union) — deleted. Worth re-running periodically; it's the cheapest net for
the next rot.

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

**1. ✅ RESOLVED 2026-07-15 (commit 319948611) — the in-game Ambition inventory
menu opened on the TITLE SCREEN.** Both menu-open routers ran in `Update` with
`GameMode` defaulting to `Playing`, so with no session the toggle still fired.
Root cause was exactly as suspected: gated on the process being up, not on a live
session. Fixed by gating both `grid_menu_open_routing` and
`kaleidoscope_menu_open_routing` on `simulation_authorized.and(in_base_mode)` — a
live Ambition session (a `SessionRoot` the active scope names) AND the active room
carrying NO demo mode tag. Added `in_base_mode` as the reusable mirror of `in_mode`
(ambition_runtime, re-exported `ambition::runtime`) so any host-only chrome can gate
the same way. So the toggle is now impossible on the title screen AND inside a
hosted demo session. Regression-tested.

**2. ✅ RESOLVED 2026-07-15 (commit bd3c41e9a, logged then fixed the next commit) —
attack fires the wrong direction after turning.** Move LEFT, press attack → the
attack came out RIGHT / read as a back-attack. Root cause: `attack_dir_from_axis`
classified on the RAW aim axis (`+x` = screen-right) but `AttackDir` is FACING-
relative (`+x` = facing), so a FORWARD press while facing left (`axis.x < 0`,
`facing = -1`) misread as `Back` and fired the aerial back-attack the wrong way.
Fix folded facing into the horizontal arm (`forward = axis.x * facing`, the same
transform `resolve_attack_intent_from_view`/`compute_aim` already apply) and
threaded `facing` through both callers (player moveset trigger reads `kin.facing`;
possessed-boss path reads `boss.kin.facing`). The swing SIDE is the latched
`MovePlayback.facing`, which is `kin.facing` integrated in `WorldPrep` *before* the
`Combat`-phase trigger, so it is already the live facing — single source of truth
confirmed. Regression test added (`moveset/tests.rs`). Entry was never marked
resolved when logged.

**3. ⏳ PARTIAL 2026-07-15 (commit fcef3a2b3) — vanity / title presentation
timing.** DONE: the "Powered by Ambition" card now eases in / holds / eases out
(`fade_basic_sequence_card` ramps the card CONTENT alpha from the sequence
runtime's elapsed time, backdrop stays opaque black) and holds 3.6s instead of
2s. `card_alpha` curve unit-tested. BLIND: the 0.55s fade + 3.6s hold are tuning
Fable should confirm at the controls. STILL OPEN (deferred to the Fable shell-UX
pass — feel + cross-crate audio): (a) the TITLE MENU still snaps in rather than
fading — a fade there means alpha-ramping the externally-rendered `ambition_menu`
launcher tree (its render is rebuild-on-change, no per-frame alpha), a bigger
change than the card; (b) the menu SOUNDTRACK starts on the vanity card (the
first Frontend audio context, via `select_shell_audio_context` →
`apply_frontend_music_policy`), and tying it to the title screen specifically is
cross-crate audio plumbing (suppress Frontend music for the startup route, or
give the card a music-less audio profile). Both are squarely Fable's feel call.

**4. ✅ RESOLVED 2026-07-15 (commit 3036cdfa2) — "Quit to Title" added to the
Ambition pause menu.** A `QuitToHome` entry on the kaleidoscope System page fires
`ShellCommand::QuitToHome` — the exact leak-free session-retire path F10/Start
already used — placed just above Quit to Desktop. The `SystemMenuAction` enum
stays a pure marker (the ShellCommand write is in the app dispatcher, not the
settings-menu engine crate). No-drift guards + an ordering test updated.

**5. ✅ RESOLVED 2026-07-15 (commit a50515594) — universal host pause menu.**
`ShellPauseMenuPlugin` (ambition_game_shell, on `MinimalShellPlugins`) gives EVERY
hosted experience + standalone demo app one menu — Resume / Quit to Title / Quit
to Desktop — opened on Escape/Start, drawn with the same `ambition_menu` renderer
the launcher uses, dispatching the same host-relative `ShellCommand`s
(`QuitToHome` / `ExitProcess`). Jon's "elegant … one primitive the host offers,
not a bespoke menu per demo" — satisfied. Coexists with Ambition's kaleidoscope
via `ShellPauseMenuSuppressed`, which the host sets from `in_base_mode`: the shell
menu runs for exactly the sessions the kaleidoscope does NOT (the two partition
every live session). Input: added a `pause` edge (Esc/Start), narrowed the blunt
`quit_to_home` to F10-only so Start opens a real menu instead of blind-retiring.
Sim-pause via `GameMode::Paused` is best-effort (Option). Poison-tested.

**6. ✅ RESOLVED 2026-07-15 — `smb1` renamed to `mary_o` across the code.** The
crates (`ambition_demo_smb1{,_app}` → `ambition_demo_mary_o{,_app}`), the `Smb1*`
types (→ `MaryO*`), the `SMB1_MODE`/`SMB1_CATALOG_RON` consts (→ `MARY_O_*`), and
the `smb1_*` fns (→ `mary_o_*`) all carry the game's own name now; the workspace
policy configs and `run_game.sh` aliases followed. The enemy was also de-branded
in the same pass: `goomba` → `crony` (file, `CRONY_*` consts, `mary_o_crony`
brain key + catalog row, display name "Mary-O Crony"); it keeps its goomba-like
stomp-walker actions and the `ai_slop` sprite. Only bare-prose "SMB1"/"goomba"
references to the inspiration remain (combat `prefabs.rs`, planning docs), which
is deliberate. See [[feedback_entity_id_matches_label]].

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

**8. ⏳ PARTIAL 2026-07-15 (commit 18ce82e35) — on-screen control hints follow the
controlled subject now.** DONE (the relativity core): `compute_player_affordances`
and `compute_controlled_actor_intent` read the primary/home avatar
(`With<PrimaryPlayer>`) — so while possessing another body the hints described the
vacated avatar. Both now resolve `ControlledSubject` (fallback primary) exactly
like `update_nearest_interactable` already did, and read THAT body's clusters, so
every renderer (the touch overlay today, a desktop row tomorrow) is
relativity-correct for free — the fix is upstream at the affordance source.
Poison-tested (possess a ledge-hanging actor → hints read Climb/Roll; drop
possession → back to Jump/Shield). REVIEW FIX 2026-07-15 (commit 8e492f1a3): the
INTENT half of 18ce82e35 read the per-body `PlayerInputFrame` MIRROR, which only
the home avatar carries — during real possession the query missed and
`PlayerIntent` froze at the pre-possession aim (the original test masked it by
hand-spawning the mirror on the actor). Now resolves the driven body's input via
its own `Brain::Player(slot)` → `SlotControls`, the same read the universal brain
tick uses; possession-faithful poison test added. STILL OPEN: (a) the hint VOCABULARY is still
the fixed six-variant enum (the Ambition protagonist's verbs) rather than sourced
from the driven body's own `ActionSet`/`ActorMoveset`; (b) "whatever UI is active
drives the row" (menu context) — no desktop hint row even exists yet (touch-only),
so that half is moot until one lands. Both are follow-ups above this source fix.

**9. ✅ RESOLVED 2026-07-15 — `WorldItem` draws a real sprite via a game-owned art
seam.** Added the generalizing seam the follow-up asked for: `WorldItem` +
`WorldItemFact` carry an optional presentation `sprite` id (separate from the
equipment row id — art id ≠ equipment id), and `sync_world_item_visuals` resolves
it through a render-owned `WorldItemArt` map the GAME fills, so asset knowledge
stays out of the reusable renderer. Mary-O tags its milk `WorldItem`
(`MILK_SPRITE`) and the app binds `sprites/props/super_mary_o_milk_carton.png` into
`WorldItemArt`; `regen_sprites.sh` publishes that flat canonical (tackon target +
held_prop_map). Chose the FLAT prop image over the animated prop-SHEET path
(`prop_asset_for_kind`): the sheet path needs a per-app `SandboxAssetCatalog` prop
registration (a `CharacterSpriteCatalogRow` + manifest), heavy for a static pickup
— the flat seam is small and self-contained, and the milk needs no idle bob. The
row-tinted quad remains the fallback until a fresh clone runs regen, so it degrades
draw-blind. NOTE (blind): shipped without a visual check (sprite gitignored/regen'd)
— the 24×28 display size is a `blind fix:` candidate if the carton reads off.
UPDATE 2026-07-15 (commit 202a4c0fb): the STANDALONE-only host gap below is now
CLOSED. The app-local `insert_resource(WorldItemArt)` was the wrong seam (bound milk
in the standalone app only, and would clobber under a multi-provider host).
Generalized to the catalog-fragment idiom: the game contributes pure DATA —
`ambition::platformer::world_item_art::{WorldItemArtEntry, WorldItemArtManifest,
register_world_item_art}` (in `ambition_platformer_primitives`, the crate render AND
the render-dep-free provider both reach) — and render resolves it
(`build_world_item_art` Startup → `WorldItemArt` handles). `MaryOExperiencePlugin`
registers the milk entry; because BOTH the standalone app and the host add that one
provider plugin, milk now draws in both, and the app-local binding is DELETED. The
manifest is a UNION (extend, never replace) → safe when a host composes providers.

**10. `WorldItem` has no locomotion (2026-07-15).** A resting collectible only —
a classic sliding mushroom wants a free-body integrator like `ground_item_physics`.
Deliberately NOT abstracted at N=2 (design-balance): unify a `settle_free_body`
helper across `GroundItem`/`WorldItem` when a THIRD moving pickup lands.

**11. ✅ RESOLVED-BY-DESIGN 2026-07-15 — crony squash despawns directly, and that
is correct.** `bounce_squash_cronies` zeroes health + despawns instead of routing
through the shared actor-death path (`HitEvent` → `apply_actor_hit` →
drops/score/debris). Investigated: that path is DEFERRED (consumed a stage later),
so a hit event emitted at the stomp would leave the crony alive-and-hostile when
`apply_actor_contact_damage` runs the same frame — it would hurt the stomper, the
exact bug the same-frame neutralize exists to avoid. A crony also has no score and
no drop table, so the shared path would carry nothing. The only thing a silent
despawn dropped was the visible pop, now emitted as a dust `VfxMessage::Burst`
through the engine's shared vfx seam. So: keep the direct neutralize; the death
pipeline's ordering is wrong for a contact stomp. (If a future crony ever scores or
drops, reconsider — but it would need a same-frame lethal application, not the
deferred event.)

**12. ✅ RESOLVED 2026-07-15 (commit 319948611) — reactive-block reuse proven;
brick-break is the second consumer.** `ContactSource::Block { GeoId }`'s head-bonk
now drives TWO opposite effects: the ?-block ADDS a milk pickup, a brick SUBTRACTS
itself. Mid-run removal reused the EXISTING immutable-base seam rather than a new
`World`-mutation path: `bricks::contribute_broken_bricks_to_overlay` extends
`FeatureEcsWorldOverlay::removed_block_names` (the same subtraction lock-walls use,
contributed AFTER `rebuild_feature_ecs_world_overlay` clears it), so collision drops
the broken brick with no base mutation. Completed that seam's MISSING render half —
`BlockVisual { block_name }` on every `spawn_block` + `sync_removed_block_visuals`
(ambition_render, in the reusable presentation) despawns a subtracted block's sprite —
so `removed_block_names` now means collision AND render. Mary-O side stays small:
`bricks.rs` (a `BrokenBricks` u32 bitset — NOT a HashSet, it's iterated each frame and
the det-contract bans hash iteration) + brick geometry helpers mirroring the power
blocks. The engine-for-other-games oracle's "second consumer" is satisfied.

**13. A riding surface-momentum body ignores solid blocks (2026-07-16).** While
`SurfaceMotion::Riding`, `advance_riding` walks pure chain arc — no sweep against
`World::blocks` — so a rider on the speedway's floor guide chain runs straight
through the solid `finish_tower` (hazards still fire via the kernel's hazard
gate; solids do not). Found during the speedway bug recon
(`game/ambition_demo_sanic/src/tests/speedway_oracles.rs` documents the related
bugs). Not player-visible today because the spikes reset first, but any chain
routed past a solid wall will exhibit it. Fix belongs with the surface-momentum
contact rework, not a point patch.

**14. Axis-swept walkers ignore chain terrain (2026-07-16).** Enemy walkers
(`Wanderer` and kin) integrate against `World::blocks` only; `SurfaceChain`
terrain is invisible to them. On the expanded Sanic speedway the rolling hills
are chain geometry over a flat solid, so a badnik placed on a hill would pace
the flat ground INSIDE the hill visual. Worked around by authoring
`EnemySpawn`s only on flat stretches (`tools/author_speedway_ldtk.py` notes
it). The real fix is a chain-aware ground probe for axis bodies — the inverse
of smell #13's gap.

**15. IntGrid lowering erases authored block names (2026-07-16).**
`area create` lowers `Solid`/`OneWayPlatform`/`BlinkWall` entities into
IntGrid paint, and the rect-merge reconstruction names blocks "ldtk solid"/
"ldtk one-way" — the authored `name` is gone. Anything that identifies blocks
by name (Sanic's monitors, Mary-O-style reactive blocks, the demo's tiled
ground re-id) must either stay an ENTITY instance (`entity add` post-pass, as
the monitors do) or re-derive identity positionally (as the speedway ground
re-id does). If named gameplay blocks become common, the lowering should
carry names into per-cell metadata or skip named entities.

**Addendum to #13 (2026-07-16):** the Sanic monitor boxes make the riding
pass-through player-visible — an un-rolled runner passes through a monitor's
26px box (its break rule fires only on stomp/roll overlap, so gameplay is
coherent, but the box reads as intangible at walking speed). Same fix locus:
the riding-side contact rework.

**16. Live param modifiers don't survive a worn-identity refresh (2026-07-16).**
A `WornCharacter` re-wear replaces a momentum body's `MomentumParams` wholesale
from the catalog row (state-preserving, params-refreshing —
`sync_worn_motion_model_preserving_state`). Any live modifier that multiplied
the params and saved a baseline to restore (Sanic's `SpeedShoes`) is silently
clobbered by the refresh, and its later "restore" writes a stale baseline over
the new identity's authored params. The Sanic super paths are guarded (the
super monitor removes live shoes; the speed monitor no-ops while super), but
the dev D-toggle mid-shoes corner remains. Real fix: timed modifiers should
re-derive the restore target from the worn identity's catalog row at expiry —
never from a captured baseline — or ride a param-modifier stack the identity
refresh re-applies.

**17. Generated secondary-world `.ldtk` defs drift from sandbox (2026-07-16).**
The dedicated-world generators (e.g. `generate_hall_of_characters`, cut-rope
arena) clone their `defs` from sandbox.ldtk ONCE, at first scaffold. On every
subsequent regen `_apply_to_dedicated_ldtk` only replaces the *level* via
`area create --replace-existing`, never re-syncing defs. So when a new engine
entity type lands in sandbox (here `SurfaceChain`/`SurfaceLoop` from the Sanic
surface work), the hall's committed defs go stale and
`validate <hall> --secondary-world sandbox` fails with "defs.entities is
missing editor definitions for supported Ambition entities" — independent of
any level content change. Real fix: the dedicated-world apply step should
re-clone/merge entity defs from sandbox on every regen (or a shared
`sync-defs` pass over all secondary worlds), not just on initial scaffold.

**18. Non-character sprites default to `npc_` catalog ids (2026-07-16).**
The actor-contract emitter's `_character_id_for`
(`tools/ambition_sprite2d_renderer/.../authoring/actor_contract.py`) falls back
to `npc_{stem}` for ANY target that doesn't declare an explicit `character_id`,
regardless of whether the sprite is a character, prop, or projectile. So props
render with NPC-namespaced ids and read as characters to anything that scans by
id prefix (it misled the Hall-of-Characters authoring into cataloging Glider and
Shrine as NPCs). The render directory (`targets/props|projectiles|characters/`)
is NOT an authoritative role signal — `news_board` renders under `props/` yet is
authored + placed as `npc_news_board` (an interactable board), so a directory-based
default would wrongly rename it. Glider + Shrine are fixed by declaring explicit
`prop_*` ids in their targets; `lasersword`, `lasersword_with_guns`,
`creator_lab_props`, `intro_cart` still emit `npc_*` sidecars, and `news_board`
should be a `prop_*` but is a cataloged+placed NPC today, so reclassifying it is a
catalog+placement rename deferred as a smell. Real fix: give targets an explicit
`role`/`kind` (character vs prop vs fx) that drives the id namespace, rather than
defaulting everything to `npc_`.

**19. A lean-catalog game loads a prop sheet by BYPASSING the asset catalog (2026-07-17).**
The animated ring pickup needs its `sanic_ring_prop` sheet in
`GameAssets.characters.props`, but the Sanic app builds a lean
`SandboxAssetCatalog` (`build_sandbox_catalog_without_worlds`) that registers
character/entity/parallax sprites — NOT arbitrary props. Rather than teach the
catalog about the prop, the app calls the new
`character_sprites::load_prop_sheet_for_target` helper, which reads the
build-embedded manifest spec (`try_load_spec_for_target`) and `asset_server.load`s
`sprites/<target>_spritesheet.png` directly — sidestepping the catalog's
profile/quality-tier authority (the ring always loads at base resolution). Fine
for one demo prop, but it's an authority bypass: the elegant fix is a per-game
"prop-sheet contribution" seam so a game registers prop targets into its asset
catalog the way it already contributes character/audio catalog fragments, and
props then load through the ONE catalog path (tiered + profile-gated) like
characters. Worth building once a second demo wants animated props; until then the
direct loader is the honest smaller thing. (Companion authoring note: rings carry
their render identity via a new `PickupSpec.sprite` presentation field, assigned in
`sanic_speedway()` beside the badnik-name patch. That keeps a render-only string on
an authored placement spec the sim also reads — defensible as authored identity,
like `PropSpec.kind`, but if presentation keeps accreting on placement specs the
render-side answer is a pickup-art manifest, à la `WorldItemArtManifest`.)

---

## 2026-07-17 — `shell_host_rendered` audio tests fail under load — ✅ RESOLVED 2026-07-19

**RESOLUTION (2026-07-19). The diagnosis recorded below was WRONG in both of its
forms — read the correction before trusting anything in the original entry.**

It is not a parallelism race, not a test-ordering leak, and no audio device is
involved at all: these tests run `AudioOutputMode::Recording` and assert
`!device_backend_installed`, so there is no process-global device to race for.
There are no statics anywhere in `ambition_audio` / `ambition_game_shell`.

Real cause — **wall-clock sensitivity**. The tests stepped the App with plain
`app.update()` under Bevy's default `TimeUpdateStrategy::Automatic`, which
advances the clock by REAL elapsed time. So the number of `FixedUpdate` steps per
`update()` depends on how busy the machine is: on an idle machine these frames run
almost NO simulation, while under load a single frame runs many sim steps. The
launched Sanic session emits its own legitimate SFX as its gameplay advances, so
the "count did not move" assertion broke exactly when real sim time elapsed.

Proven, not theorised: injecting a 40ms sleep per frame reproduced the historical
failure **deterministically with `--test-threads=1`** and with the exact reported
+1 signature (13 vs 12). A parallelism race cannot reproduce single-threaded.
(Whichever member of the family lost that race varied run to run, which is what
made it *look* like a parallelism bug.)

Fix, in two parts:
1. `game/ambition_app/tests/shell_host_rendered.rs` builds its App through
   `rendered_app()`, which pins `TimeUpdateStrategy::ManualDuration(1/60)` — the
   idiom the sibling `shell_host_startup` module already used. `settle()` now
   means an exact number of sim frames on any machine.
2. The fragile assertion is gone. `assert_eq!(accepted_playbacks, before)` was
   both REDUNDANT and wrong-headed: `audio_play_sfx_messages` sends every message
   down exactly one branch, so asserting the exact increment of
   `rejected_wrong_owner` is already necessary *and* sufficient to prove the stale
   request never reached playback — without coupling the test to unrelated
   session-ambient audio. The misleading doc comment on
   `SfxPlaybackState::accepted_playbacks` that invited the original mistake is
   corrected in `crates/ambition_audio/src/render.rs`.

Verified: green 3/3 with all cores saturated by CPU hogs, the condition under
which it previously failed every time.

**Lesson (generalisable):** a test that steps a real Bevy App and asserts anything
count-like or state-like MUST pin the timestep. Under `Automatic`, `app.update()`
is not a unit of simulation — it is a unit of wall-clock, and on a fast machine it
can be very nearly zero simulation. See `dev/journals/lessons_learned.md`.

<details><summary>Original (incorrect) entry, kept for the record</summary>

### 2026-07-17 — `shell_host_rendered` real-audio tests are order-fragile (test isolation)

`provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work`
(`game/ambition_app/tests/shell_host_rendered.rs:549`) asserts the per-App
`SfxPlaybackState.accepted_playbacks` count is UNCHANGED after writing a
stale-owner `OwnedSfxMessage` and stepping two frames. The assertion is fragile:
the fresh session it launches emits its OWN (legitimate, current-owner) SFX during
those two frames, and whether that SFX is *accepted* depends on whether the
process-global real-audio device has been opened by an EARLIER test (a
`shell_host_lifecycle` test). So the count moves by +1 (baseline: 16 vs 15; HEAD:
20 vs 19 — same +1) and the test fails, but ONLY in the full app_it suite —
it passes run alone and with its own module's siblings.

CONFIRMED PRE-EXISTING: it fails identically on baseline `1d6872c25` (before the
character-actions completion work), so it is a latent test-isolation bug, not a
gameplay regression. It surfaced now only because the full `app_it` suite was run
end-to-end (the prior "all landed" claim apparently never did — 8 app_it tests
were red on that baseline; the character-actions work reduced that to this 1).

**2026-07-19 update — confirmed a PARALLELISM race, not a stale expectation.**
Across four full-suite runs a *different* member of the family failed each time
(`provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work`, then
`provider_relative_music_drives_the_base_channel`), each passes in isolation,
and `--test-threads=1` runs all three green. So the leak is between tests racing
for the process-global audio device, exactly as theorised below. Cheapest real
fix is therefore (b): make the family independent of global device state, or
mark it `#[serial]`. It is the ONLY red in the suite and is unrelated to
whatever change is in flight — do not chase it as a regression.

Elegant fix (deferred — audio-test-infra, not the character-actions feature):
either (a) capture `accepted_before` AFTER fully settling the launched session so
its own SFX are already counted, and assert the DELTA excludes session-ambient
audio; or (b) make the `shell_host_rendered` real-audio tests independent of the
process-global device state (a per-test audio sink, or `#[serial]` + explicit
device reset) so opening the device in one test can't leak into another's
playback-accept path. Logged rather than fixed: it is orthogonal to the
character-actions gates and touching the real-audio test harness is not zero-risk.

</details>

## 2026-07-19 Title-screen menu has no mouse/touch selection — ✅ RESOLVED 2026-07-19 (VC6 landed)
- **RESOLUTION (deep-review triage, same day):** `publish_bevy_ui_menu_actions` (`crates/ambition_menu/src/render/bevy_ui/mod.rs:544`) now reads `Query<(&Interaction, &AmbitionMenuControl<Action>), With<Button>>` and emits the neutral `MenuActionActivated<Action>` on `Interaction::Pressed` (+ `publish_bevy_ui_menu_tabs`) — exactly the single generic pointer bridge proposed; fixes launcher/pause/kaleidoscope at once.
- **Where:** `crates/ambition_menu/src/render/bevy_ui/mod.rs:537-539` (observers),
  `spawn.rs:156` (rows), `crates/ambition_game_shell/src/basic_presentation.rs:517`
- **Smell:** Menu rows spawn with `Button` and carry `AmbitionMenuControl { kind,
  action, focus }`, so they are pickable and self-identifying — but NOTHING reads
  `Interaction`. The only pointer observers in the whole menu stack are the three
  scrollbar ones. So scrollbar drag works with mouse/touch; *choosing a row does
  not*, and the shell launcher's own input is keyboard/gamepad-only. AGENTS.md
  commits to preserving the Android/mobile/touch path and the title screen is the
  first thing a touch user hits — so on mobile the game cannot be started at all.
  Low urgency on desktop, load-bearing for a supported platform.
- **Noticed while:** planning the animated vanity-card startup sequence (Jon flagged it)
- **Suggested fix / size:** S. One `Pointer<Click>` observer reading `.focus` to move
  the cursor and `.action` to activate, emitting the SAME neutral commands keyboard
  nav emits — explicitly not a parallel mouse-driven selection path. Fixes the
  launcher, the shell pause menu, AND the kaleidoscope menu at once, since all three
  render through `spawn_bevy_ui_menu_with_assets`. Task card VC6 in
  `docs/planning/engine/shell-vanity-sequence.md`.

## 2026-07-19 Shell sequence card rebuilds its whole UI tree every animation frame — ✅ RESOLVED 2026-07-19 (VC2/VC5 landed)
- **RESOLUTION (deep-review triage, same day):** `sequence_frame` (`crates/ambition_game_shell/src/basic_presentation.rs:586-589`) now keys the ImageSequence on `segment.id` + `frames.len()`, deliberately NOT the frame index, with an explicit comment saying so — the root spawns once and frames mutate in place.
- **Where:** `crates/ambition_game_shell/src/basic_presentation.rs:404` (`shell_frame_key`),
  `:155-165` (`render_basic_shell`)
- **Smell:** `shell_frame_key` folds the ImageSequence `frame_index` into the key, and
  `render_basic_shell` despawns + respawns the entire node tree on key change. An
  animated card would tear down and rebuild the full UI ~50 times over 4 seconds.
  Latent today only because nothing constructs an `ImageSequence` yet.
- **Noticed while:** planning the animated vanity-card startup sequence
- **Suggested fix / size:** S/M. Key on segment identity so the root spawns once, and
  mutate `ImageNode.image` per frame in `fade_basic_sequence_card`, which already runs
  over exactly those entities. The resulting stable-root + per-frame-alpha machinery is
  also what the deferred title-menu fade-in (§3a, 2026-07-15) was blocked on. Task
  cards VC2/VC5 in `docs/planning/engine/shell-vanity-sequence.md`.

## 2026-07-19 IPFS sidecars are disconnected from the asset manager, with no fetch tool
- **Where:** `assets/{backgrounds,icons,concept_art,vanity_card}.ipfs`,
  `crates/ambition_actors/assets/fonts/bundled.ipfs`,
  `tools/LDtk-1.5.3-installer.AppImage.ipfs`; `crates/ambition_asset_manager/`
- **Smell:** Nothing in the Rust code ever reads a `.ipfs` file, and no script in
  `scripts/` or `tools/` hydrates one — re-hydration is a manual `ipfs get <cid>` into
  the sidecar's `rel_path`. So six git-ignored payload directories have no in-repo
  command to restore them, which collides with the regen-on-fresh-clone invariant.
  Separately, the sidecar records ONE directory CID with no path→CID table while
  `AssetManifest`/`AssetEntry` wants a CID per entry, so the two formats do not line
  up. `icons.ipfs` also uses an older `schema_version: 1` key layout the others omit.
- **Noticed while:** planning the animated vanity-card startup sequence
- **Suggested fix / size:** S for a hydration script (`scripts/fetch_ipfs_assets.py`,
  reads any sidecar, parses defensively across both key layouts — closes all six at
  once). L and deferred for bridging the directory-CID format to the per-entry Rust
  manifest. **Backlog only — Jon owns asset distribution and has said agents should
  not need to care about IPFS; do not fold this into feature work.**

## 2026-07-19 Room reset revives every actor without consulting its respawn policy — ✅ RESOLVED 2026-07-19
- **RESOLUTION (same day, track #9):** `reset_to_spawn` now consults `RespawnPolicy` before restoring health. A room reset is a room-scoped return, so it revives a corpse only under `OnRoomReenter` (or `InPlace`, which revives on its own timer anyway); `DeadStaysDead`/`OnRest` corpses keep only their spatial baseline. Living actors still reset to full under every policy. Pinned by `integration/respawn_policy_tests.rs` (4 tests), which assert the value IMMEDIATELY after the reset — the only place the old behavior was observable, since save-sync re-zeroed the HP later in the same frame. Poison-verified: forcing `stays_dead = false` fails 2 of them.
- **(historical)**
- **Where:** `crates/ambition_actors/src/features/ecs/reset.rs:119` →
  `reset_to_spawn` (`crates/ambition_actors/src/features/enemies/integration.rs:432`)
- **Smell:** `reset_to_spawn` full-heals EVERY actor in the query with no
  `RespawnPolicy` consultation. For a `DeadStaysDead` / `OnRest` actor that is
  currently dead, this revives it; `sync_ecs_actors_with_save` (Progression, every
  tick) re-zeroes its HP, so the net state is correct but the actor is alive for
  the remainder of that frame — it can be drawn and can act. Reset is the one
  liveness writer that does not go through the policy the kill hook and save-sync
  both read, so "who decides a dead actor comes back" has two answers instead of
  one. Triggers: `game/ambition_app/src/app/player_tick.rs:99`
  (`RoomResetReason::PlayerDeath`), `game/ambition_app/src/app/sim_systems.rs:116,171,195`.
- **Noticed while:** fixing "killed NPCs respawn immediately" (the ADR 0022
  placement pin lost across archetype projection). That defect is fixed; this one
  is adjacent and was found by the same trace, but is a separate rule.
- **Suggested fix / size:** S. Make the revive conditional — reset a currently
  DEAD actor only when its policy permits room-scoped return (`OnRoomReenter`),
  leaving `DeadStaysDead` / `OnRest` corpses alone; alive actors keep resetting to
  full as today. Wants a test that a killed `DeadStaysDead` NPC is never briefly
  alive across a `ResetRoomFeaturesEvent`, since the current wobble is invisible to
  end-of-frame assertions.

## 2026-07-19 `ambition_actors` compat-facade debris — ✅ PARTIALLY RESOLVED 2026-07-19
- **RESOLUTION (track #7):** deleted the two genuinely dead ones — `src/effects/mod.rs` (a `pub use ambition_vfx::*` facade that was not even declared in `lib.rs`, so it never compiled) and `src/debug_label.rs` (declared, zero consumers workspace-wide). Also fixed the doubled `#[cfg(test)]` on `character_roster`. **Correction to the original entry:** `src/host/` is NOT dead — `crate::host::windowing` is consumed by actors' own settings model; leave it. The ~73 `pub use ambition_*` re-export lines remain (each needs its consumers repointed at canonical homes first).

## 2026-07-19 No test drives a kill through the damage path to the death flag
- **Where:** `crates/ambition_actors/src/features/ecs/save_sync/actor_liveness_tests.rs:60`
- **Smell:** `a_killed_unprovoked_npc_stays_dead_on_load` hand-sets
  `enemy_<id>_dead` and asserts only that `sync_ecs_actors_with_save` zeroes HP. The
  entire WRITE side — kill hook → `RespawnPolicy` match → `SetFlagRequested` →
  `apply_flag_effects` → save — is unguarded, so the test stayed green through a
  regression where provocation replaced the policy and no flag was ever written.
  A test that presets the state it is meant to prove gets earned proves nothing.
- **Noticed while:** the same fix. A unit guard now pins the projection
  (`provocation_borrows_combat_numbers_but_never_the_placement_respawn_policy`), but
  the end-to-end kill→flag→re-stage path still has no coverage.
- **Suggested fix / size:** M. One integration test: spawn an NPC through the real
  lowering path, provoke + kill it via the damage systems, assert the save flag
  exists, then re-run room construction and assert it comes back dead.

## 2026-07-20 Sprite pipeline cannot tell "blank sheet" from "small character"
- **Where:** `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/authoring/sheet_build.py`
  (`alpha_bbox_metrics` ~288, `auto_crop` union at the `build_sheet` call site ~746)
  and `authoring/rigdoc.py` (`sprite_image` ~456, `paint_part` ~591).
- **Smell:** A rig whose SVG cannot be found paints nothing, and every stage
  downstream treats a fully transparent sheet as valid data: parts skip
  silently, the crop union stays empty, and `alpha_bbox_metrics` emits the
  degenerate `body_pixel_bbox (0,0,0,0)`. `neil_ongras_turfson` shipped a
  completely blank spritesheet AND portrait sheet this way, with a zero-size
  collision and hurtbox, and no test or tool flagged it.
- **Noticed while:** wiring the 2026-07-20 sprite generators into the Hall of
  Characters — Neil was about to be placed on a pedestal as an invisible NPC.
  Root cause (a hardcoded absolute `svg_source.path`) is fixed in the renderer
  submodule; the silent-failure chain is not.
- **Suggested fix / size:** S. Raise in `build_sheet` when `auto_crop` computes
  an empty union (zero opaque pixels is never intentional), and/or raise in
  `rigdoc.sprite_image` when `svg_source.path` is set but the file is missing.
  Optionally a conformance test asserting no published `*_spritesheet.ron` has
  `body_pixel_bbox.w == 0` for a character target.

## 2026-07-21 `morph_ball` owns a whole render module for one procedural circle
- **Where:** `crates/ambition_render/src/rendering/morph_ball.rs` (~240 lines),
  declared `pub mod morph_ball` in `rendering/mod.rs` alongside genuine layers
  like `actors`, `world`, `parallax`, and `camera`.
- **Smell:** A named body-mode's bespoke visual sits at the same structural
  altitude as the renderer's real subsystems. The module is a startup texture
  generator, a lazy spawner, and a per-frame position/visibility mirror for ONE
  sibling sprite — and its own header admits it exists only because the shipped
  spritesheet has no `MorphBall` row. That makes it a content workaround wearing
  an engine module's clothes. It also names a specific mechanic inside the
  reusable presentation crate, which is the same altitude violation
  `bubble_shield` and `mark_beacon` repeat: another game adding a content crate
  inherits Ambition's morph ball whether it has one or not.
- **Noticed while:** routing every body-anchored visual through the new
  presented-pose seam (`ambition_sim_view::presented_pose`). Finding the
  consumers meant grepping `pose.pos` across four unrelated top-level render
  modules that all do the same thing — mirror one sprite onto the player body.
  Each is a separate place to forget the frame clock.
- **Suggested fix / size:** M. Either (a) retire it outright by emitting a real
  `MorphBall` row from the sprite generator, deleting the procedural circle and
  letting the ordinary animation path draw it, or (b) fold morph-ball,
  bubble-shield, and mark-beacon into ONE body-attachment seam that content
  registers a spec against (sprite source + offset + visibility predicate), so
  the reusable renderer names no game's mechanics and there is a single place
  attachments read the presented pose.
