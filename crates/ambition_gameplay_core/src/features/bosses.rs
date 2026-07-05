//! Boss runtime glue for the actor simulation: the [`boss_special_for_profile`]
//! resolver that maps a `BossAttackProfile::Special(key)` to a
//! `SpecialActionSpec` (the open seam `tick_boss_brains_system` uses to emit
//! `ActorActionMessage::Special`). The boss PROFILE/pattern vocabulary and
//! sprite metrics now live in `ambition_characters::brain::boss_pattern` and
//! `crate::boss_encounter::behavior` and are re-exported here for legacy paths.

// Boss policy vocabulary (`BossMovementProfile`, `BossPatternStep`,
// `BossPattern`, `BossAttackPattern`, `BossAttackProfile`,
// `step_duration`) moved to `ambition_characters::brain::boss_pattern` per the
// "move boss policy out of BossRuntime" migration. Re-exported here
// because `BossBehaviorProfile` and the volumes / construction code
// below still reference them by their old `content::features::bosses`
// path — those references stay legal via the re-export while call
// sites migrate to the brain-module path at their leisure.
#[cfg(test)]
use ambition_characters::brain::boss_pattern::BossAttackPattern;
pub use ambition_characters::brain::boss_pattern::{BossAttackProfile, BossMovementProfile};
// `BossPattern` and `BossPatternStep` only show up inside the
// scripted profiles, which now live in `boss_profiles.ron`. They're
// still publicly accessible via `ambition_characters::brain::boss_pattern`; we
// just don't re-export them here anymore.

// `BossTickOutputs` (previously: `projectile_spawns: Vec<…>`) was
// deleted with Task B of the actor/brain follow-up plan. Apple-rain
// spawning moved to `spawn_gnu_apple_rain_from_special_messages` (an
// EFFECTS-stage consumer driven by `ActorActionMessage::Special`).
// Future boss specials follow the same pattern — one consumer per
// `SpecialActionSpec` variant — instead of accumulating side-channel
// `Vec`s the caller flushes.

// All boss-special tuning numbers (apple-rain cadence, overfit-volley sampling,
// minima-trap / saddle-point / gradient-cascade params, the eye-beam tuning)
// moved to `ambition_content::bosses::specials` with the Techniques themselves —
// the engine names no boss special's params. The engine retains only the generic
// boss machinery (profile/spec/resolver) below.

// `GNU_TON_ANCHOR_Y`, `GNU_TON_COLLISION_SCALE`, `GNU_TON_FRAME_HEIGHT`,
// and `gnu_ton_sprite_scale` were retired in the 2026-05-26
// data-driven migration. The GNU-ton per-animation hit / hurt-box
// geometry lives in `gnu_ton_boss_spritesheet.ron`'s
// `body_metrics.animations` map and flows through the generic
// `world_aabb_from_pixel_rect` transform the gradient sentinel uses.

// `BossBehaviorProfile` / `BossRewardProfile` / `ActorSpriteMetrics` /
// `canonical_boss_id_from` / `boss_animation_keys_for_profile` moved to
// `crate::boss_encounter::behavior` (Stage 20 / A2 stretch): the boss
// PROFILE vocabulary is machinery (data-driven via boss_profiles.ron);
// this module keeps the named special-spec resolver + tuning consts.
#[cfg(test)]
use crate::boss_encounter::behavior::canonical_boss_id_from;
pub use crate::boss_encounter::behavior::{
    boss_animation_keys_for_profile, ActorSpriteMetrics, BossBehaviorProfile, BossRewardProfile,
};

/// Boss-side resolver for `Special`-flavored `BossAttackProfile`s.
///
/// The Gradient Sentinel carries multiple distinct specials
/// (MemorizedVolley, PitTrap, RotatingCross, MinionCascade) — more
/// than the single `ActionSet::special` slot can express. Rather
/// than grow `ActionSet` or `ActorControlFrame` for one boss, the
/// `tick_boss_brains_system` calls this function when the brain
/// commits to a special-flavored profile and writes the resulting
/// `ActorActionMessage::Special { spec }` directly via
/// `MessageWriter`. The boss's `ActionSet.special` is set to `None`
/// for multi-special bosses so the generic resolver doesn't fire a
/// duplicate.
///
/// `None` means the profile doesn't have a registered special spec
/// — the consumer should treat that as a no-op (defensive against
/// schedule edits that introduce a profile before the spec wiring
/// lands).
pub fn boss_special_for_profile(
    profile: &ambition_characters::brain::BossAttackProfile,
) -> Option<ambition_characters::brain::SpecialActionSpec> {
    use ambition_characters::brain::SpecialActionSpec;
    // Open seam: a `Special` beat carries its content-technique key; the
    // brain emits it verbatim as `SpecialActionSpec::Special(key)` and the
    // matching content Technique reads its own params + emits the effects.
    // Ordinary (geometry) profiles never route through here — they damage
    // via `boss_attack_damage` reading `BossAttackState` directly, so they
    // map to `None`. The engine names no boss special.
    profile
        .special_key()
        .map(|key| SpecialActionSpec::Special(key.to_string()))
}

