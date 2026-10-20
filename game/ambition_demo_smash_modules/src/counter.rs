//! The counter stance: while it is live, its fighter parries.
//!
//! A move authors `smash.counter` as the sustained effect of its stance
//! window, so it fires each tick the stance is live. Each tick, the stance
//! holds its fighter's parry window open for the move's `window_s`
//! (`ambition.combat.parry_window`), and, when the move says so, its
//! projectile absorb too. The window decays, so a stance that stops asking
//! stops parrying.
//!
//! A parry caught while the stance is live answers with the stance's
//! authored technique (`response`), with the part of its params the stance
//! names (`response_params`), on its fighter or, when the stance says
//! `answers_the_attacker`, on the fighter who swung. The engine says which
//! parry a stance caught (`ambition.combat.parried`) and presses what the
//! answer names (`ambition.combat.answer_parry`); a parry into a grab, a
//! riposte and a Witch-Time slow differ only in that data.

use ambition_combat_port::{
    AnswerParry, AnswerParryPort, FighterSpecial, FighterSpecialPort, Parried, ParriedPort, ParryWindow,
    ParryWindowPort,
};
use ambition_extension_sdk::{
    phases::{HIT_REACTION, TECHNIQUE_EXECUTION},
    CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits, ModuleDescriptor, ModuleKey,
    Port, TriggerBinding, API_VERSION,
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
        entries: vec![
            EntryDescriptor {
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
            },
            EntryDescriptor {
                key: ANSWER.into(),
                phase: HIT_REACTION,
                trigger: TriggerBinding { port: ParriedPort::KEY, selector: COUNTER.into() },
                reads: Vec::new(),
                writes: Vec::new(),
                requests: vec![AnswerParryPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 1 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(answer),
            },
        ],
    }
}

/// The entry that answers a parry the stance caught.
pub const ANSWER: &str = "smash.counter_answer";

/// The params path the answer hands on.
const RESPONSE_PARAMS: &str = "response_params";

/// The answer a caught parry gets. A stance that names no response is a
/// fault; one that states no `response_params` hands on none (they are
/// optional in the authored stance), and one that does hands them on whole.
///
/// Written out from a params struct, the response params are a one-element
/// sequence (`response_params[0]`); authored by hand, a table
/// (`response_params`). The answer names the one its leaves carry.
pub fn answer_of(parried: &Parried) -> Result<AnswerParry, Fault> {
    let key = parried.text("response").ok_or(Fault::Module("response".into()))?.to_owned();
    let under = |root: &str| {
        parried.params.iter().any(|(path, _)| {
            path.strip_prefix(root).is_some_and(|rest| rest.is_empty() || rest.starts_with('.') || rest.starts_with('['))
        })
    };
    let wrapped = format!("{RESPONSE_PARAMS}[0]");
    let params_at = if under(&wrapped) {
        wrapped
    } else if under(RESPONSE_PARAMS) {
        RESPONSE_PARAMS.into()
    } else {
        String::new()
    };
    Ok(AnswerParry {
        parry: parried.parry,
        key,
        params_at,
        on_attacker: parried.flag("answers_the_attacker").unwrap_or(false),
    })
}

fn answer(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let parried = inv.trigger::<ParriedPort>()?.clone();
    let answer = answer_of(&parried)?;
    inv.submit::<AnswerParryPort>(answer)
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

    fn parried(params: Vec<(&str, SpecialParam)>) -> Parried {
        Parried {
            parry: 3,
            contact: [0.0, 0.0],
            params: params.into_iter().map(|(path, value)| (path.to_string(), value)).collect(),
        }
    }

    /// The answer is the stance's response, on whom it says, with its
    /// response params when it states them and none when it does not.
    #[test]
    fn the_answer_is_what_the_stance_authors() {
        assert_eq!(
            answer_of(&parried(vec![
                ("response", SpecialParam::Text("smash.time_dilation".into())),
                ("answers_the_attacker", SpecialParam::Flag(true)),
                ("response_params.scale", SpecialParam::Number(0.25)),
            ]))
            .unwrap(),
            AnswerParry { parry: 3, key: "smash.time_dilation".into(), params_at: "response_params".into(), on_attacker: true }
        );
        assert_eq!(
            answer_of(&parried(vec![
                ("response", SpecialParam::Text("smash.riposte_strike".into())),
                // A path that only begins like the params' is not them.
                ("response_params_note", SpecialParam::Text("x".into())),
            ]))
            .unwrap(),
            AnswerParry { parry: 3, key: "smash.riposte_strike".into(), params_at: String::new(), on_attacker: false }
        );
        assert_eq!(
            answer_of(&parried(vec![
                ("response", SpecialParam::Text("smash.riposte_strike".into())),
                ("response_params[0].damage", SpecialParam::Number(11.0)),
            ]))
            .unwrap()
            .params_at,
            "response_params[0]",
            "the params a params struct wrote are named by their index"
        );
        assert!(answer_of(&parried(Vec::new())).is_err(), "a stance with no response is a fault");
    }
}
