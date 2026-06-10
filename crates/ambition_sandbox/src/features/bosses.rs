use super::*;

// Boss policy vocabulary (`BossMovementProfile`, `BossPatternStep`,
// `BossPattern`, `BossAttackPattern`, `BossAttackProfile`,
// `step_duration`) moved to `crate::brain::boss_pattern` per the
// "move boss policy out of BossRuntime" migration. Re-exported here
// because `BossBehaviorProfile` and the volumes / construction code
// below still reference them by their old `content::features::bosses`
// path — those references stay legal via the re-export while call
// sites migrate to the brain-module path at their leisure.
pub use crate::brain::boss_pattern::{BossAttackPattern, BossAttackProfile, BossMovementProfile};
// `BossPattern` and `BossPatternStep` only show up inside the
// scripted profiles, which now live in `boss_profiles.ron`. They're
// still publicly accessible via `crate::brain::boss_pattern`; we
// just don't re-export them here anymore.

// `BossTickOutputs` (previously: `projectile_spawns: Vec<…>`) was
// deleted with Task B of the actor/brain follow-up plan. Apple-rain
// spawning moved to `spawn_gnu_apple_rain_from_special_messages` (an
// EFFECTS-stage consumer driven by `ActorActionMessage::Special`).
// Future boss specials follow the same pattern — one consumer per
// `SpecialActionSpec` variant — instead of accumulating side-channel
// `Vec`s the caller flushes.

/// Encounter id of the gnu_ton boss — derived from
/// `encounter_id_from_name("GNU-ton")`. Centralized so the boss
/// ActionSet wiring (which binds the boss's special slot to
/// `SpecialActionSpec::GnuAppleRain`) can string-match without
/// re-deriving the slug.
pub const GNU_TON_ENCOUNTER_ID: &str = "gnu_ton";

/// Apple-rain tuning consumed by the spawn-time `ActionSet` wiring
/// (spawn.rs binds these into `SpecialActionSpec::GnuAppleRain`).
/// The visual / collision constants (gravity, lifetime, half_extent,
/// spawn-height) live next to the EFFECTS consumer in
/// `content/features/ecs/brain_effects.rs` — the consumer is the
/// only thing that reads them, so they're local there instead of
/// a cross-module knob set.
pub const APPLE_RAIN_INTERVAL: f32 = 0.35;
pub const APPLE_RAIN_SPAWN_SPEED: f32 = 35.0;
pub const APPLE_RAIN_DAMAGE: i32 = 1;
/// Stable id prefix used by the visuals layer to switch the
/// flat-red-rectangle bullet shape to the apple sprite (red body +
/// green leaf + brown stem). Keep in sync with
/// `enemy_projectile::visuals::is_apple_owner`.
pub const GNU_TON_APPLE_OWNER_PREFIX: &str = "gnu_ton_apple";

// Gradient Sentinel encounter id (per `BossEncounterSpec::gradient_sentinel`).
// Audit-engine name `clockwork_warden` resolves to the same boss via
// `BossBehaviorProfile::for_authored_boss`; both ids surface through the
// `BossEncounterRegistry`, but the canonical id used by the brain config
// and EFFECTS consumers is the public name.
pub const GRADIENT_SENTINEL_ENCOUNTER_ID: &str = "gradient_sentinel";

// ===== Gradient Sentinel special-attack tuning =====
//
// Constants kept here (next to the behavior profile that authors the
// schedule) so the EFFECTS consumers and the brain wiring share one
// source. The numeric values are tuned for the
// first_system_boss arena (1280×768) — see the design doc at
// `dev/journals/gradient-sentinel-boss-design-2026-05-25.md`.

/// OverfitVolley: how often (in seconds) the brain samples the
/// player's position during the telegraph window. With 5 samples and
/// 0.30 s spacing the consumer captures ~1.5 s of player travel,
/// covering a player who is reactively zig-zagging.
pub const OVERFIT_VOLLEY_SAMPLE_INTERVAL_S: f32 = 0.30;
/// OverfitVolley: max number of position samples to memorize. Caps the
/// bolt count fired on the strike edge so the player can read the
/// barrage instead of getting blanket-coverage'd.
pub const OVERFIT_VOLLEY_SAMPLE_COUNT: u8 = 5;
/// OverfitVolley: per-bolt projectile speed (px/s). Fast enough that
/// the bolts feel decisive but slow enough to dodge if the player
/// reads the barrage early.
pub const OVERFIT_VOLLEY_SHOT_SPEED: f32 = 360.0;
/// OverfitVolley: per-bolt damage.
pub const OVERFIT_VOLLEY_SHOT_DAMAGE: i32 = 1;

