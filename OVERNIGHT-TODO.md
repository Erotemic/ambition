# Ambition refactor backlog

Prioritized backlog for autonomous agents to work through. Items are
grouped by priority. Each item documents the patch shape and the
"why" so a future agent can pick one up without re-deriving context.

## Operating principle

Pre-release rule: while nothing external depends on this repo, single-commit
cold rips beat parallel-path / shim migrations. The test suite is the
verification gate. See `feedback_pre_release_no_compat` in agent memory.
Flip the rule once an external consumer ships.

## Status snapshot (2026-05-28)

- `cargo test -p ambition_actors --lib` → **1140 passing, 0 failing**
  (engine + sandbox merged; the former `ambition_engine` test suite
  now runs as part of the sandbox lib tests under
  `crates/ambition_actors/src/engine_core/`).
- `cargo run --bin rl_smoke` → 42/42 rooms ok (200 ticks each).
- `cargo check --workspace` → **zero warnings**.
- `ambition_engine` crate was deleted 2026-05-28 — see
  `dev/journals/engine-crate-collapse-2026-05-28.md` and
  `dev/journals/player-cluster-native-push-2026-05-28.md` for the
  full migration record.

### Earlier snapshot (2026-05-20)

- `cargo test -p ambition_actors --lib` → 577 passing
- `cargo test -p ambition_engine --lib` → 222 passing

