//! Ambition room-reset bridge: `ResetRoomFeaturesEvent` → portal `ClearPortals`.
//!
//! Portal core's [`clear_portals_on_reset`](crate::portal::clear_portals_on_reset)
//! consumes the portal-owned [`ClearPortals`](crate::portal::ClearPortals) signal,
//! not the Ambition `ResetRoomFeaturesEvent`. This bridge translates the Ambition
//! room-reset event into the portal signal so portal core never names the reset
//! event — the room-reset *policy* (when a room resets) stays Ambition's, while
//! portal owns the *clear-portals* mechanic.

use bevy::prelude::*;

use crate::features::ResetRoomFeaturesEvent;
use crate::portal::ClearPortals;

/// Emit a [`ClearPortals`] for each [`ResetRoomFeaturesEvent`] this frame. Runs
/// in `PortalSet::RoomReset` before `clear_portals_on_reset`, so the portal clear
/// happens the same frame the room reset fires — identical to the old direct
/// read.
pub fn bridge_room_reset_to_clear_portals(
    mut resets: MessageReader<ResetRoomFeaturesEvent>,
    mut clear: MessageWriter<ClearPortals>,
) {
    if resets.read().next().is_none() {
        return;
    }
    clear.write(ClearPortals);
}
