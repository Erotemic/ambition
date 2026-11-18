//! Contextual Blink: a press of the Blink slot puts the body behind the
//! nearest foe in reach, on the foe's footing and facing it. With no foe in
//! reach it is a short blink along the aim, so the slot is never dead.
//!
//! The technique is the Blink slot of a body that wears a `contextual_blink`
//! row: the inventory's Contextual Blink ability (`items.ron`). Level 1
//! (`contextual_blink`) reaches a near foe; level 2 (`contextual_blink_2`)
//! reaches one further away.
//!
//! The move is a transit asked in `technique_execution`, which the body takes
//! in the next tick's carry (`ambition_abilities::pending_transit`): one tick
//! later, where every reader of the travelled path sees it. The arrival burst
//! is asked on that next tick, at the body, so it is where the body arrived,
//! walls included.
//!
//! The module cannot see the walls. A wall right behind the foe stops the
//! body short of the far side (`Destination::Along`), never inside the wall.

use ambition_combat_port::{
    BodySound, BodySoundPort, Destination, Effect, EffectPort, Foe, FoesPort, Place, SetVelocity, SetVelocityPort,
    TechniqueUse, TechniqueUsePort, Transit, TransitPort, Wielder,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The technique id each level's row puts on the Blink slot, and how far its
/// foe may be (centre to centre).
pub const LEVELS: [(&str, f32); 2] = [("contextual_blink", 170.0), ("contextual_blink_2", 320.0)];
/// With no foe in reach: how far along the aim the body blinks, walls
/// permitting.
pub const FALLBACK_DISTANCE: f32 = 150.0;
/// The gap left between the body's box and the foe's.
pub const CLEARANCE: f32 = 6.0;
/// A deliberate move, not a stream of moves.
pub const COOLDOWN_S: f32 = 0.45;
const SFX: &str = "player.blink";
const EFFECT: &str = "classic_burst";

record! {
    /// What the blink keeps between ticks.
    pub struct Blinked = SchemaKey::new(crate::PROVIDER, "contextual_blink.state", 1);
    /// Seconds until the slot blinks again.
    1 cooldown_s: f32,
    /// A blink was asked last tick: this tick the body has arrived.
    2 arriving: bool,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "contextual_blink"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Blinked::schema()],
        entries: [("blink", 0usize, blink_near as fn(&mut Invocation<'_>) -> Result<(), Fault>), ("blink_far", 1, blink_far)]
            .into_iter()
            .map(|(key, level, run)| EntryDescriptor {
                key: key.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: TechniqueUsePort::KEY, selector: LEVELS[level].0.into() },
                reads: vec![FoesPort::KEY],
                writes: vec![Blinked::KEY],
                requests: vec![TransitPort::KEY, SetVelocityPort::KEY, BodySoundPort::KEY, EffectPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 4 },
                // An idle tick must run: the cooldown runs down, and the tick
                // after a blink shows the arrival.
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(run),
            })
            .collect(),
    }
}

/// Where a press takes the body: behind the nearest foe within `reach`, or
/// [`FALLBACK_DISTANCE`] along the aim. `None` when there is no foe in reach
/// and no aim.
///
/// "Behind" is along the body's side axis, past the foe from where the body
/// stands: its box clear of the foe's by [`CLEARANCE`], its feet level with
/// the foe's, and facing the foe. A foe straight above or below is passed on
/// the side the body faces.
pub fn destination_of(user: &Wielder, foes: &[Foe], reach: f32) -> Option<Transit> {
    let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
    let offset = |foe: &Foe| [foe.position[0] - user.position[0], foe.position[1] - user.position[1]];
    // The nearest; the first of equals (the foes come in identity order).
    let nearest = foes
        .iter()
        .map(|foe| (foe, dot(offset(foe), offset(foe))))
        .filter(|(_, d2)| *d2 <= reach * reach)
        .fold(None::<(&Foe, f32)>, |best, (foe, d2)| match best {
            Some((_, best_d2)) if best_d2 <= d2 => best,
            _ => Some((foe, d2)),
        });
    let Some((foe, _)) = nearest else {
        let direction = crate::wielded::unit_or_zero(user.to_world(user.aim_local));
        return (direction != [0.0, 0.0])
            .then_some(Transit { to: Destination::Along { direction, distance: FALLBACK_DISTANCE }, facing: None });
    };
    let (side, down) = (user.frame_side, user.frame_down);
    let local = [dot(offset(foe), side), dot(offset(foe), down)];
    let pass = if local[0].abs() > 1e-3 { local[0].signum() } else if user.facing < 0.0 { -1.0 } else { 1.0 };
    // The foe's box is in the world; its halves along the body's axes.
    let foe_half = |axis: [f32; 2]| 0.5 * (axis[0].abs() * foe.size[0] + axis[1].abs() * foe.size[1]);
    let to_local = [
        local[0] + pass * (foe_half(side) + 0.5 * user.size[0] + CLEARANCE),
        local[1] + foe_half(down) - 0.5 * user.size[1],
    ];
    let travel = user.to_world(to_local);
    let distance = dot(travel, travel).sqrt();
    let direction = crate::wielded::unit_or_zero(travel);
    (direction != [0.0, 0.0])
        .then_some(Transit { to: Destination::Along { direction, distance }, facing: Some(-pass) })
}

/// What one tick of the blink asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlinkTick {
    pub transit: Option<Transit>,
    /// The body has arrived: show it there.
    pub arrived: bool,
}