Recently retired (autonomous-mission pass 2026-05-20, see git log
8b4cab1…HEAD's predecessor):

- `world/ldtk_world/tests.rs` (1735 lines) — split into per-topic
  submodules under `tests/{embedded_project, intgrid, kinematic_paths,
  metadata, surfaces}.rs` (#15)
- `assets/sandbox_assets/tests.rs` (818 lines) — split into
  `tests/{identity, profiles, static_probes, embedded_core}.rs` (#15)
- `assets/sandbox_assets/mod.rs` shed ~250 lines: `embed_core_assets!`
  macro + `AmbitionAssetSourcePlugin` moved to `sandbox_assets/embedded.rs`
  (#11)
- `persistence/settings/model.rs` shed ~90 lines via `format_shader_percent`
  / `format_audio_percent` / `format_toggle` label helpers (#12)
- `map_menu/ui.rs` — room boxes are now persistent entities keyed by
  `(MapRoomBoxKind, room_id)`; the per-frame despawn / respawn churn
  is gone (#13)
- `PlayerInputFrame` component + `sync_local_player_input_frame`
  producer; `update_projectiles`, `sandbox_update`, `attack_advance_system`,
  `record_frame_system`, and `update_body_mode` migrated off
  `Res<ControlFrame>` (#17.5 across two sessions). Systems running
  in `SandboxSet::Progression` or later can read `PlayerInputFrame`
  safely; systems running mid-`SandboxSet::PlayerInput` (e.g.
  `interaction_input_system`) MUST continue to read
  `Res<ControlFrame>` because `sync_local_player_input_frame` runs
  at the end of the input chain and mid-chain reads would deliver
  the previous frame's snapshot.
- `ae::ProjectileFaction { Player, Enemy }` engine tag on
  `ProjectileBody`, with `from_spec_with_faction` constructor; enemy
  projectile spawner now tags `Enemy` (#10 / #17.7 enabler)
- `app/plugins.rs` shed ~340 lines via thirteen domain-local Plugin
  extractions (#6 — 1265 → 926 lines):
  - `SandboxSimulationResourcesPlugin` in `app/sim_resources.rs`
  - `TraceSchedulePlugin` in `dev/trace/plugin.rs`
  - `LdtkRuntimeSpinePlugin` in `world/ldtk_world/bevy_runtime/plugin.rs`
  - `SandboxResetSchedulePlugin` in `runtime/reset.rs`
  - `CutsceneSchedulePlugin` in `presentation/cutscene.rs`
  - `GameplayEffectsSchedulePlugin` in `content/features/bus.rs`
  - `WorldPrepSchedulePlugin` in `content/features.rs`
  - `FeatureCollectionSchedulePlugin` in `content/features.rs`
  - `FeatureInteractionSchedulePlugin` in `content/features.rs`
  - `FeatureViewSyncSchedulePlugin` in `content/features.rs`
  - `SandboxAudioPlugin` in `audio/plugin.rs`
  - `PersistenceSchedulePlugin` in `persistence.rs`
  - `EncounterSimulationSchedulePlugin` in `encounter.rs`

  Remaining `register_*` / `install_*` helpers in `plugins.rs` are the
  app-local chains (`PlayerInput`, `PlayerSimulation`, `RoomTransition`,
  `Combat`, `PresentationSync`, `ProgressionChain`, `ProgressionPopulate`)
  plus the presentation install fns (`install_menu_setup_and_hotkeys`,
  `install_visual_animation_systems`, `install_misc_visual_sync_systems`,
  `install_player_visual_systems`, `install_projectile_and_vfx_systems`,
  `install_fx_and_hud_systems`, `install_camera_and_debug_overlay_systems`,
  `install_presentation_resources_and_subplugins`).
- bevy render features (`2d_bevy_render` / `ui_bevy_render` / `scene` /
  `png`) moved off the base bevy dep onto the `visible` cargo
  feature, so headless / future-non-render builds at least stop
  compiling the renderer transitively (#1, first slice — winit still
  comes in via `default_app`; documented as the next step)
- Boss-encounter authoritative state inversion (#8): engine
  `BossEncounterState` is now the source of truth for boss HP;
  `BossRuntime.health` is a one-way mirror. `record_boss_damage`
  returns a `BossDamageOutcome` so the ECS damage system can drive
  death VFX / banner on the same tick the kill lands. Invulnerable
  beats now correctly suppress hit feedback instead of papering over
  it one frame later.
- `projectile/tests.rs` (684 lines) — split into
  `tests/{mod.rs, charging.rs, collision.rs}` (#15 — third big-file
  split).
- `movement/tests.rs` (1669 lines) — split into
  `tests/{mod.rs, ability_gates, blink, climbing, clock, combat_actions,
  glide_and_air, ledge_grab, wall_collision}.rs` (#15 — fourth big-file
  split, 44 tests across 8 topic files, biggest is wall_collision at
  474 lines). The single biggest engine test file is now retired.
- `ActorFaction { Player, Enemy, Npc, Boss, Neutral }` component
  (#17.2/17.3 first slice). Distinct from `ActorDisposition`;
  attached to every actor entity (player, encounter mob, NPC,
  boss). Future projectile-faction merge / multiplayer targeting
  will dispatch on this tag instead of pattern-matching on
  per-family components.
- `dev/journals/bevy-headless-feature-graph-2026-05-20.md` + matching
  `dev/benchmark-candidates/bevy-feature-graph-headless-2026-05-20.md`
  document why a true headless build needs more than the Cargo.toml
  feature split (ui_api → default_app → bevy_window → bevy_winit
  transitive closure).
- `boss_encounter::damage::record_boss_damage` unit tests pin the four
  outcome paths: unknown runtime id → `None`; normal damage → applied
  + new hp; lethal damage → killed flag; invulnerable phase → applied
  false + hp unchanged. 566 sandbox lib tests now pass (was 553 at
  session start; the +13 are ActorFaction predicates, ProjectileFaction
  propagation, BossDamageOutcome semantics, and EnemyProjectileState
  spawn invariants).
- `sandbox_assets/builders.rs` (302 lines) split into a `builders/`
  directory: `mod.rs` (helper + re-exports), `world.rs` (LDtk +
  data), `audio.rs` (SFX bank + music), `visuals.rs` (fonts +
  characters + bosses + intro sprites). The shared
  `with_embedded_core_candidate` helper lives in `mod.rs` (#11 rest).
- Settings dev-page toggles all route through the existing
  `apply_toggle(action, || …)` helper instead of duplicating the
  `if is_toggle_action(action) { … }` shape on 10+ arms (#12
  closeout, model.rs sheds ~25 lines).
- Settings `label_with_dev` cycle/nudge rows route through a new
  `format_cycle(label, value)` helper (24 inline `< / >` literals
  collapsed to 6 — the 6 left are in the helper definitions). Ten
  dev-page toggle arms also now route through the existing
  `format_toggle` helper instead of duplicating
  `format!("{label}: {}", on_off(value))` inline (#12 deeper
  closeout, model.rs sheds another ~33 lines).
- Cross-session benchmark candidates distilled: 'sandbox runtime
  mirror vs engine state authority' (#8) and 'module-local Bevy
  plugin extraction decision criteria' (#6 retrospective). Both
  linked from `dev/benchmark-candidates/index.md`.
- `update_body_mode` migrated off `Res<ControlFrame>` onto
  `&PlayerInputFrame` (#17.5 cont.). `interaction_input_system`
  migration was attempted then reverted — it runs mid-PlayerInput
  chain, before `sync_local_player_input_frame`, so a
  PlayerInputFrame read would deliver the previous frame's snapshot.
  Lesson captured in `dev/benchmark-candidates/per-player-component-mirror-schedule-boundary-2026-05-20.md`
  and `dev/journals/per-player-input-mid-chain-vs-late-chain-2026-05-20.md`.
- New tests pin two #17.5 / #8 invariants the migrations created:
  - `multiplayer_smoke_tests::two_players_have_independent_input_frames`
    asserts two spawned players carry independent `PlayerInputFrame`s
    (regression guard against a future "make it a Resource again" move).
  - `boss_encounter::damage::tests::record_boss_damage_does_not_re_fire_killed_when_already_dead`
    pins the `prev_hp > 0` guard so a second hit on an already-dead
    boss doesn't double-route quest / save side effects.
  Sandbox lib test count: 566 → 568.
- Clippy hygiene pass across both crates: sandbox lib warnings
  dropped from 170 → 95 (-75); engine from 4 → 1. Knocked out 38
  redundant `&mut *Mut<T>` reborrows, 13 doc-list-misparse `+`/`-`
  line-starts, 5 over-indented doc list continuations, 3 derivable
  Default impls, 2 `field-reassign-after-default` patterns, 2 manual
  contains/clamp, 2 `map_or(false, ...)` → `is_some_and`, 2
  `bool::then[_some]` / `filter_map` tidies, plus single-occurrence
  fixes for orphan docstrings, `&mut Vec` → `&mut [_]`, identical
  branches, lifetime elision, `?`-operator early-out, etc. The
  remaining ~95 sandbox warnings are predominantly Bevy-inherent
  (Query type complexity / system fn arity).
- Settings `label_with_dev` cycle/nudge rows route through a new
  `format_cycle(label, value)` helper (24 → 6 inline `< / >` literals)
  and the existing `format_toggle` helper picks up 10 dev-page
  arms that were duplicating `format!("Foo: {}", on_off(...))`
  inline.
- `ActorTarget { entity, pos }` component + `select_actor_targets`
  system (#17.8 complete). Picks nearest alive `PlayerEntity` per
  non-player actor at the top of `WorldPrep`; `enemy.update` /
  `npc.update` / `boss.update` now take `target_pos: ae::Vec2`
  instead of `&ae::Player` (4 + 2 + 1 internal `player.pos` reads
  migrated; `update_ecs_actors` and `update_ecs_bosses` read
  `target.pos` from the per-actor component, 10 test fixtures
  updated to pass `player.pos`). Future per-actor target policies
  (sticky, role-based, distance-weighted) swap into the selector
  without touching any actor update signatures. +4 unit tests pin
  the selection invariants.
- Projectile world-block collision unified through
  `crate::projectile::resolve_world_collision` with a `WorldHitPolicy`
  enum (#17.7). Both `update_projectiles` (player) and
  `update_enemy_projectiles` (enemy) call the same helper; the
  bouncing-vs-expire-on-contact asymmetry lives in one place. Damage
  routing stays per-faction (player → `DamageEvent` against
  breakable/actor/boss; enemy → `PlayerDamageEvent`) — deeper merge
  waits for `ActorHurtbox` + faction-aware target query. +5 unit
  tests pin the policy boundary.

Recently retired (engine-cleanup pass, see git log e5be8c8…HEAD):

- `RoomObject` / `RoomObjectKind` IR → per-family `Authored<T>` Vecs on `RoomSpec`
- `engine/src/enemy.rs` (Dummy / spawn_dummies)
- `engine/src/state_machines.rs` (seldom_state scaffold) + `seldom_state` dep
- `engine/src/music.rs` (Motif scaffold)
- `engine/src/physics.rs` (Avian vocabulary scaffold)
- `engine/src/boss_patterns.rs` + `BossEncounterState::current_pattern_schedule`
- `engine::combat::slash_hitbox` / `player_slash_hitbox` shortcuts
- `engine::DestinationLabel`, `Player.dash_available`, `world_with_moving_platform`
- `dev::mechanics::MechanicsRegistry` (HUD catalog scaffold)
- `presentation/parallax/` orphan (820 lines)
- `host::platform::power` scaffold (PowerProfile + WindowFocusState)
- `EncounterController` write-only entities + `sync_encounter_controller_states`
- `projectile::diagnostics::projectile_status_summary`
- `PhysicsControlledPlayerPrototype`, `AMBITION_DIR`, `SimulationSetup.sandbox_data`

Engine surface is now simulation primitives only (movement, collision, AABB,
abilities, combat, character AI, projectile, quest, save, cutscene, ledge
grab, kinematic, combat slots, interaction authoring payloads, debug labels).
Authoring lives on the sandbox side end-to-end.

Completed from this backlog: #3 (ecs/mod.rs split), #4 (root re-exports —
remaining are documented external API), #5 (RoomVisual split), #9 (RoomObject
retired), and the bulk of #17 (B-bucket migration, ActivePlayerAttack,
PlayerSafetyState, heal/damage target plumbing + reader migration, multiplayer
smoke tests).

---

## P0 / P1 — biggest design payoff

### 1. Make `headless` / minimal feature builds real

`crates/ambition_actors/Cargo.toml` defines `headless = []` but disabling
runtime dependencies only works once subsystem code is gated end-to-end.

**Patch shape**

* Audit modules that implicitly depend on optional systems: audio, Kira,
  Avian, inspector, LDtk runtime, mobile, UI, presentation.
* Put subsystem code behind clear feature gates instead of letting
  `visible` / desktop assumptions leak everywhere.
* Add CI or local commands for:

  ```bash
  cargo check -p ambition_actors --no-default-features --features headless
  cargo check -p ambition_actors --no-default-features --features visible
  cargo check -p ambition_actors --no-default-features --features web
  ```
* Treat this as a boundary-cleanup refactor, not just cfg sprinkling.

**Why it matters:** reveals accidental coupling faster than almost any
other refactor and gives a fast simulation/test target.

---

### 2. Finish breaking up `sandbox_update` into Bevy-native systems

`crates/ambition_actors/src/app/update.rs` still has a large `sandbox_update`
system that takes many resources and delegates to big phase functions.
`crates/ambition_app/src/app/feedback.rs` has `SandboxEventWriters`,
`SandboxQueues`, and `ProgressionResources` wrappers partly to stay under
Bevy's system-param limit. Parameter-limit management is shaping architecture.

The schedule is already strong: `crates/ambition_actors/src/app/schedule.rs`
defines ordered `SandboxSet`s such as `WorldPrep`, `PlayerInput`,
`PlayerSimulation`, `RoomTransition`, `Combat`, `PresentationSync`,
`FeatureCollection`, `FeatureInteraction`, `EncounterSimulation`,
`GameplayEffects`, `Progression`. Lean into that.

**Patch shape**

* Turn `player_control_phase`, `player_simulation_phase`, and room / combat /
  effect handling into smaller Bevy systems.
* Move each phase into module-local `Plugin`s. Keep the existing schedule sets.
* Replace broad queue bundles with typed messages/events where possible.
* Retain deterministic order by registering systems into the existing
  `SandboxSet`s.

**Good agent-sized first task:** extract only player input / control into one
or two systems while preserving tests and behavior.

---

### 6. Move subsystem registration into real Bevy plugins

`crates/ambition_app/src/app/plugins.rs` is around 1,200 lines. It already
has helper registration functions, but it still centralizes ownership of
simulation messages/resources, progression chains, encounter simulation,
gameplay effects, pause / menu / input / mobile / audio.

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

Then `app/plugins.rs` becomes an assembler, not the owner of every subsystem's
schedule details.

**Why it matters:** Bevy-centric direction; systems / resources / messages
live near the domain that owns them. Overlaps #2.

---

## P2 — bounded design improvements

### 8. Make boss encounter state authoritative, not mirrored

`crates/ambition_actors/src/boss_encounter/systems.rs` comments say
`BossRuntime.health` is still the source of truth because existing combat /
feature systems mutate it, while engine `BossEncounterState` is fed by
deltas. The same system clones registry maps to work around borrow structure,
and music / phase hooks are partly placeholder.

**Patch shape**

* Route boss damage through a typed message:

  ```rust
  BossDamageRequested { boss_id, amount, source }
  ```
* Let encounter state own health / progression / phase.
* Let actor runtime own transform, hit boxes, choreography, presentation.
* Emit phase / music / cutscene requests from encounter transitions, not from
  ad-hoc runtime checks.

**Why it matters:** boss fights are a major extensibility point. Two
partially mirrored authorities will get expensive.

---

### 10. Consolidate player and enemy projectile systems

There are separate `projectile` and `enemy_projectile` modules. Projectile
behavior usually wants one generalized system with ownership / faction /
hit-filter data. Overlaps #17.7.

**Patch shape**

* Introduce:

  ```rust
  ProjectileOwner
  ProjectileFaction
  ProjectileHitPolicy
  ProjectileDamage
  ProjectileMotion
  ```
* Use one projectile update / collision path.
* Keep player / enemy-specific spawning helpers as thin wrappers.
* Preserve different visuals / audio via presentation specs.

**Why it matters:** projectile mechanics will keep expanding (bosses, enemies,
traps, reflected shots, charged shots, friendly fire). A factioned model
scales.

---

### 7 rest. Per-boss pattern schedules in data

Boss profile + `EnemyArchetypeSpec` + brain table already landed. What's
left: replace per-boss attack schedule constructors in
`content/features/bosses.rs` with authored data structures so adding a new
boss doesn't require code changes for every phase schedule.

**Patch shape**

* Move `BossBehaviorProfile::clockwork_warden() / mockingbird() / gnu_ton()`
  schedule data into a `Vec<BossPhaseSchedule>` keyed by boss id + phase.
* Keep the constructor functions thin (load from table).
* Adding a new boss is then "add a row to the table" not "write a new
  constructor."

---

### 11 rest. Further per-domain split of `sandbox_assets/`

Already split into `mod.rs` + `ids.rs` + `builders.rs` + `tests.rs`. Further
split by domain is open:

```text
assets/sandbox_assets/
  catalog/
    mod.rs
    world.rs
    fonts.rs
    characters.rs
    audio.rs
    backgrounds.rs
    web.rs
```

Then consider moving static asset declarations to data files or a generated
manifest so canonical paths, embedded URLs, and include paths can't drift.

---

### 12 rest. Replace remaining giant settings matches with descriptors

Page-nav rows and cycle / toggle / nudge action handlers already collapsed.
What's left: the per-variant match in `label_with_dev` and the slider rows in
`apply_action`.

**Patch shape**

Create descriptor / binding data:

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

**Avoid overdoing it:** do not create a full generic UI framework. Just
reduce setting-specific duplication.

---

## P3 — polish / quality of life

### 13. Make map UI persistent instead of repainting every frame

`map_menu/ui.rs::sync_map_menu` already has change-gating (skips when
`MapMenuState` and `RoomSet` are unchanged), but the actual draw path still
despawns and respawns room rectangles.

**Patch shape**

* Spawn map room entities keyed by room id.
* Update only on changed map state, changed visited rooms, changed cursor /
  selection, or menu open / close.
* Use components for map tile / room presentation state.

**Why it matters:** clean, bounded Bevy-idiom refactor.

---

### 14. Move narrative / cutscene / dialog content out of Rust constructors

`crates/ambition_actors/src/dialog/content.rs` (1016 lines) and intro /
cutscene modules are large content-heavy Rust files.

**Patch shape**

* Define a small schema for dialog / cutscene content.
* Start with one low-risk content group.
* Keep runtime interpretation in Rust.
* Validate content in tests.

**Why it matters:** content iteration shouldn't require recompiling large
Rust files or touching control-flow code.

---

### 15. Reorganize very large test modules

`world/ldtk_world/tests.rs` is 1759 lines; movement tests, asset catalog
tests, projectile tests are similarly large.

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

**Why it matters:** better test navigation; smaller conflicts when agents
add tests.

---

## Player / multiplayer-readiness (#17 chain)

The B-bucket migration (iterate-all-players where targeting doesn't matter)
is done. `ActivePlayerAttack`, `PlayerSafetyState`, target fields on
`PlayerHealRequested` / `PlayerDamageEvent`, and the apply-damage reader
are per-player. Multiplayer smoke tests pin the architectural invariants.

**Update 2026-05-24:** The universal-brain interface (chunks 1-4f) added
a per-player input → brain → ActorControl path that subsumes much of
#17.5's intent: every player entity now carries a `Brain::Player(slot)` +
`ActorControl` + `ActionSet`, and `tick_player_brains` writes the per-tick
input into the brain. Per-archetype Brain + ActionSet attached to every
actor (player / NPC / enemy / boss) via spawn-time wiring. ActorControl
populated by shadow-tick for hostile actors + bosses. Resolver +
`ActorActionMessage` stream emits per-tick concrete-action requests.
Combat / projectile consumers are still on the legacy paths
(EnemyRuntime / BossRuntime / update_player) — daytime work flips them.
See [`docs/systems/brain-driver.md`](docs/systems/brain-driver.md) +
[`TODO-controllable-entity.md`](TODO-controllable-entity.md) for the
current shape and what's left.

What's left in the multiplayer chain — per-player input is half done,
plus shared actor facets, projectile faction, per-target enemy AI.

### 17.2 / 17.3 Shared actor facets

Add common components usable by players, enemies, bosses, and eventually
NPCs / breakables:

```rust
ActorIdentity
ActorFactionComponent
ActorBody
ActorHurtbox
ActorCombatStatus
ActorTarget
```

Initially as read-model / sync components rather than authoritative
replacements. Attach to the player first (`ActorFaction::Player`, body /
hurtbox sync), then enemies.

### 17.5 Per-player input components

The single biggest unblocker for the rest of the C-bucket audit closeout.
Today `ControlFrame` is a global resource representing one local player's
input.

```rust
#[derive(Component, Default, Clone, Copy)]
pub struct PlayerInputFrame {
    pub frame: ControlFrame,
}
```

* Populate from the existing local input resource for the primary local
  player.
* Make player movement systems read input from the player entity.
* Keep menu / pause input global for now.
* Leave network / remote input for future work.

### 17.7 Unify projectile hit detection around faction / hit policy

Overlaps #10. **First slice landed 2026-05-20**: shared
`resolve_world_collision(body, world, WorldHitPolicy)` helper now
backs both `update_projectiles` and `update_enemy_projectiles`; the
bouncing-vs-expire-on-contact asymmetry lives in the `WorldHitPolicy`
enum rather than duplicated in two systems. What's left:

* Give projectiles a `ProjectileHitPolicy` ECS component (today the
  policy is computed at the call site from the body's `faction` tag,
  not stored). Useful once a boss or trap projectile family wants a
  custom hit policy not captured by `Player` / `Enemy`.
* Query common `ActorHurtbox` / `ActorFactionComponent` for the
  damage-routing half (today's split is `DamageEvent` vs
  `PlayerDamageEvent` against different component sets).
* Prevent friendly fire by policy, not by separate code paths.
* Keep player / enemy projectile spawn helpers as thin wrappers.

### 17.8 Generalize enemy targeting

**Landed 2026-05-20**: `ActorTarget { entity, pos }` component +
`select_actor_targets` system pick the nearest alive `PlayerEntity`
each frame for every `ActorFaction::Enemy / Boss / Npc` actor.
`enemy.update`, `npc.update`, `boss.update`, and
`BossMovementProfile::target` all take `target_pos: ae::Vec2`
instead of `&ae::Player`; live call sites read from
`ActorTarget.pos`. Policy can swap (sticky-target, role-based,
distance-weighted) inside `select_actor_targets` without rippling
through any actor update signatures.

### 17.11 Remove compatibility shims after migration

Deferred until 17.5 / 17.7 / 17.8 land:

* Remove obsolete primary-player-only resources once component equivalents
  are used.
* Remove temporary adapters only after all systems have migrated.
* Keep migration patches small and behavior-preserving.

---

## S-bucket bugs from TODO.md (cross-reference)

These are in `TODO.md` proper but called out here because they touch
systems that the backlog refactors will affect:

- ~~Morph-ball sprite lingers after exit.~~ (fixed; TODO.md S-bucket cleared 2026-05-20)
- Pickups "don't disappear when collected" — likely stale; the view-index
  test asserts the contrary. Needs in-game verification.
- Wall-cling / lock-wall collision-correction debt
  (`docs/planning/tech-debt-log.md`).
- Cutscene / dialogue input + prompt mismatch.
- Menu mouse-hover vs keyboard-nav conflict.
- Touch controls can affect the player during cutscenes.

---

## Suggested next picks (smallest payoff/size ratio)

- **#10 / #17.7 projectile faction merge** — natural follow-on to the
  per-family `Authored<T>` migration; pulls projectile spawn / owner shape
  into the same idiom.
- **#15 test reorganization** — pure file movement, easiest to land.
- **#1 headless feature gate** — biggest design payoff but high cost;
  reveals accidental coupling.
- **#17.5 per-player input** — single biggest unblocker for the rest of the
  multiplayer chain (#17.7, #17.8, audit-doc C-bucket closeout).