// ===== Smirking Behemoth / You Have To Cut The Rope tuning =====

/// Smirking Behemoth eye-beam projectile speed. Kept high because the
/// attack should read as a short bubble-laser line, not a slow barrage.
pub const SMIRKING_EYE_BEAM_SHOT_SPEED: f32 = 780.0;
pub const SMIRKING_EYE_BEAM_DAMAGE: i32 = 1;
pub const SMIRKING_EYE_BEAM_BOX_COUNT: u8 = 5;
pub const SMIRKING_EYE_BEAM_BOX_SPACING: f32 = 26.0;
pub const SMIRKING_EYE_BEAM_HALF_EXTENT_X: f32 = 15.0;
pub const SMIRKING_EYE_BEAM_HALF_EXTENT_Y: f32 = 8.0;
pub const SMIRKING_EYE_BEAM_LIFETIME_S: f32 = 0.58;

/// MinimaTrap: how long the pit hazard hitbox stays live after the
/// strike edge spawns it. Long enough to be a real area-denial threat,
/// short enough that the player isn't permanently locked out of half
/// the arena.
pub const MINIMA_TRAP_HAZARD_DURATION_S: f32 = 5.0;
/// MinimaTrap: per-tick damage. The standard `apply_hitbox_damage`
/// once-per-strike gate ensures one hit per pit lifetime.
pub const MINIMA_TRAP_DAMAGE: i32 = 2;
/// MinimaTrap: half-extent (x, y) of the pit hitbox.
pub const MINIMA_TRAP_HALF_EXTENT_X: f32 = 56.0;
pub const MINIMA_TRAP_HALF_EXTENT_Y: f32 = 24.0;

/// SaddlePoint: half-extent of each arm along its long axis.
pub const SADDLE_POINT_ARM_LENGTH: f32 = 220.0;
/// SaddlePoint: half-extent of each arm along its short axis.
pub const SADDLE_POINT_ARM_THICKNESS: f32 = 36.0;
/// SaddlePoint: seconds an axis stays active before toggling. The
/// brain's `BossPatternStep::Strike { duration }` governs total
/// strike time; this is just the rotation period.
pub const SADDLE_POINT_AXIS_PERIOD_S: f32 = 1.2;
/// SaddlePoint: per-tick damage.
pub const SADDLE_POINT_DAMAGE: i32 = 2;

/// GradientCascade: number of "slop" minions to spawn at the top of
/// the arena per strike. Kept low so the player can clear before
/// the next attack lands.
pub const GRADIENT_CASCADE_MINION_COUNT: u8 = 2;

// `GNU_TON_ANCHOR_Y`, `GNU_TON_COLLISION_SCALE`, `GNU_TON_FRAME_HEIGHT`,
// and `gnu_ton_sprite_scale` were retired in the 2026-05-26
// data-driven migration. The GNU-ton per-animation hit / hurt-box
// geometry lives in `gnu_ton_boss_spritesheet.ron`'s
// `body_metrics.animations` map and flows through the generic
// `world_aabb_from_pixel_rect` transform the gradient sentinel uses.

// `BossBehaviorProfile` / `BossRewardProfile` / `BossSpriteMetrics` /
// `canonical_boss_id_from` / `boss_animation_keys_for_profile` moved to
// `crate::boss_encounter::behavior` (Stage 20 / A2 stretch): the boss
// PROFILE vocabulary is machinery (data-driven via boss_profiles.ron);
// this module keeps the named special-spec resolver + tuning consts.
pub use crate::boss_encounter::behavior::{
    boss_animation_keys_for_profile, canonical_boss_id_from, BossBehaviorProfile,
    BossRewardProfile, BossSpriteMetrics,
};

