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
  current blocker:    sandbox-side `impl SolidWorldQuery for engine_core::World`
                      (raycast itself is now generic; only the adapter names the world)
  done (M2):          `raycast_solids` is generic over `SolidWorldQuery`, a 1-method
                      trait (`for_each_solid_aabb(include_one_way, &mut visit)`) that
                      yields only the hittable AABBs. Its signature no longer mentions
                      `engine_core`; `ray_aabb` only uses `ae::Aabb` (= `Aabb2d`, a Bevy
                      type). So the trait + both fns are extraction-ready as-is.
  extraction cond.:   move trait + raycast_solids + ray_aabb to the runtime crate;
                      leave the `impl SolidWorldQuery for engine_core::World` adapter
                      sandbox-side (the only remaining tie to the concrete world).
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
```

Stage M1 (done): the pure portal-map vector math (`portal_rotation`, `rotate`,
`portal_tangent`, `portal_map_vec`) moved to `ambition_platformer_runtime::math`;
`crate::portal_pieces` re-exports it so its AABB/piece geometry and the other
in-sandbox users keep compiling. `transit.rs` then became dependency-clean and
moved into `ambition_platformer_runtime::transit`; the sandbox facade re-exports
both. Its backlog entry is therefore removed.

When a module's blocker is removed and it moves into the crate, delete its entry here
and update the `platformer_runtime_crate_is_extracted` guardrail.
