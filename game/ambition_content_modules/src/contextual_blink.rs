//! Contextual Blink: a press of the Blink slot puts the body behind the
//! nearest foe in reach, on the foe's footing and facing it. With no foe in
//! reach it is a short blink along the aim, so the slot is never dead.
//!
//! The technique is the Blink slot of a body that wears a `contextual_blink`
//! row: the inventory's Contextual Blink ability (`items.ron`). Level 1
//! (`contextual_blink`) reaches a near foe; level 2 (`contextual_blink_2`)
//! reaches one further away. Level 3 (`contextual_blink_3`) adds the choice:
//! a second press soon after the first slows the world while the slot is
//! held, the stick picks the foe, and letting go blinks behind it.
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
    BodySound, BodySoundPort, Destination, Effect, EffectPort, Foe, FoesPort, ObserverSlow, ObserverSlowPort, Place,
    SetVelocity, SetVelocityPort, TechniqueUse, TechniqueUsePort, Transit, TransitPort, Wielder,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation,
    Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The technique id each level's row puts on the Blink slot, and how far its
/// foe may be (centre to centre).
pub const LEVELS: [(&str, f32); 3] =
    [("contextual_blink", 170.0), ("contextual_blink_2", 320.0), ("contextual_blink_3", 320.0)];
/// With no foe in reach: how far along the aim the body blinks, walls
/// permitting.
pub const FALLBACK_DISTANCE: f32 = 150.0;
/// The gap left between the body's box and the foe's.
pub const CLEARANCE: f32 = 6.0;
/// A deliberate move, not a stream of moves.
pub const COOLDOWN_S: f32 = 0.45;
/// Level 3: a press this soon after the last one is the double tap that
/// starts the choice.
pub const DOUBLE_TAP_S: f32 = 0.3;
/// Level 3: the longest a choice lasts (world seconds, so slowed), after
/// which the body blinks to the foe it has.
pub const CHOOSE_MAX_S: f32 = 1.0;
/// Level 3: how often the chosen foe's mark flashes (world seconds).
pub const MARK_EVERY_S: f32 = 0.1;
const SFX: &str = "player.blink";
const EFFECT: &str = "classic_burst";

