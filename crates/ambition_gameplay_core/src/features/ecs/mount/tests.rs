//! Tests for the rider/mount link: per-tick rider-to-mount snapping and the
//! mount-death dissolution that re-grounds and re-brains the rider.

use super::super::CenteredAabb;
use super::*;
use bevy::prelude::*;

type ActorClusterBundle = (
    super::super::actor_clusters::BodyKinematics,
    super::super::actor_clusters::ActorStatus,
    ambition_characters::actor::BodyHealth,
    super::super::actor_clusters::ActorConfig,
    super::super::actor_clusters::ActorMotionPath,
    crate::features::ActorSurfaceState,
    crate::features::BodyMelee,
    crate::actor::AncillaryMovementBundle,
    crate::combat::CombatCapabilities,
);

fn hostile(
    id: &str,
    archetype_brain: &str,
    pos: ae::Vec2,
    size: ae::Vec2,
) -> (crate::features::ActorDisposition, ActorClusterBundle) {
    let aabb = ae::Aabb::new(pos, size * 0.5);
    let mut enemy = super::super::actor_clusters::ActorClusterSeed::new(
        id,
        id,
        aabb,
        ambition_characters::actor::CharacterBrain::Custom(archetype_brain.into()),
        &[],
    );
    enemy.kin.size = size;
    enemy.kin.pos = pos;
    enemy.health.reset();
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
) -> super::super::actor_clusters::BodyKinematics {
    *world
        .entity(e)
        .get::<super::super::actor_clusters::BodyKinematics>()
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
            Mountable::at(ae::Vec2::new(0.0, -40.0)),
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
    use ambition_characters::brain::{
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
    // .1 = ActorClusterBundle; BodyHealth (the liveness authority) is at .1.2.
    if !mount_alive {
        mount_actor.1 .2.health.current = 0;
    }
    let mount = app
        .world_mut()
        .spawn((
            mount_actor,
            Mountable::at(ae::Vec2::new(0.0, -40.0)),
            MountSlot { rider: None },
        ))
        .id();

    let rider_pos = ae::Vec2::new(0.0, -40.0);
    let rider_size = ae::Vec2::new(44.0, 78.0);
    let mut rider_actor = hostile("rider", "pirate_raider", rider_pos, rider_size);
    // BodyHealth (liveness) at .1.2; ActorSurfaceState at .1.5.
    if !rider_alive {
        rider_actor.1 .2.health.current = 0;
    }
    rider_actor.1 .5.gravity_scale = 0.0;
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
        .get::<ambition_characters::brain::Brain>()
        .unwrap();
    assert!(
        matches!(
            brain,
            ambition_characters::brain::Brain::StateMachine(
                ambition_characters::brain::StateMachineCfg::MeleeBrute { .. }
            ),
        ),
        "after dismount the rider should be MeleeBrute (explicit chase + swipe)",
    );
    let slot = app.world().entity(mount).get::<MountSlot>().unwrap();
    assert!(
        slot.rider.is_some(),
        "MountSlot.rider stays populated so reset can re-arm",
    );
}

/// ADR 0020 pilot-compatibility: a rider may only pilot mount classes
/// its `CanPilot` set lists. A shark-rider carries `["shark"]` and can
/// board a shark but not a mech.
#[test]
fn can_pilot_matches_authored_classes() {
    let rider = CanPilot {
        classes: vec![MountClass("shark".into())],
    };
    assert!(
        rider.can_pilot(&MountClass("shark".into())),
        "a shark-rider can pilot a shark-class mount",
    );
    assert!(
        !rider.can_pilot(&MountClass("mech".into())),
        "a shark-rider cannot pilot a mech-class mount",
    );
}

/// ADR 0020 default: a mount extends its rider `ControlGrant::Total` and,
/// unless authored otherwise, drops the rider unharmed on death.
#[test]
fn mountable_defaults_are_total_control_and_clean_dismount() {
    let m = Mountable::at(ae::Vec2::ZERO);
    assert_eq!(m.control_grant, ControlGrant::Total);
    assert_eq!(m.death_impact, MountDeathImpact::Dismount);
}

/// Spawn a dead mount carrying `death_impact` + a live mounted rider,
/// mirroring `spawn_pair` but letting the caller set the mount's impact.
fn spawn_dead_mount_with_impact(app: &mut App, death_impact: MountDeathImpact) -> (Entity, Entity) {
    let mount_pos = ae::Vec2::new(0.0, 0.0);
    let mount_size = ae::Vec2::new(126.0, 52.0);
    let mut mount_actor = hostile("mount", "burning_flying_shark", mount_pos, mount_size);
    mount_actor.1 .2.health.current = 0; // mount dead → dissolution fires
    let mut mountable = Mountable::at(ae::Vec2::new(0.0, -40.0));
    mountable.death_impact = death_impact;
    let mount = app
        .world_mut()
        .spawn((mount_actor, mountable, MountSlot { rider: None }))
        .id();

    let rider_pos = ae::Vec2::new(0.0, -40.0);
    let rider_size = ae::Vec2::new(44.0, 78.0);
    let mut rider_actor = hostile("rider", "pirate_raider", rider_pos, rider_size);
    rider_actor.1 .5.gravity_scale = 0.0;
    // Force a known 5-HP pool so splash arithmetic is deterministic
    // regardless of what the seed default resolves to in a minimal test.
    rider_actor.1 .2 = ambition_characters::actor::BodyHealth::new(
        ambition_characters::actor::Health::new(RIDER_TEST_HP),
    );
    let rider = app
        .world_mut()
        .spawn((
            rider_actor,
            CenteredAabb::from_center_size(rider_pos, rider_size),
            Mounted,
            RidingOn { mount },
        ))
        .id();
    app.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });
    (mount, rider)
}

