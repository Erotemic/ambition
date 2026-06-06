# ECS Inventory Notes

The ECS inventory is a planning input, not a hard pass/fail metric.

## Current high-level counts

From the current reported run:

```json
{
  "architecture_items": 18,
  "bundles": 8,
  "components": 253,
  "events": 0,
  "message_channels": 27,
  "messages": 19,
  "migration_candidates_high": 135,
  "migration_candidates_medium": 152,
  "module_summaries": 82,
  "non_ecs_items": 521,
  "plugins": 41,
  "registered_systems": 368,
  "registrations": 352,
  "resource_access_entries": 156,
  "resources": 149,
  "spawn_sites": 198,
  "system_like_functions": 566,
  "unique_registration_identifiers": 647
}
```

The `non_ecs_items` count has known false positives, so use it cautiously.

## How to use the inventory

Use the JSON to answer:

```text
Which modules own the most components/resources?
Which systems are registered from app-level plugins?
Which modules have many spawn sites?
Which systems access cross-cutting resources?
Which migration candidates cluster around portal, item, gravity, room, or app code?
```

Use the Markdown to support human review and planning.

## Suggested checked-in paths

```text
docs/generated/ambition_ecs_inventory.baseline.json
docs/generated/ambition_ecs_inventory.baseline.md
docs/generated/ambition_ecs_inventory.with_tests.baseline.json
docs/generated/ambition_ecs_inventory.with_tests.baseline.md
```

## Useful trends after each refactor branch

Track these rough trends:

```text
app/plugins.rs registrations should decrease.
Portal-owned systems should move into portal plugin registration.
Raw room-local spawn sites should decrease or become lifecycle-helper calls.
Generic helpers imported from portal should go to zero outside portal.
The first extracted generic helper is `platformer_runtime::collision::raycast_solids`.
Ambition-specific item/quest/boss/world references should migrate into ambition_content.
Rendering/audio/devtool imports should leave headless/runtime layers.
```

Do not optimize for the counts themselves. Optimize for clearer ownership and supported build personas.

## Inventory-driven questions for the code branch

When deeper code work starts, ask:

```text
1. Which resources are accessed by both portal and non-portal systems?
2. Which portal systems must run before or after player/item/actor simulation?
3. Which spawn sites are room-local but lack explicit lifecycle helpers?
4. Which components are presentation-only but live in simulation modules?
5. Which modules import `crate::portal` for non-portal utilities?
6. Which systems in app/plugins.rs can move behind subsystem plugins first?
7. Which tests assume portal is always compiled?
```

## Stage 6 follow-up: gravity-zone mechanic still in portal

The ambient gravity-flip system and `GravityZoneVisual` marker still live in
`portal/implementation.rs` (around the `reset_gravity_on_room_reset` /
`GravityZoneVisual` region). These are a *gravity mechanic*, not a generic
platformer helper, so Stage 6 deliberately left them in place per the
"only move a helper if the name and API become generic" rule. Extract them to a
dedicated gravity mechanic module (e.g. `mechanics/gravity` or
`platformer_runtime/gravity`) in a later stage (see Stage 6 step 4 and Stage 12
`mechanics/gravity/`), alongside `GravityZone` / `GravityField` / `BaseGravity`
ownership.

Stage 6 did extract the genuinely-generic helpers:
`portal_transform_velocity` -> `platformer_runtime::transit::rotate_velocity_between_normals`,
and `ActorRoll` / `ensure_actor_roll` / `update_actor_roll` ->
`platformer_runtime::orientation` (gravity-upright reflex, no portal dependency).

## Stage 8 follow-up: `PortalColor` split — DONE

Task G (Stage 8) deferred the `PortalColor` split as a genuine semantic redesign
rather than a mechanical rename. It is now done. The single `PortalColor` enum is
split into two distinct domain types plus a unifying channel:

- `PortalGunColor` (Blue/Orange) — the gun's two-slot pair. Used by `PortalGun`,
  the aim/mode indicator, and the gun's place-replace logic.
- `PortalChannelColor` (Purple/Yellow/Teal/Red/Green/Magenta/Cyan/Rose) — the
  authored channel pairs. Used by LDtk authoring (`convert_portal::from_name`)
  and the gate registry (`rooms::PortalSpec.color`).
- `PortalChannel { Gun(PortalGunColor), Authored(PortalChannelColor) }` — the
  unifying pair-linking identity the shared core operates on (`Copy`/`Eq`/`Hash`,
  so registry/`HashMap` usage is unchanged). Two portals pair iff same channel.

`PlacedPortal`, `PortalShot`, `PortalTransit.straddling`, `TransitStep`,
`find_portal`, `partner()`, the carve/registry, and `portal_teleport_ground_items`
are all generic over `PortalChannel`. The gun maps `PortalGunColor -> Gun(..)` via
`.channel()`; authoring maps `PortalChannelColor -> Authored(..)`. Gun colors and
authored channel colors are distinct types at their boundaries; only the shared
pairing/transit core is generic. This was a pure TYPE redesign — gun and authored
pairs behave byte-identically (replay fixtures + scripted gameplay stayed green,
no fixtures regenerated).
