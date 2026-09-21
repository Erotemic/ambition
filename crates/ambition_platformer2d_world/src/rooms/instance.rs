//! WHICH LIVE ROOM THIS IS, which is not which room DEFINITION is selected.
//!
//! ⛔⛤ **THE WORKSPACE HAD NO ANSWER TO THIS QUESTION BEFORE 2026-09-20.**
//! `RoomSet` answers *"which of my definitions is live"* with an index, and
//! that index is the same number every time the session stands in that room.
//! `RoomConstructionPlanId` answers *"which prepared artifact is this"* and is
//! a CONTENT hash whose own doc excludes commit-time facts on purpose, so two
//! constructions of one room share it. Neither can say *"this live room is not
//! the one you were standing in a minute ago"*, and OW1 in
//! `docs/planning/engine/open-world-runtime-and-residency.md` cannot start
//! until something can.

use bevy_ecs::prelude::Component;

/// Which live room a session is standing in, as an ordinal of that session's
/// room publications.
///
/// A session starts at `0` — the room it was activated in — and every
/// publication that seats it in a room mints the next one, INCLUDING a
/// publication of the room it is already in. That is the whole point: leaving
/// `blink_run` for `portal_lab` and coming back gives three instances of two
/// definitions, and `RoomSet` alone reports the same index for the first and
/// the third.
///
/// ⚠ **THIS IS A ONE-LIVE-ROOM IDENTITY AND SAYS SO.** It lives on the session
/// root because that is where the one live room lives. Two simultaneous
/// instances — OW1's actual proof — would carry one of these each, on whatever
/// entity comes to own an instance; the ordinal does not have to move, only its
/// carrier. What it must never become is a second way to select a DEFINITION.
///
/// ⛔ It is rollback state (`root.live_room_instance`), because a rewind across
/// a room publication puts the session back in the previous live room, and an
/// identity that survived that rewind would name a room that no longer exists.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LiveRoomInstance(u32);

impl LiveRoomInstance {
    /// The activation room: a session that has published nothing is standing in
    /// its first live room, not in none.
    pub const ACTIVATION: Self = Self(0);

    /// How many rooms this session has been seated in before this one.
    pub fn ordinal(self) -> u32 {
        self.0
    }

    /// Rebuild from an ordinal — the snapshot decode side, and the only other
    /// way to obtain one. ⚠ NOT a way to CHOOSE an instance: the rollback codec
    /// is restoring an identity this session already minted, not naming a new
    /// one.
    pub fn from_ordinal(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// Mint the next instance, because a room was just published into this
    /// session.
    ///
    /// ⚠ Saturating rather than wrapping: at `u32::MAX` publications the
    /// honest failure is *"two live rooms now share an identity"* and the
    /// honest one is *"the counter stopped"*. A wrap would silently reuse the
    /// identity of a room 4 billion publications ago; a stop is visible in the
    /// census as an ordinal that will not move. Neither is reachable at 60Hz
    /// (a publication per tick for two years), which is why this is a comment
    /// and not a refusal.
    pub fn advance(&mut self) {
        self.0 = self.0.saturating_add(1);
    }
}

impl std::fmt::Display for LiveRoomInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ THE PROPERTY OW1 NEEDS: RE-ENTERING A ROOM IS NOT RE-ENTERING THE SAME
    /// LIVE ROOM.
    ///
    /// The unit half. The composed half — that a real crossing back into the
    /// room you came from mints a third instance while `RoomSet` reports the
    /// first room's index again — lives with the crossing.
    #[test]
    fn every_publication_is_a_different_live_room() {
        let mut instance = LiveRoomInstance::ACTIVATION;
        assert_eq!(instance.ordinal(), 0);

        let first = instance;
        instance.advance();
        let second = instance;
        instance.advance();
        let third = instance;

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(
            first, third,
            "coming back to a room you have already been in must not reuse the \
             identity of the live room you left"
        );
        assert_eq!(third.ordinal(), 2);
    }
}
