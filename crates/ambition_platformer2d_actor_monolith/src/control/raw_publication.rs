//! Publish raw seat frames when no tick or rollback authority owns publication.

use bevy::prelude::*;

/// Copy raw frames only in a composition without another input authority.
pub fn publish_seat_controls_when_nobody_else_does(
    latches: Option<Res<ambition_characters::control::SlotControlLatches>>,
    rollback: Option<Res<ambition_platformer2d_shared_tangle::schedule::SimulationReplayState>>,
    raw: Res<ambition_characters::control::SeatRawFrames>,
    mut slots: ResMut<ambition_characters::control::SlotControls>,
) {
    if crate::control::another_authority_publishes(latches.as_deref(), rollback.as_deref()) {
        return;
    }
    for (slot, frame) in raw.seats() {
        slots.set(slot, frame);
    }
}
