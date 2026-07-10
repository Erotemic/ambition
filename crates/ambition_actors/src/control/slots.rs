//! The two bridges that turn ONE human's device into ONE slot's body input.
//!
//! `populate_slot_controls` is the device→slot bridge: the finalized global
//! `ControlFrame` enters the canonical `SlotControls` resource as slot 0.
//! `sync_local_player_input_frame` is the slot→body bridge: each local player
//! body receives its OWN slot's frame, gated on brain ownership — so a home
//! avatar whose player brain has been transferred to a possessed actor sees
//! neutral input and has no local authority, without a possession run-condition.
//!
//! Both hold `Res<ControlFrame>`/`Res<SlotControls>` and are allowlisted as
//! input-layer bridges in `ambition_runtime/tests/control_frame_lint.rs`. Nothing
//! downstream of them may hold the global frame.

use bevy::prelude::*;

use ambition_characters::brain::{Brain, PlayerSlot, SlotControls};
use ambition_input::ControlFrame;

use super::components::{LocalPlayer, PlayerInputFrame};

/// Publish the local device's finalized [`ControlFrame`] into the slot-based
/// controller model as [`PlayerSlot::PRIMARY`]. This is the ONE place local
/// input enters the canonical [`SlotControls`] resource; every controlled body
/// reads its slot's frame from there via `Brain::Player`. Co-op / netcode add
/// their own writers for higher slots without touching this one.
pub fn populate_slot_controls(frame: Res<ControlFrame>, mut slots: ResMut<SlotControls>) {
    slots.set(PlayerSlot::PRIMARY, *frame);
}

/// Mirror a controlled body's slot frame onto its [`PlayerInputFrame`] component.
///
/// `PlayerInputFrame` is the per-body view of "the input THIS body is receiving
/// as a controlled body" — read by player-specific ability systems (held-item
/// use, heal shrine, portal gun). It is sourced from [`SlotControls`] and gated
/// on brain ownership: a body only receives its slot's frame while it carries
/// `Brain::Player(slot)`. A vacated home avatar (its player brain transferred to
/// a possessed actor) therefore sees NEUTRAL input and has no local attack
/// authority — the mandate's "the vacated body must not act", derived from the
/// brain rather than a possession run-condition.
pub fn sync_local_player_input_frame(
    slots: Res<SlotControls>,
    mut players: Query<(&mut PlayerInputFrame, Option<&Brain>), With<LocalPlayer>>,
) {
    for (mut player_input, brain) in &mut players {
        player_input.frame = match brain.and_then(Brain::player_slot) {
            Some(slot) => slots.get(slot),
            // No player brain (vacated during possession): no local control.
            None => ControlFrame::default(),
        };
    }
}