/// Aggressor push for a boss strike (matches the old `sync_boss_strike_hitboxes`
/// / `boss_attack_damage` strike arm). Carried on the geometry move's hit volume.
pub const BOSS_STRIKE_KNOCKBACK: f32 = 1.25;

/// Build the boss's data-driven attack MOVESET from its capability repertoire —
/// ONE moveset move per authored strike profile, so EVERY boss strike runs through
/// the SAME moveset runtime an actor's swing does (fable review §A1: the moveset is
/// the boss's melee system too, retiring the bespoke `sync_boss_strike_hitboxes`):
///
/// - A content-technique **`Special(key)`** profile → a move whose single window
///   SUSTAINS `Effect{key}` for the strike duration, so the technique fires every
///   frame the strike is live (the `apple_rain`-style per-frame signal) through the
///   `Effect{key}`→`Special{key}` bridge — no body-mounted hit volume.
/// - A **geometry** profile (FloorSlam / SideSweep / HazardColumn / …) → a move
///   whose Active window carries the profile's static hit volumes as BODY-LOCAL
///   [`HitVolume`]s, derived from `volumes_for_profile` at a body-local origin (the
///   world-space math cancels the boss position, leaving a constant local offset).
///   `advance_move_playback` then spawns/despawns the strike hitbox through the ONE
///   shared hitbox pipeline (`apply_hitbox_damage`'s Boss branch), exactly as the
///   old per-tick sync did — minus the sprite-frame-tracking geometry (a
///   parameterizable fidelity detail; the static fallback approximates it).
///
/// Keyed by [`BossAttackProfile::move_id`]; `trigger_boss_attack_moves` resolves the
/// active profile via `move_by_id`, not an input verb. `None` if the boss authors no
/// strike at all.
pub fn boss_attack_moveset(
    capability: &ambition_characters::brain::BossCapability,
    behavior: &BossBehaviorProfile,
    combat_size: ambition_engine_core::Vec2,
    telegraph_windows: &[(ambition_characters::brain::BossAttackProfile, f32)],
) -> Option<crate::combat::moveset::ActorMoveset> {
    use ambition_engine_core::AabbExt;
    use ambition_entity_catalog::{
        ClipBinding, EffectRef, HitVolume, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
        WindowTag,
    };
    let telegraph_for = |profile: &ambition_characters::brain::BossAttackProfile| -> f32 {
        telegraph_windows
            .iter()
            .find(|(p, _)| p == profile)
            .map(|(_, t)| t.max(0.0))
            .unwrap_or(0.0)
    };
    let moves: Vec<MoveSpec> = capability
        .specials
        .iter()
        .filter_map(|(profile, strike_s)| {
            let strike_s = strike_s.max(0.05);
            // The move spans the whole telegraph→strike as ONE timeline: its Active
            // window opens at `tel` and closes at `tel + strike` (E53). A move started
            // at `t0 = tel` (strike edge / possession) is live immediately; one started
            // at `t0 = 0` plays the telegraph first. Either way the projected
            // `active_elapsed` folds in the telegraph offset because it reads the move's
            // own clock `t`.
            let tel = telegraph_for(profile);
            let active_start = tel;
            let active_end = tel + strike_s;
            let (volumes, sustain_effect) = if let Some(key) = profile.special_key() {
                (Vec::new(), Some(EffectRef::new(key)))
            } else {
                // Geometry strike: `volumes_for_profile` at a ZERO body origin yields
                // AABBs centered on the profile's body-local offset (the boss position
                // cancels: origin = pos + attack_origin_offset, center = origin +
                // offset, local = center - pos). Convert each to a body-local
                // `HitVolume` the move runtime mirrors by facing + rotates into the
                // gravity frame at spawn.
                let volumes: Vec<HitVolume> =
                    crate::boss_encounter::attack_geometry::volumes_for_profile(
                        profile,
                        ambition_engine_core::Vec2::ZERO,
                        combat_size,
                        behavior,
                    )
                    .into_iter()
                    .map(|aabb| {
                        let c = aabb.center();
                        let h = aabb.half_size();
                        HitVolume {
                            shape: VolumeShape::Rect {
                                offset: (c.x, c.y),
                                half_extents: (h.x, h.y),
                            },
                            damage: behavior.attack_damage.max(1),
                            knockback: BOSS_STRIKE_KNOCKBACK,
                            on_hit: None,
                            // Boss geometry strikes are data-shaped volumes, not
                            // bladed swings: no slash VFX, no manifest override.
                            vfx: None,
                        }
                    })
                    .collect();
                // A geometry profile with no authored volume (defensive) contributes
                // no move — skip it rather than a hitless Active window.
                if volumes.is_empty() {
                    return None;
                }
                (volumes, None)
            };
            Some(MoveSpec {
                id: profile.move_id(),
                clip: ClipBinding {
                    clip: "attack".to_string(),
                    fallbacks: vec!["idle".to_string()],
                },
                duration_s: active_end,
                windows: vec![MoveWindow {
                    start_s: active_start,
                    end_s: active_end,
                    tag: WindowTag::Active,
                    volumes,
                    sustain_effect,
                }],
                events: Vec::new(),
                gates: Default::default(),
                start_impulse: None,
            })
        })
        .collect();
    (!moves.is_empty()).then(|| {
        crate::combat::moveset::ActorMoveset(MovesetContract {
            verbs: std::collections::BTreeMap::new(),
            moves,
        })
    })
}

