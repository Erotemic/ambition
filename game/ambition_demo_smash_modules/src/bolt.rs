//! The steered bolt: a thing you fly with the same stick you walk with.
//!
//! When a move authors `smash.steered_bolt`, its fighter puts a module entity
//! of the kind [`BOLT`] at the move's offset, and this module flies it. It
//! leaves forward at the move's speed, and each tick turns toward what its
//! caster's player holds (`ambition.world.spawner_body`), never faster than
//! the move's turn rate: a turn costs distance, and the speed never changes,
//! so the move is about aim. Once the caster is free, one stick does both,
//! walking right also steers the bolt right; that is the move's cost. It
//! draws a trail on the move's interval, or nobody could fly it.
//!
//! What it reaches ends it. A rival takes a hit credited to the caster. The
//! caster, once the bolt has got clear of them, is thrown along the bolt's
//! heading (the Thunder Jacket), so the move is also a recovery; the bolt
//! starts inside its caster, and without the latch every press would be an
//! instant self-launch. A rival beats the caster: a bolt that can connect
//! does the offensive thing. Rivals tie-break by identity
//! (`ambition.world.toucher_credits`), so the choice is the same on every
//! peer. A bolt whose caster has left the match owns its own hit.

use ambition_combat_port::{
    Blast, BlastPort, CreditPort, Effect, EffectPort, EndModuleEntity, EndModuleEntityPort, FighterSpecial,
    FighterSpecialPort, LaunchToucher, LaunchToucherPort, ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort,
    MoveModuleEntity, MoveModuleEntityPort, Place, SpawnModuleEntityPort, SpawnerBody, SpawnerBodyPort, SpecialParam,
    ToucherCreditsPort,
};
use ambition_extension_sdk::{
    phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION},
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a bolt. The Smash crate installs this
/// module for its catalog's key (`install_module_technique`).
pub const STEERED_BOLT: &str = "smash.steered_bolt";

/// The kind of the module entity a bolt in flight is.
pub const BOLT: &str = "smash.bolt";

/// The param the press adds to the move's: which way the caster faced, so
/// the bolt leaves forward.
const FACING: &str = "facing";

/// A stick held more gently than this does not steer.
const STEER_DEADZONE: f32 = 0.2;

record! {
    /// A bolt in flight: what the world's entity does not keep.
    pub struct Flight = SchemaKey::new(crate::PROVIDER, "bolt.flight", 1);
    /// Whether it has flown a tick; before that the rest is unset.
    1 launched: bool,
    /// Where it is going. Its length is the move's speed: the stick turns it.
    2 velocity: [f32; 2],
    /// Seconds to the next trail mark.
    3 trail_in_s: f32,
    /// Whether it has got clear of its caster, which it starts inside.
    4 clear_of_caster: bool,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "bolt"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Flight::schema()],
        entries: vec![
            EntryDescriptor {
                key: STEERED_BOLT.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: STEERED_BOLT.into() },
                reads: Vec::new(),
                writes: Vec::new(),
                requests: vec![SpawnModuleEntityPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(fire),
            },
            EntryDescriptor {
                key: BOLT.into(),
                phase: MODULE_ENTITY_TICK,
                trigger: TriggerBinding { port: ModuleEntityTickPort::KEY, selector: BOLT.into() },
                reads: vec![SpawnerBodyPort::KEY, ToucherCreditsPort::KEY, CreditPort::KEY],
                writes: vec![Flight::KEY],
                requests: vec![
                    MoveModuleEntityPort::KEY,
                    EffectPort::KEY,
                    BlastPort::KEY,
                    LaunchToucherPort::KEY,
                    EndModuleEntityPort::KEY,
                ],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(fly),
            },
        ],
    }
}

/// The bolt a press fires: at the move's offset (body-local, mirrored by
/// facing), as big as its radius, for its lifetime, carrying the move's
/// params and the caster's facing. Every number is the move's, which
/// admission checked (`check_steered_bolt_params`), so a missing one is a
/// fault.
pub fn bolt_of(special: &FighterSpecial) -> Result<ModuleEntitySpawn, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let radius = number("radius")?;
    let mut params = special.params.clone();
    params.push((FACING.into(), SpecialParam::Number(special.facing)));
    Ok(ModuleEntitySpawn {
        kind: BOLT.into(),
        at: [
            special.position[0] + number("offset[0]")? * special.facing,
            special.position[1] + number("offset[1]")?,
        ],
        lifetime_s: number("lifetime_s")?,
        half_extents: [radius, radius],
        params,
    })
}

/// What one tick of a bolt does.
#[derive(Debug, PartialEq)]
pub enum Tick {
    /// It flies on: where it goes, the flight it leaves, and a trail mark
    /// there when one is due.
    Fly { to: [f32; 2], flight: Flight, mark: Option<Effect> },
    /// It reached a rival.
    Hit(Blast),
    /// It came home: its caster, the toucher at this index, leaves at this
    /// velocity.
    Home { toucher: u32, velocity: [f32; 2] },
}

