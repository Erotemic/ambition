//! The bomb: a ground item with a fuse.
//!
//! When a move authors `smash.drop_bomb`, its fighter drops the held item the
//! move names at the move's offset (`ambition.world.spawn_module_item`), so any
//! body can pick it up and throw it. The bomb is a module entity of the kind
//! [`BOMB`], and this module ticks it: it burns the fuse whoever holds it, and
//! goes off when the fuse runs out or when it strikes something loose on the
//! stage at the move's `impact_speed` or faster, whichever comes first, where
//! it is then. A carried bomb cannot be set off by an impact.
//!
//! The bomb owns its blast (`ambition.combat.credit`): it changes hands, so
//! no fighter is credited.

use ambition_combat_port::{
    Blast, BlastPort, CreditPort, EndModuleEntity, EndModuleEntityPort, FighterSpecial, FighterSpecialPort, ItemState,
    ItemStatePort, ModuleEntityTick, ModuleEntityTickPort, ModuleItemSpawn, SpawnModuleItemPort,
};
use ambition_extension_sdk::{
    phases::{MODULE_ENTITY_TICK, TECHNIQUE_EXECUTION},
    record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor,
    ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a bomb drop. The Smash crate installs
/// this module for its catalog's key (`install_module_technique`).
pub const DROP_BOMB: &str = "smash.drop_bomb";

/// The kind of the module entity a dropped bomb is.
pub const BOMB: &str = "smash.bomb";

/// How long a bomb's entity outlives its fuse: the fuse ends it, and the
/// lifetime only backs that up.
const LIFETIME_PAST_FUSE_S: f32 = 1.0;

record! {
    /// How much of a bomb's fuse has burned.
    pub struct Fuse = SchemaKey::new(crate::PROVIDER, "bomb.fuse", 1);
    /// World seconds burned.
    1 burned_s: f32,
}

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "bomb"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Fuse::schema()],
        entries: vec![
            EntryDescriptor {
                key: DROP_BOMB.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: DROP_BOMB.into() },
                reads: Vec::new(),
                writes: Vec::new(),
                requests: vec![SpawnModuleItemPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(drop_bomb),
            },
            EntryDescriptor {
                key: BOMB.into(),
                phase: MODULE_ENTITY_TICK,
                trigger: TriggerBinding { port: ModuleEntityTickPort::KEY, selector: BOMB.into() },
                reads: vec![ItemStatePort::KEY, CreditPort::KEY],
                writes: vec![Fuse::KEY],
                requests: vec![BlastPort::KEY, EndModuleEntityPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(burn),
            },
        ],
    }
}

/// The bomb a press drops: the move's item at the move's offset (body-local,
/// mirrored by facing), carrying the move's params for its ticks. Every
/// number is the move's, which admission checked (`check_drop_bomb_params`),
/// so a missing one is a fault.
pub fn drop_of(special: &FighterSpecial) -> Result<ModuleItemSpawn, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let item = special.text("item_id").ok_or(Fault::Module("item_id".into()))?.to_owned();
    let at = [
        special.position[0] + number("offset[0]")? * special.facing,
        special.position[1] + number("offset[1]")?,
    ];
    Ok(ModuleItemSpawn {
        kind: BOMB.into(),
        item,
        at,
        lifetime_s: number("fuse_s")? + LIFETIME_PAST_FUSE_S,
        half_extents: [number("half_extents[0]")?, number("half_extents[1]")?],
        params: special.params.clone(),
    })
}

/// What a tick of a bomb does, given how much fuse has burned before it, the
/// world's step, the item it is and its own credit: the fuse it leaves (none
/// when it goes off) and the blast.
pub fn advance(
    tick: &ModuleEntityTick,
    burned_s: f32,
    dt: f32,
    item: ItemState,
    credit: u64,
) -> Result<(Option<f32>, Option<Blast>), Fault> {
    let number = |path: &'static str| tick.number(path).ok_or(Fault::Module(path.into()));
    let burned_s = burned_s + dt;
    // Only a loose bomb goes off on impact: a hand is not a surface.
    let struck_hard = item.in_world && item.impact_speed >= number("impact_speed")?;
    if burned_s < number("fuse_s")? && !struck_hard {
        return Ok((Some(burned_s), None));
    }
    let radius = number("blast_radius")?;
    let blast = Blast {
        center: tick.position,
        half_extent: [radius, radius],
        damage: number("damage")? as i32,
        knockback: number("knockback")?,
        lifetime_s: number("blast_s")?,
        credit,
    };
    Ok((None, Some(blast)))
}

