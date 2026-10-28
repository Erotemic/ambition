//! Spacing for a conversation: the body that starts a conversation steps to a
//! talking distance from the body it talks to, when the step is safe.
//!
//! Interact opens a conversation from anywhere in talk reach, which includes
//! a body that stands on the other one. Two bodies drawn one over the other
//! do not read as two characters that talk. So, as a pet does
//! ([`super::pet`]), the conversation walks its initiator to a mark: here,
//! [`TALK_GAP`] from the other body's side: on the initiator's own side, or
//! on the other side when only that one is safe. The other body turns to face
//! it and waits, held by the conversation.
//!
//! ⭐ THE STEP IS OPTIONAL, THE CONVERSATION IS NOT. The box opens on the
//! press whatever this decides. The initiator does not move when the mark is
//! not a safe place to stand on either side: a solid, a hazard or a rebound
//! block at the mark or on the way, or no floor under the way. Then both
//! bodies only turn to face each other.
//!
//! ⭐ ONE DECISION PER CONVERSATION. [`TalkSpacing`] is put on the initiator
//! when the conversation opens and stays until it closes, so a walk that ends
//! early (blocked, or pushed) is not started again. A pet during the
//! conversation owns the initiator's walk ([`super::PetBeat`]); the spacing
//! then stops and does not touch the pet's [`CommandedMove`].
//!
//! The mark is inside talk reach ([`ambition_interaction::TALK_REACH`]), so
//! the step does not break the conversation it is for.

use ambition_characters::control::CommandedMove;
use ambition_combat::components::CenteredAabb;
use ambition_conversation::ActiveConversation;
use ambition_platformer2d_core::{BodyKinematics, Vec2};
use bevy::prelude::*;

/// The gap between two bodies that talk, in world units, on the side axis of
/// the initiator's frame.
pub const TALK_GAP: f32 = 16.0;

/// How fast the initiator steps to its mark, in px/s: the pet's walk.
const TALK_WALK_SPEED: f32 = super::PET_WALK_SPEED;

/// How near the mark the initiator must stand to have arrived.
const TALK_ARRIVE_TOLERANCE: f32 = 2.0;

/// The time a step may take past its distance at [`TALK_WALK_SPEED`]. A step
/// that has not arrived by then stops where it is.
const TALK_WALK_SLACK_SECONDS: f32 = 0.5;

/// The initiator's part in a conversation's spacing, from the tick the
/// conversation opens until it closes. On the initiator.
///
/// Rollback state, so a rewind into the step resumes the step, and one before
/// the conversation does not keep a record the resimulation has not made.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TalkSpacing {
    /// Where the initiator stands to talk, as a coordinate on the side axis
    /// of its own frame (world x in normal gravity).
    pub mark: f32,
    /// The side of the other body the initiator is on, as a sign on that
    /// axis. It is the facing the other body takes to look at the initiator.
    pub side: f32,
    /// The time the step may still take. Zero when the initiator does not
    /// step, or the step has ended.
    pub walking: f32,
}

