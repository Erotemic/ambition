//! [`PortalColor`] — the pair-linking color of a portal, plus channel
//! parse/display helpers.

use bevy::prelude::Color;

/// A portal's color. Portals are linked into PAIRS by complementary color (one
/// of each), so several independent pairs can exist at once: the gun fires the
/// **Blue↔Orange** pair, and authored test rooms place other pairs
/// (Purple↔Yellow, Teal↔Red, Green↔Magenta) so it's clear at a glance which two
/// portals are linked. [`partner`](Self::partner) gives the linked color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PortalColor {
    Blue,
    Orange,
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
}

impl PortalColor {
    /// The complementary color this portal is linked to (its pair partner).
    pub fn partner(self) -> Self {
        use PortalColor::*;
        match self {
            Blue => Orange,
            Orange => Blue,
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

    /// Back-compat alias for [`partner`](Self::partner) (the gun's blue↔orange toggle).
    pub fn other(self) -> Self {
        self.partner()
    }

    /// True for the gun's pair — the only one the portal gun fires / owns, so the
    /// only one that despawns when the gun is gone. Authored pairs persist.
    pub fn is_gun_pair(self) -> bool {
        matches!(self, PortalColor::Blue | PortalColor::Orange)
    }

    /// `(rim, core)` display colors for the portal bar — partners are visibly
    /// complementary so a linked pair reads as a pair.
    pub fn display(self) -> (Color, Color) {
        use PortalColor::*;
        match self {
            Blue => (Color::srgb(0.30, 0.62, 1.0), Color::srgb(0.74, 0.92, 1.0)),
            Orange => (Color::srgb(1.0, 0.55, 0.20), Color::srgb(1.0, 0.86, 0.55)),
            Purple => (Color::srgb(0.55, 0.30, 0.95), Color::srgb(0.82, 0.66, 1.0)),
            Yellow => (Color::srgb(0.95, 0.85, 0.18), Color::srgb(1.0, 0.96, 0.66)),
            Teal => (Color::srgb(0.13, 0.76, 0.70), Color::srgb(0.64, 0.96, 0.92)),
            Red => (Color::srgb(0.92, 0.22, 0.25), Color::srgb(1.0, 0.62, 0.62)),
            Green => (Color::srgb(0.28, 0.80, 0.35), Color::srgb(0.72, 0.96, 0.74)),
            Magenta => (Color::srgb(0.92, 0.25, 0.80), Color::srgb(1.0, 0.70, 0.95)),
            Cyan => (Color::srgb(0.18, 0.92, 0.95), Color::srgb(0.70, 0.99, 1.0)),
            Rose => (Color::srgb(1.0, 0.40, 0.62), Color::srgb(1.0, 0.74, 0.84)),
        }
    }

    /// Lowercase name, used in logs and as the LDtk authoring token.
    pub fn name(self) -> &'static str {
        use PortalColor::*;
        match self {
            Blue => "blue",
            Orange => "orange",
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

    /// Parse a color from its [`name`](Self::name) (LDtk authoring). Case-insensitive.
    pub fn from_name(s: &str) -> Option<Self> {
        use PortalColor::*;
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "blue" => Blue,
            "orange" => Orange,
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
