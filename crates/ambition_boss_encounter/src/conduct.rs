//! Conducted bosses (fast-iteration I7): the adapters of
//! `ambition.boss.conduct`, `ambition.boss.conducted_pose`,
//! `ambition.presentation.drawn_row` and `ambition.feedback.burst`. See the
//! port cards in `ambition_boss_special_port::conduct` and
//! `ambition_combat_port::riding`.
//!
//! A conducted boss's pattern decides which move and when; its module decides
//! the rest: where the body goes, what it swings, what it is drawn as. The
//! flying spaghetti monster is the first (`ambition_content_modules::fsm`).

use ambition_boss_special_port::{
    BossConduct, BossConductPort, ConductedPosePort, DrawnRowPort, LiveMove, RoomHall,
};
use ambition_characters::actor::BodyHealth;
use ambition_characters::brain::{BossAttackProfile, BossAttackState};
use ambition_characters::control::DrivingParticipant;
use ambition_combat::components::ActorTarget;
use ambition_combat_port::BurstPort;
use ambition_extension_host::{AdmittedExtensions, ExtensionInvocations, ExtensionOutbox};
use ambition_extension_sdk::phases::BOSS_CONDUCT;
use ambition_extension_sdk::Port;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::{BossConfig, BossEncounter, BossEncounterPhase};

/// The side a conducted boss faces while its module holds its pose: written
/// by `ambition.boss.conducted_pose`, read by the next tick's trigger and by
/// [`face_conducted_bosses`]. A body fact, not module state: the boss's birth
/// sets it from the side the body was built facing.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ConductedFacing(pub f32);

impl ConductedFacing {
    /// The side a body built facing `facing` starts on.
    pub fn of_facing(facing: f32) -> Self {
        Self(if facing < 0.0 { -1.0 } else { 1.0 })
    }
}

fn live(profile: &Option<BossAttackProfile>, remaining: f32) -> Option<LiveMove> {
    match profile {
        Some(BossAttackProfile::Special(key)) => Some(LiveMove {
            key: key.clone(),
            remaining,
        }),
        _ => None,
    }
}

/// One invocation for each boss whose id is bound, each tick with a gameplay
/// step, in boss query order.
#[allow(clippy::type_complexity)]
pub fn queue_boss_conducts(
    admitted: Res<AdmittedExtensions>,
    time: Res<ambition_time::WorldTime>,
    mut invocations: ResMut<ExtensionInvocations>,
    bosses: Query<(
        Entity,
        &BossConfig,
        &BossAttackState,
        &ae::BodyKinematics,
        &BodyHealth,
        &BossEncounter,
        Option<&ActorTarget>,
        Option<&ConductedFacing>,
        Has<DrivingParticipant>,
    )>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<ae::RoomGeometry>,
    captives: Query<&ambition_combat::capture::CapturedBy>,
    mobs: Query<
        (Entity, &ambition_combat::components::EncounterMob, &BodyHealth),
        Without<BossConfig>,
    >,
    live_rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    if time.sim_dt() <= 0.0 {
        return;
    }
    let bound: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.trigger.port == BossConductPort::KEY)
        .map(|e| e.descriptor.trigger.selector.as_ref())
        .collect();
    if bound.is_empty() {
        return;
    }
    for (boss, config, attack, kin, health, encounter, target, side, driven) in &bosses {
        if !bound.contains(&config.behavior.id.as_str()) {
            continue;
        }
        let geometry = rooms.of(boss);
        let room = geometry.map(|geometry| [geometry.0.size.x, geometry.0.size.y]);
        let hall = geometry
            .and_then(|geometry| crate::hall::measure_hall(&geometry.0, kin.pos))
            .map(|h| RoomHall {
                floor: h.floor,
                left: h.left,
                right: h.right,
            });
        invocations.trigger::<BossConductPort>(
            &BOSS_CONDUCT,
            config.behavior.id.clone(),
            boss,
            None,
            false,
            BossConduct {
                position: [kin.pos.x, kin.pos.y],
                velocity: [kin.vel.x, kin.vel.y],
                facing: kin.facing,
                side: side.copied().unwrap_or(ConductedFacing::of_facing(kin.facing)).0,
                alive: health.alive(),
                telegraph: live(&attack.telegraph_profile, attack.telegraph_remaining),
                active: live(&attack.active_profile, attack.active_remaining),
                target: target.and_then(|t| t.entity.map(|_| [t.pos.x, t.pos.y])),
                hall,
                driven,
                enraged: encounter.encounter_phase() == BossEncounterPhase::Enrage,
                holding: captives.iter().any(|held| held.captor == boss),
                minions: mobs
                    .iter()
                    .filter(|(mob, of, health)| {
                        of.encounter_id == config.behavior.id
                            && health.alive()
                            && live_rooms.of(*mob) == live_rooms.of(boss)
                    })
                    .count() as u32,
                between_phases: encounter.between_phases(),
                room,
            },
        );
    }
}

