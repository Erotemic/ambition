# `ambition_platformer2d_actor_monolith` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_platformer2d_actor_monolith** — The body, control, and action kernel.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abilities`](src/abilities/mod.rs) | Body traversal through authored room geometry. |
| [`avatar`](src/avatar/mod.rs) | Controlled body capabilities and execution. |
| [`body`](src/body/mod.rs) | Accepted body control, decisions, movement, and action execution. |
| [`body_custody`](src/body_custody.rs) | Projects body custody from authoritative roots and attachment relations. |
| [`body_mode`](src/body_mode/mod.rs) | Body-mode transitions from accepted control and contact facts. |
| [`brain_tick`](src/brain_tick.rs) | THE BRAIN DISPATCH, and it lives here because this is the only crate that can see every destination. |
| [`causal`](src/causal.rs) | The message the movement kernel writes for causal instrumentation. |
| [`character_projection`](src/character_projection/mod.rs) | Apply prepared character mechanics to live bodies. |
| [`clash`](src/clash.rs) | ONE CONTEST FOR EVERY ATTACK FAMILY — swings and shots arbitrated together, before either road resolves damage. |
| [`control`](src/control/mod.rs) | Local control seam from device input to the body driven by a participant. |
| [`participant_seat`](src/participant_seat.rs) | Central conversion between [`ParticipantId`] and [`PlayerSlot`]. |
| [`repertoire`](src/repertoire.rs) | Scheduling of each body's effective repertoire. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by the actor runtime. |

_13 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->


## Notes

This crate owns body, control, and action execution. Domain owners supply
construction, sessions, items, world policy, and presentation. Runtime composes
their schedules. See the
[decomposition plan](../../docs/planning/engine/actor-monolith-decomposition.md).

The public crate root exposes owned operations and semantic modules.
`body` exposes accepted control and body execution. Call lower domain owners for their types;
do not restore foreign forwarding exports here. `avatar` contains controlled
body integration; controller kind does not select a separate physics path.

### Authoritative state — who mutates what

- **A body's motion** is `BodyKinematics` + the 18 movement clusters, and exactly
  one kernel entry writes them: `ae::step_motion`. The player
  tick and `update_ecs_actors` are two Bevy systems calling the SAME body tick.
  Do not add a third.
- **A body's melee** is a `"attack"`-verb moveset move (`combat::moveset`) for
  every body — triggered by `trigger_moveset_moves`, struck by
  `advance_move_playback`, and projected back into `BodyMelee` / `MeleeSwing` as
  the anim/HUD read-model. ONE melee path; no player/actor driver split.
- **Who the human is driving** is `ControlledSubject` (the entity carrying
  `Brain::Player(slot)`), never a possession flag and never `PrimaryPlayer`.
  `PrimaryPlayer` means *slot 0's own body*, which is a different question; every
  surviving `PrimaryPlayerOnly` filter in this crate carries a comment saying why
  it is asking that one.
- **The collision world** is `ambition_platformer2d_world::collision::CollisionWorld`, not
  a bare `SessionWorldRef<RoomGeometry>`. A sweep or raycast that reads only
  authored geometry misses
  moving platforms, ECS solids, and portal carves.

### The two lints that will fail you

- `ambition_platformer2d_runtime/tests/determinism_lints.rs` (ADR 0023) — no ambient RNG, no
  wall clock, no `std` hash-container iteration, no `Entity` as an ordering key.
- `ambition_platformer2d_runtime/tests/control_frame_lint.rs` — only the input layer may hold
  the global `ControlFrame`. A body system that reads it is silently slot-0-only.
  Its allowlist doubles as the netcode N1 checklist.

### Maintaining this file

The table above is generated from each module's own `//!` header:
`python scripts/modules_md.py --write`. `python scripts/modules_md.py` checks for
drift and exits non-zero. Everything under `## Notes` is hand-written and survives
regeneration.
