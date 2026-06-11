//! Portal pair-linking identity, split into two distinct domain types plus the
//! unifying [`PortalChannel`] that the shared transit/pairing core operates on.
//!
//! Two portals pair iff they share the same [`PortalChannel`]. The distinction
//! between gun colors and authored channel colors is REAL at the boundaries:
//!
//! - [`PortalGunColor`] — the gun's two-slot pair (Blue↔Orange). Used by the
//!   gun, its aim/mode indicator, and the gun's place-replace logic.
//! - [`PortalChannelColor`] — the authored channel pairs (Purple↔Yellow,
//!   Teal↔Red, Green↔Magenta, Cyan↔Rose). Used by LDtk authoring + the gate
//!   registry.
//!
//! Both map into [`PortalChannel`], over which [`PlacedPortal`], `transit_step`,
//! `find_portal`, the carve/registry, and `portal_teleport_ground_items` are
//! generic — so the shared machinery never needs to know whether a portal came
//! from the gun or from authoring.
//!
//! [`PlacedPortal`]: super::types::PlacedPortal

use bevy::prelude::Color;

/// The gun's two-slot pair. The portal gun fires **Blue↔Orange**; toggling the
/// gun swaps which one the next shot places. This is the only pair the gun
/// owns / despawns when the gun is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalGunColor {
    Blue,
    Orange,
}

impl PortalGunColor {
    /// The other slot of the gun's pair (its toggle target / pair partner).
    pub fn other(self) -> Self {
        match self {
            PortalGunColor::Blue => PortalGunColor::Orange,
            PortalGunColor::Orange => PortalGunColor::Blue,
        }
    }

    /// This gun color as a [`PortalChannel`] for the shared pairing/transit core.
    pub fn channel(self) -> PortalChannel {
        PortalChannel::Gun(self)
    }
}

/// An authored channel-pair color. LDtk test rooms place these pairs
/// (Purple↔Yellow, Teal↔Red, Green↔Magenta, Cyan↔Rose) so it's clear at a glance
/// which two portals are linked. Authored pairs are NOT gun-owned, so they
/// persist even with no gun around.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalChannelColor {
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
    /// A generated pair member by index — even = slot A, odd = slot B; the
    /// partner is `Indexed(n ^ 1)`. Its display color is taken from a
    /// golden-ratio hue wheel (slot B complementary to A), so a room can hold
    /// arbitrarily many visibly-distinct pairs beyond the eight named ones.
    /// `0..=7` overlap the named pairs in *index space* but the named variants
    /// are preferred for authoring; use indices `8..` (pairs 4+) for the extra
    /// channels. Max distinct pairs: 128 (`u8` / 2).
    Indexed(u8),
}

/// Golden-ratio hue (degrees) for generated pair `pair_index`, so successive
/// pairs are maximally far apart on the wheel.
fn pair_hue(pair_index: u8) -> f32 {
    (pair_index as f32 * 137.508).rem_euclid(360.0)
}

impl PortalChannelColor {
    /// The complementary authored color this channel is linked to (its partner).
    pub fn partner(self) -> Self {
        use PortalChannelColor::*;
        match self {
            Purple => Yellow,
            Yellow => Purple,
            Teal => Red,
            Red => Teal,
            Green => Magenta,
            Magenta => Green,
            Cyan => Rose,
            Rose => Cyan,
            Indexed(n) => Indexed(n ^ 1),
        }
    }

    /// This authored color as a [`PortalChannel`] for the shared core.
    pub fn channel(self) -> PortalChannel {
        PortalChannel::Authored(self)
    }

    /// Lowercase name, used in logs and as the LDtk authoring token. Generated
    /// channels are `c{index}` (e.g. `c8`).
    pub fn name(self) -> String {
        use PortalChannelColor::*;
        match self {
            Purple => "purple".into(),
            Yellow => "yellow".into(),
            Teal => "teal".into(),
            Red => "red".into(),
            Green => "green".into(),
            Magenta => "magenta".into(),
            Cyan => "cyan".into(),
            Rose => "rose".into(),
            Indexed(n) => format!("c{n}"),
        }
    }

    /// `(rim, core)` display tints for this authored channel. The eight named
    /// channels keep their hand-tuned colors; generated channels derive a
    /// saturated rim + light core from the [`pair_hue`] of their pair, with
    /// slot B taken 180° around so a pair reads complementary like the named
    /// ones.
    pub fn rim_core(self) -> (Color, Color) {
        use PortalChannelColor::*;
        let named = |rim: [f32; 3], core: [f32; 3]| {
            (
                Color::srgb(rim[0], rim[1], rim[2]),
                Color::srgb(core[0], core[1], core[2]),
            )
        };
        match self {
            Purple => named([0.55, 0.30, 0.95], [0.82, 0.66, 1.0]),
            Yellow => named([0.95, 0.85, 0.18], [1.0, 0.96, 0.66]),
            Teal => named([0.13, 0.76, 0.70], [0.64, 0.96, 0.92]),
            Red => named([0.92, 0.22, 0.25], [1.0, 0.62, 0.62]),
            Green => named([0.28, 0.80, 0.35], [0.72, 0.96, 0.74]),
            Magenta => named([0.92, 0.25, 0.80], [1.0, 0.70, 0.95]),
            Cyan => named([0.18, 0.92, 0.95], [0.70, 0.99, 1.0]),
            Rose => named([1.0, 0.40, 0.62], [1.0, 0.74, 0.84]),
            Indexed(n) => {
                // Slot B (odd) is the complementary hue of its pair.
                let hue = pair_hue(n / 2) + if n % 2 == 1 { 180.0 } else { 0.0 };
                (
                    Color::hsl(hue.rem_euclid(360.0), 0.72, 0.55),
                    Color::hsl(hue.rem_euclid(360.0), 0.85, 0.80),
                )
            }
        }
    }

