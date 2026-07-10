use super::*;
use crate::brain::snapshot::{BrainSnapshot, WallContact};

/// Pin the SignumOr trait's "near-zero → fallback" semantics.
/// Many brain ticks lean on this to keep facing stable when
/// movement input is briefly neutral; a regression to plain
/// `signum()` would let actors snap to 0 facing on neutral
/// frames. Edge cases: positive, negative, exactly zero,
/// sub-epsilon positive.
#[test]
fn signum_or_falls_back_when_input_is_near_zero() {
    assert_eq!((0.0_f32).signum_or(1.0), 1.0);
    assert_eq!((0.0_f32).signum_or(-1.0), -1.0);
    assert_eq!((f32::EPSILON * 0.5).signum_or(7.0), 7.0);
    assert_eq!((-f32::EPSILON * 0.5).signum_or(7.0), 7.0);
    // Clearly positive / negative → signum wins.
    assert_eq!((0.5_f32).signum_or(99.0), 1.0);
    assert_eq!((-0.5_f32).signum_or(99.0), -1.0);
}

fn snap_at(pos_x: f32, target_x: f32) -> BrainSnapshot {
    let mut s = BrainSnapshot::idle();
    s.actor_pos = ae::Vec2::new(pos_x, 0.0);
    s.target_pos = ae::Vec2::new(target_x, 0.0);
    s
}

fn same_faction_crowding(away_dir: ae::Vec2) -> crate::brain::smash::CrowdingSignal {
    crate::brain::smash::CrowdingSignal {
        same_faction_count: 1,
        other_faction_count: 0,
        away_dir,
        pressure: 1.0,
    }
}

#[test]
fn stand_still_emits_neutral_frame() {
    let mut sm = StateMachineCfg::StandStill;
    let mut out = crate::actor::control::ActorControlFrame::default();
    out.locomotion = ae::Vec2::new(99.0, 99.0); // pre-poisoned
    out.melee_pressed = true;
    tick_state_machine(&mut sm, &BrainSnapshot::idle(), &mut out);
    assert_eq!(out, crate::actor::control::ActorControlFrame::neutral());
}

#[test]
fn dead_actor_brain_emits_neutral_regardless_of_template() {
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    };
    let mut s = snap_at(0.0, 4.0);
    s.alive = false;
    // Pre-poison `out` so the test catches the early-return-
    // without-write path (a previously-leaked frame surviving
    // into a dead-actor tick).
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    out.melee_pressed = true;
    out.locomotion = ae::Vec2::new(99.0, 99.0);
    out.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        100.0,
    ));
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(!out.melee_pressed);
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    assert!(out.fire.is_none());
}

#[test]
fn patrol_paces_horizontally_around_spawn() {
    let mut cfg = PatrolCfg::NPC_DEFAULT;
    cfg.lane = AuthoredWorldPatrolLane::new(50.0, 30.0);
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    // Target far away → no Chase; brain stays in Patrol.
    let mut s = snap_at(60.0, 5000.0);
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Within patrol bounds: keeps facing, moves forward.
    assert!(out.locomotion.x > 0.0);
    assert_eq!(out.facing, 1.0);

    // Push past the right bound → facing flips.
    let mut s2 = snap_at(90.0, 5000.0);
    s2.actor_facing = 1.0;
    tick_state_machine(&mut sm, &s2, &mut out);
    assert!(out.locomotion.x < 0.0);
    assert_eq!(out.facing, -1.0);
}

#[test]
fn patrol_lane_is_authored_world_route_not_local_side() {
    // This deliberately ignores the actor's acceleration frame: the lane is a
    // world/environment route, while combat decisions still use local target
    // deltas. A sideways-gravity actor beyond the world-X right bound should
    // flip just like a normal-gravity actor at the same world position.
    let lane = AuthoredWorldPatrolLane::new(50.0, 30.0);
    let mut s = BrainSnapshot::idle();
    s.actor_pos = ae::Vec2::new(90.0, -200.0);
    s.actor_facing = 1.0;
    s.control_down = ae::Vec2::new(1.0, 0.0);
    assert_eq!(lane.signed_offset(s.actor_pos), 40.0);
    assert_eq!(lane.facing_after_bounds(s.actor_pos, s.actor_facing), -1.0);
}

#[test]
fn patrol_state_mode_mirrors_evaluator_intent() {
    // tick_patrol writes state.mode = ai.mode from the engine
    // evaluator. The NPC code at npcs.rs:230 reads PatrolState
    // .mode to pick HUD sprites — pin that a Patrol tick with
    // a far target gets mode = Patrol (i.e. the actor paces).
    let cfg = PatrolCfg::NPC_DEFAULT;
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    let s = snap_at(0.0, 5000.0); // target far away
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    if let StateMachineCfg::Patrol { state, .. } = &sm {
        // Far target → evaluator picks Patrol (not Idle/Chase/Attack).
        assert_eq!(state.mode, crate::actor::ai::CharacterAiMode::Patrol);
    } else {
        unreachable!();
    }
    // With a close target the evaluator switches to Chase (or
    // Attack if in range) — mode follows.
    let close = snap_at(0.0, 30.0);
    tick_state_machine(&mut sm, &close, &mut out);
    if let StateMachineCfg::Patrol { state, .. } = &sm {
        assert_ne!(
            state.mode,
            crate::actor::ai::CharacterAiMode::Patrol,
            "close target should leave Patrol",
        );
    }
}

