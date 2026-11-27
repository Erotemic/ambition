//! The tether reel: she throws a line at a ledge and it pulls her to it.
//!
//! When a move authors `smash.tether_pull`, the line goes where she faces,
//! and bites the first ledge ahead within the move's reach
//! (`ambition.body.ledge_ahead`). A tether is an aerial move: on the ground
//! the same fiction is her grab, and a grounded line would be a free
//! horizontal dash (`ambition.body.on_ground`). A line that bites nothing is
//! not an error: she keeps falling, which is the punish.
//!
//! This module catches no ledge. It holds the status [`TETHERED`] and its
//! reel in a record on her, and each tick of the status pulls her toward the
//! anchor (`ambition.motion.velocity`), never past it, until the ledge
//! authority would catch her where she is, or she is on the anchor, and then
//! it stops her dead and lets go: the authority catches a body that falls,
//! with its own cooldown and eligibility, and from zero gravity crosses its
//! snap speed in about two frames. A reel that runs out of time lets go
//! without touching her momentum, or it would delete her remaining
//! recovery. While it holds, she shows the line (`ambition.feedback.body_line`).
//!
//! A line with a `winch` SWINGS (the grapple's rope, `ambition.motion.hang_on_rope`):
//! a ledge bitten above her hangs her on a rope that takes itself in, so she
//! swings up under it with the momentum she had, and once the rope is as
//! short as a rope goes the line lets go (`ambition.motion.let_go_of_rope`)
//! and reels the rest of the way as above. A ledge level with her or below
//! is reeled straight.

use ambition_combat_port::{
    BodyLine, BodyLinePort, FighterSpecial, FighterSpecialPort, HangOnRope, HangOnRopePort, HoldStatus,
    HoldStatusPort, LedgeAheadPort, LetGoOfRope, LetGoOfRopePort, OnGroundPort, SetVelocity, SetVelocityPort,
    StatusTickPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a tether. The Smash crate installs this
/// module for its catalog's key (`install_module_technique`).
pub const TETHER_PULL: &str = "smash.tether_pull";

/// The status a fighter holds while a line reels her in.
pub const TETHERED: &str = "smash.tethered";

/// The entry that reels a tethered fighter.
pub const REEL: &str = "smash.tether_tick";

/// How close to the anchor counts as on it, in world px.
const ARRIVED_PX: f32 = 0.5;

/// The farthest from straight above her a bite may be and still swing her
/// (radians): flatter bites are reeled straight.
pub const SWING_CAP: f32 = 1.2;

/// How short the swinging rope gets before the line lets go and reels the
/// rest: a little over the shortest a held rope goes (the kernel's 40 px), so
/// the hand-off is not waiting on a rope that cannot shorten.
pub const SWING_HANDOFF_PX: f32 = 48.0;

record! {
    /// A line that bit, on the fighter it reels.
    pub struct Reel = SchemaKey::new(crate::PROVIDER, "tether.reel", 2);
    /// World seconds before the reel gives up.
    1 remaining_s: f32,
    /// How fast it pulls, world px per second.
    2 speed: f32,
    /// Where the line bit: where she will hang. Latched, not asked again: a
    /// ledge does not move, and a line that re-aimed would not be a line.
    3 anchor: [f32; 2],
    /// She is on the swinging rope (a winched line), not yet reeled straight.
    4 swinging: bool,
}

pub fn module() -> ModuleDescriptor {
    let entry = |key: &'static str, trigger, selector: &'static str, reads, run| EntryDescriptor {
        replies: Vec::new(),
        key: key.into(),
        phase: TECHNIQUE_EXECUTION,
        trigger: TriggerBinding { port: trigger, selector: selector.into() },
        reads,
        writes: vec![Reel::KEY],
        requests: vec![
            SetVelocityPort::KEY,
            HoldStatusPort::KEY,
            BodyLinePort::KEY,
            HangOnRopePort::KEY,
            LetGoOfRopePort::KEY,
        ],
        after: Vec::new(),
        limits: Limits { max_requests: 4 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "tether"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Reel::schema()],
        entries: vec![
            entry(
                TETHER_PULL,
                FighterSpecialPort::KEY,
                TETHER_PULL,
                vec![LedgeAheadPort::KEY, OnGroundPort::KEY],
                throw,
            ),
            entry(REEL, StatusTickPort::KEY, TETHERED, vec![LedgeAheadPort::KEY], reel),
        ],
    }
}

/// The reel a throw starts, or none: a grounded fighter throws no line, and
/// a line bites only the first ledge ahead within the move's reach. Every
/// number is the move's, which admission checked, so a missing one is a
/// fault.
pub fn bite(special: &FighterSpecial, on_ground: bool, ahead: Option<([f32; 2], f32)>) -> Result<Option<Reel>, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let reach = number("reach")?;
    if on_ground {
        return Ok(None);
    }
    let Some((anchor, _)) = ahead.filter(|(_, distance)| *distance <= reach) else {
        return Ok(None);
    };
    Ok(Some(Reel { remaining_s: number("timeout_s")?, speed: number("speed")?, anchor, swinging: false }))
}

