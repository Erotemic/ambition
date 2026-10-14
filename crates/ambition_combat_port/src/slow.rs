//! A slow on the bodies in a box: honey, glue, tar.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a slow on the bodies in a box.
///
/// Port card (`docs/planning/engine/extension-domain-contracts.md`):
///
/// * **Operation** — every body whose box overlaps the box `center` ±
///   `half_extent` this tick moves at `speed_scale` of its own speed for
///   `seconds` (`BodyCombat::slow_timer`). A body already slowed keeps the
///   longer of the two times and the stronger of the two slows.
/// * **Owner** — `ambition_combat::extension`.
/// * **Scope** — every body in the requester's own live room, of any side,
///   that can be hit (in play, on the playable plane), except the requester.
///   A slow is a thing in the world (a puddle), not an attack, so it holds
///   whoever steps in it.
/// * **Time** — offered in `boss_conduct`, which runs after the bodies have
///   integrated: the slow holds from the next tick's body step on.
/// * **Read model** — world units, +Y down.
/// * **Result** — submitted is not applied. No acknowledgement port yet.
pub struct SlowBodiesPort;

#[derive(Clone, Debug, PartialEq)]
pub struct SlowBodies {
    pub center: [f32; 2],
    pub half_extent: [f32; 2],
    /// The fraction of its own speed a slowed body keeps, in `(0, 1]`.
    pub speed_scale: f32,
    pub seconds: f32,
}

impl Port for SlowBodiesPort {
    const KEY: PortKey = PortKey::new("ambition.combat.slow_bodies", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = SlowBodies;

    fn encode(v: &SlowBodies, out: &mut Vec<u8>) {
        wire::put_vec2(out, v.center);
        wire::put_vec2(out, v.half_extent);
        wire::put_f32(out, v.speed_scale);
        wire::put_f32(out, v.seconds);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<SlowBodies, WireError> {
        Ok(SlowBodies {
            center: r.vec2()?,
            half_extent: r.vec2()?,
            speed_scale: r.f32()?,
            seconds: r.f32()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slow_survives_the_wire() {
        let slow = SlowBodies { center: [3.0, -4.0], half_extent: [30.0, 7.0], speed_scale: 0.4, seconds: 1.5 };
        let mut out = Vec::new();
        SlowBodiesPort::encode(&slow, &mut out);
        let mut r = WireReader::new(&out);
        assert_eq!(SlowBodiesPort::decode(&mut r).unwrap(), slow);
        r.finish().unwrap();
    }
}
