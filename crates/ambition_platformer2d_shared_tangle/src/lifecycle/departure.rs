//! A level's request to leave its room for the next one.
//!
//! The component and its vocabulary live here, beside `spawn_mode_owner`, so
//! every mode owner is born able to leave. The runtime's
//! `room_departure::drive_departures` carries a departure out.

use bevy::prelude::*;

/// Where a departure goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    /// The active room's authored `next_room`, or this room again when it
    /// names none.
    NextRoom,
    /// An authored room, by id.
    Room(String),
    /// This room, from its start.
    Replay,
}

/// How long a departure keeps asking before it gives up to a replay.
///
/// The lifecycle commit waits for the frame to be confirmed and the room
/// transaction to load, so a departure spans frames. Three seconds is long
/// past any honest wait and short enough that a broken target is noticed.
pub const DEPARTURE_GIVE_UP_S: f32 = 3.0;

/// A level's request to leave its room. Every mode owner carries one (see
/// `spawn_mode_owner`); the runtime's `drive_departures` carries it out. It is
/// rollback state: whether a level has asked to leave, and for how long,
/// decides what the simulation writes.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Departure {
    pub state: DepartureState,
}

/// Where a [`Departure`] is in its trip. The engine's driver advances it; a
/// game asks through [`Departure::leave`] and may read it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum DepartureState {
    #[default]
    Staying,
    /// Asked; resolved by the engine's driver on its next pass.
    Requested(Destination),
    /// On the way to `target`, asking for `asked` seconds.
    Leaving { target: String, asked: f32 },
    /// Replaying this room, asking for `asked` seconds. A replay request can be
    /// REFUSED (the lifecycle slot is earliest-sticky and another operation may
    /// hold it), so the intent stays here, re-asked every tick, until a replay
    /// is admitted.
    Replaying { asked: f32 },
}

impl Departure {
    /// Leave for `to`. Asking again while already leaving changes nothing: the
    /// first destination stands until the trip ends.
    pub fn leave(&mut self, to: Destination) {
        if matches!(self.state, DepartureState::Staying) {
            self.state = DepartureState::Requested(to);
        }
    }

    /// Whether a departure has been asked for and has not yet ended.
    pub fn is_leaving(&self) -> bool {
        !matches!(self.state, DepartureState::Staying)
    }

    /// The room this departure is travelling to, once resolved.
    pub fn target(&self) -> Option<&str> {
        match &self.state {
            DepartureState::Leaving { target, .. } => Some(target),
            _ => None,
        }
    }

    /// A value-complete checksum of the departure, for its rollback probe.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        match &self.state {
            DepartureState::Staying => 0u8.hash(&mut hasher),
            DepartureState::Requested(to) => {
                1u8.hash(&mut hasher);
                format!("{to:?}").hash(&mut hasher);
            }
            DepartureState::Leaving { target, asked } => {
                2u8.hash(&mut hasher);
                target.hash(&mut hasher);
                asked.to_bits().hash(&mut hasher);
            }
            DepartureState::Replaying { asked } => {
                3u8.hash(&mut hasher);
                asked.to_bits().hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_ask_does_not_redirect_a_departure_already_asked_for() {
        let mut departure = Departure::default();
        assert!(!departure.is_leaving());
        departure.leave(Destination::NextRoom);
        departure.leave(Destination::Room("elsewhere".into()));
        assert_eq!(
            departure.state,
            DepartureState::Requested(Destination::NextRoom),
            "the first destination stands until the trip ends"
        );
        assert!(departure.is_leaving());
    }

    #[test]
    fn the_checksum_sees_how_long_a_departure_has_asked() {
        let at = |asked: f32| Departure {
            state: DepartureState::Leaving {
                target: "next".into(),
                asked,
            },
        };
        assert_ne!(at(0.5).checksum(), at(0.6).checksum());
        assert_ne!(Departure::default().checksum(), at(0.0).checksum());
    }
}
