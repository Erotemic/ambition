//! Petting: a conversation's `<<pet>>` pets the character being talked to.
//!
//! A pet is a short script with two stages, recorded as one [`PetBeat`] on the
//! petter. First the petter walks to the petted body's front, by the
//! body-generic [`CommandedMove`] (the walk an encounter lures a boss with),
//! and the petted body waits. Then the gesture plays for [`PET_SECONDS`].
//!
//! ⭐ THE BEAT IS THE PET'S ONE RECORD. It owns both stages and the time left
//! in each. [`project_pet_holds`] derives everything else from it on the same
//! tick: both bodies' `ControlHold::Gesture`, and the `petting` and `petted`
//! facts that `body_state_clip` draws the two rows from. It runs in the same
//! chain, right after [`advance_pet_beats`], so the tick that starts, ends or
//! interrupts a beat also changes the holds, before the next tick's control
//! gate reads them. When the beat goes, for any reason, nothing is left on
//! either body: there is no second timer to run down on a survivor. A
//! rollback restores the beat, and the next projection restores the rest.
//!
//! A pet can be interrupted, as a conversation can: a hit that moves either
//! body, either body going away, or a walk that cannot arrive ends the beat
//! and lets both go ([`advance_pet_beats`]).
//!
//! Whether a character can be petted is authored on its catalog row
//! (`petting`). The dialogue offers the pet as a choice, so Interact always
//! talks and the pet does not take the conversation's place.

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::BodyAnimFacts;
use ambition_characters::actor::BodyCombat;
use ambition_characters::control::{
    claim_control_hold, release_control_hold, CommandedMove, ControlHold, ControlHolds,
};
use ambition_combat::components::{ActorInteraction, CenteredAabb};
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveBodies, LiveBodyId, LiveRoomInstance,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use ambition_sfx::{SfxId, SfxMessage, SfxWriter};
use ambition_vfx::vfx::VfxMessage;
use ambition_vfx::vfx::VfxWriter;
use bevy::prelude::*;

/// How long a pet lasts. The petter's `pet` row is authored to this length.
pub const PET_SECONDS: f32 = 2.0;

/// The gap between the petter and the petted body's side, in world units.
/// The petter stands just off the petted body's front, where its bowed head is.
const PET_STANDOFF: f32 = 6.0;

/// How fast the petter walks to the petted body's front, in px/s: a walk and
/// not a run, because the walk is part of the gesture.
pub const PET_WALK_SPEED: f32 = 90.0;

/// How near the mark the petter must stand for the gesture to start.
const PET_ARRIVE_TOLERANCE: f32 = 2.0;

/// The time a walk may take past its distance at [`PET_WALK_SPEED`]. A walk
/// that has not arrived by then is blocked (a wall, a ledge), and the pet ends.
const PET_WALK_SLACK_SECONDS: f32 = 1.0;

/// How far from its mark the petter may be moved while the gesture plays. A
/// body pushed farther is not petting any more.
const PET_REACH_SLACK: f32 = 16.0;

/// A pet in progress: on the petter, from the request until the gesture ends
/// or is interrupted.
///
/// Rollback state, so a rewind into the walk resumes the walk.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PetBeat {
    /// The body being petted, by live identity.
    pub petted: LiveBodyId,
    /// Where the petter stands to pet: the petted body's front, in world x.
    pub mark_x: f32,
    /// The side of the petted body the petter stands on: `1.0` right.
    pub side: f32,
    pub stage: PetStage,
}

/// The two stages of a [`PetBeat`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PetStage {
    /// The petter walks to the mark. `remaining` is the time the walk may
    /// still take.
    Walk { remaining: f32 },
    /// The gesture plays. `remaining` is its time left.
    Gesture { remaining: f32 },
}

