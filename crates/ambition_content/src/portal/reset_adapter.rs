//! Ambition room-reset bridge: `ResetRoomFeaturesEvent` → portal `ClearPortals`.
//!
//! Portal core's [`clear_portals_on_reset`](ambition_gameplay_core::portal::clear_portals_on_reset)
//! consumes the portal-owned [`ClearPortals`](ambition_gameplay_core::portal::ClearPortals) signal,
//! not the Ambition `ResetRoomFeaturesEvent`. This bridge translates the Ambition
//! room-reset event into the portal signal so portal core never names the reset
//! event — the room-reset *policy* (when a room resets) stays Ambition's, while
//! portal owns the *clear-portals* mechanic.

use bevy::prelude::*;

use ambition_gameplay_core::features::{ResetRoomFeaturesEvent, RoomResetReason};
use ambition_gameplay_core::portal::ClearPortals;

/// Emit a [`ClearPortals`] for a MANUAL room reset (the delete-key reset or a
/// scripted replay), but NOT for a player DEATH — so dying preserves the player's
/// gun-portal setup (and authored level portals are spared by
/// `clear_portals_on_reset` either way). Runs in `PortalSet::RoomReset` before
/// `clear_portals_on_reset`, so the clear happens the same frame the room reset
/// fires.
pub fn bridge_room_reset_to_clear_portals(
    mut resets: MessageReader<ResetRoomFeaturesEvent>,
    mut clear: MessageWriter<ClearPortals>,
) {
    // Clear the gun portals only if at least one reset this frame was deliberate.
    // A death (`PlayerDeath`) leaves every portal in place.
    let manual = resets.read().any(|r| r.reason == RoomResetReason::Manual);
    if manual {
        clear.write(ClearPortals);
    }
}

#[cfg(test)]
mod tests {
    //! The bridge routes by reset reason: a MANUAL reset emits `ClearPortals`
    //! (clearing the gun pair downstream), a PLAYER-DEATH reset emits nothing.
    use super::*;

    fn clears_for(reason: RoomResetReason) -> bool {
        let mut app = App::new();
        app.add_message::<ResetRoomFeaturesEvent>();
        app.add_message::<ClearPortals>();
        app.add_systems(Update, bridge_room_reset_to_clear_portals);
        app.world_mut()
            .write_message(ResetRoomFeaturesEvent { reason });
        app.update();
        let mut reader = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ClearPortals>>();
        reader.drain().count() > 0
    }

    #[test]
    fn manual_reset_clears_but_death_does_not() {
        assert!(
            clears_for(RoomResetReason::Manual),
            "a manual reset must emit ClearPortals (clears the gun pair)"
        );
        assert!(
            !clears_for(RoomResetReason::PlayerDeath),
            "a death reset must NOT emit ClearPortals (gun portals survive a death)"
        );
    }
}
