//! Lower catalog action presets into prepared action sets.

use crate::actor::character_catalog::{
    ActionSetPreset, MeleePreset, MoveStylePreset, RangedPreset, SpecialPreset,
};
use crate::brain::action_set::{
    ActionSet, BiteSpec, LungeSpec, MeleeActionSpec, MoveStyleSpec, PunchSpec, RangedActionSpec,
    SlamSpec, SpecialActionSpec, SwipeSpec,
};


/// Build a runtime [`ActionSet`] from a preset.
#[allow(
    dead_code,
    reason = "Public resolver API used by future spawn-site catalog consumers; today exercised via tests."
)]
pub fn action_set_from_preset(preset: &ActionSetPreset) -> ActionSet {
    ActionSet {
        move_style: move_style_from_preset(preset.move_style),
        melee: preset.melee.map(melee_from_preset),
        ranged: preset.ranged.clone().map(ranged_from_preset),
        special: preset.special.clone().map(special_from_preset),
    }
}

fn move_style_from_preset(p: MoveStylePreset) -> MoveStyleSpec {
    match p {
        MoveStylePreset::Walk => MoveStyleSpec::Walk,
        MoveStylePreset::WalkHeavy => MoveStyleSpec::WalkHeavy,
        MoveStylePreset::Hop => MoveStyleSpec::Hop,
        MoveStylePreset::Strafe => MoveStyleSpec::Strafe,
        MoveStylePreset::Slither => MoveStyleSpec::Slither,
        MoveStylePreset::Float => MoveStyleSpec::Float,
    }
}

fn melee_from_preset(p: MeleePreset) -> MeleeActionSpec {
    match p {
        MeleePreset::Swipe {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        } => MeleeActionSpec::Swipe(SwipeSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        }),
        MeleePreset::Lunge {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
            step_px,
            rush_px,
        } => MeleeActionSpec::Lunge(LungeSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
            step_px,
            rush_px,
        }),
        MeleePreset::Slam {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        } => MeleeActionSpec::Slam(SlamSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        }),
        MeleePreset::Bite {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        } => MeleeActionSpec::Bite(BiteSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        }),
        MeleePreset::PunchWeak {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        } => MeleeActionSpec::PunchWeak(PunchSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
        }),
    }
}

fn ranged_from_preset(p: RangedPreset) -> RangedActionSpec {
    match p {
        RangedPreset::Rock { speed, damage } => RangedActionSpec::rock(speed, damage),
        RangedPreset::Arrow { speed, damage } => RangedActionSpec::arrow(speed, damage),
        RangedPreset::Pistol { speed, damage } => RangedActionSpec::pistol(speed, damage),
        RangedPreset::Bolt { speed, damage } => RangedActionSpec::bolt(speed, damage),
        RangedPreset::Weapon(spec) => spec,
    }
}

fn special_from_preset(p: SpecialPreset) -> SpecialActionSpec {
    match p {
        SpecialPreset::Special(key) => SpecialActionSpec::Special(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_set_preset_can_author_open_special_key() {
        let preset: ActionSetPreset = ron::from_str(
            r#"(
                special: Some(Special("eye_beam")),
            )"#,
        )
        .expect("catalog action-set presets should deserialize open Special keys");

        let action_set = action_set_from_preset(&preset);

        assert_eq!(
            action_set.special,
            Some(SpecialActionSpec::Special("eye_beam".to_string())),
            "catalog presets must reach the same open content-special seam as runtime action sets"
        );
    }
}
