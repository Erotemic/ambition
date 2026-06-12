# Staged Refactor Plan

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


The preferred style is staged shotgun refactoring: intentionally break imports and assumptions along useful seams, then fix forward. Avoid long-lived compatibility layers.

## Stage 0: docs and inventory branch

Branch:

```bash
git switch -c docs/pluginized-platformer-runtime-roadmap
```

Deliverables:

```text
docs/planning/plugin_refactor/
docs/generated/ambition_ecs_inventory.baseline.json
docs/generated/ambition_ecs_inventory.baseline.md
docs/generated/ambition_ecs_inventory.with_tests.baseline.json
docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

Actions:

- Check in the refactor plan.
- Add inventory snapshots.
- Identify stale docs that conflict with the new design goals.
- Define supported build personas.
- Define plugin boundary rules.

No behavior changes.

## Stage 1: lifecycle foundation

Branch:

```bash
git switch -c refactor/runtime-lifecycle-boundary
```

Deliverables:

```text
src/platformer_runtime/lifecycle/
  mod.rs
  markers.rs
  spawn_ext.rs
  cleanup.rs
  tests.rs
```

Actions:

- Introduce `spawn_room_scoped`, `spawn_run_scoped`, `spawn_persistent` helpers.
- Move or re-export `RoomScopedEntity`, `RoomVisual`, and related markers through runtime lifecycle.
- Update room-authored spawn sites to use lifecycle helpers.
- Add architecture tests around room-local spawn policy.

Expected useful breakage:

```text
- imports of RoomScopedEntity
- raw commands.spawn in room feature modules
- tests assuming old marker paths
```

Validation:

```bash
cargo test -p ambition_sandbox lifecycle room_scoped
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox gravity_room_reachability
```

## Stage 2: plugin registration ownership

Deliverables:

```text
PortalPlugin / PortalCorePlugin / PortalGunPlugin / PortalTransitPlugin
HeldItemGameplayPlugin
GravityPlugin
AbilityPluginGroup or equivalent
```

Actions:

- Move subsystem registration out of `app/plugins.rs`.
- Keep function bodies in place initially.
- Introduce stable subsystem schedule sets if they do not already exist.
- Make `app/plugins.rs` compose plugins instead of registering subsystem internals.

Expected useful breakage:

```text
- schedule ordering assumptions
- resources initialized centrally but needed by a plugin
- systems relying on registration order in app/plugins.rs
```

Validation:

```bash
cargo test -p ambition_sandbox plugin_minimal_app
cargo test -p ambition_sandbox --lib
```

## Stage 3: split portal module mechanically

Actions:

- Convert `src/portal.rs` to `src/portal/mod.rs`.
- Add submodules without behavior changes.
- Preserve final facade exports only.

Target modules:

```text
portal/types.rs
portal/color.rs
portal/gun.rs
portal/projectile.rs
portal/placement.rs
portal/transit.rs
portal/gate.rs
portal/presentation.rs
portal/debug.rs
portal/ldtk.rs
```

Expected useful breakage:

```text
- imports of portal types/functions
- debug overlay imports
- LDtk conversion imports
- rendering imports
- tests importing PortalColor or helper functions
```

Validation:

```bash
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox portal_lab_usable
cargo test -p ambition_sandbox portal_bridge_reachability
cargo test -p ambition_sandbox --lib
```

## Stage 4: extract generic helpers out of portal

Move:

```text
portal::raycast_solids       -> platformer_runtime::world_query::raycast_solids
portal::ray_aabb             -> platformer_runtime::world_query::ray_aabb
ActorRoll/update_actor_roll  -> platformer_runtime::orientation or body orientation
IntentionalTeleport          -> BodyTeleported message
GravityZone/GravityField     -> mechanics_gravity
```

Update users in blink, grapple, dive, item pickup, portal placement, rendering, and trace.

Validation:

```bash
cargo test -p ambition_sandbox blink
cargo test -p ambition_sandbox grapple
cargo test -p ambition_sandbox dive
cargo test -p ambition_sandbox portal
```

## Stage 5: semantic portal cleanup

Actions:

- Split `PortalColor` into `PortalGunColor` and `PortalChannelColor`.
- Rename dynamic `Portal` to `PlacedPortal`.
- Rename `PortalProjectile` to `PortalShot`.
- Split gate portal registry/types from gun portal types.
- Move transit cooldown to bodies.
- Make portal transit independent of holding the gun.

Expected useful breakage:

```text
- all callers must decide whether they mean gun portals or authored gate portals
- debug overlay must use the correct color type
- LDtk conversion must target gate/channel concepts explicitly
```

## Stage 6: portal feature gate

Actions:

- Add `portal` feature.
- Gate portal plugin installation.
- Gate portal tests, portal render, and portal LDtk.
- Make LDtk conversion fail loudly if portal-authored entities exist without portal support.

Expected useful breakage:

```text
- inventory assumes PortalGun exists
- LDtk conversion assumes PortalGunSpawn always compiles
- debug overlay assumes portal colors
- room transition assumes portal gate registry directly
```

Fix by moving Ambition-specific portal wiring to Ambition content/integration.

## Stage 7: Ambition content pack boundary

Deliverables:

```text
src/ambition_content/
  plugin.rs
  worlds/
  items/
  quests/
  bosses/
  enemies/
  music/
  dialogue/
  portal/
```

Move named game content into this module. `app/plugins.rs` should install `AmbitionContentPlugin`.

## Stage 8: real crate extraction

Only after proto-module boundary tests pass.

Likely order:

```text
1. ambition_platformer_core / ambition_platformer_ecs
2. ambition_mechanics_portal
3. ambition_platformer_ldtk / render adapters
4. ambition_content
5. additional mechanics crates
```

Do not start with full crate extraction.