    /// Parse an authored channel color from its [`name`](Self::name) (LDtk
    /// authoring). Case-insensitive. Gun colors (blue/orange) are NOT authorable.
    pub fn from_name(s: &str) -> Option<Self> {
        use PortalChannelColor::*;
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "purple" => Purple,
            "yellow" => Yellow,
            "teal" => Teal,
            "red" => Red,
            "green" => Green,
            "magenta" => Magenta,
            "cyan" => Cyan,
            "rose" => Rose,
            other => {
                // Generated channels: `c{index}` (e.g. `c8`, `c9`).
                let idx = other.strip_prefix('c')?.parse::<u8>().ok()?;
                Indexed(idx)
            }
        })
    }
}

/// The unifying pair-linking identity the shared transit/pairing core operates
/// on. Portals are linked into PAIRS by complementary channel (one of each), so
/// several independent pairs can exist at once: the gun fires the
/// **Blue↔Orange** pair, and authored test rooms place other pairs.
/// [`partner`](Self::partner) gives the linked channel.
///
/// Two portals pair iff their channels are partners. `Copy`/`PartialEq`/`Hash`
/// so it drops into registry / `HashMap` usage unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalChannel {
    /// The gun's pair (Blue/Orange).
    Gun(PortalGunColor),
    /// An authored channel pair.
    Authored(PortalChannelColor),
}

impl PortalChannel {
    /// The complementary channel this portal is linked to (its pair partner).
    pub fn partner(self) -> Self {
        match self {
            PortalChannel::Gun(c) => PortalChannel::Gun(c.other()),
            PortalChannel::Authored(c) => PortalChannel::Authored(c.partner()),
        }
    }

    /// True for the gun's pair — the only one the portal gun fires / owns, so the
    /// only one that despawns when the gun is gone. Authored pairs persist.
    pub fn is_gun_pair(self) -> bool {
        matches!(self, PortalChannel::Gun(_))
    }

    /// `(rim, core)` display colors for the portal bar — partners are visibly
    /// complementary so a linked pair reads as a pair.
    pub fn display(self) -> (Color, Color) {
        use PortalGunColor::*;
        match self {
            PortalChannel::Gun(Blue) => {
                (Color::srgb(0.30, 0.62, 1.0), Color::srgb(0.74, 0.92, 1.0))
            }
            PortalChannel::Gun(Orange) => {
                (Color::srgb(1.0, 0.55, 0.20), Color::srgb(1.0, 0.86, 0.55))
            }
            PortalChannel::Authored(c) => c.rim_core(),
        }
    }

    /// Lowercase name, used in logs and entity naming.
    pub fn name(self) -> String {
        match self {
            PortalChannel::Gun(PortalGunColor::Blue) => "blue".into(),
            PortalChannel::Gun(PortalGunColor::Orange) => "orange".into(),
            PortalChannel::Authored(c) => c.name(),
        }
    }
}

impl From<PortalGunColor> for PortalChannel {
    fn from(c: PortalGunColor) -> Self {
        PortalChannel::Gun(c)
    }
}

impl From<PortalChannelColor> for PortalChannel {
    fn from(c: PortalChannelColor) -> Self {
        PortalChannel::Authored(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generated channels pair by index parity, parse round-trip via `c{N}`,
    /// and yield distinct colors — so a room can hold many pairs past the eight
    /// named ones.
    #[test]
    fn indexed_channels_pair_parse_and_color() {
        use PortalChannelColor::Indexed;
        // Pair (8,9): partners of each other, distinct from named pairs.
        assert_eq!(Indexed(8).partner(), Indexed(9));
        assert_eq!(Indexed(9).partner(), Indexed(8));
        // Name round-trips through the LDtk token.
        assert_eq!(Indexed(8).name(), "c8");
        assert_eq!(PortalChannelColor::from_name("c8"), Some(Indexed(8)));
        assert_eq!(PortalChannelColor::from_name("purple"), Some(PortalChannelColor::Purple));
        // A pair's two slots are complementary (different) colors.
        let (rim_a, _) = Indexed(8).rim_core();
        let (rim_b, _) = Indexed(9).rim_core();
        assert_ne!(rim_a, rim_b);
    }
}
