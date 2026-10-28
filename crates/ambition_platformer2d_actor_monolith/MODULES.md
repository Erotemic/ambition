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
| [`control`](src/control/mod.rs) | Participant driving authority, seat input, and body control acceptance. |
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

- **Body movement** uses `BodyKinematics` and movement clusters.
  `body::integrate_sim_bodies` advances home, actor, and boss bodies through
  the shared movement kernel. Do not add a separate body integration system.
- **A body's melee** is a `"attack"`-verb moveset move (`combat::moveset`) for
  every body — triggered by `trigger_moveset_moves`, struck by
  `advance_move_playback`, and projected back into `BodyMelee` / `MeleeSwing` as
  the anim/HUD read-model. ONE melee path; no player/actor driver split.
- **Who drives a body** is `DrivingParticipant`. `ControlledSubject` is the
  primary seat's read projection. `PlayerEntity` marks a home body;
  `PrimaryPlayer` marks the primary seat's home body. Possession records name
  each seat's target and return body. They do not replace driving authority.
- **The collision world** is `ambition_platformer2d_world::collision::CollisionWorld`, not
  a bare `SessionWorldRef<RoomGeometry>`. A sweep or raycast that reads only
  authored geometry misses
  moving platforms, ECS solids, and portal carves.

### Deterministic control

Use stable simulation identities for decisions that depend on order. See
[ADR 0023](../../docs/adr/0023-same-build-determinism.md).
Read each seat's control through the control module. Body execution consumes
`ActorControl`.

### Maintaining this file

The table above is generated from each module's own `//!` header:
`python scripts/modules_md.py --write`. `python scripts/modules_md.py` checks for
drift and exits non-zero. Everything under `## Notes` is hand-written and survives
regeneration.
