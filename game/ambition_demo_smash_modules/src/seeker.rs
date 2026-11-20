//! The seeker: a shot that locks one foe and follows that foe.
//!
//! When a move authors `smash.seeker`, its fighter puts a module entity of the
//! kind [`SEEKER_SHOT`] at the move's offset. On its first tick it locks a
//! foe: the nearest of its caster's foes (`ambition.world.spawner_foes`, the
//! engine's answer to who is a foe) on the side the caster faced and within
//! the move's lock range, the earlier in identity order on a tie. The lock is
//! the foe's credit, kept in the shot's own record ([`Seek`]), so it names
//! the same body after a rewind and on every peer. Each tick the shot turns
//! toward that body, never faster than the move's turn rate and never
//! changing speed: a foe who moves across its path outruns its turn. A foe
//! who leaves (knocked out, gone from the room) is not replaced: the shot
//! flies on straight. With no foe to lock it is a straight shot.
//!
//! Whatever rival it reaches takes the hit, credited to the caster, as the
//! steered bolt's; the caster does not stop it. It draws a trail on the
//! move's interval.

use ambition_combat_port::{
    Blast, BlastPort, CreditPort, Effect, EffectPort, EndModuleEntity, EndModuleEntityPort, FighterSpecial,
    FighterSpecialPort, MarkedFoe, ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, MoveModuleEntity,
    MoveModuleEntityPort, Place, SpawnModuleEntityPort, SpawnerBody, SpawnerBodyPort, SpawnerFoesPort, SpecialParam,
    ToucherCreditsPort,
};
use ambition_extension_sdk::{
    phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION},
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a seeker. The Smash crate installs this
/// module for its catalog's key (`install_module_technique`).
pub const SEEKER: &str = "smash.seeker";

/// The kind of the module entity a seeker in flight is.
pub const SEEKER_SHOT: &str = "smash.seeker_shot";

/// The param the press adds to the move's: which way the caster faced.
const FACING: &str = "facing";

record! {
    /// A seeker in flight: what the world's entity does not keep.
    pub struct Seek = SchemaKey::new(crate::PROVIDER, "seeker.flight", 1);
    /// Whether it has flown a tick; before that the rest is unset.
    1 launched: bool,
    /// Where it is going. Its length is the move's speed.
    2 velocity: [f32; 2],
    /// The credit of the foe it locked; `0` when it locked none.
    3 target: u64,
    /// Seconds to the next trail mark.
    4 trail_in_s: f32,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "seeker"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Seek::schema()],
        entries: vec![
            EntryDescriptor {
                replies: Vec::new(),
                key: SEEKER.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: SEEKER.into() },
                reads: Vec::new(),
                writes: Vec::new(),
                requests: vec![SpawnModuleEntityPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(fire),
            },
            EntryDescriptor {
                replies: Vec::new(),
                key: SEEKER_SHOT.into(),
                phase: MODULE_ENTITY_TICK,
                trigger: TriggerBinding { port: ModuleEntityTickPort::KEY, selector: SEEKER_SHOT.into() },
                reads: vec![SpawnerBodyPort::KEY, SpawnerFoesPort::KEY, ToucherCreditsPort::KEY, CreditPort::KEY],
                writes: vec![Seek::KEY],
                requests: vec![MoveModuleEntityPort::KEY, EffectPort::KEY, BlastPort::KEY, EndModuleEntityPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(fly),
            },
        ],
    }
}

/// The shot a press fires: at the move's offset (body-local, mirrored by
/// facing), as big as its radius, for its lifetime, carrying the move's
/// params and the caster's facing.
pub fn shot_of(special: &FighterSpecial) -> Result<ModuleEntitySpawn, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let radius = number("radius")?;
    let mut params = special.params.clone();
    params.push((FACING.into(), SpecialParam::Number(special.facing)));
    Ok(ModuleEntitySpawn {
        kind: SEEKER_SHOT.into(),
        at: [
            special.position[0] + number("offset[0]")? * special.facing,
            special.position[1] + number("offset[1]")?,
        ],
        lifetime_s: number("lifetime_s")?,
        half_extents: [radius, radius],
        params,
    })
}

