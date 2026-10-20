//! A blast: damage where a body is, credited to another body.
//!
//! A move that goes off later than it was set (a mark on a body, a fuse)
//! hurts everybody near the spot, its own setter included, and credits the
//! body that set it, which may since have left the match.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a blast.
///
/// Port card:
///
/// * **Operation** — a damage box of `half_extent` centred on `center`,
///   live for `lifetime_s`, that may hurt every body it
///   covers (that body and the credited one included: a blast takes no side),
///   with `damage` and `knockback` (a feel multiplier, not a launch speed).
///   The box is credited to the body that `credit` names (kills, grudges,
///   staleness, rage); when that body has left, to a stand-in that is no
///   participant, so the credit never falls to the body under the blast. A
///   `credit` of `0`, a centre that is not finite, a lifetime that is not
///   finite and more than 0, an extent or knockback that is not finite and
///   at least 0, and a negative damage are refused.
/// * **Owner** — `ambition_combat::extension::blast`.
/// * **Scope** — the invocation's scope; the blast is where `center` says (a
///   module reads where its body or entity is from its trigger).
/// * **Time** — `technique_execution` and `module_entity_tick`; the box meets
///   bodies this tick.
pub struct BlastPort;

#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    pub center: [f32; 2],
    pub half_extent: [f32; 2],
    pub damage: i32,
    pub knockback: f32,
    pub lifetime_s: f32,
    pub credit: u64,
}

impl Port for BlastPort {
    const KEY: PortKey = PortKey::new("ambition.combat.blast", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = Blast;

    fn encode(v: &Blast, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.center);
        wire::put_vec2(out, v.half_extent);
        wire::put_i32(out, v.damage);
        wire::put_f32(out, v.knockback);
        wire::put_f32(out, v.lifetime_s);
        wire::put_u64(out, v.credit);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Blast, WireError> {
        Ok(Blast {
            center: r.vec2()?,
            half_extent: r.vec2()?,
            damage: r.i32()?,
            knockback: r.f32()?,
            lifetime_s: r.f32()?,
            credit: r.u64()?,
        })
    }
}

/// The observation port marker for the credit that names the scope.
///
/// Port card:
///
/// * **Operation** — the credit that names the invocation's scope durably
///   (across a rewind, on every peer), for a [`BlastPort`] the scope itself
///   is credited with (a bomb owns its blast).
/// * **Owner** — `ambition_combat::extension::blast`.
/// * **Scope** — the invocation's scope.
/// * **Time** — offered in `technique_execution` and `module_entity_tick`.
/// * **Absence** — absent for a scope with no simulation identity.
pub struct CreditPort;

impl Port for CreditPort {
    const KEY: PortKey = PortKey::new("ambition.combat.credit", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = u64;

    fn encode(v: &u64, out: &mut Vec<u8>) {
        wire::put_u64(out, *v);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<u64, WireError> {
        r.u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A blast decodes to what was encoded.
    #[test]
    fn a_blast_crosses_the_wire_unchanged() {
        let blast = Blast { center: [3.0, -9.5], half_extent: [44.0, 44.0], damage: 6, knockback: 1.2, lifetime_s: 0.08, credit: 0xdead_beef_0000_0001 };
        let mut bytes = Vec::new();
        BlastPort::encode(&blast, &mut bytes);
        assert_eq!(BlastPort::decode(&mut WireReader::new(&bytes)).unwrap(), blast);
    }
}