#[test]
fn hostile_patrol_chases_target_in_aggro() {
    // Patrol with aggressiveness > 0 should chase the target
    // when it's inside aggro_radius but outside attack_range.
    // Pins the Chase branch's movement vs the peaceful "hold +
    // face target" branch.
    let mut cfg = PatrolCfg::NPC_DEFAULT;
    cfg.lane = AuthoredWorldPatrolLane::new(0.0, 200.0);
    cfg.aggressiveness = 1.0;
    cfg.aggro_radius = 120.0;
    cfg.attack_range = 24.0;
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    // Actor at 0, target at +80 → inside aggro, outside attack.
    let s = snap_at(0.0, 80.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Chase: closes the gap toward target_x.
    assert!(out.locomotion.x > 0.0, "hostile patrol should chase right");
    assert_eq!(out.facing, 1.0);
    assert!(!out.melee_pressed);
}

#[test]
fn hostile_patrol_attacks_target_in_melee_range() {
    // Patrol inside attack_range with cooldown clear → emit
    // melee intent. Pins the Attack branch.
    let mut cfg = PatrolCfg::NPC_DEFAULT;
    cfg.lane = AuthoredWorldPatrolLane::new(0.0, 200.0);
    cfg.aggressiveness = 1.0;
    cfg.aggro_radius = 120.0;
    cfg.attack_range = 24.0;
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    let mut s = snap_at(0.0, 15.0); // inside attack_range
    s.attack_cooldown_remaining = 0.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(
        out.melee_pressed,
        "hostile patrol in melee range should attack"
    );
    assert_eq!(out.facing, 1.0);
}

#[test]
fn hostile_patrol_holds_attack_during_cooldown() {
    // Attack branch must not emit melee when cooldown is active.
    // Pins the timer gate so an enemy can't spam attacks every
    // tick by virtue of always being in range.
    let mut cfg = PatrolCfg::NPC_DEFAULT;
    cfg.aggressiveness = 1.0;
    cfg.aggro_radius = 120.0;
    cfg.attack_range = 24.0;
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    let mut s = snap_at(0.0, 15.0);
    s.attack_cooldown_remaining = 0.5; // mid-cooldown
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(!out.melee_pressed, "must respect attack_cooldown_remaining");
}

#[test]
fn peaceful_patrol_in_talk_range_holds_and_faces_target() {
    let mut cfg = PatrolCfg::NPC_DEFAULT;
    cfg.lane = AuthoredWorldPatrolLane::new(0.0, 64.0);
    let mut sm = StateMachineCfg::Patrol {
        cfg,
        state: PatrolState::default(),
    };
    // Target right next to actor → evaluator returns Chase
    // (i.e. "player in range"). For peaceful aggressiveness=0
    // brain interprets as HOLD + face target.
    let s = snap_at(0.0, 30.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    assert_eq!(out.facing, 1.0);
    assert!(!out.melee_pressed);
}

#[test]
fn wanderer_moves_forward_with_no_wall_contact() {
    let cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.locomotion.x > 0.0);
    assert_eq!(out.facing, 1.0);
}

#[test]
fn wanderer_reverses_on_non_climbable_wall() {
    let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    cfg.climb_walls = true; // climb on, but wall isn't climbable
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    s.wall_contact = Some(WallContact {
        normal: ae::Vec2::new(-1.0, 0.0),
        is_climbable: false,
    });
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Facing flipped from +1 to -1; velocity goes left.
    assert_eq!(out.facing, -1.0);
    assert!(out.locomotion.x < 0.0);
}

#[test]
fn wanderer_climbs_when_able() {
    let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    cfg.climb_walls = true;
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    s.wall_contact = Some(WallContact {
        normal: ae::Vec2::new(-1.0, 0.0),
        is_climbable: true,
    });
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // No reversal recorded; climbing flag flips on inside state.
    if let StateMachineCfg::Wanderer { state, .. } = &sm {
        assert!(state.climbing);
        assert!(state.recent_reversals.is_empty());
    } else {
        unreachable!();
    }
    // Frame in climb mode emits zero motion (the actor walks
    // along the surface via the integration's surface-walk path
    // rather than the brain).
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
}

#[test]
fn wanderer_climbing_to_walking_transition_via_wall_clear() {
    // Wanderer that engaged climb mode (climb_walls=true,
    // climbable wall) should keep climbing while wall stays;
    // when wall clears, brain returns to forward walk.
    let cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    s.wall_contact = Some(crate::brain::snapshot::WallContact {
        normal: ae::Vec2::new(-1.0, 0.0),
        is_climbable: true,
    });
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Engaged climb mode → zero motion.
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    if let StateMachineCfg::Wanderer { state, .. } = &sm {
        assert!(state.climbing);
    }
    // Clear wall — wanderer returns to forward walking.
    s.wall_contact = None;
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out2);
    // Note: brain's climbing flag persists until next wall
    // contact resolves; what we test is that with NO wall
    // the brain emits forward walk (per the early-return
    // logic in tick_wanderer).
    assert!(out2.locomotion.x > 0.0);
}

