use super::*;

fn scripted_two_step_phase1(strike_profile: BossAttackProfile) -> BossAttackPattern {
    let phase1 = BossPattern {
        steps: vec![
            BossPatternStep::Telegraph {
                profile: strike_profile.clone(),
                duration: 0.5,
            },
            BossPatternStep::Strike {
                profile: strike_profile.clone(),
                duration: 0.4,
            },
            BossPatternStep::Rest { duration: 0.3 },
        ],
    };
    BossAttackPattern::Scripted {
        intro: BossPattern::default(),
        phase1,
        transition: BossPattern::default(),
        phase2: BossPattern::default(),
        enrage: BossPattern::default(),
    }
}

fn ctx(phase: BossEncounterPhase, dt: f32) -> BossPatternContext {
    BossPatternContext {
        encounter_phase: phase,
        actor_pos: ae::Vec2::ZERO,
        target_pos: ae::Vec2::new(50.0, 0.0),
        world_size: ae::Vec2::new(2_000.0, 2_000.0),
        front_wall_clearance: None,
        dt,
    }
}

fn cfg_with(pattern: BossAttackPattern) -> BossPatternCfg {
    let mut c = BossPatternCfg::neutral_test();
    c.aggressiveness = 1.0;
    c.pattern = pattern;
    c
}

#[test]
fn boss_pattern_brain_emits_neutral_in_non_attacking_phase() {
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.melee_pressed = true; // pre-poison
    out.special_pressed = true;

    tick_boss_pattern(
        &cfg,
        &mut state,
        &ctx(BossEncounterPhase::Dormant, 1.0 / 60.0),
        &mut out,
        &mut attack_state,
    );

    assert!(!out.melee_pressed, "dormant phase must not emit melee");
    assert!(!out.special_pressed, "dormant phase must not emit special");
    assert!(attack_state.active_profile.is_none());
    assert!(attack_state.telegraph_profile.is_none());
}

#[test]
fn boss_pattern_resets_cursor_on_phase_change() {
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();

    // Tick a while in Phase1 to advance the cursor past step 0.
    for _ in 0..30 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(BossEncounterPhase::Phase1, 0.05),
            &mut out,
            &mut attack_state,
        );
    }
    assert!(
        state.step_index > 0 || state.step_elapsed > 0.0,
        "cursor should have moved within phase1: index={} elapsed={}",
        state.step_index,
        state.step_elapsed,
    );

    // Phase transition → cursor resets.
    tick_boss_pattern(
        &cfg,
        &mut state,
        &ctx(BossEncounterPhase::Phase2, 0.05),
        &mut out,
        &mut attack_state,
    );
    // After one tick of the new phase, the elapsed should be 0.05
    // and the index back at 0 (assuming step 0 is longer than dt).
    assert_eq!(state.step_index, 0);
    assert!(state.step_elapsed <= 0.05 + 1e-6);
}

#[test]
fn boss_pattern_telegraph_step_updates_telegraph_profile_state() {
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();

    // First tick — step 0 is Telegraph with 0.5s.
    tick_boss_pattern(
        &cfg,
        &mut state,
        &ctx(BossEncounterPhase::Phase1, 0.1),
        &mut out,
        &mut attack_state,
    );

    assert_eq!(
        attack_state.telegraph_profile,
        Some(BossAttackProfile::FloorSlam)
    );
    assert!(attack_state.active_profile.is_none());
    assert!(!out.melee_pressed, "telegraph must not emit melee");
    assert!(!out.special_pressed, "telegraph must not emit special");
}

#[test]
fn boss_pattern_strike_step_emits_melee_intent() {
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();

    // Walk past the telegraph (0.5s) to land in the strike step.
    for _ in 0..6 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(BossEncounterPhase::Phase1, 0.1),
            &mut out,
            &mut attack_state,
        );
    }

    assert_eq!(
        attack_state.active_profile,
        Some(BossAttackProfile::FloorSlam),
        "should be in Strike step after walking past 0.5s telegraph",
    );
    assert!(
        out.melee_pressed,
        "non-special Strike profile must emit melee_pressed",
    );
    assert!(
        !out.special_pressed,
        "non-special Strike profile must NOT emit special_pressed",
    );
}

