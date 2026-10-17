//! The vocabulary of procedural lightning: what a producer asks for, with no
//! renderer attached.
//!
//! Two shapes, both [`VfxMessage`](crate::VfxMessage) rows:
//!
//! * a BOLT between two ends (`VfxMessage::Lightning`): a strike from a
//!   ceiling, a lash from a neck, force lightning from a hand;
//! * an AURA on a body (`VfxMessage::LightningAura`): arcs that crawl over
//!   and jump off it, a powered-up body's crackle.
//!
//! Each is LEASED: a row names a `key`, and holds the effect for `hold`
//! seconds. A producer that writes the same key again before the hold runs
//! out renews it (and moves its ends), so a held channel is one bolt that
//! follows, not a stack of copies; one that stops writing lets it fade. A
//! rewind that rewrites the rows rebuilds the same effect. Presentation owns
//! the jagged path, its flicker and its forks; nothing reads them back.

use bevy::prelude::Entity;

use ambition_geometry as ae;

/// Where one end of a bolt is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightningEnd {
    /// A fixed world point (+y down).
    World(ae::Vec2),
    /// A point `offset` from where `owner` is drawn: it follows the body
    /// between ticks.
    Body { owner: Entity, offset: ae::Vec2 },
}

/// How a bolt or an aura looks. Widths are world units; colours are RGBA in
/// 0..1, drawn added to what is under them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightningLook {
    /// The halo round each strand.
    pub glow: [f32; 4],
    /// The white-hot middle of each strand.
    pub core: [f32; 4],
    /// The core's width.
    pub width: f32,
    /// The halo's width over the core's.
    pub glow_scale: f32,
    /// How far a bolt strays from its straight line, over its length.
    pub jag: f32,
    /// Side strands off a bolt; arcs alive at once on an aura.
    pub forks: u8,
    /// Seconds between new paths: the flicker.
    pub flicker_s: f32,
    /// Seconds a bolt takes to reach its far end. Zero: there at once.
    pub grow_s: f32,
    /// The radius of the light where a bolt lands (and at an aura's middle).
    /// Zero: none.
    pub contact_glow: f32,
}

impl LightningLook {
    /// A bolt from the sky: one thick strand, few forks, a flash where it lands.
    pub const STRIKE: LightningLook = LightningLook {
        glow: [0.35, 0.55, 1.0, 0.55],
        core: [0.92, 0.97, 1.0, 1.0],
        width: 3.0,
        glow_scale: 5.0,
        jag: 0.09,
        forks: 3,
        flicker_s: 0.045,
        grow_s: 0.05,
        contact_glow: 46.0,
    };

    /// A channel held from a hand or a node to a target: thinner, wilder,
    /// many forks, quick to re-path.
    pub const FORCE: LightningLook = LightningLook {
        glow: [0.5, 0.45, 1.0, 0.5],
        core: [0.95, 0.95, 1.0, 1.0],
        width: 1.8,
        glow_scale: 5.5,
        jag: 0.14,
        forks: 4,
        flicker_s: 0.035,
        grow_s: 0.08,
        contact_glow: 26.0,
    };

    /// Short arcs between two close points (a pair of neck bolts).
    pub const SPARK_GAP: LightningLook = LightningLook {
        glow: [0.45, 0.7, 1.0, 0.6],
        core: [0.95, 0.98, 1.0, 1.0],
        width: 1.2,
        glow_scale: 4.0,
        jag: 0.22,
        forks: 1,
        flicker_s: 0.05,
        grow_s: 0.0,
        contact_glow: 10.0,
    };

    /// A powered-up body's crackle (a Super Saiyan 2's): arcs that crawl the
    /// outline and leap off it, there and gone.
    pub const AURA: LightningLook = LightningLook {
        glow: [0.4, 0.62, 1.0, 0.65],
        core: [0.94, 0.98, 1.0, 1.0],
        width: 1.7,
        glow_scale: 4.5,
        jag: 0.2,
        forks: 8,
        flicker_s: 0.07,
        grow_s: 0.0,
        contact_glow: 0.0,
    };

    /// The look a module names on the lightning ports, or `None`.
    pub fn named(name: &str) -> Option<LightningLook> {
        Some(match name {
            "strike" => Self::STRIKE,
            "force" => Self::FORCE,
            "spark_gap" => Self::SPARK_GAP,
            "aura" => Self::AURA,
            _ => return None,
        })
    }

    /// This look in another hue: `glow`'s colour, the core kept white-hot.
    pub fn tinted(self, glow: [f32; 4]) -> LightningLook {
        LightningLook { glow, ..self }
    }
}

/// Which lease a row renews: the producer's own number for the effect. Two
/// producers keep apart by scoping their keys (the module ports mix in the
/// body that asked).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LightningKey(pub u64);

impl LightningKey {
    /// A key for `name`, asked for by `scope`: stable for the same pair.
    pub fn scoped(scope: Option<Entity>, name: &str) -> LightningKey {
        // FNV-1a: stable across runs and platforms, which `DefaultHasher` is
        // not promised to be.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |b: u8| {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        for b in scope.map_or(u64::MAX, Entity::to_bits).to_le_bytes() {
            eat(b);
        }
        for b in name.bytes() {
            eat(b);
        }
        LightningKey(h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_stable_for_its_pair_and_apart_for_another() {
        let a = Entity::from_raw_u32(7).unwrap();
        let b = Entity::from_raw_u32(8).unwrap();
        assert_eq!(LightningKey::scoped(Some(a), "lash"), LightningKey::scoped(Some(a), "lash"));
        assert_ne!(LightningKey::scoped(Some(a), "lash"), LightningKey::scoped(Some(b), "lash"));
        assert_ne!(LightningKey::scoped(Some(a), "lash"), LightningKey::scoped(Some(a), "bolts"));
        assert_ne!(LightningKey::scoped(None, "lash"), LightningKey::scoped(Some(a), "lash"));
    }

    #[test]
    fn every_named_look_is_found_and_an_unknown_one_is_not() {
        for name in ["strike", "force", "spark_gap", "aura"] {
            assert!(LightningLook::named(name).is_some(), "{name}");
        }
        assert_eq!(LightningLook::named("plasma"), None);
    }
}
