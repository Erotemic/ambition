//! Tests for the rider/mount link: per-tick rider-to-mount snapping and the
//! mount-death dissolution that re-grounds and re-brains the rider.

use super::super::CenteredAabb;
use super::*;
use bevy::prelude::*;

type EnemyClusterBundle = (
    super::super::enemy_clusters::BodyKinematics,
    super::super::enemy_clusters::EnemyStatus,
    super::super::enemy_clusters::EnemyConfig,
    super::super::enemy_clusters::ActorMotionPath,
    crate::features::ActorSurfaceState,
    crate::features::ActorAttackState,
    crate::combat::CombatCapabilities,
);

fn hostile(
    id: &str,
    archetype_brain: &str,
    pos: ae::Vec2,
    size: ae::Vec2,
) -> (crate::features::ActorDisposition, EnemyClusterBundle) {
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = super::super::enemy_clusters::EnemyClusterSeed::new(
        id,
        id,
        aabb,
        crate::actor::EnemyBrain::Custom(archetype_brain.into()),
        &[],
    );
    enemy.kin.size = size;
    enemy.kin.pos = pos;
    enemy.status.alive = true;
    (
        crate::features::ActorDisposition::Hostile,
        enemy.into_components(),
    )
}

/// Read an entity's enemy kinematics/status/surface from its
/// cluster components for test assertions.
fn rider_kin(
    world: &bevy::prelude::World,
    e: bevy::prelude::Entity,
) -> super::super::enemy_clusters::BodyKinematics {
    *world
        .entity(e)
        .get::<super::super::enemy_clusters::BodyKinematics>()
        .expect("enemy entity has BodyKinematics")
}

fn rider_surface(
    world: &bevy::prelude::World,
    e: bevy::prelude::Entity,
) -> crate::features::ActorSurfaceState {
    *world
        .entity(e)
        .get::<crate::features::ActorSurfaceState>()
        .expect("enemy entity has ActorSurfaceState")
}

fn build_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app
}

/// Sync pose snaps rider.pos to mount.pos + Mountable.rider_offset
/// and zeroes the rider's velocity each tick.
#[test]
fn sync_riders_to_mounts_snaps_rider_to_mount_offset() {
    let mut app = build_app();
    app.add_systems(Update, sync_riders_to_mounts);

    let mount_pos = ae::Vec2::new(100.0, 50.0);
    let mount_size = ae::Vec2::new(126.0, 52.0);
    let mount = app
        .world_mut()
        .spawn((
            hostile("mount", "burning_flying_shark", mount_pos, mount_size),
            CenteredAabb::from_center_size(mount_pos, mount_size),
            Mountable {
                rider_offset: ae::Vec2::new(0.0, -40.0),
            },
            MountSlot { rider: None },
        ))
        .id();

    // Rider's authored position is something arbitrary; the sync
    // system should snap it to the mount's pos + offset on the
    // first tick.
    let rider_start = ae::Vec2::new(999.0, 999.0);
    let rider_size = ae::Vec2::new(44.0, 78.0);
    let rider = app
        .world_mut()
        .spawn((
            hostile("rider", "pirate_raider", rider_start, rider_size),
            CenteredAabb::from_center_size(rider_start, rider_size),
            RidingOn { mount },
        ))
        .id();
    // Pre-poison rider velocity so the assertion that the sync
    // zeroes it isn't a no-op against the default.
    app.world_mut()
        .get_mut::<crate::features::BodyKinematics>(rider)
        .unwrap()
        .vel = ae::Vec2::new(500.0, -200.0);
    app.world_mut()
        .get_mut::<crate::features::ActorSurfaceState>(rider)
        .unwrap()
        .gravity_scale = 1.0;

    app.update();

    let k = rider_kin(app.world(), rider);
    let s = rider_surface(app.world(), rider);
    assert_eq!(
        k.pos,
        ae::Vec2::new(100.0, 10.0),
        "rider should snap to mount.pos + offset",
    );
    assert_eq!(k.vel, ae::Vec2::ZERO, "rider vel zeroed by sync");
    assert_eq!(s.gravity_scale, 0.0, "rider gravity zeroed by sync");

    let aabb = app.world().entity(rider).get::<CenteredAabb>().unwrap();
    assert_eq!(
        aabb.center, k.pos,
        "CenteredAabb mirror updated to synced pos"
    );
}

