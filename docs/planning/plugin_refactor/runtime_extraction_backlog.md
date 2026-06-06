# Runtime extraction backlog

Tracks the sandbox-local proto-runtime modules that have NOT yet moved into the
`ambition_platformer_runtime` crate, with the concrete dependency blocking each one
and the condition that unblocks extraction. The `architecture_boundaries` guardrail
should assert that every module remaining under
`crates/ambition_sandbox/src/platformer_runtime/` (other than the facade `mod.rs`/
`prelude.rs`) has an entry here — so the backlog can't silently drift.

## Entries

```text
collision.rs
  current blocker:    crate::engine_core::World (raycast takes the concrete world)
  extraction cond.:   raycast operates on a generic SolidWorldQuery / SolidBlock view,
                      with a sandbox-side adapter from engine_core::World
  destination:        ambition_platformer_runtime::world_query
  stage:              M2

orientation.rs
  current blockers:   crate::physics::{gravity_upright_angle,GravityCtx},
                      crate::player::{PlayerEntity,PlayerKinematics,PrimaryPlayer},
                      crate::features::{ActorKinematics,BossKinematics}, crate::WorldTime
  extraction cond.:   a generic body position/velocity/facing interface (Body2d family)
                      so roll easing reads body components, not concrete player/actor/boss
                      kinematics; gravity-upright math takes a plain gravity dir
  destination:        ambition_platformer_runtime::orientation
  stage:              M3

transit.rs
  current blocker:    crate::portal_pieces (portal-map vector math)
  extraction cond.:   pure portal-map / velocity-between-normals math moved into
                      ambition_platformer_runtime::math (portal_pieces becomes a thin
                      caller of, or is folded into, runtime math)
  destination:        ambition_platformer_runtime::transit (or ::math)
  stage:              M1
```

When a module's blocker is removed and it moves into the crate, delete its entry here
and update the `platformer_runtime_crate_is_extracted` guardrail.
