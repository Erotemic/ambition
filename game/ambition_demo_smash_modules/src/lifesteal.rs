//! Lifesteal: a strike that mends the body that struck.
//!
//! A move authors `smash.lifesteal` as the on-hit effect of its volumes. Each
//! time one lands, the body whose move struck mends the move's amount
//! (`ambition.combat.mend`): its health and its damage meter, so in a match
//! it is the striker's percent that falls. The struck body keeps its hit.
//! The striker is named by the strike's credit (`Struck::striker`); a striker
//! with no credit cannot be named, and mends nothing.
//!
//! The amount is the move's (`params` of its effect), and admission checked it
//! (`check_lifesteal_params`), so a missing one is a fault.

use ambition_combat_port::{Mend, MendPort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a lifesteal. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const LIFESTEAL: &str = "smash.lifesteal";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "lifesteal"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: LIFESTEAL.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: LIFESTEAL.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![MendPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(lifesteal),
        }],
    }
}

/// The mend a strike asks for: the move's amount, to the striker. `None` for
/// a striker with no credit, which no request can name.
pub fn mend_of(struck: &Struck) -> Result<Option<Mend>, Fault> {
    let amount = struck.number("amount").ok_or(Fault::Module("amount".into()))?;
    Ok((struck.striker != 0).then_some(Mend { body: struck.striker, amount: amount as i32 }))
}

fn lifesteal(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    match mend_of(&struck)? {
        Some(mend) => inv.submit::<MendPort>(mend),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_striker_mends_the_moves_amount() {
        let struck = |striker: u64, params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker, params };
        let amount = || vec![("amount".into(), SpecialParam::Number(4.0))];
        assert_eq!(mend_of(&struck(7, amount())).unwrap(), Some(Mend { body: 7, amount: 4 }));
        assert_eq!(mend_of(&struck(0, amount())).unwrap(), None, "a striker nobody can name was mended");
        assert!(mend_of(&struck(7, Vec::new())).is_err());
    }
}