/// The rope a winched line hangs her on when it bit a ledge above her, or
/// none (no winch, or a bite too flat to swing from).
pub fn swing_rope(special: &FighterSpecial, at: [f32; 2], anchor: [f32; 2]) -> Option<HangOnRope> {
    let winch = special.number("winch").filter(|winch| *winch > 0.0)?;
    let reach = special.number("reach")?;
    let to = [anchor[0] - at[0], anchor[1] - at[1]];
    // World +Y is down: above her is negative y.
    let from_up = to[0].abs().atan2(-to[1]);
    (to[1] < 0.0 && from_up <= SWING_CAP).then_some(HangOnRope {
        anchor,
        // Past the cap, so the swing can carry her round under the ledge.
        max_angle: 1.75,
        swing_accel: 6.0,
        // The winch takes the rope in; the stick does not reel it.
        reel_speed: 0.0,
        min_length: 0.0,
        max_length: reach + 1.0,
        release_hop: 0.0,
        winch_speed: winch,
    })
}

/// What one tick of a reel does.
#[derive(Debug, PartialEq)]
pub enum Pull {
    /// She moves at this velocity, and the reel goes on.
    Toward { velocity: [f32; 2], reel: Reel },
    /// She is where the authority catches her: stopped dead, let go.
    Arrived,
    /// Out of time: let go, her momentum untouched.
    GaveUp,
}

/// A tick of `reel` for a fighter at `at`, where the ledge authority would
/// or would not catch her (`caught`), over the world's step.
pub fn pull(reel: &Reel, at: [f32; 2], caught: bool, dt: f32) -> Pull {
    // The budget is tested before it is spent, so an N-tick reel pulls N
    // times: the move's reach is promised in exactly those ticks.
    if reel.remaining_s <= 0.0 {
        return Pull::GaveUp;
    }
    let to = [reel.anchor[0] - at[0], reel.anchor[1] - at[1]];
    let distance = (to[0] * to[0] + to[1] * to[1]).sqrt();
    // The authority's own answer, not the anchor: the anchor overlaps the
    // wall a little, so the body stops about a pixel short of it.
    if caught || distance <= ARRIVED_PX {
        return Pull::Arrived;
    }
    // The last step is shortened so she lands on the anchor, not past it.
    let speed = if dt > 0.0 { reel.speed.min(distance / dt) } else { reel.speed };
    Pull::Toward {
        velocity: [to[0] / distance * speed, to[1] / distance * speed],
        reel: Reel { remaining_s: reel.remaining_s - dt, ..reel.clone() },
    }
}

/// What one tick of a swinging line does.
#[derive(Debug, PartialEq)]
pub enum Swing {
    /// She swings on; the line holds.
    Holds(Reel),
    /// The rope is short: let go and reel the rest straight.
    HandOff(Reel),
    /// She is where the authority catches her: let go, stopped dead.
    Arrived,
    /// Out of time: let go with the swing's speed.
    GaveUp,
}

/// A tick of a swinging `reel` for a fighter at `at`.
pub fn swing(reel: &Reel, at: [f32; 2], caught: bool, dt: f32) -> Swing {
    if caught {
        return Swing::Arrived;
    }
    if reel.remaining_s <= 0.0 {
        return Swing::GaveUp;
    }
    let left = Reel { remaining_s: reel.remaining_s - dt, ..reel.clone() };
    let to = [reel.anchor[0] - at[0], reel.anchor[1] - at[1]];
    if (to[0] * to[0] + to[1] * to[1]).sqrt() <= SWING_HANDOFF_PX {
        return Swing::HandOff(Reel { swinging: false, ..left });
    }
    Swing::Holds(left)
}