#[test]
fn debris_rain_strike_emits_special_intent() {
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::Special(
        "apple_rain".into(),
    )));
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();

    // Walk past the telegraph.
    for _ in 0..6 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(BossEncounterPhase::Phase1, 0.1),
            &mut out,
            &mut attack_state,
        );
    }

    assert_eq!(
        attack_state.active_profile,
        Some(BossAttackProfile::Special("apple_rain".into())),
    );
    assert!(
        out.special_pressed,
        "DebrisRain Strike must emit special_pressed (routes through SpecialActionSpec)",
    );
    assert!(
        !out.melee_pressed,
        "special-typed profile must NOT emit melee_pressed",
    );
}

#[test]
fn boss_pattern_cycle_advances_through_phases() {
    let mut cfg = cfg_with(BossAttackPattern::Cycle);
    cfg.spawn = ae::Vec2::ZERO;
    cfg.cycle_attack_cooldown = 0.2;
    cfg.cycle_attack_windup = 0.2;
    cfg.cycle_attack_active = 0.2;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();

    // Cooldown → Windup edge.
    tick_boss_pattern(
        &cfg,
        &mut state,
        &ctx(BossEncounterPhase::Phase1, 0.25),
        &mut out,
        &mut attack_state,
    );
    assert_eq!(state.cycle_phase, CyclePhase::Windup);
    assert!(attack_state.telegraph_profile.is_some());
    assert!(!out.melee_pressed);

    // Windup → Active edge.
    tick_boss_pattern(
        &cfg,
        &mut state,
        &ctx(BossEncounterPhase::Phase1, 0.25),
        &mut out,
        &mut attack_state,
    );
    assert_eq!(state.cycle_phase, CyclePhase::Active);
    assert!(attack_state.active_profile.is_some());
    assert!(out.melee_pressed, "cycle Active phase must emit melee");
}

#[test]
fn movement_for_phase_falls_back_to_default_when_overrides_unset() {
    let cfg = BossPatternCfg::neutral_test();
    for phase in [
        BossEncounterPhase::Phase1,
        BossEncounterPhase::Phase2,
        BossEncounterPhase::Transition,
        BossEncounterPhase::Enrage,
        BossEncounterPhase::Dormant,
    ] {
        assert_eq!(
            cfg.movement_for_phase(phase),
            &cfg.movement,
            "phase {phase:?} should fall back to default movement when override is None",
        );
    }
}

#[test]
fn movement_for_phase_picks_phase2_override_when_set() {
    let mut cfg = BossPatternCfg::neutral_test();
    let p2 = BossMovementProfile::AirSwoop {
        x_radius: 200.0,
        y_radius: 50.0,
        x_frequency: 1.0,
        y_frequency: 1.0,
        chase_scale: 0.2,
        chase_limit: 100.0,
        speed: 300.0,
    };
    cfg.movement_phase2 = Some(p2.clone());
    assert_eq!(
        cfg.movement_for_phase(BossEncounterPhase::Phase2),
        &p2,
        "Phase2 should use the phase2 override",
    );
    assert_eq!(
        cfg.movement_for_phase(BossEncounterPhase::Transition),
        &p2,
        "Transition routes through the phase2 override too — keeps motion continuous across the music swap",
    );
    // Phase1 still falls back to default.
    assert_eq!(
        cfg.movement_for_phase(BossEncounterPhase::Phase1),
        &cfg.movement,
    );
}

#[test]
fn movement_for_phase_picks_enrage_override_when_set() {
    let mut cfg = BossPatternCfg::neutral_test();
    let enrage = BossMovementProfile::AirSwoop {
        x_radius: 400.0,
        y_radius: 200.0,
        x_frequency: 1.5,
        y_frequency: 1.5,
        chase_scale: 0.6,
        chase_limit: 300.0,
        speed: 500.0,
    };
    cfg.movement_enrage = Some(enrage.clone());
    assert_eq!(cfg.movement_for_phase(BossEncounterPhase::Enrage), &enrage,);
    // Other phases unchanged.
    assert_eq!(
        cfg.movement_for_phase(BossEncounterPhase::Phase1),
        &cfg.movement,
    );
}

