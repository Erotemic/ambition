# Portal Plugin Design

> Historical execution note: this file records the completed plugin-refactor run. It is not current planning guidance; use `docs/planning/plugin_refactor/README.md`, `22_monolith_breaker_survey.md`, and `runtime_extraction_backlog.md` for active follow-up.


Portal is the pilot optional mechanic plugin. It is the best candidate because it is large, cross-cutting, optional, and reusable.

## Current problem

Portal currently means several different things:

```text
1. Portal gun ability
   pickup/drop/equip/fire/toggle/shot

2. Dynamic placed portals
   projectile impact, pair replacement, transit, cooldowns, visuals

3. Authored gate portals
   LoadingZone gating, PortalRegistry, PortalPhase, switch-controlled portal sprites

4. Generic platformer utilities
   raycast_solids, ray_aabb, actor roll/orientation, transit math

5. Adjacent gravity/debug/presentation behavior
   gravity zones, gravity switch, portal body pieces, disorientation indicator,
   debug overlay, trace intentional teleport
```

The plugin refactor should separate these meanings before adding a Cargo feature gate.

## Public plugin API

Reusable usage should look like:

```rust
use bevy::prelude::*;
use ambition_platformer_ecs::prelude::*;
use ambition_mechanics_portal::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            PlatformerRuntimePlugin,
            PlatformerWorldQueryPlugin,
            PlatformerMovementPlugin,
            PortalPlugin::default(),
        ))
        .insert_resource(PortalConfig {
            max_range: 1400.0,
            opening_half_length: 36.0,
            ..default()
        })
        .add_systems(Update, my_portal_input.in_set(PortalSet::InputAdapter))
        .run();
}
```

The core plugin should not require Ambition inventory, Ambition LDtk schema, Ambition debug overlay, or Ambition item IDs.

## Core components

```rust
#[derive(Component, Debug, Clone)]
pub struct PortalGun {
    pub active: bool,
    pub next_color: PortalGunColor,
    pub source: PortalSourceId,
}

#[derive(Component, Debug, Clone)]
pub struct PortalBody {
    pub enabled: bool,
    pub half_extents: Vec2,
}

#[derive(Component, Debug, Clone)]
pub struct PlacedPortal {
    pub source: PortalSourceId,
    pub color: PortalGunColor,
    pub transform: PortalTransform2d,
    pub half_extents: Vec2,
    pub persistence: PortalPersistence,
}

#[derive(Component, Debug, Clone)]
pub struct PortalShot {
    pub owner: Option<Entity>,
    pub source: PortalSourceId,
    pub color: PortalGunColor,
    pub pos: Vec2,
    pub vel: Vec2,
    pub remaining_range: f32,
}

#[derive(Component, Debug, Clone)]
pub struct PortalGunPickup {
    pub gun: PortalGun,
    pub arm_timer: Timer,
}
```

Transit cooldown belongs to the body, not the gun:

```rust
#[derive(Component, Debug, Clone)]
pub struct PortalTransitCooldown(pub Timer);
```

## Color split