/// A conversation asked one body to pet another, by live identity.
///
/// Routed by identity for the reason [`crate::features::ChallengeRequested`]
/// is: the narrative ledger releases it again on every replay of its tick, and
/// an `Entity` is not stable across that. The identity is the stable id in its
/// live room, because two instances of one room hold the same stable ids.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct PetRequested {
    /// The body that pets: the one who started the conversation.
    pub petter: LiveBodyId,
    /// The body that is petted: the one being talked to.
    pub petted: LiveBodyId,
}

/// Pin a body where it stands with no side speed, keeping its fall: the pet
/// beat holds both bodies, and a held body is pinned by the motion authority,
/// not by a bare velocity write.
fn stop_where_it_stands(kinematics: &mut ambition_platformer2d_core::BodyKinematics) {
    let at = kinematics.pos;
    let fall = kinematics.vel.y;
    ambition_platformer2d_core::movement::constrain_body_pose(
        kinematics,
        None,
        at,
        ambition_platformer2d_core::Vec2::new(0.0, fall),
    );
}

/// Start the pet a conversation asked for: the petter walks to the petted
/// body's front. (sim)
///
/// The petted body's catalog row must author `petting`, because only such a
/// character has a petted animation row and a sound for it. The petted body
/// turns to face the petter and stops where it stands. The petter gets a
/// [`CommandedMove`] to the mark and a [`PetBeat`]; [`advance_pet_beats`]
/// starts the gesture when it arrives.
///
/// The mark is on the petter's side of the petted body when the petter's body
/// fits there, and on the other side when only that one fits: a dog beside a
/// wall is petted from the open side. With no room on either side, there is
/// no pet.
#[allow(clippy::too_many_arguments)]
pub fn apply_pet_requests(
    mut commands: Commands,
    mut requests: MessageReader<PetRequested>,
    ids: LiveBodies,
    catalog: Res<CharacterCatalog>,
    pettable: Query<(&CenteredAabb, &ActorInteraction)>,
    beats: Query<&PetBeat>,
    rooms: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    mut bodies: Query<&mut BodyKinematics>,
) {
    for request in requests.read() {
        let entity_of = |wanted: &LiveBodyId| ids.entity_of(wanted);
        let (Some(petter), Some(petted)) = (entity_of(&request.petter), entity_of(&request.petted))
        else {
            warn!(
                target: "crate::features::pet",
                "a pet of {} by {} names no live body; ignoring",
                request.petted,
                request.petter,
            );
            continue;
        };
        let Ok((aabb, interaction)) = pettable.get(petted) else {
            warn!(
                target: "crate::features::pet",
                "{} is not a character that can be talked to; ignoring the pet",
                request.petted,
            );
            continue;
        };
        if ambition_conversation::character_id_of(&interaction.interactable)
            .and_then(|character| catalog.get(character))
            .and_then(|row| row.petting.as_ref())
            .is_none()
        {
            warn!(
                target: "crate::features::pet",
                "{} has no `petting` on its catalog row; ignoring the pet",
                request.petted,
            );
            continue;
        };
        // One beat per body: a body already petting, being petted, or walking
        // to either keeps the beat it has.
        let busy = |body: Entity, id: &LiveBodyId| {
            beats.get(body).is_ok() || beats.iter().any(|beat| beat.petted == *id)
        };
        if busy(petter, &request.petter) || busy(petted, &request.petted) {
            continue;
        }
        let Ok([petter_kin, mut petted_kin]) = bodies.get_many_mut([petter, petted]) else {
            continue;
        };
        // The petter's own room's solids: the mark is a place its body must
        // fit, in the room it walks in.
        let solids = collision
            .room(rooms.get(petter).ok())
            .and_then(|room| room.solids());
        let mark_for = |side: f32| {
            aabb.center.x + side * (aabb.half_size.x + petter_kin.size.x * 0.5 + PET_STANDOFF)
        };
        let fits = |side: f32| {
            let body = ambition_platformer2d_core::Aabb::new(
                ambition_platformer2d_core::Vec2::new(mark_for(side), petter_kin.pos.y),
                petter_kin.size * 0.5,
            );
            solids.as_deref().is_none_or(|world| {
                !world.body_overlaps_any(body, |block| {
                    matches!(
                        block.kind,
                        ambition_platformer2d_core::BlockKind::Solid
                            | ambition_platformer2d_core::BlockKind::BlinkWall { .. }
                            | ambition_platformer2d_core::BlockKind::Hazard
                            | ambition_platformer2d_core::BlockKind::Rebound { .. }
                    )
                })
            })
        };
        let near = if petter_kin.pos.x >= aabb.center.x {
            1.0
        } else {
            -1.0
        };
        let Some(side) = [near, -near].into_iter().find(|side| fits(*side)) else {
            warn!(
                target: "crate::features::pet",
                "no room beside {} for {} to stand and pet it; ignoring the pet",
                request.petted,
                request.petter,
            );
            continue;
        };
        // The petted body turns to that side now, and waits there.
        petted_kin.facing = side;
        // It stops where it stands: the hold blanks its control from the next
        // tick, and a walking dog would otherwise slide off the mark on its
        // momentum. A pin, through the motion authority (ADR 0024).
        stop_where_it_stands(&mut petted_kin);
        let mark_x = mark_for(side);
        let walk = (mark_x - petter_kin.pos.x).abs() / PET_WALK_SPEED;
        commands.entity(petter).insert((
            CommandedMove {
                target: ambition_platformer2d_core::Vec2::new(mark_x, petter_kin.pos.y),
                speed: PET_WALK_SPEED,
                arrive_tolerance: PET_ARRIVE_TOLERANCE,
            },
            PetBeat {
                petted: request.petted.clone(),
                mark_x,
                side,
                stage: PetStage::Walk {
                    remaining: walk + PET_WALK_SLACK_SECONDS,
                },
            },
        ));
    }
}