/// During an active special strike, `strike_speed_scale` should
/// shrink the emitted desired_vel so World-anchored hitboxes
/// (saddle cross, minima pit) stay centered on the boss.
#[test]
fn strike_speed_scale_reduces_velocity_during_active_special() {
    let mut cfg = cfg_with(BossAttackPattern::Cycle);
    cfg.movement = BossMovementProfile::AnchorSway {
        x_radius: 200.0,
        y_bob: 0.0,
        x_frequency: 0.0,
        y_frequency: 0.0,
        chase_scale: 1.0,
        chase_limit: 1000.0,
        speed: 400.0,
    };
    cfg.spawn = ae::Vec2::ZERO;
    cfg.strike_speed_scale = 0.1;
    // Sample 1: no active strike — full speed.
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let mut ctx = ctx(BossEncounterPhase::Phase1, 1.0 / 60.0);
    ctx.target_pos = ae::Vec2::new(500.0, 0.0); // pull toward +x
    ctx.actor_pos = ae::Vec2::ZERO;
    tick_boss_pattern(&cfg, &mut state, &ctx, &mut out, &mut attack_state);
    let vel_no_strike = out.desired_vel.length();

    // Sample 2: active special strike — expect ~10% of the speed.
    // Manually set attack_state.active_profile to a special.
    let mut state2 = BossPatternState::default();
    let mut attack_state2 = BossAttackState::default();
    // Pre-poison so the brain detects the strike (cycle mode will
    // overwrite, but we test the scale on the active-emit path).
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    // Drive cycle forward to Active phase with a special profile.
    cfg.cycle_attacks = vec![BossAttackProfile::Special("overfit_volley".into())];
    cfg.cycle_attack_cooldown = 0.05;
    cfg.cycle_attack_windup = 0.01;
    cfg.cycle_attack_active = 5.0; // long active so subsequent ticks stay there
                                   // Tick twice to walk Cooldown→Windup→Active.
    let mut ctx2 = ctx;
    ctx2.dt = 0.06;
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    assert_eq!(
        attack_state2.active_profile,
        Some(BossAttackProfile::Special("overfit_volley".into())),
        "should be in active overfit_volley strike for the test",
    );
    let vel_in_strike = out2.desired_vel.length();
    assert!(
        vel_in_strike < vel_no_strike * 0.5,
        "expected speed during active special strike to be much lower than no-strike speed: {vel_in_strike} vs {vel_no_strike}",
    );
}

/// Regression: `strike_speed_scale` must also apply during
/// **melee** strikes, not just special strikes. The user
/// reported "boss just floats around and never attacks" because
/// the boss was chasing the player at 1.5× speed during the
/// Strike beat; the FollowOwner melee hitbox tracked the
/// moving boss but couldn't catch a player who was still
/// running. Now any active strike (melee or special) slows
/// the boss so the hitbox actually lands.
#[test]
fn strike_speed_scale_reduces_velocity_during_active_melee_too() {
    let mut cfg = cfg_with(BossAttackPattern::Cycle);
    cfg.movement = BossMovementProfile::AnchorSway {
        x_radius: 200.0,
        y_bob: 0.0,
        x_frequency: 0.0,
        y_frequency: 0.0,
        chase_scale: 1.0,
        chase_limit: 1000.0,
        speed: 400.0,
    };
    cfg.spawn = ae::Vec2::ZERO;
    cfg.strike_speed_scale = 0.1;
    // Drive the cycle to an Active phase with a MELEE profile
    // (FloorSlam — `is_special()` returns false). Without the
    // fix, vel_in_strike would equal vel_no_strike because
    // strike_speed_scale only triggered for specials.
    cfg.cycle_attacks = vec![BossAttackProfile::FloorSlam];
    cfg.cycle_attack_cooldown = 0.05;
    cfg.cycle_attack_windup = 0.01;
    cfg.cycle_attack_active = 5.0;

    let baseline_ctx = {
        let mut c = ctx(BossEncounterPhase::Phase1, 1.0 / 60.0);
        c.target_pos = ae::Vec2::new(500.0, 0.0);
        c.actor_pos = ae::Vec2::ZERO;
        c
    };

    // Sample 1: no active strike — full speed.
    let mut state1 = BossPatternState::default();
    let mut attack_state1 = BossAttackState::default();
    let mut out1 = crate::actor::control::ActorControlFrame::neutral();
    tick_boss_pattern(
        &cfg,
        &mut state1,
        &baseline_ctx,
        &mut out1,
        &mut attack_state1,
    );
    let vel_no_strike = out1.desired_vel.length();

    // Sample 2: active MELEE strike — expect heavy slowdown.
    let mut state2 = BossPatternState::default();
    let mut attack_state2 = BossAttackState::default();
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    let mut ctx2 = baseline_ctx;
    ctx2.dt = 0.06;
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    tick_boss_pattern(&cfg, &mut state2, &ctx2, &mut out2, &mut attack_state2);
    assert_eq!(
        attack_state2.active_profile,
        Some(BossAttackProfile::FloorSlam),
        "should be in active FloorSlam strike for the test",
    );
    assert!(
        !attack_state2.active_profile.as_ref().unwrap().is_special(),
        "FloorSlam must not register as a special — this test guards against `is_special()` accidentally widening to melee profiles"
    );
    let vel_in_strike = out2.desired_vel.length();
    assert!(
        vel_in_strike < vel_no_strike * 0.5,
        "expected speed during active MELEE strike to be much lower than no-strike speed: {vel_in_strike} vs {vel_no_strike}",
    );
}

