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
///
/// ⭐ **THIS GUN OWNS ONE PAIR.** Its pair is whichever
/// [`PortalGunColor::pair`] `next_color` was built with, and no operation on a
/// gun ever changes it — [`portal_toggle_system`] flips only the END bit. So
/// the gun is a two-color object, and a world with two color pairs in it is a
/// world with two guns ([`PortalGun::for_pair`]).
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
/// ⛔ **UNEQUIPPING REMOVES [`PortalGun`] — "the hand is the only record" — so
/// without this the pair would not survive a trip through the inventory menu.**
/// Owning a gun and holding one are different facts here: the menu can stash and
/// re-equip, and a red/yellow gun that came back blue/orange from the menu would
/// be the same bug as one that came back blue/orange off the floor, reached by a
/// different road. Written whenever a gun is equipped and deliberately NOT
/// removed when it is unequipped.
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

/// Flip the gun to the OTHER END OF ITS OWN PAIR, on a [`TogglePortalGun`]
/// intent — blue↔orange for the default gun, and the equivalent two ends for a
/// gun built on any other pair. The adapter decides *whether* a press is a
/// portal toggle (vs. a door / NPC interaction); core just applies the step.
/// Operates on the [`PortalGun`] component generically and never names the
/// controlling actor.
///
/// ⛔ **THIS WAS A FULL-PALETTE WALK AND IS NOW A TWO-STATE FLIP.** It called
/// `advance`, stepping blue₀ → orange₀ → blue₁ → orange₁ → … through all four
/// pairs, so one gun could open four independent pairs and the holder had to
/// remember which of eight ends the next shot would place. A gun now owns one
/// pair for its whole life; more pairs in the world means more GUNS, which is
/// a thing the player can see and pick up rather than a mode they must track.
/// `advance` is gone rather than left unused, so the walk cannot come back by
/// someone reaching for the obvious-looking method.
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
            gun.next_color = gun.next_color.other();
        }
    }
}

// NOTE: the dev `F7` off-switch (`portal_dev_toggle_system`) reads raw keyboard
// input — a HOST input / dev concern, not the gun mechanic — so it lives
// host-side (in the host's render-gated presentation), flipping `PortalGun.active`
// the same way. The crate owns only the message-driven `portal_toggle_system`.
