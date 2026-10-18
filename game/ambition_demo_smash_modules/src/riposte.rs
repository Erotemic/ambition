//! The answering cut: a parry's response that hits back.
//!
//! When a fighter answers a parry (`smash.riposte_strike`), it cuts in front of
//! itself at the reach its move authors. The cut is the fighter's own strike
//! (`ambition.combat.body_strike`): it follows the fighter for its lifetime,
//! the fighter is not hit, and the fighter's side decides whom it hurts.
//!
//! Each number is the move's (`params` of its `smash.riposte_strike` effect), and
//! admission checked them (`check_riposte_strike_params`), so a missing one is
//! a fault and not a cut that does nothing.

use ambition_combat_port::{BodyStrike, BodyStrikePort, FighterSpecial, FighterSpecialPort};
use ambition_extension_sdk::{
    phases::TECHNIQUE_EXECUTION, CodeIdentity, EntryCode, EntryDescriptor, Fault, IdlePolicy, Invocation, Limits,
    ModuleDescriptor, ModuleKey, Port, TriggerBinding, API_VERSION,
};

/// The effect key a move authors for the cut. The Smash crate installs this
/// module for its catalog's key, and that install panics at build time when
/// the module answers another (`install_module_technique`).
pub const RIPOSTE_STRIKE: &str = "smash.riposte_strike";

pub fn module() -> ModuleDescriptor {
    ModuleDescriptor {
        key: ModuleKey::new(crate::PROVIDER, "riposte"),
        api: API_VERSION,
        code: CodeIdentity::StaticNative {
            crate_name: env!("CARGO_PKG_NAME").into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        schemas: Vec::new(),
        entries: vec![EntryDescriptor {
            key: RIPOSTE_STRIKE.into(),
            phase: TECHNIQUE_EXECUTION,
            trigger: TriggerBinding { port: FighterSpecialPort::KEY, selector: RIPOSTE_STRIKE.into() },
            reads: Vec::new(),
            writes: Vec::new(),
            requests: vec![BodyStrikePort::KEY],
            after: Vec::new(),
            limits: Limits { max_requests: 1 },
            on_idle: IdlePolicy::Invoke,
            run: EntryCode::Native(cut),
        }],
    }
}

/// The cut a riposte's params ask for.
pub fn cut_of(special: &FighterSpecial) -> Result<BodyStrike, Fault> {
    let number = |path: &'static str| special.number(path).ok_or(Fault::Module(path.into()));
    Ok(BodyStrike {
        forward: [number("reach")?, 0.0],
        half_extent: [number("half_extents[0]")?, number("half_extents[1]")?],
        damage: number("damage")? as i32,
        knockback: number("knockback")?,
        lifetime_s: number("lifetime_s")?,
        hit_sound: special.text("hit_sfx").map(str::to_owned),
    })
}

fn cut(inv: &mut Invocation<'_>) -> Result<(), Fault> {
    let special = inv.trigger::<FighterSpecialPort>()?.clone();
    let strike = cut_of(&special)?;
    inv.submit::<BodyStrikePort>(strike)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_combat_port::SpecialParam;

    fn special(params: &[(&str, SpecialParam)]) -> FighterSpecial {
        FighterSpecial {
            position: [0.0, 0.0],
            facing: 1.0,
            params: params.iter().map(|(path, value)| (path.to_string(), value.clone())).collect(),
        }
    }

    /// The cut is the move's: its reach ahead, its box, damage, knockback,
    /// lifetime and sound; and a move that states no sound leaves the hit
    /// to the victim's own.
    #[test]
    fn the_cut_is_what_the_move_authors() {
        use SpecialParam::{Number, Text};
        let mut params = vec![
            ("damage", Number(9.0)),
            ("half_extents[0]", Number(18.0)),
            ("half_extents[1]", Number(12.0)),
            ("knockback", Number(1.4)),
            ("lifetime_s", Number(0.15)),
            ("reach", Number(30.0)),
            ("hit_sfx", Text("player.slash".to_string())),
        ];
        assert_eq!(
            cut_of(&special(&params)).unwrap(),
            BodyStrike {
                forward: [30.0, 0.0],
                half_extent: [18.0, 12.0],
                damage: 9,
                knockback: 1.4,
                lifetime_s: 0.15,
                hit_sound: Some("player.slash".to_string()),
            }
        );
        params.pop();
        assert_eq!(cut_of(&special(&params)).unwrap().hit_sound, None);
        params.retain(|(path, _)| *path != "reach");
        assert!(cut_of(&special(&params)).is_err(), "a cut with no reach is a fault");
    }
}