/// Carry out a tick of a reel: pull and show the line, or stop and let go.
fn carry_out(inv: &mut Invocation<'_>, reel: &Reel, at: [f32; 2], caught: bool) -> Result<(), Fault> {
    if reel.swinging {
        return match swing(reel, at, caught, inv.dt()) {
            Swing::Holds(reel) => {
                let anchor = reel.anchor;
                reel.store(inv)?;
                inv.submit::<HoldStatusPort>(HoldStatus { kind: TETHERED.into(), held: true })?;
                inv.submit::<BodyLinePort>(BodyLine { anchor: Some(anchor) })
            }
            // Off the rope (lowered before the velocity), then the straight
            // reel's pull from where she is.
            Swing::HandOff(reel) => {
                inv.submit::<LetGoOfRopePort>(LetGoOfRope {})?;
                carry_out(inv, &reel, at, caught)
            }
            Swing::Arrived => {
                inv.submit::<LetGoOfRopePort>(LetGoOfRope {})?;
                end(inv, reel, at, true)
            }
            Swing::GaveUp => {
                inv.submit::<LetGoOfRopePort>(LetGoOfRope {})?;
                end(inv, reel, at, false)
            }
        };
    }
    match pull(reel, at, caught, inv.dt()) {
        Pull::Toward { velocity, reel } => {
            let anchor = reel.anchor;
            reel.store(inv)?;
            inv.submit::<HoldStatusPort>(HoldStatus { kind: TETHERED.into(), held: true })?;
            inv.submit::<SetVelocityPort>(SetVelocity { velocity })?;
            inv.submit::<BodyLinePort>(BodyLine { anchor: Some(anchor) })
        }
        ended => end(inv, reel, at, ended == Pull::Arrived),
    }
}

/// The line lets go: stopped dead where the authority catches her
/// (`arrived`), or with her momentum untouched.
fn end(inv: &mut Invocation<'_>, _reel: &Reel, _at: [f32; 2], arrived: bool) -> Result<(), Fault> {
    Reel::default().store(inv)?;
    if arrived {
        inv.submit::<SetVelocityPort>(SetVelocity { velocity: [0.0, 0.0] })?;
    }
    inv.submit::<HoldStatusPort>(HoldStatus { kind: TETHERED.into(), held: false })?;
    inv.submit::<BodyLinePort>(BodyLine { anchor: None })
}

fn caught(ahead: &ambition_combat_port::LedgeAhead) -> bool {
    ahead.ledge.is_some_and(|ledge| ledge.distance == 0.0)
}

/// Throw the line, and pull on the tick it bites.
fn throw(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let ahead = inv.observe::<LedgeAheadPort>()?.clone();
    let on_ground = *inv.observe::<OnGroundPort>()?;
    let sight = ahead.ledge.map(|ledge| (ledge.anchor, ledge.distance));
    match bite(&special, on_ground, sight)? {
        Some(reel) => match swing_rope(&special, special.position, reel.anchor) {
            // A winched line that bit above her hangs her on its rope; the
            // status ticks swing her from the next tick.
            Some(rope) if !caught(&ahead) => {
                let anchor = reel.anchor;
                Reel { swinging: true, ..reel }.store(inv)?;
                inv.submit::<HangOnRopePort>(rope)?;
                inv.submit::<HoldStatusPort>(HoldStatus { kind: TETHERED.into(), held: true })?;
                inv.submit::<BodyLinePort>(BodyLine { anchor: Some(anchor) })
            }
            _ => carry_out(inv, &reel, special.position, caught(&ahead)),
        },
        None => Ok(()),
    }
}

