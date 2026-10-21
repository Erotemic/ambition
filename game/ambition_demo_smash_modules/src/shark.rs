//! The shark ride: the pirate summons a burning flying shark and rides it.
//!
//! When a move authors `smash.summon_ride`, its fighter summons the move's
//! character where she is (`ambition.world.summon_ride`), reserved for her
//! to ride for the move's `seconds`. The mount system boards her, steers the
//! shark by her stick, and ends the ride when its lease runs out; this
//! module adds the genre's half, with a status on each of the two.
//!
//! On the rider ([`RIDING`]): a jump or a dodge means "put me down", a hit
//! that launches her takes her off (a flinch does not), leaving play takes
//! her off (or her respawn would snap back onto the shark), and a shark that
//! dies puts her down (or its death would keep her riding nothing).
//!
//! On the shark ([`SUMMONED`]), which it holds from birth: once nobody rides
//! it and nobody is coming (its rider left, for any reason, or never got
//! on), it flies off toward the nearer side of its room and is dismissed.
//! "Off screen" is not a fact the simulation may read: peers with different
//! windows would desync.

use ambition_combat_port::{
    BodyPresses, BodyPressesPort, BodyRoomPort, DismissPort, Dismiss, DismountPort, Dismount, DismountWhy, FighterSpecial,
    FighterSpecialPort, HoldCourse, HoldCoursePort, HoldStatus, HoldStatusPort, InPlayPort, Ride, RidePort, Saddle,
    SaddlePort, StatusTickPort, SummonRide, SummonRidePort, TumblingPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, PortKey, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a summoned ride. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const SUMMON_RIDE: &str = "smash.summon_ride";

/// The status a fighter holds from her summon until she is off the shark.
pub const RIDING: &str = "smash.riding";

/// The status a summoned shark holds from birth.
pub const SUMMONED: &str = "smash.summoned_ride";

/// The entry that ticks a riding fighter.
pub const RIDER_TICK: &str = "smash.rider_tick";

/// The entry that ticks a summoned shark.
pub const MOUNT_TICK: &str = "smash.mount_tick";

/// The summoned shark's id: one for every shark this move makes. It is also
/// a save key; a summon declines persisted liveness, which keeps a shared id
/// safe.
pub const SHARK_ID: &str = "smash_ride_shark";

/// How fast a riderless shark leaves, world px per second.
pub const DEPART_SPEED: f32 = 1400.0;

/// How long it flies before it is dismissed. Generous: it only has to
/// outlast the time the body takes to leave view.
pub const DEPART_SECONDS: f32 = 2.0;

record! {
    /// A summoned shark's way out.
    pub struct Departure = SchemaKey::new(crate::PROVIDER, "shark.departure", 1);
    /// Whether it has set off.
    1 leaving: bool,
    /// World seconds before it is dismissed.
    2 remaining_s: f32,
}

pub fn module() -> ModuleDescriptor {
    let entry = |key: &'static str, trigger, selector: &'static str, reads, writes, requests: Vec<PortKey>, run| {
        EntryDescriptor {
            key: key.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: trigger, selector: selector.into() },
            reads,
            writes,
            limits: Limits { max_requests: requests.len() as u32 },
            requests,
            after: Vec::new(),
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(run),
        }
    };
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "shark"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Departure::schema()],
        entries: vec![
            entry(
                SUMMON_RIDE,
                FighterSpecialPort::KEY,
                SUMMON_RIDE,
                Vec::new(),
                Vec::new(),
                vec![SummonRidePort::KEY, HoldStatusPort::KEY],
                summon,
            ),
            entry(
                RIDER_TICK,
                StatusTickPort::KEY,
                RIDING,
                vec![RidePort::KEY, BodyPressesPort::KEY, TumblingPort::KEY, InPlayPort::KEY],
                Vec::new(),
                vec![DismountPort::KEY, HoldStatusPort::KEY],
                rider_tick,
            ),
            entry(
                MOUNT_TICK,
                StatusTickPort::KEY,
                SUMMONED,
                vec![SaddlePort::KEY, BodyRoomPort::KEY],
                vec![Departure::KEY],
                vec![HoldCoursePort::KEY, DismissPort::KEY],
                mount_tick,
            ),
        ],
    }
}

