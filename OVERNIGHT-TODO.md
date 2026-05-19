Here’s the refactor backlog I’d give an autonomous coding agent, prioritized by maintainability/extensibility payoff. This is grounded in the latest uploaded snapshot. I could not run Rust validation here because `cargo` is unavailable in the environment, so these are static-analysis recommendations with local validation commands included.

## Status (2026-05-19 agent session)

Completed in the session that produced commits 6ef63ba…149ef06
(22 patches; `cargo test -p ambition_sandbox --lib` → 563 passed,
`cargo test -p ambition_engine --lib` → 258 passed):

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
