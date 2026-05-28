# Tech debt log

Running tally of jank, shortcuts, and "good enough for now" decisions
that should be revisited as the project matures. New items go at the
top. Each entry tags severity (LOW / MED / HIGH), a category, and
(when known) the file:line that holds the smell. Resolved items move
to the bottom under "Closed" with the commit that fixed them.

> **Note for the agent**: when you take a shortcut, write it here in
> the same commit that introduced it. When you fix something on this
> list, move it to the Closed section with the fix commit.

## Open

### Simulation-order debt

- **RESOLVED 2026-05-16 — `SandboxRuntime` collected per-player fields
  without per-player ownership** — closed by the full ECS player
  migration. All movement, health, combat timers, anim state, blink
  camera state, and interaction state live on dedicated ECS components
  on the player entity (`PlayerMovementAuthority` and friends in
  `crates/ambition_sandbox/src/player/`). The god-object resource is
  gone; the `legacy_runtime_guardrail` integration test prevents
  re-introduction.

### Runtime / state

- **HIGH — Wall-cling on the goblin_encounter lock wall teleports player onto
  the arena ceiling**
  - Trace: `debug_traces/ambition_trace_1777995217-972692847-000000_20578d15h33m37s`.
  - Repro shape: enter `goblin_encounter`, the encounter starts, the runtime-
    inserted `lockwall:goblin_encounter` block (LDtk px (480, 400) size
    (224, 208)) materializes between the hallway and arena. While
    wall-clinging on the lock wall's right edge (player x=718,
    lock_wall.right=704), the next y-sweep snaps the player from
    `(718, 434.1)` to `(718, -23)` — exactly `arena_ceiling.top - half_height`
    (`0 - 23`). The player then ping-pongs between `y=-23` (sitting
    on the ceiling, AABB.bottom = 0) and `y=423` (re-clinging on
    the lock wall) every time they press Jump. The auto-OOB
    detector does not fire because the 46 px excursion above the
    world envelope is within `OOB_MARGIN = 96.0`.
  - Same *shape* as the resolved "Wall-cling y-sweep teleports
    player to wall's far edge" lesson in
    `dev/journals/lessons_learned.md`: an unconditional
    `pos.y += hit.block.aabb.top() - body.bottom()` snap when the
    swept hit returns `time_of_impact = 0`. The `body_is_side_contact`
    predicate added by that fix correctly skips the LOCK WALL (body
    y-range is nested inside lockwall y-range 400–608) — so the
    offending hit must be on a different block. The most likely
    suspect from the trace's "nearby collision" report is
    `arena_ceiling` (top at y=0, body.bottom at y=457 → snap delta
    -457 ≈ matches the observed -457.1 px correction).
  - Hypothesis to test: parry `cast_shapes(stop_at_penetration=true)`
    is returning a `time_of_impact = 0` hit on `arena_ceiling`
    when the body is starting from an exact-edge-touching configuration
    against the LOCK WALL. The vertical sweep then snaps to the
    ceiling's top instead of the (correctly-skipped) lock wall's
    top. This is one degree removed from the original lesson — the
    skip predicate filters the wall the body is touching, not the
    bogus far-block hit.
  - Repro target for the fix: an integration test in
    `crates/ambition_sandbox/tests/repro_walls.rs` that places the
    player wall-clinging on a lock-wall-shaped block with an
    arena_ceiling-shaped block above and asserts no >100 px y-snap
    after one update. Mirror of the existing
    `square_arena_wall_cling_full_world_does_not_teleport`.
    `goblin_encounter_lock_wall_cling_does_not_teleport` (added 2026-05-07)
    pins the geometry but currently passes — the simplified fixture
    isn't enough to reproduce the production teleport. The
    `body_is_side_contact` predicate from the wall-jump fix appears
    to handle this minimal case. Production trigger likely needs
    encounter-active / hot-inserted lock-wall context that the
    fixture omits. Keep the test as a regression guard; close this
    debt entry only after the parry contact-normal fix lands AND a
    full-trace replay reproduces+passes.
  - Bumping `OOB_MARGIN` is NOT the fix — it would just hide the
    teleport. The right fix is rejecting `time_of_impact = 0` hits
    that snap the body MORE than the velocity budget for the
    frame, which is also what the `CollisionCorrection` event in
    the trace measures (`unexplained delta 457.1px (vel-budget 16.1px)`).
  - Until fixed, runtime-inserted blocks (lock walls, future
    encounter geometry) live alongside this bug. Document any new
    runtime block insertion with this caveat.

