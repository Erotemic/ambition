//! Compatibility state for Ambition's held portal gun.
//!
//! This module is deliberately small and isolated: the portal crate's core
//! concern is linked apertures, transit, and view math, not the fact that one
//! Ambition opener happens to be a gun. Equip/unequip, inventory, and input
//! gesture policy remain host-side.

use bevy::prelude::*;

use super::color::PortalGunColor;
use super::messages::TogglePortalGun;

/// Held portal-gun state for the current Ambition compatibility workflow.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGun {
    /// When false the gun ignores input.
    pub active: bool,
    /// Gun color the next `Attack` will place.
    pub next_color: PortalGunColor,
}

impl Default for PortalGun {
    fn default() -> Self {
        Self {
            active: true,
            next_color: PortalGunColor::BLUE,
        }
    }
}

/// Advance the gun to the next color, on a [`TogglePortalGun`] intent. One
/// press walks through every end of every pair (blue₀ → orange₀ → blue₁ → …),
/// so the holder can place up to [`PortalGunColor::PAIRS`] independent pairs
/// with a single toggle control. The adapter decides *whether* a press is a
/// portal toggle (vs. a door / NPC interaction); core just applies the step.
/// Operates on the [`PortalGun`] component generically and never names the
/// controlling actor.
///
/// FIXME(portal-gun-seam): move this behind an optional gun plugin once generic
/// portal-opening emitters are first-class.
pub fn portal_toggle_system(
    mut toggles: MessageReader<TogglePortalGun>,
    mut guns: Query<&mut PortalGun>,
) {
    // ⭐ THE GUN THE PRESS WAS MADE ON. This was `guns.single_mut()`, which is a
    // claim that exactly one gun exists in the world — true for one seat and
    // false the moment a second body holds one, where it would silently refuse
    // to toggle either.
    for toggle in toggles.read() {
        let Ok(mut gun) = guns.get_mut(toggle.body) else {
            continue;
        };
        if gun.active {
            gun.next_color = gun.next_color.advance();
        }
    }
}

// NOTE: the dev `F7` off-switch (`portal_dev_toggle_system`) reads raw keyboard
// input — a HOST input / dev concern, not the gun mechanic — so it lives
// host-side (in the host's render-gated presentation), flipping `PortalGun.active`
// the same way. The crate owns only the message-driven `portal_toggle_system`.