fn drop_bomb(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    inv.submit::<SpawnModuleItemPort>(drop_of(&special)?)
}

/// Burn a tick of the fuse, and go off when it runs out or on a hard impact.
fn burn(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let tick = inv.trigger::<ModuleEntityTickPort>()?.clone();
    let item = *inv.observe::<ItemStatePort>()?;
    let credit = *inv.observe::<CreditPort>()?;
    let fuse = Fuse::load(inv)?;
    match advance(&tick, fuse.burned_s, inv.dt(), item, credit)? {
        (Some(burned_s), _) => Fuse { burned_s }.store(inv),
        (None, blast) => {
            if let Some(blast) = blast {
                inv.submit::<BlastPort>(blast)?;
            }
            inv.submit::<EndModuleEntityPort>(EndModuleEntity)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn params() -> Vec<(String, SpecialParam)> {
        [
            ("fuse_s", 1.0),
            ("damage", 12.0),
            ("blast_radius", 60.0),
            ("impact_speed", 300.0),
            ("half_extents[0]", 8.0),
            ("half_extents[1]", 9.0),
            ("offset[0]", 20.0),
            ("offset[1]", -4.0),
            ("knockback", 1.6),
            ("blast_s", 0.1),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("item_id".to_string(), SpecialParam::Text("polygon_bomb".into()))))
        .collect()
    }

    /// The drop is the move's item at its offset, mirrored by facing.
    #[test]
    fn a_drop_is_the_moves_item_at_its_mirrored_offset() {
        let special = FighterSpecial { position: [100.0, 50.0], facing: -1.0, params: params() };
        let spawn = drop_of(&special).unwrap();
        assert_eq!((spawn.kind.as_str(), spawn.item.as_str()), (BOMB, "polygon_bomb"));
        assert_eq!(spawn.at, [80.0, 46.0]);
        assert_eq!(spawn.half_extents, [8.0, 9.0]);
        assert_eq!(spawn.lifetime_s, 1.0 + LIFETIME_PAST_FUSE_S);
    }

    /// The fuse burns to its end; a hard impact on the stage goes off early;
    /// a gentle one, or a hard one in a hand, does not.
    #[test]
    fn a_bomb_goes_off_at_the_end_of_its_fuse_or_on_a_hard_loose_impact() {
        let tick = ModuleEntityTick {
            position: [7.0, 8.0],
            remaining_s: 1.0,
            nearest_enemy: None,
            params: params(),
            touchers: Vec::new(),
        };
        let resting = ItemState { in_world: true, impact_speed: 0.0 };
        assert_eq!(advance(&tick, 0.5, 0.25, resting, 3).unwrap(), (Some(0.75), None));
        let (left, blast) = advance(&tick, 0.75, 0.25, resting, 3).unwrap();
        assert!(left.is_none(), "a spent fuse kept burning");
        assert_eq!(
            blast,
            Some(Blast { center: [7.0, 8.0], half_extent: [60.0, 60.0], damage: 12, knockback: 1.6, lifetime_s: 0.1, credit: 3 })
        );
        let hard = ItemState { in_world: true, impact_speed: 300.0 };
        assert!(advance(&tick, 0.0, 0.01, hard, 3).unwrap().1.is_some(), "a hard impact did not set it off");
        let gentle = ItemState { in_world: true, impact_speed: 299.0 };
        assert!(advance(&tick, 0.0, 0.01, gentle, 3).unwrap().1.is_none(), "a gentle impact set it off");
        let carried = ItemState { in_world: false, impact_speed: 900.0 };
        assert!(advance(&tick, 0.0, 0.01, carried, 3).unwrap().1.is_none(), "a carried bomb went off on impact");
    }
}
