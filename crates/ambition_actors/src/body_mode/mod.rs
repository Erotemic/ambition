//! Sandbox-side body-mode driver: facade re-exporting [`update_body_mode`].
//!
//! [`mechanics`] owns the whole driver — the crouch / climb / morph-ball
//! / stand-up state transitions read from input + contact state and ask
//! the engine to flip the player's `BodyMode`. (Morph-ball sprite visuals
//! live elsewhere, not in this module.)

mod mechanics;

pub use mechanics::update_body_mode;

use bevy::prelude::Component;

/// Per-body body-mode capability kit: which posture changes THIS body can
/// physically perform. The body-mode driver is capability-gated on this — a body
/// only crouches / morphs / climbs if it carries the matching flag, so the input
/// is a no-op for a body that lacks the capability (never a fallback to the home
/// avatar). Presence-gated: a body WITHOUT this component never body-modes at all.
///
/// This is the body-mode analogue of the movement kit on
/// [`crate::combat::CombatCapabilities`] (`can_blink` / `can_fly` / …): the
/// controller only *attempts* a posture change; the body decides whether it can.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyModeCapabilities {
    /// Can duck into a shorter Crouch stance while holding down + grounded.
    pub can_crouch: bool,
    /// Can curl into the smallest MorphBall stance (double-tap down).
    pub can_morph: bool,
    /// Can grab and climb a climbable region (ladder-style vertical span).
    pub can_climb: bool,
}

impl BodyModeCapabilities {
    /// The full player kit — every posture change enabled.
    pub const fn full() -> Self {
        Self {
            can_crouch: true,
            can_morph: true,
            can_climb: true,
        }
    }

    /// No posture changes — a body that only ever stands.
    pub const fn none() -> Self {
        Self {
            can_crouch: false,
            can_morph: false,
            can_climb: false,
        }
    }
}
