# 0003 — proto runtime lifecycle, schedule vocabulary, and subsystem ownership

Added the same-crate `platformer_runtime` proto-boundary and migrated early call
sites to explicit lifecycle helpers.

Why this matters:

- Introduces `RoomScopedEntity`, `RunScopedEntity`, `PersistentEntity`, and
  `SpawnScopedExt` as an API instead of relying only on marker convention.
- Adds reusable schedule vocabulary in `PlatformerRuntimeSet` while keeping the
  concrete sandbox schedule intact.
- Moves the first portal and held-item simulation registrations into
  `PortalSimulationPlugin` and `ItemPickupSimulationPlugin`.
- Reduces app-level registry ownership without touching inventory UI or cube menu
  code.

Main files:

- `crates/ambition_sandbox/src/platformer_runtime/**`
- `crates/ambition_sandbox/src/app/plugins.rs`
- `crates/ambition_sandbox/src/app/schedule.rs`
- `crates/ambition_sandbox/src/portal.rs`
- `crates/ambition_sandbox/src/item_pickup.rs`
- `crates/ambition_sandbox/src/content/features/ecs/spawn_static.rs`
