//! The song: a pulse that puts everyone near the singer to sleep.
//!
//! When a fighter sings (`smash.sleep`), every other body of its live room
//! inside the move's area falls asleep for the move's time
//! (`ambition.combat.sleep_area`). The singer is never caught by their own
//! song: the move puts everyone else to sleep so the singer can act. How a
//! sleeper buys time back is the game's mash rule, not the song's.
//!
//! Each number is the move's (`params` of its `smash.sleep` effect), and
//! admission checked them (`check_sleep_params`), so a missing one is a fault
//! and not a song that reaches nobody.

use ambition_combat_port::{FighterSpecial, FighterSpecialPort, SleepArea, SleepAreaPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for the song. The Smash crate installs this
/// module for its catalog's key, and that install panics at build time when
/// the module answers another (`install_module_technique`).
pub const SLEEP: &str = "smash.sleep";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "sing"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: SLEEP.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: SLEEP.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![SleepAreaPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(sing),
        }],
    }
}

/// The pulse a song's params ask for: centred on the singer, so its facing
/// does not matter.
pub fn pulse_of(special: &FighterSpecial) -> Result<SleepArea, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    Ok(SleepArea {
        forward: [0.0, 0.0],
        half_extent: [number("half_extents[0]")?, number("half_extents[1]")?],
        duration_s: number("duration_s")?,
    })
}

fn sing(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let pulse = pulse_of(&special)?;
    inv.submit::<SleepAreaPort>(pulse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special(params: &[(&str, f32)]) -> FighterSpecial {
        FighterSpecial {
            position: [0.0, 0.0],
            facing: -1.0,
            params: params.iter().map(|(path, n)| (path.to_string(), SpecialParam::Number(*n))).collect(),
        }
    }

    /// The pulse is the move's: its area around the singer and its time; and
    /// a song with no time is a fault.
    #[test]
    fn the_pulse_is_what_the_move_authors() {
        let mut params = vec![("duration_s", 1.4), ("half_extents[0]", 70.0), ("half_extents[1]", 40.0)];
        assert_eq!(
            pulse_of(&special(&params)).unwrap(),
            SleepArea { forward: [0.0, 0.0], half_extent: [70.0, 40.0], duration_s: 1.4 }
        );
        params.remove(0);
        assert!(pulse_of(&special(&params)).is_err(), "a song with no time is a fault");
    }
}