// -----------------------------------------------------------
// Macro state machine tests — chase / engage / retreat
// -----------------------------------------------------------

fn macro_cfg() -> BossPatternCfg {
    let mut cfg = cfg_with(BossAttackPattern::Cycle);
    cfg.spawn = ae::Vec2::new(640.0, 400.0);
    cfg.movement = BossMovementProfile::AnchorSway {
        x_radius: 100.0,
        y_bob: 0.0,
        x_frequency: 0.0,
        y_frequency: 0.0,
        chase_scale: 0.0,
        chase_limit: 0.0,
        speed: 200.0,
    };
    cfg.macro_tuning = BossMacroTuning {
        too_close_distance: 100.0,
        too_far_distance: 400.0,
        engage_distance: 200.0,
        approach_duration_s: 3.0,
        retreat_duration_s: 2.0,
        engage_max_duration_s: 8.0,
        front_wall_standoff: 48.0,
        idle_attack_chance_per_second: 0.0,
        hold_position_while_engaged: false,
        approach_speed_scale: 1.5,
        retreat_speed_scale: 0.8,
        retreat_distance: 250.0,
        suppress_attacks_while_moving: false,
    };
    cfg
}

fn macro_ctx(actor_pos: ae::Vec2, target_pos: ae::Vec2, dt: f32) -> BossPatternContext {
    BossPatternContext {
        encounter_phase: BossEncounterPhase::Phase1,
        actor_pos,
        target_pos,
        world_size: ae::Vec2::new(1_280.0, 768.0),
        front_wall_clearance: None,
        dt,
    }
}

/// Player far away → boss enters Approach state and moves
/// toward the player on the next tick.
#[test]
fn macro_state_transitions_to_approach_when_player_too_far() {
    let cfg = macro_cfg();
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let actor_pos = ae::Vec2::new(640.0, 400.0);
    let target_pos = ae::Vec2::new(1_100.0, 400.0); // ~460 px away > too_far(400)
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(actor_pos, target_pos, 0.05),
        &mut out,
        &mut attack_state,
    );
    assert!(
        matches!(state.macro_state, BossMacroState::Approach { .. }),
        "expected Approach with player far; got {:?}",
        state.macro_state,
    );
    // desired_vel should head toward the player (+x direction).
    assert!(
        out.desired_vel.x > 0.0,
        "Approach should chase toward player (positive x); got {:?}",
        out.desired_vel,
    );
}

/// Player very close → boss enters Retreat (anti-corner) and
/// moves AWAY from the player on the next tick.
#[test]
fn macro_state_transitions_to_retreat_when_player_too_close() {
    let cfg = macro_cfg();
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let actor_pos = ae::Vec2::new(640.0, 400.0);
    let target_pos = ae::Vec2::new(700.0, 400.0); // 60 px away < too_close(100)
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(actor_pos, target_pos, 0.05),
        &mut out,
        &mut attack_state,
    );
    assert!(
        matches!(state.macro_state, BossMacroState::Retreat { .. }),
        "expected Retreat with player too close; got {:?}",
        state.macro_state,
    );
    // desired_vel should head AWAY from the player (-x direction).
    assert!(
        out.desired_vel.x <= 0.0,
        "Retreat should move away from player (non-positive x); got {:?}",
        out.desired_vel,
    );
}

