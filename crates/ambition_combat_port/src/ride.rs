//! A body that summons something to ride, and the summon itself: the verbs a
//! move that calls a mount needs, and the facts it reads to decide when the
//! ride ends.
//!
//! The mount system owns riding: the saddle, the lease, boarding and leaving
//! (ADR 0020). What ends a ride early, and what the mount does once nobody
//! rides it, is the move's: it holds a status on the rider and another on the
//! mount it summoned ([`SummonRide::holds`]), reads these facts each tick,
//! and asks for a dismount, a course or a dismissal.

use ambition_extension_sdk::wire::{self, WireError, WireReader};
use ambition_extension_sdk::{Port, PortKey, PortRole};

/// The request port marker for a summoned mount that the asker rides.
///
/// Port card:
///
/// * **Operation** — bring the character `character_id` into the world at
///   the asker's position, with a box of `half_size`, on nobody's side (it
///   is hittable, and fights nobody). `health` replaces the character's own
///   (`None` keeps it); `keeps_contact_damage` keeps the character's contact
///   hazard. The mount is reserved for the asker: the asker boards it once
///   within `board_within` of it, and rides it for `seconds`; a mount the
///   asker has not reached in `board_deadline_s` is refused, and nobody rides
///   it. From birth it holds the status `holds` (none when empty), so the
///   module that summoned it ticks it (`ambition.body.status_tick`).
/// * **Owner** — `ambition_abilities::ride`, which lowers it into
///   `EffectRequest` → `Effect::Summon`, the summon executor's one road.
/// * **Scope** — the asker, who must have a simulation identity: the mount's
///   identity descends from it.
/// * **Identity** — `label` is the mount's id, and its save key: a summon
///   declines persisted liveness, so one label serves every mount a move
///   makes. An empty label is refused.
/// * **Time** — `technique_execution`. The executor makes the mount this
///   tick; its status is first ticked on the next.
/// * **Result** — submitted is not applied: a character that does not
///   resolve refuses the summon (logged), and a refused board leaves the
///   mount riderless ([`SaddlePort`]).
pub struct SummonRidePort;

#[derive(Clone, Debug, PartialEq)]
pub struct SummonRide {
    pub label: String,
    pub character_id: String,
    pub half_size: [f32; 2],
    pub health: Option<u32>,
    pub keeps_contact_damage: bool,
    pub seconds: f32,
    pub board_within: f32,
    pub board_deadline_s: f32,
    pub holds: String,
}

