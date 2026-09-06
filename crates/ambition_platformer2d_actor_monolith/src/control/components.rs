//! The control-seam STATE: which controlled bodies are local, plus the gesture
//! state owned by each controller slot.
//!
//! `LocalPlayer` says *this body is driven by an input source on this machine*.
//! The actual per-tick input authority is [`ambition_characters::control::SlotControls`],
//! keyed by the body's [`PlayerSlot`]; it is deliberately not copied onto the
//! body. `SlotGestures` / `SlotInteractionState` are likewise SLOT-level: a
//! gesture belongs to a controller and follows it onto whatever body it drives.

use bevy::prelude::*;


/// Marks a player whose input comes from this machine's input devices
/// (keyboard / gamepad / touch). In single-player today the local
/// player is also the primary player. In a future networked build,
/// remote players would have `PlayerEntity` (+ `PlayerSlot`) but not
/// `LocalPlayer`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalPlayer;

// Player health is now the unified `ambition_characters::actor::BodyHealth` (the keystone
// collapse of the identical `PlayerHealth` / `ActorHealth` wrappers into one
// body-health component).

#[cfg(test)]
#[path = "multiplayer_smoke_tests.rs"]
mod multiplayer_smoke_tests;
