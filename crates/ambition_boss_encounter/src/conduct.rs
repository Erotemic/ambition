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
    BodyBox, BodySwing, BossConduct, BossConductPort, ConductedPosePort, DrawnPart, DrawnRow, DrawnRowPort, LiveMove, Predecessor,
    RoomHall,
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

/// The adapter of `ambition.boss.wear`.
mod wear;
pub use wear::{drain_wears, gear_row, wear_gear};

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
    // The tracked body, read now: `ActorTarget` holds where the boss last saw it.
    bodies: Query<(&ae::BodyKinematics, Option<&ambition_combat::moveset::MovePlayback>)>,
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
        // The boss it wakes after, by placement id, in its own live room.
        let predecessor = config.wakes_after.as_deref().and_then(|after| {
            bosses.iter().find_map(|(other, other_config, _, other_kin, other_health, _, _, other_side, _)| {
                (other_config.id == after && live_rooms.of(other) == live_rooms.of(boss)).then(|| Predecessor {
                    position: [other_kin.pos.x, other_kin.pos.y],
                    size: [other_kin.size.x, other_kin.size.y],
                    side: other_side.copied().unwrap_or(ConductedFacing::of_facing(other_kin.facing)).0,
                    alive: other_health.alive(),
                })
            })
        });
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
                health: health.current(),
                invulnerable: encounter.encounter.as_ref().is_some_and(|state| state.boss_invulnerable()),
                target_body: target.and_then(|t| t.entity).and_then(|tracked| bodies.get(tracked).ok()).map(|(body, playback)| {
                    BodyBox { position: [body.pos.x, body.pos.y], size: [body.size.x, body.size.y], swing: playback.map(swing_of) }
                }),
                size: [kin.size.x, kin.size.y],
                awake: encounter.encounter_phase() != BossEncounterPhase::Dormant,
                predecessor,
            },
        );
    }
}

/// The move a body plays, as the boss that tracks it sees it.
fn swing_of(playback: &ambition_combat::moveset::MovePlayback) -> BodySwing {
    use ambition_entity_catalog::WindowTag;
    let t = playback.t;
    let mut hurts = playback.spec.windows.iter().filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty());
    let live = hurts.clone().any(|w| (w.start_s..w.end_s).contains(&t));
    // The next part that hurts, when it is not live now.
    let next = hurts.find(|w| w.start_s > t).map(|w| w.start_s - t);
    BodySwing {
        since: t,
        hurts_in: if live { 0.0 } else { next.unwrap_or(0.0) },
        live,
        ends_in: (playback.spec.duration_s - t).max(0.0),
    }
}

/// `ambition.boss.mount`: the body the boss rides, its back and its limbs.
/// Where a limb touches something solid, in the world it meets (its hole
/// and the far side of the pair it reaches into, `solids_for`), and where
/// that is: at the far end when the touch is through the pair.
fn limb_contact(world: &World, limb: Entity, kin: &ae::BodyKinematics) -> Option<ae::Vec2> {
    let room = world.get::<ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>(limb);
    let body = ae::Aabb::new(kin.pos, kin.size * 0.5);
    let point = ambition_platformer2d_world::collision::with_room_in_world(world, room, |room| {
        let walls = room.solids_for(limb)?;
        ae::collision_semantics::touching_point(&walls, body, 1.0)
    })
    .flatten()?;
    // Through the pair the limb is in: where it touches is at the far end.
    let passages = world.get_resource::<ambition_platformer2d_shared_tangle::portal_passages::PortalPassages>();
    let link = passages.and_then(|p| p.straddled_by(limb).map(|link| (link, p.convention())));
    Some(match link {
        Some((link, convention))
            if ae::frame::thread::opening(&link.enter).iter().all(|plane| plane.contains(point)) =>
        {
            link.map(convention).point(point)
        }
        _ => point,
    })
}

