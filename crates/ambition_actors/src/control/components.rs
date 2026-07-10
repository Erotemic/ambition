//! The control-seam STATE: one human's device frame, the slot it drives, and the
//! per-body input snapshot that slot's brain publishes.
//!
//! `LocalPlayer` says *this slot's input comes from this machine*.
//! `PlayerInputFrame` is the entity-local frame a body reads instead of the
//! global `Res<ControlFrame>`. `SlotGestures` / `SlotInteractionState` are
//! SLOT-level, not body-level: a gesture belongs to a controller, and follows it
//! onto whatever body it currently drives.

use bevy::prelude::*;

use ambition_input::ControlFrame;

// The slot marker every body-facing consumer keys on. Defined a tier down, in
// `ambition_characters::brain`, because a brain names its own slot.
pub use ambition_characters::brain::PlayerSlot;
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};

/// Marks a player whose input comes from this machine's input devices
/// (keyboard / gamepad / touch). In single-player today the local
/// player is also the primary player. In a future networked build,
/// remote players would have `PlayerEntity` (+ `PlayerSlot`) but not
/// `LocalPlayer`.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalPlayer;

/// Per-player input snapshot. Mirrors the single global
/// [`ambition_input::ControlFrame`] resource onto the local player
/// entity so simulation systems can move toward reading input from a
/// `Query<&PlayerInputFrame>` rather than `Res<ControlFrame>`. That's
/// the architectural seam multiplayer / netcode work needs:
///
/// - the local primary player's frame is filled by
///   `sync_local_player_input_frame` after the input pipeline writes
///   `Res<ControlFrame>`;
/// - future remote / co-op players would have their own
///   `PlayerInputFrame` populated by a network adapter or a second
///   input device, without competing for the single global resource.
///
/// Today exactly one entity (the local primary player) carries this
/// component, and `Res<ControlFrame>` stays the single writer channel.
/// New simulation systems should prefer this component so they're
/// already shaped for multiple input-bearing players.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerInputFrame {
    pub frame: ControlFrame,
}

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

    /// Mutable access to a slot's gestures; out-of-range slots fall back to slot 0
    /// so a bad index can never panic mid-frame.
    pub fn get_mut(&mut self, slot: PlayerSlot) -> &mut SlotGestures {
        let idx = (slot.0 as usize).min(Self::LAST);
        &mut self.slots[idx]
    }

    /// The local primary controller's gestures — the single-player default.
    pub fn primary(&self) -> SlotGestures {
        self.get(PlayerSlot::PRIMARY)
    }

    /// Mutable primary-controller gestures.
    pub fn primary_mut(&mut self) -> &mut SlotGestures {
        self.get_mut(PlayerSlot::PRIMARY)
    }

    const LAST: usize = ambition_characters::brain::SlotControls::MAX_SLOTS - 1;
}

#[cfg(test)]
#[path = "multiplayer_smoke_tests.rs"]
mod multiplayer_smoke_tests;
