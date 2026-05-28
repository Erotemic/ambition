# Player ECS bandaid plan

Review date: 2026-05-27  
Reviewed source snapshot: `ambition-source-2026-05-26T222032-5-3e93516618a5.tar.gz`  
Primary target: remove the live `ae::Player` / `PlayerMovementAuthority` runtime authority and make the player a Bevy ECS actor directly.

This document records the architecture discussion around the player refactor, the Silksong-inspired movement/combat findings, and a concrete plan for attacking the work quickly without leaving long-lived transition code in the repo.

## Executive summary

The important refactor is not "delete `ambition_engine`". The important refactor is to delete the **monolithic player runtime aggregate** from the live gameplay loop.

The target shape is:

```text
Before:
  Bevy ECS player entity
    -> PlayerMovementAuthority { player: ae::Player }
      -> ae::update_player_* mutates the aggregate
      -> sandbox mirrors pieces into PlayerBody / PlayerCombatState / presentation state

After:
  Bevy ECS player entity
    -> small authoritative gameplay components
      -> Bevy systems update movement, buffers, collision, combat, resources, and presentation messages directly
```

`ambition_engine` can stay as a small reusable/core crate for geometry, tuning data, pure helpers, specs, save/data structs, and deterministic math. It should stop owning the live player state machine.

The repo already points in this direction:

- ADR 0002 says the engine is allowed to be Bevy-native and should not preserve backend neutrality as a goal: [`docs/adr/0002-engine-must-be-bevy-native.md`](../adr/0002-engine-must-be-bevy-native.md).
- ADR 0012 says simulation should emit data/messages and presentation should consume them: [`docs/adr/0012-sim-presentation-split-and-events-refactor.md`](../adr/0012-sim-presentation-split-and-events-refactor.md).
- ADR 0016 says gameplay entities should converge on actor-like ECS data: [`docs/adr/0016-actor-unification.md`](../adr/0016-actor-unification.md).
- The brain/action seam exists and the player already flows through `ActorControl`: [`crates/ambition_sandbox/src/actor_control.rs`](../../crates/ambition_sandbox/src/actor_control.rs), [`crates/ambition_sandbox/src/brain/player.rs`](../../crates/ambition_sandbox/src/brain/player.rs), and [`crates/ambition_sandbox/src/app/player_tick.rs`](../../crates/ambition_sandbox/src/app/player_tick.rs).
- The player entity already carries several useful ECS components: [`crates/ambition_sandbox/src/player/components.rs`](../../crates/ambition_sandbox/src/player/components.rs).

The blocker is that `PlayerMovementAuthority` still wraps `ae::Player`, and `PlayerBody` is still a read-model mirror created from `ae::Player`. That means the player is only partially ECS-authoritative.

## What we discovered

### Movement

Current movement already has a rich base:

- coyote time;
- jump buffer;
- dash buffer;
- variable-height jump through jump-release velocity clipping;
- terminal fall-speed clamp;
- double jump;
- directional dash;
- wall cling, wall climb, wall jump;
- ledge grab / getup / release options;
- pogo / downward attack bounce path;
- glide, fast-fall, blink, dodge roll, shield/parry, body modes, water, and climbable contact scaffolding.

The most important Silksong-style gaps are not more one-off verbs yet. They are:

1. a generic input/action buffer and cancel-window system for jump, dash, attack, pogo, projectile/tool, blink, ledge actions, and future harpoon-style actions;
2. apex hang / jump sustain polish;
3. sprint/long-jump momentum rules;
4. separated movement collider, hurtbox, hitbox, and ledge-probe semantics.

`ae::Player` currently stores many of the fields that future work wants to be ordinary components: jump and dash buffers, coyote time, body mode, wall/ledge state, blink state, dodge/shield state, mana, damage multiplier, environmental contacts, and movement timers. That makes each improvement tend to add another field to the aggregate rather than another small ECS component.

### Combat / hits

The combat side has substantial behavior but lacks one canonical per-hit pipeline.

Current state is fragmented across:

- player outgoing `DamageEvent`;
- incoming `PlayerDamageEvent`;
- sandbox hostile `Hitbox` components;
- projectile collision code;
- boss-specific damage volumes;
- inline hitstop, VFX, SFX, stagger, knockback, and invulnerability decisions.