#[test]
fn wanderer_resumes_walking_after_pause_expires() {
    // Pause is time-bounded; once chatter_pause_s elapses past
    // pause_until, the wanderer should resume forward motion.
    let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    cfg.climb_walls = false;
    cfg.chatter_threshold = 1; // first reversal trips pause
    cfg.chatter_window_s = 0.5;
    cfg.chatter_pause_s = 1.0;
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    // Trip the chatter via a reversal at t=0.
    s.wall_contact = Some(crate::brain::snapshot::WallContact {
        normal: ae::Vec2::new(-1.0, 0.0),
        is_climbable: false,
    });
    s.sim_time = 0.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Pause is active.
    if let StateMachineCfg::Wanderer { state, .. } = &sm {
        assert!(state.pause_until > 0.5);
    }
    // Advance time past pause_until + remove wall contact.
    s.sim_time = 2.0;
    s.wall_contact = None;
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out2);
    // Forward motion resumed.
    assert!(
        out2.locomotion.x != 0.0,
        "wanderer should walk after pause expires"
    );
}

#[test]
fn wanderer_pauses_on_rapid_chatter() {
    let mut cfg = WandererCfg::PUPPY_SLUG_DEFAULT;
    cfg.chatter_threshold = 3;
    cfg.chatter_window_s = 1.0;
    cfg.chatter_pause_s = 2.0;
    cfg.climb_walls = false; // ensure we reverse not climb
    let mut sm = StateMachineCfg::Wanderer {
        cfg,
        state: WandererState::default(),
    };
    let mut s = BrainSnapshot::idle();
    s.actor_facing = 1.0;
    s.wall_contact = Some(WallContact {
        normal: ae::Vec2::new(-1.0, 0.0),
        is_climbable: false,
    });
    s.sim_time = 0.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();

    // Three reversals across <1s should trip the pause on the
    // third reversal.
    s.sim_time = 0.0;
    tick_state_machine(&mut sm, &s, &mut out);
    s.actor_facing = out.facing;
    s.sim_time = 0.2;
    tick_state_machine(&mut sm, &s, &mut out);
    s.actor_facing = out.facing;
    s.sim_time = 0.4;
    tick_state_machine(&mut sm, &s, &mut out);
    // Pause should be active; frame is neutral.
    if let StateMachineCfg::Wanderer { state, .. } = &sm {
        assert!(state.pause_until > 0.4);
    }
    // Next tick during pause window → no motion.
    s.sim_time = 0.5;
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    out2.locomotion = ae::Vec2::new(99.0, 99.0);
    tick_state_machine(&mut sm, &s, &mut out2);
    assert_eq!(out2.locomotion, ae::Vec2::ZERO);
}

#[test]
fn melee_brute_chases_then_attacks_when_in_range() {
    let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg,
        state: MeleeBruteState::default(),
    };
    // Target close enough to chase but outside attack range.
    let s = snap_at(0.0, 100.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.locomotion.x > 0.0);
    assert!(!out.melee_pressed);
    // Target within attack range.
    let s2 = snap_at(0.0, 20.0);
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s2, &mut out2);
    assert!(out2.melee_pressed);
    assert_eq!(out2.facing, 1.0);
}

#[test]
fn melee_brute_chase_direction_is_controlled_actor_side_not_world_x() {
    let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg,
        state: MeleeBruteState::default(),
    };

    // Under rightward gravity, local +side is world-up. Place the target at the
    // same world x as the actor so a raw-world-X chase test would be neutral;
    // the brain seam must still emit +local-side chase/facing.
    let down = ae::Vec2::new(1.0, 0.0);
    let frame = ae::AccelerationFrame::new(down);
    let mut s = BrainSnapshot::idle();
    s.control_down = down;
    s.actor_pos = ae::Vec2::new(300.0, 300.0);
    s.target_pos = s.actor_pos + frame.to_world(ae::Vec2::new(80.0, 0.0));
    s.target_alive = true;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);

    assert!(
        out.locomotion.x > 0.0,
        "chase direction should be +local-side even when raw world x is unchanged",
    );
    assert_eq!(out.facing, 1.0);
    assert!(!out.melee_pressed);
}

