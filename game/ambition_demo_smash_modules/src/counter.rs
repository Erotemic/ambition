//! The counter stance: while it is live, its fighter parries.
//!
//! A move authors `smash.counter` as the sustained effect of its stance
//! window, so it fires each tick the stance is live. Each tick, the stance
//! holds its fighter's parry window open for the move's `window_s`
//! (`ambition.combat.parry_window`), and, when the move says so, its
//! projectile absorb too. The window decays, so a stance that stops asking
//! stops parrying.
//!
//! What a caught parry answers with is the stance's authored data
//! (`response`, `response_params`, `answers_the_attacker`), which the engine
//! answers (`ambition_combat::counter`).

use ambition_combat_port::{FighterSpecial, FighterSpecialPort, ParryWindow, ParryWindowPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a stance authors. The Smash crate installs this module for
/// its catalog's key (`install_module_technique`).
pub const COUNTER: &str = "smash.counter";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "counter"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: COUNTER.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: COUNTER.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![ParryWindowPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(stance),
        }],
    }
}

/// The window a stance's params ask for. A stance that does not say whether
/// it absorbs does not.
pub fn window_of(special: &FighterSpecial) -> Result<ParryWindow, Fault> {
    let window_s = special.number("window_s").ok_or(Fault::Module("window_s".into()))?;
    Ok(ParryWindow { window_s, absorbs_projectiles: special.flag("absorbs_projectiles").unwrap_or(false) })
}

fn stance(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let window = window_of(&special)?;
    inv.submit::<ParryWindowPort>(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    /// The window is the stance's, with its absorb; one with no window is a
    /// fault.
    #[test]
    fn the_window_is_what_the_stance_authors() {
        let special = |params: Vec<(&str, SpecialParam)>| FighterSpecial {
            position: [0.0, 0.0],
            facing: 1.0,
            params: params.into_iter().map(|(path, value)| (path.to_string(), value)).collect(),
        };
        assert_eq!(
            window_of(&special(vec![
                ("window_s", SpecialParam::Number(0.05)),
                ("absorbs_projectiles", SpecialParam::Flag(true)),
            ]))
            .unwrap(),
            ParryWindow { window_s: 0.05, absorbs_projectiles: true }
        );
        assert!(window_of(&special(Vec::new())).is_err(), "a stance with no window is a fault");
    }
}
