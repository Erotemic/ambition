//! Player POLICY components — the state that is genuinely slot-0's, not any
//! body's.
//!
//! The control seam (`LocalPlayer`, `PlayerInputFrame`, the slot gesture state)
//! left for `crate::control` in the S5/S6 fold; the body vocabulary
//! (`BodyAnimFacts`, `BodyMelee`) left for `crate::actor`. What remains is
//! camera easing and respawn safety — decisions about the local human's
//! experience, which no other body has.

use ambition_engine_core as ae;
use bevy::prelude::*;

// Re-export generic player markers from the platformer runtime.
pub use ambition_platformer_primitives::markers::{PlayerEntity, PrimaryPlayer};
// Stable facade for the player-slot marker used by brain/player code.
pub use ambition_characters::brain::PlayerSlot;

/// Player money — abstract coin/credits balance shown on the HUD and spent at
// The body's coin/credits wallet is now `ambition_characters::actor::BodyWallet` (body
// vocabulary — players AND currency-dropping NPCs carry it).

// Player combat/timer state is now the unified `ambition_characters::actor::BodyCombat` (the
// keystone collapse of `BodyCombat` + the actor read-model into one body
// combat component). The player fills the reaction-timer fields; the actor fills
// the status/attack fields.

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
