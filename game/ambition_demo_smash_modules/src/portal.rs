//! The portal recovery: a linked pair of portals, one at the fighter and one
//! above, held open for a while.
//!
//! When a move authors `smash.portal_pair`, its fighter opens a pair
//! (`ambition.portal.held_pair`): the entrance where she is, its opening
//! facing up, and the exit `rise` above, facing down, so she falls into the
//! low one and comes out of the high one. Normals that faced the same way
//! would work in one direction only. The player angles the pair: `tilt` is
//! the base and `aim_tilt` the range the move's aimed stick swings it
//! through (`ambition.fighter.aimed_stick`). Only the horizontal part of the
//! stick counts, since up and down are what the pair already does, and an
//! exit aimed down would be a hole, not a recovery. Holding right puts the
//! exit to the right: the aim is on the screen, not mirrored by facing.
//!
//! The pair stays open while a status on the fighter ([`OPEN`]) asks for it,
//! for the move's `lifetime_s`, and closes after its first use when the move
//! says so. One pair per fighter: a second opening takes the first's place
//! (roster decision #17, which Jon may overrule).

use ambition_combat_port::{
    AimedStickPort, FighterSpecial, FighterSpecialPort, HoldStatus, HoldStatusPort, StatusTickPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_portal_port::{HeldPortalPair, HeldPortalPairPort, PortalEnd};

/// The effect key a move authors for a portal pair. The Smash crate installs
/// this module for its catalog's key (`install_module_technique`).
pub const PORTAL_PAIR: &str = "smash.portal_pair";

/// The status a fighter holds while her pair is open.
pub const OPEN: &str = "smash.portals_open";

/// The entry that keeps a fighter's pair open.
pub const HOLD: &str = "smash.portals_hold";

record! {
    /// A fighter's pair.
    pub struct Pair = SchemaKey::new(crate::PROVIDER, "portal.pair", 1);
    /// Whether it is open.
    1 open: bool,
    /// The slot it is held in. Each opening takes the next, so a pair that
    /// closed on its use is never asked for again.
    2 slot: u32,
    /// The tick it opened: its clock runs from the tick after.
    3 opened_tick: u64,
    /// World seconds before it closes.
    4 remaining_s: f32,
    /// The entrance's centre, and the way it faces (up, tilted).
    5 entrance: [f32; 2],
    6 up: [f32; 2],
    /// How far above the entrance the exit is.
    7 rise: f32,
    8 half_length: f32,
    9 half_thickness: f32,
    /// The entrance's colour; the exit's is its partner.
    10 look: u32,
    11 closes_on_use: bool,
}

pub fn module() -> ModuleDescriptor {
    let key = ModuleKey::new(crate::PROVIDER, "portal");
    let open_path = format!("{key}/{PORTAL_PAIR}");
    let entry = |name: &'static str, trigger, selector: &'static str, reads, after, run| EntryDescriptor {
        key: name.into(),
        phase: TECHNIQUE_EXECUTION,
        trigger: TriggerBinding { port: trigger, selector: selector.into() },
        reads,
        writes: vec![Pair::KEY],
        requests: vec![HeldPortalPairPort::KEY, HoldStatusPort::KEY],
        after,
        limits: Limits { max_requests: 2 },
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(run),
    };
    ModuleDescriptor {
        key,
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Pair::schema()],
        entries: vec![
            entry(PORTAL_PAIR, FighterSpecialPort::KEY, PORTAL_PAIR, vec![AimedStickPort::KEY], Vec::new(), open),
            // After an opening of the same tick, so a fighter who opens a
            // second pair holds the new one and lets the old one go.
            entry(HOLD, StatusTickPort::KEY, OPEN, Vec::new(), vec![open_path.into()], hold),
        ],
    }
}

/// The pair a press opens, after `previous`, aimed by the stick's
/// horizontal part. Every other number is the move's, which admission
/// checked, so a missing one is a fault.
pub fn pair_of(special: &FighterSpecial, aim_x: f32, previous: &Pair, tick: u64) -> Result<Pair, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    // The tilt turns both normals together; turning one would change where
    // the pair sends her, not its angle. Positive leans toward +x. Both
    // angles are optional in the move and absent means none: a pair with no
    // aim range ignores the stick.
    let angle = |path: &str| special.number(path).unwrap_or(0.0);
    let tilt = (angle("tilt_degrees") + aim_x.clamp(-1.0, 1.0) * angle("aim_tilt_degrees")).to_radians();
    Ok(Pair {
        open: true,
        slot: previous.slot.wrapping_add(1),
        opened_tick: tick,
        remaining_s: number("lifetime_s")?,
        entrance: special.position,
        up: [tilt.sin(), -tilt.cos()],
        rise: number("rise")?,
        half_length: number("half_extent[0]")?,
        half_thickness: number("half_extent[1]")?,
        look: number("channel_index")? as u32,
        closes_on_use: special.flag("close_on_transit").ok_or(Fault::Module("close_on_transit".into()))?,
    })
}

