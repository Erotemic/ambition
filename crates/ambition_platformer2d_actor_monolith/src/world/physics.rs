//! Avian2D-backed secondary physics for Ambition sandbox props.
//!
//! The player controller remains custom/kinematic. Avian owns only secondary
//! bodies for now: room colliders, breakable shards, defeated enemy pieces, and
//! other ragdoll-like effects. This gives us real physical motion where it adds
//! juice without surrendering platforming feel. A future physics-player mode can
//! be added behind the same boundary.

use ambition_platformer2d_core as ae;
#[cfg(feature = "physics_debris")]
use ambition_platformer2d_core::AabbExt;
#[cfg(feature = "physics_debris")]
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionSpawnScope, SpawnSessionScopedExt,
};
use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
#[cfg(feature = "physics_debris")]
use avian2d::prelude::*;
#[cfg(feature = "physics_debris")]
use bevy::math::Vec2 as BVec2;
use bevy::prelude::*;

#[cfg(feature = "physics_debris")]
use ambition_platformer2d_core::config::{world_to_bevy, WORLD_Z_BLOCK, WORLD_Z_FX};
#[cfg(feature = "physics_debris")]
use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;

#[cfg(feature = "physics_debris")]
const SANDBOX_GRAVITY: f32 = 1250.0;
#[cfg(feature = "physics_debris")]
const STATIC_COLLIDER_Z: f32 = WORLD_Z_BLOCK - 1.0;
#[cfg(feature = "physics_debris")]
const DEBRIS_Z: f32 = WORLD_Z_FX - 2.0;
#[cfg(feature = "physics_debris")]
const PHYSICS_DESPAWN_GRACE: f32 = 0.25;

/// Marker for room-owned Avian entities so room transitions can retire them
/// through the physics-safe path instead of despawning active bodies immediately.
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsRoomEntity;

/// A body that has been disabled and hidden, but not yet despawned.
#[cfg(feature = "physics_debris")]
#[derive(Component, Clone, Copy, Debug)]
pub struct PendingPhysicsDespawn {
    pub timer: f32,
}

/// A static Avian collider that mirrors a block of one live room: the floor
/// and the walls that debris of that room lands on.
#[cfg(feature = "physics_debris")]
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsRoomFloor;

/// Ephemeral Avian dynamic body spawned from breakables, defeated enemies, and
/// impact effects.
#[cfg(feature = "physics_debris")]
#[derive(Component, Clone, Copy, Debug)]
pub struct PhysicsDebris {
    pub lifetime: f32,
}

// The Avian subscriber below stays here (the adapter half).
#[cfg(any(test, feature = "physics_debris"))]
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};

/// Presentation-side subscriber. Reads `DebrisBurstMessage`s and spawns
/// Avian2D debris bodies via the existing `spawn_debris_burst` helper.
/// Skipped in headless builds.
///
/// Each burst is placed by the geometry of the live room its message names,
/// and each piece carries that room's stamp, so the room retires its own
/// debris. A message that names no room is thrown in the sole live room, and
/// is dropped while two rooms are live.
///
/// The first burst in a live room also builds that room's floor: a static
/// collider for each of its blocks, with the room's stamp, so the room
/// retires its floor with it and `OneRoomContacts` keeps another room's debris
/// off it. It is built on the first burst and not when the room goes live,
/// because a static body keeps avian awake, and a room with no debris must
/// not pay for the physics engine (`pause_physics_when_no_debris_exists`).
#[cfg(feature = "physics_debris")]
#[allow(clippy::too_many_arguments)]
pub fn physics_spawn_debris_messages(
    mut commands: Commands,
    mut messages: MessageReader<DebrisBurstMessage>,
    geometry: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<
        ambition_platformer2d_core::RoomGeometry,
    >,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    settings: Res<PhysicsSandboxSettings>,
    active_session: Option<Res<ActiveSessionScope>>,
    floors: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance, With<PhysicsRoomFloor>>,
) {
    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        messages.clear();
        return;
    };
    // The floors built this frame: their spawns are not applied yet, so the
    // query does not see them.
    let mut floored: Vec<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance> = Vec::new();
    for message in messages.read() {
        let Some(room) = message.room.or_else(|| rooms.sole()) else {
            continue;
        };
        let Some(world) = geometry.in_room(room) else {
            continue;
        };
        if !floored.contains(&room) && !floors.iter().any(|stamp| stamp.0 == room) {
            floored.push(room);
            for block in &world.0.blocks {
                spawn_static_collider_for_block(
                    &mut commands,
                    session_scope.in_room(Some(room)),
                    &world.0,
                    block,
                    *settings,
                );
            }
        }
        spawn_debris_burst(
            &mut commands,
            session_scope.in_room(Some(room)),
            &world.0,
            message.pos,
            message.cue,
            *settings,
        );
    }
}

