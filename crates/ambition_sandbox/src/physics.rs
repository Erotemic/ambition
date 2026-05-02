//! Avian2D-backed secondary physics for Ambition sandbox props.
//!
//! The player controller remains custom/kinematic. Avian owns only secondary
//! bodies for now: room colliders, breakable shards, defeated enemy pieces, and
//! other ragdoll-like effects. This gives us real physical motion where it adds
//! juice without surrendering platforming feel. A future physics-player mode can
//! be added behind the same boundary.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use avian2d::prelude::*;
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

use crate::config::{world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_FX};
use crate::rendering::RoomVisual;

const SANDBOX_GRAVITY: f32 = 1250.0;
const STATIC_COLLIDER_Z: f32 = WORLD_Z_BLOCK - 1.0;
const DEBRIS_Z: f32 = WORLD_Z_FX - 2.0;
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

/// Marker for future experiments where the player is represented as a physics
/// body. Do not attach this to the current player; Ambition's main controller is
/// still authored in `ambition_engine::movement`.
#[allow(dead_code)]
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsControlledPlayerPrototype;

/// Marker for room-owned Avian entities so room transitions can retire them
/// through the physics-safe path instead of despawning active bodies immediately.
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsRoomEntity;

/// A body that has been disabled and hidden, but not yet despawned. Giving
/// Avian a short grace period to observe `RigidBodyDisabled`/`ColliderDisabled`
/// before entity removal avoids noisy wake attempts against already-removed
/// bodies during debris cleanup and room transitions.
#[derive(Component, Clone, Copy, Debug)]
pub struct PendingPhysicsDespawn {
    pub timer: f32,
}

/// Ephemeral Avian dynamic body spawned from breakables, defeated enemies, and
/// impact effects.
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsDebris {
    pub lifetime: f32,
}

/// High-level debris recipe used by gameplay event handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsDebrisCue {
    Impact,
    Breakable,
    EnemyRagdoll,
    BossRagdoll,
}

pub struct AmbitionPhysicsPlugin;

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

/// Add an Avian static collider mirroring a room block so dynamic debris can
/// bounce against the level. Player collision does not use these bodies.
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

/// Spawn a deterministic burst of dynamic bodies at an Ambition world-space
/// position. `cue` chooses count, size, color, lifetime, and impulse.
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

fn block_accepts_dynamic_debris(kind: ae::BlockKind) -> bool {
    matches!(
        kind,
        ae::BlockKind::Solid | ae::BlockKind::BlinkWall { .. } | ae::BlockKind::OneWay
    )
}

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

fn seeded_angle(index: usize, count: usize, pos: ae::Vec2) -> f32 {
    let phase = (pos.x * 0.013 + pos.y * 0.021).sin() * 0.45;
    let base = std::f32::consts::TAU * (index as f32 + 0.35) / count.max(1) as f32;
    base + phase
}
