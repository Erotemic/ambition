//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod scripted_pattern_tests` block (test-organization campaign, 2026-07-10).
//! Pure move: same test names + logic, now an adjacent child module with
//! private access via `use super::*;` (a direct sibling, so `super` depth is
//! unchanged).

use super::*;
use crate::features::FeatureCombatTuning;
use ambition_characters::brain::boss_pattern::BossPatternStep;
use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;

fn gnu_ton_runtime() -> super::super::ecs::boss_clusters::BossClusterScratch {
    let behavior = BossBehaviorProfile::gnu_ton_rider();
    let combat_size = behavior.combat_size.unwrap_or(ae::Vec2::new(220.0, 220.0));
    let pos = ae::Vec2::new(500.0, 400.0);
    let aabb = ae::Aabb::new(pos, combat_size * 0.5);
    let mut scratch = super::super::ecs::boss_clusters::BossClusterScratch::new(
        crate::boss_encounter::test_boss_catalog(),
        "boss_gnu_ton",
        "GNU-ton",
        aabb,
        ambition_entity_catalog::placements::BossBrain::Dormant,
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
        // Match what `sprite_render_size_for(catalog, giant_behavior, boss.size)`
        // would produce for a (220, 220) spawn → GIANT_GNU_SHEET's
        // 4.5× collision_scale: render = 990×990 with aspect
        // adjustment to 1320×990 for the 768/576 frame ratio.
        sprite_render_size: ae::Vec2::new(1320.0, 990.0),
        combat_offset: ae::Vec2::ZERO,
        animations,
    }
}

#[test]
fn gnu_ton_rider_pattern_includes_explicit_rest_beats_in_every_phase() {
    let BossAttackPattern::Scripted {
        phase1,
        transition,
        phase2,
        enrage,
        ..
    } = BossBehaviorProfile::gnu_ton_rider().attack_pattern
    else {
        panic!("the gnu_ton rider must use a Scripted attack pattern");
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
fn gnu_ton_rider_phase1_is_materially_longer_than_other_bosses() {
    let gnu_phase1 = match BossBehaviorProfile::gnu_ton_rider().attack_pattern {
        BossAttackPattern::Scripted { phase1, .. } => phase1.total_duration(),
        _ => unreachable!(),
    };
    let warden = BossBehaviorProfile::clockwork_warden();
    let warden_cycle = warden.attack_windup + warden.attack_active + warden.attack_cooldown;
    assert!(
        gnu_phase1 > warden_cycle * 3.0,
        "the gnu_ton rider's phase1 ({gnu_phase1}s) should be much slower than the \
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
// EFFECTS consumer `spawn_apple_rain_from_special_messages`.

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
    let rest_head =
        crate::features::damageable_volumes(&crate::features::BossVolumeContext::from_ref(
            crate::boss_encounter::test_boss_catalog(),
            boss.as_ref(),
            &attack_state,
        ));
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
    let descent_head =
        crate::features::damageable_volumes(&crate::features::BossVolumeContext::from_ref(
            crate::boss_encounter::test_boss_catalog(),
            boss.as_ref(),
            &attack_state,
        ));
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

/// Every content-technique profile in the authored pattern must become a
/// sustained effect on the canonical boss moveset. This replaces the deleted
/// direct-special resolver check: the move itself is now the executable wiring.
#[test]
fn gradient_sentinel_every_special_strike_is_a_moveset_effect() {
    use ambition_characters::brain::{BossCapability, BossPatternCfg};

    let behavior = BossBehaviorProfile::clockwork_warden();
    let mut cfg = BossPatternCfg::neutral_test();
    cfg.aggressiveness = 1.0;
    cfg.pattern = behavior.attack_pattern.clone();
    cfg.movement = behavior.movement.clone();
    cfg.movement_phase2 = behavior.movement_phase2.clone();
    cfg.movement_enrage = behavior.movement_enrage.clone();
    cfg.cycle_attack_windup = behavior.attack_windup.max(0.01);
    cfg.cycle_attack_active = behavior.attack_active.max(0.01);
    cfg.cycle_attack_cooldown = behavior.attack_cooldown.max(0.05);

    let capability = BossCapability::from_cfg(&cfg);
    let telegraph_windows = cfg.telegraph_windows();
    let moveset = boss_attack_moveset(
        &capability,
        &behavior,
        behavior
            .combat_size
            .unwrap_or(ambition_engine_core::Vec2::new(100.0, 100.0)),
        &telegraph_windows,
    )
    .expect("the authored pattern has attack moves");

    let mut special_count = 0;
    for (profile, _) in &capability.specials {
        let Some(key) = profile.special_key() else {
            continue;
        };
        special_count += 1;
        let move_spec = moveset
            .0
            .move_by_id(&profile.move_id())
            .unwrap_or_else(|| panic!("missing move for special profile {profile:?}"));
        let effect = move_spec
            .windows
            .iter()
            .find_map(|window| window.sustain_effect.as_ref())
            .unwrap_or_else(|| panic!("special move {key:?} has no sustained effect"));
        assert_eq!(effect.key.as_str(), key);
    }
    assert!(
        special_count > 0,
        "fixture must exercise content techniques"
    );
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
        ambition_entity_catalog::placements::CharacterBrain::Passive,
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
    // A floating boss: is_aerial forces flight into the body's movement kit.
    seed.body = super::super::ecs::actor_clusters::ActorBody::from_kit(ae::AbilitySet::NONE, true);
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
        tick_boss_pattern, BossAttackIntent, BossPatternCfg, BossPatternContext, BossPatternState,
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
    let mut attack_intent = BossAttackIntent::default();
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
                actor_facing: 1.0,
                hp_current: 100,
                hp_max: 100,
                live_attack: None,
            },
            &mut frame,
            &mut attack_intent,
        );
        let mut model = crate::features::MotionModel::default();
        // Integrate through the shared flight limb (the boss's production path):
        // `flight_direct_velocity` takes `frame.velocity_target` verbatim, then
        // the pipeline collision-resolves against the wall.
        seed.update_for_test(
            &world,
            player_pos,
            combat_tuning,
            dt,
            false,
            frame,
            &mut model,
            ae::MotionFrame::from_direction(ae::Vec2::new(0.0, 1.0), ae::GRAVITY),
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
