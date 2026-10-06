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
        let hall = rooms
            .of(boss)
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
            },
        );
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
    for submitted in outbox.drain::<ConductedPosePort>() {
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

/// Lower `ambition.presentation.drawn_row` into the body's `PinnedRow`.
pub fn lower_drawn_rows(
    mut outbox: ResMut<ExtensionOutbox>,
    mut rows: Query<Option<&mut ambition_sprite_sheet::character::PinnedRow>>,
    mut commands: Commands,
) {
    for submitted in outbox.drain::<DrawnRowPort>() {
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
    use ambition_vfx::vfx::ParticleKind;
    for submitted in outbox.drain::<BurstPort>() {
        let burst = submitted.value;
        let kind = match burst.kind.as_str() {
            "spark" => ParticleKind::Spark,
            "dust" => ParticleKind::Dust,
            "shard" => ParticleKind::Shard,
            "heart" => ParticleKind::Heart,
            other => {
                warn!("extension entry {} asked for a burst of {other:?}; refused", submitted.entry);
                continue;
            }
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
    for submitted in outbox.drain::<ambition_combat_port::CameraShakePort>() {
        shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
            amplitude_px: submitted.value.amplitude_px,
        });
    }
}

/// Turn a conducted boss to the side its module chose, through the control
/// the body integrator applies. Only while the module holds the pose: a
/// driven boss faces where its participant steers it. Runs in
/// `BossSteerSlot`, after the brain and before the integration.
pub fn face_conducted_bosses(
    mut bosses: Query<(&ConductedFacing, &mut ambition_characters::control::ActorControl), With<ae::PoseOwnedExternally>>,
) {
    for (facing, mut control) in &mut bosses {
        control.0.facing = facing.0;
    }
}
