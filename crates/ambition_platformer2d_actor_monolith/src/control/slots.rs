//! The local-device → primary-slot bridge.
//!
//! [`populate_slot_controls`] publishes the finalized global `ControlFrame` into
//! the canonical [`SlotControls`] resource as slot 0. That is the end of the
//! local-device adapter: bodies never receive a copied input component. A
//! controlled body names its slot with `DrivingParticipant(slot)`, and the control tick
//! reads that slot directly. Higher local/network seats publish their own slots
//! through their respective adapters.
//!
//! This module is allowlisted as an input-layer bridge by the workspace
//! `ControlFrame` policy. Nothing downstream should hold the global frame.

use bevy::prelude::*;

use ambition_characters::brain::{PlayerSlot, SlotControls};
use ambition_input::ControlFrame;

/// The publication boundary for the local primary controller.
///
/// Any system that rewrites the global [`ControlFrame`] for this tick (gesture
/// derivation, portal input shaping, touch folding) must run **before** this set.
/// Systems that need the canonical slot value may order **after** it. Keeping the
/// boundary as a semantic set avoids coupling other crates to the leaf function
/// that performs the copy.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimarySlotInputCommit;

/// Publish the local device's finalized [`ControlFrame`] into the slot-based
/// controller model as [`PlayerSlot::PRIMARY`]. This is the ONE place local
/// primary input enters [`SlotControls`]. Every controlled body then reads its
/// own slot through `DrivingParticipant(slot)`; no entity-local input mirror exists.
pub fn populate_slot_controls(frame: Res<ControlFrame>, mut slots: ResMut<SlotControls>) {
    slots.set(PlayerSlot::PRIMARY, *frame);
}
