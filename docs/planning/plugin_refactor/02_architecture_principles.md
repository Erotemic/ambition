# Architecture Principles

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


## Principle 1: Runtime owns verbs, content owns nouns

The runtime and mechanics plugins should own reusable behavior:

```text
move body
resolve collision
raycast solid world
spawn projectile
transit body through portal
apply damage
open loading zone gate
```

Ambition content should own named game data:

```text
PortalGun item
Mockingbird boss
Gnu Ton boss
intro route
cut_rope world
sandbox.ldtk
pirate treasure quest
music cue IDs
```

## Principle 2: Plugins own registration

Subsystems should register their own systems through plugin structs. The app should compose plugins.

Bad final shape:

```rust
// app/plugins.rs
app.add_systems(Update, crate::portal::portal_fire_system);
app.add_systems(Update, crate::portal::portal_transit_system);
app.add_systems(Update, crate::portal::sync_portal_body_pieces);
```

Good final shape:

```rust
app.add_plugins((
    PlatformerRuntimePlugin,
    PortalPlugin,
    AmbitionContentPlugin,
));
```

## Principle 3: Cross-plugin ordering uses sets or messages

Do not scatter concrete `.after(crate::some_module::some_system)` dependencies across subsystem plugins.

Preferred:

```rust
PortalSet::Transit
    .after(MovementSet::Integrate)
    .before(PresentationSet::Sync);
```

Use messages for semantic communication:

```rust
BodyTeleported { entity, from, to, reason }
PortalTransit { body, from_portal, to_portal, velocity_before, velocity_after }
```

## Principle 4: Lifecycle is an API, not a marker convention

The held-item/gravity leak showed that room-local entity lifetime must be encoded in spawn APIs and tests.

Preferred:

```rust
commands.spawn_room_scoped(bundle);
commands.spawn_run_scoped(bundle);
commands.spawn_persistent(bundle);
```

Raw `commands.spawn` should be suspicious in room-feature spawning modules.

## Principle 5: Generic helpers must not live in optional mechanics

If blink, grapple, dive, item physics, and portal placement all need raycasts, raycasting belongs in platformer runtime/world query, not in portal.

Examples:

```text
portal::raycast_solids          -> platformer_runtime::world_query::raycast_solids
portal::ray_aabb                -> platformer_runtime::world_query::ray_aabb
portal ActorRoll                -> platformer_runtime::orientation::BodyRoll
portal GravityZone              -> mechanics_gravity::GravityZone
```

## Principle 6: Adapters depend inward, core does not depend outward

Good:

```text
portal_render -> portal
portal_ldtk   -> portal
ambition_content -> portal
```

Bad:

```text
portal -> portal_render
portal -> portal_ldtk
portal -> ambition_content
```

## Principle 7: Feature flags should express build personas

Avoid feature soup. Prefer a small set of supported personas and a few high-value optional mechanics.

Supported personas should be documented and checked in CI.

## Principle 8: Split by stable responsibility, not by file size alone

Large files are risky, but file splitting alone does not fix architecture. Split files after or alongside ownership boundaries.

Portal order should be:

```text
1. Plugin shell and registration ownership.
2. Extract generic helpers.
3. Split portal file mechanically.
4. Rename/split concepts.
5. Add feature gates.
```

## Principle 9: Agents need narrow, enforceable tasks

Agents should not be asked to "refactor portal" as one patch. They should be asked to perform one architectural seam per patch and run targeted validation.

Good task:

```text
Move portal registration from app/plugins.rs into PortalPlugin without moving function bodies.
```

Bad task:

```text
Rewrite portal as a plugin and clean it up.
```
