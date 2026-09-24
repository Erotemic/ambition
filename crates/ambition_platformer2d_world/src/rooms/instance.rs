//! Which live room this is, which is not which room definition is selected.
//!
//! `RoomSet` answers "which definition is live" with an index that is the
//! same every time the session stands in that room. `RoomConstructionPlanId`
//! is a content hash that excludes commit-time facts, so two constructions of
//! one room share it. Neither can tell two visits apart. OW1 in
//! `docs/planning/engine/open-world-runtime-and-residency.md` needs that.

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
/// This identity assumes one live room, so it lives on the session root. With
/// simultaneous instances (OW1), each instance's owner carries one; the
/// ordinal stays the same, only its carrier moves. It must never become a way
/// to select a definition.
///
/// It is rollback state (`root.live_room_instance`). A rewind across a room
/// publication returns to the previous live room, so the identity must rewind
/// too.
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

    /// Rebuild from an ordinal, for snapshot decode only. It restores an
    /// identity this session already minted; it does not choose a new one.
    pub fn from_ordinal(ordinal: u32) -> Self {
        Self(ordinal)
    }

    /// Mint the next instance, because a room was just published into this
    /// session.
    ///
    /// Saturates instead of wrapping. A wrap would silently reuse an old
    /// room's identity; a stuck ordinal is visible in the census. Neither is
    /// reachable at 60Hz (a publication per tick for two years), so there is
    /// no refusal.
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

    /// Re-entering a room is not re-entering the same live room.
    ///
    /// This is the unit half. The composed half (a crossing back mints a
    /// third instance while `RoomSet` reports the first index again) is
    /// tested with the crossing.
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