impl Port for SummonRidePort {
    const KEY: PortKey = PortKey::new("ambition.world.summon_ride", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = SummonRide;

    fn encode(v: &SummonRide, out: &mut Vec<u8>) {
        wire::put_str(out, &v.label);
        wire::put_str(out, &v.character_id);
        wire::put_vec2(out, v.half_size);
        wire::put_opt(out, v.health, wire::put_u32);
        wire::put_bool(out, v.keeps_contact_damage);
        wire::put_f32(out, v.seconds);
        wire::put_f32(out, v.board_within);
        wire::put_f32(out, v.board_deadline_s);
        wire::put_str(out, &v.holds);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<SummonRide, WireError> {
        Ok(SummonRide {
            label: r.str()?.to_owned(),
            character_id: r.str()?.to_owned(),
            half_size: r.vec2()?,
            health: r.opt(WireReader::u32)?,
            keeps_contact_damage: r.bool()?,
            seconds: r.f32()?,
            board_within: r.f32()?,
            board_deadline_s: r.f32()?,
            holds: r.str()?.to_owned(),
        })
    }
}

/// The observation port marker for what a body rides.
///
/// Port card:
///
/// * **Operation** — `Awaiting` while a mount is reserved for the body and
///   it has not boarded; `Aboard` while it is in a saddle, with whether that
///   mount is alive (a rider stays linked to a mount that died, ADR 0020);
///   `None` otherwise.
/// * **Owner** — `ambition_abilities::ride` (the mount system's state).
/// * **Scope** — the body.
/// * **Time** — `technique_execution`: before this tick's boards, launches
///   and dismounts, so it is what the last tick left.
/// * **Absence** — never absent.
pub struct RidePort;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Ride {
    #[default]
    None,
    Awaiting,
    Aboard {
        mount_alive: bool,
    },
}

impl Port for RidePort {
    const KEY: PortKey = PortKey::new("ambition.body.ride", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Ride;

    fn encode(v: &Ride, out: &mut Vec<u8>) {
        match v {
            Ride::None => wire::put_u8(out, 0),
            Ride::Awaiting => wire::put_u8(out, 1),
            Ride::Aboard { mount_alive } => {
                wire::put_u8(out, 2);
                wire::put_bool(out, *mount_alive);
            }
        }
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Ride, WireError> {
        Ok(match r.u8()? {
            0 => Ride::None,
            1 => Ride::Awaiting,
            2 => Ride::Aboard { mount_alive: r.bool()? },
            tag => return Err(WireError::BadTag(tag)),
        })
    }
}

/// The observation port marker for a mount's saddle.
///
/// Port card:
///
/// * **Operation** — `ridden` while somebody is in the body's saddle;
///   `awaiting` while it is reserved for a rider who has not boarded. A body
///   that is neither has nobody coming: its rider left, or its board was
///   refused.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`, as [`RidePort`].
/// * **Absence** — never absent: a body that is no mount is neither.
pub struct SaddlePort;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Saddle {
    pub ridden: bool,
    pub awaiting: bool,
}

impl Port for SaddlePort {
    const KEY: PortKey = PortKey::new("ambition.body.saddle", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = Saddle;

    fn encode(v: &Saddle, out: &mut Vec<u8>) {
        wire::put_bool(out, v.ridden);
        wire::put_bool(out, v.awaiting);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Saddle, WireError> {
        Ok(Saddle { ridden: r.bool()?, awaiting: r.bool()? })
    }
}

/// The observation port marker for the edges a body's control pressed.
///
/// Port card:
///
/// * **Operation** — whether the body's resolved control pressed jump, and
///   pressed burst (a dodge), this tick: the edge, not the hold.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`.
/// * **Absence** — absent for a body with no control.
pub struct BodyPressesPort;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyPresses {
    pub jump: bool,
    pub burst: bool,
}

impl Port for BodyPressesPort {
    const KEY: PortKey = PortKey::new("ambition.body.presses", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = BodyPresses;

    fn encode(v: &BodyPresses, out: &mut Vec<u8>) {
        wire::put_bool(out, v.jump);
        wire::put_bool(out, v.burst);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyPresses, WireError> {
        Ok(BodyPresses { jump: r.bool()?, burst: r.bool()? })
    }
}

/// The observation port marker for whether a body tumbles.
///
/// Port card:
///
/// * **Operation** — `true` while the body was launched and has no control
///   until it lands (`BodyMotionFacts::tumbling`); a hit that only flinches
///   does not tumble it.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`.
/// * **Absence** — absent for a body with no motion facts.
pub struct TumblingPort;

impl Port for TumblingPort {
    const KEY: PortKey = PortKey::new("ambition.body.tumbling", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = bool;

    fn encode(v: &bool, out: &mut Vec<u8>) {
        wire::put_bool(out, *v);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<bool, WireError> {
        r.bool()
    }
}

/// The observation port marker for the size of a body's room.
///
/// Port card:
///
/// * **Operation** — the size of the live room the body is in, in world
///   units: its playable area is `0..size`.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`.
/// * **Absence** — never absent: `size` is `None` for a body in no live room.
pub struct BodyRoomPort;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BodyRoom {
    pub size: Option<[f32; 2]>,
}

impl Port for BodyRoomPort {
    const KEY: PortKey = PortKey::new("ambition.body.room", 1);
    const ROLE: PortRole = PortRole::Observation;
    type Value = BodyRoom;

    fn encode(v: &BodyRoom, out: &mut Vec<u8>) {
        wire::put_opt(out, v.size, wire::put_vec2);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<BodyRoom, WireError> {
        Ok(BodyRoom { size: r.opt(WireReader::vec2)? })
    }
}

/// Why a rider asks to leave its saddle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DismountWhy {
    /// The rider asked to get off.
    Bailed,
    /// A hit launched the rider.
    Launched,
    /// The rider left play.
    LeftPlay,
    /// The mount died.
    MountLost,
}

/// The request port marker for leaving a saddle.
///
/// Port card:
///
/// * **Operation** — the body leaves the saddle it is in, for `why`: the
///   mount system's dismount (`DismountRequested`), which restores the
///   rider's size, ends its lease and announces the empty saddle. A body in
///   no saddle changes nothing.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the rider.
/// * **Time** — `technique_execution`; the mount system applies it at
///   `DismountRequestsApplied` of the same tick. A rider asked to leave twice
///   in one tick leaves once.
pub struct DismountPort;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dismount {
    pub why: DismountWhy,
}

impl Port for DismountPort {
    const KEY: PortKey = PortKey::new("ambition.mount.dismount", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Dismount;

    fn encode(v: &Dismount, out: &mut Vec<u8>) {
        wire::put_u8(
            out,
            match v.why {
                DismountWhy::Bailed => 0,
                DismountWhy::Launched => 1,
                DismountWhy::LeftPlay => 2,
                DismountWhy::MountLost => 3,
            },
        );
    }

    fn decode(r: &mut WireReader<'_>) -> Result<Dismount, WireError> {
        let why = match r.u8()? {
            0 => DismountWhy::Bailed,
            1 => DismountWhy::Launched,
            2 => DismountWhy::LeftPlay,
            3 => DismountWhy::MountLost,
            tag => return Err(WireError::BadTag(tag)),
        };
        Ok(Dismount { why })
    }
}

/// The request port marker for a course a body holds.
///
/// Port card:
///
/// * **Operation** — the body steers for `velocity` (world units per second,
///   +Y down) in place of what its brain asks, each tick, until a later
///   request lets go (`None`). It is an intent, not a position: the body's
///   own movement reaches the velocity, and walls stop it.
/// * **Owner** — `ambition_abilities::ride` (`HeldCourse`, rollback state).
/// * **Scope** — the body.
/// * **Time** — `technique_execution`; the body steers by it from the next
///   tick's movement. A non-finite velocity is refused.
pub struct HoldCoursePort;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoldCourse {
    pub velocity: Option<[f32; 2]>,
}

impl Port for HoldCoursePort {
    const KEY: PortKey = PortKey::new("ambition.body.hold_course", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = HoldCourse;

    fn encode(v: &HoldCourse, out: &mut Vec<u8>) {
        wire::put_opt(out, v.velocity, wire::put_vec2);
    }

    fn decode(r: &mut WireReader<'_>) -> Result<HoldCourse, WireError> {
        Ok(HoldCourse { velocity: r.opt(WireReader::vec2)? })
    }
}

/// The request port marker for removing a summoned body.
///
/// Port card:
///
/// * **Operation** — the body leaves the world. Only a body the simulation
///   minted (a summon) is removed; any other is refused, and the refusal is
///   logged.
/// * **Owner** — `ambition_abilities::ride`.
/// * **Scope** — the body.
/// * **Time** — `technique_execution`; the body is gone before the next tick.
pub struct DismissPort;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dismiss;

impl Port for DismissPort {
    const KEY: PortKey = PortKey::new("ambition.world.dismiss", 1);
    const ROLE: PortRole = PortRole::Request;
    type Value = Dismiss;

    fn encode(_: &Dismiss, _: &mut Vec<u8>) {}

    fn decode(_: &mut WireReader<'_>) -> Result<Dismiss, WireError> {
        Ok(Dismiss)
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

    /// The ride ports decode to what was encoded.
    #[test]
    fn ride_values_survive_the_wire() {
        let summon = SummonRide {
            label: "smash_ride_shark".into(),
            character_id: "npc_burning_flying_shark".into(),
            half_size: [48.0, 22.0],
            health: Some(40),
            keeps_contact_damage: false,
            seconds: 5.0,
            board_within: 200.0,
            board_deadline_s: 1.0,
            holds: "smash.summoned_ride".into(),
        };
        assert_eq!(round_trip::<SummonRidePort>(summon.clone()), summon);
        for ride in [Ride::None, Ride::Awaiting, Ride::Aboard { mount_alive: false }, Ride::Aboard { mount_alive: true }] {
            assert_eq!(round_trip::<RidePort>(ride), ride);
        }
        let saddle = Saddle { ridden: false, awaiting: true };
        assert_eq!(round_trip::<SaddlePort>(saddle), saddle);
        let presses = BodyPresses { jump: false, burst: true };
        assert_eq!(round_trip::<BodyPressesPort>(presses), presses);
        assert!(round_trip::<TumblingPort>(true));
        for size in [None, Some([640.0, 480.0])] {
            assert_eq!(round_trip::<BodyRoomPort>(BodyRoom { size }), BodyRoom { size });
        }
        for why in [DismountWhy::Bailed, DismountWhy::Launched, DismountWhy::LeftPlay, DismountWhy::MountLost] {
            assert_eq!(round_trip::<DismountPort>(Dismount { why }), Dismount { why });
        }
        for velocity in [None, Some([-1400.0, 0.0])] {
            assert_eq!(round_trip::<HoldCoursePort>(HoldCourse { velocity }), HoldCourse { velocity });
        }
        assert_eq!(round_trip::<DismissPort>(Dismiss), Dismiss);
    }
}