/// Move every pet along: start the gesture when the walk arrives, end the
/// beat when the gesture has played, and interrupt it when its assumptions
/// break. (sim)
///
/// A pet assumes both bodies stand still for it. A hit that moves either body
/// ([`BodyCombat::is_knocked`], the rule a conversation breaks on), either
/// body going away, a walk that does not arrive in time, or a petter pushed
/// off its mark during the gesture breaks that, and the beat ends at once. The projection after it lets
/// both bodies go on the same tick.
#[allow(clippy::too_many_arguments)]
pub fn advance_pet_beats(
    mut commands: Commands,
    world_time: Res<ambition_time::WorldTime>,
    catalog: Res<CharacterCatalog>,
    ids: LiveBodies,
    pettable: Query<(&CenteredAabb, &ActorInteraction)>,
    mut petters: Query<(Entity, &mut PetBeat)>,
    mut bodies: Query<(&mut BodyKinematics, Option<&BodyCombat>)>,
    mut sfx: SfxWriter,
    mut vfx: VfxWriter,
) {
    let dt = world_time.scaled_dt;
    for (petter, mut beat) in &mut petters {
        let petted = ids.entity_of(&beat.petted);
        let pair = petted.and_then(|petted| bodies.get_many_mut([petter, petted]).ok());
        let Some(
            [(mut petter_kin, petter_combat), (mut petted_kin, petted_combat)],
        ) = pair
        else {
            // A body went away (despawned, or the room was replaced).
            end_pet_beat(&mut commands, petter);
            continue;
        };
        let knocked = petter_combat.is_some_and(BodyCombat::is_knocked)
            || petted_combat.is_some_and(BodyCombat::is_knocked);
        match beat.stage {
            PetStage::Walk { remaining } => {
                let remaining = remaining - dt;
                let arrived = (petter_kin.pos.x - beat.mark_x).abs() <= PET_ARRIVE_TOLERANCE;
                if knocked || (!arrived && remaining <= 0.0) {
                    end_pet_beat(&mut commands, petter);
                    continue;
                }
                if !arrived {
                    beat.stage = PetStage::Walk { remaining };
                    continue;
                }
                let Some((center, petting)) = petted
                    .and_then(|petted| pettable.get(petted).ok())
                    .and_then(|(aabb, interaction)| {
                        ambition_conversation::character_id_of(&interaction.interactable)
                            .and_then(|character| catalog.get(character))
                            .and_then(|row| row.petting.as_ref())
                            .map(|petting| (*aabb, petting))
                    })
                else {
                    end_pet_beat(&mut commands, petter);
                    continue;
                };
                // The walk is over: the petter faces back across the petted
                // body, and both stop where they stand.
                commands.entity(petter).remove::<CommandedMove>();
                petter_kin.facing = -beat.side;
                petted_kin.facing = beat.side;
                stop_where_it_stands(&mut petter_kin);
                stop_where_it_stands(&mut petted_kin);
                beat.stage = PetStage::Gesture {
                    remaining: PET_SECONDS,
                };
                if let Some(sound) = &petting.sound {
                    sfx.write(SfxMessage::Play {
                        id: SfxId::new(sound),
                        pos: center.center,
                    });
                }
                vfx.write(VfxMessage::Hearts {
                    pos: center.center
                        + ambition_platformer2d_core::Vec2::new(
                            beat.side * center.half_size.x,
                            -center.half_size.y,
                        ),
                    count: 5,
                });
            }
            PetStage::Gesture { remaining } => {
                let remaining = remaining - dt;
                let pushed_off = (petter_kin.pos.x - beat.mark_x).abs() > PET_REACH_SLACK;
                if knocked || pushed_off || remaining <= 0.0 {
                    end_pet_beat(&mut commands, petter);
                } else {
                    beat.stage = PetStage::Gesture { remaining };
                }
            }
        }
    }
}

