//! Convert catalog preset shapes into runtime `Brain` / `ActionSet`
//! values.
//!
//! The catalog stores preset *cfg* values; the runtime `Brain` enum
//! variants pair a cfg with a per-actor `state`. Resolver functions
//! construct the runtime value with a fresh `default()` state.
//!
//! Phase 2 introduces per-spawn overrides; the resolver signature is
//! shaped to accept those as an additional argument when that lands.

use crate::brain::action_set::{
    ActionSet, BiteSpec, LungeSpec, MeleeActionSpec, MoveStyleSpec, PunchSpec, RangedActionSpec,
    SlamSpec, SpecialActionSpec, SwipeSpec,
};
use crate::brain::state_machine::{
    AuthoredWorldPatrolLane, MeleeBruteCfg, MeleeBruteState, PatrolCfg, PatrolState, SkirmisherCfg,
    SkirmisherState, SniperCfg, SniperState, StateMachineCfg, WandererCfg, WandererState,
};
use crate::brain::Brain;
use crate::brain::{BossPatternCfg, BossPatternState};

use super::entry::{
    ActionSetPreset, BrainPreset, MeleePreset, MoveStylePreset, RangedPreset, SpecialPreset,
};

/// Build a runtime [`Brain`] from a preset. `spawn_world_x` is the
/// NPC's actual spawn position in world coordinates; it is added to
/// `Patrol.spawn_local_x` to derive the patrol center. For non-patrol
/// brains it is ignored.
#[allow(
    dead_code,
    reason = "Public resolver API used by future spawn-site catalog consumers; today exercised via tests."
)]
pub fn brain_from_preset(preset: &BrainPreset, spawn_world_x: f32) -> Brain {
    let cfg = match preset {
        BrainPreset::StandStill => return Brain::StateMachine(StateMachineCfg::StandStill),
        BrainPreset::Patrol {
            spawn_local_x,
            radius,
            speed,
            aggressiveness,
            aggro_radius,
            attack_range,
        } => StateMachineCfg::Patrol {
            cfg: PatrolCfg {
                lane: AuthoredWorldPatrolLane::new(spawn_world_x + spawn_local_x, *radius),
                speed: *speed,
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
            },
            state: PatrolState::default(),
        },
        BrainPreset::Wanderer {
            speed,
            climb_walls,
            chatter_threshold,
            chatter_window_s,
            chatter_pause_s,
            aggressiveness,
        } => StateMachineCfg::Wanderer {
            cfg: WandererCfg {
                speed: *speed,
                climb_walls: *climb_walls,
                chatter_threshold: *chatter_threshold,
                chatter_window_s: *chatter_window_s,
                chatter_pause_s: *chatter_pause_s,
                aggressiveness: *aggressiveness,
            },
            state: WandererState::default(),
        },
        BrainPreset::MeleeBrute {
            aggressiveness,
            aggro_radius,
            attack_range,
            chase_speed,
        } => StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg {
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
                chase_speed: *chase_speed,
            },
            state: MeleeBruteState::default(),
        },
        BrainPreset::Skirmisher {
            aggressiveness,
            aggro_radius,
            standoff_px,
            strafe_speed,
            fire_cooldown_s,
        } => StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg {
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                standoff_px: *standoff_px,
                strafe_speed: *strafe_speed,
                fire_cooldown_s: *fire_cooldown_s,
                // Default orbit drift — slow circle (~10s lap). The
                // per-archetype variation lives in
                // `enemy_default_brain`; this character-catalog path
                // is the data-driven NPC spawn helper and uses a
                // sensible fallback.
                orbit_drift_rad_s: 0.6,
            },
            state: SkirmisherState::default(),
        },
        BrainPreset::Sniper {
            aggressiveness,
            aggro_radius,
            fire_cooldown_s,
        } => StateMachineCfg::Sniper {
            cfg: SniperCfg {
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                fire_cooldown_s: *fire_cooldown_s,
            },
            state: SniperState::default(),
        },
        BrainPreset::Aerial {
            aggressiveness,
            cruise_speed,
            dive_speed,
            aggro_radius,
            attack_range,
            roam_radius,
        } => StateMachineCfg::Aerial {
            cfg: crate::brain::state_machine::AerialCfg {
                aggressiveness: *aggressiveness,
                cruise_speed: *cruise_speed,
                dive_speed: *dive_speed,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
                roam_radius: *roam_radius,
            },
            state: crate::brain::state_machine::AerialState::default(),
        },
        BrainPreset::BossPattern {
            aggressiveness,
            encounter_id,
        } => StateMachineCfg::BossPattern {
            cfg: {
                // Catalog-built preview brains use the neutral test
                // cfg + the authored preset's encounter_id /
                // aggressiveness. Real spawn-time bosses build their
                // full `BossPatternCfg` (pattern, movement, spawn,
                // combat_size, cycle timings) in `spawn.rs::spawn_boss`
                // from `BossBehaviorProfile`; the preview path here
                // is for character-catalog displays where there is
                // no live boss runtime to read from.
                let mut cfg = BossPatternCfg::neutral_test();
                cfg.aggressiveness = *aggressiveness;
                cfg.encounter_id = encounter_id.clone();
                cfg
            },
            state: BossPatternState::default(),
        },
        BrainPreset::Smash {
            aggro_radius,
            engage_distance,
            attack_range,
            too_close_distance,
            chase_speed,
            retreat_speed,
            crowding_threshold,
            dash_to_close,
            reaction_delay_s,
            commit_probability,
            accuracy,
            mash_speed_hz,
        } => StateMachineCfg::Smash {
            cfg: crate::brain::smash::SmashCfg {
                aggro_radius: *aggro_radius,
                engage_distance: *engage_distance,
                attack_range: *attack_range,
                too_close_distance: *too_close_distance,
                chase_speed: *chase_speed,
                retreat_speed: *retreat_speed,
                crowding_threshold: *crowding_threshold,
                dash_to_close: *dash_to_close,
                difficulty: crate::brain::smash::DifficultyProfile {
                    reaction_delay_s: *reaction_delay_s,
                    commit_probability: *commit_probability,
                    accuracy: *accuracy,
                    mash_speed_hz: *mash_speed_hz,
                },
                // Neutral-game knobs aren't part of the catalog preset schema
                // yet; inherit the striker defaults (footsies off) until a
                // duelist preset surfaces them.
                ..crate::brain::smash::SmashCfg::STRIKER_DEFAULT
            },
            state: crate::brain::smash::SmashState::default(),
        },
    };
    Brain::StateMachine(cfg)
}

/// Build a runtime [`ActionSet`] from a preset.
#[allow(
    dead_code,
    reason = "Public resolver API used by future spawn-site catalog consumers; today exercised via tests."
)]
pub fn action_set_from_preset(preset: &ActionSetPreset) -> ActionSet {
    ActionSet {
        move_style: move_style_from_preset(preset.move_style),
        melee: preset.melee.map(melee_from_preset),
        ranged: preset.ranged.map(ranged_from_preset),
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
        } => MeleeActionSpec::Lunge(LungeSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
            step_px,
        }),
        MeleePreset::Slam {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
            hop_height_px,
        } => MeleeActionSpec::Slam(SlamSpec {
            windup_s,
            active_s,
            recover_s,
            damage,
            reach_px,
            hop_height_px,
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
        RangedPreset::Rock { speed, damage } => RangedActionSpec::Rock { speed, damage },
        RangedPreset::Arrow { speed, damage } => RangedActionSpec::Arrow { speed, damage },
        RangedPreset::Pistol { speed, damage } => RangedActionSpec::Pistol { speed, damage },
        RangedPreset::Bolt { speed, damage } => RangedActionSpec::Bolt { speed, damage },
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