/// Two colliders meet only when they are in one live room.
///
/// Avian has one space, and the live rooms are placed in it by their own
/// geometry, so two live rooms overlap there. Without this, debris of Bob's
/// room bounced on debris of Alice's room at the same position. A collider
/// with no room stamp is in every room's world, as unscoped work is in every
/// session's (`TransactionRooms`).
#[cfg(feature = "physics_debris")]
#[derive(bevy::ecs::system::SystemParam)]
pub struct OneRoomContacts<'w, 's> {
    stamps: Query<'w, 's, &'static ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
}

#[cfg(feature = "physics_debris")]
impl avian2d::collision::hooks::CollisionHooks for OneRoomContacts<'_, '_> {
    fn filter_pairs(&self, collider1: Entity, collider2: Entity, _commands: &mut Commands) -> bool {
        match (self.stamps.get(collider1), self.stamps.get(collider2)) {
            (Ok(first), Ok(second)) => first.0 == second.0,
            _ => true,
        }
    }
}

/// Pause avian while nothing it owns exists, and unpause the moment something does.
///
/// The predicate is debris presence: debris is the only dynamic population
/// avian has here, so "no debris" means "nothing for the solver to solve". A
/// room's floor (`PhysicsRoomFloor`) is a static body and does not count.
/// Ambition's own
/// bodies are not avian bodies — they live in `GgrsSchedule` and never appear in
/// this query.
#[cfg(feature = "physics_debris")]
fn pause_physics_when_no_debris_exists(
    // Debris only: a room's floor is a static body that stays for the life
    // of its room, and it has nothing to solve without debris on it.
    debris: Query<(), With<PhysicsDebris>>,
    mut physics_time: ResMut<Time<avian2d::schedule::Physics>>,
) {
    use avian2d::schedule::PhysicsTime as _;
    let wanted = debris.iter().next().is_some();
    if wanted == physics_time.is_paused() {
        // ⛔ Only on a CHANGE. `ResMut` marks its resource changed on DEREF, so
        // touching it every frame would announce a change that did not happen to
        // every reader — the exact defect fixed in the hot-reload watcher.
        if wanted {
            physics_time.unpause();
        } else {
            physics_time.pause();
        }
    }
}

#[cfg(feature = "physics_debris")]
pub struct AmbitionPhysicsPlugin;

