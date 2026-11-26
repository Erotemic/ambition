//! The portal domain's extension ports.
//!
//! Port card for [`HeldPortalPairPort`] (`docs/planning/engine/extension-domain-contracts.md`):
//!
//! * **Operation** — keep a linked pair of portals in the world for as long
//!   as the module asks for it. A body that goes into one end comes out of
//!   the other. The pair is named by its SLOT, which is the module's: any of
//!   its entries asks for the same pair by it.
//! * **Owner** — `ambition_portal2d::extension`, which spawns, moves and
//!   despawns the two portal entities. The module never holds an entity.
//! * **Identity** — the pair's link is the owner's simulation identity, the
//!   module and the slot. The portal core gives the link a channel pair of
//!   the owner's live room, so two owners in one room have two pairs, and a
//!   pair keeps its channels while it is held.
//! * **Scope and grant** — the owner is the body the invocation ran for. The
//!   pair is in the owner's live room.
//! * **Time** — offered in `boss_conduct` and `technique_execution`. Each
//!   tick the adapter of a phase runs, for each body: a pair held in that
//!   phase that no entry of its module submitted this tick is despawned; a
//!   submitted slot that is not held is spawned; a submitted slot that is
//!   held is kept, and its ends move to the submitted places. A pair whose
//!   owner is gone is despawned. A pair held in one phase is not released by
//!   the other's.
//! * **Use** — a pair asked with `closes_on_use` closes the first time a
//!   body comes out of either end, and stays closed while its slot is asked
//!   for: a module that wants a new pair asks in a new slot.
//! * **Read model** — world units, +Y down. A `normal` points out of the
//!   line into the room. `half_thickness` is how deep an end's opening is;
//!   `None` is the portal's own.
//! * **Look** — each end's colour is an authored channel index: presentation
//!   only. Pairing never reads it.
//! * **Replay** — the adapter's record of what each body holds is rollback
//!   state on the body.
//! * **Result** — submitted is not applied. [`HeldPortalPairsPort`] reads
//!   back each slot the body's modules hold: open, or spent (it closed on
//!   use, or fizzled because an end would overlap a portal). A slot asked
//!   for and absent was refused (an invalid end, or an owner with no
//!   simulation identity or live room; logged).
//! * **Scripts** — reachable from Rhai as a record: `ends` and `looks` are
//!   lists of two. The wire stays the version 2 field order.
//!
//! Port card for [`LightLinePort`]:
//!
//! * **Operation** — draw a line of a portal's light that is NO portal:
//!   nothing goes through it, and it pushes nothing. A look, for a thing
//!   that stays in a portal (an arm in its line). The line is named by its
//!   SLOT.
//! * **Owner** — `ambition_portal2d::extension` keeps this tick's lines of
//!   each body as a view on the body (`HeldLightLines`), and
//!   `ambition_portal2d_presentation` draws each line of the view. The
//!   simulation spawns nothing for a line.
//! * **Scope and grant** — the owner is the body the invocation ran for. The
//!   line is drawn in the owner's live room.
//! * **Time** — offered in `boss_conduct`. A line that the body's entry did
//!   NOT submit this tick is not drawn. A line whose owner is gone is not
//!   drawn.
//! * **Read model** — world units, +Y down. A `normal` points out of the
//!   line into the room.
//! * **Look** — the authored channel index whose colour the line has.
//! * **On a limb** (v2) — a line may name the limb whose cut it is on
//!   (`LineOnLimb`). The presentation then puts the line on the seam that the
//!   limb's drawn picture publishes, as wide as the limb there and over it.
//! * **Replay** — presentation: the view is derived again each tick from the
//!   module's record, and no simulation system reads it.
//! * **Result** — submitted is not applied. The observation
//!   `ambition.portal.held_pairs` replies: each held slot's
//!   [`HeldPairOutcome`] (open, pending, closed on use, fizzled, refused).

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// One end of a held pair: where its line is, the side of the line that the
/// room is on, the half length of the line, and how deep its opening is
/// (`None`: the portal's own).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalEnd {
    pub at: [f32; 2],
    pub normal: [f32; 2],
    pub half_length: f32,
    pub half_thickness: Option<f32>,
}

/// A held pair of portals a module asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldPortalPair {
    /// The module's name for the pair. Slots are per module: the entry that
    /// opens a pair and the entry that keeps it open name one pair.
    pub slot: u32,
    /// The two ends. A body that goes into one comes out of the other.
    pub ends: [PortalEnd; 2],
    /// The authored channel index that draws each end: a look only.
    pub looks: [u8; 2],
    /// Close the pair the first time a body comes out of it.
    pub closes_on_use: bool,
}