- **RESOLVED 2026-05-21 — `EnemyRuntime` movement still pre-computes
  `desired_x` from brain enums, then overrides via `ai_mode`** —
  closed by the `ActorControlFrame` brain→sim seam (commit `155171c`).
  `EnemyRuntime::update` is now BRAIN → INTEGRATION → EFFECTS:
  `build_control_frame` packs `CharacterAi` + `AttackChoreography` +
  `KinematicPath` lookahead into a single `desired_vel`, and a uniform
  `step_kinematic` call replaces the per-brain position writes.
  Aerial + grounded + patrol now all collide through the same
  primitive. Remaining surgery is the data-table cleanup (push
  archetype knobs out of match arms), tracked in
  `docs/systems/character-ai-refactor.md` Step B.

- **RESOLVED 2026-05-16 — `SandboxRuntime` god-resource** — closed by
  the full ECS player migration. Player state, feature runtimes,
  dialogue, physics tuning, timers, mana, damage multiplier,
  invincible flag, and ledge-grab state are now ECS components on
  the player entity or narrow Bevy resources
  (`SandboxSimState`, `SandboxDevState`, `MovingPlatformSet`,
  `CurrentPlayerAttack`). Multi-player / per-player input feel is
  unblocked.

- **MED — Hostile NPC conversion is one-way; conversion path now
  tested but the race-with-other-mutators invariant isn't enforced**
  - File: `crates/ambition_sandbox/src/content/features/world_overlay.rs:apply_save`
  - When an NPC's hostile flag is set, `apply_save` removes the
    `NpcRuntime` and `spawn_enemy`s a striker with the same id. The
    `enemy_<id>_dead` save flag now suppresses the respawn loop
    (closed by commit `75ebfcb`). Four tests in
    `features::conversion_tests` lock the conversion behaviors in.
    Remaining smell: the invariant "only `apply_save` mutates
    `npcs`/`enemies` during the convert window" isn't enforced by
    types — a future system could violate it.

- **MED — `EnemyRuntime` and `BossRuntime` attack-pattern timers
  still hand-rolled** (downgraded from HIGH 2026-05-21; brain shadow
  landed 2026-05-24)
  - File: `crates/ambition_sandbox/src/content/features/`
  - Movement and collision are now unified through the
    `ActorControlFrame` brain→sim seam (commits `155171c`, `66c8b0b`),
    so the ad-hoc state machine that remains is just attack-pattern
    timer bookkeeping: `EnemyRuntime`'s wind-up / active / cooldown
    fields and `BossRuntime`'s `Cycle` / `Scripted` step machinery.
    These timers run in the EFFECTS stage after the frame is
    integrated, not before, so they no longer block the collision
    unification.
  - **2026-05-24 universal-brain landing:** every enemy + boss now
    also carries `Brain::StateMachine(...)` + `ActionSet` +
    `ActorControl` sibling components. The brain shadow-ticks
    alongside `EnemyRuntime` / `BossRuntime` and the resolver emits
    `ActorActionMessage`s, but the legacy timer fields still drive
    combat spawns. The shape is now ready for the EFFECTS-flip:
    swap one melee variant (e.g. PunchWeak for sandbags, Swipe for
    Striker) at a time onto the message stream, then delete the
    matching legacy spawn path. See
    `docs/recipes/extending-brains-and-action-sets.md` (Daytime
    EFFECTS-consumer flip).
  - Real refactor (in flight): swap the timer branches over to
    `MeleeBruteState` / `BossPatternState` so all combatants share
    one state machine. The brain templates already exist; the work
    is wiring consumers + per-archetype attack-spec authoring.
  - Downgraded because the position-space write that was the
    actually-incorrect part of the hand-rolled state machine is gone;
    what's left is shape-cleanup, not a correctness bug.

