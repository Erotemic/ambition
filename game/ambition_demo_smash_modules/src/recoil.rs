//! Recoil: a strike that costs the body that struck.
//!
//! A move authors `smash.recoil` as the on-hit effect of its volumes. Each
//! time one lands, the body whose move struck pays the move's amount
//! (`ambition.combat.recoil`): a price, not an injury, so its percent rises
//! and its pool never falls below 1, and it is not stunned or credited to
//! anybody. The struck body keeps its hit. The striker is named by the
//! strike's credit (`Struck::striker`); a striker with no credit cannot be
//! named, and pays nothing.
//!
//! The amount is the move's (`params` of its effect), and admission checked it
//! (`check_recoil_params`), so a missing one is a fault.

use ambition_combat_port::{Recoil, RecoilPort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a recoil. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const RECOIL: &str = "smash.recoil";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "recoil"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: RECOIL.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: RECOIL.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![RecoilPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(recoil),
        }],
    }
}

/// The recoil a strike asks for: the move's amount, from the striker. `None`
/// for a striker with no credit.
pub fn recoil_of(struck: &Struck) -> Result<Option<Recoil>, Fault> {
    let amount = struck.number("amount").ok_or(Fault::Module("amount".into()))?;
    Ok((struck.striker != 0).then_some(Recoil { body: struck.striker, amount: amount as i32 }))
}

fn recoil(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    match recoil_of(&struck)? {
        Some(cost) => inv.submit::<RecoilPort>(cost),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_striker_pays_the_moves_amount() {
        let struck = |striker: u64, params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker, params };
        let amount = || vec![("amount".into(), SpecialParam::Number(3.0))];
        assert_eq!(recoil_of(&struck(7, amount())).unwrap(), Some(Recoil { body: 7, amount: 3 }));
        assert_eq!(recoil_of(&struck(0, amount())).unwrap(), None, "a striker nobody can name paid");
        assert!(recoil_of(&struck(7, Vec::new())).is_err());
    }
}