/// Boss in Engage for engage_max_duration_s automatically
/// transitions to Retreat — the "preparing something" beat
/// the player can read as "go chase the boss now."
#[test]
fn macro_state_periodically_retreats_after_engage_max_duration() {
    let cfg = macro_cfg();
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    // Mid-range distance — no too_close / too_far triggers.
    let actor_pos = ae::Vec2::new(640.0, 400.0);
    let target_pos = ae::Vec2::new(820.0, 400.0); // 180 px — within engage range
                                                  // Walk past engage_max_duration_s (8s) in 0.5s ticks.
    for _ in 0..18 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &macro_ctx(actor_pos, target_pos, 0.5),
            &mut out,
            &mut attack_state,
        );
    }
    assert!(
        matches!(state.macro_state, BossMacroState::Retreat { .. }),
        "expected periodic Retreat after engage_max_duration_s; got {:?}",
        state.macro_state,
    );
}

/// Approach ends and returns to Engage when the boss closes to
/// within `engage_distance` of the player.
#[test]
fn macro_state_approach_returns_to_engage_at_engage_distance() {
    let cfg = macro_cfg();
    let mut state = BossPatternState::default();
    state.macro_state = BossMacroState::Approach { remaining_s: 3.0 };
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let actor_pos = ae::Vec2::new(640.0, 400.0);
    let target_pos = ae::Vec2::new(740.0, 400.0); // 100 px < engage(200)
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(actor_pos, target_pos, 0.05),
        &mut out,
        &mut attack_state,
    );
    assert!(
        matches!(state.macro_state, BossMacroState::Engage),
        "Approach should drop back to Engage once within engage_distance",
    );
}

#[test]
fn macro_state_can_approach_even_when_player_is_close_if_retreat_disabled() {
    let mut cfg = macro_cfg();
    cfg.macro_tuning.too_close_distance = 0.0;
    cfg.macro_tuning.too_far_distance = 0.0;
    cfg.macro_tuning.engage_distance = 0.0;
    cfg.macro_tuning.approach_duration_s = 8.0;
    cfg.macro_tuning.hold_position_while_engaged = true;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let actor_pos = ae::Vec2::new(640.0, 400.0);
    let target_pos = ae::Vec2::new(700.0, 400.0); // close, but not yet overlapping
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(actor_pos, target_pos, 0.05),
        &mut out,
        &mut attack_state,
    );
    assert!(
        matches!(state.macro_state, BossMacroState::Approach { .. }),
        "retreat-disabled contact boss should approach, not back away: {:?}",
        state.macro_state,
    );
    assert!(
        out.desired_vel.x > 0.0,
        "expected chase toward player; got {:?}",
        out.desired_vel
    );
}

#[test]
fn contact_chase_mode_does_not_need_too_far_trigger() {
    let mut cfg = macro_cfg();
    cfg.macro_tuning.too_close_distance = 0.0;
    cfg.macro_tuning.too_far_distance = 0.0;
    cfg.macro_tuning.engage_distance = 0.0;
    cfg.macro_tuning.approach_duration_s = 8.0;
    cfg.macro_tuning.hold_position_while_engaged = true;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(
            ae::Vec2::new(640.0, 400.0),
            ae::Vec2::new(660.0, 400.0),
            0.05,
        ),
        &mut out,
        &mut attack_state,
    );
    assert!(
        matches!(state.macro_state, BossMacroState::Approach { .. }),
        "contact chase should close any horizontal gap when unblocked: {:?}",
        state.macro_state,
    );
    assert!(
        out.desired_vel.x > 0.0,
        "expected positive chase velocity; got {:?}",
        out.desired_vel
    );
}

#[test]
fn macro_state_holds_when_front_wall_is_inside_standoff() {
    let mut cfg = macro_cfg();
    cfg.macro_tuning.too_close_distance = 0.0;
    cfg.macro_tuning.too_far_distance = 0.0;
    cfg.macro_tuning.engage_distance = 0.0;
    cfg.macro_tuning.approach_duration_s = 8.0;
    cfg.macro_tuning.front_wall_standoff = 48.0;
    cfg.macro_tuning.hold_position_while_engaged = true;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    let mut ctx = macro_ctx(
        ae::Vec2::new(640.0, 400.0),
        ae::Vec2::new(900.0, 400.0),
        0.05,
    );
    ctx.front_wall_clearance = Some(32.0);
    tick_boss_pattern(&cfg, &mut state, &ctx, &mut out, &mut attack_state);
    assert_eq!(state.macro_state, BossMacroState::Engage);
    assert_eq!(out.desired_vel, ae::Vec2::ZERO);
}

