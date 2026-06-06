# 0004 — runtime raycast seam and set-based portal/item ordering

Completed one of the early shortcuts from the first overlay: generic solid
raycasts no longer live conceptually in the portal mechanic.

Why this matters:

- Adds `platformer_runtime::collision::raycast_solids` and the supporting AABB
  slab query.
- Migrates blink, dive, grapple, and held-projectile code away from
  `crate::portal::raycast_solids`.
- Leaves a compatibility re-export in `portal.rs` so existing portal tests and
  downstream callers have a stable bridge while the migration finishes.
- Adds `ItemPickupSet` and makes portal transit order after the public item set
  instead of the concrete `ground_item_physics` function.
- Extends architecture-boundary tests and docs for these two rules.

Main files:

- `crates/ambition_sandbox/src/platformer_runtime/collision.rs`
- `crates/ambition_sandbox/src/blink.rs`
- `crates/ambition_sandbox/src/dive.rs`
- `crates/ambition_sandbox/src/grapple.rs`
- `crates/ambition_sandbox/src/item_pickup.rs`
- `crates/ambition_sandbox/src/portal.rs`
- `crates/ambition_sandbox/tests/architecture_boundaries.rs`