/// Reel a tethered fighter one tick.
fn reel(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let at = inv.trigger::<StatusTickPort>()?.position;
    let ahead = inv.observe::<LedgeAheadPort>()?.clone();
    let reel = Reel::load(inv)?;
    carry_out(inv, &reel, at, caught(&ahead))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    const DT: f32 = 1.0 / 60.0;

    fn special() -> FighterSpecial {
        let params = [("reach", 150.0), ("speed", 900.0), ("timeout_s", 0.5)]
            .into_iter()
            .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
            .collect();
        FighterSpecial { position: [26.0, 110.0], facing: 1.0, params }
    }

    /// A ledge ahead within reach bites; one past the reach, none at all, and
    /// a throw from the ground do not.
    #[test]
    fn a_line_bites_the_first_ledge_within_reach_in_the_air() {
        let ledge = Some(([87.0, 119.0], 48.0));
        assert_eq!(
            bite(&special(), false, ledge).unwrap(),
            Some(Reel { remaining_s: 0.5, speed: 900.0, anchor: [87.0, 119.0], swinging: false })
        );
        assert_eq!(bite(&special(), false, Some(([300.0, 119.0], 160.0))).unwrap(), None, "a ledge past reach bit");
        assert_eq!(bite(&special(), false, None).unwrap(), None);
        assert_eq!(bite(&special(), true, ledge).unwrap(), None, "the line fired while standing");
    }

    /// A pull goes toward the anchor at the move's speed, and the last one is
    /// shortened onto it; caught or on the anchor, she arrives; out of time,
    /// it gives up.
    #[test]
    fn a_reel_pulls_lands_and_gives_up() {
        let reel = Reel { remaining_s: 2.0 * DT, speed: 900.0, anchor: [100.0, 0.0], swinging: false };
        let Pull::Toward { velocity, reel: left } = pull(&reel, [0.0, 0.0], false, DT) else {
            panic!("a reel with budget did not pull");
        };
        assert_eq!(velocity, [900.0, 0.0]);
        assert!((left.remaining_s - DT).abs() < 1e-7);
        let Pull::Toward { velocity, .. } = pull(&reel, [95.0, 0.0], false, DT) else {
            panic!("a reel 5 px short did not pull");
        };
        assert!((velocity[0] - 5.0 / DT).abs() < 1e-2, "the last pull overshoots: {velocity:?}");
        assert_eq!(pull(&reel, [10.0, 0.0], true, DT), Pull::Arrived, "caught, it kept pulling");
        assert_eq!(pull(&reel, [99.8, 0.0], false, DT), Pull::Arrived, "on the anchor, it kept pulling");
        assert_eq!(pull(&Reel { remaining_s: 0.0, ..reel.clone() }, [0.0, 0.0], false, DT), Pull::GaveUp);
        assert!(
            matches!(pull(&Reel { remaining_s: DT, ..reel }, [0.0, 0.0], false, DT), Pull::Toward { .. }),
            "the last tick of budget did not pull"
        );
    }

    /// A winched line swings her from a bite above her and reels a flat or
    /// low one straight; on the rope it holds, hands off when the rope is
    /// short, arrives when caught, and gives up out of time.
    #[test]
    fn a_winched_line_swings_from_above_and_hands_off_short() {
        let mut winched = special();
        winched.params.push(("winch".to_string(), SpecialParam::Number(400.0)));
        let at = [26.0, 110.0];
        let rope = swing_rope(&winched, at, [86.0, 10.0]).expect("a bite above her swings");
        assert_eq!((rope.winch_speed, rope.reel_speed, rope.anchor), (400.0, 0.0, [86.0, 10.0]));
        assert!(swing_rope(&winched, at, [176.0, 100.0]).is_none(), "a flat bite swung");
        assert!(swing_rope(&winched, at, [86.0, 140.0]).is_none(), "a bite below her swung");
        assert!(swing_rope(&special(), at, [86.0, 10.0]).is_none(), "a line with no winch swung");

        let reel = Reel { remaining_s: 0.5, speed: 900.0, anchor: [100.0, 0.0], swinging: true };
        assert!(matches!(swing(&reel, [0.0, 100.0], false, DT), Swing::Holds(r) if r.swinging));
        assert!(matches!(swing(&reel, [90.0, 30.0], false, DT), Swing::HandOff(r) if !r.swinging));
        assert_eq!(swing(&reel, [0.0, 100.0], true, DT), Swing::Arrived);
        assert_eq!(swing(&Reel { remaining_s: 0.0, ..reel }, [0.0, 100.0], false, DT), Swing::GaveUp);
    }
}
