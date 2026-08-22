//! **The hold: a projection of the authority onto the body being talked to.**
//!
//! **two mechanisms, one effect, and that is not player-centrism.** The
//! initiator's intent is already neutral — `DIALOGUE_CONTEXT` captures their
//! input, so their `ControlFrame` is default. The other participant takes its
//! intent from a BRAIN, which nothing has captured, so it needs
//! `ScriptedControl`. The rule is symmetric ("every participant's intent is
//! neutral"); the two halves differ only because the two bodies are driven from
//! different places.

use bevy::prelude::*;

use ambition_characters::control::{claim_control_hold, release_control_hold, ControlHold, ControlHolds, ScriptedControl};

use super::authority::ActiveConversation;

/// This system's claim on a body it blanked for a conversation.
///
/// **a PROJECTION, not a record — and it must NOT become rollback state.** It
/// says nothing [`ActiveConversation`] does not already say. Registering it for
/// rollback would recreate the very shape this module exists to delete: two
/// records of one fact, rewound on two different schedules. Its only job is to
/// mark which bodies THIS system put `ScriptedControl` on, so the reconcile
/// below never strips a death beat's or a flagpole's.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HeldByConversation;

/// **Make the world match the conversation authority.**
///
/// **a total function of [`ActiveConversation`] → world, and that is the whole
/// design.** It does not remember what it inserted last tick and does not diff
/// against a previous value. Whatever a rollback restored, this rebuilds the
/// hold from the authority on the next tick, because the authority IS rewound
/// and this reads nothing else.
///
/// Asking whether the world already MATCHES the authority makes every half-state
/// self-repairing, in both directions, without knowing which half went missing or why.
///
/// **the removal is scoped by the marker AND by the claim.** `ScriptedControl`
/// has other claimants — the death beat, the flagpole, act clear, versus,
/// seating, and now a capture — and this system must never strip theirs. Once
/// that was an argument ("all of those mark a body a PLAYER drives, a
/// conversation marks the body talked TO, so the sets cannot overlap today");
/// now it is arithmetic, because [`ControlHold::Conversation`] is one bit of a
/// set and releasing it cannot clear another.
/// `a_conversation_hold_never_strips_another_claimants_control` still pins it,
/// and no longer depends on the two sets staying disjoint.
pub fn project_conversation_hold(
    mut commands: Commands,
    conversation: Res<ActiveConversation>,
    mut claimed: Query<(Entity, Option<&mut ControlHolds>), With<HeldByConversation>>,
    fully_held: Query<(), (With<HeldByConversation>, With<ScriptedControl>)>,
) {
    let holding = conversation.talker();
    for (entity, holds) in &mut claimed {
        if Some(entity) == holding {
            continue;
        }
        // All three mean the same thing here and need no case analysis: this body is not held
        // now.
        commands.entity(entity).try_remove::<HeldByConversation>();
        release_control_hold(
            &mut commands,
            entity,
            holds.map(|holds| holds.into_inner()),
            ControlHold::Conversation,
        );
    }
    let Some(talker) = holding else {
        return;
    };
    // **the skip is about change detection, not about correctness.**
    // `try_insert` is idempotent, so running it unconditionally would behave
    // identically — it would just touch the entity every tick and wake every
    // change-gated reader downstream. Because the question is "does the world
    // already match the authority" rather than "did I already do this", a
    // half-applied hold falls through this and is repaired.
    if fully_held.get(talker).is_err() {
        commands.entity(talker).try_insert(HeldByConversation);
        claim_control_hold(&mut commands, talker, ControlHold::Conversation);
    }
}