The Silksong-style lesson is to introduce a `HitSpec -> HitInstance -> HitResult` pipeline eventually. This is important, but it is **not a blocker** for removing `ae::Player`. The player refactor should make room for the hit pipeline, not wait for it.

### Headless

The repo has no-display/headless-ish runners and RL smoke paths: [`docs/systems/headless-simulation.md`](../systems/headless-simulation.md). They are useful, but they are not a sufficient reason to keep `ae::Player` as a custom mini-engine aggregate.

The enduring boundary should be:

```text
simulation plugins vs presentation plugins
```

not:

```text
ambition_engine runtime vs ambition_sandbox runtime
```

Real headless will be easier when the same Bevy ECS simulation systems can run with or without presentation plugins. A separate `ae::Player` aggregate does not provide that by itself, because much of the real game behavior already lives in the sandbox ECS layer.

## Refactor policy: pull the bandaid, but avoid hemorrhage

We do not want to maintain transition code.

That means:

- no long-lived dual-authority player path;
- no merged state where both `ae::Player` and ECS components are considered authoritative;
- no bridge whose only job is to keep both models alive indefinitely;
- no new features that add fields to `ae::Player` while the branch is in flight.

Some profuse bleeding is acceptable on the refactor branch:

- it is okay for an intermediate branch commit to be red while the aggregate is being removed;
- it is okay for a large compile-error wave to happen after `PlayerMovementAuthority` is cut;
- it is okay for the final merge patch to be broad.

Hemorrhage controls:

- do not merge the branch until it compiles and the smoke checklist passes;
- keep each temporary shim local to the branch and delete it before the merge-ready diff;
- maintain a field-by-field deletion ledger for `ae::Player` so no state silently disappears;
- preserve existing gameplay semantics first, then add Silksong polish after the authority cut;
- prefer one final authority at branch head over a pretty incremental history.

A good branch strategy is micro-commits locally, squash or clean up before merge. The final reviewed state should look decisive, not transitional.

## Non-goals for the first player cut

Do not include these in the first authority-removal pass unless they fall out naturally:

- full deletion of the `ambition_engine` crate;
- complete `HitSpec -> HitInstance -> HitResult` migration;
- new harpoon dash / tool system;
- sprint/long-jump design work;
- apex-hang tuning pass;
- PyO3 / real external RL binding work;
- no-winit cargo-feature purity for headless builds;
- multiplayer behavior beyond preserving existing per-player markers/components.

The first cut is successful if the player no longer requires `ae::Player` as the live runtime authority and the current game still plays.

## Target component vocabulary

The exact names can change, but this is the intended ownership model.

### Identity / control

- `PlayerEntity`
- `PlayerSlot`
- `PrimaryPlayer`
- `LocalPlayer`
- `PlayerInputFrame`
- `ActorControl`

`ActorControl` should remain the sole simulation input source. `PlayerInputFrame` may remain as the local-input mirror / story-content compatibility component, but the movement systems should not read raw `ControlFrame` directly.

### Body / movement

Split the current `PlayerBody` / `ae::Player` movement fields into authoritative clusters:

- `PlayerKinematics { pos, vel, size, base_size, facing }`
- `PlayerGroundState { on_ground, coyote_timer }`
- `PlayerWallState { on_wall, wall_normal_x, wall_clinging, wall_climbing, pre_wall_vel, pre_wall_vel_age }`
- `PlayerDashState { charges_available, timer, cooldown }`
- `PlayerJumpState { air_jumps_available, jump_buffer_timer, jump_cut/apex/sustain fields when added }`
- `PlayerBlinkState { cooldown, hold_active, hold_timer, aiming, aim_offset, grace_timer }`
- `PlayerLedgeState { ledge_grab, release_cooldown }`
- `PlayerDodgeState { roll_timer, cooldown }`
- `PlayerShieldState { active, parry_window_timer }`
- `PlayerBodyModeState { body_mode }`
- `PlayerEnvironmentContact { water_contact, climbable_contact }`
- `PlayerFlightState { fly_enabled, flight_phase, gliding, fast_falling }`
- `PlayerPlatformRideState`
- `PlayerSafetyState`