const RIDER_TEST_HP: i32 = 5;

/// ADR 0020 resolution: an authored `(rider_id, mount_id)` link resolves into a
/// live RidingOn/MountSlot once both actors carry the matching `FeatureId`, the
/// rider `CanPilot`s the mount's class, and the mount is `Mountable`.
#[test]
fn resolve_pending_mount_links_links_a_compatible_pair() {
    use crate::combat::components::FeatureId;

    let mut app = build_app();
    app.add_systems(Update, resolve_pending_mount_links);
    let mount = app
        .world_mut()
        .spawn((
            FeatureId::new("shark_1"),
            {
                let mut m = Mountable::at(ae::Vec2::new(0.0, -40.0));
                m.class = MountClass("shark".into());
                m
            },
            MountSlot { rider: None },
        ))
        .id();
    let rider = app
        .world_mut()
        .spawn((
            FeatureId::new("rider_1"),
            CanPilot {
                classes: vec![MountClass("shark".into())],
            },
        ))
        .id();
    app.insert_resource(PendingMountLinks(vec![(
        "rider_1".into(),
        "shark_1".into(),
    )]));

    app.update();

    assert_eq!(
        app.world().entity(rider).get::<RidingOn>().map(|r| r.mount),
        Some(mount),
        "the rider should now ride the named mount",
    );
    assert!(
        app.world().entity(rider).get::<Mounted>().is_some(),
        "the rider is marked Mounted",
    );
    assert_eq!(
        app.world()
            .entity(mount)
            .get::<MountSlot>()
            .and_then(|s| s.rider),
        Some(rider),
        "the mount's MountSlot points back at the rider",
    );
    assert!(
        app.world().resource::<PendingMountLinks>().0.is_empty(),
        "the resolved link is drained from the pending set",
    );
}

/// ADR 0020: a rider that cannot pilot the mount's class is NOT linked — the
/// pilot-compatibility check drops the illegal pairing.
#[test]
fn resolve_pending_mount_links_rejects_an_incompatible_class() {
    use crate::combat::components::FeatureId;

    let mut app = build_app();
    app.add_systems(Update, resolve_pending_mount_links);
    let _mount = app
        .world_mut()
        .spawn((
            FeatureId::new("mech_1"),
            {
                let mut m = Mountable::at(ae::Vec2::ZERO);
                m.class = MountClass("mech".into());
                m
            },
            MountSlot { rider: None },
        ))
        .id();
    let rider = app
        .world_mut()
        .spawn((
            FeatureId::new("rider_1"),
            CanPilot {
                classes: vec![MountClass("shark".into())],
            },
        ))
        .id();
    app.insert_resource(PendingMountLinks(vec![("rider_1".into(), "mech_1".into())]));

    app.update();

    assert!(
        app.world().entity(rider).get::<RidingOn>().is_none(),
        "a shark-rider must not be linked to a mech-class mount",
    );
}

fn rider_health(world: &bevy::prelude::World, e: Entity) -> ambition_characters::actor::BodyHealth {
    *world
        .entity(e)
        .get::<ambition_characters::actor::BodyHealth>()
        .expect("rider has BodyHealth")
}