/// One tick of the blink for a body that uses it as `used`, `dt` seconds
/// after the last, with `foes` around it. `s` is the state; it is advanced.
pub fn blink_tick_of(s: &mut Blinked, used: &TechniqueUse, foes: &[Foe], reach: f32, dt: f32) -> BlinkTick {
    let mut out = BlinkTick { arrived: s.arriving, ..BlinkTick::default() };
    s.arriving = false;
    s.cooldown_s = (s.cooldown_s - dt).max(0.0);
    if !used.user.pressed || s.cooldown_s > 0.0 {
        return out;
    }
    out.transit = destination_of(&used.user, foes, reach);
    if out.transit.is_some() {
        s.cooldown_s = COOLDOWN_S;
        s.arriving = true;
    }
    out
}

fn blink_near(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    perform(inv, LEVELS[0].1)
}

fn blink_far(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    perform(inv, LEVELS[1].1)
}

fn perform(inv: &mut Invocation<'_>, reach: f32) -> Result<(), Fault> {
    let used: TechniqueUse = inv.trigger::<TechniqueUsePort>()?.clone();
    let foes = if used.user.pressed { inv.observe::<FoesPort>()?.foes.clone() } else { Vec::new() };
    let mut s = Blinked::load(inv)?;
    let tick = blink_tick_of(&mut s, &used, &foes, reach, inv.dt());
    s.store(inv)?;
    if tick.arrived {
        inv.submit::<EffectPort>(Effect { at: Place::Body, fx: EFFECT.into(), scale: 0.5, angle: 0.0 })?;
    }
    if let Some(transit) = tick.transit {
        inv.submit::<TransitPort>(transit)?;
        // A blink takes the body's motion: it arrives still.
        inv.submit::<SetVelocityPort>(SetVelocity { velocity: [0.0, 0.0] })?;
        inv.submit::<BodySoundPort>(BodySound { cue: SFX.into(), at: Place::Body })?;
        inv.submit::<EffectPort>(Effect { at: Place::Body, fx: EFFECT.into(), scale: 0.35, angle: 0.0 })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A grounded robot-sized body at the origin, facing +x, under the
    /// ordinary gravity (+Y down).
    fn user(pressed: bool, aim: [f32; 2]) -> Wielder {
        Wielder {
            pressed,
            driven: true,
            position: [0.0, 0.0],
            size: [28.0, 46.0],
            facing: 1.0,
            frame_side: [1.0, 0.0],
            frame_down: [0.0, 1.0],
            aim_local: aim,
            mana: Some(100.0),
            names_spawns: true,
            cooldown_ready: true,
            swept: true,
        }
    }

    fn foe(x: f32, y: f32, size: [f32; 2]) -> Foe {
        Foe { position: [x, y], size }
    }

    /// Where a transit along a line lands, with no wall.
    fn landing(user: &Wielder, transit: &Transit) -> [f32; 2] {
        match transit.to {
            Destination::Along { direction, distance } => {
                [user.position[0] + direction[0] * distance, user.position[1] + direction[1] * distance]
            }
            Destination::To(at) => at,
        }
    }

    fn pressed() -> TechniqueUse {
        TechniqueUse { held: true, released: false, user: user(true, [1.0, 0.0]) }
    }

    /// The body lands past the foe, clear of its box, feet level with its
    /// feet, and turns to face it: from either side.
    #[test]
    fn a_press_lands_behind_the_nearest_foe_facing_it() {
        let me = user(true, [0.0, 0.0]);
        // A taller foe 100 px ahead, standing on the same floor (feet at 23).
        let ahead = foe(100.0, -7.0, [40.0, 60.0]);
        let transit = destination_of(&me, &[foe(-160.0, 0.0, [28.0, 46.0]), ahead.clone()], 170.0).unwrap();
        let at = landing(&me, &transit);
        assert!((at[0] - (100.0 + 20.0 + 14.0 + CLEARANCE)).abs() < 1e-3, "past the foe's far edge: {at:?}");
        // The foe's feet are at -7 + 30 = 23; the body's centre 23 above them.
        assert!(at[1].abs() < 1e-3, "on the foe's footing: {at:?}");
        assert_eq!(transit.facing, Some(-1.0), "facing back at the foe");

        // From the other side, the other way.
        let behind = destination_of(&me, &[foe(-90.0, 0.0, [28.0, 46.0])], 170.0).unwrap();
        assert!((landing(&me, &behind)[0] - (-90.0 - 14.0 - 14.0 - CLEARANCE)).abs() < 1e-3);
        assert_eq!(behind.facing, Some(1.0));
    }

    /// The nearer of two foes is the one; a tie keeps the first.
    #[test]
    fn the_nearest_foe_is_the_one_and_a_tie_keeps_the_first() {
        let me = user(true, [0.0, 0.0]);
        let size = [28.0, 46.0];
        let near = destination_of(&me, &[foe(150.0, 0.0, size), foe(-60.0, 0.0, size)], 170.0).unwrap();
        assert!(landing(&me, &near)[0] < -60.0, "the nearer foe, behind: {near:?}");
        let tie = destination_of(&me, &[foe(80.0, 0.0, size), foe(-80.0, 0.0, size)], 170.0).unwrap();
        assert!(landing(&me, &tie)[0] > 80.0, "a tie keeps the first: {tie:?}");
    }

    /// Level 1 does not reach a foe level 2 reaches; with no foe in reach the
    /// press blinks along the aim, and an aimless press with no foe does
    /// nothing.
    #[test]
    fn reach_decides_and_no_foe_blinks_along_the_aim() {
        let far = [foe(260.0, 0.0, [28.0, 46.0])];
        let aimed = user(true, [0.0, -1.0]);
        let near = destination_of(&aimed, &far, LEVELS[0].1).unwrap();
        assert_eq!(near, Transit { to: Destination::Along { direction: [0.0, -1.0], distance: FALLBACK_DISTANCE }, facing: None });
        let reached = destination_of(&aimed, &far, LEVELS[1].1).unwrap();
        assert!(landing(&aimed, &reached)[0] > 260.0, "level 2 reaches it: {reached:?}");
        assert_eq!(destination_of(&user(true, [0.0, 0.0]), &far, LEVELS[0].1), None);
    }

    /// Under gravity turned to the wall (+X down), "behind" is along the
    /// body's own side axis, and the feet meet the foe's on the wall.
    #[test]
    fn behind_is_along_the_bodys_own_frame() {
        let mut me = user(true, [0.0, 0.0]);
        // Side is world +Y, down is world +X.
        me.frame_side = [0.0, 1.0];
        me.frame_down = [1.0, 0.0];
        // The foe's box in the world: 46 wide along X (its height), 28 along Y.
        let transit = destination_of(&me, &[foe(0.0, 80.0, [46.0, 28.0])], 170.0).unwrap();
        let at = landing(&me, &transit);
        assert!((at[1] - (80.0 + 14.0 + 14.0 + CLEARANCE)).abs() < 1e-3, "past it along the side axis: {at:?}");
        assert!(at[0].abs() < 1e-3, "on the same footing: {at:?}");
    }

    /// The cooldown holds a second press, runs down on idle ticks, and the
    /// tick after a blink is the arrival.
    #[test]
    fn the_cooldown_holds_and_the_next_tick_is_the_arrival() {
        let foes = [foe(80.0, 0.0, [28.0, 46.0])];
        let dt = 1.0 / 60.0;
        let mut s = Blinked::default();
        let first = blink_tick_of(&mut s, &pressed(), &foes, 170.0, dt);
        assert!(first.transit.is_some() && !first.arrived);
        let idle = TechniqueUse { held: false, released: false, user: user(false, [0.0, 0.0]) };
        let next = blink_tick_of(&mut s, &idle, &foes, 170.0, dt);
        assert_eq!(next, BlinkTick { transit: None, arrived: true });
        let again = blink_tick_of(&mut s, &pressed(), &foes, 170.0, dt);
        assert_eq!(again.transit, None, "a press while the cooldown runs does not blink");
        for _ in 0..30 {
            blink_tick_of(&mut s, &idle, &foes, 170.0, dt);
        }
        assert!(blink_tick_of(&mut s, &pressed(), &foes, 170.0, dt).transit.is_some(), "the cooldown ran down");
    }
}
