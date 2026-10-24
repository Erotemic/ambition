//! A field of force a module holds over a region of its room: a wind, a
//! tide, the low weight of space.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a held field of force.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — every body of the room whose box overlaps the box
///   `min`..`max` changes its velocity this tick by two parts:
///   * `lift` (0..1) takes that part of the body's own gravity away: a lift
///     of 0.8 leaves a fifth of its weight, the low weight of space. Gravity
///     is an acceleration, so it is the same for a light and a heavy body.
///   * `force` (world units per second per second on a body of mass 1, +Y
///     down) pushes it: a body of mass `m` takes `force / m` (Newton's second
///     law). A tide that throws the player across the room moves the giant
///     an eighth as far.
///
///   It does not turn which way a body's feet point.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the bodies of the requester's own live room. `slot` names
///   the field among the requester's fields.
/// * **Time** — offered in `boss_conduct`. The field is HELD: it acts on each
///   tick its requester asks for it, and on no other. The change goes into
///   the body's velocity on that tick. The field has no state of its own, so
///   a rewind and a replay give the same velocities.
/// * **Read model** — world units, +Y down.
pub struct HeldForcePort;

#[derive(Clone, Debug, PartialEq)]
pub struct HeldForce {
    pub slot: u32,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub lift: f32,
    pub force: [f32; 2],
}

impl Port for HeldForcePort {
    const KEY: PortKey = PortKey::new("ambition.world.held_force", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldForce;

    fn encode(v: &HeldForce, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        wire::put_vec2(out, v.min);
        wire::put_vec2(out, v.max);
        wire::put_f32(out, v.lift);
        wire::put_vec2(out, v.force);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldForce, WireError> {
        Ok(HeldForce { slot: r.u32()?, min: r.vec2()?, max: r.vec2()?, lift: r.f32()?, force: r.vec2()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_force_survives_the_wire() {
        let force = HeldForce { slot: 2, min: [0.0, -10.0], max: [1800.0, 1280.0], lift: 0.8, force: [600.0, 0.0] };
        let mut out = Vec::new();
        HeldForcePort::encode(&force, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(HeldForcePort::decode(&mut r).unwrap(), force);
        r.finish().unwrap();
    }
}