- **RESOLVED 2026-05-07 — `mana_current` / `mana_max` live on
  `SandboxRuntime`, not `Player`**
  - Resolved by promoting to `Player::mana: ResourceMeter` at the
    engine layer. The F3 inspector keeps the `i32` editable surface
    and converts at the boundary; reset path uses `mana.refill_full()`.
    See `crates/ambition_sandbox/src/engine_core/movement.rs` and
    `crates/ambition_sandbox/src/dev_tools.rs::sync_player_stats_with_inspector`.

### Encounter

- **MED — Encounter chest reward is hard-coded to a small heal**
  - File: `crates/ambition_sandbox/src/encounter/systems.rs:update_encounters_from_world`
  - When an encounter clears we drop a chest with
    `PickupKind::Health { amount: 2 }`. Real encounters want
    per-encounter reward authoring (an `EncounterSpec::reward` field
    pointing at a `PickupKind` or a chest spec id).

- **RESOLVED 2026-05-07 — `runtime.player_died_pending` is a
  side-channel boolean**
  - Resolved by promoting to a Bevy 0.18 buffered `PlayerDiedMessage`.
    `death_respawn_player` pushes into `FrameFeedback.died`;
    `flush_feedback` drains into `MessageWriter`; encounter system
    reads `MessageReader<PlayerDiedMessage>`. See
    `crates/ambition_sandbox/src/lib.rs::PlayerDiedMessage` and
    `crates/ambition_sandbox/src/app.rs::death_respawn_player`.

- **LOW — Camera ease snaps in overview mode**
  - File: `crates/ambition_sandbox/src/presentation/rendering/camera.rs:camera_follow`
  - Overview camera (F5) sets the live scale directly so the
    debug toggle is instant. If we ever want a smooth dev-only
    transition, parameterize the rate.

### Architecture

- **MED — `sandbox_update` is still a procedural orchestrator over
  named `*_phase` helpers**
  - File: `crates/ambition_sandbox/src/app/update.rs`,
    `crates/ambition_sandbox/src/app/phases.rs`
  - Promoted to real Bevy systems in
    `crates/ambition_sandbox/src/app/sim_systems.rs` (and gated by
    run-conditions where appropriate):
    - `input_timer_system` (was `input_timer_phase`),
    - `cleanup_timers_system` (was `cleanup_timers_phase`),
    - `apply_suspended_time_scale_system` (was `mode_gate_phase`),
    - `sync_live_player_dev_edits_system` (was a direct call to
      `dev_tools::sync_live_ability_edits` at the top of
      `sandbox_update`),
    - `interaction_input_system` (was `interaction_input_phase`),
    - `detect_room_transition_system` (was `room_transition_phase`,
      runs post-`sandbox_update`),
    - `attack_advance_system` (was `attack_phase`, post-tick; writes
      sfx / vfx / damage / pogo directly via `MessageWriter`s).
    `sandbox_update` itself now runs only in `GameMode::Playing`. The
    remaining inline phases — `reset_phase`, `player_control_phase`,
    `player_simulation_phase`, the inline `player_damage_events`
    collect, and `damage_heal_dialogue_phase` — still share
    `&mut ae::Player` and `&mut FrameFeedback`. Promote one phase at
    a time, gated by integration tests, when the borrow graph allows.
    See `feedback.rs` for the parallel `FrameFeedback` Vec-collector
    retirement plan (down to two channels: `sfx` and `vfx`).

- **RESOLVED 2026-05-19 — `RoomVisual` is a dual-purpose marker**
  — closed by the lifecycle/rendering separation in
  `presentation/rendering/primitives.rs`: `RoomScopedEntity` carries
  the room-scoped lifetime, `RoomVisual` carries only the "rendered
  this room" tag (with `#[require(RoomScopedEntity)]` so existing
  spawn sites stay correct). The matching bundle split landed in
  commit 6ef63ba: `FeatureLifecycleBundle` (sim + RoomScopedEntity +
  id/name/aabb) for headless / sim-only spawns, `FeatureRenderedBundle`
  (lifecycle + RoomVisual) for everything that draws today.
  `FeatureBaseBundle` is now a `pub type` alias for
  `FeatureRenderedBundle` so callers keep compiling unchanged.