`PlayerBody` can either disappear or become a deliberately small read-model assembled for camera/HUD/trace convenience. It should not be a mirror of a hidden `ae::Player` authority.

### Resources / combat / presentation signals

- `PlayerHealth`
- `PlayerMana` or `PlayerResourceMeters`
- `PlayerOffense { damage_multiplier }`
- `PlayerCombatState { flash_timer, hitstop_timer, damage_invuln_timer, hitstun_timer, attacking }`
- `ActivePlayerAttack`
- `PlayerInteractionState`
- `PlayerAnimState`
- `PlayerBlinkCameraState`

The combat state can stay as-is initially. It is already ECS-owned and much closer to the desired shape than movement is.

### Action buffers

Add a small ECS-owned action buffer early, because removing `ae::Player` forces jump/dash buffers to move somewhere anyway.

Start with semantic compatibility:

```text
PlayerActionBuffer
  jump
  dash
  attack
  pogo
  projectile
  blink
  interact, if it replaces PlayerInteractionState's specific buffer later
```

The first pass may only wire jump and dash fully, preserving existing behavior. But the component should be shaped so attack/pogo/projectile/blink buffering can land next without adding more one-off timers.

## Triage: what blocks the Player cut?

### Required before cutting `PlayerMovementAuthority`

1. **Field ledger for `ae::Player`.**  
   Create a table mapping every `ae::Player` field to a destination component, helper-local temporary, or deletion reason. This prevents silent behavior loss.

2. **Baseline movement smoke evidence.**  
   Capture a cheap pre-refactor baseline with existing tests and a manual/headless smoke command. The point is not perfect determinism; the point is to know when the refactor obviously broke walk/jump/dash/attack/reset.

3. **Authoritative ECS component bundle.**  
   Update `PlayerSimulationBundle` so the player can be spawned with the new component set without needing `ae::Player` as storage.

4. **Timer tick home.**  
   Decide which system decrements movement/control timers after `ae::Player` is gone. Do not leave timer decay hidden in old engine calls.

5. **Collision helper boundary.**  
   Keep pure geometry/sweep helpers in `ambition_engine`, but make runtime movement systems pass explicit body/contact state instead of an entire `Player`.

### Not blockers

These are important but should not delay the cut:

- generic `HitInstance` pipeline;
- boss/enemy/player combat unification;
- full action-cancel matrix;
- apex hang;
- sprint jump / long jump;
- real no-render cargo-feature headless build.

## Detailed action plan

### Phase 0 — Pre-op safety rails

Goal: make the broad cut survivable.

1. Create a dedicated branch, e.g. `player-ecs-bandaid`.
2. Run and record baseline status:

   ```bash
   cargo check -p ambition_engine
   cargo check -p ambition_sandbox
   cargo test -p ambition_engine --lib
   cargo test -p ambition_sandbox --lib
   cargo run -p ambition_sandbox --bin headless 120
   ```

   If a command is already failing before the branch starts, record the failure in the branch notes and do not use it as a regression gate.
3. Freeze movement-feature additions while the branch is active. Bug fixes are allowed only if they unblock the refactor.
4. Add or identify one small smoke route for: idle, walk, jump, dash, attack, projectile, blink, ledge/wall, hazard damage, room reset.
5. Draft the `ae::Player` field ledger before deleting code.

Exit criteria: there is a known baseline and a complete destination map for the aggregate fields.

### Phase 1 — Create the ECS player runtime shape

Goal: make the ECS player entity capable of owning all player runtime state.

