//! **Which physical input SOURCE a participant owns.**
//!
//! [`crate::local_seats`] already answers this for gamepads: seat `n` owns the
//! `n`-th connected pad, in remembered arrival order. It cannot answer it for
//! the keyboard, because the keyboard is not a row in that model at all —
//! keyboard bindings live in every seat's `InputMap`, put there by a
//! [`crate::presets`] preset, and nothing owns them.
//!
//! With two participants it is the couch bug — **both seats answer the keyboard**, so player one's
//! WASD also moves player two if player two's preset overlaps, and a keyboard player and a pad
//! player cannot coexist because the keyboard has no owner to be.
//!
//! ## The two questions, kept apart
//!
//! * WHAT sources exist, and in what order people took them ([`InputSourceId`],
//!   ordered by [`crate::local_seats::LocalDeviceOrder`] for pads);
//! * WHETHER a source is owned by one participant or shared by the primary
//!   ([`InputAssignmentPolicy`]).
//!
//! They are separate because the answer to the second is a SESSION decision that
//! must not change while a match is running, while the first keeps moving as
//! people plug controllers in. Folding them together is what makes "a device
//! connected" and "a player joined" the same event, which they are not: a pad
//! plugged in during a match is a spare, not a third fighter.

use bevy::prelude::*;

use crate::participant::ParticipantId;

/// One thing a person can hold.
///
/// The keyboard-and-mouse bundle is ONE source, not two: nobody plays with the
/// mouse while somebody else has the keys, and modelling them separately would
/// invent an ownership question no player ever asks.
///
/// a gamepad is identified by its `Entity`, which is only meaningful while it
/// stays connected. That is deliberate and is why [`InputAssignmentPolicy`]
/// exists separately: a frozen assignment is what survives a disconnect, not the
/// entity id. Bevy recycles entity indices, so an id that outlived its device
/// would eventually name a different one — see `local_seats`' arrival-order note
/// for the same trap caught in the gamepad ordering.
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

/// How local sources become participants.
///
/// Default is [`Self::UnifiedPrimary`], which is today's behaviour exactly: this
/// type must be installable without changing what any existing game does, or the
/// couch work has to be finished before anything can ship.
/// **not a `Resource` — it is carried by [`crate::LocalSeatOffer`].** A
/// policy with no owner could only be given back by value equality, which is
/// how one surface's teardown erased a successor's identical claim.
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

/// **Who owns the keyboard**, when ownership is a question at all.
///
/// `None` means nobody owns it exclusively — under [`InputAssignmentPolicy::UnifiedPrimary`]
/// that is the correct and only answer.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardOwner(pub Option<ParticipantId>);

/// Which participant, if any, should have EXCLUSIVE keyboard bindings.
///
/// Pure so the decision can be tested without an `App`, and so the two callers
/// that need it — the binding pass and any UI that wants to say "keyboard:
/// player one" — cannot drift apart by re-deriving it.
///
/// `seats < 2` returns `None` under EVERY policy, including `JoinToClaim`.
/// A lone participant owning the keyboard exclusively is the same state as
/// nobody owning it, and returning `Some` there would make the binding pass
/// rewrite an `InputMap` that did not need to change — which marks it changed
/// for the settings UI that rebuilds on exactly that signal.
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
