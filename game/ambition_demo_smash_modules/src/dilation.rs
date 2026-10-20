//! Time dilation: a body runs on a slower clock for a while.
//!
//! When a move authors `smash.time_dilation`, the body it runs for runs at
//! the move's scale of the world's clock for the move's seconds
//! (`ambition.combat.dilate_time`). A move from its own timeline slows its own
//! fighter. The clerk's Witch-Time slows the attacker, because the counter's
//! answer runs for the attacker (`answers_the_attacker`). Who is slowed is the
//! dispatch's choice; this module decides only by how much and for how long.
//!
//! Both numbers are the move's (`params` of its effect), and admission checked
//! them (`check_time_dilation_params`), so a missing one is a fault and not a
//! slow of nothing.

use ambition_combat_port::{DilateTime, DilateTimePort, FighterSpecial, FighterSpecialPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a dilation. The Smash crate installs this
/// module for its catalog's key, and that install panics at build time when
/// the module answers another (`install_module_technique`).
pub const TIME_DILATION: &str = "smash.time_dilation";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "dilation"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: TIME_DILATION.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: TIME_DILATION.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![DilateTimePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(dilate),
        }],
    }
}

/// The dilation a move's params ask for.
pub fn dilation_of(special: &FighterSpecial) -> Result<DilateTime, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    Ok(DilateTime { scale: number("scale")?, seconds: number("seconds")? })
}

fn dilate(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let dilation = dilation_of(&special)?;
    inv.submit::<DilateTimePort>(dilation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special(params: &[(&str, f32)]) -> FighterSpecial {
        FighterSpecial {
            position: [0.0, 0.0],
            facing: 1.0,
            params: params.iter().map(|(path, n)| (path.to_string(), SpecialParam::Number(*n))).collect(),
        }
    }

    /// The dilation is the move's scale and time; one with no time is a fault.
    #[test]
    fn the_dilation_is_what_the_move_authors() {
        assert_eq!(
            dilation_of(&special(&[("scale", 0.35), ("seconds", 0.45)])).unwrap(),
            DilateTime { scale: 0.35, seconds: 0.45 }
        );
        assert!(dilation_of(&special(&[("scale", 0.35)])).is_err(), "a dilation with no time is a fault");
    }
}
