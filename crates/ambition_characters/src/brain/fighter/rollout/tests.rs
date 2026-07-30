//! Unit properties for the shadow model (FB6b–d, fighter-brain.md §12).
//!
//! These assert MODEL properties and structural invariants — never "which
//! move a rollout picks at v1 tuning" beyond the constructed scenarios below,
//! because that is a calibration claim and calibration belongs to FB4's
//! ladder and FB6e's fidelity instrument.

use super::*;
use crate::actor::ActorFaction;
use crate::brain::fighter::habit::Choice;
use crate::brain::fighter::options::AttackOption;
use crate::perception::{PerceivedProjectile, WorldView};

fn frames(startup_s: f32, reach: f32, max_damage: i32, max_knockback: f32) -> MoveFrameData {
    MoveFrameData {
        total_s: startup_s + 0.1 + 0.2,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s: 0.2,
        cancel_windows: Vec::new(),
        reach,
        max_damage,
        max_knockback,
    }
}

fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
    }
}

fn view_with(me_x: f32, foe_x: f32) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos: ae::Vec2::new(me_x, 300.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            half_extent: ae::Vec2::new(12.0, 16.0),
            alive: true,
            on_ground: true,
            health_max: 100,
            ..Default::default()
        },
        stage: stage(),
        actors: vec![PerceivedActor {
            id: "foe".to_string(),
            pos: ae::Vec2::new(foe_x, 300.0),
            half_extent: ae::Vec2::new(12.0, 16.0),
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            health_max: 100,
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn state(me_x: f32, foe_x: f32) -> ShadowState {
    let view = view_with(me_x, foe_x);
    ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view")
}

fn profile(rollout_k: u32, rollout_depth: u32, read_weight: f32) -> FighterBrainProfile {
    FighterBrainProfile {
        level: 9,
        reaction_ms: 150.0,
        apm_cap: 360.0,
        execution_noise: 0.05,
        rollout_depth,
        rollout_k,
        read_weight,
        utility_weights: crate::brain::fighter::options::UtilityWeights::v1(),
    }
}

fn attack(id: &str, frames: MoveFrameData) -> AttackOption {
    AttackOption {
        move_id: id.to_string(),
        frames,
        score: 0.0,
        features: Default::default(),
    }
}

const DT: f32 = 1.0 / 60.0;

/// Step `n` ticks with both fighters holding; collect every event.
fn run(s: &mut ShadowState, n: u32, my: &ShadowIntent, tuning: &ShadowTuning) -> Vec<ShadowEvent> {
    let mut events = Vec::new();
    for _ in 0..n {
        events.extend(shadow_step(s, DT, my, &ShadowIntent::Hold, tuning));
    }
    events
}

// ── the model ────────────────────────────────────────────────────────────

#[test]
fn a_move_with_reach_connects_and_the_whiff_does_not() {
    let tuning = ShadowTuning::default();

    let mut s = state(300.0, 380.0); // gap 80
    s.me.phase = ShadowPhase::Move {
        frames: frames(0.1, 100.0, 8, 0.0),
        t: 0.0,
        landed: false,
    };
    let events = run(&mut s, 30, &ShadowIntent::Hold, &tuning);
    assert!(
        events.contains(&ShadowEvent::Hit {
            on_me: false,
            damage: 8
        }),
        "{events:?}"
    );
    assert_eq!(s.foe.damage, 8, "single-hit v1: one active span, one hit");

    let mut s = state(300.0, 700.0); // gap 400 — nothing reaches
    s.me.phase = ShadowPhase::Move {
        frames: frames(0.1, 100.0, 8, 0.0),
        t: 0.0,
        landed: false,
    };
    let events = run(&mut s, 30, &ShadowIntent::Hold, &tuning);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(s.foe.damage, 0);
}

/// The strike is the REAL kernel, not an imitation: the launched velocity and
/// the armed hitstun equal `ae::hit_response` called with the same inputs.
/// If someone re-inlines a private formula here, this is the test that goes
/// red when the kernel and the copy drift.
#[test]
fn the_hit_response_is_the_authoritative_kernel_not_an_imitation() {
    let tuning = ShadowTuning::default();
    let mut s = state(300.0, 380.0);
    let move_frames = frames(0.1, 100.0, 8, 500.0);
    s.me.phase = ShadowPhase::Move {
        frames: move_frames,
        t: 0.0,
        landed: false,
    };
    // Advance to just short of the active span (5 × 1/60 s < 0.1 s startup),
    // then take the one striking step.
    run(&mut s, 5, &ShadowIntent::Hold, &tuning);
    let foe_before = s.foe.clone();
    let me_before = s.me.clone();
    let events = shadow_step(&mut s, DT, &ShadowIntent::Hold, &ShadowIntent::Hold, &tuning);
    assert!(matches!(events[..], [ShadowEvent::Hit { on_me: false, .. }]));

    let kb = HitKnockback {
        dir: me_before.facing,
        magnitude: HitKnockbackMagnitude::LaunchSpeed(500.0),
        source_pos: me_before.pos,
        impact_pos: foe_before.pos,
        launch_dir: None,
    };
    let expected_vel = hit_response::knockback_velocity(
        foe_before.pos,
        foe_before.facing,
        s.gravity_down,
        Some(&kb),
        ae::Vec2::ZERO,
        &tuning.response,
    );
    let expected_stun = hit_response::hitstun_duration(Some(&kb), &tuning.response);
    // Hits resolve AFTER integration in the step order (§12.3), so at step end
    // the struck body carries exactly the kernel's launch — gravity first
    // touches it next tick.
    assert_eq!(s.foe.vel, expected_vel);
    assert_eq!(
        s.foe.phase,
        ShadowPhase::Hitstun {
            remaining: expected_stun
        }
    );
}

#[test]
fn a_launched_foe_offstage_in_hitstun_is_a_ko() {
    let tuning = ShadowTuning::default();
    // Stage x spans 0..800; the foe stands near the right blastzone.
    let mut s = state(700.0, 770.0);
    s.me.phase = ShadowPhase::Move {
        frames: frames(0.05, 80.0, 10, 900.0),
        t: 0.0,
        landed: false,
    };
    let events = run(&mut s, 30, &ShadowIntent::Hold, &tuning);
    assert!(
        events.contains(&ShadowEvent::Ko { of_me: false }),
        "a body reeling past the blastzone is a stock: {events:?}"
    );
    assert!(s.foe.koed);
}

#[test]
fn ballistic_projectiles_strike_the_body_they_are_flying_at() {
    let tuning = ShadowTuning::default();
    let mut s = state(300.0, 700.0);
    s.projectiles.push(ShadowProjectile {
        pos: ae::Vec2::new(380.0, 300.0),
        vel: ae::Vec2::new(-240.0, 0.0), // closing on me
        damage: 6,
    });
    s.projectiles.push(ShadowProjectile {
        pos: ae::Vec2::new(420.0, 300.0),
        vel: ae::Vec2::new(240.0, 0.0), // flying away
        damage: 6,
    });
    let events = run(&mut s, 40, &ShadowIntent::Hold, &tuning);
    assert_eq!(
        events,
        vec![ShadowEvent::Hit {
            on_me: true,
            damage: 6
        }],
        "exactly the closing projectile lands"
    );
    assert_eq!(s.me.damage, 6);
    assert_eq!(s.projectiles.len(), 1, "the landed projectile is spent");
}

#[test]
fn a_body_that_started_airborne_has_no_floor_to_land_on() {
    let tuning = ShadowTuning::default();
    let mut view = view_with(300.0, 500.0);
    view.self_view.on_ground = false;
    let mut s = ShadowState::from_perceived(Perceived::cheating(&view)).unwrap();
    assert_eq!(s.me.ground_level, None);
    let y0 = s.me.pos.y;
    run(&mut s, 60, &ShadowIntent::Hold, &tuning);
    assert!(
        s.me.pos.y > y0 + 100.0,
        "no invented terrain caught it: {} -> {}",
        y0,
        s.me.pos.y
    );
    assert!(!s.me.on_ground);
}

// ── the predicted opponent (FB6c) ────────────────────────────────────────

#[test]
fn prediction_uses_the_modal_habit_only_when_it_beats_chance() {
    let tuning = ShadowTuning::default();
    let s = state(300.0, 400.0);

    // A genuine read: they attack out of neutral, over and over.
    let mut habits = HabitModel::new(0.5);
    for _ in 0..6 {
        habits.observe(Situation::Neutral, Choice::Attack);
    }
    assert_eq!(
        predicted_foe_intent(&s, Situation::Neutral, &habits, 1.0, &tuning),
        ShadowIntent::StartAttack
    );

    // `read_weight = 0` never consults the model, however confident it is.
    assert_eq!(
        predicted_foe_intent(&s, Situation::Neutral, &habits, 0.0, &tuning),
        ShadowIntent::Hold,
        "a low rung predicts inertia, not habits"
    );

    // An empty model is the uniform prior: no read, inertia only.
    let empty = HabitModel::new(0.5);
    assert_eq!(
        predicted_foe_intent(&s, Situation::Neutral, &empty, 1.0, &tuning),
        ShadowIntent::Hold
    );

    // Inertia is direction-preserving: a foe walking left keeps walking left.
    let mut moving = state(300.0, 400.0);
    moving.foe.vel = ae::Vec2::new(-tuning.ground_speed, 0.0);
    match predicted_foe_intent(&moving, Situation::Neutral, &empty, 1.0, &tuning) {
        ShadowIntent::Drive { lateral } => assert!(lateral < 0.0),
        other => panic!("expected inertia drive, got {other:?}"),
    }
}

// ── refine_by_rollout (FB6d) ─────────────────────────────────────────────

#[test]
fn zero_depth_or_zero_k_is_l2s_order_unchanged() {
    let view = view_with(300.0, 380.0);
    let options = OptionSet {
        movement: Vec::new(),
        attacks: vec![attack("jab", frames(0.1, 100.0, 4, 0.0))],
    };
    let habits = HabitModel::new(0.5);
    let tuning = ShadowTuning::default();
    for (k, depth) in [(0, 20), (4, 0), (0, 0)] {
        assert_eq!(
            refine_by_rollout(
                Perceived::cheating(&view),
                Situation::Neutral,
                &options,
                &habits,
                &profile(k, depth, 0.0),
                &tuning,
                60.0,
            ),
            None,
            "k={k} depth={depth} must degrade to L2, not to a zero-step rollout"
        );
    }
}

/// The marquee property: L2's order said jab first (it is listed first); the
/// rollouts discover the jab whiffs at this gap and the lunge connects, and
/// re-rank. This is L3 buying something L2's features cannot see.
#[test]
fn the_rollout_prefers_the_move_that_actually_connects() {
    let view = view_with(300.0, 380.0); // gap 80
    let options = OptionSet {
        movement: Vec::new(),
        attacks: vec![
            attack("jab", frames(0.08, 40.0, 4, 0.0)), // 40 + 12 extent < 80: whiffs
            attack("lunge", frames(0.2, 100.0, 8, 300.0)), // connects
        ],
    };
    let habits = HabitModel::new(0.5);
    let refined = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Advantage,
        &options,
        &habits,
        &profile(4, 30, 0.0),
        &ShadowTuning::default(),
        60.0,
    )
    .expect("rollouts are on and a hostile is in view");
    assert_eq!(refined.move_id, "lunge");
    assert!(
        refined.value_over_baseline > 0.0,
        "connecting must beat doing nothing: {}",
        refined.value_over_baseline
    );
}

/// FB6e's determinism half, at the unit level: the SAME inputs produce the
/// bit-identical choice, twice. No clock, no RNG, no allocator luck.
#[test]
fn l3_decides_identically_twice() {
    let view = view_with(300.0, 380.0);
    let options = OptionSet {
        movement: Vec::new(),
        attacks: vec![
            attack("jab", frames(0.08, 40.0, 4, 0.0)),
            attack("lunge", frames(0.2, 100.0, 8, 300.0)),
            attack("smash", frames(0.3, 90.0, 20, 700.0)),
        ],
    };
    let mut habits = HabitModel::new(0.5);
    habits.observe(Situation::Advantage, Choice::Shield);
    habits.observe(Situation::Advantage, Choice::Shield);
    habits.observe(Situation::Advantage, Choice::Shield);
    let p = profile(3, 24, 1.0);
    let tuning = ShadowTuning::default();
    let one = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Advantage,
        &options,
        &habits,
        &p,
        &tuning,
        60.0,
    );
    let two = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Advantage,
        &options,
        &habits,
        &p,
        &tuning,
        60.0,
    );
    assert_eq!(one, two);
    // And the state trajectory itself is bit-identical, not merely the label.
    let mut a = state(300.0, 380.0);
    let mut b = state(300.0, 380.0);
    for _ in 0..30 {
        shadow_step(&mut a, DT, &ShadowIntent::Hold, &ShadowIntent::Hold, &tuning);
        shadow_step(&mut b, DT, &ShadowIntent::Hold, &ShadowIntent::Hold, &tuning);
    }
    assert_eq!(a, b);
}
