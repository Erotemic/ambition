//! Pulse Forge's charge and emitter rules for linked Rust and WASM.

use ambition_combat_port::{
    EndModuleEntity, EndModuleEntityPort, ModuleEntitySpawn, ModuleEntityTickPort,
    SpawnModuleEntityPort, SpecialParam, SpendMana, SpendManaPort, TechniqueUsePort,
};
use ambition_extension_sdk::{
    phases, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy,
    Invocation, Limits, ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};
use ambition_projectile_spec::{ProjectileSpawn, ProjectileSpawnPort};
use bevy_math::Vec2;

pub const PROVIDER: &str = "pulse_forge";
pub const KIND: &str = "pulse_forge.emitter";
const CHARGE_PER_PULSE_S: f32 = 0.2;
const MAX_CHARGE_S: f32 = 1.0;
const PULSE_INTERVAL_S: f32 = 0.25;
const BOLT_SPEED: f32 = 240.0;

record! {
    /// The body's charge in gameplay seconds.
    pub struct Charge = SchemaKey::new(PROVIDER, "emitter.charge", 1);
    1 seconds: f32,
}

record! {
    /// The emitter's pulse count and clock.
    pub struct Cadence = SchemaKey::new(PROVIDER, "emitter.cadence", 1);
    1 fired: u32,
    2 cooldown_s: f32,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(PROVIDER, "emitter"), api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(), version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Charge::schema(), Cadence::schema()],
        entries: vec![
            EntryDescriptor {
                key: "charge".into(), phase: phases::TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: TechniqueUsePort::KEY, selector: PROVIDER.into() },
                reads: vec![], writes: vec![Charge::KEY], requests: vec![SpendManaPort::KEY, SpawnModuleEntityPort::KEY], after: vec![],
                limits: Limits { max_requests: 2 }, on_idle: IdlePolicy::ResetState,
                run: EntryCode::Native(charge),
            },
            EntryDescriptor {
                key: "pulse".into(), phase: phases::MODULE_ENTITY_TICK,
                trigger: TriggerBinding { port: ModuleEntityTickPort::KEY, selector: KIND.into() },
                reads: vec![], writes: vec![Cadence::KEY],
                requests: vec![ProjectileSpawnPort::KEY, EndModuleEntityPort::KEY], after: vec![],
                limits: Limits { max_requests: 4 }, on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(pulse),
            },
        ],
    }
}

fn charge(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let trigger = inv.trigger::<TechniqueUsePort>()?.clone();
    let mut c = Charge::load(inv)?;
    if trigger.held || trigger.user.pressed {
        c.seconds = (c.seconds + inv.dt()).min(MAX_CHARGE_S);
    }
    if trigger.released {
        let pulses = ((c.seconds / CHARGE_PER_PULSE_S) as u32).min(5);
        if pulses > 0 && trigger.user.names_spawns {
            inv.submit::<SpendManaPort>(SpendMana { amount: pulses as f32 * 12.0 })?;
            inv.submit::<SpawnModuleEntityPort>(ModuleEntitySpawn {
                kind: KIND.into(), at: trigger.user.position, lifetime_s: 3.0,
                half_extents: [0.0, 0.0],
                params: vec![("pulses".into(), SpecialParam::Number(pulses as f32))],
            })?;
        }
        c.seconds = 0.0;
    }
    c.store(inv)
}

fn pulse(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let trigger = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let pulses = trigger.number("pulses").ok_or_else(|| Fault::Module("emitter params need pulses".into()))?;
    let mut c = Cadence::load(inv)?;
    if c.fired as f32 >= pulses {
        inv.submit::<EndModuleEntityPort>(EndModuleEntity {})?;
        return Ok(());
    }
    c.cooldown_s -= inv.dt();
    if c.cooldown_s <= 0.0 {
        for direction in [[1.0, 0.0], [0.0, 1.0], [-1.0, 0.0], [0.0, -1.0]] {
            inv.submit::<ProjectileSpawnPort>(ProjectileSpawn {
                origin: Vec2::from(trigger.position), dir: Vec2::from(direction),
                speed: BOLT_SPEED, damage: 2, max_lifetime: 1.0, half_extent: Vec2::splat(5.0),
                gravity: 0.0, visual_id: String::new(), bounces: 0, bounce_on_world_contact: false,
                splash_half_extent: 0.0, boomerang_return_s: None,
            })?;
        }
        c.fired += 1;
        c.cooldown_s = PULSE_INTERVAL_S;
    }
    c.store(inv)
}

pub const MODULES: &[fn() -> ModuleDescriptor] = &[module];

ambition_extension_sdk::export_modules!(list: crate::MODULES);
