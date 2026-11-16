//! Timed numeric effects on a body, each owned by its source.
//!
//! ```text
//! intrinsic value (the authored params)
//! + every live effect on that attribute
//! -> the value the movement step reads
//! ```
//!
//! A source adds and ends only its own instance. Nothing writes the authored
//! value, so an expiry has nothing to restore, and a re-wear or a change of
//! tuning while an effect is live changes the base under it.
//!
//! The fold is [`fold`], which equipment uses too: `(base + Σ additions) ·
//! Π multipliers`. It does not depend on the order effects were applied in.
//!
//! Stacking: one instance per (source, attribute). A source's second
//! application replaces its first (Speed Shoes restart their clock); two
//! sources on one attribute both count, and either can end without the other.
//! A source that needs repeated independent instances takes distinct keys.

use crate::snapshot::{put_f32, put_u32, put_u64, put_u8, Reader, SnapshotState};

/// A number of a body that live effects change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NumericAttribute {
    /// A surface-momentum body's top speed.
    TopSpeed,
    /// A surface-momentum body's ground acceleration.
    GroundAccel,
    /// The multiplier on an axis-swept body's gravity (the kernel's product,
    /// after the frame's own scale).
    Gravity,
}

impl NumericAttribute {
    const fn code(self) -> u8 {
        match self {
            Self::TopSpeed => 0,
            Self::GroundAccel => 1,
            Self::Gravity => 2,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        Some(match code {
            0 => Self::TopSpeed,
            1 => Self::GroundAccel,
            2 => Self::Gravity,
            _ => return None,
        })
    }
}

/// What an effect (or a worn row's modifier) does to a number.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum NumericOp {
    Add(f32),
    Mul(f32),
}

/// `base` under `ops`: every addition first, then every multiplier, so the
/// result does not depend on the order of `ops` (`+10` and `x2` give
/// `(base + 10) x 2` either way).
pub fn fold(base: f32, ops: impl Iterator<Item = NumericOp> + Clone) -> f32 {
    let mut value = base;
    for op in ops.clone() {
        if let NumericOp::Add(add) = op { value += add; }
    }
    for op in ops {
        if let NumericOp::Mul(mul) = op { value *= mul; }
    }
    value
}

/// Who owns an effect: a stable key from a name, the same in every build and
/// on every peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EffectSource(pub u64);

impl EffectSource {
    /// The key of `name` (FNV-1a, 64 bit).
    pub const fn named(name: &str) -> Self {
        let bytes = name.as_bytes();
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            i += 1;
        }
        Self(hash)
    }

    /// A move's gravity beat and the aimed air dodge's weightless window. One
    /// source, so the latest of them replaces the other: two gravity scales
    /// multiplying into one body is a regime nobody authored.
    pub const MANEUVER_GRAVITY: Self = Self::named("movement.maneuver_gravity");
}

/// One live effect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericEffect {
    pub source: EffectSource,
    pub attribute: NumericAttribute,
    pub op: NumericOp,
    /// Seconds left on the body's movement clock.
    pub remaining_s: f32,
}

/// Every live numeric effect on a body: the authority for "Speed Shoes from
/// this source until then", not a cached result.
///
/// Required by [`crate::BodyKinematics`], so every body carries it, empty by
/// default. The movement step spends it ([`Self::spend`]) once a step, on
/// the clock every other maneuver timer runs on.
#[derive(bevy_ecs::component::Component, Clone, Debug, Default, PartialEq)]
pub struct BodyEffects {
    /// Sorted by (source, attribute), one entry each.
    effects: Vec<NumericEffect>,
}

impl BodyEffects {
    /// Put `op` on `attribute` from `source` for `seconds`, replacing that
    /// source's live instance on the attribute. A non-positive duration ends
    /// it instead.
    pub fn apply(&mut self, source: EffectSource, attribute: NumericAttribute, op: NumericOp, seconds: f32) {
        let at = self.effects.binary_search_by(|effect| (effect.source, effect.attribute).cmp(&(source, attribute)));
        if !(seconds > 0.0) {
            if let Ok(index) = at { self.effects.remove(index); }
            return;
        }
        let effect = NumericEffect { source, attribute, op, remaining_s: seconds };
        match at {
            Ok(index) => self.effects[index] = effect,
            Err(index) => self.effects.insert(index, effect),
        }
    }

    /// End every instance `source` owns. Others are untouched.
    pub fn end(&mut self, source: EffectSource) {
        self.effects.retain(|effect| effect.source != source);
    }

