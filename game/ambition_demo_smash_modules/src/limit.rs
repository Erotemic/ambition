//! A move that fills its own fighter's Limit meter.
//!
//! When a move authors `smash.fill_meter`, the fighter it runs for gains the
//! move's amount of `smash.limit` (`ambition.resources.refill`). A fighter
//! that holds no Limit gains nothing. The other sources of the meter (time,
//! damage, blocks) are the match's rule, not a move's, and stay in the game.
//!
//! The amount is the move's (`params` of its effect), and admission checked
//! that it hydrates, so a missing one is a fault and not a fill of nothing.

use ambition_combat_port::{FighterSpecial, FighterSpecialPort, RefillResource, RefillResourcePort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a fill. The Smash crate installs this
/// module for its catalog's key (`install_module_technique`).
pub const FILL_METER: &str = "smash.fill_meter";

/// The resource a fill adds to: the Limit's id (`smash_limit::LIMIT`).
pub const LIMIT: &str = "smash.limit";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "limit"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: FILL_METER.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: FILL_METER.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![RefillResourcePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(fill),
        }],
    }
}

/// The refill a move's params ask for.
pub fn refill_of(special: &FighterSpecial) -> Result<RefillResource, Fault> {
    let amount = special.number("amount").ok_or(Fault::Module("amount".into()))?;
    Ok(RefillResource { resource: LIMIT.into(), amount })
}

fn fill(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let refill = refill_of(&special)?;
    inv.submit::<RefillResourcePort>(refill)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    /// The fill is the move's amount of the Limit; one with no amount is a
    /// fault.
    #[test]
    fn the_fill_is_what_the_move_authors() {
        let special = |params: Vec<(String, SpecialParam)>| FighterSpecial { position: [0.0, 0.0], facing: 1.0, params };
        assert_eq!(
            refill_of(&special(vec![("amount".into(), SpecialParam::Number(12.0))])).unwrap(),
            RefillResource { resource: LIMIT.into(), amount: 12.0 }
        );
        assert!(refill_of(&special(Vec::new())).is_err(), "a fill with no amount is a fault");
    }
}
