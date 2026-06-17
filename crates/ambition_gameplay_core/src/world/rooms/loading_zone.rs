//! Loading zones — activation rules + readiness.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

/// How a loading zone should be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingZoneActivation {
    /// Walk-off-the-edge transition. Validator requires the zone to
    /// touch a level edge so the player physically walks off the
    /// screen into it. Arrival on the target side is 92px inset
    /// from the matching edge.
    EdgeExit,
    /// Interact-to-enter door. Doesn't require an edge; the player
    /// presses Interact while overlapping the zone to fire the
    /// transition. Arrival on the target side is centered on the
    /// target zone, bottom-26px.
    Door,
    /// Walk-into-the-zone trigger. Like `EdgeExit` (overlap = fire)
    /// but NOT required to touch a level edge — used for portals
    /// and other mid-room walk-through transitions where the
    /// player just steps inside the rectangle and the transition
    /// fires. Arrival uses the same centered-bottom rule as `Door`.
    Walk,
}

impl LoadingZoneActivation {
    pub fn label(self) -> &'static str {
        match self {
            Self::EdgeExit => "edge exit",
            Self::Door => "door",
            Self::Walk => "walk",
        }
    }
}

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub id: String,
    pub name: String,
    pub activation: LoadingZoneActivation,
    pub aabb: ae::Aabb,
}

impl LoadingZone {
    pub fn is_ready(&self, wants_interact: bool) -> bool {
        match self.activation {
            LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => true,
            LoadingZoneActivation::Door => wants_interact,
        }
    }

    pub fn hint(&self, _flying: bool) -> String {
        match self.activation {
            LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => {
                format!("{}: {}", self.activation.label(), self.name)
            }
            LoadingZoneActivation::Door => {
                format!(
                    "{}: {} (Interact / double-tap up)",
                    self.activation.label(),
                    self.name
                )
            }
        }
    }
}
