//! A plate on the floor that throws whoever steps on it.
//!
//! The move drops it (`smash.place_spring`): the world puts a module entity
//! of kind [`PLATE`] where the move asks, with the move's params, and ticks it
//! through this module until its lifetime ends. The plate throws any body that
//! touches it, its dropper and an opponent alike: a plate that served only its
//! dropper would be a second recovery.
//!
//! Each number is the move's (`params` of its `smash.place_spring` effect),
//! and admission checked them (`check_place_spring_params`), so a missing one
//! is a fault and not a plate that does nothing. The plate's clocks and its
//! uses are a record of the plate ([`Plate`]), so a rewind restores them.

use ambition_combat_port::{
    Effect, EffectPort, EndModuleEntity, EndModuleEntityPort, FighterSpecial, FighterSpecialPort, LaunchToucher,
    LaunchToucherPort, ModuleEntitySpawn, ModuleEntityTick, ModuleEntityTickPort, Place, SpawnModuleEntityPort,
};
use ambition_extension_sdk::{
    phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION},
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey,
    Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors to drop a plate. The Smash crate installs
/// this module for its catalog's key (`install_module_technique`).
pub const PLACE_SPRING: &str = "smash.place_spring";

/// The kind of the module entity a plate is.
pub const PLATE: &str = "smash.spring";

record! {
    /// The plate's clocks and uses, from the move's params on its first tick.
    pub struct Plate = SchemaKey::new(crate::PROVIDER, "spring.plate", 1);
    /// False before the plate's first tick.
    1 landed: bool,
    /// Seconds before it throws anybody. The plate lands under the dropper's
    /// feet, so without this delay he throws himself.
    2 arm_s: f32,
    /// Seconds before it throws again. A throw does not move the body off the
    /// plate in the same tick, so without this delay one touch spends every
    /// use.
    3 rearm_s: f32,
    /// Throws left in it.
    4 uses_left: u32,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "spring"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Plate::schema()],
        entries: vec![
            EntryDescriptor {
                key: PLACE_SPRING.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: PLACE_SPRING.into() },
                reads: Vec::new(),
                writes: Vec::new(),
                requests: vec![SpawnModuleEntityPort::KEY, EffectPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(drop_plate),
            },
            EntryDescriptor {
                key: PLATE.into(),
                phase: MODULE_ENTITY_TICK,
                trigger: TriggerBinding { port: ModuleEntityTickPort::KEY, selector: PLATE.into() },
                reads: Vec::new(),
                writes: vec![Plate::KEY],
                requests: vec![LaunchToucherPort::KEY, EffectPort::KEY, EndModuleEntityPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 3 },
                // A plate on the floor is never idle.
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(plate),
            },
        ],
    }
}

fn number(params: impl Fn(&'static str) -> Option<f32>, path: &'static str) -> Result<f32, Fault> {
    params(path).ok_or(Fault::Module(path.into()))
}

/// The plate a drop asks for: where it lands (body-local, mirrored by the
/// fighter's facing), its size and its lifetime. Its params go with it.
pub fn plate_of(special: &FighterSpecial) -> Result<ModuleEntitySpawn, Fault> {
    let n = |path| number(|p| special.number(p), path);
    let offset = [n("offset[0]")? * special.facing, n("offset[1]")?];
    Ok(ModuleEntitySpawn {
        kind: PLATE.into(),
        at: [special.position[0] + offset[0], special.position[1] + offset[1]],
        lifetime_s: n("lifetime_s")?,
        half_extents: [n("half_extents[0]")?, n("half_extents[1]")?],
        params: special.params.clone(),
    })
}

/// The plate's cue, where it is: a plate nobody saw arrive is an ambush.
fn announce(inv: &mut Invocation<'_>, fx: Option<&str>, at: [f32; 2]) -> Result<(), Fault> {
    match fx.filter(|fx| !fx.trim().is_empty()) {
        Some(fx) => inv.submit::<EffectPort>(Effect { at: Place::World(at), fx: fx.to_owned(), scale: 1.0 }),
        None => Ok(()),
    }
}

fn drop_plate(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let spawn = plate_of(&special)?;
    let at = spawn.at;
    inv.submit::<SpawnModuleEntityPort>(spawn)?;
    announce(inv, special.text("vfx"), at)
}

/// One tick of a plate: spend its clocks, throw the first toucher (in
/// identity order) when it is armed, and take it away when its uses run out.
fn plate(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick: ModuleEntityTick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let n = |path| number(|p| tick.number(p), path);
    let dt = inv.dt();
    let mut plate = Plate::load(inv)?;
    if !plate.landed {
        plate.landed = true;
        plate.arm_s = n("arm_s")?;
        plate.uses_left = n("uses")? as u32;
    }
    plate.arm_s = (plate.arm_s - dt).max(0.0);
    plate.rearm_s = (plate.rearm_s - dt).max(0.0);
    let ready = plate.arm_s <= 0.0 && plate.rearm_s <= 0.0;
    if ready && plate.uses_left > 0 && !tick.touchers.is_empty() {
        inv.submit::<LaunchToucherPort>(LaunchToucher { toucher: 0, velocity: [n("launch[0]")?, n("launch[1]")?] })?;
        announce(inv, tick.text("vfx"), tick.position)?;
        plate.uses_left -= 1;
        plate.rearm_s = n("rearm_s")?;
    }
    let spent = plate.uses_left == 0;
    plate.store(inv)?;
    if spent {
        inv.submit::<EndModuleEntityPort>(EndModuleEntity)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    /// The plate lands at the move's offset, mirrored by the facing, with the
    /// move's size and lifetime, and keeps the move's params.
    #[test]
    fn the_plate_lands_where_the_move_asks() {
        use SpecialParam::Number;
        let params: Vec<(String, SpecialParam)> = [
            ("half_extents[0]", 20.0),
            ("half_extents[1]", 6.0),
            ("lifetime_s", 4.0),
            ("offset[0]", 30.0),
            ("offset[1]", 18.0),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_owned(), Number(n)))
        .collect();
        for (facing, x) in [(1.0, 130.0), (-1.0, 70.0)] {
            let special = FighterSpecial { position: [100.0, 50.0], facing, params: params.clone() };
            let spawn = plate_of(&special).unwrap();
            assert_eq!(spawn.at, [x, 68.0], "facing {facing}");
            assert_eq!((spawn.half_extents, spawn.lifetime_s, spawn.kind.as_str()), ([20.0, 6.0], 4.0, PLATE));
            assert_eq!(spawn.params, params);
        }
        let without_lifetime: Vec<_> = params.iter().filter(|(path, _)| path != "lifetime_s").cloned().collect();
        let special = FighterSpecial { position: [0.0, 0.0], facing: 1.0, params: without_lifetime };
        assert!(plate_of(&special).is_err(), "a plate with no lifetime is a fault");
    }
}
