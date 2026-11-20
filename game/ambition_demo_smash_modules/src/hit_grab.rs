//! The hit-grab: a strike that seizes the body it struck.
//!
//! A move authors `smash.hit_grab` as the on-hit effect of its volumes. When
//! one lands, the body whose move struck grabs the struck body
//! (`ambition.combat.seize_struck`) and holds it at the move's hold point,
//! as its grab would: from there the hold is an ordinary grab (pummel, throw,
//! mash free). The seize replaces the hit's launch and stun. It is judged by
//! every rule a grab is but the reach, so a body in the air keeps its hit.
//! A shield that blocks the strike blocks the grab: a blocked strike struck
//! nothing.
//!
//! The striker is named by the strike's credit (`Struck::striker`); a striker
//! with no credit cannot be named, and seizes nothing. The hold point is the
//! move's (`params` of its effect), which admission checked
//! (`check_hit_grab_params`), so a missing one is a fault.

use ambition_combat_port::{SeizeStruck, SeizeStruckPort, Struck, StruckPort};
use ambition_extension_sdk::{
    phases::HIT_REACTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The on-hit effect key a move authors for a hit-grab. The Smash crate
/// installs this module for its catalog's key (`install_module_technique`).
pub const HIT_GRAB: &str = "smash.hit_grab";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "hit_grab"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            build: None,
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            replies: Vec::new(),
            key: HIT_GRAB.into(),
            phase: HIT_REACTION,
            trigger: TriggerBinding { port: StruckPort::KEY, selector: HIT_GRAB.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![SeizeStruckPort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(hit_grab),
        }],
    }
}

/// The seize a strike asks for: the striker holds the struck body at the
/// move's hold point. `None` for a striker with no credit.
pub fn seize_of(struck: &Struck) -> Result<Option<SeizeStruck>, Fault> {
    let number = |path: &'static str| struck.number(path).ok_or(Fault::Module(path.into()));
    let hold_offset = [number("hold_offset[0]")?, number("hold_offset[1]")?];
    Ok((struck.striker != 0).then_some(SeizeStruck { striker: struck.striker, hold_offset }))
}

fn hit_grab(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let struck = inv.trigger::<StruckPort>()?.clone();
    match seize_of(&struck)? {
        Some(seize) => inv.submit::<SeizeStruckPort>(seize),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    #[test]
    fn the_striker_seizes_at_the_moves_hold_point() {
        let struck = |striker: u64, params: Vec<(String, SpecialParam)>| Struck { contact: [0.0, 0.0], striker, params };
        let hold = || vec![("hold_offset[0]".into(), SpecialParam::Number(18.0)), ("hold_offset[1]".into(), SpecialParam::Number(-4.0))];
        assert_eq!(seize_of(&struck(7, hold())).unwrap(), Some(SeizeStruck { striker: 7, hold_offset: [18.0, -4.0] }));
        assert_eq!(seize_of(&struck(0, hold())).unwrap(), None, "a striker nobody can name seized");
        assert!(seize_of(&struck(7, Vec::new())).is_err());
    }
}
