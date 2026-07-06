//! Avian2D-backed secondary physics for Ambition sandbox props.
//!
//! The player controller remains custom/kinematic. Avian owns only secondary
//! bodies for now: room colliders, breakable shards, defeated enemy pieces, and
//! other ragdoll-like effects. This gives us real physical motion where it adds
//! juice without surrendering platforming feel. A future physics-player mode can
//! be added behind the same boundary.

use ambition_engine_core as ae;
#[cfg(feature = "physics_debris")]
use ambition_engine_core::AabbExt;
#[cfg(feature = "physics_debris")]
use avian2d::prelude::*;
#[cfg(feature = "physics_debris")]
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

#[cfg(feature = "physics_debris")]
use crate::platformer_runtime::lifecycle::RoomVisual;
#[cfg(feature = "physics_debris")]
use ambition_engine_core::config::{world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_FX};

#[cfg(feature = "physics_debris")]
const SANDBOX_GRAVITY: f32 = 1250.0;
#[cfg(feature = "physics_debris")]
const STATIC_COLLIDER_Z: f32 = WORLD_Z_BLOCK - 1.0;
#[cfg(feature = "physics_debris")]
const DEBRIS_Z: f32 = WORLD_Z_FX - 2.0;
#[cfg(feature = "physics_debris")]
const PHYSICS_DESPAWN_GRACE: f32 = 0.25;

/// Runtime switch/tuning for secondary physics. It intentionally does not
/// affect the custom player controller.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PhysicsSandboxSettings {
    pub debris_enabled: bool,
    pub static_room_colliders: bool,
    pub default_lifetime: f32,
}

impl Default for PhysicsSandboxSettings {
    fn default() -> Self {
        Self {
            debris_enabled: true,
            static_room_colliders: true,
            default_lifetime: 4.2,
        }
    }
}

/// Marker for room-owned Avian entities so room transitions can retire them
/// through the physics-safe path instead of despawning active bodies immediately.
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsRoomEntity;

/// A body that has been disabled and hidden, but not yet despawned. Giving
/// Avian a short grace period to observe `RigidBodyDisabled`/`ColliderDisabled`
/// before entity removal avoids noisy wake attempts against already-removed
/// bodies during debris cleanup and room transitions.
#[cfg(feature = "physics_debris")]
#[derive(Component, Clone, Copy, Debug)]
pub struct PendingPhysicsDespawn {
    pub timer: f32,
}

/// Ephemeral Avian dynamic body spawned from breakables, defeated enemies, and
/// impact effects.
#[cfg(feature = "physics_debris")]
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsDebris {
    pub lifetime: f32,
}

// `PhysicsDebrisCue` / `DebrisBurstMessage` moved to `ambition_vfx::vfx`
// (E2): they are effect vocabulary a sim system EMITS — same family as
// `VfxMessage`. The Avian subscriber below stays here (the adapter half).
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};

/// Presentation-side subscriber. Reads `DebrisBurstMessage`s and spawns
/// Avian2D debris bodies via the existing `spawn_debris_burst` helper.
/// Skipped in headless builds.
#[cfg(feature = "physics_debris")]
pub fn physics_spawn_debris_messages(
    mut commands: Commands,
    mut messages: MessageReader<DebrisBurstMessage>,
    world: Res<ambition_engine_core::RoomGeometry>,
    settings: Res<PhysicsSandboxSettings>,
) {
    for message in messages.read() {
        spawn_debris_burst(&mut commands, &world.0, message.pos, message.cue, *settings);
    }
}

#[cfg(feature = "physics_debris")]
pub struct AmbitionPhysicsPlugin;

#[cfg(feature = "physics_debris")]
impl Plugin for AmbitionPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsSandboxSettings::default())
            .insert_resource(Gravity(BVec2::new(0.0, -SANDBOX_GRAVITY)))
            .add_plugins(PhysicsPlugins::default())
            .add_systems(
                Update,
                (
                    update_physics_debris_lifetimes,
                    complete_pending_physics_despawns,
                )
                    .chain(),
            );
    }
}