    /// End `source`'s instance on `attribute`.
    pub fn end_on(&mut self, source: EffectSource, attribute: NumericAttribute) {
        self.effects.retain(|effect| (effect.source, effect.attribute) != (source, attribute));
    }

    /// `base` under every live effect on `attribute`.
    pub fn resolve(&self, attribute: NumericAttribute, base: f32) -> f32 {
        fold(base, self.effects.iter().filter(|effect| effect.attribute == attribute).map(|effect| effect.op))
    }

    /// Is any effect from `source` live?
    pub fn holds(&self, source: EffectSource) -> bool {
        self.effects.iter().any(|effect| effect.source == source)
    }

    pub fn is_empty(&self) -> bool { self.effects.is_empty() }

    pub fn iter(&self) -> impl Iterator<Item = &NumericEffect> { self.effects.iter() }

    /// Spend `dt` of every effect; each ends when its own time runs out.
    pub fn spend(&mut self, dt: f32) {
        if dt <= 0.0 { return; }
        for effect in &mut self.effects { effect.remaining_s -= dt; }
        self.effects.retain(|effect| effect.remaining_s > 0.0);
    }
}

impl SnapshotState for BodyEffects {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.effects.len() as u32);
        for effect in &self.effects {
            put_u64(out, effect.source.0);
            put_u8(out, effect.attribute.code());
            let (kind, value) = match effect.op {
                NumericOp::Add(value) => (0, value),
                NumericOp::Mul(value) => (1, value),
            };
            put_u8(out, kind);
            put_f32(out, value);
            put_f32(out, effect.remaining_s);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let len = r.u32()? as usize;
        let mut effects = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            let source = EffectSource(r.u64()?);
            let attribute = NumericAttribute::from_code(r.u8()?)?;
            let op = match r.u8()? {
                0 => NumericOp::Add(r.f32()?),
                1 => NumericOp::Mul(r.f32()?),
                _ => return None,
            };
            effects.push(NumericEffect { source, attribute, op, remaining_s: r.f32()? });
        }
        effects.windows(2).all(|pair| (pair[0].source, pair[0].attribute) < (pair[1].source, pair[1].attribute))
            .then_some(Self { effects })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: EffectSource = EffectSource::named("test.a");
    const B: EffectSource = EffectSource::named("test.b");

    #[test]
    fn two_sources_on_one_number_end_apart_and_in_either_order_resolve_alike() {
        let mut ab = BodyEffects::default();
        ab.apply(A, NumericAttribute::TopSpeed, NumericOp::Mul(1.4), 0.5);
        ab.apply(B, NumericAttribute::TopSpeed, NumericOp::Mul(0.5), 1.0);
        let mut ba = BodyEffects::default();
        ba.apply(B, NumericAttribute::TopSpeed, NumericOp::Mul(0.5), 1.0);
        ba.apply(A, NumericAttribute::TopSpeed, NumericOp::Mul(1.4), 0.5);
        assert_eq!(ab, ba);
        assert!((ab.resolve(NumericAttribute::TopSpeed, 100.0) - 70.0).abs() < 1e-4);
        ab.spend(0.6);
        assert!(!ab.holds(A) && ab.holds(B), "A's expiry took B with it: {ab:?}");
        assert!((ab.resolve(NumericAttribute::TopSpeed, 100.0) - 50.0).abs() < 1e-4);
        ab.spend(0.6);
        assert!(ab.is_empty());
        assert_eq!(ab.resolve(NumericAttribute::TopSpeed, 100.0), 100.0);
    }

    #[test]
    fn a_sources_second_application_replaces_its_first_and_snapshots_round_trip() {
        let mut effects = BodyEffects::default();
        effects.apply(A, NumericAttribute::Gravity, NumericOp::Mul(0.25), 0.2);
        effects.apply(A, NumericAttribute::Gravity, NumericOp::Mul(0.0), 0.4);
        effects.apply(B, NumericAttribute::GroundAccel, NumericOp::Add(10.0), 0.4);
        assert_eq!(effects.resolve(NumericAttribute::Gravity, 1.0), 0.0);
        assert_eq!(effects.iter().count(), 2);
        let bytes = crate::snapshot::encode_state(&effects);
        let back = BodyEffects::decode(&mut Reader::new(&bytes)).unwrap();
        assert_eq!(back, effects);
        effects.apply(A, NumericAttribute::Gravity, NumericOp::Mul(0.5), 0.0);
        assert_eq!(effects.resolve(NumericAttribute::Gravity, 1.0), 1.0);
    }
}
