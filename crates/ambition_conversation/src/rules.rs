//! Conversation continuity rules and cut notifications. Dialogue does not
//! suspend body simulation, so participant contact and damage can end a live
//! conversation, and a threat can break into one.
//!
//! A conversation holds any number of bodies: the pair that opened it and
//! whoever joined. Each member stays while it can still be part of it, and
//! leaves on its own when it cannot — struck, gone (dead or despawned), or
//! out of reach. The conversation ends when its initiator leaves or fewer
//! than two bodies are left.

use bevy::prelude::*;

use ambition_characters::actor::{BodyCombat, BodyHealth};
use ambition_combat::components::{ActorAggression, ActorDisposition};
use ambition_dialog::DialogueBreak;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_platformer2d_shared_tangle::sim_id::SimId;
// Use the core re-export to avoid adding a direct geometry dependency.
use ambition_platformer2d_core::{AabbExt, CenteredAabb};

use super::authority::{ActiveConversation, Leaving};

/// How near a body that JOINED a conversation must keep to one of its
/// members, and how near a threat comes before it breaks in: within
/// earshot. Wider than the talk reach, which is the opening pair's: two
/// bodies talk face to face, and a third stands in the group.
pub const EARSHOT: f32 = 128.0;

/// The suffix of the node a conversation goes on to when a threat breaks
/// into it: `creator_intro` goes on to `creator_intro__interrupted`. A
/// conversation whose node has no such node authored keeps the plain rule —
/// a threat breaks it by striking somebody.
pub const INTERRUPTED_SUFFIX: &str = "__interrupted";

/// The box within earshot of a body's footprint.
fn earshot_of(footprint: ambition_platformer2d_core::Aabb) -> ambition_platformer2d_core::Aabb {
    let grow = ambition_platformer2d_core::Vec2::splat(EARSHOT);
    ambition_platformer2d_core::Aabb {
        min: footprint.min - grow,
        max: footprint.max + grow,
    }
}

/// Requests a bark from the body whose conversation was cut. This continuity
/// layer names the speaker; cast/content code chooses the line.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationCutBark {
    /// The body that should speak — the one carrying an `ActorInteraction`, so
    /// its character id and therefore its voice can be found.
    pub speaker: Entity,
}

/// Why a member leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Why {
    /// Despawned, dead, or the room swapped under the conversation.
    Gone,
    /// The DialogueBreak rules: knocked back, or out of reach.
    Break(DialogueBreak),
}

