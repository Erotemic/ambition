//! The crumple: a strike that buckles a standing body to the floor.
//!
//! A move authors `smash.crumple` as the on-hit effect of its volumes. When
//! one lands on a body that stands on the ground, that body sags where it
//! stood for the move's time and then collapses prone into the floor game
//! (`ambition.combat.crumple`). The crumple replaces the hit's launch and
//! stun; a body struck in the air keeps its hit. The sag and the collapse are
//! the engine's (`BodyCombat::crumple_timer`); there is no mash, because the
//! getups of the floor game are the escape.
//!
//! The time is the move's (`params` of its effect), and admission checked it
//! (`check_crumple_params`), so a missing one is a fault.

use ambition_combat_port::{Crumple, CrumplePort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a crumple. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const CRUMPLE: &str = "smash.crumple";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "crumple"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: CRUMPLE.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: CRUMPLE.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![CrumplePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(crumple),
        }],
    }
}

/// The crumple a strike's params ask for.
pub fn crumple_of(struck: &Struck) -> Result<Crumple, Fault> {
    let duration_s = struck.number("duration_s").ok_or(Fault::Module("duration_s".into()))?;
    Ok(Crumple { duration_s })
}

fn crumple(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    let sag = crumple_of(&struck)?;
    inv.submit::<CrumplePort>(sag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_crumple_is_the_moves_time_and_a_missing_one_is_a_fault() {
        let struck = |params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker: 7, params };
        assert_eq!(crumple_of(&struck(vec![("duration_s".into(), SpecialParam::Number(0.9))])).unwrap(), Crumple { duration_s: 0.9 });
        assert!(crumple_of(&struck(Vec::new())).is_err());
    }
}
