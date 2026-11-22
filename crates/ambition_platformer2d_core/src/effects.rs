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
//! ONE APPLICATION PER SOURCE, WITH ONE INTERVAL (effects 6c). A source's
//! application carries any number of terms, on any attributes, two on one
//! attribute included, and they all end together: Speed Shoes are one pair of
//! shoes on two numbers, not two timers that happen to start together. A
//! source's second application restarts its interval (a second pair of shoes
//! restarts the clock); two sources on one attribute both count, and either
//! can end without the other. A source that needs repeated independent
//! instances takes distinct keys.

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

/// One source's live application: its terms and their one interval.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectApplication {
    pub source: EffectSource,
    /// What it does: each term an operation on one attribute. Two terms may
    /// name one attribute.
    pub terms: Vec<(NumericAttribute, NumericOp)>,
    /// Seconds left on the body's movement clock, for every term at once.
    pub remaining_s: f32,
}

/// Every live numeric application on a body: the authority for "Speed Shoes
/// from this source until then", not a cached result.
///
/// Required by [`crate::BodyKinematics`], so every body carries it, empty by
/// default. The movement step spends it ([`Self::spend`]) once a step, on
/// the clock every other maneuver timer runs on.
#[derive(bevy_ecs::component::Component, Clone, Debug, Default, PartialEq)]
pub struct BodyEffects {
    /// Sorted by source, one application each, none without terms.
    applications: Vec<EffectApplication>,
}

impl BodyEffects {
    fn at(&self, source: EffectSource) -> Result<usize, usize> {
        self.applications.binary_search_by(|application| application.source.cmp(&source))
    }

    /// `source`'s application is now `terms` for `seconds`: one interval for
    /// all of them, replacing whatever the source had. No terms, or a
    /// non-positive duration, ends it instead.
    pub fn grant(&mut self, source: EffectSource, terms: &[(NumericAttribute, NumericOp)], seconds: f32) {
        let at = self.at(source);
        if !(seconds > 0.0) || terms.is_empty() {
            if let Ok(index) = at { self.applications.remove(index); }
            return;
        }
        let application = EffectApplication { source, terms: terms.to_vec(), remaining_s: seconds };
        match at {
            Ok(index) => self.applications[index] = application,
            Err(index) => self.applications.insert(index, application),
        }
    }

    /// Put `op` on `attribute` in `source`'s application, replacing its term
    /// on that attribute, and restart the application's one interval at
    /// `seconds`. A non-positive duration ends that term instead.
    pub fn apply(&mut self, source: EffectSource, attribute: NumericAttribute, op: NumericOp, seconds: f32) {
        if !(seconds > 0.0) {
            self.end_on(source, attribute);
            return;
        }
        let mut terms = match self.at(source) {
            Ok(index) => std::mem::take(&mut self.applications[index].terms),
            Err(_) => Vec::new(),
        };
        terms.retain(|(on, _)| *on != attribute);
        terms.push((attribute, op));
        self.grant(source, &terms, seconds);
    }

    /// End `source`'s application. Others are untouched.
    pub fn end(&mut self, source: EffectSource) {
        if let Ok(index) = self.at(source) { self.applications.remove(index); }
    }

    /// End `source`'s terms on `attribute`; the rest of its application keeps
    /// its interval.
    pub fn end_on(&mut self, source: EffectSource, attribute: NumericAttribute) {
        let Ok(index) = self.at(source) else { return };
        self.applications[index].terms.retain(|(on, _)| *on != attribute);
        if self.applications[index].terms.is_empty() {
            self.applications.remove(index);
        }
    }

    /// `base` under every live term on `attribute`.
    pub fn resolve(&self, attribute: NumericAttribute, base: f32) -> f32 {
        fold(base, self.applications.iter().flat_map(|application| application.terms.iter())
            .filter(|(on, _)| *on == attribute).map(|(_, op)| *op))
    }

    /// Is `source`'s application live?
    pub fn holds(&self, source: EffectSource) -> bool {
        self.at(source).is_ok()
    }

    pub fn is_empty(&self) -> bool { self.applications.is_empty() }

    pub fn iter(&self) -> impl Iterator<Item = &EffectApplication> { self.applications.iter() }

    /// Spend `dt` of every application; each ends, all its terms together,
    /// when its own time runs out.
    pub fn spend(&mut self, dt: f32) {
        if dt <= 0.0 { return; }
        for application in &mut self.applications { application.remaining_s -= dt; }
        self.applications.retain(|application| application.remaining_s > 0.0);
    }
}

impl SnapshotState for BodyEffects {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.applications.len() as u32);
        for application in &self.applications {
            put_u64(out, application.source.0);
            put_f32(out, application.remaining_s);
            put_u32(out, application.terms.len() as u32);
            for (attribute, op) in &application.terms {
                put_u8(out, attribute.code());
                let (kind, value) = match op {
                    NumericOp::Add(value) => (0, *value),
                    NumericOp::Mul(value) => (1, *value),
                };
                put_u8(out, kind);
                put_f32(out, value);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let len = r.u32()? as usize;
        let mut applications = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            let source = EffectSource(r.u64()?);
            let remaining_s = r.f32()?;
            let count = r.u32()? as usize;
            let mut terms = Vec::with_capacity(count.min(16));
            for _ in 0..count {
                let attribute = NumericAttribute::from_code(r.u8()?)?;
                let op = match r.u8()? {
                    0 => NumericOp::Add(r.f32()?),
                    1 => NumericOp::Mul(r.f32()?),
                    _ => return None,
                };
                terms.push((attribute, op));
            }
            if terms.is_empty() { return None; }
            applications.push(EffectApplication { source, terms, remaining_s });
        }
        applications.windows(2).all(|pair| pair[0].source < pair[1].source)
            .then_some(Self { applications })
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

    /// ⭐ ONE APPLICATION, ONE INTERVAL, ANY TERMS (effects 6c). Two terms on
    /// one attribute and one on another all start and end together; a second
    /// grant restarts the one interval; ending one attribute keeps the rest
    /// and its clock; the snapshot keeps the terms and the interval.
    #[test]
    fn an_application_owns_one_interval_for_all_its_terms() {
        let mut effects = BodyEffects::default();
        let terms = [
            (NumericAttribute::TopSpeed, NumericOp::Add(10.0)),
            (NumericAttribute::TopSpeed, NumericOp::Mul(2.0)),
            (NumericAttribute::GroundAccel, NumericOp::Mul(1.5)),
        ];
        effects.grant(A, &terms, 1.0);
        assert!((effects.resolve(NumericAttribute::TopSpeed, 100.0) - 220.0).abs() < 1e-4, "two channels on one attribute");
        assert!((effects.resolve(NumericAttribute::GroundAccel, 10.0) - 15.0).abs() < 1e-4);
        effects.spend(0.6);
        effects.grant(A, &terms, 1.0);
        effects.spend(0.6);
        assert!(effects.holds(A), "the second grant restarted the one interval");
        let bytes = crate::snapshot::encode_state(&effects);
        assert_eq!(BodyEffects::decode(&mut Reader::new(&bytes)).unwrap(), effects);
        effects.end_on(A, NumericAttribute::TopSpeed);
        assert_eq!(effects.resolve(NumericAttribute::TopSpeed, 100.0), 100.0);
        assert!((effects.resolve(NumericAttribute::GroundAccel, 10.0) - 15.0).abs() < 1e-4, "the rest keeps its clock");
        effects.spend(0.5);
        assert!(effects.is_empty(), "every term ended with the one interval");
    }
}
