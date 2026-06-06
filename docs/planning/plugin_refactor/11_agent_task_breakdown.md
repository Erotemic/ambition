# Agent Task Breakdown

This document turns the roadmap into agent-sized tasks. Each task should be one architectural seam, not a vague refactor request.

## Task 1: add docs and inventory snapshots

Goal: check in planning docs and baseline inventory.

Steps:

```bash
python tools/ecs_inventory.py \
  --json docs/generated/ambition_ecs_inventory.baseline.json \
  --markdown docs/generated/ambition_ecs_inventory.baseline.md

python tools/ecs_inventory.py --include-tests \
  --json docs/generated/ambition_ecs_inventory.with_tests.baseline.json \
  --markdown docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

Expected output:

```text
docs/planning/plugin_refactor/
docs/generated/ambition_ecs_inventory.*
```

Do not change runtime behavior.

## Task 2: add architecture boundary test skeleton

Goal: create enforceable dependency direction before moving code.

Deliverables:

```text
tests/architecture_boundaries.rs
```

Test initial rules:

```text
- platformer_runtime does not import content/app/dev/presentation/portal
- mechanics portal does not import Ambition content/inventory/debug/LDtk directly
- app/plugins.rs direct subsystem registrations are tracked by allowlist
```

The first version can be allowlist-heavy.

## Task 3: create lifecycle proto-module

Goal: make entity lifetime a reusable API.

Deliverables:

```text
src/platformer_runtime/lifecycle/
  mod.rs
  markers.rs
  spawn_ext.rs
  cleanup.rs
```

Add:

```rust
commands.spawn_room_scoped(bundle);
commands.spawn_run_scoped(bundle);
commands.spawn_persistent(bundle);
```

Migrate room-authored spawn helpers and dynamic room-local entities incrementally.

## Task 4: portal plugin shell

Goal: move portal registration ownership out of app-level modules without moving function bodies.

Steps:

```bash
mkdir -p crates/ambition_sandbox/src/portal
git mv crates/ambition_sandbox/src/portal.rs crates/ambition_sandbox/src/portal/mod.rs
```

Add:

```text
src/portal/plugin.rs
src/portal/sets.rs
```

Create:

```rust
PortalPlugin
PortalCorePlugin
PortalGunPlugin
PortalTransitPlugin
PortalPresentationPlugin
PortalDebugPlugin
```

Keep existing public paths working where they are intended final facade paths.

Validation:

```bash
cargo test -p ambition_sandbox portal
cargo test -p ambition_sandbox plugin_minimal_app
```

## Task 5: extract generic raycasts

Goal: remove non-portal dependencies on `crate::portal` for generic world queries.

Move:

```text
portal::raycast_solids -> platformer_runtime::world_query::raycast_solids
portal::ray_aabb       -> platformer_runtime::world_query::ray_aabb
```

Update callers:

```text
blink
grapple
dive
item_pickup
portal placement/projectiles
```

Validation:

```bash
cargo test -p ambition_sandbox blink grapple dive portal
```

## Task 6: extract orientation/transit primitives

Goal: remove generic body orientation and teleport trace concepts from portal.

Move:

```text
ActorRoll/update_actor_roll -> platformer_runtime::orientation or body orientation
IntentionalTeleport         -> BodyTeleported message
```

Portal should emit `PortalTransit`; trace should consume `BodyTeleported` or `PortalTransit`.

## Task 7: split portal file mechanically

Goal: reduce risk and improve ownership without behavior changes.

Create modules:

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

Validation after each move:

```bash
cargo test -p ambition_sandbox portal
```

## Task 8: split dynamic portals from gate portals

Goal: separate gun portals from authored channel/gate portals.

Rename/split:

```text
Portal              -> PlacedPortal
PortalProjectile    -> PortalShot
PortalColor         -> PortalGunColor + PortalChannelColor
PortalRegistry      -> GatePortalRegistry
PortalPhase         -> GatePortalPhase
PortalConfig        -> GatePortalConfig or PortalGateConfig
```

Expect broad compile errors. Fix callers according to whether they mean gun portals or gate portals.

## Task 9: isolate Ambition portal integration

Goal: remove Ambition-specific wiring from reusable portal.

Create:

```text
src/ambition_content/portal/
  plugin.rs
  input_adapter.rs
  inventory_adapter.rs
  ldtk_schema.rs
  save_adapter.rs
  debug_adapter.rs
```

Move:

```text
ControlFrame -> FirePortalGun adapter
Item::PortalGun binding
OwnedItems / StashedActionSet integration
LDtk PortalGunSpawnSpec wiring
Debug overlay portal rows
```

## Task 10: add portal feature gate

Goal: make portal optional.

Add features:

```text
portal
portal_render
portal_ldtk
```

Gate:

```text
PortalPlugin install
PortalRenderPlugin install
PortalLdtkPlugin install
portal tests
portal content bindings
```

LDtk conversion should fail loudly if portal-authored entities exist while portal feature is disabled.

## Task 11: extract real crates

Only after proto-module boundary tests pass.

Start with:

```text
ambition_platformer_core
ambition_platformer_ecs
ambition_mechanics_portal
ambition_content
```

Do not extract every mechanic at once.