/// What a tick of a bolt does, given its flight so far, the world's step,
/// its caster, who touches it, and its own credit.
pub fn advance(
    tick: &ModuleEntityTick,
    flight: &Flight,
    dt: f32,
    caster: SpawnerBody,
    touchers: &[u64],
    own_credit: u64,
) -> Result<Tick, Fault> {
    let number = |path: &'static str| tick.number(path).ok_or(Fault::Module(path.into()));
    let mut flight = flight.clone();
    if !flight.launched {
        // It leaves forward, and the first trail mark is due at once, so the
        // bolt is seen from its first tick.
        flight = Flight {
            launched: true,
            velocity: [number(FACING)?.signum() * number("speed")?, 0.0],
            trail_in_s: 0.0,
            clear_of_caster: false,
        };
    }
    // A caster that left the match is nobody: credit 0 names no body.
    let is_caster = |credit: u64| caster.credit != 0 && credit == caster.credit;
    // Clearance first, every tick, whatever else is in reach: the touch that
    // clears it uses the same box as the touch that brings it home, so it
    // cannot clear and at once come home.
    if !touchers.iter().any(|&credit| is_caster(credit)) {
        flight.clear_of_caster = true;
    }
    if touchers.iter().any(|&credit| !is_caster(credit)) {
        let radius = number("radius")?;
        return Ok(Tick::Hit(Blast {
            center: tick.position,
            half_extent: [radius, radius],
            damage: number("damage")? as i32,
            knockback: number("knockback")?,
            lifetime_s: number("hit_s")?,
            credit: if caster.credit != 0 { caster.credit } else { own_credit },
        }));
    }
    if flight.clear_of_caster {
        if let Some(index) = touchers.iter().position(|&credit| is_caster(credit)) {
            // Along the bolt's heading, not the stick: the player aimed by
            // flying it.
            let [x, y] = flight.velocity;
            let length = (x * x + y * y).sqrt();
            let push = if length > 0.0 { number("self_launch")? / length } else { 0.0 };
            return Ok(Tick::Home { toucher: index as u32, velocity: [x * push, y * push] });
        }
    }
    flight.velocity = steer(flight.velocity, caster.stick, number("turn_rate_deg")?.to_radians() * dt);
    let to = [tick.position[0] + flight.velocity[0] * dt, tick.position[1] + flight.velocity[1] * dt];
    flight.trail_in_s -= dt;
    let mark = if flight.trail_in_s <= 0.0 {
        flight.trail_in_s = number("trail_every_s")?;
        let fx = tick.text("trail_vfx").ok_or(Fault::Module("trail_vfx".into()))?.to_owned();
        Some(Effect { at: Place::World(to), fx, scale: 1.0 })
    } else {
        None
    };
    Ok(Tick::Fly { to, flight, mark })
}

/// Turn `velocity` toward `stick` by at most `max_turn` radians, keeping its
/// length: a rotation, never a snap.
pub fn steer(velocity: [f32; 2], stick: [f32; 2], max_turn: f32) -> [f32; 2] {
    let stick_length = (stick[0] * stick[0] + stick[1] * stick[1]).sqrt();
    let speed = (velocity[0] * velocity[0] + velocity[1] * velocity[1]).sqrt();
    if stick_length <= STEER_DEADZONE || speed == 0.0 {
        return velocity;
    }
    let have = [velocity[0] / speed, velocity[1] / speed];
    let want = [stick[0] / stick_length, stick[1] / stick_length];
    let cross = have[0] * want[1] - have[1] * want[0];
    let dot = (have[0] * want[0] + have[1] * want[1]).clamp(-1.0, 1.0);
    let (sin, cos) = (max_turn.min(dot.acos()) * cross.signum()).sin_cos();
    [(have[0] * cos - have[1] * sin) * speed, (have[0] * sin + have[1] * cos) * speed]
}

fn fire(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    inv.submit::<SpawnModuleEntityPort>(bolt_of(&special)?)
}

