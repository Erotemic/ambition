//! Compatibility state for Ambition's held portal gun.
//!
//! Kept small and separate: the portal core is about apertures, transit, and
//! view math, not guns. Equip, inventory, and input policy are host-side.

use bevy::prelude::*;

use super::color::PortalGunColor;
use super::messages::{TogglePortalGun, TogglePortalGunsActive};

/// Held portal-gun state for the current Ambition compatibility workflow.
///
/// A gun owns one pair ([`PortalGunColor::pair`] of `next_color`), and nothing
/// changes it: [`portal_toggle_system`] flips only the end bit. For a second
/// pair, use a second gun ([`PortalGun::for_pair`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGun {
    /// When false the gun ignores input.
    pub active: bool,
    /// Gun color the next `Attack` will place. Always one of the two ends of
    /// this gun's own pair.
    pub next_color: PortalGunColor,
}

/// The pair of the portal gun this body OWNS, kept while the gun is not in hand.
///
/// Unequipping removes [`PortalGun`], so this keeps the pair through a trip
/// through the inventory menu. Written whenever a gun is equipped; not removed
/// when it is unequipped.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedPortalGunPair(pub u8);

impl PortalGun {
    /// An active gun owning `pair`, aimed at that pair's A end.
    ///
    /// Two guns built from different pairs never open a portal into each
    /// other's: pairing is by channel partner, and partners share a pair.
    pub fn for_pair(pair: u8) -> Self {
        Self {
            active: true,
            next_color: PortalGunColor::for_pair(pair),
        }
    }

    /// Which pair this gun owns — fixed for the gun's whole life.
    pub fn pair(self) -> u8 {
        self.next_color.pair()
    }
}

impl Default for PortalGun {
    /// The classic blue↔orange gun: pair 0.
    fn default() -> Self {
        Self::for_pair(0)
    }
}

/// On a [`TogglePortalGun`] intent, flip the gun to the other end of its own
/// pair (blue↔orange for the default gun). The adapter decides whether a
/// press is a portal toggle (not a door or NPC interaction). This never
/// changes the pair.
///
/// FIXME(portal-gun-seam): move this behind an optional gun plugin once generic
/// portal-opening emitters are first-class.
pub fn portal_toggle_system(
    mut toggles: MessageReader<TogglePortalGun>,
    mut guns: Query<&mut PortalGun>,
) {
    // Toggle the gun of the body that pressed. Several bodies can hold guns.
    for toggle in toggles.read() {
        let Ok(mut gun) = guns.get_mut(toggle.body) else {
            continue;
        };
        if gun.active {
            gun.next_color = gun.next_color.other();
        }
    }
}

/// On a [`TogglePortalGunsActive`] intent, turn every gun on or off. (sim)
///
/// A developer switch, so the gun does not fire portals on every Attack while
/// other mechanics are tested. The host reads the key and writes the intent;
/// this is the only writer of `PortalGun::active` for it.
pub fn portal_guns_active_toggle_system(
    mut toggles: MessageReader<TogglePortalGunsActive>,
    mut guns: Query<&mut PortalGun>,
) {
    for _ in toggles.read() {
        for mut gun in &mut guns {
            gun.active = !gun.active;
            bevy::log::info!(target: "ambition_portal2d", "portal gun active = {}", gun.active);
        }
    }
}

#[cfg(test)]
mod active_toggle_tests {
    use super::*;

    /// One developer press flips every gun once; no press changes nothing.
    #[test]
    fn one_press_turns_every_gun_the_other_way() {
        let mut app = App::new();
        app.add_message::<TogglePortalGunsActive>();
        app.add_systems(Update, portal_guns_active_toggle_system);
        let on = app.world_mut().spawn(PortalGun::default()).id();
        let off = app
            .world_mut()
            .spawn(PortalGun {
                active: false,
                ..PortalGun::default()
            })
            .id();
        app.update();
        assert!(app.world().get::<PortalGun>(on).unwrap().active, "no press");

        app.world_mut().write_message(TogglePortalGunsActive);
        app.update();
        assert!(!app.world().get::<PortalGun>(on).unwrap().active);
        assert!(app.world().get::<PortalGun>(off).unwrap().active);
    }
}
