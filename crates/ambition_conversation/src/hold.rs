//! Project conversation authority onto participant control.
//! Dialogue input neutralizes the initiator; the other participant receives a
//! conversation-owned `ScriptedControl` hold so both sides remain neutral.

use bevy::prelude::*;

use ambition_characters::control::{claim_control_hold, release_control_hold, ControlHold, ControlHolds, ScriptedControl};

use super::authority::ActiveConversation;

/// Projection marker for the conversation-owned control hold.
/// Derived from [`ActiveConversation`]; it is not rollback authority.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct HeldByConversation;

/// Reconcile conversation-owned control holds from [`ActiveConversation`].
///
/// The projection is idempotent and rollback-reconstructible. Removal releases
/// only [`ControlHold::Conversation`], leaving other `ScriptedControl` claims intact.
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
    // the skip is about change detection, not about correctness.
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