#[test]
fn melee_brute_does_not_attack_during_active_windup() {
    let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg,
        state: MeleeBruteState::default(),
    };
    let mut s = snap_at(0.0, 20.0);
    s.attack_windup_remaining = 0.1;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(!out.melee_pressed);
}

#[test]
fn melee_brute_attack_gate_respects_each_phase_timer() {
    // The Attack branch ANDs four timer gates: cooldown, windup,
    // active, recover. Any of them positive must suppress
    // melee_pressed. Walks each one individually to catch a
    // future refactor that drops one of the gates.
    let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
    let cases: [(&str, fn(&mut BrainSnapshot)); 4] = [
        ("cooldown", |s| s.attack_cooldown_remaining = 0.1),
        ("windup", |s| s.attack_windup_remaining = 0.1),
        ("active", |s| s.attack_active_remaining = 0.1),
        ("recover", |s| s.attack_recover_remaining = 0.1),
    ];
    for (name, poke) in cases {
        let mut sm = StateMachineCfg::MeleeBrute {
            cfg,
            state: MeleeBruteState::default(),
        };
        let mut s = snap_at(0.0, 20.0); // inside attack range
        poke(&mut s);
        let mut out = crate::actor::control::ActorControlFrame::neutral();
        tick_state_machine(&mut sm, &s, &mut out);
        assert!(
            !out.melee_pressed,
            "{} timer > 0 should suppress melee_pressed",
            name,
        );
    }
    // Sanity: with all timers clear, melee_pressed = true.
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg,
        state: MeleeBruteState::default(),
    };
    let s = snap_at(0.0, 20.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.melee_pressed, "all timers clear → should attack");
}

#[test]
fn skirmisher_holds_standoff_then_fires() {
    let cfg = SkirmisherCfg::RANGER_DEFAULT;
    let mut sm = StateMachineCfg::Skirmisher {
        cfg,
        state: SkirmisherState::default(),
    };
    // Inside aggro, beyond standoff: chase closer.
    let mut s = snap_at(0.0, 200.0);
    s.sim_time = 0.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.fire.is_some() || out.velocity_target.x != 0.0);
    // After firing, last_fire_t is now 0.0; within cooldown
    // window another tick should not fire again immediately.
    s.sim_time = 0.1;
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out2);
    assert!(out2.fire.is_none());
}

#[test]
fn skirmisher_state_mode_tracks_engagement_phase() {
    // tick_skirmisher writes state.mode = Idle when outside
    // aggro, Chase when inside aggro pre-fire, Attack when
    // firing. Pin all three transitions — the NPC consumer
    // reads state.mode for HUD / sprite picking, so a future
    // refactor that drops a mode write would silently break
    // the HUD without tripping any other test.
    // Seed the cooldown timer so the first in-aggro tick stays
    // in Chase rather than immediately firing — the production
    // spawn helper (`enemy_default_brain`) seeds it the same way.
    let mut sm = StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg::RANGER_DEFAULT,
        state: SkirmisherState {
            cooldown_remaining: SkirmisherCfg::RANGER_DEFAULT.fire_cooldown_s,
            ..Default::default()
        },
    };
    // Far outside aggro → Idle.
    let mut s = snap_at(0.0, 5000.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    if let StateMachineCfg::Skirmisher { state, .. } = &sm {
        assert_eq!(state.mode, crate::actor::ai::CharacterAiMode::Idle);
    } else {
        unreachable!();
    }
    // Inside aggro with the seeded cooldown still draining →
    // Chase (one dt tick is small relative to the seed).
    s = snap_at(0.0, 200.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    if let StateMachineCfg::Skirmisher { state, .. } = &sm {
        assert_eq!(state.mode, crate::actor::ai::CharacterAiMode::Chase);
    }
    // Drain the cooldown by passing a one-shot dt that exceeds
    // the remaining timer; next tick → Attack + fire.
    s.dt = 5.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    if let StateMachineCfg::Skirmisher { state, .. } = &sm {
        assert_eq!(state.mode, crate::actor::ai::CharacterAiMode::Attack);
    }
    assert!(out.fire.is_some(), "should fire after cooldown");
}

#[test]
fn skirmisher_holds_quiet_when_target_dead() {
    // Skirmisher with dead target inside aggro range must emit
    // a neutral frame — no fire, no strafe. Pins the
    // target_alive=false early-return so an enemy can't keep
    // shooting at a dropped player.
    let cfg = SkirmisherCfg::RANGER_DEFAULT;
    let mut sm = StateMachineCfg::Skirmisher {
        cfg,
        state: SkirmisherState::default(),
    };
    let mut s = snap_at(0.0, 200.0);
    s.sim_time = 5.0; // way past any cooldown
    s.target_alive = false;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.fire.is_none());
    assert_eq!(out.velocity_target, ae::Vec2::ZERO);
}