/// Space the two bodies of a conversation: start the initiator's step when
/// the conversation opens, end it on arrival, and forget it when the
/// conversation closes. (sim)
///
/// It reads only rollback state ([`ActiveConversation`], [`TalkSpacing`] and
/// the bodies), so a rewind needs no message to start the step again.
#[allow(clippy::too_many_arguments)]
pub fn space_the_talkers(
    mut commands: Commands,
    // `Option`: no conversation authority (`Capability::Dialogue` omitted) means
    // nobody is talking; the step only forgets records it already holds.
    conversation: Option<Res<ActiveConversation>>,
    world_time: Res<ambition_time::WorldTime>,
    rooms: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    frames: Query<&ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame>,
    boxes: Query<&CenteredAabb>,
    grounds: Query<&ambition_platformer2d_core::BodyGroundState>,
    pets: Query<(), With<super::PetBeat>>,
    mut spacings: Query<(Entity, &mut TalkSpacing)>,
    mut bodies: Query<&mut BodyKinematics>,
) {
    let pair = conversation
        .as_deref()
        .and_then(|conversation| conversation.initiator().zip(conversation.talker()));
    // A record whose conversation has closed goes, with its step. A pet owns
    // the walk it put on the body, so that walk stays.
    for (body, spacing) in &spacings {
        if pair.is_some_and(|(initiator, _)| initiator == body) {
            continue;
        }
        commands.entity(body).remove::<TalkSpacing>();
        if spacing.walking > 0.0 && !pets.contains(body) {
            commands.entity(body).remove::<CommandedMove>();
        }
    }
    let Some((initiator, other)) = pair else {
        return;
    };
    let frame = super::pet::pet_frame(&frames, initiator);
    if let Ok((_, mut spacing)) = spacings.get_mut(initiator) {
        if spacing.walking <= 0.0 {
            return;
        }
        // The pet put its own walk on the body: it is the pet's from now.
        if pets.contains(initiator) {
            spacing.walking = 0.0;
            return;
        }
        let Ok([mut initiator_kin, mut other_kin]) = bodies.get_many_mut([initiator, other]) else {
            return;
        };
        let arrived = (initiator_kin.pos.dot(frame.side) - spacing.mark).abs() <= TALK_ARRIVE_TOLERANCE;
        let remaining = spacing.walking - world_time.sim_dt();
        if !arrived && remaining > 0.0 {
            spacing.walking = remaining;
            return;
        }
        // The step is over: the initiator stops and faces back across.
        spacing.walking = 0.0;
        commands.entity(initiator).remove::<CommandedMove>();
        super::pet::stop_where_it_stands(&mut initiator_kin, frame.down);
        initiator_kin.facing = -spacing.side;
        other_kin.facing = spacing.side;
        return;
    }
    // The conversation has just opened.
    let (Ok(other_box), Ok([mut initiator_kin, mut other_kin])) =
        (boxes.get(other), bodies.get_many_mut([initiator, other]))
    else {
        return;
    };
    let on_side = |pos: Vec2| pos.dot(frame.side);
    let here = on_side(initiator_kin.pos);
    let mark_for = |side: f32| {
        on_side(other_box.center)
            + side
                * (super::pet::half_on(other_box.half_size, frame.side)
                    + initiator_kin.size.x * 0.5
                    + TALK_GAP)
    };
    let level = initiator_kin.pos.dot(frame.down);
    let place = |mark: f32| frame.side * mark + frame.down * level;
    let half = initiator_kin.half_oriented(frame.down);
    let grounded = grounds.get(initiator).is_ok_and(|ground| ground.on_ground);
    let solids = collision
        .room(rooms.get(initiator).ok())
        .and_then(|room| room.solids());
    let safe = |mark: f32| {
        grounded
            && solids.as_deref().is_some_and(|world| {
                step_is_safe(world, initiator_kin.pos, place(mark), half, frame.down)
            })
    };
    // The initiator's own side first. A body beside a wall is talked to from
    // its open side, as a pet is.
    let near = if here >= on_side(other_box.center) { 1.0 } else { -1.0 };
    let step = if (mark_for(near) - here).abs() <= TALK_ARRIVE_TOLERANCE {
        None
    } else {
        [near, -near].into_iter().find(|side| safe(mark_for(*side)))
    };
    let side = step.unwrap_or(near);
    // With no step, the initiator stands where it is.
    let mark = if step.is_some() { mark_for(side) } else { here };
    // The two face each other now, whether or not the initiator steps.
    other_kin.facing = side;
    initiator_kin.facing = -side;
    let walking = if step.is_some() {
        // The other body waits where it is, as a petted body does.
        super::pet::stop_where_it_stands(&mut other_kin, frame.down);
        commands.entity(initiator).insert(CommandedMove {
            target: place(mark),
            speed: TALK_WALK_SPEED,
            arrive_tolerance: TALK_ARRIVE_TOLERANCE,
        });
        (mark - here).abs() / TALK_WALK_SPEED + TALK_WALK_SLACK_SECONDS
    } else {
        0.0
    };
    commands.entity(initiator).insert(TalkSpacing {
        mark,
        side,
        walking,
    });
}

/// Whether a body with half extents `half` can walk on the floor from `from`
/// to `to`: nothing solid, hazardous or springy on the way or at the end, and
/// a floor under it at each half body width of the way.
fn step_is_safe(
    world: &ambition_platformer2d_core::World,
    from: Vec2,
    to: Vec2,
    half: Vec2,
    down: Vec2,
) -> bool {
    use ambition_platformer2d_core::{collision_semantics, Aabb, BlockKind};
    let swept = Aabb::new((from + to) * 0.5, half + ((to - from) * 0.5).abs());
    let blocked = world.body_overlaps_any(swept, |block| {
        collision_semantics::is_full_collision_surface(block.kind)
            || matches!(block.kind, BlockKind::Hazard | BlockKind::Rebound { .. })
    });
    if blocked {
        return false;
    }
    // Half a body width along the way, so no gap the body can fall into is
    // between two samples.
    let width = super::pet::half_on(half, (to - from).normalize_or_zero());
    let steps = ((to - from).length() / width.max(1.0)).ceil().max(1.0) as usize;
    (0..=steps).all(|step| {
        let at = from.lerp(to, step as f32 / steps as f32);
        collision_semantics::supporting_block(world, Aabb::new(at, half), down, false).is_some()
    })
}

#[cfg(test)]
mod tests;