- **RESOLVED 2026-05-16 — Two parallel chains in the Update schedule**
  — closed by introducing `SandboxSet` in
  `crates/ambition_sandbox/src/app/schedule.rs`. The schedule is now
  expressed as named system sets (`CoreSimulation`,
  `FeatureCollection`, `FeatureInteraction`, `LdtkRuntimeSpine`,
  `EncounterSimulation`, `Cutscene`, `GameplayEffects`, `Progression`,
  `ResetProcessing`, `Trace`) with `configure_sandbox_sets` chaining
  the main set order. Individual `add_systems` tuples stay under the
  20-system arity cap.

- **MED — `FeatureEventBus` is a workaround for the param-count cap**
  - File: `crates/ambition_sandbox/src/content/features/`
  - We fan events out through a resource because `sandbox_update`
    can't accept more `ResMut`s. Once the crate split lands and
    sandbox_update is replaced by per-entity systems, the bus may
    not be necessary at all.

- **LOW — `ProgressionResources` and `SandboxQueues` SystemParam
  bundles are ad-hoc**
  - File: `crates/ambition_sandbox/src/app.rs`
  - Pure pragmatism (16-param cap). Not jank exactly, but the bundles
    are growing. When the crate split lands, take a moment to make
    each bundle a public type with a docstring.

### Content / authoring

- **LOW — Boss spec id derives from name; explicit LDtk field still
  open**
  - File: `crates/ambition_sandbox/src/boss_encounter/ids.rs:encounter_id_from_name`
  - The encounter id now normalizes the LDtk `BossSpawn::name` field
    (closed by commit `75ebfcb`). A purpose-built `encounter_id`
    LDtk field would still be cleaner — the name doubles as both the
    HUD display string and the save key.

- **MED — Music tracks for boss phases are placeholders**
  - File: `crates/ambition_sandbox/src/boss_encounter.rs:BossEncounterSpec::gradient_sentinel`
  - Phase 1 / 2 / Enrage all reuse existing sandbox tracks. The swap
    *mechanism* works end-to-end (see the integration test) but the
    audio identity doesn't change yet. Authoring 3–4 tracks in the
    existing RON arrangement format is the gap.

- ~~**MED — Cutscene system has no skip-with-warning UX**~~
  *(Closed — `CutsceneAdvanceRequest::skip_hold_seconds` accumulates
  while the player holds `Reset` (Backspace/Delete/pad-Select); the
  HUD shows a progress bar above `SKIP_HOLD_THRESHOLD_SECS = 1.2`,
  and the input layer flips `skip_cutscene = true` once the hold
  passes the threshold. Reset was chosen over Start so the pause
  toggle is unaffected during cutscenes.)*

- **LOW — Quest log lines are inlined in the HUD format string**
  - File: `crates/ambition_sandbox/src/app/hud.rs:update_hud`
  - Real game wants a dedicated quest panel. Right now the lines
    just get appended to the debug HUD text.

- ~~**LOW — Map menu is text-only**~~
  *(Closed by commit `75ebfcb` — `MapMenuState::rooms` is populated
  from `LdtkProject::levels`, and `sync_map_menu` paints each room
  as a Bevy UI rectangle on a full-screen panel + corner minimap,
  with the active room highlighted. Despawn-and-respawn each tick is
  fine for <20 rooms; switch to per-room entities + change detection
  if room count grows.)*

- **LOW — Map UI repaints rectangles every frame**
  - File: `crates/ambition_sandbox/src/map_menu/ui.rs:sync_map_menu`
  - Despawn + respawn pattern is cheap for the current room count
    but isn't ideal. Switch to per-room entities with change
    detection if the map ever holds many rooms.

### Tests / observability

- ~~**MED — Hostile-NPC conversion has no integration test**~~
  *(Closed by commit on 2026-05-05 — `features::conversion_tests`
  now exercises strike-flips-hostile, apply_save replaces NPC with
  enemy, dead flag suppresses the respawn, and authored EnemySpawn
  enemies are marked dead from the save flag. Doesn't yet drive a
  full Bevy app; pure-data tests covered the four scenarios.)*