#[cfg(test)]
mod boss_profile_data_tests {
    use super::*;

    /// `assets/data/boss_profiles.ron` must carry a row for every
    /// boss the codebase has a constructor for. Without this, the
    /// `from_data` lookup would panic at the first spawn of a
    /// missing boss.
    #[test]
    fn ron_carries_every_known_boss() {
        for id in [
            "clockwork_warden",
            "mockingbird",
            "gnu_ton",
            "smirking_behemoth_boss",
        ] {
            // `from_data` panics with a clear message when the row is
            // missing (the registry static is private to behavior.rs).
            let _ = BossBehaviorProfile::from_data(id);
        }
    }

    /// Spot-check the legacy pre-data values for a divergent
    /// archetype: the Clockwork Warden's macro tuning and attack
    /// damage. Catches accidental tuning drift on the row the
    /// player notices first.
    #[test]
    fn legacy_baseline_pins() {
        let warden = BossBehaviorProfile::clockwork_warden();
        assert_eq!(warden.id, "clockwork_warden");
        assert_eq!(warden.attack_damage, 2);
        assert_eq!(warden.body_damage, 1);
        assert!((warden.strike_speed_scale - 0.20).abs() < f32::EPSILON);
        assert!((warden.macro_tuning.too_close_distance - 110.0).abs() < f32::EPSILON);
        assert!((warden.macro_tuning.engage_max_duration_s - 9.0).abs() < f32::EPSILON);
        let gnu = BossBehaviorProfile::gnu_ton();
        assert_eq!(gnu.body_damage, 0);
        assert_eq!(gnu.attacks.len(), 5);
        let mocker = BossBehaviorProfile::mockingbird();
        assert!(matches!(mocker.attack_pattern, BossAttackPattern::Cycle));
    }
}

#[cfg(test)]
mod canonical_boss_id_tests {
    use super::*;
    use ambition_engine_core as ae;

