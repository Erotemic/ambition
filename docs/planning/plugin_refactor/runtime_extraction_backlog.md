# Runtime extraction backlog

Tracks the sandbox-local proto-runtime modules that have NOT yet moved into the
`ambition_platformer_runtime` crate, with the concrete dependency blocking each one
and the condition that unblocks extraction. The `architecture_boundaries` guardrail
should assert that every module remaining under
`crates/ambition_sandbox/src/platformer_runtime/` (other than the facade `mod.rs`/
`prelude.rs`) has an entry here — so the backlog can't silently drift.

## Entries

```text
(none — the proto-runtime remainder is fully extracted as of Stage 16.)
```

Stage 16 (done): the generic ECS runtime layer moved into the crate, so there is
no not-yet-extracted remainder left under `platformer_runtime/` — every module
there is a facade/adapter:
- `world_query` (S1): `SolidWorldQuery` + `raycast_solids` + `ray_aabb` moved to
  `ambition_platformer_runtime::world_query`. The `impl SolidWorldQuery for
  engine_core::World` adapter moved with them (the orphan rule precludes keeping
  it sandbox-side once the trait is foreign; both the trait and `World`/`BlockKind`
  are content-free foundation types, so the adapter stays sandbox-free).
- `body` (S2): the unified `BodyKinematics` re-export moved to
  `ambition_platformer_runtime::body`, which also defines the neutral `PrimaryBody`
  marker (the sandbox adds it to the player bundle).
- gravity (S3–S4): `crate::physics` decoupled from `crate::WorldTime` (→ the
  neutral `SimDt` resource the sandbox mirrors from `WorldTime.sim_dt()`) and from
  the player markers (→ `PrimaryBody`), then moved to
  `ambition_platformer_runtime::gravity`; `crate::physics` is now a glob facade.
  The gravity *mechanic* (`GravityFlipSwitch` + switch system, room-reset reset,
  `GravityPlugin`, zone/switch visuals) stays sandbox-side in
  `crate::mechanics::gravity` (sandbox content deps) and consumes the moved core
  types via the facade — no duplicate gravity state.
- `orientation` (S5): `ActorRoll` + `ensure_actor_roll`/`update_actor_roll` moved to
  `ambition_platformer_runtime::orientation`; with gravity in-crate, scaled dt via
  `SimDt`, and the unified `BodyKinematics`, the dual player/actor query arms
  collapsed to a single `With<BodyKinematics>` query.

Stage M1 (done): the pure portal-map vector math (`portal_rotation`, `rotate`,
`portal_tangent`, `portal_map_vec`) moved to `ambition_platformer_runtime::math`;
`crate::portal_pieces` re-exports it so its AABB/piece geometry and the other
in-sandbox users keep compiling. `transit.rs` then became dependency-clean and
moved into `ambition_platformer_runtime::transit`; the sandbox facade re-exports
both. Its backlog entry is therefore removed.

When a module's blocker is removed and it moves into the crate, delete its entry here
and update the `platformer_runtime_crate_is_extracted` guardrail.
