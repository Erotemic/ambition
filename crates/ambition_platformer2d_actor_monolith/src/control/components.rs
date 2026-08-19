//! The control-seam STATE: which controlled bodies are local, plus the gesture
//! state owned by each controller slot.
//!
//! `LocalPlayer` says *this body is driven by an input source on this machine*.
//! The actual per-tick input authority is [`ambition_characters::brain::SlotControls`],
//! keyed by the body's [`PlayerSlot`]; it is deliberately not copied onto the
//! body. `SlotGestures` / `SlotInteractionState` are likewise SLOT-level: a
//! gesture belongs to a controller and follows it onto whatever body it drives.

use bevy::prelude::*;

// The slot marker every body-facing consumer keys on. Defined a tier down, in
// `ambition_characters::brain`, because a brain names its own slot.
pub use ambition_characters::brain::PlayerSlot;
pub use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};

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

/// One controller slot's gesture/buffer state: double-tap timers, the interact
/// buffer, and the pending double-tap edges. This is SLOT-level state (it belongs
/// to a controller, not to any one body) — the local input systems publish it from
/// the device each frame, and gameplay systems (body-mode, interaction) consume it
/// for whatever body that slot currently controls. Held in [`SlotInteractionState`],
/// keyed by [`PlayerSlot`]; deliberately NOT a `Component`, so no body privately
/// owns "the interaction state".
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SlotGestures {
    /// Counts down after a double-tap-down edge; non-zero means morph-ball
    /// entry is pending for the body-mode driver.
    pub down_tap_timer: f32,
    /// Counts down after a double-tap-up edge; drives door/NPC triggers.
    pub up_tap_timer: f32,
    /// Counts down after `interact_pressed`; keeps the interact signal alive
    /// across frames so the player doesn't need to hold the button until the
    /// door animation completes.
    pub interact_buffer_timer: f32,
    /// Set true by `input_timer_system` when a double-tap-down is detected;
    /// consumed by the body-mode driver after the player tick.
    pub double_tap_down_pending: bool,
    /// Set true by `input_timer_system` when a double-tap-up gesture is
    /// detected; consumed (via `mem::take`) by `interaction_input_system`
    /// the same frame to fold it into the hit-stun-gated interact buffer
    /// that drives door / NPC / chest activation.
    pub double_tap_up_pending: bool,
}

impl SlotGestures {
    /// Advance timers and detect a double-tap-down edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_down_tap(&mut self, down_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.down_tap_timer = (self.down_tap_timer - frame_dt).max(0.0);
        if !down_pressed {
            return false;
        }
        if self.down_tap_timer > 0.0 {
            self.down_tap_timer = 0.0;
            true
        } else {
            self.down_tap_timer = window;
            false
        }
    }

    /// Advance timers and detect a double-tap-up edge. Returns `true` when
    /// two taps arrive within `window` seconds.
    pub fn register_up_tap(&mut self, up_pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.up_tap_timer = (self.up_tap_timer - frame_dt).max(0.0);
        if !up_pressed {
            return false;
        }
        if self.up_tap_timer > 0.0 {
            self.up_tap_timer = 0.0;
            true
        } else {
            self.up_tap_timer = window;
            false
        }
    }

    /// Update the interact buffer and return whether the buffer is live.
    pub fn buffered_interact(&mut self, pressed: bool, frame_dt: f32, window: f32) -> bool {
        self.interact_buffer_timer = (self.interact_buffer_timer - frame_dt).max(0.0);
        if pressed {
            self.interact_buffer_timer = window;
        }
        self.interact_buffer_timer > 0.0
    }

    pub fn buffered(self) -> bool {
        self.interact_buffer_timer > 0.0
    }

    pub fn clear(&mut self) {
        self.interact_buffer_timer = 0.0;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Slot-keyed gesture/buffer state — the explicit authority for "which controller
/// wants to interact / morph / double-tap", replacing the old per-body
/// `PlayerInteractionState` component. Local input publishes into the slot; body
/// mode, interaction, and room transitions consume the slot of the body they act
/// on (defaulting to the controlled subject's slot), so a possessed body's gestures
/// come from the controller driving it, never from a privileged home avatar.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SlotInteractionState {
    slots: [SlotGestures; ambition_characters::brain::SlotControls::MAX_SLOTS],
}

impl SlotInteractionState {
    /// This slot's gestures (default for an out-of-range slot).
    pub fn get(&self, slot: PlayerSlot) -> SlotGestures {
        self.slots.get(slot.0 as usize).copied().unwrap_or_default()
    }

    /// **Mutable access to a slot's gestures — `None` for a slot that does not
    /// exist.**
    ///
    /// ⛔ **it used to CLAMP, and clamping a participant identifier is not a
    /// defensive measure, it is a wrong write to somebody else's controller.**
    /// `PlayerSlot(9)` resolved to the LAST valid slot, so a caller that had
    /// mistaken a slot could take player 4's buffered interact and consume it —
    /// a bug that presents as "somebody else's door opened" and can never be
    /// traced back to an index. (The comment on the old body was wrong in its
    /// own right: it promised a fallback to slot 0 while the code clamped to
    /// the last one, so neither the promise nor the behaviour was defensible.)
    ///
    /// ⭐ **`SlotControls` already models the honest policy** — a write to a
    /// slot that does not exist is ignored — and returning `Option` states the
    /// same thing where the caller can see it. [`get`](Self::get) still answers
    /// `default()` for an out-of-range READ, which is a different question with
    /// a defensible answer: a controller that does not exist is pressing
    /// nothing.
    pub fn get_mut(&mut self, slot: PlayerSlot) -> Option<&mut SlotGestures> {
        self.slots.get_mut(slot.0 as usize)
    }

    /// The local primary controller's gestures — the single-player default.
    pub fn primary(&self) -> SlotGestures {
        self.get(PlayerSlot::PRIMARY)
    }

    /// Mutable primary-controller gestures.
    ///
    /// ⚠ **the one unconditional accessor on this type, and it is sound by
    /// construction rather than by a caller's care**: the const assertion below
    /// pins that the primary slot is inside the array, so this cannot become
    /// the clamp it replaced by way of a later change to either constant.
    pub fn primary_mut(&mut self) -> &mut SlotGestures {
        &mut self.slots[PlayerSlot::PRIMARY.0 as usize]
    }
}

/// The primary slot exists. `primary_mut` indexes unconditionally on the
/// strength of this, so a build where the two constants disagreed would fail
/// here rather than panic in a frame.
const _: () =
    assert!((PlayerSlot::PRIMARY.0 as usize) < ambition_characters::brain::SlotControls::MAX_SLOTS);

#[cfg(test)]
#[path = "multiplayer_smoke_tests.rs"]
mod multiplayer_smoke_tests;

#[cfg(test)]
#[path = "slot_gesture_tests.rs"]
mod slot_gesture_tests;