/// Fly a tick of a bolt, or end it on what it reached.
fn fly(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let caster = *inv.observe::<SpawnerBodyPort>()?;
    let touchers = inv.observe::<ToucherCreditsPort>()?.clone();
    let own_credit = *inv.observe::<CreditPort>()?;
    let flight = Flight::load(inv)?;
    match advance(&tick, &flight, inv.dt(), caster, &touchers, own_credit)? {
        Tick::Fly { to, flight, mark } => {
            flight.store(inv)?;
            inv.submit::<MoveModuleEntityPort>(MoveModuleEntity { to })?;
            match mark {
                Some(mark) => inv.submit::<EffectPort>(mark),
                None => Ok(()),
            }
        }
        Tick::Hit(blast) => {
            inv.submit::<BlastPort>(blast)?;
            inv.submit::<EndModuleEntityPort>(EndModuleEntity)
        }
        Tick::Home { toucher, velocity } => {
            inv.submit::<LaunchToucherPort>(LaunchToucher { toucher, velocity })?;
            inv.submit::<EndModuleEntityPort>(EndModuleEntity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASTER: u64 = 11;
    const RIVAL: u64 = 22;
    const DT: f32 = 0.25;

    fn tick() -> ModuleEntityTick {
        let params = [
            ("speed", 300.0),
            ("turn_rate_deg", 90.0),
            ("lifetime_s", 2.0),
            ("trail_every_s", 0.5),
            ("damage", 8.0),
            ("radius", 10.0),
            ("knockback", 1.5),
            ("self_launch", 600.0),
            ("hit_s", 0.3),
            (FACING, -1.0),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("trail_vfx".to_string(), SpecialParam::Text("trail".into()))))
        .collect();
        ModuleEntityTick { position: [100.0, 0.0], remaining_s: 1.0, nearest_enemy: None, params, touchers: Vec::new() }
    }

    fn caster() -> SpawnerBody {
        SpawnerBody { credit: CASTER, stick: [0.0, 0.0] }
    }

    fn flying(clear: bool) -> Flight {
        Flight { launched: true, velocity: [-300.0, 0.0], trail_in_s: 0.5, clear_of_caster: clear }
    }

    /// The first tick leaves the way the caster faced at the move's speed,
    /// marks the trail at once, and is not yet clear of a caster it is inside.
    #[test]
    fn a_bolt_leaves_forward_and_is_seen_at_once() {
        let Tick::Fly { to, flight, mark } = advance(&tick(), &Flight::default(), DT, caster(), &[CASTER], 9).unwrap()
        else {
            panic!("a new bolt did not fly");
        };
        assert_eq!((to, flight.velocity, flight.clear_of_caster), ([25.0, 0.0], [-300.0, 0.0], false));
        assert_eq!(mark, Some(Effect { at: Place::World([25.0, 0.0]), fx: "trail".into(), scale: 1.0 }));
        assert_eq!(flight.trail_in_s, 0.5);
    }

    /// The stick turns it by at most the turn rate and never changes its
    /// speed; a gentle stick does not steer.
    #[test]
    fn the_stick_turns_it_gradually() {
        let turned = steer([-300.0, 0.0], [1.0, 0.0], 0.1);
        assert!(turned[0] < 0.0, "one step reversed it: {turned:?}");
        assert!((turned[0].hypot(turned[1]) - 300.0).abs() < 1e-3, "turning changed its speed: {turned:?}");
        assert!((turned[1].atan2(-turned[0]).abs() - 0.1).abs() < 1e-5, "it turned other than the step: {turned:?}");
        assert_eq!(steer([-300.0, 0.0], [0.1, 0.1], 0.1), [-300.0, 0.0], "a gentle stick steered");
        let up = steer([-300.0, 0.0], [0.0, -1.0], 0.1);
        assert!(up[1] < 0.0, "it turned away from the stick: {up:?}");
    }

    /// A rival in reach is hit, credited to the caster, even with the caster
    /// in reach too; a bolt whose caster left owns its hit.
    #[test]
    fn a_rival_takes_the_hit_before_the_caster_is_carried() {
        let hit = Blast { center: [100.0, 0.0], half_extent: [10.0, 10.0], damage: 8, knockback: 1.5, lifetime_s: 0.3, credit: CASTER };
        for touchers in [[RIVAL, CASTER], [CASTER, RIVAL]] {
            assert_eq!(advance(&tick(), &flying(true), DT, caster(), &touchers, 9).unwrap(), Tick::Hit(hit.clone()));
        }
        let gone = SpawnerBody::default();
        assert_eq!(
            advance(&tick(), &flying(true), DT, gone, &[RIVAL], 9).unwrap(),
            Tick::Hit(Blast { credit: 9, ..hit })
        );
    }

    /// The caster is carried along the bolt's heading only once it is clear;
    /// before that, flying through them clears nothing and carries nobody.
    #[test]
    fn the_caster_is_carried_only_once_the_bolt_is_clear() {
        let Tick::Fly { flight, .. } = advance(&tick(), &flying(false), DT, caster(), &[CASTER], 9).unwrap() else {
            panic!("a bolt inside its caster did something");
        };
        assert!(!flight.clear_of_caster, "a bolt inside its caster cleared it");
        let Tick::Fly { flight, .. } = advance(&tick(), &flying(false), DT, caster(), &[], 9).unwrap() else {
            panic!("a bolt in the open did something");
        };
        assert!(flight.clear_of_caster, "a bolt out of its caster did not clear it");
        assert_eq!(
            advance(&tick(), &flying(true), DT, caster(), &[CASTER], 9).unwrap(),
            Tick::Home { toucher: 0, velocity: [-600.0, 0.0] }
        );
    }

    /// The trail is marked on its interval, not every tick.
    #[test]
    fn the_trail_is_marked_on_its_interval() {
        let Tick::Fly { mark, flight, .. } = advance(&tick(), &flying(true), DT, caster(), &[], 9).unwrap() else {
            panic!("the bolt did not fly");
        };
        assert_eq!((mark, flight.trail_in_s), (None, 0.25));
    }
}
