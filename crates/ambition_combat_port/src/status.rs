//! A body's statuses: state a module puts on a body and ticks there.
//!
//! A move that leaves something on a body for a while (a slowed clock, a dash
//! that carries, a fuse) keeps that state in its module's records on the
//! body, and holds a status of its own kind there. The world ticks each body
//! that holds a status through the entries bound to that kind
//! ([`StatusTickPort`]), so the module spends its clock and decides the end;
//! the engine keeps no component per move.
//!
//! The verbs a status's module needs that are not elsewhere live here too: a
//! body's clock scale ([`TimeScalePort`]).

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for holding or letting go of a status.
///
/// Port card:
///
/// * **Operation** — with `held`, the body the invocation ran for holds the
///   status `kind` (holding one it holds already changes nothing); without
///   it, the body lets go of it (letting go of one it does not hold changes
///   nothing). A kind is a module's own name (`smash.dilated`). An empty kind
///   is refused.
/// * **Owner** — `ambition_combat::extension::status` (`BodyStatuses`, rollback
///   state).
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `technique_execution`. Requests of one tick apply in the order
///   they were submitted, across invocations, so an entry that lets go of a
///   status and a later entry that holds it again leave it held. A status
///   held this tick is first ticked on the next tick.
pub struct HoldStatusPort;

#[derive(Clone, Debug, PartialEq)]
pub struct HoldStatus {
    pub kind: String,
    pub held: bool,
}

impl Port for HoldStatusPort {
    const KEY: PortKey = PortKey::new("ambition.body.hold_status", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = HoldStatus;

    fn encode(v: &HoldStatus, out: &mut Vec<u8>) {
        wire::put_str(out, &v.kind);
        wire::put_bool(out, v.held);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HoldStatus, WireError> {
        Ok(HoldStatus { kind: r.str()?.to_owned(), held: r.bool()? })
    }
}

/// The trigger port marker for one tick of a status a body holds.
///
/// Port card:
///
/// * **Operation** — a body holds the status whose kind is the selector this
///   tick. The entry's records are the body's, so it reads and spends the
///   state its module left there.
/// * **Owner** — `ambition_combat::extension::status`.
/// * **Scope** — the body. One invocation for EACH body that holds a bound
///   kind, EACH tick with a gameplay step greater than zero (a paused world
///   ticks no status), in the order of the bodies' simulation identity, then
///   their entity.
/// * **Time** — `technique_execution`. An entry that must run before another
///   entry of its module on the same tick (spend an old clock before a new
///   one is set) names it in `after`.
/// * **Read model** — world units, +Y down. `facing` is the sign of the
///   body's facing (`1.0` or `-1.0`). `Invocation::dt` is the world's step,
///   not the body's own clock, so a clock a status spends is not stretched by
///   a slow it put on the body.
pub struct StatusTickPort;

#[derive(Clone, Debug, PartialEq)]
pub struct StatusTick {
    pub position: [f32; 2],
    pub facing: f32,
}

impl Port for StatusTickPort {
    const KEY: PortKey = PortKey::new("ambition.body.status_tick", 1);
    const ROLE: PortRole = PortRole::Trigger;
    type Value = StatusTick;

    fn encode(v: &StatusTick, out: &mut Vec<u8>) {
        wire::put_f32(out, v.position[0]);
        wire::put_f32(out, v.position[1]);
        wire::put_f32(out, v.facing);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<StatusTick, WireError> {
        Ok(StatusTick { position: [r.f32()?, r.f32()?], facing: r.f32()? })
    }
}

/// The request port marker for a body's clock scale.
///
/// Port card:
///
/// * **Operation** — the body the invocation ran for runs at `scale` of the
///   world's clock (its `ProperTimeScale`), until a later request sets
///   another; `1.0` is its own clock. A scale that is not finite or not more
///   than 0 is refused: a body's clock does not stop or run backwards here.
///   Two requests of one tick apply in the order they were submitted.
/// * **Owner** — `ambition_combat::extension::status`.
/// * **Scope** — the body the invocation ran for.
/// * **Time** — `technique_execution`. The body's own step reads it from the
///   next tick.
pub struct TimeScalePort;

#[derive(Clone, Debug, PartialEq)]
pub struct TimeScale {
    pub scale: f32,
}

impl Port for TimeScalePort {
    const KEY: PortKey = PortKey::new("ambition.body.time_scale", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = TimeScale;

    fn encode(v: &TimeScale, out: &mut Vec<u8>) {
        wire::put_f32(out, v.scale);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<TimeScale, WireError> {
        Ok(TimeScale { scale: r.f32()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip<P: Port>(value: P::Value) -> P::Value {
        let mut bytes = Vec::new();
        P::encode(&value, &mut bytes);
        P::decode(&mut WireReader::new(&bytes)).unwrap()
    }

    /// Each status value decodes to what was encoded.
    #[test]
    fn status_values_cross_the_wire_unchanged() {
        let hold = HoldStatus { kind: "smash.dilated".into(), held: true };
        assert_eq!(round_trip::<HoldStatusPort>(hold.clone()), hold);
        let tick = StatusTick { position: [3.0, -4.5], facing: -1.0 };
        assert_eq!(round_trip::<StatusTickPort>(tick.clone()), tick);
        assert_eq!(round_trip::<TimeScalePort>(TimeScale { scale: 0.35 }), TimeScale { scale: 0.35 });
    }
}