#[test]
fn approach_clamps_to_front_wall_standoff_before_collision() {
    let mut cfg = macro_cfg();
    cfg.macro_tuning.front_wall_standoff = 48.0;
    let mut state = BossPatternState::default();
    state.macro_state = BossMacroState::Approach { remaining_s: 3.0 };
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    // Keep the player past `too_far_distance` (400) so the boss
    // stays in Approach and actually reaches the front-wall clamp.
    // A nearer player flips the macro state to Engage (the boss
    // holds instead of grinding into the wall), which is correct
    // but exercises a different path than this clamp test.
    let mut ctx = macro_ctx(
        ae::Vec2::new(640.0, 400.0),
        ae::Vec2::new(1_120.0, 400.0),
        0.10,
    );
    ctx.front_wall_clearance = Some(60.0);
    tick_boss_pattern(&cfg, &mut state, &ctx, &mut out, &mut attack_state);
    assert!(
        matches!(state.macro_state, BossMacroState::Approach { .. }),
        "player past too_far should keep the boss approaching; got {:?}",
        state.macro_state,
    );
    assert!(
        out.desired_vel.x > 0.0,
        "should still close toward the player"
    );
    assert!(
        out.desired_vel.x <= 120.1,
        "60px clearance with 48px standoff allows only a 12px/0.1s step; got {:?}",
        out.desired_vel,
    );
}

#[test]
fn idle_attack_chance_can_gate_rest_into_eye_beam() {
    let phase1 = BossPattern {
        steps: vec![
            BossPatternStep::Rest { duration: 0.1 },
            BossPatternStep::Telegraph {
                profile: BossAttackProfile::Special("eye_beam".into()),
                duration: 0.5,
            },
            BossPatternStep::Strike {
                profile: BossAttackProfile::Special("eye_beam".into()),
                duration: 0.25,
            },
        ],
    };
    let mut cfg = cfg_with(BossAttackPattern::Scripted {
        intro: BossPattern::default(),
        phase1,
        transition: BossPattern::default(),
        phase2: BossPattern::default(),
        enrage: BossPattern::default(),
    });
    cfg.macro_tuning.idle_attack_chance_per_second = 100.0;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_boss_pattern(
        &cfg,
        &mut state,
        &macro_ctx(
            ae::Vec2::new(640.0, 400.0),
            ae::Vec2::new(640.0, 400.0),
            0.11,
        ),
        &mut out,
        &mut attack_state,
    );
    assert!(matches!(
        attack_state.telegraph_profile,
        Some(BossAttackProfile::Special(ref k)) if k == "eye_beam"
    ));
}

/// Disabled macro tuning → boss permanently stays in Engage.
#[test]
fn macro_state_stays_engage_when_tuning_disabled() {
    let mut cfg = macro_cfg();
    cfg.macro_tuning = BossMacroTuning::disabled();
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    // Player very far — would normally trigger Approach.
    let actor_pos = ae::Vec2::new(0.0, 0.0);
    let target_pos = ae::Vec2::new(2_000.0, 0.0);
    for _ in 0..200 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &macro_ctx(actor_pos, target_pos, 0.1),
            &mut out,
            &mut attack_state,
        );
    }
    assert_eq!(
        state.macro_state,
        BossMacroState::Engage,
        "disabled tuning must never transition out of Engage",
    );
}

#[test]
fn peaceful_brain_does_not_emit_attack_intent() {
    // aggressiveness == 0 means the cursor still advances but the
    // attack-intent emit gate stays closed.
    let mut cfg = cfg_with(scripted_two_step_phase1(BossAttackProfile::FloorSlam));
    cfg.aggressiveness = 0.0;
    cfg.spawn = ae::Vec2::ZERO;
    let mut state = BossPatternState::default();
    let mut attack_state = BossAttackState::default();
    let mut out = crate::actor::control::ActorControlFrame::default();
    for _ in 0..10 {
        tick_boss_pattern(
            &cfg,
            &mut state,
            &ctx(BossEncounterPhase::Phase1, 0.1),
            &mut out,
            &mut attack_state,
        );
    }
    assert!(!out.melee_pressed);
    assert!(!out.special_pressed);
}
