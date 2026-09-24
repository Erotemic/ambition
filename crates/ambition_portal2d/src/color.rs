//! Portal pair-linking identity.
//!
//! The shared transit/pairing core operates on [`PortalChannel`]: two portals
//! pair iff their channels are partners. Ambition currently has two channel
//! origins at the boundary:
//!
//! - [`PortalGunColor`] — compatibility slots for Ambition's current gun-owned
//!   portal workflow.
//! - [`PortalChannelColor`] — authored/runtime channel colors for level-placed
//!   and scriptable portals.
//!
//! Both map into [`PortalChannel`], over which [`PlacedPortal`], `transit_step`,
//! `find_portal`, the carve/registry, and `portal_teleport_ground_items` are
//! generic.
//!
//! FIXME(portal-api): a standalone crate should likely expose an opaque
//! host-defined channel/key type plus optional color helpers, instead of baking
//! Ambition's gun palette into the public core API.
//!
//! [`PlacedPortal`]: super::types::PlacedPortal

use bevy::prelude::Color;

/// One end of one gun-owned portal pair. `slot` packs both facts: `pair =
/// slot / 2`, end = `slot & 1` (0 = the "blue"/A end, 1 = the "orange"/B end).
/// Two ends of the SAME pair are [`other`](Self::other) partners — they link.
///
/// A gun owns one pair, and the pair never changes. Toggling a gun flips only
/// the end bit, so one gun has exactly two colors. For another pair, spawn a
/// second gun with [`for_pair`](Self::for_pair).
///
/// All gun ends are gun-owned, so they despawn together when their gun is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalGunColor {
    pub slot: u8,
}

impl PortalGunColor {
    /// How many distinct pairs the gun channel space can name: half of a `u8`
    /// slot, the same 128 as [`PortalChannelColor::Indexed`]. Not a cap on the
    /// number of guns.
    pub const PAIRS: u8 = 128;
    /// Pair 0, end A — the classic "blue" entrance and the default gun's.
    pub const BLUE: Self = Self { slot: 0 };
    /// Pair 0, end B — the classic "orange" exit.
    pub const ORANGE: Self = Self { slot: 1 };

    /// End A of `pair`. The entry point for giving a gun its own pair; pairs
    /// wrap at [`PAIRS`](Self::PAIRS) so the `* 2` below cannot overflow.
    pub fn for_pair(pair: u8) -> Self {
        Self {
            slot: (pair % Self::PAIRS) * 2,
        }
    }

    /// Which pair this end belongs to.
    pub fn pair(self) -> u8 {
        self.slot / 2
    }

    /// Which END of the pair this is: `false` = A ("blue"), `true` = B ("orange").
    pub fn is_end_b(self) -> bool {
        self.slot & 1 == 1
    }

    /// The other end of the same pair: its link partner, and the gun's toggle.
    pub fn other(self) -> Self {
        Self {
            slot: self.slot ^ 1,
        }
    }

    /// This gun color as a [`PortalChannel`] for the shared pairing/transit core.
    pub fn channel(self) -> PortalChannel {
        PortalChannel::Gun(self)
    }

    /// Degrees to rotate the authored gun art so it shows this pair.
    ///
    /// One angle serves both ends: the blue and orange arts are 180° apart, as
    /// are a pair's two ends, so `(hue(A) - 210)` and `(hue(B) - 30)` are equal
    /// mod 360. If the arts stop being complementary, this must become two
    /// angles; `the_two_gun_ends_need_one_rotation` checks it. Pair 0 returns
    /// `0.0`, because the authored art is pair 0.
    pub fn art_hue_shift(self) -> f32 {
        if self.pair() == 0 {
            0.0
        } else {
            (pair_hue(self.pair()) - GUN_ART_BASE_HUE).rem_euclid(360.0)
        }
    }
}

/// An authored or runtime channel-pair color. LDtk test rooms place these pairs
/// (Purple↔Yellow, Teal↔Red, Green↔Magenta, Cyan↔Rose) so linked portals are
/// easy to see. Authored pairs are not gun-owned, so they persist without a gun.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PortalChannelColor {
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
    /// A generated pair member by index: even is slot A, odd is slot B, and
    /// the partner is `Indexed(n ^ 1)`. Its color comes from a golden-ratio hue
    /// wheel (B complementary to A). `0..=7` overlap the named pairs in index
    /// space; author with the named variants and use `8..` for extra channels.
    /// Max distinct pairs: 128.
    Indexed(u8),
}

/// The hue the authored blue gun art is drawn at — pair 0's A end, and the
/// reference every other pair's art rotation is measured from.
const GUN_ART_BASE_HUE: f32 = 210.0;

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