/// Helper: spawn a mount + rider pair wired the same way the
/// composite-spawn helper does, but using a placeholder mounted
/// brain (Skirmisher with explicit cfg) so the cache check has
/// something concrete to compare against.
fn spawn_pair(app: &mut App, mount_alive: bool, rider_alive: bool) -> (Entity, Entity) {
    use crate::brain::{
        ActionSet, Brain, RangedActionSpec, SkirmisherCfg, SkirmisherState, StateMachineCfg,
    };
    let mounted_brain = Brain::StateMachine(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg {
            aggressiveness: 1.0,
            aggro_radius: 1200.0,
            standoff_px: 385.0,
            strafe_speed: 230.0,
            fire_cooldown_s: 1.5,
            orbit_drift_rad_s: 0.6,
        },
        state: SkirmisherState::default(),
    });
    let mounted_action_set = ActionSet {
        ranged: Some(RangedActionSpec::Bolt {
            speed: 500.0,
            damage: 2,
        }),
        ..Default::default()
    };
    let mount_pos = ae::Vec2::new(0.0, 0.0);
    let mount_size = ae::Vec2::new(126.0, 52.0);
    let mut mount_actor = hostile("mount", "burning_flying_shark", mount_pos, mount_size);
    // .1 = EnemyClusterBundle, .1.1 = EnemyStatus.
    mount_actor.1 .1.alive = mount_alive;
    let mount = app
        .world_mut()
        .spawn((
            mount_actor,
            Mountable {
                rider_offset: ae::Vec2::new(0.0, -40.0),
            },
            MountSlot { rider: None },
        ))
        .id();

    let rider_pos = ae::Vec2::new(0.0, -40.0);
    let rider_size = ae::Vec2::new(44.0, 78.0);
    let mut rider_actor = hostile("rider", "pirate_raider", rider_pos, rider_size);
    // .1.1 = EnemyStatus, .1.4 = ActorSurfaceState.
    rider_actor.1 .1.alive = rider_alive;
    rider_actor.1 .4.gravity_scale = 0.0;
    let rider = app
        .world_mut()
        .spawn((
            rider_actor,
            CenteredAabb::from_center_size(rider_pos, rider_size),
            mounted_brain.clone(),
            mounted_action_set.clone(),
            MountedBrainCache {
                brain: mounted_brain,
                action_set: mounted_action_set,
            },
            Mounted,
            RidingOn { mount },
        ))
        .id();
    app.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });
    (mount, rider)
}

/// Mount's death dissolves the link: rider's gravity flips on,
/// brain swaps to the solo PirateRaider Smash, and the Mounted
/// marker is removed. RidingOn + MountSlot stay attached so the
/// same-room reset path can re-arm the link without an id
/// lookup.
#[test]
fn dead_mount_dissolves_link_keeping_records() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (mount, rider) = spawn_pair(&mut app, /*mount_alive*/ false, true);

    app.update();

    assert!(
        app.world().entity(rider).get::<RidingOn>().is_some(),
        "RidingOn stays attached so reset can re-arm without id lookup",
    );
    assert!(
        app.world().entity(rider).get::<Mounted>().is_none(),
        "Mounted marker removed on dissolve",
    );
    assert_eq!(
        rider_surface(app.world(), rider).gravity_scale,
        1.0,
        "PirateRaider rider gets gravity 1.0"
    );
    let brain = app
        .world()
        .entity(rider)
        .get::<crate::brain::Brain>()
        .unwrap();
    assert!(
        matches!(
            brain,
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::MeleeBrute { .. }),
        ),
        "after dismount the rider should be MeleeBrute (explicit chase + swipe)",
    );
    let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
    assert!(
        slot.rider.is_some(),
        "MountSlot.rider stays populated so reset can re-arm",
    );
}

/// Same-room reset re-arms the link: starting from a dissolved
/// state (mount dead, rider with solo brain), once the mount's
/// `alive` flag is set back to true the enforcer restores the
/// MOUNTED brain + Mounted marker + zero gravity on the rider.
#[test]
fn reviving_mount_re_arms_rider_to_mounted_brain() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (mount, rider) = spawn_pair(&mut app, /*mount_alive*/ false, true);

    // First tick: dissolve.
    app.update();
    assert!(app.world().entity(rider).get::<Mounted>().is_none());

    // Simulate the same-room reset: flip mount.alive back to
    // true (reset_to_spawn would do this). The enforcer should
    // re-arm the link on the next tick.
    app.world_mut()
        .get_mut::<crate::features::EnemyStatus>(mount)
        .unwrap()
        .alive = true;
    app.update();

    assert!(
        app.world().entity(rider).get::<Mounted>().is_some(),
        "Mounted marker should be re-added on revive",
    );
    assert_eq!(
        rider_surface(app.world(), rider).gravity_scale,
        0.0,
        "rider gravity should be zeroed back to mounted state",
    );
    let brain = app
        .world()
        .entity(rider)
        .get::<crate::brain::Brain>()
        .unwrap();
    assert!(
        matches!(
            brain,
            crate::brain::Brain::StateMachine(crate::brain::StateMachineCfg::Skirmisher { .. }),
        ),
        "after revive the rider's mounted brain (Skirmisher) should be restored",
    );
}

/// Dead rider leaves the link records in place — the mount keeps
/// its MountSlot back-reference (no re-arming needed since the
/// rider's just dead, not transitioning). Mount stays alive.
#[test]
fn dead_rider_does_not_disturb_mount_records() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (mount, _rider) = spawn_pair(
        &mut app, /*mount_alive*/ true, /*rider_alive*/ false,
    );

    app.update();

    let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
    assert!(
        slot.rider.is_some(),
        "MountSlot.rider stays populated even with dead rider",
    );
    assert!(
        app.world()
            .entity(mount)
            .get::<crate::features::EnemyStatus>()
            .unwrap()
            .alive,
        "mount stays alive when rider dies"
    );
}