/// The ride a press summons. Every value is the move's, which admission
/// checked, so a missing one is a fault.
pub fn ride_of(special: &FighterSpecial) -> Result<SummonRide, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let character_id = special.text("character_id").ok_or(Fault::Module("character_id".into()))?;
    Ok(SummonRide {
        label: SHARK_ID.into(),
        character_id: character_id.into(),
        half_size: [number("half_extents[0]")?, number("half_extents[1]")?],
        health: Some(number("health")? as u32),
        // Nobody's side is not harmless: a neutral body that kept the
        // character's contact hazard would burn whoever it flew through.
        keeps_contact_damage: false,
        seconds: number("seconds")?,
        board_within: number("board_radius")?,
        board_deadline_s: number("board_deadline_s")?,
        holds: SUMMONED.into(),
    })
}

/// What one tick of a riding fighter does.
#[derive(Debug, PartialEq)]
pub enum RiderTick {
    /// Nothing: she rides, or waits for her shark.
    Stay,
    /// She leaves the saddle.
    Leave(DismountWhy),
    /// She rides nothing and nothing is coming: the status ends.
    Done,
}

/// A tick of a fighter who summoned a ride. A shark that died comes first:
/// whatever else happened, she rides nothing.
pub fn rider_tick_of(ride: Ride, presses: BodyPresses, tumbling: bool, in_play: bool) -> RiderTick {
    match ride {
        Ride::None => RiderTick::Done,
        Ride::Awaiting => RiderTick::Stay,
        Ride::Aboard { mount_alive: false } => RiderTick::Leave(DismountWhy::MountLost),
        Ride::Aboard { .. } if tumbling => RiderTick::Leave(DismountWhy::Launched),
        Ride::Aboard { .. } if !in_play => RiderTick::Leave(DismountWhy::LeftPlay),
        Ride::Aboard { .. } if presses.jump || presses.burst => RiderTick::Leave(DismountWhy::Bailed),
        Ride::Aboard { .. } => RiderTick::Stay,
    }
}

/// What one tick of a summoned shark does.
#[derive(Debug, PartialEq)]
pub enum MountTick {
    /// Somebody rides it or is coming.
    Wait,
    /// It sets off at this velocity.
    Leave { velocity: [f32; 2], departure: Departure },
    /// It flies on.
    Fly(Departure),
    /// Its departure is over: it is dismissed.
    Gone,
}

/// A tick of a summoned shark at `x`, in a room `room_width` wide.
pub fn mount_tick_of(departure: &Departure, saddle: Saddle, x: f32, room_width: Option<f32>, dt: f32) -> MountTick {
    if departure.leaving {
        let remaining_s = departure.remaining_s - dt;
        return if remaining_s <= 0.0 { MountTick::Gone } else { MountTick::Fly(Departure { remaining_s, ..*departure }) };
    }
    if saddle.ridden || saddle.awaiting {
        return MountTick::Wait;
    }
    // Horizontal, toward the nearer side. On the centre line, and in no room,
    // right: a shark with no direction would hover until its clock ran out.
    let side = if room_width.is_some_and(|width| x < width / 2.0) { -1.0 } else { 1.0 };
    MountTick::Leave {
        velocity: [side * DEPART_SPEED, 0.0],
        departure: Departure { leaving: true, remaining_s: DEPART_SECONDS },
    }
}

/// Summon her ride, and hold the status that ends it.
fn summon(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let ride = ride_of(&special)?;
    inv.submit::<SummonRidePort>(ride)?;
    inv.submit::<HoldStatusPort>(HoldStatus { kind: RIDING.into(), held: true })
}

/// Take a riding fighter off when the genre says so.
fn rider_tick(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let ride = *inv.observe::<RidePort>()?;
    let presses = *inv.observe::<BodyPressesPort>()?;
    let tumbling = *inv.observe::<TumblingPort>()?;
    let in_play = *inv.observe::<InPlayPort>()?;
    match rider_tick_of(ride, presses, tumbling, in_play) {
        RiderTick::Stay => Ok(()),
        RiderTick::Leave(why) => inv.submit::<DismountPort>(Dismount { why }),
        RiderTick::Done => inv.submit::<HoldStatusPort>(HoldStatus { kind: RIDING.into(), held: false }),
    }
}