pub fn mount_of(world: &World, scope: Entity) -> Option<ambition_boss_special_port::MountView> {
    use ambition_boss_special_port::{MountLimb, MountView};
    let mount = world.get::<ambition_mount::RidingOn>(scope)?.mount;
    let kin = world.get::<ae::BodyKinematics>(mount)?;
    let unmirrored = world.get::<ae::Unmirrored>(mount).is_some();
    let back_ledge = world.get::<ambition_mount::BackLedge>(mount);
    let back = back_ledge.map(|back| {
        let b = back.aabb(kin, unmirrored);
        [b.min.x, b.min.y, b.max.x, b.max.y]
    });
    // Each ledge of one-way ground where it is now (a drawn ledge where the
    // drawing put it): its top.
    let ledges = back_ledge
        .map(|back| back.ground(kin, unmirrored).map(|b| [b.min.x, b.min.y, b.max.x, b.min.y]).collect())
        .unwrap_or_default();
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
                        held: world
                            .get::<ae::PoseOwnedExternally>(limb)
                            .is_some_and(|holds| holds.holds(ae::PoseHold::Conductor)),
                        contact: limb_contact(world, limb, limb_kin).map(|p| [p.x, p.y]),
                        struck: world
                            .get::<LimbStrikeBox>(limb)
                            .and_then(|carried| world.get::<ambition_combat::strike::HitboxHits>(carried.hitbox))
                            .map_or(0, |hits| hits.hit.len() as u32),
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
        ledges,
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
        Option<&ae::PoseOwnedExternally>,
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
        let Ok((mut kin, mut aabb, mut ground, mut sweep, holds)) = limbs.get_mut(limb) else {
            continue;
        };
        let held = holds.is_some_and(|holds| holds.holds(ae::PoseHold::Conductor));
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
                    ae::claim_pose_hold(&mut commands, limb, ae::PoseHold::Conductor);
                }
                // A discrete pose change: the next step finds the ground again.
                ground.invalidate();
            }
            None => {
                if held {
                    ae::release_pose_hold(&mut commands, limb, ae::PoseHold::Conductor);
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

/// The `ambition.boss.limb_bond` requests of this tick, for [`bond_limbs`].
pub fn drain_limb_bonds(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbBond>> {
    outbox.drain::<ambition_boss_special_port::LimbBondPort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.limb_bond`: each limb asked for this tick has its
/// bond start (`LimbBondStart`), and each other limb has none, so its bond
/// is from its host's body. This adapter is the one writer of the start.
pub fn bond_limbs(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbBond>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    starts: Query<(Entity, &ambition_characters::actor::LimbBondStart)>,
    mut commands: Commands,
) {
    use ambition_characters::actor::LimbBondStart;
    let mut asked: Vec<Entity> = Vec::new();
    for submitted in requests {
        let bond = submitted.value;
        let Some(limb) = ridden_limb(submitted.scope, &bond.slot, &riders, &rigs) else {
            warn!(
                "extension entry {} asked for the bond of limb {:?} of the mount {:?} rides, which it does not have; refused",
                submitted.entry, bond.slot, submitted.scope
            );
            continue;
        };
        asked.push(limb);
        let start = bond.start.map(ae::Vec2::from);
        if starts.get(limb).ok().map(|(_, now)| now.0) != Some(start) {
            commands.entity(limb).try_insert(LimbBondStart(start));
        }
    }
    for (limb, _) in &starts {
        if !asked.contains(&limb) {
            commands.entity(limb).try_remove::<LimbBondStart>();
        }
    }
}

/// The `ambition.boss.limb_opening` requests of this tick, for [`open_limbs`].
pub fn drain_limb_openings(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbOpening>> {
    outbox.drain::<ambition_boss_special_port::LimbOpeningPort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.limb_opening`: a closed limb of the ridden mount
/// refuses damage (its scripted invulnerability).
pub fn open_limbs(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbOpening>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    mut limbs: Query<&mut BodyHealth, Without<BossConfig>>,
) {
    for submitted in requests {
        let opening = submitted.value;
        let Some(Ok(mut health)) = ridden_limb(submitted.scope, &opening.slot, &riders, &rigs).map(|limb| limbs.get_mut(limb))
        else {
            warn!(
                "extension entry {} asked to open limb {:?} of the mount {:?} rides, which it does not have; refused",
                submitted.entry, opening.slot, submitted.scope
            );
            continue;
        };
        health.health.invulnerable.set(ambition_characters::actor::Invulnerability::SCRIPTED, !opening.open);
    }
}

/// The damage box a limb carries for its conductor (`ambition.boss.limb_strike`):
/// the generation asked for, and the box. Rollback state: a rewind restores
/// which box the limb holds, or the replay spawns a second.
#[derive(Component, Clone, Copy, Debug)]
pub struct LimbStrikeBox {
    pub generation: u32,
    pub hitbox: Entity,
}

impl bevy::ecs::entity::MapEntities for LimbStrikeBox {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.hitbox = mapper.get_mapped(self.hitbox);
    }
}

impl ae::snapshot::SnapshotCursor for LimbStrikeBox {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        ae::snapshot::put_u32(out, self.generation);
    }
}

/// How long a limb's box lives if it is never let go: it is let go the first
/// tick it is not asked for, so this only bounds a box whose conductor stopped.
const LIMB_STRIKE_LIFETIME_S: f32 = 10.0;

/// The `ambition.boss.limb_strike` requests of this tick, for [`strike_with_limbs`].
pub fn drain_limb_strikes(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbStrike>> {
    outbox.drain::<ambition_boss_special_port::LimbStrikePort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.limb_strike`: each limb asked for carries its box,
/// which follows it (`HitboxAnchor::FollowOwner`), so the portal pair the
/// limb is in cuts it; each limb not asked for lets its box go.
pub fn strike_with_limbs(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbStrike>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    held: Query<(Entity, &LimbStrikeBox)>,
    mut commands: Commands,
) {
    use ambition_combat::strike::{Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime};
    let mut asked: Vec<Entity> = Vec::new();
    for submitted in requests {
        let strike = submitted.value;
        let Some(limb) = ridden_limb(submitted.scope, &strike.slot, &riders, &rigs) else {
            warn!(
                "extension entry {} asked for a strike of limb {:?} of the mount {:?} rides, which it does not have; refused",
                submitted.entry, strike.slot, submitted.scope
            );
            continue;
        };
        asked.push(limb);
        match held.get(limb) {
            Ok((_, carried)) if carried.generation == strike.generation => continue,
            Ok((_, carried)) => {
                if let Ok(mut box_) = commands.get_entity(carried.hitbox) {
                    box_.try_despawn();
                }
            }
            Err(_) => {}
        }
        let hitbox = commands
            .spawn((
                Hitbox {
                    strike_sfx: None,
                    owner: limb,
                    source: ambition_combat::strike::HitSide::Boss,
                    anchor: HitboxAnchor::FollowOwner { local_offset: ae::Vec2::ZERO },
                    half_extent: ae::Vec2::from(strike.half_extent),
                    shape: None,
                    facing: 1.0,
                    damage: strike.damage,
                    knockback: HitboxKnockback::FeelScale(strike.knockback),
                    launch_dir: None,
                    frame_down: ae::Vec2::new(0.0, 1.0),
                    reaction: None,
                    guard: Default::default(),
                },
                HitboxLifetime { remaining_s: LIMB_STRIKE_LIFETIME_S },
                HitboxHits::default(),
                // The limb's own picture is the strike: no stand-in for an
                // unauthored attack is drawn over the fist.
                ambition_combat::strike::DepictedByOwner,
                Name::new("Limb strike"),
            ))
            .id();
        commands.entity(limb).try_insert(LimbStrikeBox { generation: strike.generation, hitbox });
    }
    for (limb, carried) in &held {
        if asked.contains(&limb) {
            continue;
        }
        if let Ok(mut box_) = commands.get_entity(carried.hitbox) {
            box_.try_despawn();
        }
        commands.entity(limb).try_remove::<LimbStrikeBox>();
    }
}

/// The `ambition.boss.limb_blow` requests of this tick, for [`pass_limb_blows`].
pub fn drain_limb_blows(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbBlow>> {
    outbox.drain::<ambition_boss_special_port::LimbBlowPort>(&BOSS_CONDUCT)
}

/// The flash of a boss that a blow passed on to hurt: the native
/// conductor's.
const PASSED_BLOW_FLASH_S: f32 = 0.18;

/// Lower `ambition.boss.limb_blow`: heal what the limb lost, and with `pass`
/// give it to the boss, unless the encounter refuses damage now.
#[allow(clippy::type_complexity)]
pub fn pass_limb_blows(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::LimbBlow>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    mut limbs: Query<&mut BodyHealth, Without<BossConfig>>,
    mut bosses: Query<(&mut BodyHealth, &mut ambition_characters::actor::BodyCombat, &mut BossEncounter), With<BossConfig>>,
) {
    for submitted in requests {
        let blow = submitted.value;
        let Some(Ok(mut limb)) = ridden_limb(submitted.scope, &blow.slot, &riders, &rigs).map(|limb| limbs.get_mut(limb))
        else {
            warn!(
                "extension entry {} asked to pass a blow of limb {:?} of the mount {:?} rides, which it does not have; refused",
                submitted.entry, blow.slot, submitted.scope
            );
            continue;
        };
        let lost = limb.max() - limb.current();
        if lost <= 0 {
            continue;
        }
        limb.heal(lost);
        if !blow.pass {
            continue;
        }
        let Ok((mut health, mut combat, mut encounter)) = bosses.get_mut(submitted.scope) else {
            continue;
        };
        if encounter.encounter.as_ref().is_some_and(|state| state.boss_invulnerable()) {
            continue;
        }
        combat.hit_flash = PASSED_BLOW_FLASH_S;
        if health.damage(lost) {
            if let Some(state) = encounter.encounter.as_mut() {
                let _ = state.kill();
            }
        }
    }
}

/// The `ambition.boss.mount_launch` requests of this tick, for [`launch_mounts`].
pub fn drain_mount_launches(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::MountLaunch>> {
    outbox.drain::<ambition_boss_special_port::MountLaunchPort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.mount_launch`: stage a launch of the mount the boss
/// rides; the body step spends it.
pub fn launch_mounts(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::MountLaunch>>>,
    riders: Query<&ambition_mount::RidingOn>,
    mut flights: Query<&mut ae::BodyFlightState>,
) {
    for submitted in requests {
        let launch = submitted.value;
        let Some(Ok(mut flight)) = riders.get(submitted.scope).ok().map(|riding| flights.get_mut(riding.mount)) else {
            warn!(
                "extension entry {} asked to launch the mount of {:?}, which rides no body that can fly; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        flight.stage_launch(ae::Vec2::from(launch.velocity), launch.flinchless);
    }
}

/// The `ambition.boss.mount_walk` requests of this tick, for [`walk_mounts`].
pub fn drain_mount_walks(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::MountWalk>> {
    outbox.drain::<ambition_boss_special_port::MountWalkPort>(&BOSS_CONDUCT)
}

/// How near its mark a walked mount stops (world units).
const MOUNT_WALK_ARRIVED: f32 = 6.0;

/// Lower `ambition.boss.mount_walk`: walk the mount the boss rides to its mark
/// (`CommandedMove`), or stop it. Its rider does not steer it while it walks
/// (`ambition_mount::steer_mount_from_rider`).
pub fn walk_mounts(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::MountWalk>>>,
    riders: Query<&ambition_mount::RidingOn>,
    mounts: Query<(&ae::BodyKinematics, Option<&ambition_characters::control::CommandedMove>)>,
    mut commands: Commands,
) {
    for submitted in requests {
        let walk = submitted.value;
        let Some((mount, (kin, walking))) =
            riders.get(submitted.scope).ok().and_then(|riding| mounts.get(riding.mount).ok().map(|m| (riding.mount, m)))
        else {
            warn!(
                "extension entry {} asked to walk the mount of {:?}, which rides no body; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        if walk.speed <= 0.0 {
            if walking.is_some() {
                commands.entity(mount).remove::<ambition_characters::control::CommandedMove>();
            }
            continue;
        }
        let wanted = ambition_characters::control::CommandedMove {
            target: ae::Vec2::new(walk.to_x, kin.pos.y),
            speed: walk.speed,
            arrive_tolerance: MOUNT_WALK_ARRIVED,
        };
        // The mark is on the floor under it: a walk to the same x is the same
        // walk, and is not written again.
        if walking.is_none_or(|w| w.target.x != wanted.target.x || w.speed != wanted.speed) {
            commands.entity(mount).insert(wanted);
        }
    }
}

/// The `ambition.boss.foe_launch` requests of this tick, for [`launch_foes`].
pub fn drain_foe_launches(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_boss_special_port::FoeLaunch>> {
    outbox.drain::<ambition_boss_special_port::FoeLaunchPort>(&BOSS_CONDUCT)
}

/// Lower `ambition.boss.foe_launch`: stage a launch of the body the boss
/// targets; the body step spends it.
pub fn launch_foes(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_boss_special_port::FoeLaunch>>>,
    targets: Query<&ambition_combat::components::ActorTarget>,
    mut flights: Query<&mut ae::BodyFlightState>,
) {
    for submitted in requests {
        let launch = submitted.value;
        let Some(Ok(mut flight)) =
            targets.get(submitted.scope).ok().and_then(|target| target.entity).map(|body| flights.get_mut(body))
        else {
            warn!(
                "extension entry {} asked to launch the target of {:?}, which has no target that can be launched; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        flight.stage_launch(ae::Vec2::from(launch.velocity), launch.flinchless);
    }
}

/// The `ambition.feedback.speech_bubble` requests of this tick in the phase
/// `L`, for [`show_speech_bubbles`].
pub fn drain_speech_bubbles<L: ambition_extension_host::LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<ambition_combat_port::SpeechBubble>> {
    outbox.drain::<ambition_combat_port::SpeechBubblePort>(&L::PHASE)
}

/// Lower `ambition.feedback.speech_bubble` into `VfxMessage::SpeechBubble`,
/// in the live room of the body the entry ran for.
pub fn show_speech_bubbles(
    In(requests): In<Vec<ambition_extension_host::Submitted<ambition_combat_port::SpeechBubble>>>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    for submitted in requests {
        let bubble = submitted.value;
        vfx.for_room(rooms.of(submitted.scope))
            .write(ambition_vfx::vfx::VfxMessage::SpeechBubble { pos: ae::Vec2::from(bubble.at), text: bubble.text });
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
            Option<&ae::PoseOwnedExternally>,
            Option<&mut ConductedFacing>,
        ),
        With<BossConfig>,
    >,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<ConductedPosePort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let Ok((mut kin, mut aabb, mut sweep, holds, facing)) = bosses.get_mut(submitted.scope) else {
            warn!(
                "extension entry {} asked to conduct {:?}, which is not a boss; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let owned = holds.is_some_and(|holds| holds.holds(ae::PoseHold::Conductor));
        let pose = submitted.value;
        match pose.pose {
            Some(p) => {
                let pos = ae::Vec2::from(p.position);
                ae::movement::constrain_body_pose(&mut kin, sweep.as_deref_mut(), pos, ae::Vec2::from(p.velocity));
                aabb.center = pos;
                if !owned {
                    ae::claim_pose_hold(&mut commands, submitted.scope, ae::PoseHold::Conductor);
                }
            }
            None => {
                if owned {
                    ae::release_pose_hold(&mut commands, submitted.scope, ae::PoseHold::Conductor);
                }
            }
        }
        match (facing, pose.side) {
            (Some(mut facing), Some(side)) => facing.0 = side,
            (None, Some(side)) => {
                commands.entity(submitted.scope).try_insert(ConductedFacing(side));
            }
            // No side: the boss's facing stays whoever drives it.
            (_, None) => {}
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

/// Lower `ambition.boss.scale`: build each boss at the scale its conductor
/// states ([`crate::BossMut::rescale`]). A scope that is not a boss is refused.
pub fn lower_boss_scales(
    mut outbox: ResMut<ExtensionOutbox>,
    // `Option`: a composition that runs conductors may hold no boss catalog;
    // then a boss is scaled by its box alone.
    boss_catalog: Option<Res<crate::BossCatalog>>,
    mut bosses: Query<crate::BossClusterQueryData>,
) {
    let empty = crate::BossCatalog::default();
    let boss_catalog = boss_catalog.as_deref().unwrap_or(&empty);
    for submitted in outbox.drain::<ambition_boss_special_port::BossScalePort>(&BOSS_CONDUCT) {
        match bosses.get_mut(submitted.scope) {
            Ok(mut boss) => boss.as_boss_mut().rescale(submitted.value.scale, boss_catalog),
            Err(_) => warn!(
                "extension entry {} asked to scale {:?}, which is not a boss; refused",
                submitted.entry, submitted.scope
            ),
        }
    }
}

/// Lower `ambition.boss.stage` into the boss's `BossEncounter::stage`. A
/// scope that is not a boss is refused.
pub fn lower_boss_stages(mut outbox: ResMut<ExtensionOutbox>, mut bosses: Query<&mut BossEncounter>) {
    for submitted in outbox.drain::<ambition_boss_special_port::BossStagePort>(&BOSS_CONDUCT) {
        match bosses.get_mut(submitted.scope) {
            Ok(mut status) => {
                if status.stage != submitted.value {
                    status.stage = submitted.value;
                }
            }
            Err(_) => warn!(
                "extension entry {} asked to stage {:?}, which is not a boss; refused",
                submitted.entry, submitted.scope
            ),
        }
    }
}

/// The `ambition.presentation.drawn_row` requests of this tick in the phase
/// `L`, for [`draw_rows`].
pub fn drain_drawn_rows<L: ambition_extension_host::LowersIn>(
    mut outbox: ResMut<ExtensionOutbox>,
) -> Vec<ambition_extension_host::Submitted<DrawnRow>> {
    outbox.drain::<DrawnRowPort>(&L::PHASE)
}

/// Lower `ambition.presentation.drawn_row` into the part's `PinnedRow`. The
/// adapter is `drain_drawn_rows.pipe(draw_rows)`.
pub fn draw_rows(
    In(requests): In<Vec<ambition_extension_host::Submitted<DrawnRow>>>,
    riders: Query<&ambition_mount::RidingOn>,
    rigs: Query<&ambition_characters::actor::LimbRig>,
    mut rows: Query<Option<&mut ambition_sprite_sheet::character::PinnedRow>>,
    mut commands: Commands,
) {
    for submitted in requests {
        let drawn = submitted.value;
        let part = match &drawn.on {
            DrawnPart::Body => Some(submitted.scope),
            DrawnPart::Mount => riders.get(submitted.scope).ok().map(|riding| riding.mount),
            DrawnPart::Limb(slot) => ridden_limb(submitted.scope, slot, &riders, &rigs),
            DrawnPart::Layer => Some(submitted.scope),
        };
        let Some((part, Ok(row))) = part.map(|part| (part, rows.get_mut(part))) else {
            warn!(
                "extension entry {} asked to draw {:?} of {:?}, which is not there; refused",
                submitted.entry, drawn.on, submitted.scope
            );
            continue;
        };
        let names: Vec<&str> = drawn.names.iter().map(String::as_str).collect();
        // A layer is drawn over the body's row and leaves its pin as it is.
        if drawn.on == DrawnPart::Layer {
            match row {
                Some(mut row) => row.pin_layer(&names, drawn.elapsed, drawn.looping),
                None if !names.is_empty() => {
                    let mut row = ambition_sprite_sheet::character::PinnedRow::default();
                    row.pin_layer(&names, drawn.elapsed, drawn.looping);
                    commands.entity(part).try_insert(row);
                }
                None => {}
            }
            continue;
        }
        match (row, names.is_empty()) {
            (Some(mut row), false) => row.pin(&names, drawn.elapsed, drawn.looping),
            (Some(mut row), true) => row.clear(),
            (None, false) => {
                let mut row = ambition_sprite_sheet::character::PinnedRow::default();
                row.pin(&names, drawn.elapsed, drawn.looping);
                commands.entity(part).try_insert(row);
            }
            (None, true) => {}
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

/// Lower `ambition.feedback.lightning` (a bolt that shows, not one that
/// hurts: that is a ray) into `VfxMessage::Lightning`: a body end follows the
/// asking body, and the lease key is scoped to it.
pub fn lower_lightning(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    use ambition_combat_port::{LightningPort, Spot};
    use ambition_vfx::{LightningEnd, LightningKey, LightningLook};
    for submitted in outbox.drain::<LightningPort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let bolt = submitted.value;
        let Some(mut look) = LightningLook::named(&bolt.look) else {
            warn!("extension entry {} asked for lightning that looks {:?}; refused", submitted.entry, bolt.look);
            continue;
        };
        if let Some(glow) = bolt.glow {
            look = look.tinted(glow);
        }
        let owner = submitted.scope;
        let end = |end: Spot| match end {
            Spot::World(at) => LightningEnd::World(ae::Vec2::from(at)),
            Spot::Body(offset) => LightningEnd::Body { owner, offset: ae::Vec2::from(offset) },
        };
        vfx.for_room(rooms.of(owner)).write(ambition_vfx::vfx::VfxMessage::Lightning {
            key: LightningKey::scoped(Some(owner), &bolt.key),
            from: end(bolt.from),
            to: end(bolt.to),
            look,
            hold: bolt.hold_s,
        });
    }
}

/// Lower `ambition.feedback.lightning_aura` into `VfxMessage::LightningAura`
/// on the asking body.
pub fn lower_lightning_auras(
    mut outbox: ResMut<ExtensionOutbox>,
    mut vfx: ambition_vfx::vfx::VfxWriter,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
) {
    use ambition_vfx::{LightningKey, LightningLook};
    for submitted in outbox.drain::<ambition_combat_port::LightningAuraPort>(&ambition_extension_sdk::phases::BOSS_CONDUCT) {
        let aura = submitted.value;
        let Some(mut look) = LightningLook::named(&aura.look) else {
            warn!("extension entry {} asked for an aura that looks {:?}; refused", submitted.entry, aura.look);
            continue;
        };
        if let Some(glow) = aura.glow {
            look = look.tinted(glow);
        }
        let owner = submitted.scope;
        vfx.for_room(rooms.of(owner)).write(ambition_vfx::vfx::VfxMessage::LightningAura {
            key: LightningKey::scoped(Some(owner), &aura.key),
            owner,
            center: ae::Vec2::from(aura.center),
            half: ae::Vec2::from(aura.half),
            look,
            hold: aura.hold_s,
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
            .spawn((body(ae::Vec2::new(630.0, 970.0), ae::Vec2::new(130.0, 100.0)), ambition_characters::actor::BodyHealth::new(left_health), ae::PoseOwnedExternally::only(ae::PoseHold::Conductor)))
            .id();
        let right = world
            .spawn((body(ae::Vec2::new(1170.0, 970.0), ae::Vec2::new(130.0, 100.0)), ambition_characters::actor::BodyHealth::new(Health::new(10))))
            .id();
        let mut gnu_kin = body(ae::Vec2::new(900.0, 1040.0), ae::Vec2::new(439.0, 414.0));
        gnu_kin.vel = ae::Vec2::new(0.0, -520.0);
        gnu_kin.facing = -1.0;
        let back = ambition_mount::BackLedge::new(ambition_characters::actor::MountBack { left: -208.0, right: 169.0, top: -9.75, thickness: 14.0 });
        let gnu = world
            .spawn((gnu_kin, ae::Unmirrored, back.clone(), room, LimbRig::from_pairs([(LimbSlot::HAND_LEFT, left), (LimbSlot::HAND_RIGHT, right)])))
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
                ledges: Vec::new(),
                hall: Some(RoomHall { floor: 1250.0, left: 48.0, right: 1800.0 }),
                limbs: vec![
                    ambition_boss_special_port::MountLimb {
                        slot: "hand_left".into(),
                        position: [630.0, 970.0],
                        size: [130.0, 100.0],
                        home_offset: [-270.0, -70.0],
                        lost_health: 3,
                        held: true,
                        contact: None,
                        struck: 0,
                    },
                    ambition_boss_special_port::MountLimb {
                        slot: "hand_right".into(),
                        position: [1170.0, 970.0],
                        size: [130.0, 100.0],
                        home_offset: [270.0, -70.0],
                        lost_health: 0,
                        held: false,
                        contact: None,
                        struck: 0,
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
        let request = Submitted { scope: boss, invocation: Default::default(), occurrence: None, entry: "test::gnu/conduct".into(), value: pose };
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

#[cfg(test)]
mod limb_strike_tests {
    use super::*;
    use ambition_boss_special_port::LimbStrike;
    use ambition_characters::actor::{LimbRig, LimbSlot};
    use ambition_combat::strike::{Hitbox, HitboxAnchor};
    use ambition_extension_host::Submitted;
    use bevy::ecs::system::RunSystemOnce as _;

    fn strike(world: &mut World, boss: Entity, asks: &[(&str, u32)]) {
        let requests = asks
            .iter()
            .map(|(slot, generation)| Submitted {
                scope: boss,
                invocation: Default::default(),
                occurrence: None,
                entry: "test::gnu/conduct".into(),
                value: LimbStrike { slot: (*slot).into(), generation: *generation, half_extent: [40.0, 30.0], damage: 2, knockback: 1.2 },
            })
            .collect();
        world.run_system_once_with(strike_with_limbs, requests).unwrap();
    }

    /// Each box: its owner, and whether it follows the owner's middle.
    fn boxes(world: &mut World) -> Vec<(Entity, bool)> {
        world
            .query::<&Hitbox>()
            .iter(world)
            .map(|h| (h.owner, matches!(h.anchor, HitboxAnchor::FollowOwner { local_offset } if local_offset == ae::Vec2::ZERO)))
            .collect()
    }

    /// ⭐ A LIMB CARRIES THE BOX ASKED FOR IT: it is the limb's and follows
    /// it, so a pair the limb is in cuts it. The same generation keeps it, a
    /// new one replaces it, and a tick that does not ask lets it go.
    #[test]
    fn a_limb_carries_the_box_asked_for_it_while_it_is_asked() {
        let mut world = World::new();
        let fist = world.spawn_empty().id();
        let mount = world.spawn(LimbRig::from_pairs([(LimbSlot::HAND_RIGHT, fist)])).id();
        let boss = world.spawn(ambition_mount::RidingOn { mount }).id();

        strike(&mut world, boss, &[("hand_right", 1)]);
        assert_eq!(boxes(&mut world), vec![(fist, true)]);
        let first = world.get::<LimbStrikeBox>(fist).unwrap().hitbox;
        strike(&mut world, boss, &[("hand_right", 1)]);
        assert_eq!(world.get::<LimbStrikeBox>(fist).unwrap().hitbox, first, "the same generation is the same box");
        strike(&mut world, boss, &[("hand_right", 2)]);
        assert_ne!(world.get::<LimbStrikeBox>(fist).unwrap().hitbox, first, "a new generation is a new box");
        assert_eq!(boxes(&mut world).len(), 1);
        strike(&mut world, boss, &[]);
        assert!(boxes(&mut world).is_empty() && world.get::<LimbStrikeBox>(fist).is_none(), "not asked, let go");
    }
}

#[cfg(test)]
mod limb_bond_tests {
    use super::*;
    use ambition_boss_special_port::LimbBond;
    use ambition_characters::actor::{LimbBondStart, LimbRig, LimbSlot};
    use ambition_extension_host::Submitted;
    use bevy::ecs::system::RunSystemOnce as _;

    fn bond(world: &mut World, boss: Entity, bonds: &[(&str, [f32; 2])]) {
        let requests = bonds
            .iter()
            .map(|(slot, start)| Submitted {
                scope: boss,
                invocation: Default::default(),
                occurrence: None,
                entry: "test::gnu/conduct".into(),
                value: LimbBond { slot: (*slot).into(), start: Some(*start) },
            })
            .collect();
        world.run_system_once_with(bond_limbs, requests).unwrap();
    }

    fn start(world: &World, limb: Entity) -> Option<ae::Vec2> {
        world.get::<LimbBondStart>(limb).and_then(|start| start.0)
    }

    /// ⭐ A LIMB'S BOND START IS THE ONE ASKED FOR THIS TICK. Each fist has the
    /// start asked for it, a fist not asked for has none (its bond is from
    /// the giant again), and a slot the rig does not have puts a start on no
    /// limb.
    #[test]
    fn a_limb_has_the_bond_start_asked_for_it_this_tick() {
        let mut world = World::new();
        let left = world.spawn_empty().id();
        let right = world.spawn_empty().id();
        let gnu = world.spawn(LimbRig::from_pairs([(LimbSlot::HAND_LEFT, left), (LimbSlot::HAND_RIGHT, right)])).id();
        let scholar = world.spawn(ambition_mount::RidingOn { mount: gnu }).id();

        bond(&mut world, scholar, &[("hand_left", [100.0, 800.0]), ("hand_right", [900.0, 820.0])]);
        assert_eq!((start(&world, left), start(&world, right)), (Some(ae::Vec2::new(100.0, 800.0)), Some(ae::Vec2::new(900.0, 820.0))));

        bond(&mut world, scholar, &[("hand_right", [880.0, 500.0]), ("tail", [0.0, 0.0])]);
        assert_eq!(
            (start(&world, left), start(&world, right)),
            (None, Some(ae::Vec2::new(880.0, 500.0))),
            "a fist not asked for has its bond from the giant again, and the other follows"
        );
        assert_eq!(start(&world, gnu), None, "a slot the rig does not have puts a start on no limb");

        bond(&mut world, scholar, &[]);
        assert_eq!((start(&world, left), start(&world, right)), (None, None), "nothing asked for, no start");
    }
}

#[cfg(test)]
mod drawn_row_tests {
    use super::*;
    use ambition_characters::actor::{LimbRig, LimbSlot};
    use ambition_extension_host::Submitted;
    use ambition_sprite_sheet::character::PinnedRow;
    use bevy::ecs::system::RunSystemOnce as _;

    fn draw(world: &mut World, boss: Entity, row: DrawnRow) {
        let request = Submitted { scope: boss, invocation: Default::default(), occurrence: None, entry: "test::gnu/conduct".into(), value: row };
        world.run_system_once_with(draw_rows, vec![request]).unwrap();
    }

    fn rows(world: &World, part: Entity) -> Option<Vec<String>> {
        world.get::<PinnedRow>(part).map(|row| row.rows.clone())
    }

    /// ⭐ A row is pinned on the part it names: the boss, the mount it rides,
    /// or a limb of that mount, with every fallback name kept in order; no
    /// names clears it. The control: a slot the rig does not have draws
    /// nothing anywhere.
    #[test]
    fn a_row_is_drawn_on_the_part_it_names() {
        let mut world = World::new();
        let fist = world.spawn_empty().id();
        let gnu = world.spawn((LimbRig::from_pairs([(LimbSlot::HAND_LEFT, fist)]), PinnedRow::default())).id();
        let scholar = world.spawn(ambition_mount::RidingOn { mount: gnu }).id();

        let names = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        draw(&mut world, scholar, DrawnRow { on: DrawnPart::Limb("hand_right".into()), names: names(&["hit"]), elapsed: 0.0, looping: false });
        assert_eq!((rows(&world, scholar), rows(&world, gnu), rows(&world, fist)), (None, Some(Vec::new()), None), "a missing slot drew a row");

        draw(&mut world, scholar, DrawnRow { on: DrawnPart::Limb("hand_left".into()), names: names(&["hit", "rest"]), elapsed: 0.0, looping: false });
        draw(&mut world, scholar, DrawnRow { on: DrawnPart::Mount, names: names(&["buck"]), elapsed: 0.3, looping: false });
        draw(&mut world, scholar, DrawnRow::body("tumble", 0.1, true));
        assert_eq!(rows(&world, fist), Some(names(&["hit", "rest"])), "the limb keeps its fallback");
        assert_eq!(rows(&world, gnu), Some(names(&["buck"])));
        assert_eq!(rows(&world, scholar), Some(names(&["tumble"])));
        let pinned = world.get::<PinnedRow>(gnu).unwrap();
        assert_eq!((pinned.elapsed, pinned.looping), (0.3, false));

        draw(&mut world, scholar, DrawnRow { on: DrawnPart::Mount, names: Vec::new(), elapsed: 0.0, looping: false });
        assert_eq!(rows(&world, gnu), Some(Vec::new()), "no names draws the mount as the engine would");
        assert_eq!(rows(&world, scholar), Some(names(&["tumble"])), "the mount's clear left the rider alone");
    }
}

#[cfg(test)]
mod launch_and_bubble_tests {
    use super::*;
    use ambition_extension_host::Submitted;
    use bevy::ecs::system::RunSystemOnce as _;

    fn submitted<T>(scope: Entity, value: T) -> Vec<Submitted<T>> {
        vec![Submitted { scope, invocation: Default::default(), occurrence: None, entry: "test::gnu/conduct".into(), value }]
    }

    /// ⭐ The launch is staged on the mount, not on its rider. The control: a
    /// boss that rides nothing stages nothing anywhere.
    #[test]
    fn a_mount_launch_is_staged_on_the_ridden_body() {
        use ambition_boss_special_port::MountLaunch;
        let mut world = World::new();
        let gnu = world.spawn(ae::BodyFlightState::default()).id();
        let scholar = world.spawn((ae::BodyFlightState::default(), ambition_mount::RidingOn { mount: gnu })).id();
        let walker = world.spawn(ae::BodyFlightState::default()).id();
        let hop = MountLaunch { velocity: [0.0, -520.0], flinchless: true };

        world.run_system_once_with(launch_mounts, submitted(walker, hop)).unwrap();
        let staged = |world: &World, body: Entity| world.get::<ae::BodyFlightState>(body).unwrap().pending_launch_state();
        assert_eq!(staged(&world, walker).velocity, ae::Vec2::ZERO, "a boss that rides nothing launched itself");

        world.run_system_once_with(launch_mounts, submitted(scholar, hop)).unwrap();
        let launch = staged(&world, gnu);
        assert_eq!((launch.velocity, launch.flinchless), (ae::Vec2::new(0.0, -520.0), true));
        assert_eq!(staged(&world, scholar).velocity, ae::Vec2::ZERO, "the rider is not launched");
    }

    /// ⭐ The bubble is written once, with its text, in the room of the body.
    #[test]
    fn a_speech_bubble_is_written_in_the_room_of_the_body() {
        use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
        let mut world = World::new();
        world.init_resource::<bevy::ecs::message::Messages<ambition_vfx::vfx::VfxInRoom>>();
        let scholar = world.spawn(InRoomInstance(LiveRoomInstance::ACTIVATION)).id();
        let bubble = ambition_combat_port::SpeechBubble { at: [900.0, 700.0], text: "Eureka!".into() };
        world.run_system_once_with(show_speech_bubbles, submitted(scholar, bubble)).unwrap();
        let messages = world.resource::<bevy::ecs::message::Messages<ambition_vfx::vfx::VfxInRoom>>();
        let written: Vec<_> = messages.iter_current_update_messages().collect();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].room, Some(LiveRoomInstance::ACTIVATION));
        match &written[0].vfx {
            ambition_vfx::vfx::VfxMessage::SpeechBubble { pos, text } => {
                assert_eq!((*pos, text.as_str()), (ae::Vec2::new(900.0, 700.0), "Eureka!"));
            }
            other => panic!("not a speech bubble: {other:?}"),
        }
    }
}
