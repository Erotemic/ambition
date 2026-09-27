//! Ambition portal → player-ability / player-input adapters.
//!
//! [`withhold_wall_verbs_during_transit`]: while a body carries the
//! portal-owned [`PortalTransit`] latch, withhold its wall abilities
//! (ledge-grab / cling / wall-jump / wall-climb) so they don't grab the carved
//! aperture edges. It contributes to the body's ability projection behind an
//! Ambition toggle ([`SuppressWallAbilitiesInPortal`]), so it is Ambition glue,
//! not crate core. The seat's input guards are the runtime's
//! (`ambition_platformer2d_runtime::warp_portal_input`).

use bevy::prelude::*;

use ambition_portal2d::{PortalTransit, PortalTuning};

/// Runtime toggle for [`withhold_wall_verbs_during_transit`]. Default ON; flip it
/// off to play with ledge-grab / wall-movement INTO portals enabled (the
/// "ledge-grab through a portal" experiment — see TODO.md). Toggleable at runtime
/// (e.g. via the inspector) so both behaviors can be tried without a recompile.
///
/// This is an Ambition ability-policy toggle (the suppressed thing is a PLAYER
/// ability), so it lives with the adapter, not in the portal crate.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SuppressWallAbilitiesInPortal(pub bool);

impl Default for SuppressWallAbilitiesInPortal {
    fn default() -> Self {
        Self(true)
    }
}

/// The portal crossing's key in a body's [`AbilityContributions`].
///
/// [`AbilityContributions`]: ambition_platformer2d_core::AbilityContributions
pub const PORTAL_TRANSIT: &str = "portal.transit";

/// Every verb except the four that grab a wall.
const NO_WALL_VERBS: ambition_platformer2d_core::AbilitySet = ambition_platformer2d_core::AbilitySet {
    ledge_grab: false,
    wall_cling: false,
    wall_jump: false,
    wall_climb: false,
    ..ambition_platformer2d_core::AbilitySet::ALL
};

/// While a body is mid-transit, withhold its wall verbs (ledge-grab, cling,
/// wall-jump, wall-climb) so it doesn't latch onto the carved aperture EDGES —
/// the carve splits the host block, and those new edges read as grabbable
/// ledges / climbable walls, so a body would cling "into" a portal and pop back
/// out the entry instead of sinking through and crossing.
///
/// A ceiling contribution, not an edit of the effective set: when the transit
/// ends the portal withdraws it, and the body's verbs are whatever its base and
/// its other contributions make them. BODY-GENERIC: the hazard is a property of
/// transiting, not of being the primary player. Gated on
/// [`PortalTuning::suppress_wall_abilities`].
pub fn withhold_wall_verbs_during_transit(
    tuning: Res<PortalTuning>,
    mut bodies: Query<(
        &mut ambition_platformer2d_core::AbilityContributions,
        Has<PortalTransit>,
    )>,
) {
    for (mut contributions, transiting) in &mut bodies {
        let withhold = transiting && tuning.suppress_wall_abilities;
        match (withhold, contributions.get(PORTAL_TRANSIT).is_some()) {
            (true, false) => contributions.set(
                PORTAL_TRANSIT,
                ambition_platformer2d_core::AbilityContribution::Ceiling(NO_WALL_VERBS),
            ),
            (false, true) => contributions.clear(PORTAL_TRANSIT),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests;