/// Send a riderless shark away, and dismiss it at the end.
fn mount_tick(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let x = inv.trigger::<StatusTickPort>()?.position[0];
    let saddle = *inv.observe::<SaddlePort>()?;
    let room_width = inv.observe::<BodyRoomPort>()?.size.map(|[width, _]| width);
    match mount_tick_of(&Departure::load(inv)?, saddle, x, room_width, inv.dt()) {
        MountTick::Wait => Ok(()),
        MountTick::Leave { velocity, departure } => {
            departure.store(inv)?;
            inv.submit::<HoldCoursePort>(HoldCourse { velocity: Some(velocity) })
        }
        MountTick::Fly(departure) => departure.store(inv),
        MountTick::Gone => inv.submit::<DismissPort>(Dismiss),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special() -> FighterSpecial {
        let params = [
            ("half_extents[0]", 48.0),
            ("half_extents[1]", 22.0),
            ("seconds", 5.0),
            ("health", 40.0),
            ("board_radius", 200.0),
            ("board_deadline_s", 1.0),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("character_id".to_string(), SpecialParam::Text("npc_burning_flying_shark".into()))))
        .collect();
        FighterSpecial { position: [100.0, 200.0], facing: 1.0, params }
    }

    /// The move's character, size, health, lease and boarding go into the
    /// summon, which holds the shark's status from birth and keeps no
    /// contact hazard.
    #[test]
    fn a_press_summons_the_moves_ride() {
        assert_eq!(
            ride_of(&special()).unwrap(),
            SummonRide {
                label: SHARK_ID.into(),
                character_id: "npc_burning_flying_shark".into(),
                half_size: [48.0, 22.0],
                health: Some(40),
                keeps_contact_damage: false,
                seconds: 5.0,
                board_within: 200.0,
                board_deadline_s: 1.0,
                holds: SUMMONED.into(),
            }
        );
    }

    /// Each reason takes her off, a dead shark first; waiting for the shark
    /// keeps the status, and riding nothing ends it.
    #[test]
    fn the_genre_takes_her_off() {
        let aboard = Ride::Aboard { mount_alive: true };
        let none = BodyPresses::default();
        let jump = BodyPresses { jump: true, burst: false };
        let dodge = BodyPresses { jump: false, burst: true };
        assert_eq!(rider_tick_of(aboard, none, false, true), RiderTick::Stay);
        assert_eq!(rider_tick_of(aboard, jump, false, true), RiderTick::Leave(DismountWhy::Bailed));
        assert_eq!(rider_tick_of(aboard, dodge, false, true), RiderTick::Leave(DismountWhy::Bailed));
        assert_eq!(rider_tick_of(aboard, none, true, true), RiderTick::Leave(DismountWhy::Launched));
        assert_eq!(rider_tick_of(aboard, none, false, false), RiderTick::Leave(DismountWhy::LeftPlay));
        assert_eq!(
            rider_tick_of(Ride::Aboard { mount_alive: false }, jump, true, false),
            RiderTick::Leave(DismountWhy::MountLost)
        );
        assert_eq!(rider_tick_of(Ride::Awaiting, jump, true, false), RiderTick::Stay);
        assert_eq!(rider_tick_of(Ride::None, none, false, true), RiderTick::Done);
    }

    /// A shark waits while it is ridden or awaited, then leaves toward the
    /// nearer side for its departure, and is dismissed at the end.
    #[test]
    fn a_riderless_shark_leaves_toward_the_nearer_side() {
        const DT: f32 = 1.0 / 60.0;
        let home = Departure::default();
        let ridden = Saddle { ridden: true, awaiting: false };
        let awaited = Saddle { ridden: false, awaiting: true };
        assert_eq!(mount_tick_of(&home, ridden, 100.0, Some(640.0), DT), MountTick::Wait);
        assert_eq!(mount_tick_of(&home, awaited, 100.0, Some(640.0), DT), MountTick::Wait);
        let set_off = Departure { leaving: true, remaining_s: DEPART_SECONDS };
        let empty = Saddle::default();
        assert_eq!(
            mount_tick_of(&home, empty, 100.0, Some(640.0), DT),
            MountTick::Leave { velocity: [-DEPART_SPEED, 0.0], departure: set_off.clone() }
        );
        assert_eq!(
            mount_tick_of(&home, empty, 320.0, Some(640.0), DT),
            MountTick::Leave { velocity: [DEPART_SPEED, 0.0], departure: set_off.clone() },
            "on the centre line it did not leave right"
        );
        let MountTick::Fly(flying) = mount_tick_of(&set_off, ridden, 0.0, None, DT) else {
            panic!("a departing shark stopped for a rider");
        };
        assert!((flying.remaining_s - (DEPART_SECONDS - DT)).abs() < 1e-6);
        assert_eq!(mount_tick_of(&Departure { remaining_s: DT, ..set_off }, empty, 0.0, None, DT), MountTick::Gone);
    }
}
