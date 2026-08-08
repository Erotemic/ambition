//! **When a conversation ends, and the bark that says so.**
//!
//! The consumer of Jon's continuity design
//! (`docs/planning/engine/dialogue-continuity.md`). Since `GameMode::Dialogue`
//! left the suspend set, a conversation is a SUSTAINED condition rather than a
//! modal state — bodies keep moving, hits keep landing, and a text box that
//! survives either is a text box floating over two people who are no longer
//! talking.

use bevy::prelude::*;

use ambition_characters::actor::BodyCombat;
use ambition_dialog::DialogueBreak;
// ⚠ `CenteredAabb` through core's re-export, NOT `ambition_geometry` where it is
// defined: the monolith does not depend on that crate directly, and reaching for
// the definition site would add a dependency edge — which the contracts job
// fails until `fixtures/minimal_game/Cargo.lock` is regenerated, for a type that
// is already in reach.
use ambition_platformer2d_core::{AabbExt, CenteredAabb};

use super::authority::ActiveConversation;

/// **A conversation was cut, and this body should say so.**
///
/// ⛔ **the PORT, and it is why this module can leave the crate.** Asking "what
/// line does this character say" is a CAST question — it needs the character
/// catalog, the prepared registry, and the character-id lookup that resolves an
/// `Interactable` to a voice, none of which is about continuity. Doing it here
/// meant continuity reached back into `crate::features` and
/// `crate::character_runtime` for the only two inward edges this module has.
///
/// ⭐ so continuity says WHO should speak and the cast answers WHAT they say.
/// The speaking body is named; the responder owns the pool, the fallback, the
/// rotation and the bubble.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversationCutBark {
    /// The body that should speak — the one carrying an `ActorInteraction`, so
    /// its character id and therefore its voice can be found.
    pub speaker: Entity,
}

/// **Break a conversation the world has carried its participants out of.**
///
/// ⭐ **symmetric, and that is load-bearing.** It reads
/// [`ActiveConversation::participants`], which yields BOTH bodies, and folds
/// them into one `any_struck` before asking. There is deliberately no place in
/// this system for "was the player hit" — an NPC knocked off a ledge mid-sentence
/// has ended the conversation just as surely as the player being knocked across
/// the room. Jon: *"both characters should hover"*.
///
/// ⚠ **the reach test is the interaction's own**, not a second authored range:
/// the same `strict_intersects` of the two bodies' AABBs that decided the
/// conversation could START decides it can continue. Two ranges would drift, and
/// the symptom — a conversation you can begin but not sustain, or one that
/// follows you across a room — is the kind nobody reports as a range bug.
///
/// A conversation with fewer than two in-world participants (scripted dialogue,
/// a system-started box) cannot be walked away from, and is left alone.
///
/// ⚠ **this no longer takes the hold.** It did until 2026-08-07, and holding
/// inside the rule that decides whether to STOP holding is what let a rewind
/// strand the two halves apart. The hold is now
/// [`super::hold::project_conversation_hold`], which reads the same authority
/// this one writes.
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

    // ⚠ KNOCKBACK, not damage. The reason a hit ends a conversation is that it
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

    // **THE BARK.** Jon: *"A broken dialog can have some bark to indicate that
    // it was broken."*
    //
    // ⛔ only for the break that has no voice yet. A conversation broken by a
    // HIT already barks — `npc_hit_bark_line` fires on every strike and falls
    // back to a generic line when a character authored none — so adding a second
    // bubble for one event would be worse than none. `wants_its_own_bark` is
    // where that lives, beside the reason it is about.
    //
    // ⭐ **and this ASKS rather than answers.** Which line, from which pool,
    // with which fallback, is a CAST question; see [`ConversationCutBark`]. An
    // empty pool is still SILENCE, and that is still the finished behaviour —
    // the responder decides it, and no character has a `conversation_cut` line
    // yet because those are Jon's voice to write, not the engine's to invent.
    if reason.wants_its_own_bark() {
        barks.write(ConversationCutBark { speaker: *b });
    }
    conversation.close();
}