Dynamic gun portals and authored gate portals should not share one overloaded enum.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortalGunColor {
    Blue,
    Orange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortalChannelColor {
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
}
```

`PortalGunColor` is for player-placed blue/orange pairs. `PortalChannelColor` is for authored gate/channel portals and should own `from_name`, `partner`, and display-color logic.

## Messages into portal

```rust
pub struct FirePortalGun {
    pub owner: Entity,
    pub origin: Vec2,
    pub direction: Vec2,
}

pub struct TogglePortalGun {
    pub owner: Entity,
}

pub struct DropPortalGun {
    pub owner: Entity,
}

pub struct SpawnPortalPair {
    pub source: PortalSourceId,
    pub blue: PortalTransform2d,
    pub orange: PortalTransform2d,
    pub persistence: PortalPersistence,
}
```

Portal should consume semantic commands, not game-specific input state. Ambition should adapt `ControlFrame` into these messages.

## Messages out of portal

```rust
pub struct PortalPlaced {
    pub owner: Option<Entity>,
    pub portal: Entity,
    pub source: PortalSourceId,
    pub color: PortalGunColor,
    pub transform: PortalTransform2d,
}

pub struct PortalPlacementFailed {
    pub owner: Option<Entity>,
    pub source: PortalSourceId,
    pub color: PortalGunColor,
    pub reason: PortalPlacementFailure,
}

pub struct PortalTransit {
    pub body: Entity,
    pub from_portal: Entity,
    pub to_portal: Entity,
    pub from_pos: Vec2,
    pub to_pos: Vec2,
    pub velocity_before: Vec2,
    pub velocity_after: Vec2,
}

pub struct PortalGunDropped {
    pub owner: Entity,
    pub pickup: Entity,
}
```

Trace, audio, camera, particles, achievements, tests, and debug overlay can listen to these messages.

## Config

```rust
#[derive(Resource, Debug, Clone)]
pub struct PortalConfig {
    pub shot_speed: f32,
    pub max_range: f32,
    pub opening_half_length: f32,
    pub thickness_half: f32,
    pub transit_cooldown: f32,
    pub exit_clearance_margin: f32,
    pub min_exit_speed: f32,
    pub replace_existing_pair: bool,
    pub surface_policy: PortalSurfacePolicy,
}
```

## Plugin split

```rust
pub struct PortalPlugin;

impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            PortalCorePlugin,
            PortalGunPlugin,
            PortalPlacementPlugin,
            PortalTransitPlugin,
            PortalGatePlugin,
        ));
    }
}
```

Optional adapters:

```text
PortalRenderPlugin
PortalLdtkPlugin
PortalDebugPlugin
AmbitionPortalIntegrationPlugin
```

## Schedule sets

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PortalSet {
    InputAdapter,
    Gun,
    Shot,
    Placement,
    Transit,
    Cleanup,
    GateProgression,
    Presentation,
}
```

Cross-plugin ordering should use sets and messages, not concrete portal function names.

## Gate portal API

Authored/loading-zone gate portals are separate from gun portals.

```rust
#[derive(Component)]
pub struct GatePortal {
    pub channel: PortalChannelColor,
    pub side: PortalChannelSide,
    pub loading_zone: Option<LoadingZoneId>,
    pub phase: GatePortalPhase,
}

pub enum GatePortalPhase {
    Closed,
    Opening { t: f32 },
    Open,
    Closing { t: f32 },
}
```

Room transition should not depend directly on portal internals. Gate portals should write into a generic `LoadingZoneGateRegistry` owned by the room/runtime layer.

## Required infrastructure

The reusable portal plugin should require only:

```text
- Bevy app/ECS/math/transform basics.
- Generic platformer world query: raycast and body-fit checks.
- Generic body components: transform, velocity, half extents.
- Generic schedule sets or a way to map PortalSet into a game schedule.
- Optional lifecycle integration for room-scoped games.
```

It should not require:

```text
- Ambition Item enum.
- Ambition inventory.
- Ambition ControlFrame.
- Ambition PlayerKinematics.
- Ambition GameWorld.
- Ambition LDtk schema.
- Ambition debug overlay.
- Bevy rendering unless PortalRenderPlugin is installed.
```

## Staged portal refactor

1. Create portal plugin shell; move registration ownership out of `app/plugins.rs` and `presentation/rendering.rs`.
2. Extract generic helpers from portal into platformer runtime: raycast, fit checks, orientation/roll, generic transit messages.
3. Split the portal file mechanically into modules.
4. Rename/split concepts: `PlacedPortal`, `PortalGunColor`, `PortalChannelColor`, `GatePortalRegistry`, `PortalTransitCooldown`.
5. Isolate Ambition inventory/input/LDtk/debug glue.
6. Add `portal` Cargo feature gate.
7. Extract to real portal crates after intra-crate boundaries pass architecture tests.
