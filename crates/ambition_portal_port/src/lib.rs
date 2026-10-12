//! The portal domain's extension ports.
//!
//! Port card for [`HeldPortalPairPort`] (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — keep a linked pair of portals in the world for as long
//!   as the module asks for it. A body that goes into one end comes out of
//!   the other. The pair is named by its SLOT.
//! * **Owner** — `ambition_portal2d::extension`, which spawns, moves and
//!   despawns the two portal entities. The module never holds an entity.
//! * **Identity** — the pair's link is the owner's simulation identity, the
//!   entry and the slot. The portal core gives the link a channel pair of
//!   the owner's live room, so two owners in one room have two pairs, and a
//!   pair keeps its channels while it is held.
//! * **Scope and grant** — the owner is the body the invocation ran for. The
//!   pair is in the owner's live room.
//! * **Time** — offered in `boss_conduct`. Each tick the adapter runs, for
//!   each body: a held pair that the body's entry did NOT submit this tick is
//!   despawned; a submitted slot that is not held is spawned; a submitted
//!   slot that is held is kept, and its ends move to the submitted places. A
//!   pair whose owner is gone is despawned.
//! * **Read model** — world units, +Y down. A `normal` points out of the
//!   line into the room.
//! * **Look** — each end's colour is an authored channel index: presentation
//!   only. Pairing never reads it.
//! * **Replay** — the adapter's record of what each body holds is rollback
//!   state on the body.
//! * **Result** — submitted is not applied. No acknowledgement port yet.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// One end of a held pair: where its line is, the side of the line that the
/// room is on, and the half length of the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalEnd {
    pub at: [f32; 2],
    pub normal: [f32; 2],
    pub half_length: f32,
}

/// A held pair of portals a module asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldPortalPair {
    /// The module's name for the pair. Slots are per entry.
    pub slot: u32,
    /// The two ends. A body that goes into one comes out of the other.
    pub ends: [PortalEnd; 2],
    /// The authored channel index that draws each end: a look only.
    pub looks: [u8; 2],
}

/// The request port marker for a HELD pair of portals. See the port card
/// at the top of this crate.
pub struct HeldPortalPairPort;

fn put_end(out: &mut Vec<u8>, end: &PortalEnd) {
    wire::put_vec2(out, end.at);
    wire::put_vec2(out, end.normal);
    wire::put_f32(out, end.half_length);
}

fn end(r: &mut WireReader<'_>) -> Result<PortalEnd, WireError> {
    Ok(PortalEnd {
        at: r.vec2()?,
        normal: r.vec2()?,
        half_length: r.f32()?,
    })
}

impl Port for HeldPortalPairPort {
    const KEY: PortKey = PortKey::new("ambition.portal.held_pair", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldPortalPair;

    fn encode(v: &HeldPortalPair, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        put_end(out, &v.ends[0]);
        put_end(out, &v.ends[1]);
        wire::put_u8(out, v.looks[0]);
        wire::put_u8(out, v.looks[1]);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldPortalPair, WireError> {
        Ok(HeldPortalPair {
            slot: r.u32()?,
            ends: [end(r)?, end(r)?],
            looks: [r.u8()?, r.u8()?],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_pair_comes_back_from_the_wire_equal() {
        let pair = HeldPortalPair {
            slot: 3,
            ends: [
                PortalEnd { at: [10.0, -20.0], normal: [0.0, -1.0], half_length: 44.0 },
                PortalEnd { at: [300.0, 5.5], normal: [0.0, 1.0], half_length: 44.0 },
            ],
            looks: [24, 25],
        };
        let mut bytes = Vec::new();
        HeldPortalPairPort::encode(&pair, &mut bytes);
        let mut reader = WireReader::new(&bytes);
        assert_eq!(HeldPortalPairPort::decode(&mut reader).unwrap(), pair);
        assert!(reader.finish().is_ok());
    }
}