- **MED — Boss music swap requests aren't asserted in tests**
  - The integration test
    `boss_encounter::tests::full_encounter_progression_intro_to_death`
    checks that "some" music change fired but doesn't assert the
    specific track sequence. Tighten the assertion when the real
    boss tracks land.

- **LOW — Ledge grab probe doesn't test the "diagonal corner" case**
  - File: `crates/ambition_sandbox/src/engine_core/ledge_grab.rs`
  - We test wall-on-right + clear-above and reject blocked-above; we
    don't test what happens when two adjacent solid blocks form a
    long wall and the ledge is at the corner of the upper one. Edge
    case for an authoring scenario we haven't built yet.

### Build / repo

- **MED — Sandbox `headless` feature build pulls in `bevy_winit` via Cargo features**
  - File: `crates/ambition_sandbox/Cargo.toml` (root cause), plus
    `crates/ambition_sandbox/src/world/ldtk_world/bevy_runtime.rs`
    (separate code-side gating gap).
  - Root cause: the unconditional `bevy = { features = ["ui_api",
    "ui_bevy_render", "2d_bevy_render", ...] }` block chains into
    `ui_api -> default_app -> custom_cursor -> bevy_winit`. Cargo
    feature unification means `--features headless` cannot disable
    `bevy_winit` while those render features are unconditional, so
    `winit` enters the dep graph and fails to compile on hosts
    without x11/wayland dev libs (validated 2026-05-19 via
    `cargo tree -p ambition_sandbox --no-default-features
    --features headless --invert bevy_winit -e features`).
  - To unblock truly-headless builds, the `2d_bevy_render`, `ui_api`,
    `ui_bevy_render`, `scene`, and `png` bevy features must move out
    of the unconditional `[dependencies]` block and into a
    `visible_render` (or similar) sandbox feature, AND the code
    referencing `Text2d` / `Sprite` / `Camera2d` / UI nodes must be
    cfg-gated end-to-end. This is the boundary cleanup OVERNIGHT-TODO
    item #1 is asking for; Cargo-side flipping alone is impotent.
  - Per the LDtk runtime spine memory, the headless feature gate
    also doesn't fully cfg-out the LDtk plugin yet. The default
    visible build is fine; `cargo check --features headless` errors
    on `register_ldtk_entity` / `init_collection`. Tracked alongside
    `docs/systems/headless-simulation.md`.

- **LOW — Stale tools directory + AppImage at repo root**
  - Files: `LDtk 1.5.3 installer.AppImage`, `tmp-config`, `todo.txt`,
    various `tools/...` artifacts.
  - Many are .gitignored or untracked but the repo root is starting
    to look noisy. Periodic cleanup.

## Closed
- **MED — `SandboxRuntime::ledge_grab` stored player state outside `Player`** —
  closed by the ledge/swim movement-pipeline refactor (2026-05-13).
  `Player::ledge_grab` now lives in the engine and is ticked by
  `update_player_simulation_with_tuning`.

- **HIGH — Ledge grab / swim bypassed the main movement tick** —
  closed by the ledge/swim movement-pipeline refactor (2026-05-13).
  Ledge grab state now lives on `ae::Player` and is advanced by
  `update_player_simulation_with_tuning`; water/swim was already engine-owned
  and now emits an explicit `MovementOp::SwimStroke`.

- **HIGH — Hostile NPC death wasn't persisted (respawn loop)** —
  closed by commit `75ebfcb` (2026-05-05). `apply_player_attack`
  writes `enemy_<id>_dead` flag for non-encounter, non-sandbag
  enemies on death; `apply_save` honors the flag for both NPC
  conversions and authored EnemySpawn entries.
- **MED — Boss spec registration used LDtk iid, not a semantic id** —
  closed by commit `75ebfcb`. `encounter_id_from_name` normalizes
  the BossSpawn `name` field; runtime_id link table still maps to
  the iid for combat damage routing.
- **MED — Hostile-NPC conversion had no test** — closed by
  `features::conversion_tests` (4 tests).
- **LOW — Map menu was text-only** — closed by commit `75ebfcb`. Real
  Bevy UI panel + minimap with room rectangles drawn from LDtk
  worldX/worldY.
