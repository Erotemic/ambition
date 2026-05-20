Here’s the refactor backlog I’d give an autonomous coding agent, prioritized by maintainability/extensibility payoff. This is grounded in the latest uploaded snapshot. I could not run Rust validation here because `cargo` is unavailable in the environment, so these are static-analysis recommendations with local validation commands included.

## Status (2026-05-20 engine-cleanup pass — pre-release no-compat rule)

The user established a new rule: while nothing depends on this repo
externally and no release has shipped, single-commit cold rips beat
bridge / shim / parallel-path migrations
(`feedback_pre_release_no_compat`). This session applied that rule to
engine-side legacy systems blocking backend design.

Commits 2f65c36…0d0744d land. Sandbox lib: 555 tests pass. Engine
lib: 219 tests pass. Both crate builds clean. Engine surface
shrank by ~2,300 lines of dead scaffolding + IR.

**Engine-side rips:**

- ✅ **#9 RoomObject / RoomObjectKind retired** — engine no longer
  carries an authored-entity IR. The 11-variant `RoomObjectKind` enum
  + `RoomObject { id, name, aabb, kind }` wrapper + `World.objects`
  field gone. Sandbox-side `RoomSpec` carries per-family typed
  `Authored<T>` Vecs (hazards, pickups, chests, breakables,
  interactables, enemy_spawns, boss_spawns, debug_labels) — one
  spawn loop per family in `spawn_room_feature_entities` instead of
  a giant `match` on `RoomObjectKind`. Runtime constructors take
  `(id, name, aabb, payload, …)` directly. Commits 2f65c36, fb07e33.
- ✅ **engine `enemy.rs` deleted** — `Dummy`, `DummyKind`,
  `spawn_dummies` were a pre-`EnemyArchetype` first-pass enemy
  primitive with no external callers (397 lines). Plus
  `combat::slash_hitbox` / `player_slash_hitbox` legacy shortcuts.
  Commit 2007897.
- ✅ **`Player.dash_available` deleted** — derived bool labeled
  "Back-compat/debug convenience"; was just
  `dash_charges_available > 0` recomputed everywhere. Plus
  `world_with_moving_platform` singular shim. Commit 0779682.
- ✅ **seldom_state scaffolding retired** — `state_machines.rs`
  (287 lines) declared 25+ marker components for a state-machine
  migration that never happened. `AmbitionStateMachinePlugin`
  registered seldom_state but no entity spawned any `StateMachine`.
  Encounter-controller markers (`EncounterDormant` /
  `EncounterStarting` / `EncounterActive` / `EncounterCleared` /
  `EncounterFailed`) were written-only — every consumer queried
  `EncounterRegistry` directly. Deleted module + `seldom_state`
  Cargo dep + plugin registration + `EncounterController` entity +
  `sync_encounter_controller_states` system + reset's controller
  despawn. Commit e9fad85.
- ✅ **engine `music.rs` + `physics.rs` deleted** — both 50-260-line
  "vocabulary for future story crates" modules without a single
  game/runtime caller. `Motif` + `TANGENT_MOTIF` had two
  self-tests; `PhysicsBodySpec` / `PhysicsBodyRole` /
  `PhysicsMaterial` / `RagdollSpec` had vocabulary-only proptest
  + snapshot self-tests. Commit e59fc30.
- ✅ **`engine::DestinationLabel` deleted** — typed sandbox-side
  authored entity for loading-zone destination labels was wired
  through RoomSpec / RuntimeEntityEmission / render but no LDtk
  entity ever produced one. Commit 592066a.
- ✅ **engine `boss_patterns.rs` deleted (660 lines)** plus
  `BossEncounterState::current_pattern_schedule` /
  `evaluate_pattern` — the entire engine boss-attack-scheduling
  surface (`BossPatternSchedule` / `BossPatternStep` /
  `BossAttackKind` / `BossMovementKind` / `ArenaAnchor` /
  `BossBeatPhase` / `ActiveBossBeat` plus six hardcoded
  `gradient_sentinel_*` / `mockingbird_*` / `gnu_ton_*` constructor
  methods). Sandbox uses its own typed `BossPatternStep` enum in
  `content/features/bosses.rs`; engine module had zero external
  callers beyond its own self-tests. `BossEncounterPhase::pattern_phase`
  followed (1523c98 — was only called from the deleted
  `current_pattern_schedule`). Commit 9c9a2ee.
- ✅ **`host::platform::power` retired (148 lines)** —
  `PowerProfile { Performance, Balanced, BatterySaver }` resource +
  `WindowFocusState` resource + `track_window_focus` system +
  `should_pause_nonessential_work` run-condition. Plumbed
  end-to-end but write-only: no gameplay system gated work behind
  the decision. Commit 1ed2469.