/// ADR 0020: a non-lethal mount `death_impact: Splash(n)` subtracts `n`
/// from the rider's separate HP pool on the death transition, then the
/// rider still dismounts (gravity on, Mounted removed).
#[test]
fn nonlethal_mount_death_splash_damages_the_rider_then_dismounts() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (_mount, rider) = spawn_dead_mount_with_impact(&mut app, MountDeathImpact::Splash(2));

    app.update();

    assert_eq!(
        rider_health(app.world(), rider).current(),
        RIDER_TEST_HP - 2,
        "a Splash(2) mount death should take 2 off the rider's HP",
    );
    assert!(
        rider_health(app.world(), rider).alive(),
        "5-HP rider survives a 2-damage splash",
    );
    assert!(
        app.world().entity(rider).get::<Mounted>().is_none(),
        "surviving rider still dismounts (Mounted removed)",
    );
    assert_eq!(
        rider_surface(app.world(), rider).gravity_scale,
        1.0,
        "surviving rider falls off the dead mount",
    );
}

/// ADR 0020: a lethal `death_impact: Splash(n)` (mech explosion) kills the
/// rider — its HP pool drops to non-alive and no solo brain is installed.
#[test]
fn lethal_mount_death_splash_kills_the_rider() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (_mount, rider) = spawn_dead_mount_with_impact(&mut app, MountDeathImpact::Splash(99));

    app.update();

    assert!(
        !rider_health(app.world(), rider).alive(),
        "a lethal splash (mech explosion) kills the rider too",
    );
}

/// ADR 0020: the default `Dismount` impact leaves the rider's HP intact —
/// a dead shark drops its rider unharmed.
#[test]
fn dismount_impact_leaves_rider_hp_intact() {
    let mut app = build_app();
    app.add_systems(Update, enforce_mount_rider_link);
    let (_mount, rider) = spawn_dead_mount_with_impact(&mut app, MountDeathImpact::Dismount);

    app.update();

    assert_eq!(
        rider_health(app.world(), rider).current(),
        RIDER_TEST_HP,
        "a clean dismount takes no HP from the rider",
    );
    assert!(app.world().entity(rider).get::<Mounted>().is_none());
}

/// ADR 0020 control routing: with the default `Total` grant, the rider's
/// brain locomotion intent is copied onto the mount (the orbit lives on the
/// rider), while attack/fire intent is NOT copied — the rider fires from the
/// saddle. Runs `steer_mount_from_rider` directly on hand-built control
/// frames so the assertion is about the routing, not the whole brain tick.
#[test]
fn total_grant_routes_rider_locomotion_to_mount_but_not_fire() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::ActorControl;

    let mut app = build_app();
    app.add_systems(Update, steer_mount_from_rider);

    let mount = app
        .world_mut()
        .spawn((
            Mountable::at(ae::Vec2::new(0.0, -40.0)),
            MountSlot { rider: None },
            ActorControl(ActorControlFrame::neutral()),
        ))
        .id();

    let mut rider_frame = ActorControlFrame::neutral();
    rider_frame.velocity_target = ae::Vec2::new(120.0, -30.0);
    rider_frame.locomotion = ae::Vec2::new(1.0, 0.0);
    rider_frame.facing = -1.0;
    rider_frame.fire = Some(
        ambition_characters::actor::control::ActorFireRequest::world_space(
            ae::Vec2::new(1.0, 0.0),
            100.0,
        ),
    );
    let rider = app
        .world_mut()
        .spawn((Mounted, RidingOn { mount }, ActorControl(rider_frame)))
        .id();
    app.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });

    app.update();

    let mount_frame = app.world().entity(mount).get::<ActorControl>().unwrap().0;
    assert_eq!(
        mount_frame.velocity_target,
        ae::Vec2::new(120.0, -30.0),
        "Total grant copies the rider's velocity_target onto the mount",
    );
    assert_eq!(mount_frame.facing, -1.0, "and the rider's facing");
    assert!(
        mount_frame.fire.is_none(),
        "but the mount does NOT inherit the rider's fire intent — the rider fires",
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

    // Simulate the same-room reset: restore the mount's HP (liveness
    // authority) the way reset_to_spawn would. The enforcer should
    // re-arm the link on the next tick.
    app.world_mut()
        .get_mut::<ambition_characters::actor::BodyHealth>(mount)
        .unwrap()
        .reset();
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
        .get::<ambition_characters::brain::Brain>()
        .unwrap();
    assert!(
        matches!(
            brain,
            ambition_characters::brain::Brain::StateMachine(
                ambition_characters::brain::StateMachineCfg::Skirmisher { .. }
            ),
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
            .get::<ambition_characters::actor::BodyHealth>()
            .unwrap()
            .alive(),
        "mount stays alive when rider dies"
    );
}