1. Add authoritative movement/state components listed in [Target component vocabulary](#target-component-vocabulary).
2. Update `PlayerSimulationBundle` to spawn them.
3. Make `PlayerBody` either:
   - the authoritative compact kinematic/body component, or
   - a generated read-model updated from the new ECS components.

   Prefer the first option if it does not become too large. Prefer small clusters if the file is turning into a second aggregate.
4. Add `PlayerActionBuffer` with at least jump and dash wired to existing constants.
5. Keep `ActorControl` as the input source for the player tick.
6. Do **not** add a new `PlayerRuntime` god component as a replacement for `ae::Player`.

Exit criteria: the player spawn shape contains the future authoritative state, even if old systems still fail to compile locally after the next phase.

### Phase 2 — Cut the authority

Goal: remove `PlayerMovementAuthority` and force compile errors to reveal every old dependency.

1. Remove `PlayerMovementAuthority` from the player bundle and common queries.
2. Delete or comment out `PlayerBody::from_player` and `write_player_ecs_components` usage rather than keeping them as mirrors.
3. Change `player_control_system` and `player_simulation_system` query shapes to borrow the new components directly.
4. Replace calls that take `&mut ae::Player` with local systems/helper functions that take explicit components.
5. Let the compiler produce the dependency list. Fix by category, not by reintroducing the aggregate.

Expected bleeding:

- player tick will break first;
- presentation/audio/trace readers that still ask for `PlayerMovementAuthority` will break;
- RL/headless observations will break;
- reset/room transition helpers will break;
- engine movement tests that instantiate `ae::Player` will either need to move up to sandbox-level tests or become pure helper tests.

Exit criteria: `PlayerMovementAuthority` is gone from the live player entity, and no code reads `authority.player`.

### Phase 3 — Rebuild movement feature clusters

Goal: preserve current behavior with ECS-owned state.

Port in this order:

1. **Kinematics + gravity + collision.**  
   Walk, fall, wall collision, ground contact, one-way drop-through, platform riding. This is the base required by every other verb.

2. **Jump cluster.**  
   Jump pressed/held/released, coyote timer, jump buffer, air jumps, variable-height jump release. Preserve current timing before adding apex hang.

3. **Dash cluster.**  
   Dash buffer, charges, cooldown, dash active timer, direction selection, refresh rules.

4. **Wall and ledge cluster.**  
   Wall cling/climb/jump, ledge grab state, ledge release cooldown, ledge options, pre-wall momentum carry.

5. **Blink cluster.**  
   Cooldown, hold-to-aim, precision aim offset, quick blink, blink grace, camera presentation message/state.

6. **Dodge/shield/parry cluster.**  
   Roll timer/cooldown, invulnerability, shield active, parry window.

7. **Environment/body-mode cluster.**  
   Water contact, swim behavior, climbable contact, crouch/crawl/morph body resize, fast-fall, glide, fly toggle.

8. **Resource/combat hooks.**  
   Mana spend/gain, damage multiplier, active attack state, projectile charge/release integration, pogo bounce queues.

Exit criteria: the manual smoke checklist works at approximately current behavior parity.

### Phase 4 — Repair dependent systems

Goal: replace old aggregate readers with explicit queries.

Likely files/modules:

- player spawn/state: [`crates/ambition_sandbox/src/player/components.rs`](../../crates/ambition_sandbox/src/player/components.rs), [`crates/ambition_sandbox/src/player/bundles.rs`](../../crates/ambition_sandbox/src/player/bundles.rs), [`crates/ambition_sandbox/src/player/systems.rs`](../../crates/ambition_sandbox/src/player/systems.rs);
- player tick: [`crates/ambition_sandbox/src/app/player_tick.rs`](../../crates/ambition_sandbox/src/app/player_tick.rs);
- broad sim helpers: [`crates/ambition_sandbox/src/app/sim_systems.rs`](../../crates/ambition_sandbox/src/app/sim_systems.rs), [`crates/ambition_sandbox/src/app/world_flow.rs`](../../crates/ambition_sandbox/src/app/world_flow.rs);
- movement helpers: [`crates/ambition_sandbox/src/engine_core/movement/`](../../crates/ambition_sandbox/src/engine_core/movement/);
- projectile charge/fire: [`crates/ambition_sandbox/src/projectile/`](../../crates/ambition_sandbox/src/projectile/);
- body modes: [`crates/ambition_sandbox/src/body_mode/`](../../crates/ambition_sandbox/src/body_mode/);
- player affordances: [`crates/ambition_sandbox/src/player/affordances/`](../../crates/ambition_sandbox/src/player/affordances/);
- audio environment: [`crates/ambition_sandbox/src/audio/environment.rs`](../../crates/ambition_sandbox/src/audio/environment.rs);
- presentation/camera/fx/HUD readers;
- trace/rl/headless observation paths.

Repair rule: when a system needs only position/velocity/contact, query only that. Do not introduce a large compatibility query that recreates the old aggregate shape.

Exit criteria: old aggregate readers are gone, and dependent systems express their actual data needs.

### Phase 5 — Delete old engine player runtime

Goal: remove dead code and prevent regression.

1. Delete `ae::Player` once no live sandbox code or desired tests require it.
2. Delete old `update_player_control_with_tuning` / `update_player_simulation_with_tuning` entry points if they only operate on `Player`.
3. Keep or extract pure helpers that are still useful:
   - AABB/sweep/collision helpers;
   - movement tuning constants;
   - ledge detection math;
   - blink destination math;
   - resource-meter math;
   - deterministic helper tests.
4. Update docs that still imply `ae::Player` is the player authority.
5. Add a regression check that greps for forbidden authority names if useful:

   ```bash
   rg 'PlayerMovementAuthority|authority\.player|ae::Player' crates/ambition_sandbox/src docs
   ```

   There may be acceptable references in archived docs, tests, or pure helper constructors. The point is to catch live runtime reintroduction.

Exit criteria: no live gameplay path depends on `ae::Player`.

### Phase 6 — Stabilize and preserve behavior

Goal: make the branch mergeable.

Run the gate again:

```bash
cargo check -p ambition_engine
cargo check -p ambition_sandbox
cargo test -p ambition_engine --lib
cargo test -p ambition_sandbox --lib
cargo run -p ambition_sandbox --bin headless 120
python scripts/check_doc_links.py
```

Manual smoke checklist:

- spawn into the default room;
- walk left/right;
- short jump and full jump;
- coyote jump;
- dash from ground and air;
- double jump;
- wall cling / wall jump;
- ledge grab / climb / release;
- attack forward/up/down;
- pogo bounce;
- charge/release projectile;
- blink quick and precision;
- shield/parry and dodge roll;
- take damage, hitstun, invulnerability, death/reset;
- interact with door/NPC/chest/switch;
- water/swim if the test room exposes it;
- morph/crouch/body mode;
- room transition and reset.

Exit criteria: current behavior is preserved closely enough that future movement polish has a stable base.

## Immediate follow-up after the player cut

Once the player is ECS-owned, the next work should attack the systems that motivated this refactor.

### 1. Generic action buffering and cancel windows

Build `PlayerActionBuffer` into an actor-level `ActionBuffer` that can be shared by player, enemies, bosses, and future possessed/remote actors.

First expansion:

- attack buffer;
- pogo buffer;
- projectile/tool buffer;
- blink buffer;
- ledge action buffers;
- explicit cancel-window table.

This should be easier after the player no longer stores jump/dash buffers inside `ae::Player`.

### 2. Hit pipeline

Introduce:

```text
HitSpec -> HitInstance -> HitResult
```

Use it to unify player attacks, projectiles, enemy hitboxes, boss volumes, hazards, stagger, hitstop, VFX/SFX, parry, and resource gain.

This is the combat equivalent of the player authority cut. It should come after the player cut unless combat breaks so badly during the refactor that a minimal hit payload becomes cheaper than preserving old event paths.

### 3. Silksong-style jump polish

Add or tune:

- apex hang;
- jump sustain;
- stronger short-hop/full-hop separation;
- fall gravity ramp;
- sprint/long-jump momentum rules;
- hurtbox/hitbox/probe audit.

Do this after authority migration so these are new components/systems, not more fields on the deleted aggregate.

### 4. Real headless hardening

Treat headless as a Bevy simulation-app composition problem:

```text
AmbitionSimulationPlugins
AmbitionPresentationPlugins
```

The player ECS cut helps because simulation state becomes visible/queryable directly instead of hidden behind `ae::Player`.

## Merge posture

This should be a bold branch, not a forever migration.

Recommended final PR description:

```text
This removes `ae::Player` / `PlayerMovementAuthority` as the live player runtime authority.
Player state is now owned by ECS components on the player entity.
The existing movement/combat feature set is preserved as closely as possible; follow-up work will add shared action buffering, HitInstance, and jump polish.
```

Recommended commit message if squashing:

```text
Make player runtime ECS-authoritative
```

Do not sell the PR as a full engine deletion. Sell it as the necessary cut that lets the engine shrink naturally and lets Bevy ECS become the real gameplay runtime.
