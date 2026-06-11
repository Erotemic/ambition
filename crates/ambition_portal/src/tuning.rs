//! Runtime-tunable portal feel and convention policy.
//!
//! The portal map convention is intentionally a resource-facing enum here, even
//! though the pure math layer stores the live convention in a tiny global. That
//! lets dev tools edit it as ordinary Bevy state while the pure helpers stay
//! usable from tests and non-Bevy callers.

use bevy::prelude::*;

use crate::pieces::set_portal_map_rotation;
use crate::types::{MIN_EXIT_SPEED, TELEPORT_COOLDOWN_S};

/// Which isometry glues a portal pair together.
#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum PortalConvention {
    /// Historical det -1 portal map: tangents are preserved and normals flip.
    #[default]
    Reflection,
    /// Proper det +1 portal map: the entry-facing chart rotates into the exit.
    Rotation,
}

impl PortalConvention {
    /// The boolean convention used by the pure geometry layer.
    pub const fn is_rotation(self) -> bool {
        matches!(self, Self::Rotation)
    }

    /// Convert from the pure geometry layer's boolean convention.
    pub const fn from_rotation(rotation: bool) -> Self {
        if rotation {
            Self::Rotation
        } else {
            Self::Reflection
        }
    }
}

/// Portal mechanics and adapter feel knobs surfaced in the F3 dev inspector.
#[derive(Resource, Clone, Copy, Debug, Reflect)]
#[reflect(Resource)]
pub struct PortalTuning {
    /// Active portal map convention. Exposed as a combo box by the inspector.
    pub convention: PortalConvention,
    /// Budget for portal-aware logic raycasts. Current production fire traces do
    /// not recurse yet, but tests/tools can use this instead of hard-coding.
    pub raycast_recursion_depth: u32,
    /// Minimum exit speed along the exit normal after a body transfers.
    pub min_exit_speed: f32,
    /// Per-body anti-ping-pong latch after a transfer.
    pub teleport_cooldown_s: f32,
    /// Duration of the input guard that prevents immediate pushback into the
    /// exit wall.
    pub emission_time_s: f32,
    /// Stick/axis magnitude above which movement counts as held.
    pub input_held_epsilon: f32,
    /// Cosine threshold before a changed held direction drops the input warp.
    pub input_warp_keep_cos: f32,
    /// While a player is in a portal aperture, disable wall movement abilities
    /// so carved aperture edges cannot catch them.
    pub suppress_wall_abilities: bool,
}

impl Default for PortalTuning {
    fn default() -> Self {
        Self {
            convention: PortalConvention::Reflection,
            raycast_recursion_depth: 4,
            min_exit_speed: MIN_EXIT_SPEED,
            teleport_cooldown_s: TELEPORT_COOLDOWN_S,
            emission_time_s: 0.18,
            input_held_epsilon: 0.25,
            input_warp_keep_cos: 0.5,
            suppress_wall_abilities: true,
        }
    }
}

/// Mirror editable Bevy state into the pure portal-map convention.
pub fn sync_portal_tuning_convention(tuning: Res<PortalTuning>) {
    set_portal_map_rotation(tuning.convention.is_rotation());
}