    /// PhaseScript brain wins over display name. The user-reported
    /// bug: BossSpawn named "System Boss" in `first_system_boss`
    /// derived encounter_id "system_boss" (no profile, no music).
    /// With `canonical_boss_id_from` reading the brain's
    /// `PhaseScript:clockwork_warden` it resolves to the
    /// authored profile and the boss fight gets its violin music.
    #[test]
    fn phase_script_brain_wins_over_display_name() {
        let id = canonical_boss_id_from(
            "System Boss",
            &ambition_characters::actor::BossBrain::PhaseScript {
                script_id: "clockwork_warden".to_string(),
            },
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// Empty PhaseScript falls back to the display name.
    #[test]
    fn empty_phase_script_falls_back_to_name() {
        let id = canonical_boss_id_from(
            "System Boss",
            &ambition_characters::actor::BossBrain::PhaseScript {
                script_id: String::new(),
            },
        );
        assert_eq!(id, "system_boss");
    }

    /// Custom brain with a non-empty label is treated like a name
    /// (gets normalized to an encounter_id slug).
    #[test]
    fn custom_brain_label_becomes_encounter_id_slug() {
        let id = canonical_boss_id_from(
            "Display",
            &ambition_characters::actor::BossBrain::Custom("Clockwork Warden".to_string()),
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// Dormant brain falls back to the display name.
    #[test]
    fn dormant_brain_falls_back_to_name() {
        let id = canonical_boss_id_from(
            "Clockwork Warden",
            &ambition_characters::actor::BossBrain::Dormant,
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// BossRuntime constructed with a "System Boss" name + PhaseScript
    /// brain ends up with the clockwork_warden behavior — the runtime
    /// resolves the canonical id before reading
    /// `BossBehaviorProfile::for_authored_boss`. Without this fix the
    /// runtime would carry a generic placeholder behavior.
    #[test]
    fn boss_runtime_uses_phase_script_for_behavior_lookup() {
        let aabb = ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(40.0, 50.0));
        let boss = super::super::ecs::boss_clusters::BossClusterScratch::new(
            "boss_under_test",
            "System Boss",
            aabb,
            ambition_characters::actor::BossBrain::PhaseScript {
                script_id: "clockwork_warden".to_string(),
            },
        );
        assert_eq!(boss.config.behavior.id, "clockwork_warden");
        // Sanity: the Gradient Sentinel macro tuning is non-trivial
        // (chase/retreat thresholds non-zero), which the generic
        // boss profile doesn't set.
        assert!(
            boss.config.behavior.macro_tuning.is_enabled(),
            "clockwork_warden behavior should carry macro tuning",
        );
    }
}

#[cfg(test)]
mod boss_special_resolver_tests {
    use super::*;

    /// Every special-flavored profile must map to a Some(spec) — otherwise
    /// the boss tick will emit no Special message for that beat and the
    /// schedule silently degrades. Pin the mapping so future schedule
    /// edits can't introduce a profile without its consumer wiring.
    #[test]
    fn every_special_profile_resolves_to_a_spec_for_gradient_sentinel() {
        use ambition_characters::brain::BossAttackProfile;
        for key in [
            "overfit_volley",
            "minima_trap",
            "saddle_point",
            "gradient_cascade",
        ] {
            let profile = BossAttackProfile::Special(key.into());
            assert!(
                boss_special_for_profile(&profile).is_some(),
                "{profile:?} must resolve to a spec for Gradient Sentinel",
            );
        }
    }

    /// GNU-ton's apple rain still resolves through the open seam: the
    /// `Special("apple_rain")` beat maps to a `Special` spec carrying the
    /// verbatim key, which the content apple-rain Technique recognizes.
    #[test]
    fn gnu_apple_rain_profile_resolves_to_apple_rain_spec_for_gnu_ton() {
        use ambition_characters::brain::{BossAttackProfile, SpecialActionSpec};
        match boss_special_for_profile(&BossAttackProfile::Special("apple_rain".into())) {
            Some(SpecialActionSpec::Special(key)) => assert_eq!(key, "apple_rain"),
            other => panic!("expected Special(apple_rain) spec, got {other:?}"),
        }
    }

    /// Ordinary melee-style profiles return None — they don't go
    /// through the Special path; their damage routes via
    /// `boss_attack_damage` reading `BossAttackState` directly.
    #[test]
    fn ordinary_profiles_resolve_to_none() {
        use ambition_characters::brain::BossAttackProfile;
        for profile in [
            BossAttackProfile::Strike("floor_slam".to_string()),
            BossAttackProfile::Strike("side_sweep".to_string()),
            BossAttackProfile::Strike("full_body_pulse".to_string()),
            BossAttackProfile::Strike("hazard_column".to_string()),
        ] {
            assert!(
                boss_special_for_profile(&profile).is_none(),
                "{profile:?} should not have a Special spec",
            );
        }
    }
}

// `step_duration` moved to `ambition_characters::brain::boss_pattern`.

#[cfg(test)]
mod scripted_pattern_tests {
    use super::*;
    use crate::features::FeatureCombatTuning;
    use ambition_characters::brain::boss_pattern::BossPatternStep;
    use ambition_engine_core as ae;
    use ambition_engine_core::AabbExt;

    fn gnu_ton_runtime() -> super::super::ecs::boss_clusters::BossClusterScratch {
        let behavior = BossBehaviorProfile::gnu_ton();
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut scratch = super::super::ecs::boss_clusters::BossClusterScratch::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            ambition_characters::actor::BossBrain::Dormant,
        );
        scratch.config.behavior = behavior;
        scratch.status.encounter_phase = crate::boss_encounter::BossEncounterPhase::Phase1;
        // After the data-driven migration, the head-position invariants
        // (rest above shoulder, descent at player level) live in the
        // sprite RON's per-animation `hurtbox.parts`. The test fixture
        // pre-populates a minimal `sprite_metrics` snapshot so
        // `damageable_volumes` flows through the same lookup path the
        // live runtime uses. Mirrors the rest/gnu_head_descent rows in
        // `gnu_ton_boss_spritesheet.ron`.
        scratch.status.sprite_metrics = Some(gnu_ton_sprite_metrics_fixture());
        scratch
    }

    /// Build a minimal `ActorSpriteMetrics` whose per-animation
    /// `hurtbox.parts` mirror the rest / descent head positions
    /// authored in the live spritesheet RON. Tests that exercise
    /// `damageable_volumes` use this so the head invariants stay
    /// pinned even though gnu_ton_runtime doesn't go through
    /// `derive_boss_sprite_metrics`.
    fn gnu_ton_sprite_metrics_fixture() -> super::ActorSpriteMetrics {
        use ambition_sprite_sheet::{AnimationBox, AnimationMetrics, NamedPixelRect};
        use std::collections::HashMap;
        let head_rest = NamedPixelRect {
            name: "head".to_string(),
            x: 301,
            y: 146,
            w: 166,
            h: 133,
        };
        let head_descent_hurt = NamedPixelRect {
            name: "head".to_string(),
            x: 301,
            y: 252,
            w: 166,
            h: 133,
        };
        let head_descent_hit = NamedPixelRect {
            name: "head".to_string(),
            x: 292,
            y: 244,
            w: 184,
            h: 148,
        };
        let rest_entry = AnimationMetrics {
            hurtbox: Some(AnimationBox {
                parts: vec![head_rest.clone()],
                bbox: None,
                poly: Vec::new(),
                frames: Vec::new(),
            }),
            hitbox: None,
            frame_duration_secs: None,
        };
        let descent_entry = AnimationMetrics {
            hurtbox: Some(AnimationBox {
                parts: vec![head_descent_hurt],
                bbox: None,
                poly: Vec::new(),
                frames: Vec::new(),
            }),
            hitbox: Some(AnimationBox {
                parts: vec![head_descent_hit],
                bbox: None,
                poly: Vec::new(),
                frames: Vec::new(),
            }),
            frame_duration_secs: None,
        };
        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert("rest".to_string(), rest_entry);
        animations.insert("gnu_head_descent".to_string(), descent_entry);
        super::ActorSpriteMetrics {
            frame_width: 768,
            frame_height: 576,
            body_pixel_bbox: None,
            body_pixel_parts: Vec::new(),
            // Match what `sprite_render_size_for("gnu_ton_boss", boss.size)`
            // would produce for a (220, 220) spawn → GNU_TON_SHEET's
            // 4.5× collision_scale: render = 990×990 with aspect
            // adjustment to 1320×990 for the 768/576 frame ratio.
            sprite_render_size: ae::Vec2::new(1320.0, 990.0),
            combat_offset: ae::Vec2::ZERO,
            animations,
        }
    }

    #[test]
    fn gnu_ton_pattern_includes_explicit_rest_beats_in_every_phase() {
        let BossAttackPattern::Scripted {
            phase1,
            transition,
            phase2,
            enrage,
            ..
        } = BossBehaviorProfile::gnu_ton().attack_pattern
        else {
            panic!("gnu_ton must use a Scripted attack pattern");
        };
        for (label, pattern) in [
            ("phase1", &phase1),
            ("transition", &transition),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let has_rest = pattern
                .steps
                .iter()
                .any(|step| matches!(step, BossPatternStep::Rest { .. }));
            assert!(
                has_rest,
                "{label} pattern must include at least one Rest beat so the \
                 player has breathing room — got steps {:?}",
                pattern.steps
            );
        }
    }

    #[test]
    fn gnu_ton_phase1_is_materially_longer_than_other_bosses() {
        let gnu_phase1 = match BossBehaviorProfile::gnu_ton().attack_pattern {
            BossAttackPattern::Scripted { phase1, .. } => phase1.total_duration(),
            _ => unreachable!(),
        };
        let warden = BossBehaviorProfile::clockwork_warden();
        let warden_cycle = warden.attack_windup + warden.attack_active + warden.attack_cooldown;
        assert!(
            gnu_phase1 > warden_cycle * 3.0,
            "gnu_ton phase1 ({gnu_phase1}s) should be much slower than the \
             clockwork warden cycle ({warden_cycle}s) — design intent is a \
             deliberate, memorizable rhythm"
        );
    }

    // `gnu_ton_scripted_advance_cycles_telegraph_strike_rest` deleted:
    // the cursor-through-steps invariant moved to
    // `brain::boss_pattern::tests::{boss_pattern_telegraph_step_updates_telegraph_profile_state,
    // boss_pattern_strike_step_emits_melee_intent,
    // boss_pattern_resets_cursor_on_phase_change}`. The runtime no
    // longer ticks the cursor (the brain does), so polling
    // `boss.update(...)` and reading `boss.telegraph_profile` is no
    // longer a meaningful exercise — those mirror fields are written
    // by the boss tick system, not advanced by the runtime.

    #[test]
    fn gnu_ton_hand_slam_anchors_to_drawn_hands() {
        // GNU-ton's transform sits on the shoulder ridge. Hand-slam
        // hitboxes should land *below* the shoulder (positive y) and on
        // opposite sides of it (one to the left, one to the right), no
        // matter how the sprite is resized. Earlier revisions pinned
        // these to absolute world-pixel thresholds (>400, >300) tuned to
        // a 384-tall frame; bumping the source PNG to 768×576 silently
        // broke the test even though the visual / hitbox correspondence
        // stayed correct. Stick to invariants instead of magic numbers.
        let boss = gnu_ton_runtime();
        // Note: after the 2026-05-26 data-driven migration, this
        // exercises the `volumes_for_profile` FALLBACK math for the
        // `HandSlam` arm — the live game routes through
        // `sprite_authored_volumes` reading the
        // `gnu_ton_boss_spritesheet.ron` `gnu_hand_slam` hitbox parts.
        // The fallback's combat_size-relative offsets keep the
        // left/right/below-pos invariants this test pins, so the
        // assertions still hold without needing a sprite_metrics
        // snapshot in the runtime fixture.
        let slam = crate::features::volumes_for_profile(
            &BossAttackProfile::Strike("hand_slam".to_string()),
            boss.kin.pos,
            boss.as_ref().combat_size(),
            &boss.config.behavior,
        );
        assert_eq!(slam.len(), 2);
        let (left, right) = if slam[0].center().x < slam[1].center().x {
            (&slam[0], &slam[1])
        } else {
            (&slam[1], &slam[0])
        };
        assert!(left.center().x < boss.kin.pos.x, "{slam:?}");
        assert!(right.center().x > boss.kin.pos.x, "{slam:?}");
        assert!(left.center().y > boss.kin.pos.y, "{slam:?}");
        assert!(right.center().y > boss.kin.pos.y, "{slam:?}");
    }

    // `gnu_ton_body_contact_does_not_damage_player` +
    // `boss_body_contact_attributes_the_attacking_boss_entity` deleted with fable
    // AD2: boss body-contact damage flows through the shared `apply_actor_contact_damage`
    // now (the boss's contact tuning is driven from `behavior.body_damage` at spawn),
    // not the deleted `boss_attack_damage` poll. The "body_damage = 0 ⇒ no contact"
    // gate is the spawn tuning (`body_contact_damage: body_damage > 0`); the attacker
    // stamp is the shared contact path's, exercised by `app/tests/boss_contact_iframes`.

    // `gnu_ton_scripted_patterns_skip_non_attacking_phases` deleted:
    // the "Dormant / Stagger / Death emit neutral intent + clear
    // attack-state mirror" invariant moved to
    // `brain::boss_pattern::tests::boss_pattern_brain_emits_neutral_in_non_attacking_phase`.
    // The runtime no longer chooses the pattern step, so polling
    // `boss.update(...)` and reading the mirror fields is no longer
    // the right exercise — the brain owns the gate.

    // The `gnu_ton_apple_rain_strike_emits_falling_apple_spawns`,
    // `gnu_ton_apple_rain_spawns_avoid_self_aabb`,
    // `gnu_ton_apple_rain_spawns_cover_full_arena_width`, and
    // `gnu_ton_apple_rain_resets_accumulator_when_strike_ends` tests
    // were deleted with Task B of the actor/brain follow-up plan.
    // They tested `BossRuntime::tick_apple_rain` directly, which no
    // longer exists. The same invariants (downward gravity, owner
    // prefix, self-aabb dodge, full-width coverage, reset-on-leave)
    // are now exercised in
    // `features/ecs/brain_effects.rs::tests` against the
    // EFFECTS consumer `spawn_gnu_apple_rain_from_special_messages`.

    #[test]
    fn gnu_ton_apple_rain_volumes_are_empty_so_contact_does_not_double_count() {
        // The strike's damage path goes through enemy projectiles, not
        // a stationary boss AABB. `volumes_for_profile(DebrisRain, …)`
        // must return an empty list so the regular contact-damage
        // check in `boss_attack_damage` doesn't ALSO hit the player
        // at the boss's position while apples are in flight.
        let boss = gnu_ton_runtime();
        assert!(
            crate::features::volumes_for_profile(
                &BossAttackProfile::Special("apple_rain".into()),
                boss.kin.pos,
                boss.as_ref().combat_size(),
                &boss.config.behavior,
            )
            .is_empty(),
            "apple-rain volumes must be empty — damage routes through projectiles"
        );
    }

    #[test]
    fn gnu_ton_head_is_always_damageable_but_descent_brings_it_lower() {
        // The head is always a valid hit target — the older "only
        // damageable during head_descent strike" rule made the boss
        // permanently invulnerable in Phase1 (no descent beat) and
        // therefore unkillable. Now the head is always hittable; the
        // descent window (signaled by `BossAttackState.active_profile
        // == HeadDescent`) just moves it down to player level so
        // the player doesn't have to climb. Both states must produce
        // exactly one head AABB.
        let boss = gnu_ton_runtime();
        let mut attack_state = ambition_characters::brain::BossAttackState::default();
        let rest_head = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_ref(boss.as_ref(), &attack_state),
        );
        assert_eq!(
            rest_head.len(),
            1,
            "head must always be a damageable target"
        );
        let rest_y = rest_head[0].center().y;
        // Rest head sits ABOVE the shoulder anchor (player must climb).
        assert!(
            rest_y < boss.kin.pos.y,
            "rest head should be above the shoulder anchor, got y={rest_y} vs pos.y={}",
            boss.kin.pos.y
        );

        attack_state.active_profile = Some(BossAttackProfile::Strike("head_descent".to_string()));
        let descent_head = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_ref(boss.as_ref(), &attack_state),
        );
        assert_eq!(descent_head.len(), 1);
        let descent_y = descent_head[0].center().y;
        // Descended head sits BELOW the shoulder anchor (at player level).
        assert!(
            descent_y > boss.kin.pos.y,
            "descent head should be below the shoulder anchor"
        );
        // And materially lower than the rest position — that's the
        // whole point of the vulnerability window.
        assert!(
            descent_y > rest_y + 50.0,
            "descent must drop the head meaningfully (got rest_y={rest_y}, descent_y={descent_y})"
        );
    }

    // -------------------------------------------------------------
    // Gradient Sentinel (clockwork_warden) — Scripted schedule sanity
    // -------------------------------------------------------------
    //
    // The Gradient Sentinel boss flipped from `Cycle` to `Scripted`
    // with 4 phases (intro/phase1/transition/phase2/enrage). These
    // tests pin design invariants so future schedule edits can't
    // silently drop the rest-beat windows the player needs, drop a
    // special profile so the EFFECTS consumer never fires, or
    // accidentally make the encounter too short to learn.

    #[test]
    fn gradient_sentinel_uses_scripted_pattern() {
        let behavior = BossBehaviorProfile::clockwork_warden();
        match behavior.attack_pattern {
            BossAttackPattern::Scripted { .. } => {}
            BossAttackPattern::Cycle => {
                panic!("Gradient Sentinel should use Scripted, not Cycle");
            }
        }
    }

    #[test]
    fn gradient_sentinel_every_phase_includes_rest_beats() {
        let BossAttackPattern::Scripted {
            intro,
            phase1,
            transition,
            phase2,
            enrage,
        } = BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted attack pattern");
        };
        for (label, pattern) in [
            ("intro", &intro),
            ("phase1", &phase1),
            ("transition", &transition),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let has_rest = pattern
                .steps
                .iter()
                .any(|s| matches!(s, BossPatternStep::Rest { .. }));
            assert!(
                has_rest,
                "{label} pattern must include at least one Rest beat — got {:?}",
                pattern.steps
            );
        }
    }

    /// Phase 1 should teach the player the HazardColumn + MemorizedVolley
    /// profiles (the new fundamentals) before phase 2 layers in
    /// hazards + minions. Without this, the player wouldn't see
    /// these attacks until phase 2 and the difficulty curve would
    /// spike sharply.
    #[test]
    fn gradient_sentinel_phase1_includes_gradient_lane_and_overfit_volley() {
        use ambition_characters::brain::BossAttackProfile;
        let BossAttackPattern::Scripted { phase1, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let profiles: Vec<_> = phase1
            .steps
            .iter()
            .filter_map(|s| match s {
                BossPatternStep::Telegraph { profile, .. }
                | BossPatternStep::Strike { profile, .. } => Some(profile.clone()),
                _ => None,
            })
            .collect();
        assert!(
            profiles.contains(&BossAttackProfile::Strike("hazard_column".to_string())),
            "phase1 must include HazardColumn — got {profiles:?}"
        );
        assert!(
            profiles.contains(&BossAttackProfile::Special("overfit_volley".into())),
            "phase1 must include overfit_volley — got {profiles:?}"
        );
    }

    /// Phase 2 introduces the hazard + minion specials. These are
    /// the "advanced" attacks; if phase 2 doesn't include them, the
    /// encounter degenerates into "phase 1 forever, but slightly
    /// faster", which defeats the design.
    #[test]
    fn gradient_sentinel_phase2_includes_all_advanced_specials() {
        use ambition_characters::brain::BossAttackProfile;
        let BossAttackPattern::Scripted { phase2, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let profiles: Vec<_> = phase2
            .steps
            .iter()
            .filter_map(|s| match s {
                BossPatternStep::Telegraph { profile, .. }
                | BossPatternStep::Strike { profile, .. } => Some(profile.clone()),
                _ => None,
            })
            .collect();
        for key in ["minima_trap", "saddle_point", "gradient_cascade"] {
            let required = BossAttackProfile::Special(key.into());
            assert!(
                profiles.contains(&required),
                "phase2 must include {required:?} — got {profiles:?}"
            );
        }
    }

    /// Every Strike profile in the schedule that `is_special()` must
    /// have a registered SpecialActionSpec via
    /// `boss_special_for_profile`. Otherwise the boss tick emits no
    /// Special message for that beat and the strike silently does
    /// nothing — the worst kind of design bug because the telegraph
    /// still plays.
    #[test]
    fn gradient_sentinel_every_special_strike_has_a_registered_spec() {
        let behavior = BossBehaviorProfile::clockwork_warden();
        let BossAttackPattern::Scripted {
            phase1,
            phase2,
            enrage,
            ..
        } = behavior.attack_pattern.clone()
        else {
            panic!("expected Scripted");
        };
        for (label, pattern) in [
            ("phase1", &phase1),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            for step in &pattern.steps {
                if let BossPatternStep::Strike { profile, .. } = step {
                    if profile.is_special() {
                        assert!(
                            boss_special_for_profile(profile).is_some(),
                            "{label} strike of {profile:?} has no registered \
                             SpecialActionSpec — boss_special_for_profile must \
                             return Some so tick_boss_brains_system can emit \
                             the Special message",
                        );
                    }
                }
            }
        }
    }

    /// Every Telegraph step must be immediately followed by a Strike
    /// step for the SAME profile. Otherwise the player sees a windup
    /// for an attack that never fires (or fires a different one),
    /// which breaks the "telegraph teaches the strike shape" contract.
    #[test]
    fn gradient_sentinel_telegraph_steps_are_paired_with_matching_strike() {
        let BossAttackPattern::Scripted {
            intro,
            phase1,
            phase2,
            enrage,
            ..
        } = BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        for (label, pattern) in [
            ("intro", &intro),
            ("phase1", &phase1),
            ("phase2", &phase2),
            ("enrage", &enrage),
        ] {
            let mut iter = pattern.steps.iter().peekable();
            while let Some(step) = iter.next() {
                if let BossPatternStep::Telegraph { profile, .. } = step {
                    let next = iter.peek().unwrap_or_else(|| {
                        panic!("{label} ends on a Telegraph without a matching Strike")
                    });
                    match next {
                        BossPatternStep::Strike {
                            profile: strike_profile,
                            ..
                        } => {
                            assert_eq!(
                                profile, strike_profile,
                                "{label} Telegraph({profile:?}) must be followed by \
                                 Strike({profile:?}), got Strike({strike_profile:?})",
                            );
                        }
                        other => panic!(
                            "{label} Telegraph({profile:?}) must be followed by a \
                             Strike — got {other:?}",
                        ),
                    }
                }
            }
        }
    }

    /// Phase 1 should be appreciably longer than the legacy
    /// Cycle-mode loop so the player has enough time to learn the
    /// schedule. Lower bound is intentionally loose — tighter
    /// numerical checks belong in the design doc, not the test.
    #[test]
    fn gradient_sentinel_phase1_loop_is_substantial() {
        let BossAttackPattern::Scripted { phase1, .. } =
            BossBehaviorProfile::clockwork_warden().attack_pattern
        else {
            panic!("expected Scripted");
        };
        let total = phase1.total_duration();
        assert!(
            total >= 12.0,
            "phase1 loop should be at least 12s for memorability, got {total}s",
        );
        assert!(
            total <= 30.0,
            "phase1 loop shouldn't exceed 30s or each cycle drags, got {total}s",
        );
    }

    /// Bosses used to write `self.pos` via a bespoke per-axis sweep
    /// against `boss_space_is_free`. With the brain→sim seam they
    /// run through the SAME `step_kinematic` primitive every other
    /// actor uses — so a wall placed in the chase path blocks them
    /// at the wall instead of relying on a parallel-but-different
    /// collision code path. This guards against future regressions
    /// where someone reintroduces a position-space write.
    #[test]
    fn boss_motion_respects_world_collision_against_a_wall() {
        let combat_size = ae::Vec2::new(80.0, 80.0);
        let spawn = ae::Vec2::new(200.0, 400.0);
        let aabb = ae::Aabb::new(spawn, combat_size * 0.5);
        // A boss IS an aerial actor since AS4c: drive the boss pattern's
        // `velocity_target` through the SHARED flight limb (the production path,
        // `integrate_boss_bodies` → `ActorMut::update`, direct-velocity). This
        // exercises the flight-limb wall-collision sweep — the same guard the old
        // bespoke float had — over the REAL integration a boss now uses.
        let mut seed = super::super::ecs::actor_clusters::ActorClusterSeed::new(
            "test_warden",
            "Clockwork Warden",
            aabb,
            ambition_characters::actor::CharacterBrain::Passive,
            &[],
        );
        seed.kin.size = combat_size;
        seed.kin.pos = spawn;
        // Aerial, direct-velocity, high-speed free-mover — matches the boss spawn
        // cluster (`boss_actor_cluster`).
        seed.surface.gravity_scale = 0.0;
        seed.config.tuning.is_aerial = true;
        seed.config.tuning.chase_speed = 1200.0;
        seed.config.tuning.max_run_speed = 1200.0;
        seed.config.tuning.flight_direct_velocity = true;
        seed.caps = crate::combat::CombatCapabilities {
            can_fly: true,
            ..Default::default()
        };
        seed.body = super::super::ecs::actor_clusters::ActorBody::from_caps(&seed.caps, true);
        let behavior = BossBehaviorProfile::clockwork_warden();
        // World: a wall at x=400 blocks any rightward chase past it.
        let world = ae::World::new(
            String::from("boss_collision_test"),
            ae::Vec2::new(1200.0, 800.0),
            ae::Vec2::new(100.0, 100.0),
            vec![
                ae::Block::solid(
                    String::from("floor"),
                    ae::Vec2::new(0.0, 760.0),
                    ae::Vec2::new(1200.0, 40.0),
                ),
                ae::Block::solid(
                    String::from("wall"),
                    ae::Vec2::new(400.0, 200.0),
                    ae::Vec2::new(40.0, 500.0),
                ),
            ],
        );
        // Place the player far to the right of the wall so the
        // AnchorSway profile pulls the boss as far right as its
        // chase_limit allows.
        let player_pos = ae::Vec2::new(1000.0, 400.0);
        // Build the brain cfg + state directly — the runtime no
        // longer ticks scripted attacks, so we drive
        // `tick_boss_pattern` ourselves and hand the resulting
        // `desired_vel` to `integrate_body`. This mirrors what
        // `tick_boss_brains_system` + `update_ecs_bosses` do in the
        // real schedule.
        use ambition_characters::brain::{
            tick_boss_pattern, BossAttackState, BossPatternCfg, BossPatternContext,
            BossPatternState,
        };
        let mut cfg = BossPatternCfg::neutral_test();
        cfg.aggressiveness = 1.0;
        cfg.pattern = behavior.attack_pattern.clone();
        cfg.movement = behavior.movement.clone();
        cfg.spawn = spawn;
        cfg.combat_size = combat_size;
        cfg.cycle_attack_windup = behavior.attack_windup.max(0.01);
        cfg.cycle_attack_active = behavior
            .attack_active
            .max(FeatureCombatTuning::default().boss_attack_active)
            .max(0.01);
        cfg.cycle_attack_cooldown = behavior.attack_cooldown.max(0.05);
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let dt = 1.0 / 60.0;
        let combat_tuning = FeatureCombatTuning::default();
        for _ in 0..600 {
            let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
            tick_boss_pattern(
                &cfg,
                &mut state,
                &BossPatternContext {
                    encounter_phase: crate::boss_encounter::BossEncounterPhase::Phase1,
                    actor_pos: seed.kin.pos,
                    target_pos: player_pos,
                    world_size: world.size,
                    front_wall_clearance: None,
                    dt,
                },
                &mut frame,
                &mut attack_state,
            );
            // Integrate through the shared flight limb (the boss's production path):
            // `flight_direct_velocity` takes `frame.velocity_target` verbatim, then
            // the pipeline collision-resolves against the wall.
            seed.update_for_test(
                &world,
                player_pos,
                combat_tuning,
                None,
                dt,
                false,
                frame,
                ae::Vec2::new(0.0, 1.0),
            );
        }
        let boss_right_edge = seed.kin.pos.x + combat_size.x * 0.5;
        let wall_left_edge = 400.0;
        assert!(
            boss_right_edge <= wall_left_edge + 0.5,
            "boss clipped into wall at pos {:?} (right edge {}); wall left edge {}",
            seed.kin.pos,
            boss_right_edge,
            wall_left_edge,
        );
    }
}