#[cfg(feature = "physics_debris")]
pub fn update_physics_debris_lifetimes(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<PhysicsSandboxSettings>,
    mut query: Query<(Entity, &mut PhysicsDebris, Option<&PendingPhysicsDespawn>)>,
) {
    let dt = time.delta_secs();
    for (entity, mut debris, pending) in &mut query {
        if pending.is_some() {
            continue;
        }
        if !settings.debris_enabled {
            retire_physics_entity(&mut commands, entity);
            continue;
        }
        debris.lifetime = debris.lifetime.min(settings.default_lifetime.max(0.1));
        debris.lifetime -= dt;
        if debris.lifetime <= 0.0 {
            retire_physics_entity(&mut commands, entity);
        }
    }
}

#[cfg(feature = "physics_debris")]
pub fn complete_pending_physics_despawns(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PendingPhysicsDespawn)>,
) {
    let dt = time.delta_secs();
    for (entity, mut pending) in &mut query {
        pending.timer -= dt;
        if pending.timer <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Tear down an Avian-managed entity safely. With `physics_debris` off this
/// is a no-op — sim code (room transitions) calls it unconditionally and
/// should compile/run regardless of whether debris bodies actually exist.
#[cfg(feature = "physics_debris")]
pub fn retire_physics_entity(commands: &mut Commands, entity: Entity) {
    commands.entity(entity).insert((
        RigidBodyDisabled,
        ColliderDisabled,
        PendingPhysicsDespawn {
            timer: PHYSICS_DESPAWN_GRACE,
        },
        Visibility::Hidden,
    ));
}

#[cfg(not(feature = "physics_debris"))]
pub fn retire_physics_entity(_commands: &mut Commands, _entity: Entity) {}

/// Add an Avian static collider mirroring a room block so dynamic debris can
/// bounce against the level. Player collision does not use these bodies.
/// No-op without the `physics_debris` feature.
#[cfg(feature = "physics_debris")]
pub fn spawn_static_collider_for_block(
    commands: &mut Commands,
    world: &ae::World,
    block: &ae::Block,
    settings: PhysicsSandboxSettings,
) {
    if !settings.static_room_colliders || !block_accepts_dynamic_debris(block.kind) {
        return;
    }
    let size = block.aabb.half_size() * 2.0;
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(size.x, size.y),
        Transform::from_translation(world_to_bevy(world, block.aabb.center(), STATIC_COLLIDER_Z)),
        Name::new(format!("Physics collider: {}", block.name)),
        RoomVisual,
        PhysicsRoomEntity,
    ));
}

#[cfg(not(feature = "physics_debris"))]
pub fn spawn_static_collider_for_block(
    _commands: &mut Commands,
    _world: &ae::World,
    _block: &ae::Block,
    _settings: PhysicsSandboxSettings,
) {
}

/// Spawn a deterministic burst of dynamic bodies at an Ambition world-space
/// position. `cue` chooses count, size, color, lifetime, and impulse.
#[cfg(feature = "physics_debris")]
pub fn spawn_debris_burst(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    cue: PhysicsDebrisCue,
    settings: PhysicsSandboxSettings,
) {
    if !settings.debris_enabled {
        return;
    }
    let mut spec = debris_recipe(cue);
    spec.lifetime = spec.lifetime.min(settings.default_lifetime.max(0.1));
    for index in 0..spec.count {
        let angle = seeded_angle(index, spec.count, pos);
        let speed = spec.min_speed
            + (spec.max_speed - spec.min_speed) * index as f32 / spec.count.max(1) as f32;
        let velocity = BVec2::new(angle.cos() * speed, angle.sin() * speed + spec.y_boost);
        let angular = if index % 2 == 0 {
            spec.spin
        } else {
            -spec.spin
        };
        let wobble = ((index as f32 * 1.37 + pos.x * 0.017 + pos.y * 0.011).sin() * 0.5 + 0.5)
            .clamp(0.0, 1.0);
        let size = BVec2::new(
            spec.size.x * (0.75 + 0.50 * wobble),
            spec.size.y * (1.15 - 0.30 * wobble),
        );
        spawn_debris_piece(
            commands,
            world,
            pos,
            size,
            velocity,
            angular,
            spec.color,
            spec.lifetime,
        );
    }
}

