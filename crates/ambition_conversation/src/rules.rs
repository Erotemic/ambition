//! Conversation continuity rules and cut notifications. Dialogue does not
//! suspend body simulation, so participant contact and damage can end a live
//! conversation.

use bevy::prelude::*;

use ambition_characters::actor::BodyCombat;
use ambition_dialog::DialogueBreak;
// Use the core re-export to avoid adding a direct geometry dependency.
use ambition_platformer2d_core::{AabbExt, CenteredAabb};

use super::authority::ActiveConversation;

/// Requests a bark from the body whose conversation was cut. This continuity
/// layer names the speaker; cast/content code chooses the line.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationCutBark {
    /// The body that should speak — the one carrying an `ActorInteraction`, so
    /// its character id and therefore its voice can be found.
    pub speaker: Entity,
}

pub fn break_dialogue_on_hit_or_separation(
    mut conversation: ResMut<ActiveConversation>,
    bodies: Query<(&CenteredAabb, Option<&BodyCombat>)>,
    mut barks: MessageWriter<ConversationCutBark>,
) {
    if !conversation.is_live() {
        return;
    }
    let participants: Vec<_> = conversation.participants().collect();
    let [a, b] = participants.as_slice() else {
        // Scripted dialogue with no two in-world bodies. Nothing here can walk
        // away from anything.
        return;
    };
    let (Ok((a_aabb, a_combat)), Ok((b_aabb, b_combat))) = (bodies.get(*a), bodies.get(*b)) else {
        // A participant stopped existing — despawned, or the room swapped under
        // the conversation. That is a separation of the most literal kind.
        conversation.close();
        return;
    };

    //  KNOCKBACK, not damage. The reason a hit ends a conversation is that it
    // MOVES you, so the signal is the recoil/hitstun control lock rather than
    // any health change: a poison tick or a chip of environmental damage leaves
    // both bodies standing where they were and leaves them talking.
    let struck = |combat: Option<&BodyCombat>| {
        combat.is_some_and(|c| c.recoil_lock_timer > 0.0 || c.hitstun_timer > 0.0)
    };
    let any_struck = struck(a_combat) || struck(b_combat);
    let in_reach = a_aabb.aabb().strict_intersects(b_aabb.aabb());

    let Some(reason) = DialogueBreak::evaluate(any_struck, in_reach) else {
        return;
    };

    //  only for the break that has no voice yet. A conversation broken by a
    // HIT already barks — `npc_hit_bark_line` fires on every strike and falls
    // back to a generic line when a character authored none — so adding a second
    // bubble for one event would be worse than none. `wants_its_own_bark` is
    // where that lives, beside the reason it is about.
    //
    // and this ASKS rather than answers. Which line, from which pool, with which fallback, is a
    // CAST question; see [`ConversationCutBark`].
    if reason.wants_its_own_bark() {
        barks.write(ConversationCutBark { speaker: *b });
    }
    conversation.close();
}