pub fn break_dialogue_on_hit_or_separation(
    mut conversation: ResMut<ActiveConversation>,
    bodies: Query<(
        &CenteredAabb,
        Option<&BodyCombat>,
        Option<(
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ambition_platformer2d_core::SweepSample>,
        )>,
        Option<&BodyHealth>,
    )>,
    mut barks: MessageWriter<ConversationCutBark>,
) {
    let Some(live) = conversation.live() else {
        return;
    };
    let members: Vec<Entity> = live.bodies().collect();
    if members.len() < 2 {
        // Scripted dialogue with fewer than two in-world bodies. Nothing here
        // can walk away from anything.
        return;
    }
    let initiator = live.initiator;
    let talker = live.talker;

    //  KNOCKBACK, not damage: see `BodyCombat::is_knocked`, the rule every
    // beat two bodies share asks.
    let mut why: Vec<Option<Why>> = members
        .iter()
        .map(|member| match bodies.get(*member) {
            Err(_) => Some(Why::Gone),
            Ok((_, _, _, Some(health))) if !health.alive() => Some(Why::Gone),
            Ok((_, Some(combat), _, _)) if combat.is_knocked() => Some(Why::Break(DialogueBreak::Struck)),
            Ok(_) => None,
        })
        .collect();

    // Who is still in it: the first member standing, and everyone linked to
    // it. The opening pair is linked by the reach Interact opened it in —
    // the talk reach of the initiator's collision box against the talker's
    // footprint. A body with no kinematics talks from its footprint. A body
    // that joined is linked by earshot of any member still in it.
    let footprint = |member: Entity| bodies.get(member).ok().map(|(aabb, ..)| aabb.aabb());
    let talk_box = |member: Entity| {
        bodies.get(member).ok().map(|(aabb, _, body, _)| match body {
            Some((kin, step)) => ambition_interaction::talk_reach(
                kin.aabb(),
                step.map_or(ambition_platformer2d_core::Vec2::ZERO, |step| step.down),
            ),
            None => ambition_interaction::talk_reach(aabb.aabb(), ambition_platformer2d_core::Vec2::ZERO),
        })
    };
    let mut linked = vec![false; members.len()];
    if let Some(root) = why.iter().position(Option::is_none) {
        linked[root] = true;
        // The talker is linked to the initiator by the talk reach alone.
        if let (Some(i), Some(t)) = (
            initiator.and_then(|body| members.iter().position(|m| *m == body)),
            talker.and_then(|body| members.iter().position(|m| *m == body)),
        ) {
            if linked[i] && why[t].is_none() {
                linked[t] = matches!((talk_box(members[i]), footprint(members[t])), (Some(reach), Some(other)) if reach.strict_intersects(other));
            }
        }
        let joined_from = members.len() - live.joined.len();
        loop {
            let mut grew = false;
            for at in joined_from..members.len() {
                if linked[at] || why[at].is_some() {
                    continue;
                }
                let Some(own) = footprint(members[at]) else {
                    continue;
                };
                let heard = (0..members.len()).any(|other| {
                    linked[other] && footprint(members[other]).is_some_and(|other| earshot_of(other).strict_intersects(own))
                });
                if heard {
                    linked[at] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
    }
    for (at, why) in why.iter_mut().enumerate() {
        if why.is_none() && !linked[at] {
            *why = Some(Why::Break(DialogueBreak::Separated));
        }
    }

    // The initiator first: its leaving ends the conversation for everybody.
    let mut order: Vec<usize> = (0..members.len()).collect();
    order.sort_by_key(|at| Some(members[*at]) != initiator);
    for at in order {
        let Some(why) = why[at] else {
            continue;
        };
        if conversation.leave(members[at]) != Leaving::Closed {
            continue;
        }
        //  only for the break that has no voice yet. A conversation broken by a
        // HIT already barks — `npc_hit_bark_line` fires on every strike and falls
        // back to a generic line when a character authored none — so adding a second
        // bubble for one event would be worse than none. A body that is gone says
        // nothing at all. `wants_its_own_bark` is where that lives, beside the
        // reason it is about.
        //
        // and this ASKS rather than answers. Which line, from which pool, with which fallback, is a
        // CAST question; see [`ConversationCutBark`].
        if let Why::Break(reason) = why {
            if reason.wants_its_own_bark() {
                barks.write(ConversationCutBark { speaker: members[at] });
            }
        }
        return;
    }
}

/// A hostile body comes within earshot of a conversation whose node has an
/// interruption authored (`<node>__interrupted`): it breaks in. It joins —
/// held like any member, so it speaks its piece before it fights — and the
/// narrative goes on from the interruption node. Every other hostile that
/// arrives while that node runs joins it.
///
/// A body counts as a threat by the rule that already refuses it a
/// conversation (`ActorDisposition::refuses_talk`): it is fighting, and not
/// on the talker's side.
pub fn interrupt_a_conversation_on_a_threat(
    mut conversation: ResMut<ActiveConversation>,
    tick: Option<Res<ambition_time::SimTick>>,
    nodes: Res<ambition_dialog::DialogueNodeIndex>,
    rooms: LiveRooms,
    threats: Query<(
        Entity,
        &CenteredAabb,
        &ActorDisposition,
        Option<&ActorAggression>,
        Option<&BodyHealth>,
        Option<&SimId>,
    )>,
    footprints: Query<&CenteredAabb>,
) {
    let Some(live) = conversation.live() else {
        return;
    };
    let node = live.dialogue_id();
    let (base, interrupted) = match node.strip_suffix(INTERRUPTED_SUFFIX) {
        Some(base) => (base, true),
        None => (node, false),
    };
    let interruption = format!("{base}{INTERRUPTED_SUFFIX}");
    if !nodes.contains(&interruption) {
        return;
    }
    let members: Vec<_> = live
        .bodies()
        .filter_map(|member| footprints.get(member).ok().map(|aabb| (member, aabb.aabb(), rooms.of(member))))
        .collect();
    let mut arrivals: Vec<(Option<&SimId>, Entity)> = threats
        .iter()
        .filter(|(body, aabb, disposition, aggression, health, _)| {
            disposition.refuses_talk(*aggression)
                && health.is_none_or(|health| health.alive())
                && !members.iter().any(|(member, ..)| member == body)
                && members.iter().any(|(_, member, room)| {
                    *room == rooms.of(*body) && earshot_of(*member).strict_intersects(aabb.aabb())
                })
        })
        .map(|(body, _, _, _, _, id)| (id, body))
        .collect();
    if arrivals.is_empty() {
        return;
    }
    // In a stable order, so a resimulation breaks in with the same body:
    // a restore can renumber entities, not `SimId`s.
    arrivals.sort();
    let tick = tick.map_or(0, |tick| tick.0);
    for (at, (_, body)) in arrivals.into_iter().enumerate() {
        if at == 0 && !interrupted {
            conversation.interrupt(body, interruption.clone(), tick);
        } else {
            conversation.join(body);
        }
    }
}