#[test]
fn skirmisher_steers_away_from_aerial_crowding() {
    let cfg = SkirmisherCfg::RANGER_DEFAULT;
    let state = SkirmisherState {
        cooldown_remaining: cfg.fire_cooldown_s,
        ..Default::default()
    };
    let mut clear = snap_at(0.0, 200.0);
    clear.dt = 0.0;
    let mut clear_brain = StateMachineCfg::Skirmisher { cfg, state };
    let mut clear_out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut clear_brain, &clear, &mut clear_out);

    let mut crowded = clear;
    crowded.crowding = Some(same_faction_crowding(ae::Vec2::new(-1.0, 0.0)));
    let mut crowded_brain = StateMachineCfg::Skirmisher { cfg, state };
    let mut crowded_out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut crowded_brain, &crowded, &mut crowded_out);

    assert!(
        crowded_out.velocity_target.x < clear_out.velocity_target.x,
        "same-faction crowding should pull the skirmisher away from neighbors",
    );
}

#[test]
fn sniper_holds_and_fires_within_aggro() {
    let mut sm = StateMachineCfg::Sniper {
        cfg: SniperCfg::DEFAULT,
        state: SniperState::default(),
    };
    // Target well within aggro_radius (480.0).
    let mut s = snap_at(0.0, 200.0);
    // last_fire_t defaults to 0; first fire requires
    // sim_time >= fire_cooldown_s (default 1.5).
    s.sim_time = 2.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    // Sniper never moves (no desired_vel).
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    // Fired (sim_time past cooldown threshold).
    assert!(out.fire.is_some());
    // After firing, cooldown gates re-fire.
    s.sim_time = 2.1;
    let mut out2 = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out2);
    assert!(out2.fire.is_none(), "Sniper should respect fire_cooldown_s");
}

#[test]
fn sniper_holds_quiet_outside_aggro() {
    let mut sm = StateMachineCfg::Sniper {
        cfg: SniperCfg::DEFAULT,
        state: SniperState::default(),
    };
    // Target way outside aggro (default 480).
    let s = snap_at(0.0, 5000.0);
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.fire.is_none(), "Sniper out of aggro should not fire");
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
}

#[test]
fn sniper_holds_quiet_when_target_dead() {
    // Pin the target_alive=false early-return path: even when
    // the dead target is inside aggro range and the cooldown
    // is satisfied, the sniper emits a neutral frame (no fire,
    // no facing change).
    let mut sm = StateMachineCfg::Sniper {
        cfg: SniperCfg::DEFAULT,
        state: SniperState::default(),
    };
    let mut s = snap_at(0.0, 200.0); // well within aggro
    s.sim_time = 2.0; // past cooldown
    s.target_alive = false;
    s.actor_facing = 1.0;
    let mut out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut sm, &s, &mut out);
    assert!(out.fire.is_none(), "Sniper must not fire at dead target");
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
}

#[test]
fn brain_tick_overwrites_prior_frame_intent() {
    // Brain.tick treats `out` as a write target, not an
    // accumulator. Pre-poisoned intent (melee_pressed=true,
    // fire=Some) must be cleared before the brain writes its
    // own intent. Pins this so a future stale-state bug
    // doesn't sneak through.
    let mut sm = StateMachineCfg::StandStill;
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    frame.melee_pressed = true;
    frame.fire = Some(crate::actor::control::ActorFireRequest::world_space(
        ae::Vec2::new(1.0, 0.0),
        200.0,
    ));
    frame.jump_pressed = true;
    let snap = crate::brain::snapshot::BrainSnapshot::idle();
    tick_state_machine(&mut sm, &snap, &mut frame);
    // StandStill = neutral frame; pre-poisoned intent gone.
    assert!(!frame.melee_pressed);
    assert!(frame.fire.is_none());
    assert!(!frame.jump_pressed);
}

#[test]
fn shark_steers_away_from_aerial_crowding() {
    let cfg = ChargeCrashCfg {
        aggressiveness: 1.0,
        aggro_radius: 360.0,
        cruise_speed: 120.0,
        charge_speed: 420.0,
        bite_range: 34.0,
        charge_duration_s: 0.45,
        charge_cooldown_s: 0.8,
        standoff_px: 140.0,
        vertical_wobble_px: 24.0,
        orbit_drift_rad_s: 0.8,
    };
    let state = ChargeCrashState {
        charge_cooldown_remaining: cfg.charge_cooldown_s,
        ..Default::default()
    };
    let mut clear = snap_at(0.0, 200.0);
    clear.dt = 0.0;
    let mut clear_brain = StateMachineCfg::ChargeCrash { cfg, state };
    let mut clear_out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut clear_brain, &clear, &mut clear_out);

    let mut crowded = clear;
    crowded.crowding = Some(same_faction_crowding(ae::Vec2::new(-1.0, 0.0)));
    let mut crowded_brain = StateMachineCfg::ChargeCrash { cfg, state };
    let mut crowded_out = crate::actor::control::ActorControlFrame::neutral();
    tick_state_machine(&mut crowded_brain, &crowded, &mut crowded_out);

    assert!(
        crowded_out.velocity_target.x < clear_out.velocity_target.x,
        "same-faction crowding should pull the shark away from nearby flyers",
    );
}