#[cfg(feature = "physics_debris")]
impl Plugin for AmbitionPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsSandboxSettings::default())
            .insert_resource(Gravity(BVec2::new(0.0, -SANDBOX_GRAVITY)))
            .add_plugins(PhysicsPlugins::default().with_collision_hooks::<OneRoomContacts>())
            // ⛔⛔ AVIAN REGISTERS THESE IN `Plugin::finish`, AND `App::update()`
            // NEVER CALLS `finish()` — only `App::run()` does. So every
            // composition driven by an update loop (the whole test suite, the
            // no-window render recipe, the headless hosts) reaches avian's
            // systems with the diagnostics resources ABSENT, and a system taking
            // `ResMut<...Diagnostics>` fails param validation with "Resource does
            // not exist" — which Bevy 0.19 turns into a panic that unwinds the
            // whole fixed-main schedule. Explicit init is idempotent and is the
            // only thing standing between that and a dozen unrelated red tests.
            //
            // ⚠ THIS LIST GOES STALE ON AN AVIAN BUMP. avian 0.7 added a fourth
            // (`ColliderTreeDiagnostics`, for the new collider-tree broad phase)
            // and the symptom was twelve app tests failing inside
            // `update_moved_collider_aabbs`, none of which is about physics. If
            // an avian upgrade reddens tests with that message, the fix is
            // another line here: grep avian for `register_physics_diagnostics`.
            .init_resource::<avian2d::collision::CollisionDiagnostics>()
            .init_resource::<avian2d::dynamics::solver::SolverDiagnostics>()
            .init_resource::<avian2d::spatial_query::SpatialQueryDiagnostics>()
            .init_resource::<avian2d::collider_tree::ColliderTreeDiagnostics>()
            .add_systems(
                Update,
                (
                    update_physics_debris_lifetimes,
                    complete_pending_physics_despawns,
                    // ⭐⭐ PAUSE AVIAN WHILE THERE IS NO DEBRIS. This plugin is
                    // gated on a feature literally called `physics_debris`, and
                    // it installs `PhysicsPlugins::default()` — the WHOLE of
                    // avian2d. Tracy on a Smash match, 2026-08-29, found its
                    // schedules running every fixed step with no debris in the
                    // world at all: `PhysicsSchedule` plus ~6.4 `SubstepSchedule`
                    // steps per frame, while Ambition's own collision runs
                    // separately in `GgrsSchedule`.
                    //
                    // ⚠ SIZED BEFORE IT WAS WRITTEN: ~40–60us/frame after the
                    // measured 2.4x Tracy inflation, about 1% and BELOW this
                    // machine's noise floor. ⛔ NO frame-time claim is made. It
                    // is here because it is the brief's invariant stated
                    // exactly — *a capability installed but dormant should cost
                    // very little* — and a cosmetic effect paying for a physics
                    // engine is that invariant broken.
                    //
                    // ⭐ PAUSE, NOT A SCHEDULE GATE, on purpose: pausing is
                    // avian's own supported API (`PhysicsTime::pause`), so its
                    // schedules still run their bookkeeping and a body spawned
                    // this frame is initialised normally — it simply does not
                    // step. Skipping the schedules outright would be reaching
                    // past the library's contract to save the same microseconds.
                    pause_physics_when_no_debris_exists,
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
    // Disabling the physics of a body that no longer exists is the outcome this wants.
    commands.entity(entity).try_insert((
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
    session_scope: SessionSpawnScope,
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
    commands.spawn_session_scoped(
        session_scope,
        (
            RigidBody::Static,
            Collider::rectangle(size.x, size.y),
            Transform::from_translation(world_to_bevy(
                world,
                block.aabb.center(),
                STATIC_COLLIDER_Z,
            )),
            Name::new(format!("Physics collider: {}", block.name)),
            RoomVisual,
            PhysicsRoomEntity,
            PhysicsRoomFloor,
            // See `OneRoomContacts`.
            ActiveCollisionHooks::FILTER_PAIRS,
        ),
    );
}

#[cfg(not(feature = "physics_debris"))]
pub fn spawn_static_collider_for_block(
    _commands: &mut Commands,
    _session_scope: ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope,
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
    session_scope: SessionSpawnScope,
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
            session_scope,
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
    session_scope: SessionSpawnScope,
    world: &ae::World,
    pos: ae::Vec2,
    size: BVec2,
    velocity: BVec2,
    angular_velocity: f32,
    color: Color,
    lifetime: f32,
) {
    commands.spawn_session_scoped(
        session_scope,
        (
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
            // See `OneRoomContacts`.
            ActiveCollisionHooks::FILTER_PAIRS,
        ),
    );
}

#[cfg(feature = "physics_debris")]
fn block_accepts_dynamic_debris(kind: ae::BlockKind) -> bool {
    ae::collision_semantics::is_support_surface(kind)
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

    /// Each debris burst is thrown in the live room its message names. Two
    /// live rooms have geometry of two heights, and one burst is asked for in
    /// each at one position. Each piece carries the stamp of its own room and
    /// is placed by the geometry of that room. A third burst names no room,
    /// and it is dropped while two rooms are live. When the reader took the
    /// sole live room's geometry, it did not run while two rooms were live.
    ///
    /// Two debris pieces overlap at one position, with no gravity. In one
    /// live room they collide and are pushed apart (the control); in two live
    /// rooms they do not meet and stay where they are. Avian has one space,
    /// and two live rooms overlap in it.
    ///
    /// ⚠ Built only with the `physics_debris` feature, as the hook is.
    #[cfg(feature = "physics_debris")]
    #[test]
    fn debris_of_two_live_rooms_does_not_meet() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let apart_after = |second_room: LiveRoomInstance| -> f32 {
            let mut app = App::new();
            app.add_plugins((
                bevy::MinimalPlugins,
                bevy::transform::TransformPlugin,
                bevy::asset::AssetPlugin::default(),
                bevy::mesh::MeshPlugin,
                AmbitionPhysicsPlugin,
            ));
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ));
            app.insert_resource(Gravity(BVec2::ZERO));
            app.finish();
            app.cleanup();
            let piece = |room: LiveRoomInstance, x: f32| {
                (
                    Transform::from_xyz(x, 0.0, 0.0),
                    RigidBody::Dynamic,
                    Collider::rectangle(8.0, 8.0),
                    ActiveCollisionHooks::FILTER_PAIRS,
                    InRoomInstance(room),
                    // Debris, as production's pieces are: avian runs only
                    // while debris exists.
                    PhysicsDebris { lifetime: 10.0 },
                )
            };
            let first = app.world_mut().spawn(piece(LiveRoomInstance::ACTIVATION, 0.0)).id();
            let second = app.world_mut().spawn(piece(second_room, 1.0)).id();
            for _ in 0..30 {
                app.update();
            }
            let at = |entity| app.world().get::<Transform>(entity).expect("a debris piece").translation.x;
            (at(second) - at(first)).abs()
        };
        let one_room = apart_after(LiveRoomInstance::ACTIVATION);
        let two_rooms = apart_after(LiveRoomInstance::ACTIVATION.next());
        assert!(
            one_room > 2.0 && (two_rooms - 1.0).abs() < 1e-3,
            "(the pieces' distance after 30 steps in one live room, in two live rooms) = \
             ({one_room}, {two_rooms}); in one room they push apart, in two they stay 1.0 apart"
        );
    }

    /// ⚠ This witness is built only with the `physics_debris` feature, as
    /// the reader is.
    #[cfg(feature = "physics_debris")]
    #[test]
    fn each_debris_burst_is_thrown_in_the_live_room_its_message_names() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
        };
        let mut app = App::new();
        app.insert_resource(PhysicsSandboxSettings::default());
        app.add_message::<DebrisBurstMessage>();
        app.add_systems(Update, physics_spawn_debris_messages);
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for (room, height) in rooms.into_iter().zip([600.0, 1000.0]) {
            app.world_mut().spawn((
                RoomInstanceRoot,
                room,
                ambition_platformer2d_core::RoomGeometry(ae::World::new(
                    "room",
                    ae::Vec2::new(1000.0, height),
                    ae::Vec2::ZERO,
                    Vec::new(),
                )),
            ));
        }
        let pos = ae::Vec2::new(100.0, 100.0);
        let cue = PhysicsDebrisCue::Impact;
        for room in [Some(rooms[0]), Some(rooms[1]), None] {
            app.world_mut().write_message(DebrisBurstMessage { room, pos, cue });
        }
        app.update();
        let mut pieces = app
            .world_mut()
            .query_filtered::<(Option<&InRoomInstance>, &Transform, Option<&ActiveCollisionHooks>), With<PhysicsDebris>>();
        let thrown: std::collections::BTreeSet<_> = pieces
            .iter(app.world())
            .map(|(stamp, at, hooks)| {
                (
                    stamp.map(|stamp| stamp.0),
                    at.translation.y as i32,
                    hooks.is_some_and(|hooks| hooks.contains(ActiveCollisionHooks::FILTER_PAIRS)),
                )
            })
            .collect();
        assert_eq!(
            thrown,
            [(Some(rooms[0]), 200, true), (Some(rooms[1]), 400, true)].into_iter().collect(),
            "(the room each debris piece is stamped for, its height on screen, whether \
             `OneRoomContacts` filters its contacts): one burst in each of two live rooms at one \
             position, and one burst that names no room"
        );
    }

    /// Debris lands on the floor of its own live room. The first burst in a
    /// room builds that room's floor (a static collider per block, with the
    /// room's stamp), so a piece thrown above the floor is still above it a
    /// second later. Before, no system built the floor and every piece fell
    /// out of the room. The control is a room that authors no floor: its
    /// pieces fall past where the floor would be. A second burst in the same
    /// room builds no second floor.
    ///
    /// ⚠ Built only with the `physics_debris` feature, as the reader is.
    #[cfg(feature = "physics_debris")]
    #[test]
    fn debris_lands_on_the_floor_of_its_own_live_room() {
        use ambition_platformer2d_shared_tangle::lifecycle::{
            InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
        };
        const FLOOR_TOP: f32 = 500.0;
        let after = |floor: bool| -> (f32, usize, Vec<Option<LiveRoomInstance>>, bool) {
            let mut app = App::new();
            app.add_plugins((
                bevy::MinimalPlugins,
                bevy::transform::TransformPlugin,
                bevy::asset::AssetPlugin::default(),
                bevy::mesh::MeshPlugin,
                AmbitionPhysicsPlugin,
            ));
            app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ));
            app.add_message::<DebrisBurstMessage>();
            app.add_systems(Update, physics_spawn_debris_messages);
            app.finish();
            app.cleanup();
            let blocks = if floor {
                vec![ae::Block::solid("floor", ae::Vec2::new(0.0, FLOOR_TOP), ae::Vec2::new(1000.0, 100.0))]
            } else {
                Vec::new()
            };
            let world = ae::World::new("room", ae::Vec2::new(1000.0, 600.0), ae::Vec2::ZERO, blocks);
            let room = LiveRoomInstance::ACTIVATION;
            app.world_mut().spawn((
                RoomInstanceRoot,
                room,
                ambition_platformer2d_core::RoomGeometry(world.clone()),
            ));
            for _ in 0..2 {
                app.world_mut().write_message(DebrisBurstMessage {
                    room: Some(room),
                    pos: ae::Vec2::new(500.0, 400.0),
                    cue: PhysicsDebrisCue::Impact,
                });
            }
            for _ in 0..60 {
                app.update();
            }
            let floor_top = world_to_bevy(&world, ae::Vec2::new(500.0, FLOOR_TOP), 0.0).y;
            let lowest = app
                .world_mut()
                .query_filtered::<&Transform, With<PhysicsDebris>>()
                .iter(app.world())
                .map(|at| at.translation.y - floor_top)
                .fold(f32::INFINITY, f32::min);
            let stamps = app
                .world_mut()
                .query_filtered::<Option<&InRoomInstance>, With<PhysicsRoomFloor>>()
                .iter(app.world())
                .map(|stamp| stamp.map(|stamp| stamp.0))
                .collect::<Vec<_>>();
            // The debris lives 1.8 s. When it is gone, avian pauses again,
            // with the floor still there.
            for _ in 0..180 {
                app.update();
            }
            use avian2d::schedule::PhysicsTime as _;
            let paused = app.world().resource::<Time<avian2d::schedule::Physics>>().is_paused();
            (lowest, stamps.len(), stamps, paused)
        };
        let (lowest, floors, _, _) = after(false);
        assert!(
            lowest < -50.0 && floors == 0,
            "control: with no floor authored the lowest piece is {lowest} from where the floor \
             would be, and {floors} floors were built"
        );
        let (lowest, floors, stamps, paused) = after(true);
        assert!(
            lowest > -8.0,
            "the lowest piece is {lowest} from the floor's top, a second after it was thrown \
             above it: debris fell through the floor of its room"
        );
        assert_eq!(
            (floors, stamps),
            (1, vec![Some(LiveRoomInstance::ACTIVATION)]),
            "one floor collider for the room's one block, stamped with the room, for two bursts"
        );
        assert!(paused, "the debris is gone and avian still runs: the room's floor keeps it awake");
    }

}
