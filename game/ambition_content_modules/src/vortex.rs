//! Vortex: Attack while holding the vortex gauntlet opens a singularity ahead
//! of the body along the aim. For a moment it drags nearby enemies toward its
//! centre. It deals no damage: it gathers a group for a follow-up. Driven
//! bodies only. Migrated from the native systems (fast-iteration I7): a
//! module-owned entity, like the sentry's turret.
//!
//! The well PULLS, THEN AGES: it pulls on the tick it opens, and on the tick
//! its time runs out. So the module keeps the well's clock in a record and
//! ends the well itself; the world's lifetime is only the backstop.

use ambition_combat_port::{
    EndModuleEntity, EndModuleEntityPort, ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, PullBodies,
    PullBodiesPort, SpawnModuleEntityPort, WieldedUsePort, Wielder,
};
use ambition_extension_sdk::{
    phases::MODULE_ENTITY_TICK, record, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, Port, SchemaKey, TriggerBinding,
};
use bevy_math::Vec2;

use crate::wielded;

/// The held item's id, and the well's kind.
pub const ITEM: &str = "vortex";

const MANA_COST: f32 = 22.0;
/// How far ahead of the body, along the aim, the well opens.
const RANGE: f32 = 200.0;
/// Enemies within this distance of the centre are pulled.
const RADIUS: f32 = 220.0;
/// The fraction of the remaining gap closed per second.
const PULL_RATE: f32 = 5.0;
/// How long the well pulls.
const LIFETIME_S: f32 = 0.9;
/// The world's lifetime for the well: later than the module's own end.
const BACKSTOP_S: f32 = LIFETIME_S + 1.0;

record! {
    /// The well's clock.
    pub struct Well = SchemaKey::new(crate::PROVIDER, "vortex.well", 1);
    /// False before the well's first tick.
    1 armed: bool,
    /// Gameplay seconds the well still pulls.
    2 remaining_s: f32,
}

pub fn module() -> ModuleDescriptor {
    let mut module = wielded::module("vortex", ITEM, wielded::requests(SpawnModuleEntityPort::KEY), 3, cast);
    module.entries[0].key = "cast".into();
    module.schemas.push(Well::schema());
    module.entries.push(EntryDescriptor {
        key: "well".into(),
        phase: MODULE_ENTITY_TICK,
        trigger: TriggerBinding {
            port: ModuleEntityTickPort::KEY,
            selector: ITEM.into(),
        },
        reads: Vec::new(),
        writes: vec![Well::KEY],
        requests: vec![PullBodiesPort::KEY, EndModuleEntityPort::KEY],
        after: Vec::new(),
        limits: Limits { max_requests: 2 },
        // A living well is never idle.
        on_idle: IdlePolicy::Invoke,
        run: EntryCode::Native(well),
    });
    module
}

fn cast(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let w: Wielder = inv.trigger::<WieldedUsePort>()?.clone();
    // A body that cannot name the well is not charged for it.
    if !w.driven || !w.names_spawns || !wielded::pay(inv, &w, MANA_COST)? {
        return Ok(());
    }
    let aim = Vec2::from(w.to_world(w.aim_local)).normalize_or_zero();
    // Paid, and no direction: as the native cast, the mana is spent and no
    // well opens.
    if aim == Vec2::ZERO {
        return Ok(());
    }
    let center = Vec2::from(w.position) + aim * RANGE;
    inv.submit::<SpawnModuleEntityPort>(ModuleEntitySpawn {
        kind: ITEM.into(),
        at: center.into(),
        lifetime_s: BACKSTOP_S,
    })?;
    inv.submit::<ambition_combat_port::BodySoundPort>(ambition_combat_port::BodySound {
        cue: "player.blink".into(),
        at: ambition_combat_port::Place::World(center.into()),
    })
}

fn well(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick: ModuleEntityTick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let dt = inv.dt();
    let mut c = Well::load(inv)?;
    if !c.armed {
        c.armed = true;
        c.remaining_s = LIFETIME_S;
    }
    inv.submit::<PullBodiesPort>(PullBodies {
        center: tick.position,
        radius: RADIUS,
        rate: PULL_RATE,
    })?;
    c.remaining_s -= dt;
    if c.remaining_s <= 0.0 {
        inv.submit::<EndModuleEntityPort>(EndModuleEntity)?;
    }
    c.store(inv)
}