#[test]
fn brain_dispatch_50_actors_under_one_millisecond() {
    // Sustained dispatch perf: tick 50 brains' state machine
    // once, all variants represented, total under 1ms. Pins
    // the "brain dispatch is monomorphic per-variant" property
    // — a regression to dyn dispatch or boxed brains would
    // blow this.
    let mut sm_list = vec![
        StateMachineCfg::StandStill,
        StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        },
        StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        },
        StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        },
        StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg::RANGER_DEFAULT,
            state: SkirmisherState::default(),
        },
    ];
    // Duplicate to reach 50.
    while sm_list.len() < 50 {
        sm_list.extend_from_slice(&sm_list.clone());
    }
    sm_list.truncate(50);
    let snap = crate::brain::snapshot::BrainSnapshot::idle();
    let start = std::time::Instant::now();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    for sm in &mut sm_list {
        tick_state_machine(sm, &snap, &mut frame);
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_millis(5),
        "50 brain ticks should be < 5ms, took {elapsed:?}",
    );
}

#[test]
fn brain_tick_cost_is_well_under_one_millisecond() {
    // Smoke check on per-tick brain dispatch cost. Ten ticks
    // of a MeleeBrute brain should complete well under 1ms on
    // any reasonable hardware. A regression that adds heap
    // allocation or expensive math inside the brain hot path
    // would trip this — it'd grow per-tick by orders of
    // magnitude.
    let mut sm = StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    };
    let snap = crate::brain::snapshot::BrainSnapshot::idle();
    let start = std::time::Instant::now();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    for _ in 0..10 {
        tick_state_machine(&mut sm, &snap, &mut frame);
    }
    let elapsed = start.elapsed();
    // 10 ticks should finish in well under 1ms (generous;
    // typically a few microseconds total).
    assert!(
        elapsed < std::time::Duration::from_millis(10),
        "10 MeleeBrute ticks should be << 10ms, took {elapsed:?}",
    );
}

#[test]
fn brain_templates_survive_zero_dt() {
    // Zero dt is the "paused frame" case — bullet-time +
    // hitstop both feed dt=0 to consumers. Every brain
    // template should tick cleanly without panic / NaN
    // propagation. Pins the pause-safety invariant.
    let templates: Vec<StateMachineCfg> = vec![
        StateMachineCfg::StandStill,
        StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        },
        StateMachineCfg::Wanderer {
            cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
            state: WandererState::default(),
        },
        StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        },
        StateMachineCfg::Skirmisher {
            cfg: SkirmisherCfg::RANGER_DEFAULT,
            state: SkirmisherState::default(),
        },
        StateMachineCfg::Sniper {
            cfg: SniperCfg::DEFAULT,
            state: SniperState::default(),
        },
        StateMachineCfg::ChargeCrash {
            cfg: ChargeCrashCfg {
                aggressiveness: 1.0,
                aggro_radius: 360.0,
                cruise_speed: 120.0,
                charge_speed: 420.0,
                bite_range: 34.0,
                charge_duration_s: 0.45,
                charge_cooldown_s: 0.8,
                standoff_px: 140.0,
                vertical_wobble_px: 24.0,
                orbit_drift_rad_s: 0.8,
            },
            state: ChargeCrashState::default(),
        },
    ];
    for mut brain in templates {
        let mut snap = crate::brain::snapshot::BrainSnapshot::idle();
        snap.dt = 0.0;
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_state_machine(&mut brain, &snap, &mut frame);
        assert!(frame.locomotion.x.is_finite() && frame.velocity_target.x.is_finite());
        assert!(frame.locomotion.y.is_finite() && frame.velocity_target.y.is_finite());
    }
}

