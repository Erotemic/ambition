//! The player-held [`PortalGun`]: active/next-color state and the color toggle.
//!
//! The Ambition equip/unequip glue (action-set stashing for the inventory menu)
//! lives in the host portal adapter; core keeps only
//! the gun component and the message-driven color toggle.

use bevy::prelude::*;

use super::color::PortalGunColor;
use super::messages::TogglePortalGun;

/// Player-held portal gun state.
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGun {
    /// When false the gun ignores input (stand-in for "not equipped" until
    /// held-item equip exists).
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
/// so the player can place up to [`PortalGunColor::PAIRS`] independent pairs
/// with a single toggle control. The adapter decides *whether* a press is a
/// portal toggle (vs. a door / NPC interaction); core just applies the step.
/// Operates on the [`PortalGun`] component generically (the gun mechanic) — it
/// never names the player, so it stays in the crate.
pub fn portal_toggle_system(
    mut toggles: MessageReader<TogglePortalGun>,
    mut guns: Query<&mut PortalGun>,
) {
    if toggles.read().next().is_none() {
        return;
    }
    let Ok(mut gun) = guns.single_mut() else {
        return;
    };
    if gun.active {
        gun.next_color = gun.next_color.advance();
    }
}

// NOTE: the dev `F7` off-switch (`portal_dev_toggle_system`) reads raw keyboard
// input — a HOST input / dev concern, not the gun mechanic — so it lives
// host-side (in the host's render-gated presentation), flipping `PortalGun.active`
// the same way. The crate owns only the message-driven `portal_toggle_system`.
