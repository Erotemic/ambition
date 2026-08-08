//! Player POLICY components — the state that is genuinely slot-0's, not any
//! body's.
//!
//! The control seam (`LocalPlayer`, `PlayerInputFrame`, the slot gesture state)
//! left for `crate::control` in the S5/S6 fold; the body vocabulary
//! (`BodyAnimFacts`, `BodyMelee`) left for `crate::actor`. What remains is
//! camera easing and respawn safety — decisions about the local human's
//! experience, which no other body has.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

// Re-export generic player markers from the platformer runtime.
pub use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
// Stable facade for the player-slot marker used by brain/player code.
pub use ambition_characters::brain::PlayerSlot;

/// Player money — abstract coin/credits balance shown on the HUD and spent at
// The body's coin/credits wallet is now `ambition_characters::actor::BodyWallet` (body
// vocabulary — players AND currency-dropping NPCs carry it).

// Player combat/timer state is now the unified `ambition_characters::actor::BodyCombat` (the
// keystone collapse of `BodyCombat` + the actor read-model into one body
// combat component). The player fills the reaction-timer fields; the actor fills
// the status/attack fields.

// Camera easing and blink-in presentation state is now
// `ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState`. It
// carried no actor-domain type — four `f32`s and two `Vec2`s — and every reader
// outside this crate (`ambition_sim_view`'s pose/camera snapshots, the runtime's
// reset / room-transition / rollback paths) sits ABOVE this crate, so owning it
// here made the actor crate a way-station on an edge that never needed it.
// Named from its owner; deliberately NOT re-exported.

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
