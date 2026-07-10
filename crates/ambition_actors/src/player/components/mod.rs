//! Player ECS components.
//!
//! The player entity carries one of each of these as its frame-to-frame
//! authoritative state. See [`super::bundles::PlayerSimulationBundle`] for
//! the canonical spawn shape.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_input::ControlFrame;

// Re-export generic player markers from the platformer runtime.
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
// Stable facade for the player-slot marker used by brain/player code.
pub use ambition_characters::brain::PlayerSlot;

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

/// Player money — abstract coin/credits balance shown on the HUD and spent at
// The body's coin/credits wallet is now `ambition_characters::actor::BodyWallet` (body
// vocabulary — players AND currency-dropping NPCs carry it).

// Player combat/timer state is now the unified `ambition_characters::actor::BodyCombat` (the
// keystone collapse of `BodyCombat` + the actor read-model into one body
// combat component). The player fills the reaction-timer fields; the actor fills
// the status/attack fields.

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

/// Camera easing and blink-in presentation state. Authoritative ECS component;
/// written by `cleanup_timers_system`, `load_room`, and `handle_player_events`
/// (blink path). Read by the camera follow system and the sprite animator.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PlayerBlinkCameraState {
    /// Counts down from `blink_in_duration` to 0 after a blink; the camera
    /// and animator use this to play the arrival ease-in.
    pub blink_in_timer: f32,
    /// Set to `BLINK_IN_ANIM_TIME` when a blink fires; used to normalise
    /// `blink_in_timer` into a 0..1 progress value.
    pub blink_in_duration: f32,
    /// World-space camera position at the moment the blink fired; the camera
    /// eases from here toward the new player position.
    pub blink_camera_from: ambition_engine_core::Vec2,
    /// Blink destination in world space (set alongside `blink_camera_from`
    /// for future use; not yet consumed by the camera easing path).
    pub blink_camera_to: ambition_engine_core::Vec2,
    /// Positive while the camera should snap (not ease) to the player position.
    /// Set on door transitions; zero on edge exits to allow scroll effects.
    pub camera_snap_timer: f32,
}

impl Default for PlayerBlinkCameraState {
    fn default() -> Self {
        Self {
            blink_in_timer: 0.0,
            blink_in_duration: 0.0,
            blink_camera_from: ambition_engine_core::Vec2::ZERO,
            blink_camera_to: ambition_engine_core::Vec2::ZERO,
            camera_snap_timer: 0.0,
        }
    }
}

impl PlayerBlinkCameraState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Per-player "last known safe spot" used by hazard knockback and debug
/// respawn helpers. Stored on each player so future co-op builds keep safe
/// anchors independent.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSafetyState {
    /// Last grounded, gameplay-safe position the safety gate
    /// approved (see `crate::remember_safe_player_position`). The
    /// hazard / OOB respawn path warps the player here.
    pub last_safe_pos: ae::Vec2,
}

impl PlayerSafetyState {
    pub fn new(initial: ae::Vec2) -> Self {
        Self {
            last_safe_pos: initial,
        }
    }
}

#[cfg(test)]
mod multiplayer_smoke_tests;