/// `ambition.boss.mount`: the body the boss rides, its back and its limbs.
pub fn mount_of(world: &World, scope: Entity) -> Option<ambition_boss_special_port::MountView> {
    use ambition_boss_special_port::{MountLimb, MountView};
    let mount = world.get::<ambition_mount::RidingOn>(scope)?.mount;
    let kin = world.get::<ae::BodyKinematics>(mount)?;
    let unmirrored = world.get::<ae::Unmirrored>(mount).is_some();
    let back = world.get::<ambition_mount::BackLedge>(mount).map(|back| {
        let b = back.aabb(kin, unmirrored);
        [b.min.x, b.min.y, b.max.x, b.max.y]
    });
    // The floor and walls under the mount, in its own live room.
    let room = world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(mount);
    let hall = ambition_platformer2d_world::collision::with_room_in_world(world, room, |room| {
        crate::hall::measure_hall(room.base(), kin.pos)
    })
    .flatten()
    .map(|h| RoomHall { floor: h.floor, left: h.left, right: h.right });
    let limbs = world
        .get::<ambition_characters::actor::LimbRig>(mount)
        .map(|rig| {
            rig.limbs
                .iter()
                .filter_map(|(slot, &limb)| {
                    let limb_kin = world.get::<ae::BodyKinematics>(limb)?;
                    let home = world.get::<ambition_characters::actor::Limb>(limb).map_or(ae::Vec2::ZERO, |l| l.home_offset);
                    let lost = world.get::<BodyHealth>(limb).map_or(0, |h| h.max() - h.current());
                    Some(MountLimb {
                        slot: slot.as_str().to_owned(),
                        position: [limb_kin.pos.x, limb_kin.pos.y],
                        size: [limb_kin.size.x, limb_kin.size.y],
                        home_offset: [home.x, home.y],
                        lost_health: lost,
                        held: world.get::<ae::PoseOwnedExternally>(limb).is_some(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(MountView {
        position: [kin.pos.x, kin.pos.y],
        velocity: [kin.vel.x, kin.vel.y],
        size: [kin.size.x, kin.size.y],
        facing: kin.facing,
        unmirrored,
        back,
        hall,
        limbs,
    })
}

/// The limb in `slot` of the rig of the mount `rider` rides.
fn ridden_limb(
    rider: Entity,
    slot: &str,
    riders: &Query<&ambition_mount::RidingOn>,
    rigs: &Query<&ambition_characters::actor::LimbRig>,
) -> Option<Entity> {
    let rig = rigs.get(riders.get(rider).ok()?.mount).ok()?;
    rig.limbs.iter().find(|(name, _)| name.as_str() == slot).map(|(_, &limb)| limb)
}

/// The `ambition.boss.limb_pose` requests of this tick, for [`pose_limbs`].
pub fn drain_limb_poses(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbPose>> {
    outbox.drain::<ambition_boss_special_port::LimbPosePort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.limb_pose`: hold a limb of the ridden mount at a
/// pose, or let it go. The adapter is `drain_limb_poses.pipe(pose_limbs)`,
/// so a test can give requests without a module.
#[allow(clippy::type_complexity)]
pub fn pose_limbs(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbPose>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    mut limbs: Query<(
        &mut ae::BodyKinematics,
        &mut ae::CenteredAabb,
        &mut ae::BodyGroundState,
        Option<&mut ae::SweepSample>,
        Has<ae::PoseOwnedExternally>,
    )>,
    mut commands: Commands,
) {
    for submitted in requests {
        let pose = submitted.value;
        let Some(limb) = ridden_limb(submitted.scope, &pose.slot, &riders, &rigs) else {
            warn!(
                "extension entry {} asked to pose limb {:?} of the mount {:?} rides, which it does not have; refused",
                submitted.entry, pose.slot, submitted.scope
            );
            continue;
        };
        let Ok((mut kin, mut aabb, mut ground, mut sweep, held)) = limbs.get_mut(limb) else {
            continue;
        };
        match pose.hold {
            Some(hold) => {
                ae::movement::constrain_body_pose(
                    &mut kin,
                    sweep.as_deref_mut(),
                    ae::Vec2::from(hold.position),
                    ae::Vec2::from(hold.velocity),
                );
                kin.facing = hold.facing;
                if !held {
                    commands.entity(limb).try_insert(ae::PoseOwnedExternally);
                }
                // A discrete pose change: the next step finds the ground again.
                ground.invalidate();
            }
            None => {
                if held {
                    commands.entity(limb).try_remove::<ae::PoseOwnedExternally>();
                }
                // It falls straight down from where it is.
                let (pos, fall) = (kin.pos, kin.vel.y);
                ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, ae::Vec2::new(0.0, fall));
            }
        }
        // The centre only: body integration publishes the size of the
        // footprint, turned to the limb's down.
        aabb.center = kin.pos;
    }
}

/// Lower `ambition.boss.conducted_pose`: hold the boss's pose, or release it;
/// keep the side it faces.
#[allow(clippy::type_complexity)]
pub fn lower_conducted_poses(
    mut outbox: ResMut<ExtensionOutbox>,
    mut bosses: Query<
        (
            &mut ae::BodyKinematics,
            &mut ae::CenteredAabb,
            Option<&mut ae::SweepSample>,
            Has<ae::PoseOwnedExternally>,
            Option<&mut ConductedFacing>,
        ),
        With<BossConfig>,
    >,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<ConductedPosePort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let Ok((mut kin, mut aabb, mut sweep, owned, facing)) = bosses.get_mut(submitted.scope) else {
            warn!(
                "extension entry {} asked to conduct {:?}, which is not a boss; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let pose = submitted.value;
        match pose.pose {
            Some(p) => {
                let pos = ae::Vec2::from(p.position);
                ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, ae::Vec2::from(p.velocity));
                aabb.center = pos;
                if !owned {
                    commands.entity(submitted.scope).try_insert(ae::PoseOwnedExternally);
                }
            }
            None => {
                if owned {
                    commands.entity(submitted.scope).try_remove::<ae::PoseOwnedExternally>();
                }
            }
        }
        match facing {
            Some(mut facing) => facing.0 = pose.side,
            None => {
                commands.entity(submitted.scope).try_insert(ConductedFacing(pose.side));
            }
        }
    }
}

/// Lower `ambition.boss.guard` into the boss's `BossEncounter::guarded`.
pub fn lower_boss_guards(mut outbox: ResMut<ExtensionOutbox>, mut bosses: Query<&mut BossEncounter>) {
    for submitted in outbox.drain::<ambition_boss_special_port::BossGuardPort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        if let Ok(mut status) = bosses.get_mut(submitted.scope) {
            if status.guarded != submitted.value.guarded {
                status.guarded = submitted.value.guarded;
            }
        }
    }
}

/// Lower `ambition.presentation.drawn_row` into the body's `PinnedRow`.
pub fn lower_drawn_rows(
    mut outbox: ResMut<ExtensionOutbox>,
    mut rows: Query<Option<&mut ambition_sprite_sheet::character::PinnedRow>>,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<DrawnRowPort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let Ok(row) = rows.get_mut(submitted.scope) else {
            continue;
        };
        let drawn = submitted.value;
        match (row, &drawn.name) {
            (Some(mut row), Some(name)) => row.pin(&[name.as_str()], drawn.elapsed, drawn.looping),
            (Some(mut row), None) => row.clear(),
            (None, Some(name)) => {
                let mut row = ambition_sprite_sheet::character::PinnedRow::default();
                row.pin(&[name.as_str()], drawn.elapsed, drawn.looping);
                commands.entity(submitted.scope).try_insert(row);
            }
            (None, None) => {}
        }
    }
}

/// Lower `ambition.feedback.burst` into `VfxMessage::Burst`.
pub fn lower_bursts(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    // A burst is drawn in the live room of the body its entry runs for.
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for submitted in outbox.drain::<BurstPort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let burst = submitted.value;
        let Some(kind) = ambition_vfx::vfx::ParticleKind::named(&burst.kind) else {
            warn!("extension entry {} asked for a burst of {:?}; refused", submitted.entry, burst.kind);
            continue;
        };
        vfx.for_room(rooms.of(submitted.scope)).write(ambition_vfx::vfx::VfxMessage::Burst {
            pos: ae::Vec2::from(burst.at),
            count: burst.count,
            speed: burst.speed,
            color: burst.color,
            kind,
        });
    }
}

/// A conducted boss's camera shakes, as the boss phase change asks for its
/// own: an intent the presentation applies on the confirmed frame.
pub fn lower_camera_shakes(
    mut outbox: ResMut<ExtensionOutbox>,
    mut shake: MessageWriter<ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest>,
) {
    for submitted in outbox.drain::<ambition_combat_port::CameraShakePort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
            amplitude_px: submitted.value.amplitude_px,
        });
    }
}

/// Turn a conducted boss to the side its module chose, through the control
/// the body integrator applies. Only while the module holds the pose: a
/// driven boss faces where its participant steers it. Runs in
/// `BossSteerSlot`, in the control gate after the brain and before the
/// integration.
pub fn face_conducted_bosses(
    mut bosses: Query<(&ConductedFacing, &mut ambition_characters::control::ActorControl), With<ae::PoseOwnedExternally>>,
) {
    for (facing, mut control) in &mut bosses {
        control.0.facing = facing.0;
    }
}

#[cfg(test)]
mod mount_view_tests {
    use super::*;
    use ambition_characters::actor::{Health, Limb, LimbRig, LimbSlot};
    use ambition_platformer2d_shared_tangle::lifecycle::{insert_live_room_component, InRoomInstance, LiveRoomInstance};

    fn body(pos: ae::Vec2, size: ae::Vec2) -> ae::BodyKinematics {
        let mut kin = ae::BodyKinematics::default();
        kin.pos = pos;
        kin.size = size;
        kin
    }

    /// ⭐ The view is the mount's components as they are: its body, its back
    /// where it is drawn, the hall under it, and each limb with its rest, its
    /// lost health and whether it is held. The control: a boss that rides
    /// nothing has no view.
    #[test]
    fn the_mount_view_is_the_mount_as_it_is() {
        let mut world = World::new();
        let floor = ae::Block::solid("floor", ae::Vec2::new(0.0, 1250.0), ae::Vec2::new(2000.0, 50.0));
        let wall_l = ae::Block::solid("wall_l", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(48.0, 1300.0));
        let wall_r = ae::Block::solid("wall_r", ae::Vec2::new(1800.0, 0.0), ae::Vec2::new(48.0, 1300.0));
        insert_live_room_component(
            &mut world,
            ae::RoomGeometry(ae::World::new("hall", ae::Vec2::new(2000.0, 1300.0), ae::Vec2::new(100.0, 100.0), vec![floor, wall_l, wall_r])),
        );
        let room = InRoomInstance(LiveRoomInstance::ACTIVATION);
        let mut left_health = Health::new(10);
        left_health.current = 7;
        let left = world
            .spawn((body(ae::Vec2::new(630.0, 970.0), ae::Vec2::new(130.0, 100.0)), ambition_characters::actor::BodyHealth::new(left_health), ae::PoseOwnedExternally))
            .id();
        let right = world
            .spawn((body(ae::Vec2::new(1170.0, 970.0), ae::Vec2::new(130.0, 100.0)), ambition_characters::actor::BodyHealth::new(Health::new(10))))
            .id();
        let mut gnu_kin = body(ae::Vec2::new(900.0, 1040.0), ae::Vec2::new(439.0, 414.0));
        gnu_kin.vel = ae::Vec2::new(0.0, -520.0);
        gnu_kin.facing = -1.0;
        let back = ambition_mount::BackLedge(ambition_characters::actor::MountBack { left: -208.0, right: 169.0, top: -9.75, thickness: 14.0 });
        let gnu = world
            .spawn((gnu_kin, ae::Unmirrored, back, room, LimbRig::from_pairs([(LimbSlot::HAND_LEFT, left), (LimbSlot::HAND_RIGHT, right)])))
            .id();
        world.entity_mut(left).insert(Limb { of: gnu, slot: LimbSlot::HAND_LEFT, home_offset: ae::Vec2::new(-270.0, -70.0) });
        world.entity_mut(right).insert(Limb { of: gnu, slot: LimbSlot::HAND_RIGHT, home_offset: ae::Vec2::new(270.0, -70.0) });
        let scholar = world.spawn((body(ae::Vec2::new(950.0, 1000.0), ae::Vec2::new(40.0, 60.0)), ambition_mount::RidingOn { mount: gnu })).id();

        let view = mount_of(&world, scholar).expect("the scholar rides the gnu");
        let drawn = back.aabb(&gnu_kin, true);
        assert_eq!(
            view,
            ambition_boss_special_port::MountView {
                position: [900.0, 1040.0],
                velocity: [0.0, -520.0],
                size: [439.0, 414.0],
                facing: -1.0,
                unmirrored: true,
                back: Some([drawn.min.x, drawn.min.y, drawn.max.x, drawn.max.y]),
                hall: Some(RoomHall { floor: 1250.0, left: 48.0, right: 1800.0 }),
                limbs: vec![
                    ambition_boss_special_port::MountLimb {
                        slot: "hand_left".into(),
                        position: [630.0, 970.0],
                        size: [130.0, 100.0],
                        home_offset: [-270.0, -70.0],
                        lost_health: 3,
                        held: true,
                    },
                    ambition_boss_special_port::MountLimb {
                        slot: "hand_right".into(),
                        position: [1170.0, 970.0],
                        size: [130.0, 100.0],
                        home_offset: [270.0, -70.0],
                        lost_health: 0,
                        held: false,
                    },
                ],
            }
        );
        let walker = world.spawn(body(ae::Vec2::ZERO, ae::Vec2::ONE)).id();
        assert_eq!(mount_of(&world, walker), None, "a boss that rides nothing has no mount");
    }
}

#[cfg(test)]
mod limb_pose_tests {
    use super::*;
    use ambition_boss_special_port::{LimbHold, LimbPose};
    use ambition_characters::actor::{LimbRig, LimbSlot};
    use ambition_extension_host::Submitted;
    use bevy::ecs::system::RunSystemOnce as _;

    fn submit(world: &mut World, boss: Entity, pose: LimbPose) {
        let request = Submitted { scope: boss, occurrence: None, entry: "test::gnu/conduct".into(), value: pose };
        world.run_system_once_with(pose_limbs, vec![request]).unwrap();
    }

    /// ⭐ A hold poses the limb, holds it and clears its ground; a release
    /// lets it fall straight down from where it is. The control: a slot the
    /// rig does not have changes nothing.
    #[test]
    fn a_held_limb_is_where_it_was_put_and_a_released_one_falls() {
        let mut world = World::new();
        let mut fist = ae::BodyKinematics::default();
        fist.pos = ae::Vec2::new(600.0, 1000.0);
        fist.vel = ae::Vec2::new(40.0, 90.0);
        let mut ground = ae::BodyGroundState::default();
        ground.on_ground = true;
        let limb = world.spawn((fist, ae::CenteredAabb::new(fist.pos, ae::Vec2::new(65.0, 50.0)), ground)).id();
        let mount = world.spawn(LimbRig::from_pairs([(LimbSlot::HAND_LEFT, limb)])).id();
        let boss = world.spawn(ambition_mount::RidingOn { mount }).id();

        submit(&mut world, boss, LimbPose { slot: "hand_right".into(), hold: Some(LimbHold { position: [1.0, 1.0], velocity: [0.0; 2], facing: -1.0 }) });
        let kin = *world.get::<ae::BodyKinematics>(limb).unwrap();
        assert_eq!((kin.pos, kin.vel), (ae::Vec2::new(600.0, 1000.0), ae::Vec2::new(40.0, 90.0)), "another slot moved the limb");
        assert!(world.get::<ae::PoseOwnedExternally>(limb).is_none(), "another slot held the limb");

        submit(&mut world, boss, LimbPose { slot: "hand_left".into(), hold: Some(LimbHold { position: [630.0, 970.0], velocity: [12.0, -3.0], facing: -1.0 }) });
        let kin = *world.get::<ae::BodyKinematics>(limb).unwrap();
        assert_eq!((kin.pos, kin.vel, kin.facing), (ae::Vec2::new(630.0, 970.0), ae::Vec2::new(12.0, -3.0), -1.0));
        assert_eq!(world.get::<ae::CenteredAabb>(limb).unwrap().center, ae::Vec2::new(630.0, 970.0));
        assert!(!world.get::<ae::BodyGroundState>(limb).unwrap().on_ground, "a posed limb's ground is cleared");
        assert!(world.get::<ae::PoseOwnedExternally>(limb).is_some(), "a posed limb is held");

        submit(&mut world, boss, LimbPose { slot: "hand_left".into(), hold: None });
        let kin = *world.get::<ae::BodyKinematics>(limb).unwrap();
        assert_eq!((kin.pos, kin.vel), (ae::Vec2::new(630.0, 970.0), ae::Vec2::new(0.0, -3.0)), "a released limb falls straight down");
        assert!(world.get::<ae::PoseOwnedExternally>(limb).is_none(), "a released limb is not held");
    }
}
