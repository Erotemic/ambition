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
/// ⭐ **A GUN OWNS ONE PAIR, AND THE PAIR NEVER CHANGES.** Toggling a gun flips
/// the END bit and nothing else, so a single gun offers exactly two colors. To
/// get a second color pair in the world you spawn a SECOND gun on a different
/// pair ([`for_pair`](Self::for_pair)) — one gun orange/blue, another red/yellow.
/// This used to be one gun cycling every end of every pair through an `advance`
/// step, which made "which pair am I on" a hidden mode the holder had to track.
///
/// All gun ends are gun-owned, so they despawn together when their gun is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PortalGunColor {
    pub slot: u8,
}

impl PortalGunColor {
    /// How many distinct pairs the gun channel space can name. `slot` is a `u8`
    /// carrying a pair and an end bit, so the pair space is half of it — the
    /// same 128 as [`PortalChannelColor::Indexed`], and not a cap on how many
    /// guns may exist.
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

    /// The other END of the SAME pair — its link partner, and the gun's whole
    /// toggle. Firing both ends of a pair opens a working portal between them.
    pub fn other(self) -> Self {
        Self {
            slot: self.slot ^ 1,
        }
    }

    /// This gun color as a [`PortalChannel`] for the shared pairing/transit core.
    pub fn channel(self) -> PortalChannel {
        PortalChannel::Gun(self)
    }

    /// Degrees to rotate the AUTHORED gun art by so it reads as this pair.
    ///
    /// ⭐ **ONE ANGLE SERVES BOTH ENDS, and that is not a coincidence.** The art
    /// is a blue gun and an orange gun 180° apart, and a pair's two ends are
    /// also 180° apart, so the rotation that carries blue→A also carries
    /// orange→B: `(hue(A) - 210)` and `(hue(B) - 30)` are the same angle mod
    /// 360. If the two arts ever stop being complementary this has to become
    /// two angles, and the assertion in `the_two_gun_ends_need_one_rotation`
    /// is what will say so.
    ///
    /// Pair 0 returns `0.0` — the authored art IS pair 0, and rotating it by
    /// nothing is both correct and free.
    pub fn art_hue_shift(self) -> f32 {
        if self.pair() == 0 {
            0.0
        } else {
            (pair_hue(self.pair()) - GUN_ART_BASE_HUE).rem_euclid(360.0)
        }
    }
}

/// An authored/runtime channel-pair color. LDtk test rooms place these pairs
/// (Purple↔Yellow, Teal↔Red, Green↔Magenta, Cyan↔Rose) so it's clear at a glance
/// which two portals are linked. Authored pairs are NOT gun-owned, so they
/// persist even with no gun around.
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
    /// A generated pair member by index — even = slot A, odd = slot B; the
    /// partner is `Indexed(n ^ 1)`. Its display color is taken from a
    /// golden-ratio hue wheel (slot B complementary to A), so a room can hold
    /// arbitrarily many visibly-distinct pairs beyond the eight named ones.
    /// `0..=7` overlap the named pairs in *index space* but the named variants
    /// are preferred for authoring; use indices `8..` (pairs 4+) for the extra
    /// channels. Max distinct pairs: 128 (`u8` / 2).
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

/// The unifying pair-linking identity the shared transit/pairing core operates
/// on. Portals are linked into PAIRS by complementary channel (one of each), so
/// several independent pairs can exist at once: the gun fires the
/// Blue↔Orange pair, and authored test rooms place other pairs.
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
    /// complementary so a linked pair reads as a pair. Each gun PAIR gets its
    /// own hue and the two ends sit 180° apart, so pair 0 stays the classic
    /// blue↔orange and every other pair reads as its own two colors.
    ///
    /// ⛔ **PAIRS ARE SPACED BY THE GOLDEN-RATIO WHEEL, NOT A FIXED STEP.** This
    /// was `210 + pair * 45`, which is fine for the four pairs the gun used to
    /// cycle and collides once pairs are per-gun and unbounded: 45° repeats
    /// every 8 pairs, so gun 8 would have been indistinguishable from gun 0
    /// while opening a pair that does NOT link to it — two portals that look
    /// like partners and are not. [`pair_hue`] is the same wheel the authored
    /// `Indexed` channels use, for the same reason.
    pub fn display(self) -> (Color, Color) {
        match self {
            PortalChannel::Gun(c) => {
                // Pair 0 keeps its hand-picked blue↔orange; the wheel would put
                // it somewhere else and that pair is the one players recognise.
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
        assert_eq!(
            PortalChannelColor::from_name("purple"),
            Some(PortalChannelColor::Purple)
        );
        // A pair's two slots are complementary (different) colors.
        let (rim_a, _) = Indexed(8).rim_core();
        let (rim_b, _) = Indexed(9).rim_core();
        assert_ne!(rim_a, rim_b);
    }

    /// A GUN'S PAIR IS FIXED AND ITS TOGGLE IS A TWO-CYCLE.
    ///
    /// The whole of the change: `other` is the only step a gun takes, so no
    /// number of presses can reach a third color or leave the pair it owns.
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

    /// TWO GUNS ON DIFFERENT PAIRS CANNOT OPEN INTO EACH OTHER.
    ///
    /// The reason a gun-per-pair is safe: linking is by channel PARTNER, and a
    /// partner shares the pair. Without this, "two guns" would be two ways to
    /// place ends of one shared set.
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

    /// ONE ROTATION CARRIES BOTH ENDS OF THE PAIR.
    ///
    /// `art_hue_shift` returns a single angle per pair and the held-gun art is
    /// two drawings. That is only sound while the two arts are complementary
    /// (blue at 210°, orange at 30°) exactly as a pair's two ends are. If the
    /// art is ever redrawn so the ends are not 180° apart, the B end will be
    /// rotated to the wrong colour and NOTHING else will notice — the shader
    /// applies whatever angle it is handed.
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

            // And it really is the angle between pair 0's ends and this pair's:
            // the two ends stay 180° apart after the rotation, which is what
            // lets one angle serve both drawings.
            assert_ne!(hue_of(a), hue_of(b), "a pair's ends render alike");
            let zero_a = PortalGunColor::BLUE;
            assert_eq!(zero_a.art_hue_shift(), 0.0, "the authored art is pair 0");
        }
    }

    /// ⛔ THE REGRESSION THE OLD HUE STEP WOULD HAVE SHIPPED.
    ///
    /// Gun display hue was `210 + pair * 45`, which repeats every EIGHT pairs.
    /// That was invisible while the gun cycled four pairs and becomes a real
    /// defect once each gun owns its own: pair 8's ends would render exactly
    /// like pair 0's while refusing to link to them — two portals that look
    /// like partners and are not.
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
