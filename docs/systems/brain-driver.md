# Universal brain driver

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

The universal-brain interface is the current controllable-actor seam. It separates:

- **policy**: what the actor wants to do this tick (`Brain`);
- **capability**: what concrete effects that actor can perform (`ActionSet`);
- **integration**: the abstract per-tick frame consumed by movement/combat systems (`ActorControl` / `ae::ActorControlFrame`);
- **effects**: resolved `ActorActionMessage`s consumed by focused systems.

```text
input / AI snapshot / boss pattern
        ↓
Brain::tick or Brain::tick_with_actions
        ↓
ActorControl(ae::ActorControlFrame)
        ↓
movement/control consumers + ActionSet resolver
        ↓
ActorActionMessage { actor, request }
        ↓
focused EFFECTS/Combat consumers
```

## Current code shape

Key files:

```text
crates/ambition_sandbox/src/brain/
├── mod.rs              # Brain enum, ActorControl, ActorActionMessage, resolver emission
├── snapshot.rs         # BrainSnapshot, player input slot, wall/contact view
├── state_machine.rs    # reusable AI templates + tick_state_machine
├── action_set.rs       # ActionSet, ActionRequest, action specs, resolve()
├── boss_pattern.rs     # boss pattern brain profiles/states
├── player.rs           # PlayerInputFrame / ControlFrame -> ActorControlFrame
└── smash/              # Smash-style experimental brain and observation/action types
```

Sibling components on controllable entities:

| Component | Role |
|---|---|
| `Brain` | Policy backend: `Player(slot)` or `StateMachine(cfg)`. |
| `ActionSet` | Capability mapping from abstract intent to concrete melee/ranged/special specs. |
| `ActorControl` | Last-tick `ae::ActorControlFrame` emitted by the brain. |

`StateMachineCfg` currently covers `StandStill`, `Patrol`, `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`, `BossPattern`, and `Smash`.

## What is live now

This is no longer only a shadow or observation stream.

| Area | Current state |
|---|---|
| Player input -> brain | `tick_player_brains` reads the per-player input snapshot and fills `ActorControl`. |
| Player movement/control | `player_control_system` and `player_simulation_system` consume `ActorControl`; raw `ControlFrame` is not read inside those phases. |
| Player melee start | `attack_advance_system` gates player melee start from this player's `ActorActionMessage::Melee`; pogo start is still player-specific, while target-surface policy is centralized through `BlockKind::is_pogo_target()`. |
| Player projectiles | `emit_player_projectile_tick_messages` surfaces player projectile press/held/released/axis data as `ActionRequest::PlayerProjectileTick`; `update_projectiles` consumes that message stream instead of reading `PlayerInputFrame` directly. |
| NPCs | Peaceful NPCs tick through `Brain::StateMachine(Patrol/StandStill)` and apply the resulting frame through the shared kinematic path. |
| Enemy ranged | `spawn_enemy_projectiles_from_brain_actions` consumes `ActorActionMessage::Ranged` for hostile actors. |
| Enemy melee | `start_enemy_melee_from_brain_actions` consumes `ActorActionMessage::Melee` and starts the enemy windup/cooldown; `update_ecs_actors` still owns the windup -> active hitbox edge because the runtime owns that state. |
| Boss specials | GNU-ton apple rain and Gradient Sentinel special attacks consume `ActorActionMessage::Special` via focused systems in `content/features/ecs/brain_effects.rs`. |
| Boss movement/patterns | Bosses carry `BossPattern` brains and `ActionSet`s; current authored specials are on the message stream, while some boss runtime/body state remains in sandbox feature components. |

## Current scheduling

`add_simulation_plugins` installs `BrainPlugin`, then schedules the active pipeline:

1. `sync_local_player_input_frame` mirrors the primary `ControlFrame` into the local player's `PlayerInputFrame`.
2. `tick_player_brains` translates player input into `ActorControl`.
3. `emit_brain_action_messages` resolves every actor's `ActionSet` against its `ActorControl` and emits concrete requests.
4. `observe_brain_action_counter` records per-frame message counts for debug/HUD tooling.
5. Player simulation consumes `ActorControl` in `SandboxSet::PlayerSimulation`.
6. Combat/effects consumers read `ActorActionMessage` in `SandboxSet::Combat`.

The ordering is important: new consumers should run after `emit_brain_action_messages` and before the system they need to feed, for example before projectile ticking if spawned projectiles should move on the same frame.

## Remaining work

The main structural migration has landed. Remaining work is cleanup and extension:

