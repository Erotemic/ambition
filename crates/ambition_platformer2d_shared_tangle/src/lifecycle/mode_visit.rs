//! Which room a mode owner is in, and whether it arrived there this tick.
//!
//! The component lives here, beside `spawn_mode_owner`, so every mode owner is
//! born with it. The runtime's `mode_scope::follow_mode_owner_rooms` keeps it.

use bevy::prelude::*;

/// How a mode owner came to be in its room this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Arrival {
    /// It was in this room on the tick before.
    #[default]
    Staying,
    /// Its first room: the owner came into being this tick.
    First,
    /// It came from another room this tick.
    FromAnotherRoom,
}

/// The room a mode owner is in. Every mode owner carries one (see
/// `spawn_mode_owner`), and the engine updates it before a game's rules run, so
/// a game asks [`Self::arrival`] and keeps no room memory of its own.
///
/// Rollback state: an arrival decides what a game starts over, and a rewind
/// across an arrival must decide it the same way again.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct ModeVisit {
    room: Option<String>,
    arrival: Arrival,
}

impl ModeVisit {
    /// The room the owner is in, once the engine has seen it in one.
    pub fn room(&self) -> Option<&str> {
        self.room.as_deref()
    }

    /// How the owner came to be in its room this tick.
    pub fn arrival(&self) -> Arrival {
        self.arrival
    }

    /// The visit on a tick whose active room is `active`.
    pub fn after(&self, active: &str) -> Self {
        let arrival = match self.room.as_deref() {
            Some(room) if room == active => Arrival::Staying,
            Some(_) => Arrival::FromAnotherRoom,
            None => Arrival::First,
        };
        Self {
            room: Some(active.to_owned()),
            arrival,
        }
    }

    /// A value-complete checksum of the visit, for its rollback probe.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.room.hash(&mut hasher);
        self.arrival.hash(&mut hasher);
        hasher.finish()
    }
}
