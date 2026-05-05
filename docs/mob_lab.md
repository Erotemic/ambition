# Mob lab encounter

A reusable encounter / wave system for the basement "mob lab"
sandbox area. The foundation lands as a self-contained Bevy
resource + state machine; the actual LDtk room and the spawn
pipeline are deferred and documented here.

## What lands now

```
crates/ambition_sandbox/src/encounter.rs
  EncounterMobSpec / EncounterWaveSpec / EncounterSpec
    (authored data — what to spawn, where, in what order)
  EncounterPhase  Inactive | Active{wave_index, remaining_mobs} | Cleared | Failed
  EncounterState  (Bevy resource)
    .maybe_start(player_pos, player_size)  → Vec<EncounterEvent>
    .on_mob_defeated()                     → Vec<EncounterEvent>
    .on_player_death()                     → Vec<EncounterEvent>
    .reset_for_retry()
    .hud_summary()
  EncounterEvent  Started | WaveStarted | EnemySpawned | Cleared
                  | Failed | LockChanged
```

The resource is registered in `add_simulation_plugins` with `spec`
set to `None`; once an LDtk-authored encounter is present the
loader will populate it.

## Lock / unlock semantics

- Player enters the trigger AABB → state advances to
  `Active { wave_index: 0, remaining_mobs: <wave 0 size> }`,
  `lock_active = true`, fires `Started` + `WaveStarted` +
  `EnemySpawned` per mob + `LockChanged { locked: true }`.
- Mob defeated → `on_mob_defeated()` decrements `remaining_mobs`;
  when zero it advances to the next wave (`WaveStarted` +
  `EnemySpawned` events) or marks the encounter `Cleared` (with
  `LockChanged { locked: false }`) if no wave remains.
- Player dies → `on_player_death()` marks the encounter `Failed`,
  releases the lock, and emits `Failed` + `LockChanged`.
- After respawn the sandbox calls `reset_for_retry()` which returns
  to `Inactive` so the next trigger entry restarts the encounter.

The locked state is intentionally a single boolean on the
`EncounterState` resource. The exit-lock behavior itself
(suppressing room transitions in the matching `LoadingZone`) is
sandbox-side and lives next to the loading-zone consumer once the
LDtk markers land.

## What's deferred (LDtk room + spawn pipeline)

The following pieces still need authoring + wiring before the lab
is playable in the visible binary. They're called out here so the
next agent can pick this up without re-deriving the design:

1. **LDtk authoring** — extend
   `crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`:
   - new active area `mob_lab` reachable from the basement via a
     `LoadingZone` round trip,
   - `Solid` floor / walls / ceiling boundary of the lab,
   - one `LoadingZone` for entrance + one for exit (the exit zone
     is suppressed while the encounter is locked),
   - `EnemySpawn` markers for each wave's mobs,
   - a custom `EncounterTrigger` entity for the activation AABB
     plus a wave-ordering int field,
   - a `CameraZone` matching the lab interior with the
     `camera_zoom = 1.5` zoom-out factor.

   Update `tools/validate_ambition_ldtk.py` to know the new entity
   identifier, then run the standard repair / round-trip / validate
   triple before saving.

2. **Loader** — extend `LdtkProject::to_room_set` (or a new
   `EncounterSpec::from_ldtk_area` helper) to read the mob_lab
   area's encounter trigger + spawn markers and populate
   `EncounterState.spec`.

3. **Spawn pipeline** — when `EncounterEvent::EnemySpawned` fires,
   the sandbox must spawn the actual enemy entity (today
   `features::FeatureRuntime` owns enemy lifetime; either reuse it
   or carry the enemy spec through the encounter event payload).

4. **Lock enforcement** — the exit `LoadingZone` consumer should
   read `EncounterState.lock_active` and refuse the room
   transition while the encounter is active.

5. **Camera zoom-out** — `CameraZone` already supports a zoom
   factor; the encounter starting / clearing should toggle the
   relevant zone or override the camera-follow scale by
   `spec.camera_zoom`.

6. **Trace plumbing** — extend `GameplayTraceEvent` with an
   `Encounter` variant (mirror of the `Projectile` shape) and
   project `EncounterEvent::label()` through the trace recorder.

## Tests landed today

`cargo test -p ambition_sandbox --lib encounter::` covers:

- entering the trigger starts the first wave,
- standing outside the trigger does not start,
- defeating each wave eventually clears the encounter,
- player death during an active encounter unlocks + marks failed,
- `reset_for_retry()` returns to `Inactive` after failure,
- HUD summary shows wave progress (`wave 1/2  remaining 1`).

## Why a foundation-only landing

The encounter system has well-defined semantics that benefit from
unit tests; landing it as a typed state machine plus events lets
the LDtk authoring + spawn pipeline land in a focused follow-up
patch without inventing the model under time pressure. The state
machine is small enough to be obvious and pure enough to be
testable — the next agent can wire spawning, locking, and zoom on
top without reverse-engineering the lifecycle.
