//! Time dilation: a body runs on a slower clock for a while.
//!
//! When a move authors `smash.time_dilation`, the body it runs for runs at
//! the move's scale of the world's clock for the move's seconds. A move from
//! its own timeline slows its own fighter. The clerk's Witch-Time slows the
//! attacker, because the counter's answer runs for the attacker
//! (`answers_the_attacker`). Who is slowed is the dispatch's choice; this
//! module decides by how much, for how long, and when it ends.
//!
//! The clock is this module's: a record on the slowed body ([`Clock`]), spent
//! each tick of the status [`DILATED`] on the world's step, so a slow does not
//! stretch its own countdown. A newer dilation replaces the scale and the
//! remainder, so two slows do not multiply, and the end gives the body its
//! own clock back. The countdown is spent before a new dilation is set on the
//! same tick ([`DILATION`] runs after [`TICK`]), so a dilation of N ticks
//! slows N ticks.
//!
//! Both numbers are the move's (`params` of its effect), and admission checked
//! them (`check_time_dilation_params`), so a missing one is a fault and not a
//! slow of nothing.

use ambition_combat_port::{
    FighterSpecial, FighterSpecialPort, HoldStatus, HoldStatusPort, StatusTickPort, TimeScale, TimeScalePort,
};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, record, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, SchemaKey, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for a dilation. The Smash crate installs this
/// module for its catalog's key, and that install panics at build time when
/// the module answers another (`install_module_technique`).
pub const TIME_DILATION: &str = "smash.time_dilation";

/// The status a slowed body holds while its clock runs.
pub const DILATED: &str = "smash.dilated";

/// The entry that spends a slowed body's clock.
const TICK: &str = "smash.dilated_tick";

/// The entry that starts (or replaces) a dilation.
const DILATION: &str = TIME_DILATION;

record! {
    /// What is left of a body's dilation.
    pub struct Clock = SchemaKey::new(crate::PROVIDER, "dilation.clock", 1);
    /// World seconds left.
    1 remaining_s: f32,
}

pub fn module() -> ModuleDescriptor {
    let key = ModuleKey::new(crate::PROVIDER, "dilation");
    let tick_path = format!("{key}/{TICK}");
    ModuleDescriptor {
        key,
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: vec![Clock::schema()],
        entries: vec![
            EntryDescriptor {
                key: TICK.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: StatusTickPort::KEY, selector: DILATED.into() },
                reads: Vec::new(),
                writes: vec![Clock::KEY],
                requests: vec![TimeScalePort::KEY, HoldStatusPort::KEY],
                after: Vec::new(),
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(spend),
            },
            EntryDescriptor {
                key: DILATION.into(),
                phase: TECHNIQUE_EXECUTION,
                trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: TIME_DILATION.into() },
                reads: Vec::new(),
                writes: vec![Clock::KEY],
                requests: vec![TimeScalePort::KEY, HoldStatusPort::KEY],
                after: vec![tick_path.into()],
                limits: Limits { max_requests: 2 },
                on_idle: IdlePolicy::Invoke,
                run: EntryCode::Native(dilate),
            },
        ],
    }
}

/// A dilation a move's params ask for: its scale and its seconds. Only a slow
/// with an end is a dilation (`0 <= scale < 1`, seconds more than 0).
pub fn dilation_of(special: &FighterSpecial) -> Result<(f32, f32), Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    let (scale, seconds) = (number("scale")?, number("seconds")?);
    if !(0.0..1.0).contains(&scale) || !(seconds.is_finite() && seconds > 0.0) {
        return Err(Fault::Module(format!("a dilation of scale {scale} for {seconds} s is not a slow with an end").into()));
    }
    Ok((scale, seconds))
}

fn status(held: bool) -> HoldStatus {
    HoldStatus { kind: DILATED.into(), held }
}

/// Start a dilation, or replace the one the body has.
fn dilate(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let (scale, seconds) = dilation_of(&special)?;
    Clock { remaining_s: seconds }.store(inv)?;
    inv.submit::<TimeScalePort>(TimeScale { scale })?;
    inv.submit::<HoldStatusPort>(status(true))
}

/// Spend a tick of the world's step, and give the body its own clock back
/// when the dilation runs out.
fn spend(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let mut clock = Clock::load(inv)?;
    clock.remaining_s -= inv.dt();
    if clock.remaining_s > 0.0 {
        return clock.store(inv);
    }
    Clock::default().store(inv)?;
    inv.submit::<TimeScalePort>(TimeScale { scale: 1.0 })?;
    inv.submit::<HoldStatusPort>(status(false))
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

    /// The dilation is the move's scale and time; one with no time, a
    /// speed-up and a slow with no end are faults.
    #[test]
    fn the_dilation_is_what_the_move_authors() {
        assert_eq!(dilation_of(&special(&[("scale", 0.35), ("seconds", 0.45)])).unwrap(), (0.35, 0.45));
        assert!(dilation_of(&special(&[("scale", 0.35)])).is_err(), "a dilation with no time is a fault");
        assert!(dilation_of(&special(&[("scale", 2.0), ("seconds", 0.45)])).is_err(), "a speed-up is a fault");
        assert!(dilation_of(&special(&[("scale", 0.35), ("seconds", 0.0)])).is_err(), "a slow with no end is a fault");
    }
}