/// End a pet beat on `petter`: its walk and its record go.
fn end_pet_beat(commands: &mut Commands, petter: Entity) {
    commands.entity(petter).remove::<(CommandedMove, PetBeat)>();
}

/// Derive each body's part in a pet from the beats, on the tick the beats
/// change. (sim)
///
/// A petter (the body with a [`PetBeat`]) and the body its beat names are
/// held (`ControlHold::Gesture`) for the whole beat: the petter is walked by
/// its script and the petted body waits. While the gesture plays, the petter
/// is `petting` and the petted body is `petted`. A body in no beat has none of
/// these. The petted body is matched by live identity, so a duplicate in
/// another live room is not held.
///
/// ⛔ IN THE PET'S OWN CHAIN, after [`advance_pet_beats`], and not in the
/// feature phase's hold projection, which runs before the beats move. There,
/// a new pet took one tick of ordinary control, and a hit or an end kept both
/// bodies held one tick more.
pub fn project_pet_holds(
    mut commands: Commands,
    beats: Query<&PetBeat>,
    mut bodies: Query<(
        Entity,
        &mut BodyAnimFacts,
        Option<&SimId>,
        (Option<&InRoomInstance>, Option<&LiveRoomInstance>),
        Option<&mut ControlHolds>,
    )>,
) {
    for (entity, mut anim, id, (stamp, root), holds) in &mut bodies {
        let own = beats.get(entity).ok();
        let petted_by = id.and_then(|id| beats.iter().find(|beat| beat.petted.is(id, stamp, root)));
        let gesture = |beat: Option<&PetBeat>| {
            beat.is_some_and(|beat| matches!(beat.stage, PetStage::Gesture { .. }))
        };
        let (petting, petted) = (gesture(own), gesture(petted_by));
        if anim.petting != petting || anim.petted != petted {
            anim.petting = petting;
            anim.petted = petted;
        }
        let in_a_beat = own.is_some() || petted_by.is_some();
        let held = holds
            .as_ref()
            .is_some_and(|holds| holds.holds(ControlHold::Gesture));
        match (in_a_beat, held) {
            (true, false) => claim_control_hold(&mut commands, entity, ControlHold::Gesture),
            (false, true) => release_control_hold(
                &mut commands,
                entity,
                holds.map(|holds| holds.into_inner()),
                ControlHold::Gesture,
            ),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