ambition_extension_sdk::port_record! {
    PortalEnd {
        /// The middle of the end's line, world units, +Y down.
        at: [f32; 2],
        /// Out of the line into the room.
        normal: [f32; 2],
        /// Half the line's length.
        half_length: f32,
        /// How deep the opening is; absent is the portal's own.
        half_thickness: Option<f32> = None,
    }
}
ambition_extension_sdk::port_record! {
    HeldPortalPair {
        /// The module's name for the pair.
        slot: u32,
        /// The two ends; a body that goes into one comes out of the other.
        ends: [PortalEnd; 2] as ambition_extension_sdk::structure::Pair<PortalEnd>,
        /// The authored channel index that draws each end.
        looks: [u8; 2] as ambition_extension_sdk::structure::Pair<u8> = [0, 0],
        /// Close the pair the first time a body comes out of it.
        closes_on_use: bool = false,
    }
}

/// The request port marker for a HELD pair of portals. See the port card
/// at the top of this crate.
pub struct HeldPortalPairPort;

/// The observation port marker for the held pairs of the invocation's body:
/// each slot its modules hold, and whether its ends are open.
pub struct HeldPortalPairsPort;

ambition_extension_sdk::port_choices! {
    /// What became of a held slot's ask, as the adapter admitted it: the
    /// reply a module reads instead of keeping its own books on a spent slot.
    pub enum HeldPairOutcome {
        /// Both ends are open and joined: a body can go through.
        Open = 0 => "open",
        /// Admitted, but the ends are not joined yet (no channel for the
        /// pair, or an end is gone): not a way through.
        Pending = 1 => "pending",
        /// It closed when a body came out of it (`closes_on_use`). Closed
        /// while its slot is asked; a new slot opens a new pair.
        ClosedOnUse = 2 => "closed_on_use",
        /// An end would overlap a portal, so it never opened or it closed.
        /// Closed while its slot is asked.
        Fizzled = 3 => "fizzled",
        /// Refused: an end is not a valid portal (a non-finite place, a
        /// zero normal or opening). Asked again, it is judged again.
        RefusedInvalid = 4 => "refused_invalid",
        /// Refused: the body has no simulation identity or is in no live
        /// room. Asked again, it is judged again.
        RefusedNoRoom = 5 => "refused_no_room",
    }
}

/// One held slot.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldPairState {
    pub module: String,
    pub slot: u32,
    pub outcome: HeldPairOutcome,
}
/// Every slot a body's modules hold, by module and slot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeldPairs {
    pub pairs: Vec<HeldPairState>,
}
ambition_extension_sdk::port_record! {
    HeldPairState {
        /// `provider::module`, the module that holds it.
        module: String,
        /// The module's slot.
        slot: u32,
        /// What became of its ask.
        outcome: HeldPairOutcome,
    }
}
ambition_extension_sdk::port_record! {
    HeldPairs {
        /// The held slots, by module and slot.
        pairs: Vec<HeldPairState>,
    }
}
impl Port for HeldPortalPairsPort {
    /// Version 2 (2026-11-26): each slot's `outcome` replaces `open`, so
    /// a refusal, a pending join and a fizzle are told apart.
    const KEY: PortKey = PortKey::new("ambition.portal.held_pairs", 2);
    const ROLE: PortRole = PortRole::Observation;
    type Value = HeldPairs;
    fn encode(v: &HeldPairs, out: &mut Vec<u8>) {
        ambition_extension_sdk::structure::put(v, out)
    }
    fn decode(r: &mut WireReader<'_>) -> Result<HeldPairs, WireError> {
        ambition_extension_sdk::structure::read(r)
    }
}
ambition_extension_sdk::script_port!(HeldPortalPairsPort);

fn put_end(out: &mut Vec<u8>, end: &PortalEnd) {
    wire::put_vec2(out, end.at);
    wire::put_vec2(out, end.normal);
    wire::put_f32(out, end.half_length);
    wire::put_opt(out, end.half_thickness, wire::put_f32);
}

fn end(r: &mut WireReader<'_>) -> Result<PortalEnd, WireError> {
    Ok(PortalEnd {
        at: r.vec2()?,
        normal: r.vec2()?,
        half_length: r.f32()?,
        half_thickness: r.opt(WireReader::f32)?,
    })
}