record! {
    /// What the blink keeps between ticks.
    pub struct Blinked = SchemaKey::new(crate::PROVIDER, "contextual_blink.state", 2);
    /// Seconds until the slot blinks again.
    1 cooldown_s: f32,
    /// A blink was asked last tick: this tick the body has arrived.
    2 arriving: bool,
    /// Level 3: seconds left in which a press is the double tap.
    3 tap_window_s: f32,
    /// Level 3: the slot is held to choose a foe, the world slowed.
    4 choosing: bool,
    /// Level 3: seconds the choice has lasted.
    5 chosen_s: f32,
    /// Level 3: seconds until the chosen foe's mark flashes again.
    6 mark_s: f32,
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
        entries: [
            ("blink", 0usize, blink_near as fn(&mut Invocation<'_>) -> Result<(), Fault>),
            ("blink_far", 1, blink_far),
            ("blink_choose", 2, blink_choose),
        ]
        .into_iter()
            .map(|(key, level, run)| EntryDescriptor {
                replies: Vec::new(),
                key: key.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: TechniqueUsePort::KEY, selector: LEVELS[level].0.into() },
                reads: vec![FoesPort::KEY],
                writes: vec![Blinked::KEY],
                // Only level 3 slows the world.
                requests: [TransitPort::KEY, SetVelocityPort::KEY, BodySoundPort::KEY, EffectPort::KEY]
                    .into_iter()
                    .chain((level == 2).then_some(ObserverSlowPort::KEY))
                    .collect(),
                after: Vec::new(),
                // A blink is four; the arrival comes the tick after it, and a
                // choosing tick is the arrival, the slow and the mark.
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

/// The foe the stick picks among those in reach: the one nearest the
/// stick's direction (the first of equals by distance). With the stick at
/// rest, the nearest. `None` when no foe is in reach.
pub fn choice_of<'a>(user: &Wielder, foes: &'a [Foe], reach: f32) -> Option<&'a Foe> {
    let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
    let aim = crate::wielded::unit_or_zero(user.to_world(user.aim_local));
    let in_reach = foes.iter().filter_map(|foe| {
        let offset = [foe.position[0] - user.position[0], foe.position[1] - user.position[1]];
        let d2 = dot(offset, offset);
        (d2 <= reach * reach).then_some((foe, offset, d2))
    });
    // Lower is better: the angle off the stick (as `-cos`), then the distance.
    let score = |offset: [f32; 2], d2: f32| {
        let along = if aim == [0.0, 0.0] { 0.0 } else { -dot(crate::wielded::unit_or_zero(offset), aim) };
        (along, d2)
    };
    in_reach
        .fold(None::<(&Foe, (f32, f32))>, |best, (foe, offset, d2)| {
            let s = score(offset, d2);
            match best {
                Some((_, b)) if b <= s => best,
                _ => Some((foe, s)),
            }
        })
        .map(|(foe, _)| foe)
}

/// What one tick of the blink asks for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlinkTick {
    pub transit: Option<Transit>,
    /// The body has arrived: show it there.
    pub arrived: bool,
    /// Level 3: the world is slowed this tick for the choice.
    pub slow: bool,
    /// Level 3: flash the chosen foe's mark here.
    pub mark: Option<[f32; 2]>,
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

/// One tick of level 3. A press within [`DOUBLE_TAP_S`] of the last one
/// starts the choice (even while the slot cools down); any other press is
/// level 2's blink. While the slot is held the choice slows the world and the
/// stick picks the foe; letting go, or [`CHOOSE_MAX_S`], blinks behind it.
pub fn choose_tick_of(s: &mut Blinked, used: &TechniqueUse, foes: &[Foe], reach: f32, dt: f32) -> BlinkTick {
    if s.choosing {
        let mut out = BlinkTick { arrived: s.arriving, ..BlinkTick::default() };
        s.arriving = false;
        s.cooldown_s = (s.cooldown_s - dt).max(0.0);
        s.chosen_s += dt;
        let chosen = choice_of(&used.user, foes, reach);
        if used.held && !used.released && s.chosen_s < CHOOSE_MAX_S {
            out.slow = true;
            s.mark_s -= dt;
            if s.mark_s <= 0.0 {
                s.mark_s = MARK_EVERY_S;
                out.mark = chosen.map(|foe| foe.position);
            }
            return out;
        }
        s.choosing = false;
        out.transit = destination_of(&used.user, chosen.map(std::slice::from_ref).unwrap_or(&[]), reach);
        if out.transit.is_some() {
            s.cooldown_s = COOLDOWN_S;
            s.arriving = true;
        }
        return out;
    }
    let double_tap = used.user.pressed && s.tap_window_s > 0.0;
    s.tap_window_s = (s.tap_window_s - dt).max(0.0);
    if double_tap {
        let out = BlinkTick { arrived: s.arriving, slow: true, ..BlinkTick::default() };
        s.arriving = false;
        s.cooldown_s = (s.cooldown_s - dt).max(0.0);
        s.choosing = true;
        s.chosen_s = 0.0;
        s.mark_s = 0.0;
        s.tap_window_s = 0.0;
        return out;
    }
    if used.user.pressed {
        s.tap_window_s = DOUBLE_TAP_S;
    }
    blink_tick_of(s, used, foes, reach, dt)
}

fn blink_choose(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let used: TechniqueUse = inv.trigger::<TechniqueUsePort>()?.clone();
    let mut s = Blinked::load(inv)?;
    let foes = if used.user.pressed || s.choosing { inv.observe::<FoesPort>()?.foes.clone() } else { Vec::new() };
    let tick = choose_tick_of(&mut s, &used, &foes, LEVELS[2].1, inv.dt());
    s.store(inv)?;
    if tick.slow {
        inv.submit::<ObserverSlowPort>(ObserverSlow {})?;
    }
    if let Some(at) = tick.mark {
        inv.submit::<EffectPort>(Effect { at: Place::World(at), fx: EFFECT.into(), scale: 0.2, angle: 0.0 })?;
    }
    submit_blink(inv, tick)
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
    submit_blink(inv, tick)
}

/// The arrival and the blink a tick asks for.
fn submit_blink(inv: &mut Invocation<'_>, tick: BlinkTick) -> Result<(), Fault> {
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
        assert_eq!(next, BlinkTick { transit: None, arrived: true, ..BlinkTick::default() });
        let again = blink_tick_of(&mut s, &pressed(), &foes, 170.0, dt);
        assert_eq!(again.transit, None, "a press while the cooldown runs does not blink");
        for _ in 0..30 {
            blink_tick_of(&mut s, &idle, &foes, 170.0, dt);
        }
        assert!(blink_tick_of(&mut s, &pressed(), &foes, 170.0, dt).transit.is_some(), "the cooldown ran down");
    }

    /// The stick picks the foe nearest its direction, not the nearest foe;
    /// at rest it picks the nearest; out of reach is no choice.
    #[test]
    fn the_stick_picks_the_foe() {
        let size = [28.0, 46.0];
        let foes = [foe(60.0, 0.0, size), foe(-200.0, 0.0, size), foe(0.0, -150.0, size)];
        let pick = |aim: [f32; 2]| choice_of(&user(true, aim), &foes, 320.0).map(|f| f.position);
        assert_eq!(pick([-1.0, 0.0]), Some([-200.0, 0.0]), "the far foe the stick points at");
        assert_eq!(pick([0.0, -1.0]), Some([0.0, -150.0]));
        assert_eq!(pick([0.0, 0.0]), Some([60.0, 0.0]), "at rest, the nearest");
        assert_eq!(choice_of(&user(true, [-1.0, 0.0]), &foes, 100.0).map(|f| f.position), Some([60.0, 0.0]));
        assert!(choice_of(&user(true, [0.0, 0.0]), &foes, 40.0).is_none());
    }

    /// Level 3: the first press blinks; a second press soon after (in the
    /// cooldown) starts the choice, which slows the world and marks the
    /// stick's foe while held; letting go blinks behind that foe.
    #[test]
    fn a_double_tap_holds_the_world_for_the_choice() {
        let size = [28.0, 46.0];
        let foes = [foe(60.0, 0.0, size), foe(-200.0, 0.0, size)];
        let dt = 1.0 / 60.0;
        let mut s = Blinked::default();
        let tap = |aim: [f32; 2]| TechniqueUse { held: true, released: false, user: user(true, aim) };
        let hold = |aim: [f32; 2]| TechniqueUse { held: true, released: false, user: user(false, aim) };
        let up = |aim: [f32; 2]| TechniqueUse { held: false, released: true, user: user(false, aim) };
        let idle = TechniqueUse { held: false, released: false, user: user(false, [0.0, 0.0]) };

        assert!(choose_tick_of(&mut s, &tap([0.0, 0.0]), &foes, 320.0, dt).transit.is_some(), "the first press blinks");
        choose_tick_of(&mut s, &up([0.0, 0.0]), &foes, 320.0, dt);
        choose_tick_of(&mut s, &idle, &foes, 320.0, dt);
        let second = choose_tick_of(&mut s, &tap([0.0, 0.0]), &foes, 320.0, dt);
        assert!(second.slow && second.transit.is_none(), "the double tap starts the choice: {second:?}");
        let held = choose_tick_of(&mut s, &hold([-1.0, 0.0]), &foes, 320.0, dt);
        assert!(held.slow && held.transit.is_none());
        assert_eq!(held.mark, Some([-200.0, 0.0]), "the stick's foe is marked");
        let quiet = choose_tick_of(&mut s, &hold([-1.0, 0.0]), &foes, 320.0, dt);
        assert!(quiet.slow && quiet.mark.is_none(), "the mark flashes, not every tick");
        let go = choose_tick_of(&mut s, &up([-1.0, 0.0]), &foes, 320.0, dt);
        assert!(!go.slow);
        let me = user(false, [-1.0, 0.0]);
        assert!(landing(&me, &go.transit.unwrap())[0] < -200.0, "behind the chosen foe");
        assert!(choose_tick_of(&mut s, &idle, &foes, 320.0, dt).arrived);
    }

    /// Level 3: a choice held past its longest blinks anyway; a second press
    /// after the window is a plain blink, not a choice.
    #[test]
    fn a_choice_ends_and_a_late_press_is_a_blink() {
        let foes = [foe(60.0, 0.0, [28.0, 46.0])];
        let dt = 1.0 / 60.0;
        let hold = TechniqueUse { held: true, released: false, user: user(false, [0.0, 0.0]) };
        let idle = TechniqueUse { held: false, released: false, user: user(false, [0.0, 0.0]) };
        let mut s = Blinked { tap_window_s: DOUBLE_TAP_S, ..Blinked::default() };
        assert!(choose_tick_of(&mut s, &pressed(), &foes, 320.0, dt).slow);
        let ticks = (0..120).map(|_| choose_tick_of(&mut s, &hold, &foes, 320.0, dt)).position(|t| t.transit.is_some());
        let at = ticks.expect("the choice ended while held");
        assert!((at as f32 + 1.0) * dt >= CHOOSE_MAX_S - dt && (at as f32) * dt < CHOOSE_MAX_S + dt, "at {at}");

        let mut late = Blinked::default();
        choose_tick_of(&mut late, &pressed(), &foes, 320.0, dt);
        for _ in 0..60 {
            choose_tick_of(&mut late, &idle, &foes, 320.0, dt);
        }
        let press = choose_tick_of(&mut late, &pressed(), &foes, 320.0, dt);
        assert!(!press.slow && press.transit.is_some(), "a late press blinks: {press:?}");
    }
}
