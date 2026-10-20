//! A body and the stage's ledges: the facts a move that reaches for a ledge
//! reads, and the line the player sees while it does.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// How far the host looks along a body's facing for [`LedgeAheadPort`]. A
/// module that reaches less far reads [`LedgeSight::distance`].
pub const LEDGE_SEARCH_REACH: f32 = 400.0;

/// How far apart the host's probes sit along the line. A ledge's grab band
/// is 28 px up and 30 px down (`LEDGE_REACH_UP`, `LEDGE_REACH_DOWN`), so a
/// 16 px step cannot step over one.
pub const LEDGE_SAMPLE_PX: f32 = 16.0;

/// The observation port marker for the first ledge ahead of a body.
///
/// Port card:
///
/// * **Operation** — the first ledge the body could hang from, along its
///   facing in its own frame's side axis, from where it is, within
///   [`LEDGE_SEARCH_REACH`]: the host asks the ledge authority's own probe
///   (`probe_ledge_grab_in_frame`, with the body's size and the wall facing
///   back at it) every [`LEDGE_SAMPLE_PX`]. `anchor` is where the body would
///   hang; `distance` is how far along the line the probe that found it sat.
///   A `distance` of `0` is a ledge the authority would catch the body on
///   where it is now.
/// * **Owner** — `ambition_abilities::extension` (the collision world).
/// * **Scope** — the body; the ledges of the live room it is in.
/// * **Time** — `technique_execution`, read when the entry runs. The host
///   probes only for an entry that runs.
/// * **Absence** — never absent for a body: `ledge` is `None` when no probe
///   finds one, or the body has no kinematics or frame.
pub struct LedgeAheadPort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LedgeSight {
    pub anchor: [f32; 2],
    pub distance: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LedgeAhead {
    pub ledge: Option<LedgeSight>,
}

impl Port for LedgeAheadPort {
    const KEY: PortKey = PortKey::new("ambition.body.ledge_ahead", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = LedgeAhead;

    fn encode(v: &LedgeAhead, out: &mut Vec<u8>) {
        wire::put_opt(out, v.ledge, |out, ledge| {
            wire::put_vec2(out, ledge.anchor);
            wire::put_f32(out, ledge.distance);
        });
    }

    fn decode(r: &mut WireReader<'_>) -> Result<LedgeAhead, WireError> {
        Ok(LedgeAhead { ledge: r.opt(|r| Ok(LedgeSight { anchor: r.vec2()?, distance: r.f32()? }))? })
    }
}

/// The observation port marker for whether a body stands on the ground.
///
/// Port card:
///
/// * **Operation** — `true` while the body's ground state says it stands on
///   something, as the movement kernel resolved it this tick.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`.
/// * **Absence** — absent for a body with no ground state.
pub struct OnGroundPort;

impl Port for OnGroundPort {
    const KEY: PortKey = PortKey::new("ambition.body.on_ground", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = bool;

    fn encode(v: &bool, out: &mut Vec<u8>) {
        wire::put_bool(out, *v);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<bool, WireError> {
        r.bool()
    }
}

/// The request port marker for the line a body shows to a point.
///
/// Port card:
///
/// * **Operation** — the body shows a line from itself to `anchor` (world
///   units), or none: presentation draws it as it draws a grab's line
///   (`BodyLineAnchor`). It is not gameplay state: a module that holds a
///   line states it again each tick it holds it.
/// * **Owner** — `ambition_abilities::extension`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`.
pub struct BodyLinePort;

#[derive(Clone, Debug, PartialEq)]
pub struct BodyLine {
    pub anchor: Option<[f32; 2]>,
}

impl Port for BodyLinePort {
    const KEY: PortKey = PortKey::new("ambition.feedback.body_line", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = BodyLine;

    fn encode(v: &BodyLine, out: &mut Vec<u8>) {
        wire::put_opt(out, v.anchor, wire::put_vec2);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyLine, WireError> {
        Ok(BodyLine { anchor: r.opt(WireReader::vec2)? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<P: Port>(value: P::Value) -> P::Value {
        let mut out = Vec::new();
        P::encode(&value, &mut out);
        let mut r = WireReader::new(&out);
        let back = P::decode(&mut r).unwrap();
        r.finish().unwrap();
        back
    }

    /// The ledge ports decode to what was encoded.
    #[test]
    fn ledge_values_survive_the_wire() {
        for ledge in [None, Some(LedgeSight { anchor: [87.0, 119.0], distance: 48.0 })] {
            assert_eq!(round_trip::<LedgeAheadPort>(LedgeAhead { ledge }), LedgeAhead { ledge });
        }
        assert!(round_trip::<OnGroundPort>(true));
        for anchor in [None, Some([3.0, -4.0])] {
            assert_eq!(round_trip::<BodyLinePort>(BodyLine { anchor }), BodyLine { anchor });
        }
    }
}