/// The foe a shot at `at`, fired facing `facing`, locks: the nearest foe on
/// the faced side within `range`, the earlier in the list (identity order)
/// on a tie. `0` when there is none.
pub fn lock(at: [f32; 2], facing: f32, range: f32, foes: &[MarkedFoe]) -> u64 {
    let mut best: Option<(f32, u64)> = None;
    for foe in foes {
        let (dx, dy) = (foe.position[0] - at[0], foe.position[1] - at[1]);
        let distance_sq = dx * dx + dy * dy;
        if foe.credit == 0 || dx * facing.signum() < 0.0 || distance_sq > range * range {
            continue;
        }
        if best.is_none_or(|(nearest, _)| distance_sq < nearest) {
            best = Some((distance_sq, foe.credit));
        }
    }
    best.map_or(0, |(_, credit)| credit)
}

/// What one tick of a shot does.
#[derive(Debug, PartialEq)]
pub enum Tick {
    /// It flies on: where it goes, the record it leaves, and a trail mark
    /// there when one is due.
    Fly { to: [f32; 2], seek: Seek, mark: Option<Effect> },
    /// It reached a rival.
    Hit(Blast),
}

/// What a tick of a shot does, given its record so far, the world's step,
/// its caster, the caster's foes, who touches it, and its own credit.
pub fn advance(
    tick: &ModuleEntityTick,
    seek: &Seek,
    dt: f32,
    caster: SpawnerBody,
    foes: &[MarkedFoe],
    touchers: &[u64],
    own_credit: u64,
) -> Result<Tick, Fault> {
    let number = |path: &'static str| tick.number(path).ok_or(Fault::Module(path.into()));
    let mut seek = seek.clone();
    if !seek.launched {
        let facing = number(FACING)?;
        seek = Seek {
            launched: true,
            velocity: [facing.signum() * number("speed")?, 0.0],
            target: lock(tick.position, facing, number("lock_range")?, foes),
            trail_in_s: 0.0,
        };
    }
    let is_caster = |credit: u64| caster.credit != 0 && credit == caster.credit;
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
    if let Some(foe) = foes.iter().find(|foe| seek.target != 0 && foe.credit == seek.target) {
        let toward = [foe.position[0] - tick.position[0], foe.position[1] - tick.position[1]];
        seek.velocity = crate::bolt::turn_toward(seek.velocity, toward, number("turn_rate_deg")?.to_radians() * dt);
    }
    let to = [tick.position[0] + seek.velocity[0] * dt, tick.position[1] + seek.velocity[1] * dt];
    seek.trail_in_s -= dt;
    let mark = if seek.trail_in_s <= 0.0 {
        seek.trail_in_s = number("trail_every_s")?;
        let fx = tick.text("trail_vfx").ok_or(Fault::Module("trail_vfx".into()))?.to_owned();
        Some(Effect { at: Place::World(to), fx, scale: 1.0, angle: 0.0 })
    } else {
        None
    };
    Ok(Tick::Fly { to, seek, mark })
}

fn fire(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    inv.submit::<SpawnModuleEntityPort>(shot_of(&special)?)
}

