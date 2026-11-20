//! The paralysis: a strike that holds the body it struck.
//!
//! A move authors `smash.paralyze` as the on-hit effect of its volumes. When
//! one lands, the struck body is held where it was for the move's time
//! (`ambition.combat.paralyze`): it cannot act or move, and the stun of the
//! hit waits for the end. The hold itself is the engine's
//! (`BodyCombat::paralysis_timer`); how a paralyzed body buys time back is the
//! game's mash rule, not the move's.
//!
//! The time is the move's (`params` of its effect), and admission checked it
//! (`check_paralyze_params`), so a missing one is a fault and not a paralysis
//! of nothing.

use ambition_combat_port::{Paralyze, ParalyzePort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a paralysis. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const PARALYZE: &str = "smash.paralyze";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "paralyze"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: PARALYZE.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: PARALYZE.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![ParalyzePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(paralyze),
        }],
    }
}

/// The paralysis a strike's params ask for.
pub fn paralysis_of(struck: &Struck) -> Result<Paralyze, Fault> {
    let duration_s = struck.number("duration_s").ok_or(Fault::Module("duration_s".into()))?;
    Ok(Paralyze { duration_s })
}

fn paralyze(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    let paralysis = paralysis_of(&struck)?;
    inv.submit::<ParalyzePort>(paralysis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_paralysis_is_the_moves_time_and_a_missing_one_is_a_fault() {
        let struck = |params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker: 7, params };
        let held = paralysis_of(&struck(vec![("duration_s".into(), SpecialParam::Number(0.5))])).unwrap();
        assert_eq!(held, Paralyze { duration_s: 0.5 });
        assert!(paralysis_of(&struck(Vec::new())).is_err());
    }
}