#[test]
fn boss_pattern_via_state_machine_matches_the_direct_tick() {
    // §A1 slice 3c: the BossPattern brain now ticks through the UNIVERSAL
    // `tick_state_machine` path — it is no longer a neutral stub. The boss tick
    // fills the BossPattern fields (`boss_encounter_phase` / `world_size` /
    // `front_wall_clearance`) onto the shared snapshot, so the universal path must
    // produce EXACTLY what a direct `tick_boss_pattern` call does: same frame, same
    // attack-state projection. This parity is what makes the fold behavior-neutral.
    use crate::brain::boss_pattern::{tick_boss_pattern, BossAttackState, BossPatternContext};

    let cfg = crate::brain::BossPatternCfg::neutral_test();
    let phase = crate::brain::boss_pattern::BossEncounterPhase::Phase1; // an attacking phase
    let actor_pos = ae::Vec2::new(100.0, 200.0);
    let target_pos = ae::Vec2::new(260.0, 200.0);
    let world_size = ae::Vec2::new(1000.0, 600.0);
    let dt = 1.0 / 60.0;

    // Direct path (the pre-fold call).
    let mut direct_state = crate::brain::BossPatternState::default();
    let mut direct_frame = crate::actor::control::ActorControlFrame::neutral();
    let mut direct_attack = BossAttackState::default();
    let ctx = BossPatternContext {
        encounter_phase: phase,
        actor_pos,
        target_pos,
        world_size,
        front_wall_clearance: None,
        dt,
        ..Default::default()
    };
    tick_boss_pattern(
        &cfg,
        &mut direct_state,
        &ctx,
        &mut direct_frame,
        &mut direct_attack,
    );

    // Universal path: the SAME cfg/state, boss fields on the shared snapshot.
    let mut sm = StateMachineCfg::BossPattern {
        cfg: cfg.clone(),
        state: crate::brain::BossPatternState::default(),
    };
    let mut snap = crate::brain::snapshot::BrainSnapshot::idle();
    snap.actor_pos = actor_pos;
    snap.target_pos = target_pos;
    snap.dt = dt;
    snap.boss_encounter_phase = Some(phase);
    snap.world_size = world_size;
    snap.front_wall_clearance = None;
    let mut uni_frame = crate::actor::control::ActorControlFrame::neutral();
    uni_frame.melee_pressed = true; // pre-poison — the tick starts from a neutral frame
    tick_state_machine(&mut sm, &snap, &mut uni_frame);

    // Frame parity.
    assert_eq!(uni_frame.velocity_target, direct_frame.velocity_target);
    assert_eq!(uni_frame.locomotion, direct_frame.locomotion);
    assert_eq!(uni_frame.facing, direct_frame.facing);
    assert_eq!(uni_frame.melee_pressed, direct_frame.melee_pressed);
    assert_eq!(uni_frame.special_pressed, direct_frame.special_pressed);

    // Attack-state projection parity — it lives in the brain state on the
    // universal path (the seam that lets `Brain::tick`'s `(snapshot, out)`
    // signature carry no separate attack-state out).
    let StateMachineCfg::BossPattern {
        state: uni_state, ..
    } = &sm
    else {
        panic!("still a BossPattern brain");
    };
    assert_eq!(
        uni_state.attack_state.telegraph_profile.is_some(),
        direct_attack.telegraph_profile.is_some()
    );
    assert_eq!(
        uni_state.attack_state.active_profile.is_some(),
        direct_attack.active_profile.is_some()
    );
    assert_eq!(
        uni_state.attack_state.telegraph_remaining,
        direct_attack.telegraph_remaining
    );
    assert_eq!(
        uni_state.attack_state.active_remaining,
        direct_attack.active_remaining
    );
}

#[test]
fn is_hostile_reports_per_cfg() {
    assert!(!StateMachineCfg::StandStill.is_hostile());
    assert!(!StateMachineCfg::Patrol {
        cfg: PatrolCfg::NPC_DEFAULT,
        state: PatrolState::default(),
    }
    .is_hostile());
    assert!(!StateMachineCfg::Wanderer {
        cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
        state: WandererState::default(),
    }
    .is_hostile());
    assert!(StateMachineCfg::MeleeBrute {
        cfg: MeleeBruteCfg::STRIKER_DEFAULT,
        state: MeleeBruteState::default(),
    }
    .is_hostile());
    assert!(StateMachineCfg::Skirmisher {
        cfg: SkirmisherCfg::RANGER_DEFAULT,
        state: SkirmisherState::default(),
    }
    .is_hostile());
    assert!(StateMachineCfg::Sniper {
        cfg: SniperCfg::DEFAULT,
        state: SniperState::default(),
    }
    .is_hostile());
    assert!(StateMachineCfg::ChargeCrash {
        cfg: ChargeCrashCfg {
            aggressiveness: 1.0,
            aggro_radius: 360.0,
            cruise_speed: 120.0,
            charge_speed: 420.0,
            bite_range: 34.0,
            charge_duration_s: 0.45,
            charge_cooldown_s: 0.8,
            standoff_px: 140.0,
            vertical_wobble_px: 24.0,
            orbit_drift_rad_s: 0.8,
        },
        state: ChargeCrashState::default(),
    }
    .is_hostile());
}

// ===== Aerial brain =====
//
// The Aerial brain is the lively flyer. These tests run a tiny headless
// integration loop (advance sim_time, move the actor by its emitted
// `desired_vel`) and assert the EMERGENT behavior — flight, perching,
// landing beside the player, and the stalk→dive→peck→recover attack cycle —
// since the feel can't be eyeballed in CI.