#[cfg(feature = "physics_debris")]
fn spawn_debris_piece(
    commands: &mut Commands,
    world: &ae::World,
    pos: ae::Vec2,
    size: BVec2,
    velocity: BVec2,
    angular_velocity: f32,
    color: Color,
    lifetime: f32,
) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_translation(world_to_bevy(world, pos, DEBRIS_Z)),
        RigidBody::Dynamic,
        Collider::rectangle(size.x.max(1.0), size.y.max(1.0)),
        LinearVelocity(velocity),
        AngularVelocity(angular_velocity),
        PhysicsDebris { lifetime },
        Name::new("Physics debris"),
        RoomVisual,
        PhysicsRoomEntity,
    ));
}

#[cfg(feature = "physics_debris")]
fn block_accepts_dynamic_debris(kind: ae::BlockKind) -> bool {
    matches!(
        kind,
        ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
    )
}

#[cfg(feature = "physics_debris")]
#[derive(Clone, Copy, Debug)]
struct DebrisRecipe {
    count: usize,
    size: BVec2,
    min_speed: f32,
    max_speed: f32,
    y_boost: f32,
    spin: f32,
    lifetime: f32,
    color: Color,
}

#[cfg(feature = "physics_debris")]
fn debris_recipe(cue: PhysicsDebrisCue) -> DebrisRecipe {
    match cue {
        PhysicsDebrisCue::Impact => DebrisRecipe {
            count: 4,
            size: BVec2::new(4.0, 4.0),
            min_speed: 75.0,
            max_speed: 170.0,
            y_boost: 70.0,
            spin: 6.0,
            lifetime: 1.8,
            color: Color::srgba(1.0, 0.38, 0.30, 0.86),
        },
        PhysicsDebrisCue::Breakable => DebrisRecipe {
            count: 9,
            size: BVec2::new(8.0, 6.0),
            min_speed: 120.0,
            max_speed: 280.0,
            y_boost: 135.0,
            spin: 9.0,
            lifetime: 4.5,
            color: Color::srgba(0.68, 0.46, 0.27, 0.92),
        },
        PhysicsDebrisCue::EnemyRagdoll => DebrisRecipe {
            count: 7,
            size: BVec2::new(9.0, 7.0),
            min_speed: 105.0,
            max_speed: 250.0,
            y_boost: 120.0,
            spin: 8.0,
            lifetime: 4.0,
            color: Color::srgba(0.96, 0.28, 0.24, 0.92),
        },
        PhysicsDebrisCue::BossRagdoll => DebrisRecipe {
            count: 16,
            size: BVec2::new(12.0, 9.0),
            min_speed: 130.0,
            max_speed: 340.0,
            y_boost: 180.0,
            spin: 10.0,
            lifetime: 5.8,
            color: Color::srgba(0.78, 0.25, 0.95, 0.94),
        },
    }
}

#[cfg(feature = "physics_debris")]
fn seeded_angle(index: usize, count: usize, pos: ae::Vec2) -> f32 {
    let phase = (pos.x * 0.013 + pos.y * 0.021).sin() * 0.45;
    let base = std::f32::consts::TAU * (index as f32 + 0.35) / count.max(1) as f32;
    base + phase
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_sandbox_settings_defaults_are_sensible() {
        let s = PhysicsSandboxSettings::default();
        assert!(s.debris_enabled, "debris should default on for the sandbox");
        assert!(
            s.static_room_colliders,
            "room colliders should default on so debris bounces off geometry"
        );
        assert!(
            s.default_lifetime > 0.0,
            "debris lifetime must be positive: got {}",
            s.default_lifetime
        );
        assert!(
            s.default_lifetime < 60.0,
            "debris lifetime should be a few seconds, not minutes"
        );
    }

    #[test]
    fn physics_debris_cue_variants_compare_distinct() {
        // Equality is consumed by debris_recipe pattern matching;
        // two variants compared equal would silently fall through to
        // the wrong recipe arm.
        assert_ne!(PhysicsDebrisCue::Impact, PhysicsDebrisCue::Breakable);
        assert_ne!(PhysicsDebrisCue::Impact, PhysicsDebrisCue::EnemyRagdoll);
        assert_ne!(PhysicsDebrisCue::Impact, PhysicsDebrisCue::BossRagdoll);
        assert_ne!(PhysicsDebrisCue::Breakable, PhysicsDebrisCue::EnemyRagdoll);
        assert_ne!(PhysicsDebrisCue::Breakable, PhysicsDebrisCue::BossRagdoll);
        assert_ne!(
            PhysicsDebrisCue::EnemyRagdoll,
            PhysicsDebrisCue::BossRagdoll
        );
    }
}