impl Port for HeldPortalPairPort {
    /// Version 2 (2026-10-20): an end's `half_thickness`, and
    /// `closes_on_use`.
    const KEY: PortKey = PortKey::new("ambition.portal.held_pair", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = HeldPortalPair;

    fn encode(v: &HeldPortalPair, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        put_end(out, &v.ends[0]);
        put_end(out, &v.ends[1]);
        wire::put_u8(out, v.looks[0]);
        wire::put_u8(out, v.looks[1]);
        wire::put_bool(out, v.closes_on_use);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HeldPortalPair, WireError> {
        Ok(HeldPortalPair {
            slot: r.u32()?,
            ends: [end(r)?, end(r)?],
            looks: [r.u8()?, r.u8()?],
            closes_on_use: r.bool()?,
        })
    }
}
ambition_extension_sdk::script_port!(HeldPortalPairPort);

/// A line of a portal's light that a module asks for: a look, and no way
/// through.
#[derive(Clone, Debug, PartialEq)]
pub struct LightLine {
    /// The module's name for the line. Slots are per entry.
    pub slot: u32,
    /// Its middle.
    pub at: [f32; 2],
    /// The side of the line that the room is on.
    pub normal: [f32; 2],
    /// Its length.
    pub length: f32,
    /// The authored channel index whose colour it has.
    pub look: u8,
    /// The limb whose cut the line is on, when it is on one (v2).
    pub on_limb: Option<LineOnLimb>,
}

/// The cut of a limb that a line of light is on. The presentation puts the
/// line on the cut that the limb's picture publishes (its seam): at the
/// middle of the cut, along it, and over the limb. The line is never shorter
/// than the limb is wide there; the line's own `length` is how far it is
/// open. `at` and `normal` are where the line is while no seam is published,
/// and the presentation eases between the two.
#[derive(Clone, Debug, PartialEq)]
pub struct LineOnLimb {
    /// The limb's slot (`hand_left`).
    pub limb: String,
    /// The cut is on the limb's own body (the end of an arm that came out of
    /// a portal), not on the body it is a limb of.
    pub on_its_body: bool,
}

/// The request port marker for a line of light. See its port card at the
/// top of this crate.
pub struct LightLinePort;

impl Port for LightLinePort {
    const KEY: PortKey = PortKey::new("ambition.portal.light_line", 2);
    const ROLE: PortRole = PortRole::Request;
    type Value = LightLine;

    fn encode(v: &LightLine, out: &mut Vec<u8>) {
        wire::put_u32(out, v.slot);
        wire::put_vec2(out, v.at);
        wire::put_vec2(out, v.normal);
        wire::put_f32(out, v.length);
        wire::put_u8(out, v.look);
        wire::put_opt(out, v.on_limb.as_ref(), |out, on| {
            wire::put_str(out, &on.limb);
            wire::put_bool(out, on.on_its_body);
        });
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LightLine, WireError> {
        Ok(LightLine {
            slot: r.u32()?,
            at: r.vec2()?,
            normal: r.vec2()?,
            length: r.f32()?,
            look: r.u8()?,
            on_limb: r.opt(|r| Ok(LineOnLimb { limb: r.str()?.to_owned(), on_its_body: r.bool()? }))?,
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
                PortalEnd { at: [10.0, -20.0], normal: [0.0, -1.0], half_length: 44.0, half_thickness: None },
                PortalEnd { at: [300.0, 5.5], normal: [0.0, 1.0], half_length: 44.0, half_thickness: Some(6.0) },
            ],
            looks: [24, 25],
            closes_on_use: true,
        };
        let mut bytes = Vec::new();
        HeldPortalPairPort::encode(&pair, &mut bytes);
        let mut reader = WireReader::new(&bytes);
        assert_eq!(HeldPortalPairPort::decode(&mut reader).unwrap(), pair);
        assert!(reader.finish().is_ok());
    }

    #[test]
    fn a_light_line_comes_back_from_the_wire_equal() {
        let on_limb = LineOnLimb { limb: "hand_left".into(), on_its_body: true };
        for on_limb in [None, Some(on_limb)] {
            let line = LightLine { slot: 2, at: [-4.0, 96.5], normal: [1.0, 0.0], length: 130.0, look: 25, on_limb };
            let mut bytes = Vec::new();
            LightLinePort::encode(&line, &mut bytes);
            let mut reader = WireReader::new(&bytes);
            assert_eq!(LightLinePort::decode(&mut reader).unwrap(), line);
            assert!(reader.finish().is_ok());
        }
    }
}