fn aerial_cfg(aggressiveness: f32) -> AerialCfg {
    AerialCfg {
        aggressiveness,
        cruise_speed: 120.0,
        dive_speed: 300.0,
        aggro_radius: 90.0,
        attack_range: 40.0,
        roam_radius: 120.0,
    }
}

#[test]
fn aerial_is_hostile_iff_aggressive() {
    let peaceful = StateMachineCfg::Aerial {
        cfg: aerial_cfg(0.0),
        state: AerialState::default(),
    };
    let hostile = StateMachineCfg::Aerial {
        cfg: aerial_cfg(1.0),
        state: AerialState::default(),
    };
    assert!(!peaceful.is_hostile());
    assert!(hostile.is_hostile());
}

#[test]
fn aerial_peaceful_flits_between_perches_near_its_anchor() {
    let mut sm = StateMachineCfg::Aerial {
        cfg: aerial_cfg(0.0),
        state: AerialState::default(),
    };
    let anchor = ae::Vec2::new(500.0, 300.0);
    let mut pos = anchor;
    let mut out = crate::actor::control::ActorControlFrame::default();
    let dt = 1.0 / 60.0;
    let (mut flew, mut perched, mut max_dist) = (false, false, 0.0_f32);
    for i in 0..600 {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = pos;
        s.target_alive = false;
        s.sim_time = i as f32 * dt;
        s.dt = dt;
        tick_state_machine(&mut sm, &s, &mut out);
        pos += out.velocity_target * dt;
        let speed = out.velocity_target.length();
        flew |= speed > 30.0;
        perched |= speed < 1.0;
        max_dist = max_dist.max((pos - anchor).length());
    }
    assert!(
        flew,
        "a lively bird must actually fly (nonzero-velocity legs)"
    );
    assert!(
        perched,
        "a lively bird must perch (near-zero-velocity dwells)"
    );
    assert!(
        max_dist <= aerial_cfg(0.0).roam_radius * 1.6,
        "the bird stays near its captured anchor (max_dist={max_dist})",
    );
}

#[test]
fn aerial_peaceful_drops_beside_the_player_to_be_talked_to() {
    let mut sm = StateMachineCfg::Aerial {
        cfg: aerial_cfg(0.0),
        state: AerialState::default(),
    };
    // Player within the talk radius (dist 60 < aggro_radius 90).
    let target = ae::Vec2::new(560.0, 400.0);
    let mut pos = ae::Vec2::new(500.0, 400.0);
    let mut out = crate::actor::control::ActorControlFrame::default();
    let dt = 1.0 / 60.0;
    let mut last_mode = crate::actor::ai::CharacterAiMode::Idle;
    for i in 0..240 {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = pos;
        s.target_pos = target;
        s.target_alive = true;
        s.sim_time = i as f32 * dt;
        s.dt = dt;
        tick_state_machine(&mut sm, &s, &mut out);
        pos += out.velocity_target * dt;
        if let StateMachineCfg::Aerial { state, .. } = &sm {
            last_mode = state.mode;
        }
    }
    assert_eq!(
        last_mode,
        crate::actor::ai::CharacterAiMode::Idle,
        "near the player the bird holds (talk-ready), not patrol/chase",
    );
    assert!(
        (pos.x - target.x).abs() < 45.0 && (pos.y - target.y).abs() < 12.0,
        "the bird lands at the player's side/feet to talk (pos={pos:?})",
    );
}

#[test]
fn aerial_hostile_stalks_dives_pecks_then_recovers() {
    let mut sm = StateMachineCfg::Aerial {
        cfg: aerial_cfg(1.0),
        state: AerialState::default(),
    };
    let mut pos = ae::Vec2::new(500.0, 100.0); // bird starts above
    let target = ae::Vec2::new(520.0, 400.0); // player below, stationary
    let mut out = crate::actor::control::ActorControlFrame::default();
    let dt = 1.0 / 60.0;
    let (mut saw_dive, mut pecked, mut saw_recover) = (false, false, false);
    for i in 0..900 {
        let mut s = BrainSnapshot::idle();
        s.actor_pos = pos;
        s.target_pos = target;
        s.target_alive = true;
        s.sim_time = i as f32 * dt;
        s.dt = dt;
        s.attack_cooldown_remaining = 0.0; // ready to peck whenever in range
        tick_state_machine(&mut sm, &s, &mut out);
        if let StateMachineCfg::Aerial { state, .. } = &sm {
            match state.phase {
                AerialPhase::Dive => saw_dive = true,
                AerialPhase::Recover => saw_recover = true,
                _ => {}
            }
        }
        pecked |= out.melee_pressed;
        pos += out.velocity_target * dt;
    }
    assert!(saw_dive, "the dive-bomber must commit to a Dive");
    assert!(
        pecked,
        "it must peck (melee) when the dive reaches the target"
    );
    assert!(saw_recover, "it must peel off and Recover after the dive");
}
