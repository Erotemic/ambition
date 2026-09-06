//! Data-driven sandbox room-set and loading-zone graph.
//!
//! Rooms are runtime graph nodes built from LDtk-authored runtime data. This module owns
//! transition graph assembly and arrival validation, while LDtk owns sandbox world authoring.

#![allow(unused_imports)]
use ambition_platformer2d_core as ae;

pub mod binding;
// `RoomTransitionApplication::apply` is the live commit and says the same things in the same
// order. It surfaced while unifying room RESIDENCY: it carried its own `With<RoomScopedEntity>`
// roster parameter, so it was a second place the "a carried object is not a resident" rule
// would have had to be applied, and a dead one. A fork you cannot reach is still a fork that
// lies about how many commits this engine has.
mod reconstitution;
mod stage;
mod systems;
pub(crate) mod transaction;

#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod tests;

// ⛔ NO GLOB FORWARD OF `ambition_platformer2d_world::rooms`. Callers name that
// crate: a forward here would let this crate's own coupling census count the
// world crate as monolith coupling, which is what it did until 2026-08-26.
// What this module re-exports below is what it OWNS.
pub use binding::RoomBindings;
pub use reconstitution::retire_the_previous_attempt;
pub use stage::{
    LastRoomConstructionCommit, RoomConstructionError, RoomConstructionPlan, RoomConstructionPlanId,
};
pub use systems::{
    detect_room_transition_system, sync_active_room_metadata, sync_room_music_request,
    tick_portal_phases_system, ActiveRoomMetadataSynced,
};
pub use transaction::{ActiveContentBinding, LastConstructionVerification};

#[cfg(test)]
mod rooms_unit_tests {
    use super::*;
    use ambition_platformer2d_world::rooms::*;

    fn zone(activation: LoadingZoneActivation) -> LoadingZone {
        LoadingZone {
            id: "z".into(),
            name: "Zone".into(),
            activation,
            aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(16.0, 16.0)),
        }
    }

    #[test]
    fn loading_zone_readiness_follows_the_interact_binding_rule() {
        // EdgeExit / Walk fire on contact; a Door requires Interact, so
        // a plain step (or single-press Up) onto a door does NOT trigger.
        assert!(zone(LoadingZoneActivation::EdgeExit).is_ready(false));
        assert!(zone(LoadingZoneActivation::Walk).is_ready(false));
        assert!(!zone(LoadingZoneActivation::Door).is_ready(false));
        assert!(zone(LoadingZoneActivation::Door).is_ready(true));
    }

    #[test]
    fn portal_phase_opens_holds_and_closes_with_the_switch() {
        let mut p = GatePortalPhase::Off;
        assert!(!p.allows_traversal());

        // Switch on → starts opening.
        tick_gate_portal_phase(&mut p, true, 0.016);
        assert!(matches!(p, GatePortalPhase::Opening { .. }));
        assert!(!p.allows_traversal(), "not traversable while opening");

        // Hold the switch long enough to finish opening → On.
        tick_gate_portal_phase(&mut p, true, 100.0);
        assert_eq!(p, GatePortalPhase::On);
        assert!(p.allows_traversal());
        assert!(p.portal_sprite_visible());

        // Switch off → closes.
        tick_gate_portal_phase(&mut p, false, 0.016);
        assert!(matches!(p, GatePortalPhase::Closing { .. }));
        assert!(!p.allows_traversal());
    }

    #[test]
    fn portal_interrupted_mid_open_reverses_to_closing() {
        let mut p = GatePortalPhase::Off;
        tick_gate_portal_phase(&mut p, true, 0.05); // partway open
        assert!(matches!(p, GatePortalPhase::Opening { .. }));
        tick_gate_portal_phase(&mut p, false, 0.016); // switch released mid-open
        assert!(matches!(p, GatePortalPhase::Closing { .. }));
    }
}
