# Target Plugin Topology

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


The final plugin topology should be shallow at the bottom, modular in the middle, and game-specific only at the top.

## Layers

```text
Layer 0: leaf utility crates/plugins
Layer 1: reusable platformer runtime
Layer 2: reusable optional mechanics
Layer 3: adapters, presentation, authoring, devtools
Layer 4: Ambition-specific content and apps
```

## Runtime plugins

These plugins should be reusable and mostly game-agnostic.

```text
PlatformerRuntimePlugin
  umbrella plugin for core runtime pieces

PlatformerSchedulePlugin
  shared schedule sets and ordering conventions

PlatformerLifecyclePlugin
  RoomScopedEntity, RunScopedEntity, spawn helpers, cleanup

PlatformerWorldQueryPlugin
  solid world query, raycast, fit checks

PlatformerBodyPlugin
  body components, velocity, half extents, transit messages

PlatformerMovementPlugin
  movement integration, collision, ledge grab, ability suppression

PlatformerRoomPlugin
  rooms, loading zones, room transition, loading-zone gate registry

PlatformerProjectilePlugin
  generic projectile motion/collision primitives

PlatformerInteractionPlugin
  generic affordance/interactable/proximity concepts
```

## Optional mechanics plugins

```text
GravityPlugin
  gravity zones, base gravity, gravity field, gravity reset

HeldItemPlugin
  generic pickup/drop/throw/ground-item behavior

CombatPlugin
  damage, hitboxes, health, teams, status effects

EncounterRuntimePlugin
  encounter state, waves, lock walls, rewards interface

BossRuntimePlugin
  boss phase runtime, generic attack primitives, pattern execution

PortalPlugin
  portal gun, portal shots, placed portals, transit, authored gate state
```

## Adapter and presentation plugins

```text
PlatformerRenderPlugin
  generic presentation primitives, camera sync, room visuals, actor visuals

PlatformerLdtkPlugin
  generic LDtk adapter for rooms, collision, loading zones, authored spawns

PortalRenderPlugin
  portal body pieces, rings, shots, disorientation indicator, debug gizmos

PortalLdtkPlugin
  portal gun spawn, gate portal, placed portal pair LDtk conversion

AudioPlugin
  audio runtime and music cues

DialoguePlugin
  Yarn/dialogue runtime and bindings

UiCorePlugin
  reusable UI navigation/pointer/list primitives

DevtoolsPlugin
  debug overlay, trace, profiling, inspectors
```

## Ambition-specific plugins

```text
AmbitionContentPlugin
  named worlds, items, quests, enemies, bosses, music/dialogue IDs

AmbitionPortalIntegrationPlugin
  ControlFrame -> portal messages, Item::PortalGun binding, LDtk schema hooks,
  save/debug/presentation adapters

AmbitionInventoryPlugin
  Ambition inventory UI and item behavior bindings

AmbitionQuestPlugin
  Ambition quest registry and progression bindings

AmbitionBossContentPlugin
  named boss profiles, sprites, banter, rewards, encounter content

AmbitionSandboxPlugin
  executable shell composition
```

## Example final app composition

```rust
app.add_plugins((
    PlatformerRuntimePlugin,
    PlatformerMovementPlugin,
    PlatformerRoomPlugin,
    PlatformerProjectilePlugin,
    GravityPlugin,
    HeldItemPlugin,
    CombatPlugin,
    EncounterRuntimePlugin,
    AmbitionContentPlugin,
));

#[cfg(feature = "portal")]
app.add_plugins((
    PortalPlugin,
    AmbitionPortalIntegrationPlugin,
));

#[cfg(feature = "visible")]
app.add_plugins((
    PlatformerRenderPlugin,
    AmbitionPresentationPlugin,
));

#[cfg(all(feature = "portal", feature = "visible"))]
app.add_plugins(PortalRenderPlugin);

#[cfg(feature = "ldtk")]
app.add_plugins(PlatformerLdtkPlugin);

#[cfg(all(feature = "portal", feature = "ldtk"))]
app.add_plugins(PortalLdtkPlugin);
```

## Third-party portal-only usage

A non-Ambition game should be able to do:

```rust
app.add_plugins((
    PlatformerRuntimePlugin,
    PlatformerWorldQueryPlugin,
    PlatformerMovementPlugin,
    PortalPlugin,
));
```

It should not need Ambition inventory, Ambition LDtk schema, Ambition debug overlay, Ambition item IDs, or Ambition content.
