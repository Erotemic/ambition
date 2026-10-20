//! The remote mine: a stage object that answers to the fighter who placed it
//! and nobody else.
//!
//! One press, two outcomes. When a move authors `smash.place_mine` and its
//! fighter has no mine out, the press places the held item the move names at
//! the move's offset (`ambition.world.spawn_module_item`); any body can pick it
//! up and throw it, and it stays its placer's. When the fighter has one out
//! (`ambition.world.owned_entities`), the press sets it off where it is now,
//! whoever holds it, once it has lived the move's `arm_s`; a press while it is
//! still arming does nothing. One mine per fighter, which makes the arming
//! delay a brake.
//!
//! The mine owns its blast (its handle is its credit), and the blast hurts its
//! placer too: they choose the instant, so standing next to their own mine is
//! not free. The numbers of a blast are the press's, which is the move that
//! placed the mine.

use ambition_combat_port::{
    Blast, BlastPort, EndOwnedEntity, EndOwnedEntityPort, FighterSpecial, FighterSpecialPort, ModuleItemSpawn,
    OwnedEntitiesPort, OwnedEntity, SpawnModuleItemPort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a mine press. The Smash crate installs
/// this module for its catalog's key (`install_module_technique`).
pub const PLACE_MINE: &str = "smash.place_mine";

/// The kind of the module entity a placed mine is.
pub const MINE: &str = "smash.mine";

/// How long a placed mine lives if nobody sets it off: as long as a match
/// lasts, and short enough that a gameplay step still registers on it.
pub const MINE_LIFETIME_S: f32 = 3600.0;

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "mine"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: PLACE_MINE.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: PLACE_MINE.into() },
            reads: vec![OwnedEntitiesPort::KEY],
            writes: Vec::new(),
            requests: vec![SpawnModuleItemPort::KEY, BlastPort::KEY, EndOwnedEntityPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 2 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(press),
        }],
    }
}

/// What one press does.
#[derive(Debug, PartialEq)]
pub enum Press {
    Place(ModuleItemSpawn),
    Detonate { blast: Blast, mine: u64 },
    /// The mine out is still arming.
    Nothing,
}

/// What a press does, given the press and the fighter's own entities. Every
/// number is the move's, which admission checked
/// (`check_place_mine_params`), so a missing one is a fault.
pub fn press_of(special: &FighterSpecial, owned: &[OwnedEntity]) -> Result<Press, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    if let Some(mine) = owned.iter().find(|entity| entity.kind == MINE) {
        if mine.age_s < number("arm_s")? {
            return Ok(Press::Nothing);
        }
        let radius = number("blast_radius")?;
        let blast = Blast {
            center: mine.position,
            half_extent: [radius, radius],
            damage: number("damage")? as i32,
            knockback: number("knockback")?,
            lifetime_s: number("blast_s")?,
            credit: mine.handle,
        };
        return Ok(Press::Detonate { blast, mine: mine.handle });
    }
    let item = special.text("item_id").ok_or(Fault::Module("item_id".into()))?.to_owned();
    Ok(Press::Place(ModuleItemSpawn {
        kind: MINE.into(),
        item,
        at: [
            special.position[0] + number("offset[0]")? * special.facing,
            special.position[1] + number("offset[1]")?,
        ],
        lifetime_s: MINE_LIFETIME_S,
        half_extents: [number("half_extents[0]")?, number("half_extents[1]")?],
        params: Vec::new(),
    }))
}

fn press(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let owned = inv.observe::<OwnedEntitiesPort>()?.entities.clone();
    match press_of(&special, &owned)? {
        Press::Place(spawn) => inv.submit::<SpawnModuleItemPort>(spawn),
        Press::Detonate { blast, mine } => {
            inv.submit::<BlastPort>(blast)?;
            inv.submit::<EndOwnedEntityPort>(EndOwnedEntity { handle: mine })
        }
        Press::Nothing => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special() -> FighterSpecial {
        let params = [
            ("arm_s", 0.5),
            ("damage", 9.0),
            ("blast_radius", 50.0),
            ("half_extents[0]", 6.0),
            ("half_extents[1]", 7.0),
            ("offset[0]", 10.0),
            ("offset[1]", 0.0),
            ("knockback", 1.7),
            ("blast_s", 0.08),
        ]
        .into_iter()
        .map(|(path, n)| (path.to_string(), SpecialParam::Number(n)))
        .chain(std::iter::once(("item_id".to_string(), SpecialParam::Text("polygon_mine".into()))))
        .collect();
        FighterSpecial { position: [100.0, 20.0], facing: -1.0, params }
    }

    fn mine(age_s: f32) -> OwnedEntity {
        OwnedEntity { kind: MINE.into(), position: [40.0, 25.0], age_s, handle: 77 }
    }

    /// No mine out places one; an arming one ignores the press; an armed one
    /// goes off where it is, credited to itself, and is ended; another kind of
    /// the fighter's own is not a mine.
    #[test]
    fn a_press_places_waits_or_detonates() {
        let other = OwnedEntity { kind: "smash.bomb".into(), ..mine(9.0) };
        let Press::Place(spawn) = press_of(&special(), &[other]).unwrap() else {
            panic!("a press with no mine out did not place one");
        };
        assert_eq!((spawn.kind.as_str(), spawn.item.as_str(), spawn.at), (MINE, "polygon_mine", [90.0, 20.0]));
        assert_eq!(press_of(&special(), &[mine(0.49)]).unwrap(), Press::Nothing);
        assert_eq!(
            press_of(&special(), &[mine(0.5)]).unwrap(),
            Press::Detonate {
                blast: Blast {
                    center: [40.0, 25.0],
                    half_extent: [50.0, 50.0],
                    damage: 9,
                    knockback: 1.7,
                    lifetime_s: 0.08,
                    credit: 77
                },
                mine: 77,
            }
        );
    }
}
