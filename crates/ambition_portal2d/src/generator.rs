//! Whatever opens portals.
//!
//! The portal crate knows only that SOMETHING is a portal generator: the
//! [`PortalGenerator`] component on whatever fires shots. What carries one, and
//! when, is the host's (Ambition's portal gun is an ordinary held item, and its
//! adapter gives the holder a generator while the gun is in hand).

use bevy::prelude::*;

use super::color::PortalGunColor;
use super::messages::{TogglePortalGenerator, TogglePortalGeneratorsActive};

/// A portal generator: the pair it opens and which end of it opens next.
///
/// A generator owns one pair ([`PortalGunColor::pair`] of `next_color`), and
/// nothing changes it: [`portal_toggle_system`] flips only the end bit. For a
/// second pair, use a second generator ([`PortalGenerator::for_pair`]).
#[derive(Component, Clone, Copy, Debug)]
pub struct PortalGenerator {
    /// When false the generator ignores input.
    pub active: bool,
    /// The end the next shot opens. Always one of the two ends of this
    /// generator's own pair.
    pub next_color: PortalGunColor,
}

/// The pair of the generator a body last held, kept while it holds none, so
/// the next generator it takes up opens the same pair.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnedPortalPair(pub u8);

impl PortalGenerator {
    /// An active generator owning `pair`, aimed at that pair's A end.
    ///
    /// Two generators of different pairs never open a portal into each
    /// other's: pairing is by channel partner, and partners share a pair.
    pub fn for_pair(pair: u8) -> Self {
        Self {
            active: true,
            next_color: PortalGunColor::for_pair(pair),
        }
    }

    /// Which pair this generator owns — fixed for its whole life.
    pub fn pair(self) -> u8 {
        self.next_color.pair()
    }
}

impl Default for PortalGenerator {
    /// The classic blue↔orange pair: pair 0.
    fn default() -> Self {
        Self::for_pair(0)
    }
}

/// On a [`TogglePortalGenerator`] intent, flip that generator to the other end
/// of its own pair (blue↔orange for pair 0). The host decides whether a press
/// is a toggle (not a door or NPC interaction). This never changes the pair.
pub fn portal_toggle_system(
    mut toggles: MessageReader<TogglePortalGenerator>,
    mut generators: Query<&mut PortalGenerator>,
) {
    for toggle in toggles.read() {
        let Ok(mut generator) = generators.get_mut(toggle.body) else {
            continue;
        };
        if generator.active {
            generator.next_color = generator.next_color.other();
        }
    }
}

/// On a [`TogglePortalGeneratorsActive`] intent, turn every generator on or
/// off. (sim)
///
/// A developer switch, so a generator does not fire portals on every Attack
/// while other mechanics are tested. The host reads the key and writes the
/// intent; this is the only writer of `PortalGenerator::active` for it.
pub fn portal_generators_active_toggle_system(
    mut toggles: MessageReader<TogglePortalGeneratorsActive>,
    mut generators: Query<&mut PortalGenerator>,
) {
    for _ in toggles.read() {
        for mut generator in &mut generators {
            generator.active = !generator.active;
            bevy::log::info!(target: "ambition_portal2d", "portal generator active = {}", generator.active);
        }
    }
}

#[cfg(test)]
mod active_toggle_tests {
    use super::*;

    /// One developer press flips every generator once; no press changes nothing.
    #[test]
    fn one_press_turns_every_generator_the_other_way() {
        let mut app = App::new();
        app.add_message::<TogglePortalGeneratorsActive>();
        app.add_systems(Update, portal_generators_active_toggle_system);
        let on = app.world_mut().spawn(PortalGenerator::default()).id();
        let off = app
            .world_mut()
            .spawn(PortalGenerator {
                active: false,
                ..PortalGenerator::default()
            })
            .id();
        app.update();
        assert!(app.world().get::<PortalGenerator>(on).unwrap().active, "no press");

        app.world_mut().write_message(TogglePortalGeneratorsActive);
        app.update();
        assert!(!app.world().get::<PortalGenerator>(on).unwrap().active);
        assert!(app.world().get::<PortalGenerator>(off).unwrap().active);
    }
}