1. **Pogo action ownership.** Pogo start is still a player-specific verb alongside `ActorActionMessage::Melee`. Its target policy is centralized, but the action-routing shape still needs a deliberate home: attack variant, special action, or `HitResult` reaction rule.
2. **Brain construction policy.** Enemy brain construction now applies stable per-actor variation, but the policy is still spread across default spawn, composite fan-out, and dismount paths. A shared builder would make future actor work safer.
3. **`ae::Player` decomposition.** ✅ Landed 2026-05-28 (commit `c02ca686`). The player entity carries 18 cluster components (`PlayerKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`); the monolithic `ae::Player` aggregate and the `PlayerMovementAuthority` wrapper are gone.
4. **Canonical hit pipeline.** Brain/action messages now start attacks, but the actual hit/damage metadata is still fragmented across `DamageEvent`, hostile `Hitbox`, `PlayerDamageEvent`, and boss outcomes.
5. **Possession / co-op.** The architecture supports swapping `Brain::Player(slot)` onto arbitrary actors, but production routing and UX are not implemented.

## Extension rule

When adding a new actor behavior, choose the smallest current seam:

1. A new **brain template** only when the actor needs a new policy/state graph.
2. A new **ActionSet spec** when the policy already exists but the concrete effect differs.
3. A new **EFFECTS consumer** when an `ActionRequest` variant has no real-world effect yet.
4. A new **hit pipeline field** when the effect is really damage/reaction metadata rather than a new side-effect message.

Per-entity variety should usually live in `ActionSet`, not in a new brain variant.

## What the seam enables

Because policy (`Brain`) and capability (`ActionSet`) are separate, several future features become component swaps instead of new special-case loops:

- **Possession.** Swap an entity to `Brain::Player(slot)` while keeping its `ActionSet`; the player's input resolves to that body's attacks/projectiles/specials. Production possession routing, camera ownership, and UI are still future work.
- **Runtime disposition changes.** A peaceful actor can become hostile by changing both policy and capability: for example `Patrol` + peaceful `ActionSet` to `MeleeBrute` + swipe/lunge `ActionSet`.
- **Wide variety from shared templates.** A striker, brute, shark, or future goblin variant can share one `MeleeBrute` policy but differ through `MeleeActionSpec`.
- **Different player bodies / co-op.** Multiple `Brain::Player(slot)` components can drive different bodies once input/camera/UX routing exists.
- **Scripted, remote, or test agents.** Future `Brain` variants can emit the same `ActorControlFrame` without changing enemy, boss, or player movement consumers.

Minimal possession sketch:

```rust
commands.entity(goblin).insert(Brain::Player(PlayerSlot::PRIMARY));
// Keep the goblin ActionSet. Attack input now resolves through goblin capabilities.
```

## Performance and maintenance notes

- Brain dispatch is intentionally enum-match based, not `Box<dyn Brain>`. Add a dynamic backend only when there is a real plugin/runtime requirement.
- Per-actor tick code should avoid allocation; build small snapshots and mutate the actor's `ActorControlFrame`.
- Do not add a separate hardcoded path for simple actors just to avoid the brain seam. Profile first; if dispatch ever matters, prefer batching by brain variant over forking behavior.
- Runtime components may own integration state such as windup timers, spawn accumulators, and active-window clocks. Policy decisions should still come from the brain/action path.
- `ActionSet` can become a god-struct if every special case becomes a top-level field. Prefer a small set of abstract verbs (`melee`, `ranged`, `special`, `move_style`) with enum specs that carry their own timings and tuning.

## Helper API

| Type | Helper | Notes |
|---|---|---|
| `Brain` | `stand_still()`, `npc_patrol(...)`, `is_player()`, `player_slot()`, `is_hostile()`, `label()` | Public actor-policy helpers. |
| `Brain` | `boss_pattern_state()` | Debug/presentation read for boss-pattern clocks. |
| `ActorActionMessage` | `is_melee()`, `is_ranged()`, `is_special()` | Cheap consumer filters. |
| `ActionRequest` | `label()` / `Display` | Trace/debug labels. |
| `ActionSet` | `peaceful()`, `can_attack()` | Capability helpers. |
| `ActorControlFrame` | `neutral()`, `wants_any_action()`, `clear_edges()` | Engine-side control helpers. |
| `BrainActionCounter` | resource | Per-frame emitted-request counts. |

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib actor_control
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox --lib player::systems
cargo test -p ambition_sandbox --lib content::features::ecs::brain_effects
cargo run -p ambition_sandbox --bin headless -- --ticks 30
```
