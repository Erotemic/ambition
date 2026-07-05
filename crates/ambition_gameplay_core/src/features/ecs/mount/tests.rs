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
    // `enforce_mount_rider_link` emits `MountDied` on dissolution; register the
    // message so its `MessageWriter` resolves in the harness (Q19).
    app.add_message::<MountDied>();
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

/// Q19b (ADR 0020): a rider whose identity is AUTHORED — it carries
/// `BossConfig` — keeps its `Brain` untouched on dismount (its behavior is not
/// derived from a kit, so re-deriving it would be wrong). It still re-grounds
/// (gravity on, `Mounted` removed) and emits `MountDied`, but lands on foot
/// still running its authored brain — gnuton stepping off his dead giant.
#[test]
fn boss_rider_keeps_its_brain_and_emits_mount_died_on_dismount() {
    use ambition_characters::brain::{Brain, PlayerSlot};

    #[derive(Resource, Default)]
    struct MountDiedLog(Vec<(Entity, Entity)>);
    fn log_mount_died(
        mut reader: bevy::prelude::MessageReader<MountDied>,
        mut log: ResMut<MountDiedLog>,
    ) {
        for ev in reader.read() {
            log.0.push((ev.mount, ev.rider));
        }
    }

    let mut app = build_app();
    app.init_resource::<MountDiedLog>();
    app.add_systems(Update, (enforce_mount_rider_link, log_mount_died).chain());

    // A dead mount + a live mounted rider (default `Dismount` impact).
    let (mount, rider) = spawn_dead_mount_with_impact(&mut app, MountDeathImpact::Dismount);
    // Make the rider a BOSS: an authored `BossConfig` marker + a distinctive
    // `Brain::Player` marker. The dismount rebuild would produce a
    // `Brain::StateMachine`, so a surviving `Player` proves the brain is
    // untouched — no new flag, the component IS the marker (Q19b).
    app.world_mut().entity_mut(rider).insert((
        crate::features::BossConfig {
            id: "boss_rider".into(),
            name: "Boss Rider".into(),
            spawn: ae::Vec2::ZERO,
            brain: ambition_characters::actor::BossBrain::Dormant,
            behavior: crate::features::BossBehaviorProfile::generic("boss_rider"),
        },
        Brain::Player(PlayerSlot(0)),
    ));

    app.update();

    // Brain untouched: still the authored `Player` marker, not a rebuilt
    // solo-melee `StateMachine`.
    assert!(
        matches!(
            app.world().entity(rider).get::<Brain>().unwrap(),
            Brain::Player(_)
        ),
        "a BossConfig rider must keep its authored Brain on dismount",
    );
    // Re-grounding still happens: gravity flipped on, Mounted marker cleared.
    assert_eq!(
        rider_surface(app.world(), rider).gravity_scale,
        1.0,
        "the dismounted boss rider still gets gravity so it falls to the floor",
    );
    assert!(
        app.world().entity(rider).get::<Mounted>().is_none(),
        "Mounted marker is removed on dismount even for a boss rider",
    );
    // The dissolution fact is announced (Q19a) with both entities.
    let log = app.world().resource::<MountDiedLog>();
    assert_eq!(
        log.0,
        vec![(mount, rider)],
        "MountDied is emitted once, naming the dead mount and its rider",
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

/// M5 (ADR 0020 §4) — **player-piloting through the control seam is
/// rider-agnostic.** A PLAYER-driven rider pilots the mount through the exact
/// same two coupling systems an AI rider uses. Coupling keys on the STRUCTURAL
/// facts (both bodies alive + carrying their mount-role components), never on
/// disposition: this rider carries `Brain::Player` and a `Peaceful` disposition
/// (the shape a possessed / human-driven body has — possession transfers the
/// player brain but never touches disposition; `Peaceful` here proves the
/// coupling ignores disposition entirely). It both (a) STEERS the mount — its
/// locomotion intent flows through `steer_mount_from_rider` onto the mount — and
/// (b) WELDS to the mount — `sync_riders_to_mounts` snaps its pose — identically
/// to the enemy Skirmisher rider. Before M5 the `is_hostile()` gate skipped a
/// non-hostile rider, so a human piloting a vehicle would ride nothing.
#[test]
fn a_player_controlled_rider_pilots_the_mount_agnostically() {
    use ambition_characters::actor::control::ActorControlFrame;
    use ambition_characters::brain::{ActorControl, Brain, PlayerSlot};

    let mut app = build_app();
    // The two coupling systems in their schedule order: steer routes the rider's
    // intent onto the mount, then the pose sync welds the rider back on.
    app.add_systems(
        Update,
        (steer_mount_from_rider, sync_riders_to_mounts).chain(),
    );

    let mount_pos = ae::Vec2::new(0.0, 0.0);
    let mount_size = ae::Vec2::new(126.0, 52.0);
    let mount = app
        .world_mut()
        .spawn((
            hostile("mount", "burning_flying_shark", mount_pos, mount_size),
            CenteredAabb::from_center_size(mount_pos, mount_size),
            Mountable::at(ae::Vec2::new(0.0, -40.0)),
            MountSlot { rider: None },
            ActorControl(ActorControlFrame::neutral()),
        ))
        .id();

    // A hand-authored PLAYER locomotion intent (what `Brain::Player` would emit
    // from slot input): drive right at 200 px/s, facing left.
    let mut rider_frame = ActorControlFrame::neutral();
    rider_frame.locomotion = ae::Vec2::new(1.0, 0.0);
    rider_frame.velocity_target = ae::Vec2::new(200.0, 0.0);
    rider_frame.facing = -1.0;

    let rider_start = ae::Vec2::new(999.0, 999.0);
    let rider_size = ae::Vec2::new(44.0, 78.0);
    // The full actor-cluster body, but spawned with a PLAYER identity: a
    // `Peaceful` disposition + `Brain::Player` instead of the enemy default.
    let (_enemy_disposition, rider_bundle) =
        hostile("rider", "pirate_raider", rider_start, rider_size);
    let rider = app
        .world_mut()
        .spawn((
            crate::features::ActorDisposition::Peaceful,
            rider_bundle,
            CenteredAabb::from_center_size(rider_start, rider_size),
            Brain::Player(PlayerSlot::PRIMARY),
            ActorControl(rider_frame),
            Mounted,
            RidingOn { mount },
        ))
        .id();
    app.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });

    app.update();

    // (a) STEERED: the mount executes the PLAYER rider's locomotion intent.
    let mount_frame = app.world().entity(mount).get::<ActorControl>().unwrap().0;
    assert_eq!(
        mount_frame.velocity_target,
        ae::Vec2::new(200.0, 0.0),
        "the mount obeys the player rider's velocity_target — piloting through the control seam",
    );
    assert_eq!(
        mount_frame.facing, -1.0,
        "the mount inherits the player rider's facing",
    );

    // (b) WELDED: the player rider snapped onto mount.pos + offset, exactly as an
    // AI rider would — the sync did NOT skip it for being non-hostile.
    let k = rider_kin(app.world(), rider);
    assert_eq!(
        k.pos,
        ae::Vec2::new(0.0, -40.0),
        "the player rider welds to mount.pos + offset (controller-agnostic coupling)",
    );
    assert_eq!(
        k.vel,
        ae::Vec2::ZERO,
        "the welded player rider's velocity is zeroed"
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

/// G2-archetypes end-to-end (ADR 0020; Q19): the REAL authored `giant_gnu`
/// mount + `gnu_ton_rider` boss pair, exercised through the whole
/// dismount→on-foot bridge.
///
/// This ties together every G2 authoring seam at once:
///   * the `giant_gnu` archetype parses with `mount_class == "giant"` (it IS a
///     rideable mount),
///   * `npc_giant_gnu` resolves a character sprite (the mount renders via the
///     character-sprite path, not the boss split-overlay),
///   * the `gnu_ton_rider` boss profile carries the authored `mount_died`
///     External phase trigger (its on-foot mini-phase), and
///   * linking the pair and killing the mount drives the Q19 bridge: the boss
///     dismounts KEEPING its Brain (the BossConfig rule), gravity flips on, and
///     its phase advances to the authored on-foot `Enrage` via `mount_died`.
#[test]
fn giant_gnu_mount_and_gnu_ton_rider_dismount_bridge_end_to_end() {
    use crate::boss_encounter::{
        BossEncounterPhase, BossProfile, PhaseTrigger, PhaseTriggerCondition,
    };
    use ambition_characters::brain::{Brain, PlayerSlot};

    // (1) The `giant_gnu` archetype parses as a rideable "giant"-class mount.
    let mount_spec = crate::features::enemies::test_spec("giant_gnu");
    assert_eq!(
        mount_spec.mount_class.as_deref(),
        Some("giant"),
        "the giant_gnu archetype must be a rideable 'giant'-class mount",
    );
    assert!(
        !mount_spec.body_contact_damage,
        "the carried giant deals no contact damage (its rider is the threat)",
    );

    // (2) The `npc_giant_gnu` catalog id resolves a character sprite — the mount
    // renders through the character-sprite path. Gated on the baked sheet being
    // present (sprites are gitignored/regenerated; a fresh clone has none).
    if crate::character_sprites::record_for_target("giant_gnu").is_some() {
        assert!(
            crate::character_sprites::sheet_for_character_id("npc_giant_gnu").is_some(),
            "npc_giant_gnu should resolve the baked giant_gnu sheet spec",
        );
    }

    // (3) The authored `gnu_ton_rider` boss profile carries the on-foot
    // `mount_died` External trigger (this is what makes the mini-phase authored,
    // not test-injected).
    let profile = BossProfile::from_id("gnu_ton_rider")
        .expect("gnu_ton_rider boss profile+encounter are authored");
    assert_eq!(
        profile.behavior.pilotable_mount_classes,
        vec!["giant".to_string()],
        "the rider boss pilots the 'giant' mount class",
    );
    let triggers = PhaseTrigger::intrinsic_from_spec(&profile.encounter);
    let mount_died_to = triggers.iter().find_map(|t| match &t.when {
        PhaseTriggerCondition::External(g) if g == "mount_died" => Some(t.to),
        _ => None,
    });
    assert_eq!(
        mount_died_to,
        Some(BossEncounterPhase::Enrage),
        "the authored gnu_ton_rider encounter must carry a mount_died -> Enrage \
         External trigger (the on-foot mini-phase)",
    );

    // (4) Spawn the REAL pair, link it, kill the mount, and tick the whole
    // bridge (dissolution + boss-encounter notify) in one update.
    let mut app = build_app();
    app.add_systems(
        Update,
        (
            enforce_mount_rider_link,
            crate::boss_encounter::notify_bosses_on_mount_death,
        )
            .chain(),
    );

    // The giant_gnu MOUNT — spawned already dead so the dissolution fires this
    // frame. Rideable "giant" class + the standard MountSlot back-reference.
    let mount_pos = ae::Vec2::new(0.0, 0.0);
    let mount_size = ae::Vec2::new(220.0, 220.0);
    let mut mount_actor = hostile("giant_gnu", "giant_gnu", mount_pos, mount_size);
    mount_actor.1 .2.health.current = 0; // dead → dissolution fires
    let mut mountable = Mountable::at(ae::Vec2::new(0.0, -140.0));
    mountable.class = MountClass("giant".into());
    let mount = app
        .world_mut()
        .spawn((mount_actor, mountable, MountSlot { rider: None }))
        .id();

    // The gnu_ton_rider BOSS — a live mounted rider carrying the authored
    // encounter phase state (at Phase1) + a distinctive `Brain::Player` marker
    // so a surviving marker proves the BossConfig brain-keep rule. The dismount
    // rebuild would produce a `Brain::StateMachine`, so `Player` surviving is
    // load-bearing.
    let rider_pos = ae::Vec2::new(0.0, -140.0);
    let rider_size = ae::Vec2::new(54.0, 96.0);
    let mut rider_actor = hostile("gnu_ton_rider", "gnu_ton_rider", rider_pos, rider_size);
    rider_actor.1 .5.gravity_scale = 0.0; // mounted → gravity off
    let (boss_encounter, _hp) = crate::combat::boss_clusters::test_support::test_boss_status_with(
        profile.encounter.max_hp,
        BossEncounterPhase::Phase1,
        triggers,
    );
    let boss_config = crate::features::BossConfig {
        id: "gnu_ton_rider".into(),
        name: profile.display_name.clone(),
        spawn: rider_pos,
        brain: ambition_characters::actor::BossBrain::Dormant,
        behavior: profile.behavior.clone(),
    };
    let rider = app
        .world_mut()
        .spawn((
            rider_actor,
            CenteredAabb::from_center_size(rider_pos, rider_size),
            boss_encounter,
            boss_config,
            Brain::Player(PlayerSlot(0)),
            CanPilot {
                classes: vec![MountClass("giant".into())],
            },
            Mounted,
            RidingOn { mount },
        ))
        .id();
    app.world_mut()
        .entity_mut(mount)
        .insert(MountSlot { rider: Some(rider) });

    app.update();

    // The boss kept its authored Brain (BossConfig rule, Q19b) — not a rebuilt
    // solo StateMachine.
    assert!(
        matches!(
            app.world().entity(rider).get::<Brain>().unwrap(),
            Brain::Player(_)
        ),
        "the dismounted gnu_ton_rider boss must keep its authored Brain",
    );
    // Gravity flipped on so the scholar falls off the dead giant.
    assert_eq!(
        rider_surface(app.world(), rider).gravity_scale,
        1.0,
        "the dismounted boss gets gravity so it lands on foot",
    );
    // Mounted marker cleared.
    assert!(
        app.world().entity(rider).get::<Mounted>().is_none(),
        "the Mounted marker is removed on dismount",
    );
    // And the phase advanced to the authored on-foot mini-phase via mount_died.
    let phase = app
        .world()
        .entity(rider)
        .get::<crate::features::BossEncounter>()
        .unwrap()
        .encounter
        .as_ref()
        .unwrap()
        .phase;
    assert_eq!(
        phase,
        BossEncounterPhase::Enrage,
        "mount death must flip the rider boss into its authored on-foot phase",
    );
}