/// The held pair `pair` asks for: the entrance facing up, the exit above it
/// facing down.
pub fn held(pair: &Pair) -> HeldPortalPair {
    let [ux, uy] = pair.up;
    let end = |at: [f32; 2], normal: [f32; 2]| PortalEnd {
        at,
        normal,
        half_length: pair.half_length,
        half_thickness: Some(pair.half_thickness),
    };
    let exit = [pair.entrance[0] + ux * pair.rise, pair.entrance[1] + uy * pair.rise];
    let look = pair.look as u8;
    HeldPortalPair {
        slot: pair.slot,
        ends: [end(pair.entrance, [ux, uy]), end(exit, [-ux, -uy])],
        looks: [look, look ^ 1],
        closes_on_use: pair.closes_on_use,
    }
}

/// A tick of an open pair: spent from the tick after it opened; `None` when
/// it closes.
pub fn age(pair: &Pair, tick: u64, dt: f32) -> Option<Pair> {
    if !pair.open {
        return None;
    }
    let remaining_s = if tick > pair.opened_tick { pair.remaining_s - dt } else { pair.remaining_s };
    (remaining_s > 0.0).then(|| Pair { remaining_s, ..pair.clone() })
}

fn status(held: bool) -> HoldStatus {
    HoldStatus { kind: OPEN.into(), held }
}

/// Open a pair, in place of any she has.
fn open(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let aim_x = inv.observe::<AimedStickPort>()?.stick.map_or(0.0, |[x, _]| x);
    let pair = pair_of(&special, aim_x, &Pair::load(inv)?, inv.tick())?;
    pair.store(inv)?;
    inv.submit::<HoldStatusPort>(status(true))?;
    inv.submit::<HeldPortalPairPort>(held(&pair))
}

/// Keep her pair open for another tick, or let it close.
fn hold(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let pair = Pair::load(inv)?;
    match age(&pair, inv.tick(), inv.dt()) {
        Some(left) => {
            left.store(inv)?;
            inv.submit::<HeldPortalPairPort>(held(&left))
        }
        None => {
            Pair { open: false, ..pair }.store(inv)?;
            inv.submit::<HoldStatusPort>(status(false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special(close_on_transit: bool) -> FighterSpecial {
        let params = [
            ("rise", 150.0),
            ("half_extent[0]", 26.0),
            ("half_extent[1]", 6.0),
            ("lifetime_s", 2.5),
            ("tilt_degrees", 0.0),
            ("aim_tilt_degrees", 30.0),
            ("channel_index", 8.0),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("close_on_transit".to_string(), SpecialParam::Flag(close_on_transit))))
        .collect();
        FighterSpecial { position: [100.0, 200.0], facing: -1.0, params }
    }

    /// The entrance is at her facing up and the exit `rise` above facing
    /// down, in the next slot, coloured as a pair; a right stick leans the
    /// pair right by the aim range, whichever way she faces.
    #[test]
    fn a_press_opens_an_entrance_and_an_exit_above_it() {
        let pair = pair_of(&special(false), 0.0, &Pair { slot: 4, ..Pair::default() }, 9).unwrap();
        let asked = held(&pair);
        assert_eq!(asked.slot, 5);
        assert_eq!((asked.ends[0].at, asked.ends[0].normal), ([100.0, 200.0], [0.0, -1.0]));
        assert_eq!((asked.ends[1].at, asked.ends[1].normal), ([100.0, 50.0], [-0.0, 1.0]));
        assert_eq!((asked.ends[0].half_length, asked.ends[0].half_thickness), (26.0, Some(6.0)));
        assert_eq!((asked.looks, asked.closes_on_use), ([8, 9], false));
        let leaned = held(&pair_of(&special(true), 1.0, &Pair::default(), 9).unwrap());
        assert!((leaned.ends[1].at[0] - (100.0 + 150.0 * 30f32.to_radians().sin())).abs() < 1e-3, "{leaned:?}");
        assert!(leaned.closes_on_use);
    }

    /// The clock runs from the tick after the opening, and the pair closes
    /// when it runs out.
    #[test]
    fn a_pair_closes_when_its_clock_runs_out() {
        let pair = Pair { open: true, opened_tick: 9, remaining_s: 0.5, ..Pair::default() };
        assert_eq!(age(&pair, 9, 0.25).map(|p| p.remaining_s), Some(0.5), "the opening tick spent the clock");
        assert_eq!(age(&pair, 10, 0.25).map(|p| p.remaining_s), Some(0.25));
        assert_eq!(age(&Pair { remaining_s: 0.25, ..pair.clone() }, 10, 0.25), None, "a spent pair stayed open");
        assert_eq!(age(&Pair { open: false, ..pair }, 10, 0.25), None);
    }
}