/// The pair-linking identity the transit and pairing core uses. Two portals
/// pair iff their channels are partners ([`partner`](Self::partner)), so
/// several independent pairs can exist at once.
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

    /// True for a gun-owned pair, which despawns when its gun is gone.
    /// Authored pairs persist.
    pub fn is_gun_pair(self) -> bool {
        matches!(self, PortalChannel::Gun(_))
    }

    /// `(rim, core)` display colors for the portal bar. Partners are
    /// complementary. Each gun pair has its own hue with ends 180° apart; pair
    /// 0 is blue↔orange.
    ///
    /// Pairs use the golden-ratio wheel ([`pair_hue`]), not a fixed step. A
    /// fixed step repeats, and two unlinked pairs would look the same.
    pub fn display(self) -> (Color, Color) {
        match self {
            PortalChannel::Gun(c) => {
                // Pair 0 keeps its hand-picked blue↔orange.
                let base = if c.pair() == 0 {
                    210.0
                } else {
                    pair_hue(c.pair())
                };
                let hue = (base + if c.is_end_b() { 180.0 } else { 0.0 }).rem_euclid(360.0);
                (Color::hsl(hue, 0.78, 0.58), Color::hsl(hue, 0.90, 0.82))
            }
            PortalChannel::Authored(c) => c.rim_core(),
        }
    }

    /// Lowercase name, used in logs and entity naming.
    pub fn name(self) -> String {
        match self {
            PortalChannel::Gun(c) => match c.slot {
                0 => "blue".into(),
                1 => "orange".into(),
                _ => format!(
                    "gun_p{}{}",
                    c.pair(),
                    if c.slot & 1 == 0 { "a" } else { "b" }
                ),
            },
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

    /// Generated channels pair by index parity, round-trip through `c{N}`, and
    /// have distinct colors.
    #[test]
    fn indexed_channels_pair_parse_and_color() {
        use PortalChannelColor::Indexed;
        // Pair (8,9): partners of each other, distinct from named pairs.
        assert_eq!(Indexed(8).partner(), Indexed(9));
        assert_eq!(Indexed(9).partner(), Indexed(8));
        // Name round-trips through the LDtk token.
        assert_eq!(Indexed(8).name(), "c8");
        assert_eq!(PortalChannelColor::from_name("c8"), Some(Indexed(8)));
        assert_eq!(
            PortalChannelColor::from_name("purple"),
            Some(PortalChannelColor::Purple)
        );
        // A pair's two slots are complementary (different) colors.
        let (rim_a, _) = Indexed(8).rim_core();
        let (rim_b, _) = Indexed(9).rim_core();
        assert_ne!(rim_a, rim_b);
    }

    /// A gun's pair is fixed and its toggle is a two-cycle: `other` never
    /// reaches a third color.
    #[test]
    fn a_guns_toggle_never_leaves_its_own_pair() {
        for pair in [0u8, 1, 7, 42, 127] {
            let a = PortalGunColor::for_pair(pair);
            assert_eq!(a.pair(), pair, "for_pair({pair}) did not land on its pair");
            assert!(!a.is_end_b(), "for_pair must start on the A end");

            // Two ends, and pressing twice is the identity.
            let b = a.other();
            assert_eq!(b.pair(), pair, "the toggle left the gun's pair");
            assert!(b.is_end_b());
            assert_eq!(b.other(), a, "toggle is not an involution");

            // Any number of presses only ever yields those two.
            let mut seen = std::collections::HashSet::new();
            let mut cur = a;
            for _ in 0..16 {
                seen.insert(cur);
                cur = cur.other();
            }
            assert_eq!(
                seen.len(),
                2,
                "a gun on pair {pair} reached {} colors, not two",
                seen.len()
            );
        }
    }

    /// Two guns on different pairs cannot link: a partner always shares the
    /// pair.
    #[test]
    fn guns_on_different_pairs_never_link() {
        let orange_blue = PortalGunColor::for_pair(0);
        let other_gun = PortalGunColor::for_pair(3);
        for a in [orange_blue, orange_blue.other()] {
            for b in [other_gun, other_gun.other()] {
                assert_ne!(a, b, "two pairs share an end");
                assert_ne!(
                    a.channel().partner(),
                    b.channel(),
                    "an end of pair 0 links to an end of pair 3"
                );
            }
        }
    }

    /// One rotation carries both ends of the pair. `art_hue_shift` returns one
    /// angle, which is correct only while the two arts are complementary (blue
    /// 210°, orange 30°). Nothing else checks this.
    #[test]
    fn the_two_gun_ends_need_one_rotation() {
        for pair in [1u8, 2, 9, 40] {
            let a = PortalGunColor::for_pair(pair);
            let b = a.other();
            let hue_of = |color: PortalGunColor| {
                let (rim, _) = PortalChannel::Gun(color).display();
                rim
            };
            // The rotation the A end needs, taken from the colours themselves.
            let shift = a.art_hue_shift();
            assert_eq!(shift, b.art_hue_shift(), "the pair's ends disagree");

            // The two ends stay 180° apart after the rotation.
            assert_ne!(hue_of(a), hue_of(b), "a pair's ends render alike");
            let zero_a = PortalGunColor::BLUE;
            assert_eq!(zero_a.art_hue_shift(), 0.0, "the authored art is pair 0");
        }
    }

    /// Gun pair hues must not repeat: a pair that looks like another but does
    /// not link to it would confuse players.
    #[test]
    fn distant_gun_pairs_do_not_render_alike() {
        let zero = PortalChannel::Gun(PortalGunColor::for_pair(0));
        let eight = PortalChannel::Gun(PortalGunColor::for_pair(8));
        assert_ne!(zero.display().0, eight.display().0);
        // And the classic pair keeps the look players know it by.
        let (rim, _) = zero.display();
        assert_eq!(rim, Color::hsl(210.0, 0.78, 0.58));
        let (orange_rim, _) = PortalChannel::Gun(PortalGunColor::ORANGE).display();
        assert_eq!(orange_rim, Color::hsl(30.0, 0.78, 0.58));
    }
}
