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
        }
    }

    /// This authored color as a [`PortalChannel`] for the shared core.
    pub fn channel(self) -> PortalChannel {
        PortalChannel::Authored(self)
    }

    /// Lowercase name, used in logs and as the LDtk authoring token.
    pub fn name(self) -> &'static str {
        use PortalChannelColor::*;
        match self {
            Purple => "purple",
            Yellow => "yellow",
            Teal => "teal",
            Red => "red",
            Green => "green",
            Magenta => "magenta",
            Cyan => "cyan",
            Rose => "rose",
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
            _ => return None,
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
        use PortalChannelColor::*;
        use PortalGunColor::*;
        match self {
            PortalChannel::Gun(Blue) => {
                (Color::srgb(0.30, 0.62, 1.0), Color::srgb(0.74, 0.92, 1.0))
            }
            PortalChannel::Gun(Orange) => {
                (Color::srgb(1.0, 0.55, 0.20), Color::srgb(1.0, 0.86, 0.55))
            }
            PortalChannel::Authored(Purple) => {
                (Color::srgb(0.55, 0.30, 0.95), Color::srgb(0.82, 0.66, 1.0))
            }
            PortalChannel::Authored(Yellow) => {
                (Color::srgb(0.95, 0.85, 0.18), Color::srgb(1.0, 0.96, 0.66))
            }
            PortalChannel::Authored(Teal) => {
                (Color::srgb(0.13, 0.76, 0.70), Color::srgb(0.64, 0.96, 0.92))
            }
            PortalChannel::Authored(Red) => {
                (Color::srgb(0.92, 0.22, 0.25), Color::srgb(1.0, 0.62, 0.62))
            }
            PortalChannel::Authored(Green) => {
                (Color::srgb(0.28, 0.80, 0.35), Color::srgb(0.72, 0.96, 0.74))
            }
            PortalChannel::Authored(Magenta) => {
                (Color::srgb(0.92, 0.25, 0.80), Color::srgb(1.0, 0.70, 0.95))
            }
            PortalChannel::Authored(Cyan) => {
                (Color::srgb(0.18, 0.92, 0.95), Color::srgb(0.70, 0.99, 1.0))
            }
            PortalChannel::Authored(Rose) => {
                (Color::srgb(1.0, 0.40, 0.62), Color::srgb(1.0, 0.74, 0.84))
            }
        }
    }

    /// Lowercase name, used in logs and entity naming.
    pub fn name(self) -> &'static str {
        match self {
            PortalChannel::Gun(PortalGunColor::Blue) => "blue",
            PortalChannel::Gun(PortalGunColor::Orange) => "orange",
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
