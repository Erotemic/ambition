//! The freeze: a strike that encases the body it struck.
//!
//! A move authors `smash.freeze` as the on-hit effect of its volumes. When
//! one lands, the struck body is encased for the move's time
//! (`ambition.combat.freeze`): it cannot act or steer, but it is not held:
//! the hit launches the block as usual, and it falls and slides. The freeze
//! itself is the engine's (`BodyCombat::freeze_timer`); how a frozen body
//! thaws is the game's mash rule, not the move's.
//!
//! The time is the move's (`params` of its effect), and admission checked it
//! (`check_freeze_params`), so a missing one is a fault.

use ambition_combat_port::{Freeze, FreezePort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a freeze. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const FREEZE: &str = "smash.freeze";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "freeze"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: FREEZE.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: FREEZE.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![FreezePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(freeze),
        }],
    }
}

/// The freeze a strike's params ask for.
pub fn freeze_of(struck: &Struck) -> Result<Freeze, Fault> {
    let duration_s = struck.number("duration_s").ok_or(Fault::Module("duration_s".into()))?;
    Ok(Freeze { duration_s })
}

fn freeze(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    let ice = freeze_of(&struck)?;
    inv.submit::<FreezePort>(ice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_freeze_is_the_moves_time_and_a_missing_one_is_a_fault() {
        let struck = |params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker: 7, params };
        assert_eq!(freeze_of(&struck(vec![("duration_s".into(), SpecialParam::Number(0.9))])).unwrap(), Freeze { duration_s: 0.9 });
        assert!(freeze_of(&struck(Vec::new())).is_err());
    }
}
