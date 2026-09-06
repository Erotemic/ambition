//! Physical input-source identity and participant assignment policy.
//!
//! Device discovery may change as controllers connect, while source ownership is a session-level
//! decision. Keep those facts separate so connecting a device does not implicitly join a player.

use bevy::prelude::*;

use crate::participant::ParticipantId;

/// A physical local input source.
///
/// Keyboard and mouse are one ownership unit. A gamepad entity is valid only while that device is
/// connected; session assignment must not treat the entity ID as durable identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputSourceId {
    /// The keyboard-and-mouse bundle. Exactly one exists.
    Keyboard,
    /// A connected gamepad, while it is connected.
    Gamepad(Entity),
}

impl InputSourceId {
    pub const fn is_keyboard(self) -> bool {
        matches!(self, Self::Keyboard)
    }
}

/// How local input sources become participants.
///
/// The policy is carried by [`crate::LocalSeatOffer`] rather than stored as a global resource so
/// teardown removes only the owning surface's offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputAssignmentPolicy {
    /// Keyboard, gamepads and everything else drive the PRIMARY participant.
    ///
    /// Solo play.
    #[default]
    UnifiedPrimary,
    /// An unassigned source claims a participant by joining.
    ///
    /// The normal couch flow: press start on a pad and you are player two; the
    /// keyboard stays with whoever already had it.
    JoinToClaim,
    /// The host supplies the mapping outright.
    ///
    /// For replays, tests, and remote sessions that already know who is who.
    ExplicitAssignment,
}

/// Who owns the keyboard, when ownership is a question at all.
///
/// `None` means nobody owns it exclusively — under [`InputAssignmentPolicy::UnifiedPrimary`]
/// that is the correct and only answer.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardOwner(pub Option<ParticipantId>);

/// Which participant, if any, should have exclusive keyboard bindings.
///
/// With fewer than two seats, exclusivity is unnecessary and this returns `None` for every policy.
///
/// ⛔ THIS IS THE FALLBACK, NOT THE AUTHORITY. Once a match freezes a
/// `LocalChannelPlan` that plan owns the question — it knows who actually
/// claimed the keyboard, and it may say NOBODY. This generic policy answers only
/// where nothing has been declared (launcher, menus, a lobby still filling), and
/// `JoinToClaim`'s "leave it with player one" is the right answer THERE.
pub fn keyboard_owner_for(
    policy: InputAssignmentPolicy,
    owner: KeyboardOwner,
    seats: usize,
) -> Option<ParticipantId> {
    if seats < 2 {
        return None;
    }
    match policy {
        InputAssignmentPolicy::UnifiedPrimary => None,
        // Nobody has claimed it yet, so it stays with the seat that has been
        // playing — player one does not lose their keyboard because somebody
        // else picked up a pad.
        InputAssignmentPolicy::JoinToClaim => Some(owner.0.unwrap_or(ParticipantId::PRIMARY)),
        InputAssignmentPolicy::ExplicitAssignment => owner.0,
    }
}

#[cfg(test)]
mod tests;
