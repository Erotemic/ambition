//! The player-held [`PortalGun`]: active/next-color state and the color toggle.
//!
//! The Ambition equip/unequip glue (action-set stashing for the inventory menu)
//! lives in `crate::ambition_content::portal::inventory_adapter`; core keeps only
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
            next_color: PortalGunColor::Blue,
        }
    }
}

/// Flip which color the next shot will place, on a [`TogglePortalGun`] intent.
/// The adapter decides *whether* a press is a portal toggle (vs. a door / NPC
/// interaction); core just applies the flip. Operates on the [`PortalGun`]
/// component generically (the gun mechanic) — it never names the player, so it
/// stays in the crate. `PortalGun` only ever lives on the primary player today,
/// so the generic single-gun query resolves to exactly the same gun the old
/// `(PlayerEntity, PrimaryPlayer)`-filtered query did (identical-sim).
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
        gun.next_color = gun.next_color.other();
    }
}

/// Dev off-switch: `F7` toggles the portal gun active/inactive so the
/// always-on slice gun doesn't fire portals on every Attack while testing
/// other sandbox mechanics. (Visible build only.) Final gating is via
/// held-item equip; this is a developer convenience until then. Reads generic
/// Bevy keyboard input, not Ambition content, so it stays in core.
pub fn portal_dev_toggle_system(keys: Res<ButtonInput<KeyCode>>, mut guns: Query<&mut PortalGun>) {
    if !keys.just_pressed(KeyCode::F7) {
        return;
    }
    for mut gun in &mut guns {
        gun.active = !gun.active;
        bevy::log::info!(target: "ambition::portal", "portal gun active = {}", gun.active);
    }
}
