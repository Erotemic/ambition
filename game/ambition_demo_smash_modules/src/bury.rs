//! The burial: a strike that drives a standing body into the floor.
//!
//! A move authors `smash.bury` as the on-hit effect of its volumes. When one
//! lands on a body that stands on the ground, that body is buried for the
//! move's time (`ambition.combat.bury`): held in the floor, it cannot act,
//! and the burial replaces the hit's launch and stun. A body struck in the
//! air keeps the hit as it was, so a spike stays a spike. The burial itself
//! is the engine's (`BodyCombat::bury_timer`); how a buried body digs out is
//! the game's mash rule, not the move's.
//!
//! The time is the move's (`params` of its effect), and admission checked it
//! (`check_bury_params`), so a missing one is a fault.

use ambition_combat_port::{Bury, BuryPort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a burial. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const BURY: &str = "smash.bury";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "bury"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: BURY.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: BURY.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![BuryPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(bury),
        }],
    }
}

/// The burial a strike's params ask for.
pub fn burial_of(struck: &Struck) -> Result<Bury, Fault> {
    let duration_s = struck.number("duration_s").ok_or(Fault::Module("duration_s".into()))?;
    Ok(Bury { duration_s })
}

fn bury(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    let burial = burial_of(&struck)?;
    inv.submit::<BuryPort>(burial)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_burial_is_the_moves_time_and_a_missing_one_is_a_fault() {
        let struck = |params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker: 7, params };
        assert_eq!(burial_of(&struck(vec![("duration_s".into(), SpecialParam::Number(0.9))])).unwrap(), Bury { duration_s: 0.9 });
        assert!(burial_of(&struck(Vec::new())).is_err());
    }
}
