//! Build fresh autonomous brains from catalog presets and placement context.

use ambition_characters::actor::character_catalog::{BrainPreset, CharacterCatalog};
use ambition_characters::brain::state_machine::{
    AuthoredWorldPatrolLane, MeleeBruteCfg, MeleeBruteState, PatrolCfg, PatrolState, SkirmisherCfg,
    SkirmisherState, SniperCfg, SniperState, StateMachineCfg, WandererCfg,
};
use ambition_characters::brain::{BossPatternCfg, BossPatternState, Brain};
use ambition_characters::control::brain_binding::{
    BrainBinding, BrainBuildContext, BrainBuildError,
};

/// Resolve a character_id into a runtime [`ambition_characters::brain::Brain`]
/// using its catalog default. `spawn_world_x` becomes the patrol
/// center for `Patrol`-brain characters.
pub fn build_default_brain(
    catalog: &CharacterCatalog,
    character_id: &str,
    spawn_world_x: f32,
) -> Option<ambition_characters::brain::Brain> {
    let entry = catalog.get(character_id)?;
    // A character that names no preset has no default brain to build — its
    // definition's autonomous profile is the answer, and this road is not it.
    if entry.default_brain.is_empty() {
        return None;
    }
    let preset = catalog.data().brain_presets.get(&entry.default_brain)?;
    Some(brain_from_preset(preset, spawn_world_x))
}

/// Build a runtime [`ambition_characters::brain::Brain`] from a NAMED brain preset,
/// threading the per-spawn [`BrainBuildContext`] (patrol lane center / radius
/// / path). `None` if the preset name is not in `brain_presets`.
///
/// This is the single seam `preset name → Brain`. Both
/// [`resolve_initial_brain`] (spawn) and the runtime `BrainCommand` reducer
/// call it, so a preset resolves identically at spawn and at a later runtime
/// switch. It never inspects the built brain.
pub fn build_brain_from_preset(
    catalog: &CharacterCatalog,
    preset_name: &str,
    ctx: &BrainBuildContext,
) -> Option<ambition_characters::brain::Brain> {
    let preset = catalog.data().brain_presets.get(preset_name)?;
    Some(brain_from_preset_with_context(preset, ctx))
}

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
            aggressiveness,
        } => StateMachineCfg::Wanderer {
            cfg: WandererCfg {
                speed: *speed,
                aggressiveness: *aggressiveness,
            },
        },
        BrainPreset::MeleeBrute {
            aggressiveness,
            aggro_radius,
            attack_range,
            chase_speed,
            navigates,
        } => StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg {
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
                chase_speed: *chase_speed,
                navigates: *navigates,
                retreat_after_swing_s: 0.0,
                tactics: Default::default(),
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
                ranged_volley: None,
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
            cfg: ambition_characters::brain::state_machine::AerialCfg {
                aggressiveness: *aggressiveness,
                cruise_speed: *cruise_speed,
                dive_speed: *dive_speed,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
                roam_radius: *roam_radius,
                dive_tell_s: 0.0,
            },
            state: ambition_characters::brain::state_machine::AerialState::default(),
        },
        BrainPreset::Roam {
            speed,
            trot_speed,
            rest_min_s,
            rest_max_s,
            company,
            stay_within,
            playful,
            notice_radius,
        } => StateMachineCfg::Roam {
            cfg: ambition_characters::brain::state_machine::RoamCfg {
                speed: *speed,
                trot_speed: *trot_speed,
                rest_min_s: *rest_min_s,
                rest_max_s: *rest_max_s,
                company: *company,
                stay_within: *stay_within,
                playful: *playful,
                notice_radius: *notice_radius,
            },
            state: ambition_characters::brain::state_machine::RoamState::default(),
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
        BrainPreset::Fighter {
            level,
            decision_interval_ticks,
        } => {
            let mut cfg = ambition_characters::brain::fighter::FighterCfg::new(
                ambition_characters::brain::fighter::FighterBrainProfile::engine_default(*level),
            );
            cfg.decision_interval_ticks = *decision_interval_ticks;
            let state = ambition_characters::brain::fighter::FighterState::new(
                &cfg,
                // level-only, and that is correct HERE and nowhere else.
                // This builds a catalog PREVIEW brain — there is no live runtime,
                // no match and therefore no PARTICIPANT to key a stream on, so
                // the rung is the only stable identity available.
                //
                // do not copy this into a construction path. The identical
                // expression on the real road made every CPU on one rung the SAME
                // MIND, so a same-character CPU-vs-CPU match played as a perfect
                // reflection; the live seam is
                // `brain_builders::fighter_cognition_seed`, which mixes the
                // participant in and carries Emmy's authored exception. A replay
                // still reproduces this preview because the rung does not change.
                0x5F37_7A11_u64.wrapping_mul(*level as u64 + 1),
            );
            StateMachineCfg::Fighter {
                cfg: Box::new(cfg),
                state: Box::new(state),
            }
        }
        BrainPreset::Smash {
            aggro_radius,
            engage_distance,
            attack_range,
            too_close_distance,
            chase_speed,
            retreat_speed,
            crowding_threshold,
            sprint_to_close,
            reaction_delay_s,
            commit_probability,
            accuracy,
        } => StateMachineCfg::Smash {
            cfg: ambition_characters::brain::smash::SmashCfg {
                aggro_radius: *aggro_radius,
                engage_distance: *engage_distance,
                attack_range: *attack_range,
                too_close_distance: *too_close_distance,
                chase_speed: *chase_speed,
                retreat_speed: *retreat_speed,
                crowding_threshold: *crowding_threshold,
                sprint_to_close: *sprint_to_close,
                difficulty: ambition_characters::brain::smash::DifficultyProfile {
                    reaction_delay_s: *reaction_delay_s,
                    commit_probability: *commit_probability,
                    accuracy: *accuracy,
                },
                // Neutral-game knobs aren't part of the catalog preset schema
                // yet; inherit the striker defaults (footsies off) until a
                // duelist preset surfaces them.
                ..ambition_characters::brain::smash::SmashCfg::STRIKER_DEFAULT
            },
            state: ambition_characters::brain::smash::SmashState::default(),
        },
    };
    Brain::StateMachine(cfg)
}

