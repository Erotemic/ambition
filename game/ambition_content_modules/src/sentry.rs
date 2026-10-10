//! Sentry: Attack while holding the sentry gauntlet drops a turret at the
//! body. The turret lives a few seconds and, on a cadence, fires a bolt at the
//! nearest enemy in range. Migrated from the native systems (fast-iteration
//! I7): the first module-owned entity. The world owns the turret (its
//! identity, side, position and lifetime); this module owns what it does.
//!
//! Two entries: `deploy` on the held item's use, and `turret` on each tick of
//! a living turret. The turret's cadence is a record scoped to the turret.

use ambition_combat_port::{
    ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, SpawnModuleEntityPort, WieldedUsePort, Wielder,
};
use ambition_combat_port::{BodySound, BodySoundPort};
use ambition_extension_sdk::{
    phases::MODULE_ENTITY_TICK, record, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, Port, SchemaKey, TriggerBinding,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

use crate::wielded;

/// The held item's id, and the turret's kind.
pub const ITEM: &str = "sentry";

const MANA_COST: f32 = 28.0;
const LIFETIME_S: f32 = 5.0;
/// The delay before the first shot.
const ARM_DELAY_S: f32 = 0.25;
const FIRE_INTERVAL_S: f32 = 0.55;
/// Enemies further than this are not targets.
const RANGE: f32 = 480.0;
const BOLT_SPEED: f32 = 430.0;
pub const BOLT_DAMAGE: i32 = 2;
const BOLT_LIFETIME: f32 = 1.4;
const BOLT_HALF: Vec2 = Vec2::new(7.0, 7.0);

record! {
    /// The turret's cadence.
    pub struct Cadence = SchemaKey::new(crate::PROVIDER, "sentry.cadence", 1);
    /// False before the turret's first tick.
    1 armed: bool,
    /// Gameplay seconds until the turret can fire.
    2 cooldown_s: f32,
}

pub fn module() -> ModuleDescriptor {
    let mut module = wielded::module("sentry", ITEM, wielded::requests(SpawnModuleEntityPort::KEY), 3, deploy);
    module.entries[0].key = "deploy".into();
    module.schemas.push(Cadence::schema());
    module.entries.push(EntryDescriptor {
        key: "turret".into(),
        phase: MODULE_ENTITY_TICK,
        trigger: TriggerBinding {
            port: ModuleEntityTickPort::KEY,
            selector: ITEM.into(),
        },
        reads: Vec::new(),
        writes: vec![Cadence::KEY],
        requests: vec![ProjectileSpawnPort::KEY, BodySoundPort::KEY],
        after: Vec::new(),
        limits: Limits { max_requests: 2 },
        // A living turret is never idle.
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(turret),
    });
    module
}

fn deploy(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    // A body that cannot name the turret is not charged for it.
    if !w.names_spawns || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    inv.submit::<SpawnModuleEntityPort>(ModuleEntitySpawn {
        kind: ITEM.into(),
        at: w.position,
        lifetime_s: LIFETIME_S,
    })?;
    wielded::rock_hit(inv, &w)
}

/// One bolt of the turret, as the game fires it, from `origin` along `dir`.
pub fn authored_bolt(origin: Vec2, dir: Vec2) -> ProjectileSpawn {
    ProjectileSpawn {
        origin,
        dir,
        speed: BOLT_SPEED,
        damage: BOLT_DAMAGE,
        max_lifetime: BOLT_LIFETIME,
        half_extent: BOLT_HALF,
        gravity: 0.0,
        visual_id: String::new(),
        bounces: 0,
        bounce_on_world_contact: false,
        splash_half_extent: 0.0,
        boomerang_return_s: None,
    }
}

fn turret(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick: ModuleEntityTick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let dt = inv.dt();
    let mut c = Cadence::load(inv)?;
    if !c.armed {
        c.armed = true;
        c.cooldown_s = ARM_DELAY_S;
    }
    c.cooldown_s -= dt;
    let fired = if c.cooldown_s > 0.0 {
        Ok(())
    } else {
        fire(inv, &tick, &mut c)
    };
    c.store(inv)?;
    fired
}

fn fire(inv: &mut Invocation<'_>, tick: &ModuleEntityTick, c: &mut Cadence) -> Result<(), Fault> {
    let at = Vec2::from(tick.position);
    let target = tick.nearest_enemy.map(Vec2::from).filter(|t| t.distance(at) <= RANGE);
    let Some(target) = target else {
        // No target: idle, ready to fire as soon as an enemy comes.
        c.cooldown_s = 0.0;
        return Ok(());
    };
    let dir = (target - at).normalize_or_zero();
    if dir == Vec2::ZERO {
        return Ok(());
    }
    inv.submit::<ProjectileSpawnPort>(authored_bolt(at, dir))?;
    c.cooldown_s = FIRE_INTERVAL_S;
    inv.submit::<BodySoundPort>(BodySound {
        cue: "world.rock.hit".into(),
        at: ambition_combat_port::Place::World(tick.position),
    })
}