/// Fly a tick of a shot, or end it on the rival it reached.
fn fly(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let caster = *inv.observe::<SpawnerBodyPort>()?;
    let foes = inv.observe::<SpawnerFoesPort>()?.foes.clone();
    let touchers = inv.observe::<ToucherCreditsPort>()?.clone();
    let own_credit = *inv.observe::<CreditPort>()?;
    let seek = Seek::load(inv)?;
    match advance(&tick, &seek, inv.dt(), caster, &foes, &touchers, own_credit)? {
        Tick::Fly { to, seek, mark } => {
            seek.store(inv)?;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASTER: u64 = 11;
    const NEAR: u64 = 22;
    const FAR: u64 = 33;
    const DT: f32 = 0.1;

    fn tick(position: [f32; 2], facing: f32) -> ModuleEntityTick {
        let params = [
            ("speed", 400.0),
            ("turn_rate_deg", 90.0),
            ("lock_range", 500.0),
            ("lifetime_s", 2.0),
            ("trail_every_s", 0.5),
            ("damage", 6.0),
            ("radius", 10.0),
            ("knockback", 0.9),
            ("hit_s", 0.1),
            (FACING, facing),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("trail_vfx".to_string(), SpecialParam::Text("trail".into()))))
        .collect();
        ModuleEntityTick { position, remaining_s: 1.0, nearest_enemy: None, params, touchers: Vec::new() }
    }

    fn foe(credit: u64, position: [f32; 2]) -> MarkedFoe {
        MarkedFoe { credit, position, size: [20.0, 40.0] }
    }

    fn caster() -> SpawnerBody {
        SpawnerBody { credit: CASTER, stick: [0.0, 0.0] }
    }

    /// It locks the nearest foe on the faced side within range: a nearer foe
    /// behind it and a foe out of range are not locked, and a tie goes to the
    /// earlier in identity order.
    #[test]
    fn it_locks_the_nearest_foe_ahead_in_range() {
        let foes = [foe(FAR, [300.0, 0.0]), foe(NEAR, [-50.0, 0.0])];
        assert_eq!(lock([0.0, 0.0], 1.0, 500.0, &foes), FAR, "it locked the foe behind it");
        assert_eq!(lock([0.0, 0.0], -1.0, 500.0, &foes), NEAR);
        assert_eq!(lock([0.0, 0.0], 1.0, 200.0, &foes), 0, "it locked a foe out of range");
        let tied = [foe(NEAR, [100.0, 0.0]), foe(FAR, [0.0, 100.0]), foe(FAR + 1, [100.0, 0.0])];
        assert_eq!(lock([0.0, 0.0], 1.0, 500.0, &tied), NEAR);
    }

    /// Locked on a foe above its path, it turns toward that foe a little each
    /// tick and keeps its speed; the lock survives another foe coming nearer.
    #[test]
    fn it_turns_toward_its_locked_foe_and_keeps_it() {
        let foes = [foe(NEAR, [200.0, -200.0])];
        let Tick::Fly { seek, .. } = advance(&tick([0.0, 0.0], 1.0), &Seek::default(), DT, caster(), &foes, &[CASTER], 9).unwrap()
        else {
            panic!("a new shot did not fly");
        };
        assert_eq!(seek.target, NEAR);
        assert!(seek.velocity[1] < 0.0 && seek.velocity[0] > 0.0, "it did not turn up toward its foe: {:?}", seek.velocity);
        assert!((seek.velocity[0].hypot(seek.velocity[1]) - 400.0).abs() < 1e-2, "turning changed its speed");
        let turned = seek.velocity[1].atan2(seek.velocity[0]).abs();
        assert!((turned - 9f32.to_radians()).abs() < 1e-4, "it turned other than its rate allows: {turned}");
        let crowded = [foe(NEAR, [200.0, -200.0]), foe(FAR, [40.0, 30.0])];
        let Tick::Fly { seek, .. } = advance(&tick([40.0, 0.0], 1.0), &seek, DT, caster(), &crowded, &[], 9).unwrap() else {
            panic!("the shot did not fly");
        };
        assert_eq!(seek.target, NEAR, "a nearer foe took the lock");
        assert!(seek.velocity[1] < -60.0, "it stopped following its foe: {:?}", seek.velocity);
    }

    /// A foe who leaves is not replaced: the shot flies on straight.
    #[test]
    fn a_lost_foe_leaves_a_straight_shot() {
        let seek = Seek { launched: true, velocity: [400.0, 0.0], target: NEAR, trail_in_s: 0.5 };
        let Tick::Fly { to, seek: after, .. } =
            advance(&tick([0.0, 0.0], 1.0), &seek, DT, caster(), &[foe(FAR, [100.0, -100.0])], &[], 9).unwrap()
        else {
            panic!("the shot did not fly");
        };
        assert_eq!((to, after.velocity, after.target), ([40.0, 0.0], [400.0, 0.0], NEAR));
    }

    /// A rival in reach is hit, credited to the caster; the caster in reach
    /// does not stop it.
    #[test]
    fn a_rival_is_hit_and_the_caster_is_not() {
        let seek = Seek { launched: true, velocity: [400.0, 0.0], target: NEAR, trail_in_s: 0.5 };
        assert!(matches!(advance(&tick([0.0, 0.0], 1.0), &seek, DT, caster(), &[], &[CASTER], 9).unwrap(), Tick::Fly { .. }));
        assert_eq!(
            advance(&tick([0.0, 0.0], 1.0), &seek, DT, caster(), &[], &[CASTER, FAR], 9).unwrap(),
            Tick::Hit(Blast { center: [0.0, 0.0], half_extent: [10.0, 10.0], damage: 6, knockback: 0.9, lifetime_s: 0.1, credit: CASTER })
        );
    }
}