/// Build a runtime [`Brain`] from a preset, threading a per-spawn
/// [`BrainBuildContext`](BrainBuildContext).
///
/// Identical to [`brain_from_preset`] for every non-patrol preset. For a
/// [`BrainPreset::Patrol`], the lane center is `ctx.spawn_world_x +
/// spawn_local_x` and the lane radius is the placement's `patrol_radius`
/// override when one was authored, else the preset's authored `radius`. The
/// placement's patrol parameters PARAMETERIZE a chosen patrol preset — they
/// never select it (that is the caller's explicit selection).
pub fn brain_from_preset_with_context(preset: &BrainPreset, ctx: &BrainBuildContext) -> Brain {
    if let BrainPreset::Patrol {
        spawn_local_x,
        radius,
        speed,
        aggressiveness,
        aggro_radius,
        attack_range,
    } = preset
    {
        let lane_radius = ctx.patrol_radius.unwrap_or(*radius);
        return Brain::StateMachine(StateMachineCfg::Patrol {
            cfg: PatrolCfg {
                lane: AuthoredWorldPatrolLane::new(ctx.spawn_world_x + spawn_local_x, lane_radius),
                speed: *speed,
                aggressiveness: *aggressiveness,
                aggro_radius: *aggro_radius,
                attack_range: *attack_range,
            },
            state: PatrolState::default(),
        });
    }
    brain_from_preset(preset, ctx.spawn_world_x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};
    use ambition_characters::actor::character_catalog::{BrainPresetId};
    use ambition_characters::control::brain_binding::AutonomousDefault;
    use ambition_characters::brain::{Brain, StateMachineCfg};
    use ambition_characters::control::brain_binding::{AutonomousSource, PresetSource};

    // A fixture roster: a peaceful WANDERER default (puppy slug), a HOSTILE
    // MeleeBrute default (a fighter placed as a talkable NPC), and a PATROL
    // default whose lane radius a placement can override.
    const CATALOG: &str = r#"(
        brain_presets: {
            "stand_still": StandStill,
            "wanderer_puppy_slug": Wanderer(speed: 36.0, aggressiveness: 0.0),
            "melee_brute_striker": MeleeBrute(
                aggressiveness: 1.0, aggro_radius: 220.0, attack_range: 36.0, chase_speed: 110.0,
            ),
            "patrol_peaceful": Patrol(
                spawn_local_x: 0.0, radius: 64.0, speed: 28.0,
                aggressiveness: 0.0, aggro_radius: 80.0, attack_range: 0.0,
            ),
        },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "npc_puppy_slug": (
                display_name: "Puppy Slug", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Crawler, composition: None,
                default_brain: "wanderer_puppy_slug", default_action_set: "peaceful", tags: [],
            ),
            "npc_brute": (
                display_name: "Brute", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "melee_brute_striker", default_action_set: "peaceful", tags: [],
            ),
            "npc_patroller": (
                display_name: "Patroller", manifest: "x_spritesheet.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "patrol_peaceful", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(parse_catalog(CATALOG))
    }

    fn resolve(
        cid: &str,
        authored: Option<&str>,
        ctx: BrainBuildContext,
    ) -> Result<(BrainBinding, Brain), BrainBuildError> {
        resolve_initial_brain(&catalog(), cid, authored, &ctx)
    }

    /// #1 — a puppy slug with no override receives its `wanderer_puppy_slug`
    /// default, and the binding records that default on the Default selection.
    #[test]
    fn character_default_resolves_the_catalog_default_brain() {
        let (binding, brain) = resolve("npc_puppy_slug", None, BrainBuildContext::at(0.0)).unwrap();
        assert_eq!(brain.label(), "wanderer");
        assert_eq!(
            binding.default_preset,
            AutonomousDefault::Preset(BrainPresetId::new("wanderer_puppy_slug"))
        );
        assert_eq!(binding.source, AutonomousSource::CatalogDefault);
        assert_eq!(
            binding.active_preset(),
            Some(&BrainPresetId::new("wanderer_puppy_slug"))
        );
    }

    /// #2 — a puppy slug with a `stand_still` override receives a StandStill
    /// brain, and the binding records the override (default preset preserved).
    #[test]
    fn stand_still_override_wins_over_the_wander_default() {
        let (binding, brain) = resolve(
            "npc_puppy_slug",
            Some("stand_still"),
            BrainBuildContext::at(0.0),
        )
        .unwrap();
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert_eq!(
            binding.default_preset,
            AutonomousDefault::Preset(BrainPresetId::new("wanderer_puppy_slug"))
        );
        assert_eq!(
            binding.source,
            AutonomousSource::CatalogPreset(BrainPresetId::new("stand_still"))
        );
    }

    /// #6 — a hostile catalog default is USED when no override is authored (the
    /// removed is_hostile gate no longer peaceful-izes a placed hostile default).
    #[test]
    fn hostile_default_is_used_without_an_override() {
        let (_, brain) = resolve("npc_brute", None, BrainBuildContext::at(0.0)).unwrap();
        assert_eq!(brain.label(), "melee_brute");
        assert!(
            brain.is_hostile(),
            "the hostile default drives a hostile brain"
        );
    }

    /// #7 — a hostile character with a StandStill override stays stationary.
    #[test]
    fn hostile_character_with_stand_still_override_is_stationary() {
        let (_, brain) =
            resolve("npc_brute", Some("stand_still"), BrainBuildContext::at(0.0)).unwrap();
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert!(!brain.is_hostile());
    }

    /// #8 — `patrol_radius > 0` does NOT independently select a patrol brain: a
    /// wanderer default with a placement patrol_radius stays a wanderer.
    #[test]
    fn patrol_radius_does_not_select_a_patrol_brain() {
        let (_, brain) = resolve(
            "npc_puppy_slug",
            None,
            BrainBuildContext::from_placement(0.0, 96.0),
        )
        .unwrap();
        assert_eq!(
            brain.label(),
            "wanderer",
            "a non-patrol default ignores placement patrol_radius; it must not become Patrol"
        );
    }

    /// #9 — a SELECTED patrol preset consumes the placement's patrol radius (and
    /// centers its lane on the spawn anchor); with no placement override it keeps
    /// the preset's authored radius.
    #[test]
    fn selected_patrol_preset_consumes_placement_radius() {
        // Placement radius override wins.
        let (_, brain) = resolve(
            "npc_patroller",
            None,
            BrainBuildContext::from_placement(100.0, 200.0),
        )
        .unwrap();
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.lane.radius_px, 200.0, "placement radius feeds the lane");
                assert_eq!(cfg.lane.center_x, 100.0, "lane centers on spawn_world_x");
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
        // No placement override -> the preset's authored radius (64).
        let (_, brain) = resolve("npc_patroller", None, BrainBuildContext::at(100.0)).unwrap();
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(
                    cfg.lane.radius_px, 64.0,
                    "keeps the preset radius with no override"
                );
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    /// #10 — an unknown non-empty preset id fails resolution (no silent fallback),
    /// and an unknown character id fails too.
    #[test]
    fn unknown_preset_and_character_fail_resolution() {
        let err = resolve(
            "npc_puppy_slug",
            Some("no_such_preset"),
            BrainBuildContext::at(0.0),
        )
        .unwrap_err();
        assert!(
            matches!(err, BrainBuildError::UnknownPreset { source: PresetSource::Override, .. }),
            "an unknown override preset is an Override-sourced error, not a silent fallback: {err:?}"
        );

        let err = resolve("no_such_character", None, BrainBuildContext::at(0.0)).unwrap_err();
        assert!(matches!(err, BrainBuildError::UnknownCharacter(_)));
    }

    /// Empty / whitespace `brain_override` means the character default.
    #[test]
    fn empty_override_means_character_default() {
        for authored in [Some(""), Some("   "), None] {
            let (binding, _) =
                resolve("npc_puppy_slug", authored, BrainBuildContext::at(0.0)).unwrap();
            assert_eq!(
                binding.source,
                AutonomousSource::CatalogDefault,
                "{authored:?}"
            );
        }
    }

    // They pinned a real ruling — *"a character definition may state its normal
    // autonomous behaviour, and it outranks the catalog row"* — through the
    // `definition_default` parameter, a `BrainPresetRef` a definition could
    // author. That parameter is deleted: no character in the repo ever
    // authored one, and its absence is what made the resolver manufacture an
    // empty preset id and crash two shipped rooms.
    //
    //  the RULING survives; only its vocabulary changed. A character states its
    // policy as a `BrainProfile` now, and the same three claims — the character
    // outranks the row, an authored placement override outranks both, and a
    // silent character leaves the row standing — are asserted in
    // `features::npcs`, which is where the profile can be lowered against a body.
    // Re-asserting them here would need a body this crate does not have.
}
#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn brain_preset_patrol_offsets_spawn_world_x() {
        // The patrol lane center is `spawn_world_x + spawn_local_x`.
        // Pin the offset arithmetic so a refactor that drops the add
        // breaks here rather than at first-spawn.
        let preset = BrainPreset::Patrol {
            spawn_local_x: 5.0,
            radius: 64.0,
            speed: 32.0,
            aggressiveness: 0.0,
            aggro_radius: 80.0,
            attack_range: 0.0,
        };
        let brain = brain_from_preset(&preset, 100.0);
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.lane.center_x, 105.0);
                assert_eq!(cfg.lane.radius_px, 64.0);
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    #[test]
    fn brain_preset_smash_threads_cfg_and_difficulty() {
        // The data-exposed Smash fighter preset must resolve to a
        // StateMachineCfg::Smash with its tuning + difficulty floats
        // threaded through — this is the seam the PCA encounter swaps in.
        let preset = BrainPreset::Smash {
            aggro_radius: 460.0,
            engage_distance: 76.0,
            attack_range: 52.0,
            too_close_distance: 34.0,
            chase_speed: 150.0,
            retreat_speed: 120.0,
            crowding_threshold: 0.65,
            sprint_to_close: true,
            reaction_delay_s: 0.12,
            commit_probability: 0.85,
            accuracy: 0.9,
        };
        let brain = brain_from_preset(&preset, 0.0);
        match brain {
            Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => {
                assert_eq!(cfg.attack_range, 52.0);
                assert!(cfg.sprint_to_close);
                assert_eq!(cfg.difficulty.reaction_delay_s, 0.12);
                assert_eq!(cfg.difficulty.commit_probability, 0.85);
                assert_eq!(cfg.difficulty.accuracy, 0.9);
            }
            other => panic!("expected Smash, got {other:?}"),
        }
        // Smash brains are hostile by construction (the encounter only
        // arms them on the explicit challenge choice).
        assert!(brain_from_preset(&preset, 0.0).is_hostile());
    }
}

/// Build the autonomous preset selected by the controller's authored rules.
pub fn resolve_initial_brain(
    catalog: &CharacterCatalog,
    character_id: &str,
    authored_override: Option<&str>,
    ctx: &BrainBuildContext,
) -> Result<(BrainBinding, Brain), BrainBuildError> {
    let binding = ambition_characters::control::brain_binding::resolve_initial_binding(
        catalog,
        character_id,
        authored_override,
    )?;
    let preset = binding
        .active_preset()
        .expect("initial selection names a preset");
    let brain = build_brain_from_preset(catalog, preset.as_str(), ctx)
        .expect("initial selection validates the preset against this catalog");
    Ok((binding, brain))
}