- ✅ **Misc small scaffolds dropped** —
  `projectile::diagnostics::projectile_status_summary` (HUD hook
  for an overlay that never landed), `world::physics::
  PhysicsControlledPlayerPrototype` marker (future-experiment
  placeholder, never attached), `AMBITION_DIR` const ("for tooling"
  with no tooling caller), and `SimulationSetup.sandbox_data` field
  ("for symmetry with PresentationSetup"; destructured to `_` and
  comments admitted nothing read it). Commits 6027452, 1680fac,
  601154a.

**Sandbox-side rips:**

- ✅ **`MechanicsRegistry` retired (393 lines + 8 self-tests)** —
  scaffolded HUD catalog of "verbs the sandbox demos" that
  openly admitted nothing consumed it. Commit 5b3bf33.
- ✅ **`presentation/parallax` orphan retired (820 lines)** — a
  data-authored parallax prototype from the 2026-05-17 themed
  reorg, never wired into any `App::add_plugins` call. The real
  parallax lives in `presentation::rendering::parallax`. Commit
  deffe89.
- ✅ **`add_http_asset_source` no-op + Brain placeholder docs** —
  empty function reserved for "slice 9 (WebHttp packaging)" with
  no callers; `EnemyBrain`/`BossBrain` docs updated from
  "Placeholder enum before adopting a state-machine crate" to
  match what they actually are (typed authoring payloads). Commit
  0b4e9a9.
- ✅ **engine lib.rs doc updated, `.agent` indexes refreshed** —
  Commits a684989, 0d0744d.

**Why this matters for engine design:**

Engine now carries simulation primitives only:
movement / collision / AABB / abilities / combat / character AI /
projectile / quest / save / cutscene / ledge-grab / kinematic /
combat slots / interaction authoring payloads / debug labels.
No authored-entity IR (RoomObject is gone), no
state-machine scaffold (seldom_state retired), no orphan
vocabulary modules (music, physics, mechanics, parallax orphan,
boss_patterns), no dead Dummy/slash_hitbox shortcuts. Adding a
new authored entity type is "add a `Vec<Authored<T>>` field +
spawn loop on the sandbox side", not "edit a kind enum in the
engine and 22+ match arms."

## Status (2026-05-19 agent session — second pass)

Continuing from the prior session, commits 2aab57e…871cf95 land:

- ✅ **P17.4 per-player attack state** — `ActivePlayerAttack` component on the player entity replaces the global `CurrentPlayerAttack` resource. 14 sites migrated; two multiplayer smoke tests (`two_players_have_independent_active_attacks`, `clear_is_per_entity`) cover the per-entity invariant.
- ✅ **P17.9 per-player safety state** — `PlayerSafetyState { last_safe_pos }` component replaces `SandboxSimState::last_safe_player_pos`. 11 sites migrated across world_flow / phases / sim_systems / dev_runtime / runtime/reset / setup_systems / rl_sim / trace. `remember_safe_player_position` now takes `&mut PlayerSafetyState`; `two_players_have_independent_safety_anchors` smoke test added.
- ✅ **P2 partial: settings page-nav descriptor table** — the seven page-navigation `SettingsItem` variants (`OpenVideo`/`OpenAudio`/.../`Back`) used to carry 14 nearly-identical match arms in `apply_action` + `label_with_dev`; collapse them into a `PAGE_NAV_ROWS: &[PageNavRow]` table. Adding a new sub-page is now one row. The slider/cycle/toggle rows keep their per-variant logic for now.
- ✅ **P2 partial: sandbox_assets further split** — extracted `ids.rs` (60 lines, stable AssetId constructors) and `builders.rs` (302 lines, 9 per-domain `extend_with_*` manifest builders). `sandbox_assets/mod.rs` is now 541 lines (down from 1686 originally) and focused on catalog construction + the source plugin.
- ✅ **P17.6/P17.8 B-bucket iterate-all-players migration** — seven feature-runtime systems no longer `single()`-pull the primary player. Hazards / enemy_projectiles / breakables / pickups / chests / interact all iterate every player so a future co-op build hits whichever player actually overlaps. Bosses + enemy AI keep `PrimaryPlayerOnly` to document the deliberate targeting decision until per-target AI lands (#17.8 deep version). Touched commits: bd306f0 (enemy_projectile), c626d35 (hazards), f0a4e08 (bosses+actors targeting), a086f07 (breakables+pickups), 1ba01b0 (chests), 0a569dd (interact), 4ece6ad (encounter), c80283a (projectile spawn + charge visual → `PrimaryPlayerOnly`). Audit doc updated in 59f7fe0 + 668ee08 + a4b37ab + 100251e. All B-bucket sites now classified.
- ✅ **P7 partial: EnemyArchetype spec table + brain table** — the ten parallel per-variant `match self` blocks in `EnemyArchetype` (`max_health`, `patrol_speed`, `chase_speed`, `aggro_radius`, `attack_range`, `contact_strength`, `damage_amount`, `default_size`, `is_aerial`, `is_sandbag`, `rider_max_health`) collapse into one `EnemyArchetypeSpec` row driven by `archetype_spec(arch)`. Adding a new enemy is now one row in the spec table plus one row in the `BRAIN_NAME_TO_ARCHETYPE: &[(&str, EnemyArchetype)]` lookup. `choreography()` stays a method because its ranged variants hold non-Copy parameter bags. Commits 615b882, 9fb7e1e.
- ✅ **P17.10 multiplayer smoke tests** — extended the per-player smoke set with `primary_player_query_resolves_with_two_players_spawned` and `player_entity_query_iterates_all_spawned_players` so the architectural invariants the B-bucket migration depends on (exactly-one primary, generic queries see all spawned players) are pinned. Commit 181942e.
- ✅ **P12 cycle / toggle / nudge helpers** — `apply_action` had three repeated row shapes (prev/next cycle, boolean toggle, slider nudge); each spelled out 5-6 lines per row across ~25 sites. Hoisted into `apply_cycle`, `apply_toggle`, and `nudge_delta`. Net -40 lines in `apply_action`, single place to change semantics. Commit 89bd383.
- ✅ **Warning cleanup** — `cargo check --tests` was at 2 warnings; both leftover-import false positives from earlier per-player refactors. Commit c75cb2f.
- ✅ **`.agent` indexes refreshed** — Commit f688309.
- ✅ **P17.6 bridge: PlayerHealRequested.target** — added `target: Option<Entity>` field. Pickup collection routes the heal to the player who actually overlapped the heart via `PlayerHealRequested::for_target(amount, collector_entity)`. Cutscene/quest/inventory heals keep using `new(amount)` and fall through to primary. Fixes a latent bug where the B-bucket pickup migration could heal the primary when a non-primary collector picked it up. Two new smoke tests pin both routes. Commit 4681b51.
- ✅ **P17.6 bridge: PlayerDamageEvent.target** — same shape applied to damage events. Hazards and enemy projectiles stamp `target: Some(player_entity)` per overlapping player. Boss and enemy contact damage keep `target: None` because their producers already filter `PrimaryPlayerOnly`. Commit 1aca6d2.
- ✅ **P17.6 reader migration** — `apply_player_damage_system` now iterates `player_q` and filters events per resolved target (`event.target.or(primary)`). Each player gets `handle_player_damage_events` + safe-position update keyed on their own damaged-this-frame flag. With one player today, behavior is unchanged; with two players + a hazard event tagged `target: Some(p2)`, p2 takes the hit and p1 is untouched. Commit f157014.

Counts: `cargo test -p ambition_sandbox --lib` → 570 passed (566 pre-existing + 4 new multiplayer smoke). Warnings: 0.

## Status (2026-05-19 agent session)

Completed in the session that produced commits 6ef63ba…eb4a575
(60 patches; `cargo test -p ambition_sandbox --lib` → 563 passed,
`cargo test -p ambition_engine --lib` → 258 passed; sandbox lib
warnings: **102 → 0**):

- ✅ **P3 PrimaryPlayerOnly docs** — removed the non-existent `PrimaryPlayerOnlyMut` reference; clarified the filter type works for both immutable and mutable component access.
- ✅ **P2 FeatureBaseBundle split** — introduced `FeatureLifecycleBundle` (sim + RoomScopedEntity + id/name/aabb) and `FeatureRenderedBundle` (lifecycle + RoomVisual). `FeatureBaseBundle` is now a type alias for `FeatureRenderedBundle`, so existing PickupBundle/ChestBundle/EnemyActorBundle call sites compile unchanged.
- ✅ **P1 content/features/ecs/mod.rs split** — 2412 → 104 lines. Submodules: `actors`, `anim_helpers`, `banner`, `bosses`, `breakables`, `chests`, `damage` (pre-existing), `encounter_rewards`, `falling_chest`, `hazards`, `interact`, `overlay`, `pickups`, `reset`, `save_sync`, `spawn`, `tests`, `view_index`.
- ✅ **P3 map_menu repaint gating** — `sync_map_menu` now skips its despawn-and-rebuild pass when neither `MapMenuState` nor `RoomSet` changed this frame. Visibility / status-text branches stay per-frame (cheap and not fully covered by `is_changed`).
- ✅ **P17.1 player singleton audit refresh** — `docs/planning/player-singleton-audit.md` B-bucket table updated to point at the new ecs submodules (pickups/chests/breakables/hazards/bosses/actors/interact) instead of the old mod.rs line numbers.
- ✅ **Dead facade re-export cleanup** — `cargo check -p ambition_sandbox --lib` had 30 `unused-import` warnings at session start; now zero. Touched `body_mode`, `dialog`, `encounter`, `host::mobile_input`, `host::platform`, `map_menu`, `pause_menu`, `persistence::settings`, `presentation::character_sprites`, `presentation::parallax`, `presentation::rendering`, `projectile`, `ui_nav`. Test-only re-exports moved behind `#[cfg(test)] pub(crate) use` where needed.
- ✅ **Benchmark candidate (E0364)** — new `dev/benchmark-candidates/rust-pub-use-pub-crate-mismatch-2026-05-19.md` distils the visibility-mismatch trap encountered while splitting the ecs module.
- ✅ **P2 partial: sandbox_assets.rs (1686 lines) → folder** — converted to `sandbox_assets/mod.rs` (875 lines, catalog construction + plugin) + `sandbox_assets/tests.rs` (818 lines, 39 tests). All 22 `include_bytes!` paths re-rooted; the `no_unauthorized_bevy_asset_root_probes` guardrail allowlist updated for the new layout.
- ✅ **P1/P2 partial: boss profile data-drive** — `BossProfile::for_encounter_id_or_name` and `default_boss_profiles` now walk a single `AUTHORED_BOSS_PROFILES: &[(&str, fn() -> BossProfile)]` constructor table instead of carrying two separate id-string lists. Adding a new boss is now one row beside its constructor.
- ✅ **Dead code: clear_combat_slots_on_room_change** — removed an unused pub fn that was never registered; the actual room-reset clearer is inline in `features::ecs::reset`.

Other warnings investigated and left in place because they're feature-gated (e.g. `entity_sprite_embedded_core_url` under `static_core_assets`) or are documented future-slice stubs (`add_http_asset_source`).

- ✅ **Warning cleanup pass** — `cargo check -p ambition_sandbox --lib` went from 102 warnings at session start to 7 at session end (~93% reduction). Reductions came from removing genuinely-dead facade re-exports, removing the unused `clear_combat_slots_on_room_change` system, and targeted `#[cfg_attr(not(test/feature), allow(dead_code))]` on items the lib build can't see but tests/features do — plus module-wide `#![allow(dead_code)]` on the un-wired `dev::mechanics` HUD catalog, the `presentation::parallax` orphan stage, the Android-conditional `host::platform::android` module, the power-policy scaffold, the `ui_nav::list` reserved menu helpers, and ADR-0010 reserved time-control variants. Remaining warnings are pub struct fields (intentional API surface) that didn't justify per-field annotations.
- ⚠️ **P1 crate-root path migration** — the comment in `lib.rs` lines 58-65 confirms the internal `pub(crate) use` shims were removed on the 2026-05-19 shim-cleanup pass. The remaining `pub use content::features;`, `pub use dev::trace;`, `pub use runtime::game_mode;`, `pub use world::{ldtk_world, rooms};` are documented external API (used from bins/tests/engine docs). Deferred as low-ROI.

Backlog below is the original task list — items marked above are no longer current.


P2: RoomVisual split is structurally good, but FeatureBaseBundle still encodes the old dual role

The lifecycle split itself is good. But FeatureBaseBundle still includes RoomVisual, and its comment still describes the old “every authored feature both is simulation and renders” dual-role rationale, including a now-stale “planned split into RoomScopedEntity” note. EnemyActorBundle also says visual components can be omitted in headless builds, but its base: FeatureBaseBundle still carries RoomVisual.

This is not a current regression because RoomVisual requires RoomScopedEntity, so teardown still works. But it leaves the next headless/sim-only refactor with an awkward base bundle.

Suggested follow-up:

FeatureLifecycleBundle:
  FeatureSimEntity
  RoomScopedEntity
  FeatureId
  FeatureName
  FeatureAabb

FeatureRenderedBundle:
  FeatureLifecycleBundle
  RoomVisual

Then only currently-rendered entities opt into RoomVisual.

P3: PrimaryPlayerOnly helper docs overpromise slightly

player/queries.rs documents both PrimaryPlayerOnly and PrimaryPlayerOnlyMut, but only PrimaryPlayerOnly exists. That is small, but worth fixing because the audit task is explicitly about making singleton intent clear.

Either add the alias or adjust the docs:

pub type PrimaryPlayerOnly = (With<PlayerEntity>, With<PrimaryPlayer>);

There is no separate mutable filter type needed in Bevy; the same filter works for mutable queries.



### 1. Make `headless` / minimal feature builds real

**Priority: P0/P1**

`crates/ambition_sandbox/Cargo.toml` defines `headless = []`, and comments say it intentionally enables nothing, but the same file also says disabling runtime dependencies only works once subsystem code is gated end-to-end. That is a very high-value refactor because it gives you a fast simulation/test target and forces cleaner boundaries between game logic, Bevy rendering, audio, physics plugins, inspector UI, and platform integrations.

**Patch shape**

* Audit modules that implicitly depend on optional systems: audio, Kira, Avian, inspector, LDtk runtime, mobile, UI, presentation.
* Put subsystem code behind clear feature gates instead of letting `visible`/desktop assumptions leak everywhere.
* Add CI or local commands for:

  ```bash
  cargo check -p ambition_sandbox --no-default-features --features headless
  cargo check -p ambition_sandbox --no-default-features --features visible
  cargo check -p ambition_sandbox --no-default-features --features web
  ```
* Treat this as a boundary-cleanup refactor, not just cfg sprinkling.

**Why it matters:** this will reveal accidental coupling faster than almost any other refactor.

---

### 2. Finish breaking up `sandbox_update` into Bevy-native systems

**Priority: P1**

`crates/ambition_sandbox/src/app/update.rs` still has a large `sandbox_update` system that takes many resources and delegates to big phase functions. `crates/ambition_sandbox/src/app/feedback.rs` has `SandboxEventWriters`, `SandboxQueues`, and `ProgressionResources` wrappers partly to stay under Bevy’s system-param limit. That’s a smell: parameter-limit management is currently shaping architecture.

The schedule is already strong: `crates/ambition_sandbox/src/app/schedule.rs` defines ordered `SandboxSet`s such as `WorldPrep`, `PlayerInput`, `PlayerSimulation`, `RoomTransition`, `Combat`, `PresentationSync`, `FeatureCollection`, `FeatureInteraction`, `EncounterSimulation`, `GameplayEffects`, and `Progression`. Lean into that.

**Patch shape**

* Turn `player_control_phase`, `player_simulation_phase`, and room/combat/effect handling into smaller Bevy systems.
* Keep the existing schedule sets, but move each phase into module-local `Plugin`s.
* Replace broad queue bundles with typed messages/events where possible.
* Retain deterministic order by registering systems into the existing `SandboxSet`s.

**Good agent-sized first task:** extract only player input/control into one or two systems while preserving tests and behavior.

---

### 3. Split `content/features/ecs/mod.rs`

**Priority: P1**

`crates/ambition_sandbox/src/content/features/ecs/mod.rs` is about 2,400 lines and contains spawning, pickups, chests, breakables, hazards, bosses, actors, switches, save sync, view indexing, overlay rebuilds, helper queries, and tests. That file is doing too much.

**Patch shape**

Split into something like:

```text
content/features/ecs/
  mod.rs
  spawn.rs
  overlay.rs
  pickups.rs
  chests.rs
  breakables.rs
  hazards.rs
  actors.rs
  bosses.rs
  switches.rs
  save_sync.rs
  view_index.rs
  tests.rs
```

Keep public exports stable at first. Do not redesign behavior in the same patch. The first pass should be a pure movement/split refactor with `cargo fmt` and tests.

**Why it matters:** this is one of the best “unlock future agents” refactors. Smaller files reduce merge conflicts and make later behavior changes safer.

---

### 4. Remove crate-root compatibility re-export shims

**Priority: P1**

`crates/ambition_sandbox/src/lib.rs` still has compatibility shims like:

```rust
pub use content::features;
pub use dev::trace;
pub use runtime::game_mode;
pub use world::{ldtk_world, rooms};
```

and many `pub(crate) use ...` old-path shims. The comments say these exist so old `crate::X` call sites keep resolving.

**Patch shape**

* Run a mechanical migration from old root paths to canonical module paths.
* Remove the shims only after all imports are updated.
* Keep this separate from behavior changes.

**Why it matters:** these shims make the codebase look flatter and more ambiguous than it really is. Removing them will make module ownership clearer.

---

### 5. Split `RoomVisual` into lifecycle and presentation tags

**Priority: P1/P2**

`crates/ambition_sandbox/src/presentation/rendering/primitives.rs` explicitly says `RoomVisual` currently means both “rendered entity” and “despawn this on room transition.” `crates/ambition_sandbox/src/content/features/components.rs` also assumes authored features are both simulated and rendered. `app/world_flow.rs` despawns `RoomVisual` on room transitions.

That dual meaning will become increasingly painful for headless mode, persistent room state, lazy presentation sync, and non-rendered simulation entities.

**Patch shape**

* Introduce:

  ```rust
  RoomScopedEntity
  RoomVisual
  ```
* Use `RoomScopedEntity` for room-transition lifetime.
* Use `RoomVisual` only for entities that actually render.
* Update room despawn code to target `RoomScopedEntity`.
* Update feature bundles where necessary.

**Why it matters:** this is small but foundational. It separates simulation lifetime from presentation.

---

### 6. Move subsystem registration into real Bevy plugins

**Priority: P1/P2**

`crates/ambition_sandbox/src/app/plugins.rs` is around 1,200 lines. It already has helper registration functions, but it still centralizes a lot of ownership: simulation messages/resources, progression chains, encounter simulation, gameplay effects, pause/menu/input/mobile/audio, etc.

**Patch shape**

Introduce module-local plugins:

```rust
WorldRuntimePlugin
PlayerSimulationPlugin
FeatureEcsPlugin
ProgressionPlugin
EncounterSimulationPlugin
GameplayEffectsPlugin
PresentationSyncPlugin
DevToolsPlugin
MobileInputPlugin
SettingsPlugin
```

Then `app/plugins.rs` becomes an assembler, not the owner of every subsystem’s schedule details.

**Why it matters:** this is the Bevy-centric direction. Systems/resources/messages live near the domain that owns them.

---

### 7. Data-drive enemy archetypes and boss profiles

**Priority: P1/P2**

`crates/ambition_sandbox/src/content/features/enemies.rs` has a large `EnemyRuntime` struct and a hard-coded `EnemyArchetype` enum with many behavior methods. `BossRuntime` and `BossBehaviorProfile` in `crates/ambition_sandbox/src/content/features/bosses.rs` are similarly hard-coded, with profile construction based on specific boss IDs like `clockwork_warden`, `mockingbird`, and `gnu_ton`.

This is fine for early prototyping, but it will hurt content expansion.

**Patch shape**

For enemies:

* Create `EnemyArchetypeSpec` data:

  ```rust
  struct EnemyArchetypeSpec {
      max_health: i32,
      size: Vec2,
      patrol_speed: f32,
      gravity_scale: f32,
      choreography: Option<...>,
      projectile_spec: Option<...>,
      movement_kind: EnemyMovementKind,
  }
  ```
* Keep the enum temporarily as a key into the spec table.
* Move hard-coded methods into spec data.

For bosses:

* Separate authored profile data from runtime state.
* Avoid id-string conditionals for boss behavior.
* Convert attack/movement profile data into table-driven specs.

**Why it matters:** this converts “add a new enemy by editing Rust control flow” into “add a new spec.” It also makes AI, content validation, and tooling easier.

---

### 8. Make boss encounter state authoritative, not mirrored

**Priority: P2**

`crates/ambition_sandbox/src/boss_encounter/systems.rs` comments say `BossRuntime.health` is still the source of truth because existing combat/feature systems mutate it, while engine `BossEncounterState` is fed by deltas. The same system clones registry maps to work around borrow structure, and music/phase hooks are partly placeholder.

**Patch shape**

* Route boss damage through a typed message:

  ```rust
  BossDamageRequested { boss_id, amount, source }
  ```
* Let encounter state own health/progression/phase.
* Let actor runtime own transform, hit boxes, choreography, and presentation.
* Emit phase/music/cutscene requests from encounter transitions, not from ad-hoc runtime checks.

**Why it matters:** boss fights are likely to become a major extensibility point. Having two partially mirrored authorities will get expensive.

---

### 9. Retire legacy RON-shaped room/object paths

**Priority: P2**

The LDtk modules are moving toward a typed runtime spine, but there are still compatibility seams: RON-shaped room objects, legacy movement-platform defaults, legacy surface shorthands, and older consumers of `RoomObjectKind::KinematicPath`.

**Patch shape**

* Identify all consumers of `RoomObject` / `RoomObjectKind` that are only there for the old RON-shaped middle layer.
* Replace with typed LDtk projection structs/components.
* Keep compatibility at the import boundary only.
* Strengthen projection tests before deleting old paths.

**Why it matters:** level-authoring code should have one clear source of truth. Right now, old manifest shape and new LDtk runtime shape coexist.

---

### 10. Consolidate player and enemy projectile systems

**Priority: P2**

There are separate `projectile` and `enemy_projectile` modules. That may have made sense early, but projectile behavior usually wants one generalized system with ownership/faction/hit-filter data.

**Patch shape**

* Introduce:

  ```rust
  ProjectileOwner
  ProjectileFaction
  ProjectileHitPolicy
  ProjectileDamage
  ProjectileMotion
  ```
* Use one projectile update/collision path.
* Keep player/enemy-specific spawning helpers as thin wrappers.
* Preserve different visuals/audio via presentation specs.

**Why it matters:** projectile mechanics will keep expanding: bosses, enemies, traps, reflected shots, charged shots, friendly fire rules. A factioned projectile model will scale better.

---

### 11. Split and possibly generate the asset catalog

**Priority: P2**

`crates/ambition_sandbox/src/assets/sandbox_assets.rs` is around 1,700 lines and mixes embedded core asset registration, catalog construction, path aliases, web profiles, tests, and guardrails.

**Patch shape**

Split by domain:

```text
assets/
  sandbox_assets.rs
  catalog/
    mod.rs
    world.rs
    fonts.rs
    characters.rs
    audio.rs
    backgrounds.rs
    web.rs
```

Then consider moving static asset declarations to data files or a generated manifest so canonical paths, embedded URLs, and include paths cannot drift independently.

**Why it matters:** asset-path drift is exactly the kind of maintenance bug that shows up late and across platforms.

---

### 12. Replace giant settings matches with descriptors

**Priority: P2**

`crates/ambition_sandbox/src/persistence/settings/model.rs` has a large `SettingsItem` enum and big matches for rows, labels, actions, and dev info. It works, but it is brittle and repetitive.

**Patch shape**

Create descriptor/binding data:

```rust
struct SettingDescriptor {
    item: SettingsItem,
    page: SettingsPage,
    label: &'static str,
    dev_label: Option<&'static str>,
    action_kind: SettingActionKind,
    visibility: SettingVisibility,
}
```

Then keep custom logic only where necessary.

**Avoid overdoing it:** do not create a full generic UI framework. Just reduce the setting-specific duplication.

---

### 13. Make map UI persistent instead of repainting every frame

**Priority: P3**

The tech-debt log points out that map UI rebuilds room rectangles every frame in `map_menu/ui.rs::sync_map_menu`. That is acceptable now, but it will scale poorly and makes the code less Bevy-like.

**Patch shape**

* Spawn map room entities keyed by room id.
* Update only on changed map state, changed visited rooms, changed cursor/selection, or menu open/close.
* Use components for map tile/room presentation state.

**Why it matters:** this is a clean, bounded Bevy-idiom refactor.

---

### 14. Move narrative/cutscene/dialog content out of Rust constructors

**Priority: P3**

Files like `crates/ambition_sandbox/src/dialog/content.rs` and intro/cutscene modules are large content-heavy Rust files. That is okay for prototype speed, but not ideal long term.

**Patch shape**

* Define a small schema for dialog/cutscene content.
* Start with one low-risk content group.
* Keep runtime interpretation in Rust.
* Validate content in tests.

**Why it matters:** content iteration should not require recompiling large Rust files or touching control-flow code.

---

### 15. Reorganize very large test modules

**Priority: P3**

There are very large test files/modules: LDtk world tests, movement tests, asset catalog tests, projectile tests. These are valuable, but big monolithic test modules make it harder to find focused coverage.

**Patch shape**

* Keep true unit tests near code.
* Move integration-style and fixture-heavy tests into topic files:

  ```text
  tests/ldtk_projection.rs
  tests/movement_wall_cling.rs
  tests/asset_catalog.rs
  tests/projectile_behavior.rs
  ```
* Extract fixture builders.

**Why it matters:** better test navigation and smaller conflicts when agents add tests.


### 17. Player / enemy actor unification and multiplayer-readiness

Refactor player, enemy, boss, projectile, and hazard interactions around shared actor/combat primitives while removing accidental “exactly one player” assumptions. Preserve current single-player behavior, but make the code naturally support multiple player entities later.

#### 17.1 Audit and classify player singleton assumptions

* Search for `single()`, `single_mut()`, `get_single()`, `get_single_mut()`, `PrimaryPlayer`, `PlayerEntity`, `PlayerBody`, `PlayerHealth`, and `CurrentPlayerAttack`.
* Classify each singleton query as one of:

  * intentionally primary-player-only: camera, HUD, audio listener, dev tools;
  * should iterate all players: hazards, enemy attacks, pickups, interactions, respawn checks;
  * should target a specific player slot/entity: damage, healing, input, attack state.
* Add comments where singleton behavior is intentionally retained.

#### 17.2 Introduce shared actor facets

Add common components usable by players, enemies, bosses, and eventually NPCs/breakables:

```rust
ActorIdentity
ActorFactionComponent
ActorBody
ActorHurtbox
ActorCombatStatus
ActorTarget
```

Initially, these can be read-model/sync components rather than authoritative replacements.

#### 17.3 Attach actor facets to the player

* Give the player an `ActorIdentity`.
* Mark the player with `ActorFaction::Player`.
* Sync player position/body size/facing into a common `ActorBody`.
* Sync vulnerability/alive/invulnerability state into shared combat/hurtbox state.
* Do not rewrite player movement yet.

#### 17.4 Move current player attack state onto the player entity

Replace global or primary-player-only attack state with a per-player component:

```rust
#[derive(Component, Default)]
pub struct ActivePlayerAttack {
    pub state: Option<PlayerAttackState>,
}
```

* Update attack start/update/resolve systems to query the player entity.
* Keep primary-player rendering/debug views working by filtering `With<PrimaryPlayer>`.
* Add a smoke test proving two player entities can have independent attack state.

#### 17.5 Move gameplay input toward per-player components

Introduce:

```rust
#[derive(Component, Default, Clone, Copy)]
pub struct PlayerInputFrame {
    pub frame: ControlFrame,
}
```

* Populate this from the existing local input resource for the primary local player.
* Make player movement systems read input from the player entity.
* Keep menu/pause input global for now.
* Leave network/remote input for future work.

#### 17.6 Target player damage and healing explicitly

Replace untargeted player events like “damage the player” / “heal the player” with either targeted player messages or generic actor damage.

Preferred long-term shape:

```rust
pub struct DamageRequested {
    pub target: Entity,
    pub source: Option<Entity>,
    pub amount: i32,
    pub impact_pos: Vec2,
    pub knockback_dir: Option<f32>,
}
```

Short-term acceptable bridge:

```rust
pub enum PlayerTarget {
    Primary,
    Slot(PlayerSlot),
    Entity(Entity),
    AllLocal,
}
```

#### 17.7 Unify projectile hit detection around faction/hit policy

* Give projectiles owner/faction/hit-policy components.
* Replace separate player/enemy projectile collision paths where possible.
* Query common `ActorHurtbox` / `ActorFactionComponent`.
* Prevent friendly fire by policy, not by separate code paths.
* Keep player/enemy projectile spawn helpers as thin wrappers.

#### 17.8 Generalize enemy targeting

* Replace implicit “target the single player” logic with target selection over hostile actors.
* Start with nearest alive `ActorFaction::Player`.
* Store selected target in `ActorTarget`.
* Preserve current behavior when only one player exists.

#### 17.9 Make safety/respawn state per-player

Move primary-player-only safe position state into a component:

```rust
#[derive(Component)]
pub struct PlayerSafetyState {
    pub last_safe_pos: Vec2,
    pub respawn_anchor: Option<Vec2>,
}
```

* Update safe-position tracking to iterate players.
* Keep room transition cooldown/global sim state separate.
* Add a test with two players having independent safe positions.

#### 17.10 Add multiplayer-readiness smoke tests

Add tests that spawn two players and verify:

* both players can exist without singleton query failures;
* only one player is marked `PrimaryPlayer`;
* player attacks are independent;
* damage can target player 0 or player 1;
* enemy/projectile hit detection can choose the correct player by collision;
* camera/HUD systems still intentionally use the primary player.

#### 17.11 Remove compatibility shims after migration

* Remove obsolete primary-player-only resources once component equivalents are used.
* Remove temporary adapters only after all systems have migrated.
* Keep migration patches small and behavior-preserving.


#### TODO;

Remaining unfinished items from the OVERNIGHT-TODO backlog:

Big architectural lifts (P0–P2)

#1 — Headless feature builds real (P0/P1): cfg-gate audit + CI commands for --features headless/visible/web. Largest item; reveals coupling.
#2 — Break up sandbox_update: extract player_control / player_simulation / room / combat / effects into module-local plugins. P1.
#6 — Module-local Bevy plugins: plugins.rs is still a 1268-line owner of every subsystem schedule. P1/P2.
#8 — Boss encounter authoritative: route boss damage through BossDamageRequested instead of mirroring BossRuntime.health. P2.
#9 — Retire legacy RON-shaped room/object paths: 167 RoomObject sites; replace with typed LDtk projection. P2.
#10 — Unify projectile + enemy_projectile around faction/owner/hit-policy components. P2.
Bounded but not done (P2–P3)

#7 rest — boss profile data-drive done; per-boss pattern schedules in data still pending (TODO.md C-bucket has matching item).
#11 rest — sandbox_assets/ was split into mod + ids + builders + tests; further per-domain split (world/fonts/characters/audio/backgrounds/web) is open.
#13 — persistent map UI keyed by room id (only repaint-gating shipped; entity-keyed rewrite open). P3.
#14 — move dialog/cutscene content out of Rust (dialog/content.rs is 1016 lines). P3.
#15 — reorganize large test modules (world/ldtk_world/tests.rs is 1759 lines). P3.
Player/multiplayer unification (#17)

17.2 / 17.3 — shared ActorIdentity / ActorFactionComponent / ActorBody / ActorHurtbox / ActorCombatStatus / ActorTarget facets on player + enemies.
17.5 — per-player PlayerInputFrame component (today: global ControlFrame resource).
17.7 — projectile faction/owner/hit-policy (overlaps #10).
17.8 — generalize enemy/boss targeting (currently PrimaryPlayerOnly); pick "nearest hostile actor of faction Player".
17.11 — remove compatibility shims after migration (deferred until 17.5/17.7/17.8 land).
S-bucket bugs from TODO.md (worth flagging)

Morph-ball sprite lingers after exit.
Pickups "don't disappear when collected" (suspect stale — the view-index test asserts the contrary; needs in-game verification).
Wall-cling / lock-wall collision-correction debt.
Goblin encounter music transition still sounds like a section swap.
Cutscene/dialogue input + prompt-mismatch.
Menu mouse-hover vs keyboard-nav conflict.
Touch controls can affect player during cutscenes.
Suggested next pick (smallest payoff/size ratio): #13 persistent map UI or #15 test reorganization for pure bounded work, or #17.5 per-player input if you want to keep pushing the multiplayer chain — that's the gate for retiring the rest of the C-bucket single_mut sites in the audit doc