/// Boss-side resolver for `Special`-flavored `BossAttackProfile`s.
///
/// The Gradient Sentinel carries multiple distinct specials
/// (OverfitVolley, MinimaTrap, SaddlePoint, GradientCascade) — more
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
    profile: &crate::brain::BossAttackProfile,
) -> Option<crate::brain::SpecialActionSpec> {
    use crate::brain::{BossAttackProfile, SpecialActionSpec};
    match profile {
        BossAttackProfile::GnuAppleRain => Some(SpecialActionSpec::GnuAppleRain {
            interval_s: APPLE_RAIN_INTERVAL,
            spawn_speed: APPLE_RAIN_SPAWN_SPEED,
            damage: APPLE_RAIN_DAMAGE,
        }),
        BossAttackProfile::OverfitVolley => Some(SpecialActionSpec::OverfitVolley {
            sample_interval_s: OVERFIT_VOLLEY_SAMPLE_INTERVAL_S,
            sample_count: OVERFIT_VOLLEY_SAMPLE_COUNT,
            shot_speed: OVERFIT_VOLLEY_SHOT_SPEED,
            damage: OVERFIT_VOLLEY_SHOT_DAMAGE,
        }),
        BossAttackProfile::EyeBeam => Some(SpecialActionSpec::EyeBeam {
            shot_speed: SMIRKING_EYE_BEAM_SHOT_SPEED,
            damage: SMIRKING_EYE_BEAM_DAMAGE,
            box_count: SMIRKING_EYE_BEAM_BOX_COUNT,
            box_spacing: SMIRKING_EYE_BEAM_BOX_SPACING,
            half_extent_x: SMIRKING_EYE_BEAM_HALF_EXTENT_X,
            half_extent_y: SMIRKING_EYE_BEAM_HALF_EXTENT_Y,
            lifetime_s: SMIRKING_EYE_BEAM_LIFETIME_S,
        }),
        BossAttackProfile::MinimaTrap => Some(SpecialActionSpec::MinimaTrap {
            hazard_duration_s: MINIMA_TRAP_HAZARD_DURATION_S,
            damage: MINIMA_TRAP_DAMAGE,
            half_extent_x: MINIMA_TRAP_HALF_EXTENT_X,
            half_extent_y: MINIMA_TRAP_HALF_EXTENT_Y,
            spawn_minion: true,
        }),
        BossAttackProfile::SaddlePoint => Some(SpecialActionSpec::SaddlePoint {
            arm_length: SADDLE_POINT_ARM_LENGTH,
            arm_thickness: SADDLE_POINT_ARM_THICKNESS,
            axis_period_s: SADDLE_POINT_AXIS_PERIOD_S,
            damage: SADDLE_POINT_DAMAGE,
        }),
        BossAttackProfile::GradientCascade => Some(SpecialActionSpec::GradientCascade {
            minion_count: GRADIENT_CASCADE_MINION_COUNT,
        }),
        // Ordinary melee profiles never route through this resolver
        // (they damage via `boss_attack_damage` reading `BossAttackState`
        // directly). The `_` arm keeps this function the single
        // source of truth for *which* special spec each profile maps to.
        _ => None,
    }
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
            &crate::actor::BossBrain::PhaseScript {
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
            &crate::actor::BossBrain::PhaseScript {
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
            &crate::actor::BossBrain::Custom("Clockwork Warden".to_string()),
        );
        assert_eq!(id, "clockwork_warden");
    }

    /// Dormant brain falls back to the display name.
    #[test]
    fn dormant_brain_falls_back_to_name() {
        let id = canonical_boss_id_from("Clockwork Warden", &crate::actor::BossBrain::Dormant);
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
            crate::actor::BossBrain::PhaseScript {
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
        use crate::brain::BossAttackProfile;
        for profile in [
            BossAttackProfile::OverfitVolley,
            BossAttackProfile::MinimaTrap,
            BossAttackProfile::SaddlePoint,
            BossAttackProfile::GradientCascade,
        ] {
            assert!(
                boss_special_for_profile(&profile).is_some(),
                "{profile:?} must resolve to a spec for Gradient Sentinel",
            );
        }
    }

    /// GNU-ton's apple rain still resolves through the new path so
    /// the consumer (`spawn_gnu_apple_rain_from_special_messages`)
    /// keeps receiving messages after the migration.
    #[test]
    fn gnu_apple_rain_profile_resolves_to_apple_rain_spec_for_gnu_ton() {
        use crate::brain::{BossAttackProfile, SpecialActionSpec};
        match boss_special_for_profile(&BossAttackProfile::GnuAppleRain) {
            Some(SpecialActionSpec::GnuAppleRain {
                interval_s,
                spawn_speed,
                damage,
            }) => {
                assert!((interval_s - APPLE_RAIN_INTERVAL).abs() < f32::EPSILON);
                assert!((spawn_speed - APPLE_RAIN_SPAWN_SPEED).abs() < f32::EPSILON);
                assert_eq!(damage, APPLE_RAIN_DAMAGE);
            }
            other => panic!("expected GnuAppleRain spec, got {other:?}"),
        }
    }

    /// Ordinary melee-style profiles return None — they don't go
    /// through the Special path; their damage routes via
    /// `boss_attack_damage` reading `BossAttackState` directly.
    #[test]
    fn ordinary_profiles_resolve_to_none() {
        use crate::brain::BossAttackProfile;
        for profile in [
            BossAttackProfile::FloorSlam,
            BossAttackProfile::SideSweep,
            BossAttackProfile::FullBodyPulse,
            BossAttackProfile::GradientLane,
        ] {
            assert!(
                boss_special_for_profile(&profile).is_none(),
                "{profile:?} should not have a Special spec",
            );
        }
    }
}

// `step_duration` moved to `crate::brain::boss_pattern`.

#[cfg(test)]
mod scripted_pattern_tests {
    use super::*;
    use crate::brain::boss_pattern::BossPatternStep;
    use crate::engine_core as ae;

    fn gnu_ton_runtime() -> super::super::ecs::boss_clusters::BossClusterScratch {
        let behavior = BossBehaviorProfile::gnu_ton();
        let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
        let pos = ae::Vec2::new(500.0, 400.0);
        let aabb = ae::Aabb::new(pos, combat_size * 0.5);
        let mut scratch = super::super::ecs::boss_clusters::BossClusterScratch::new(
            "boss_gnu_ton",
            "GNU-ton",
            aabb,
            crate::actor::BossBrain::Dormant,
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

    /// Build a minimal `BossSpriteMetrics` whose per-animation
    /// `hurtbox.parts` mirror the rest / descent head positions
    /// authored in the live spritesheet RON. Tests that exercise
    /// `damageable_volumes` use this so the head invariants stay
    /// pinned even though gnu_ton_runtime doesn't go through
    /// `derive_boss_sprite_metrics`.
    fn gnu_ton_sprite_metrics_fixture() -> super::BossSpriteMetrics {
        use crate::presentation::character_sprites::registry::{
            AnimationBox, AnimationMetrics, NamedPixelRect,
        };
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
                frames: Vec::new(),
            }),
            hitbox: None,
            frame_duration_secs: None,
        };
        let descent_entry = AnimationMetrics {
            hurtbox: Some(AnimationBox {
                parts: vec![head_descent_hurt],
                bbox: None,
                frames: Vec::new(),
            }),
            hitbox: Some(AnimationBox {
                parts: vec![head_descent_hit],
                bbox: None,
                frames: Vec::new(),
            }),
            frame_duration_secs: None,
        };
        let mut animations: HashMap<String, AnimationMetrics> = HashMap::new();
        animations.insert("rest".to_string(), rest_entry);
        animations.insert("gnu_head_descent".to_string(), descent_entry);
        super::BossSpriteMetrics {
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
        // `GnuHandSlam` arm — the live game routes through
        // `sprite_authored_volumes` reading the
        // `gnu_ton_boss_spritesheet.ron` `gnu_hand_slam` hitbox parts.
        // The fallback's combat_size-relative offsets keep the
        // left/right/below-pos invariants this test pins, so the
        // assertions still hold without needing a sprite_metrics
        // snapshot in the runtime fixture.
        let slam = crate::features::volumes_for_profile(
            &BossAttackProfile::GnuHandSlam,
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

    #[test]
    fn gnu_ton_body_contact_does_not_damage_player() {
        // `body_damage: 0` on the gnu_ton behavior is the authored
        // statement "no contact damage from the offscreen body". A prior
        // revision still dealt 1 damage because `player_damage` used
        // `body_damage.max(1)` after the intersect test. Now guarded by
        // the `body_damage > 0` check inside `boss_attack_damage`.
        // Concrete repro: a player AABB identical to the boss body
        // AABB with no active strike must produce no event.
        let boss = gnu_ton_runtime();
        let attack_state = crate::brain::BossAttackState::default();
        let ctx = crate::features::BossVolumeContext::from_ref(boss.as_ref(), &attack_state);
        let player_body =
            crate::features::body_damage_aabb(boss.kin.pos, boss.as_ref().combat_size());
        // Synthetic player entity — the test only checks the
        // None branch, the entity is never read out of the event.
        let synthetic_player =
            bevy::prelude::Entity::from_raw_u32(1).expect("nonzero raw entity index");
        assert!(
            crate::features::boss_attack_damage(&ctx, synthetic_player, player_body).is_none(),
            "gnu_ton must not deal contact damage when body_damage = 0"
        );
    }

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
    // `content/features/ecs/brain_effects.rs::tests` against the
    // EFFECTS consumer `spawn_gnu_apple_rain_from_special_messages`.

    #[test]
    fn gnu_ton_apple_rain_volumes_are_empty_so_contact_does_not_double_count() {
        // The strike's damage path goes through enemy projectiles, not
        // a stationary boss AABB. `volumes_for_profile(GnuAppleRain, …)`
        // must return an empty list so the regular contact-damage
        // check in `boss_attack_damage` doesn't ALSO hit the player
        // at the boss's position while apples are in flight.
        let boss = gnu_ton_runtime();
        assert!(
            crate::features::volumes_for_profile(
                &BossAttackProfile::GnuAppleRain,
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
        // == GnuHeadDescent`) just moves it down to player level so
        // the player doesn't have to climb. Both states must produce
        // exactly one head AABB.
        let boss = gnu_ton_runtime();
        let mut attack_state = crate::brain::BossAttackState::default();
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

        attack_state.active_profile = Some(BossAttackProfile::GnuHeadDescent);
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

    /// Phase 1 should teach the player the GradientLane + OverfitVolley
    /// profiles (the new fundamentals) before phase 2 layers in
    /// hazards + minions. Without this, the player wouldn't see
    /// these attacks until phase 2 and the difficulty curve would
    /// spike sharply.
    #[test]
    fn gradient_sentinel_phase1_includes_gradient_lane_and_overfit_volley() {
        use crate::brain::BossAttackProfile;
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
            profiles.contains(&BossAttackProfile::GradientLane),
            "phase1 must include GradientLane — got {profiles:?}"
        );
        assert!(
            profiles.contains(&BossAttackProfile::OverfitVolley),
            "phase1 must include OverfitVolley — got {profiles:?}"
        );
    }

    /// Phase 2 introduces the hazard + minion specials. These are
    /// the "advanced" attacks; if phase 2 doesn't include them, the
    /// encounter degenerates into "phase 1 forever, but slightly
    /// faster", which defeats the design.
    #[test]
    fn gradient_sentinel_phase2_includes_all_advanced_specials() {
        use crate::brain::BossAttackProfile;
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
        for required in [
            BossAttackProfile::MinimaTrap,
            BossAttackProfile::SaddlePoint,
            BossAttackProfile::GradientCascade,
        ] {
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
        let mut boss = super::super::ecs::boss_clusters::BossClusterScratch::new(
            "test_warden",
            "Clockwork Warden",
            aabb,
            crate::actor::BossBrain::Dormant,
        );
        boss.config.behavior = BossBehaviorProfile::clockwork_warden();
        boss.status.encounter_phase = crate::boss_encounter::BossEncounterPhase::Phase1;
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
        use crate::brain::{
            tick_boss_pattern, BossAttackState, BossPatternCfg, BossPatternContext,
            BossPatternState,
        };
        let mut cfg = BossPatternCfg::neutral_test();
        cfg.aggressiveness = 1.0;
        cfg.pattern = boss.config.behavior.attack_pattern.clone();
        cfg.movement = boss.config.behavior.movement.clone();
        cfg.spawn = boss.config.spawn;
        cfg.combat_size = boss.as_ref().combat_size();
        cfg.cycle_attack_windup = boss.config.behavior.attack_windup.max(0.01);
        cfg.cycle_attack_active = boss
            .config
            .behavior
            .attack_active
            .max(FeatureCombatTuning::default().boss_attack_active)
            .max(0.01);
        cfg.cycle_attack_cooldown = boss.config.behavior.attack_cooldown.max(0.05);
        let mut state = BossPatternState::default();
        let mut attack_state = BossAttackState::default();
        let dt = 1.0 / 60.0;
        for _ in 0..600 {
            let mut frame = crate::actor::control::ActorControlFrame::neutral();
            tick_boss_pattern(
                &cfg,
                &mut state,
                &BossPatternContext {
                    encounter_phase: boss.status.encounter_phase,
                    actor_pos: boss.kin.pos,
                    target_pos: player_pos,
                    world_size: world.size,
                    front_wall_clearance: None,
                    dt,
                },
                &mut frame,
                &mut attack_state,
            );
            boss.as_mut().integrate_body(&world, frame.desired_vel, dt);
        }
        let boss_right_edge = boss.kin.pos.x + boss.as_ref().combat_size().x * 0.5;
        let wall_left_edge = 400.0;
        assert!(
            boss_right_edge <= wall_left_edge + 0.5,
            "boss clipped into wall at pos {:?} (right edge {}); wall left edge {}",
            boss.kin.pos,
            boss_right_edge,
            wall_left_edge,
        );
    }
}
