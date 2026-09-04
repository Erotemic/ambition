//! Unit properties for the shadow model (FB6b–d, fighter-brain.md §12).
//!
//! These assert MODEL properties and structural invariants — never "which
//! move a rollout picks at v1 tuning" beyond the constructed scenarios below,
//! because that is a calibration claim and calibration belongs to FB4's
//! ladder and FB6e's fidelity instrument.

use super::*;
use ambition_characters::actor::ActorFaction;
use ambition_characters::brain::fighter::habit::Choice;
use ambition_characters::brain::fighter::options::AttackOption;
use ambition_characters::perception::WorldView;

fn frames(startup_s: f32, reach: f32, max_damage: i32, max_knockback: f32) -> MoveFrameData {
    MoveFrameData {
        total_s: startup_s + 0.1 + 0.2,
        charge_hold_at_s: None,
        startup_s,
        active_spans: vec![(startup_s, startup_s + 0.1)],
        recovery_s: 0.2,
        cancel_windows: Vec::new(),
        reach,
        ignores_guard: false,
        // A forward poke of that length — the shape these fixtures mean.
        coverage: (reach > 0.0).then(|| ambition_entity_catalog::MoveCoverage {
            min: (0.0, -12.0),
            max: (reach, 12.0),
        }),
        max_damage,
        max_knockback,
        start_impulse: (0.0, 0.0),
        lift_speed: 0.0,
        lift_at_s: 0.0,
        lift_side: 0.0,
        recovery_route: Default::default(),
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
        utility_weights: ambition_characters::brain::fighter::options::UtilityWeights::v1(),
    }
}

fn attack(id: &str, frames: MoveFrameData) -> AttackOption {
    AttackOption {
        binding: ambition_characters::brain::attack_kit::AttackBinding {
            verb: ambition_characters::brain::attack_kit::AttackVerb::Basic,
            direction: ambition_characters::actor::attack_gesture::AttackDir::Forward,
        },
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
    let events = shadow_step(
        &mut s,
        DT,
        &ShadowIntent::Hold,
        &ShadowIntent::Hold,
        &tuning,
    );
    assert!(matches!(
        events[..],
        [ShadowEvent::Hit { on_me: false, .. }]
    ));

    let kb = HitKnockback {
        // An ordinary hit: it stuns.
        reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
        dir: me_before.facing,
        magnitude: HitKnockbackMagnitude::LaunchSpeed(500.0),
        source_pos: me_before.pos,
        impact_pos: foe_before.pos,
        launch_dir: None,
        follow: None,
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

/// The fidelity instrument's first finding, pinned at the unit level: a real
/// swing carries its authored start impulse, so a lunge's EFFECTIVE range is
/// reach plus travel. The same move without its impulse whiffs from here.
#[test]
fn a_lunge_reaches_past_its_static_reach_because_the_body_travels() {
    let tuning = ShadowTuning::default();
    let gap = 160.0; // far beyond 51 + extents
    let mut swing = frames(0.25, 51.0, 8, 0.0);

    let mut s = state(300.0, 300.0 + gap);
    s.me.phase = ShadowPhase::Idle;
    let start = ShadowIntent::StartMove {
        frames: swing.clone(),
    };
    let mut events = shadow_step(&mut s, DT, &start, &ShadowIntent::Hold, &tuning);
    events.extend(run(&mut s, 30, &ShadowIntent::Hold, &tuning));
    assert!(
        events.is_empty(),
        "without the impulse it whiffs: {events:?}"
    );

    swing.start_impulse = (600.0, 0.0);
    let mut s = state(300.0, 300.0 + gap);
    let start = ShadowIntent::StartMove {
        frames: swing.clone(),
    };
    let mut events = shadow_step(&mut s, DT, &start, &ShadowIntent::Hold, &tuning);
    events.extend(run(&mut s, 30, &ShadowIntent::Hold, &tuning));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ShadowEvent::Hit { on_me: false, .. })),
        "the lunge travels ~{}px over the move and connects: {events:?}",
        600.0 * swing.total_s
    );
    // And the impulse dies with the move — but at a RATE, not instantly.
    run(&mut s, 5, &ShadowIntent::Hold, &tuning);
    let after_five_steps = s.me.vel.x;
    assert!(
        after_five_steps > 100.0 && after_five_steps < 300.0,
        "five 60Hz steps of 7600 px/s² friction should leave this lunge around \
         220 px/s; it left {after_five_steps}. ⚠ the BAND is the assertion — an \
         exact number here would pin whatever the model does rather than the \
         fact that it coasts"
    );
    // and it DOES stop — the rate is a rate, not a leak.
    run(&mut s, 5, &ShadowIntent::Hold, &tuning);
    assert_eq!(s.me.vel.x, 0.0, "ground friction ends the lunge");
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
                6,
                None,
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
        6,
        None,
    )
    .expect("rollouts are on and a hostile is in view");
    assert_eq!(refined.move_id.as_deref(), Some("lunge"));
    assert!(
        refined.value_over_baseline > 0.0,
        "connecting must beat doing nothing: {}",
        refined.value_over_baseline
    );
}

/// FB6e's bench pin (§12.6): the worst shipped budget is a NON-EVENT. 100 decisions at k=4 ×
/// depth=20 (the worst §12.3 contemplates shipping) must clear 100 ms total, i.e. <1 ms per
/// decision with an order of magnitude of CI-noise headroom over the ~100 µs target.
#[test]
fn the_worst_shipped_budget_is_cheap_enough_to_be_a_non_event() {
    let view = view_with(300.0, 380.0);
    let options = OptionSet {
        movement: Vec::new(),
        attacks: vec![
            attack("jab", frames(0.08, 40.0, 4, 0.0)),
            attack("lunge", frames(0.2, 100.0, 8, 300.0)),
            attack("smash", frames(0.3, 90.0, 20, 700.0)),
            attack("sweep", frames(0.15, 70.0, 6, 150.0)),
        ],
    };
    let mut habits = HabitModel::new(0.5);
    for _ in 0..4 {
        habits.observe(Situation::Neutral, Choice::Approach);
    }
    let p = profile(4, 20, 1.0);
    let tuning = ShadowTuning::default();
    let started = std::time::Instant::now();
    for _ in 0..100 {
        let refined = refine_by_rollout(
            Perceived::cheating(&view),
            Situation::Neutral,
            &options,
            &habits,
            &p,
            &tuning,
            60.0,
            6,
            None,
        );
        assert!(refined.is_some());
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed.as_millis() < 100,
        "100 worst-case decisions took {elapsed:?}; the budget stopped being free"
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
        6,
        None,
    );
    let two = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Advantage,
        &options,
        &habits,
        &p,
        &tuning,
        60.0,
        6,
        None,
    );
    assert_eq!(one, two);
    // And the state trajectory itself is bit-identical, not merely the label.
    let mut a = state(300.0, 380.0);
    let mut b = state(300.0, 380.0);
    for _ in 0..30 {
        shadow_step(
            &mut a,
            DT,
            &ShadowIntent::Hold,
            &ShadowIntent::Hold,
            &tuning,
        );
        shadow_step(
            &mut b,
            DT,
            &ShadowIntent::Hold,
            &ShadowIntent::Hold,
            &tuning,
        );
    }
    assert_eq!(a, b);
}

// ── the floor has an END ─────────────────────────────────────────────────

/// A view whose only terrain is a platform of `half_width` centred under the
/// viewer, with the stage envelope well outside it — a smash stage, not a room.
fn view_on_platform(me_x: f32, half_width: f32) -> WorldView {
    let mut view = view_with(me_x, me_x + 400.0);
    view.terrain = vec![ambition_characters::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 332.0), ae::Vec2::new(half_width, 16.0)),
        kind: ambition_characters::perception::SolidKind::Solid,
    }];
    view
}

#[test]
fn a_shadow_body_driven_past_the_platforms_edge_falls_off_it() {
    let tuning = ShadowTuning::default();
    let view = view_on_platform(400.0, 60.0);
    let mut s =
        ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view");
    assert_eq!(
        s.me.ground_span,
        Some((340.0, 460.0)),
        "the shadow should have read the platform it stands on off the view"
    );

    // Walk right for a second. The platform ends 60 px away.
    let start_y = s.me.pos.y;
    run(&mut s, 60, &ShadowIntent::Drive { lateral: 1.0 }, &tuning);

    assert!(
        !s.me.on_ground,
        "a body driven past x=460 has run out of floor; it is not standing on anything"
    );
    assert!(
        s.me.pos.y > start_y + 1.0,
        "and having run out of floor it should be FALLING, not strolling at platform height (y {} -> {})",
        start_y,
        s.me.pos.y
    );
}

#[test]
fn the_same_walk_ends_in_a_ko_and_the_old_infinite_plane_never_does() {
    let tuning = ShadowTuning::default();
    let view = view_on_platform(400.0, 60.0);
    let build =
        || ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view");

    // 90 ticks = 1.5 s. Long enough to leave the platform's edge (0.375 s at
    // 160 px/s) and fall the 284 px to the bottom of the envelope; SHORT enough
    // that a body strolling at platform height reaches only x=640, well inside
    // the envelope's x=800 wall. So the KO below can only have come from falling.
    // 60 ticks, not 90, and the number is load-bearing. A one-second walk
    // at `ground_speed` covers 270px from x=400, which stays inside the 800-wide
    // fixture stage; ninety ticks covers 405 and leaves it, so the INFINITE-plane
    // control KO'd on the horizontal blastzone and the test reported *"the
    // walk-off died even without a floor"* about a fixture that had stopped
    // isolating the floor. It went unnoticed while `ground_speed` was 160 —
    // 240px, comfortably inside — and surfaced the moment that was corrected to
    // the engine's 270.
    let mut walked_off = build();
    let events = run(
        &mut walked_off,
        60,
        &ShadowIntent::Drive { lateral: 1.0 },
        &tuning,
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ShadowEvent::Ko { of_me: true })),
        "walking off the stage should cost me the stock; the rollout has to be able to price it"
    );

    let mut infinite_plane = build();
    infinite_plane.me.ground_span = None;
    let events = run(
        &mut infinite_plane,
        60,
        &ShadowIntent::Drive { lateral: 1.0 },
        &tuning,
    );
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, ShadowEvent::Ko { of_me: true })),
        "guard is not measuring the floor's extent: the walk-off died even without one"
    );
    assert!(
        infinite_plane.me.on_ground,
        "guard is not measuring the floor's extent: the plane model let go of the body anyway"
    );
}

#[test]
fn the_movement_veto_survives_having_nothing_to_swing() {
    let view = view_on_platform(440.0, 60.0);
    // `OptionSet::attacks` is empty in exactly one situation, and its own doc
    // names it: `Recovery` — "a body past the blastzone has exactly one
    // problem". That body was the one the veto skipped.
    let options = ambition_characters::brain::fighter::options::OptionSet {
        attacks: Vec::new(),
        movement: vec![ambition_characters::brain::fighter::options::MoveOption {
            verb: ambition_characters::brain::fighter::options::MovementVerb::Approach,
            score: 1.0,
        }],
    };
    let refined = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Neutral,
        &options,
        &HabitModel::default(),
        &profile(4, 12, 0.0),
        &ShadowTuning::default(),
        60.0,
        // A body committed for a full second: long enough that walking 400 px to
        // a foe off the end of a 120 px platform is genuinely underway.
        60,
        // No lens: this pins the SHADOW's own verdict, which is what a body whose
        // kit never reached the snapshot still gets.
        None,
    );
    let refined = refined.expect("no attacks is not a reason to skip movement vetting");
    assert!(
        refined.move_id.is_none(),
        "with no attacks there is no move to name"
    );
    assert_eq!(
        refined.suicidal_movement,
        vec![ambition_characters::brain::fighter::options::MovementVerb::Approach],
        "approaching a foe 400 px away off the right end of a 120 px platform \
         walks this body out of the world; the veto has to say so"
    );
}

#[test]
fn the_veto_horizon_reaches_past_the_edge_and_the_attack_horizon_does_not() {
    // Pins the RATIO, which is the finding: 12 ticks is 0.2 s (a startup plus an active span —
    // right for "does this connect") and 0.2 s cannot see a walk-off.
    let depth = 12u32;
    let attack_horizon_s = depth as f32 / 60.0;
    let veto_horizon_s = (depth * MOVEMENT_HORIZON_MULTIPLE) as f32 / 60.0;
    // 640 px stage, walking from the middle at the shipped ground speed.
    let time_to_edge_s = 320.0 / ShadowTuning::default().ground_speed;
    assert!(
        attack_horizon_s < time_to_edge_s,
        "if the attack horizon already reached the edge this ratio would be pointless"
    );
    assert!(
        veto_horizon_s > time_to_edge_s,
        "the veto horizon ({veto_horizon_s}s) must outreach the walk to the edge \
         ({time_to_edge_s}s) or the veto is decorative"
    );
}

#[test]
fn a_shadow_body_that_jumps_while_driving_lands_where_it_drifted_to() {
    // The commonest death in the game, and for a long time the one the model
    // could not represent: walk to the ledge, jump, and keep holding a
    // direction. A shadow with no air control lands where it took off.
    let tuning = ShadowTuning::default();
    let view = view_on_platform(400.0, 60.0);
    let mut s =
        ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view");

    let takeoff = s.me.pos.x;
    run(&mut s, 1, &ShadowIntent::Jump, &tuning);
    assert!(!s.me.on_ground, "the jump should have left the ground");
    // 30 ticks is half a second; the jump's airtime is 2 x 420 / 1400 = 0.6 s,
    // so the body is STILL AIRBORNE at the assertion. That is the whole point —
    // running long enough to land turns this into a test of grounded walking,
    // which passes with or without air control and proves nothing.
    run(&mut s, 30, &ShadowIntent::Drive { lateral: 1.0 }, &tuning);
    assert!(
        !s.me.on_ground,
        "the measurement window has to close before the body lands, or it is \
         measuring the walk that follows"
    );
    assert!(
        s.me.pos.x > takeoff + 40.0,
        "holding right through a jump has to carry the body sideways; it went \
         from {takeoff} to {}",
        s.me.pos.x
    );
}

/// A body ALREADY offstage is not already dead, and the shadow says it is.
///
/// `shadow_step` KOs anything outside the stage envelope on the tick it looks,
/// on the argument that *"on a platform stage the envelope IS the blast zone"*.
/// That is right for a body walking off and wrong for one RECOVERING: a fighter
/// knocked above or beside the stage starts outside the envelope and has to come
/// back, which is the whole premise of §8's four recovery quadrants.
///
/// The consequence is not a slightly wrong score — it is NO score. Every option
/// rolls out to "I am dead", so the rollout cannot tell a recovery from a
/// suicide, and the rung that uses it does worse than the rung that does not.
/// `ladder_rig --scenarios` shows exactly that: `recovery_above 9v6` inverts.
#[test]
fn a_body_recovering_from_offstage_is_not_scored_as_already_dead() {
    let tuning = ShadowTuning::default();
    let mut view = view_with(400.0, 500.0);
    // ABOVE the stage — where a fighter knocked upward has to recover from.
    view.self_view.pos = ae::Vec2::new(400.0, -40.0);
    view.self_view.on_ground = false;
    let mut s =
        ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view");
    assert!(
        !s.me.koed,
        "the body is dead before a single step, so nothing below can discriminate"
    );

    // One step of doing nothing.
    let events = shadow_step(
        &mut s,
        DT,
        &ShadowIntent::Hold,
        &ShadowIntent::Hold,
        &tuning,
    );
    let died = events
        .iter()
        .any(|e| matches!(e, ShadowEvent::Ko { of_me: true }));
    assert!(
        !died,
        "a body recovering from above was KO'd on its first shadow step, so every \
         option it could roll scores identically and the rollout is blind exactly \
         where §8 says it should matter most"
    );
}

/// The shadow's movement numbers ARE the engine's, not a copy of them.
///
/// A second table looks maintained, which is why nobody checked it for weeks. This asserts the
/// identity rather than the values — a table that drifts cannot drift past it.
#[test]
fn the_default_shadow_is_the_engines_own_movement_law() {
    let engine = ae::MovementTuning::default();
    let shadow = ShadowTuning::default();
    assert_eq!(shadow.gravity, engine.gravity);
    assert_eq!(shadow.ground_speed, engine.max_run_speed);
    assert_eq!(shadow.jump_speed, engine.jump_speed);
    assert_eq!(shadow.dash_speed, engine.dash_speed);
    assert_eq!(shadow.dash_time, engine.dash_time);
    assert_eq!(shadow.ground_coast_decel, engine.ground_friction);
    assert_eq!(shadow.air_coast_decel, engine.air_friction);
    assert_eq!(shadow.slash_recoil, engine.slash_recoil);
}

/// A body that authors its own movement is PREDICTED as that body.
///
/// The reason a copied table would still have been wrong even if every number
/// had been right: a heavier fighter's gravity or a faster one's run speed
/// changed the body and not the model, so the rollout kept planning arcs the
/// body could not fly.
///
/// and the foe assumptions survive the fold, because they are NOT body-derived
/// — the view names an opponent's phase and its clock, never its move.
#[test]
fn an_authored_body_is_predicted_with_its_own_movement_law() {
    let mut heavy = ae::MovementTuning::default();
    heavy.gravity *= 2.0;
    heavy.jump_speed *= 0.5;
    heavy.max_run_speed *= 0.5;

    let mut authored = ShadowTuning::default();
    authored.assumed_foe_reach = 999.0;
    authored.response.hitstun_time = 1.25;

    let folded = authored.clone().with_movement(&heavy);
    assert_eq!(folded.gravity, heavy.gravity);
    assert_eq!(folded.jump_speed, heavy.jump_speed);
    assert_eq!(folded.ground_speed, heavy.max_run_speed);
    assert_ne!(
        folded.gravity, authored.gravity,
        "the body's own gravity did not reach the predictor, so the rollout is \
         planning arcs for somebody else"
    );
    assert_eq!(
        folded.assumed_foe_reach, 999.0,
        "the fold overwrote an assumption the body cannot supply"
    );
    assert_eq!(folded.response.hitstun_time, 1.25);
}

/// And it arrives through the world-in port, which is the only channel the
/// brain has. A snapshot that carries no law leaves the config's tuning alone.
#[test]
fn a_snapshot_without_a_movement_law_changes_nothing() {
    let cfg = ShadowTuning::default();
    let snapshot = ambition_characters::brain::BrainSnapshot::idle();
    assert!(
        snapshot.movement_tuning.is_none(),
        "an idle snapshot claims a movement law it never resolved"
    );
    let resolved = match snapshot.movement_tuning.as_ref() {
        Some(movement) => cfg.clone().with_movement(movement),
        None => cfg.clone(),
    };
    assert_eq!(resolved, cfg);
}

// ── the recovery lens (the veto's body-generic half) ─────────────────────

/// This body is ALREADY in the air, left of the platform and below its top
/// face, with the foe across the gap. The shadow has no floor under it
/// (`floor_below` finds none at this x), so the line falls out of the envelope
/// and the shadow condemns it — whatever the body owns.
fn view_falling_beside_the_platform() -> WorldView {
    let mut view = view_on_platform(300.0, 60.0);
    view.self_view.pos = ae::Vec2::new(300.0, 330.0);
    view.self_view.on_ground = false;
    // Unspent, and the SAME for both kits below. The difference under test is
    // the verb, not the budget.
    view.self_view.air_jumps_left = 1;
    view
}

fn lens_for(
    view: &WorldView,
    abilities: ae::AbilitySet,
) -> crate::brain::fighter::recovery::RecoveryLens {
    crate::brain::fighter::recovery::RecoveryLens::from_view(
        view,
        crate::brain::fighter::recovery::BodyKit {
            abilities,
            movement: ae::MovementTuning::default(),
        },
        // No routes: these fixtures test the VETO, and the difference under test
        // is the body's verb. A body carrying a way home would make the verdicts
        // below about a repertoire instead.
        &[],
        DT,
    )
    .expect("the fixture stage is known and gravity is non-zero")
}

/// THE SELF-KO STOPS BEING ATTRACTIVE, AND THE REASON IS THE BODY.
///
/// One trajectory, three verdicts. The shadow line is byte-identical in all
/// three — same start, same sustained verb, same foe — so nothing about the
/// POSITION distinguishes them:
///
/// * no lens  the shadow's own answer, which is "this kills you", because its
///   line HOLDS after `commit_ticks` and it has no notion of coming back;
/// * a lens over a body with no mid-air jump  the real kernel agrees;
/// * a lens over a body that owns one  the kernel drives it back onto the
///   platform and the verb is reprieved.
///
/// `decide` picks the first movement option the veto did not name, so an
/// emptied veto is the difference between choosing this line and falling through
/// to `least_bad_movement` — the line that merely dies LATEST. That fallback is
/// what a fighter near a ledge has been getting.
///
/// the refused *"airborne, below the lip, outside the span"* predicate cannot
/// produce this table: it reads the position, which is the one thing all three
/// rows share.
#[test]
fn the_same_falling_line_is_condemned_or_reprieved_by_the_bodys_own_kit() {
    let view = view_falling_beside_the_platform();
    let options = ambition_characters::brain::fighter::options::OptionSet {
        attacks: Vec::new(),
        movement: vec![ambition_characters::brain::fighter::options::MoveOption {
            verb: ambition_characters::brain::fighter::options::MovementVerb::Approach,
            score: 1.0,
        }],
    };
    let approach = vec![ambition_characters::brain::fighter::options::MovementVerb::Approach];
    let refine = |lens: Option<&crate::brain::fighter::recovery::RecoveryLens>| {
        refine_by_rollout(
            Perceived::cheating(&view),
            Situation::Neutral,
            &options,
            &HabitModel::default(),
            &profile(4, 12, 0.0),
            &ShadowTuning::default(),
            60.0,
            60,
            lens,
        )
        .expect("rollouts are on and a hostile is in view")
    };

    assert_eq!(
        refine(None).suicidal_movement,
        approach,
        "without a lens this is the shadow's verdict, and the shadow kills every \
         line that leaves the ground here"
    );

    let grounded_kit = lens_for(&view, ae::AbilitySet::basic());
    assert_eq!(
        refine(Some(&grounded_kit)).suicidal_movement,
        approach,
        "a body with no mid-air jump really cannot get back from there, and the \
         kernel must not reprieve it"
    );

    let jumping_kit = lens_for(
        &view,
        ae::AbilitySet {
            double_jump: true,
            ..ae::AbilitySet::basic()
        },
    );
    assert!(
        refine(Some(&jumping_kit)).suicidal_movement.is_empty(),
        "the SAME line, by a body that owns the mid-air jump, climbs back onto \
         the platform; vetoing it is what makes a fighter refuse to move"
    );
}

/// An UNMODELLED verb stays unjudged, lens or no lens.
///
/// `movement_intent` returns `None` for a verb the shadow cannot simulate, and
/// "unmodelled means unjudged, in both directions" is a rule the lens must not
/// quietly break — a probe attached to a line that was never rolled would be
/// condemning a maneuver nobody simulated.
#[test]
fn an_unmodelled_verb_is_still_unjudged_with_a_lens_attached() {
    let view = view_on_platform(440.0, 60.0);
    let options = ambition_characters::brain::fighter::options::OptionSet {
        attacks: Vec::new(),
        movement: vec![ambition_characters::brain::fighter::options::MoveOption {
            // ⛔ `Dodge`, NOT `Shield`. This test named Shield when Shield was
            // unmodelled; `movement_intent` models it now
            // (`ShadowIntent::Hold`, 2026-09-04) and the assertions below kept
            // passing for an unrelated reason — holding still on a platform is
            // not suicidal — while their stated premise had become false. A
            // guard whose premise is gone still goes green, which is why the
            // verb has to be one the shadow genuinely refuses.
            verb: ambition_characters::brain::fighter::options::MovementVerb::Dodge,
            score: 1.0,
        }],
    };
    let lens = lens_for(&view, ae::AbilitySet::basic());
    let refined = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Neutral,
        &options,
        &HabitModel::default(),
        &profile(4, 12, 0.0),
        &ShadowTuning::default(),
        60.0,
        60,
        Some(&lens),
    )
    .expect("rollouts are on and a hostile is in view");
    assert!(
        refined.suicidal_movement.is_empty(),
        "`Dodge` is unmodelled, so it is unjudged — a lens must not turn an \
         unjudged verb into a condemned one"
    );
    assert!(
        refined.least_bad_movement.is_none(),
        "and with nothing condemned there is no least-bad fallback to name"
    );
}

/// The kernel path is as deterministic as the shadow path (ADR 0023).
///
/// A rollback-resimulated decision tick has to reproduce its original answer bit-for-bit, and
/// the lens is the first thing in this module that leaves it — it clones a body and drives
/// `step_motion`.
#[test]
fn a_decision_taken_through_the_lens_repeats_exactly() {
    let view = view_falling_beside_the_platform();
    let options = ambition_characters::brain::fighter::options::OptionSet {
        attacks: vec![attack("jab", frames(0.08, 40.0, 4, 0.0))],
        movement: vec![
            ambition_characters::brain::fighter::options::MoveOption {
                verb: ambition_characters::brain::fighter::options::MovementVerb::Approach,
                score: 1.0,
            },
            ambition_characters::brain::fighter::options::MoveOption {
                verb: ambition_characters::brain::fighter::options::MovementVerb::Retreat,
                score: 0.5,
            },
        ],
    };
    let lens = lens_for(
        &view,
        ae::AbilitySet {
            double_jump: true,
            ..ae::AbilitySet::basic()
        },
    );
    let once = || {
        refine_by_rollout(
            Perceived::cheating(&view),
            Situation::Neutral,
            &options,
            &HabitModel::default(),
            &profile(4, 12, 0.0),
            &ShadowTuning::default(),
            60.0,
            60,
            Some(&lens),
        )
    };
    assert_eq!(once(), once());
}

// ── the ledge TRANSITION (the case the fixtures above start past) ────────

/// The lip walk-off, from a body that is genuinely STANDING on the platform —
/// which is the state every fixture above skips by starting airborne and clear.
/// The platform ends at `x = 460` and the body's centre starts at `440` with a
/// 12px half-width, so the centre clears the lip after 20px of walking and the
/// FOOTPRINT clears it after 31px — the whole disagreement, and both inside the
/// first ten ticks at the shipped ground speed, far short of `commit_ticks`.
fn view_standing_at_the_lip() -> WorldView {
    let mut view = view_on_platform(440.0, 60.0);
    // Nothing in the air can save this body: the kit below owns no mid-air
    // jump AND the line has no charge left to spend, so a `Regained` verdict
    // can only have come from a SURFACE.
    view.self_view.air_jumps_left = 0;
    view
}

/// THE SHADOW MUST LET GO OF THE FLOOR WHERE THE KERNEL DOES, NOT A
/// HALF-EXTENT EARLIER.
///
/// `refine_by_rollout` captures `left_the_ground` from this exact transition and
/// hands the position to the real movement kernel. The kernel's support test is
/// `perpendicular_overlap` — the body's FOOTPRINT against the surface's span —
/// so a shadow that let go while the centre passed the lip handed over a
/// position at which the kernel still found the body standing on the platform,
/// and reprieved a walk-off by the very floor the body was leaving.
///
/// Asserted against the kernel's own predicate rather than a number, so it
/// cannot drift from `EDGE_OVERLAP_SLOP`, and the failure message prints both
/// spans — a genuine red names a footprint that still overlaps the platform.
#[test]
fn the_shadow_lets_go_of_the_lip_where_the_kernel_does() {
    let tuning = ShadowTuning::default();
    let view = view_standing_at_the_lip();
    let mut s =
        ShadowState::from_perceived(Perceived::cheating(&view)).expect("a hostile is in view");
    assert!(
        s.me.on_ground,
        "the fixture body has to START standing on the platform — that is the \
         transition under test"
    );

    let mut ticks = 0;
    while s.me.on_ground && ticks < 60 {
        run(&mut s, 1, &ShadowIntent::Drive { lateral: 1.0 }, &tuning);
        ticks += 1;
    }
    assert!(
        !s.me.on_ground,
        "one second of walking right never left a platform whose edge is 20px \
         away; the fixture, not the model, is wrong"
    );

    let span =
        s.me.ground_span
            .expect("the shadow read the platform it stood on");
    let half = s.me.half_extent.x;
    let footprint = (s.me.pos.x - half, s.me.pos.x + half);
    assert!(
        !ae::collision_semantics::spans_overlap_for_support(footprint, span),
        "the shadow let go at x={} with its footprint still at {:?} on the \
         platform {:?} — the real kernel asked at this position stands the body \
         straight back up, so a recovery probe taken here is reprieved by the \
         floor the body is walking off",
        s.me.pos.x,
        footprint,
        span
    );
}

/// AND THE VETO SURVIVES THE TRANSITION: a body that walks off the lip with
/// nothing to get back on is condemned.
///
/// The line is the same walk as above. The kit owns no mid-air jump, no wall
/// verb and no ledge grab, and the query carries zero unspent air jumps, so once
/// the body is past the lip and below it there is nothing left — the only thing
/// that could report `Regained` is the platform it just left, at a position the
/// shadow reached while still half-standing on it.
///
/// The second row is the proof — identical body, identical kit, identical walk, one extra shelf
/// in the flight path, opposite verdict.
#[test]
fn a_walk_off_the_lip_is_not_reprieved_by_the_platform_it_is_leaving() {
    let options = ambition_characters::brain::fighter::options::OptionSet {
        attacks: Vec::new(),
        movement: vec![ambition_characters::brain::fighter::options::MoveOption {
            verb: ambition_characters::brain::fighter::options::MovementVerb::Approach,
            score: 1.0,
        }],
    };
    let approach = vec![ambition_characters::brain::fighter::options::MovementVerb::Approach];
    let refine = |view: &WorldView| {
        let lens = lens_for(view, ae::AbilitySet::basic());
        refine_by_rollout(
            Perceived::cheating(view),
            Situation::Neutral,
            &options,
            &HabitModel::default(),
            &profile(4, 12, 0.0),
            &ShadowTuning::default(),
            60.0,
            60,
            Some(&lens),
        )
        .expect("rollouts are on and a hostile is in view")
        .suicidal_movement
    };

    let off_the_lip = view_standing_at_the_lip();
    assert_eq!(
        refine(&off_the_lip),
        approach,
        "walking off the lip with no air jump, no wall verb and no ledge grab \
         drops this body into the void — the kernel must not reprieve it with \
         the platform it is walking off"
    );

    // The falsifier: a shelf in the flight path, 104px below the lip and
    // starting past its edge. `floor_below` cannot see it (it begins at x=455,
    // right of this body's footprint), so the shadow's line is byte-identical
    // and only the kernel's world changed.
    let mut caught = view_standing_at_the_lip();
    caught
        .terrain
        .push(ambition_characters::perception::PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(572.5, 436.0), ae::Vec2::new(117.5, 16.0)),
            kind: ambition_characters::perception::SolidKind::Solid,
        });
    assert!(
        refine(&caught).is_empty(),
        "the SAME walk, with something to land on, is a walk — condemning it \
         would mean the veto had stopped reading the surfaces"
    );
}

/// ⭐⭐ A VERB THE SHADOW CANNOT SIMULATE IS PUBLISHED AS UNJUDGED.
///
/// ⛔⛔ ABSENCE FROM `suicidal_movement` USED TO BE THE ONLY SIGNAL, and it
/// conflates two opposite facts: "the rollout modelled this line and it lives"
/// and "the rollout could not model this line at all". `movement_intent`'s own
/// header says a rollout reporting every unknown as safe would be "lying in one
/// direction"; the consumer then read its silence as safety, so an unmodelled
/// verb outranked every modelled one the moment those were struck off — and
/// suppressed `least_bad_movement`, which is gated on every offered verb being
/// vetoed.
#[test]
fn a_verb_the_shadow_cannot_model_is_reported_as_unjudged() {
    use ambition_characters::brain::fighter::options::{MoveOption, MovementVerb, OptionSet};

    let view = view_falling_beside_the_platform();
    let options = OptionSet {
        attacks: vec![attack("jab", frames(0.08, 40.0, 4, 0.0))],
        movement: vec![
            MoveOption {
                verb: MovementVerb::Approach,
                score: 1.0,
            },
            // Dodge and Blink are the two `movement_intent` returns `None` for.
            MoveOption {
                verb: MovementVerb::Dodge,
                score: 0.9,
            },
            MoveOption {
                verb: MovementVerb::Blink,
                score: 0.8,
            },
        ],
    };
    let refined = refine_by_rollout(
        Perceived::cheating(&view),
        Situation::Recovery,
        &options,
        &HabitModel::default(),
        &profile(4, 12, 0.0),
        &ShadowTuning::default(),
        60.0,
        60,
        None,
    )
    .expect("rollouts are on");

    assert_eq!(
        refined.unmodelled_movement,
        vec![MovementVerb::Dodge, MovementVerb::Blink],
        "the two verbs the shadow has no intent for must be named as unjudged"
    );
    // ⛔ AND THEY ARE NOT VETOED EITHER. That is the whole hazard: a reader
    // asking only `suicidal_movement` sees nothing about them at all.
    assert!(
        !refined.suicidal_movement.contains(&MovementVerb::Dodge),
        "an unmodelled verb must not be reported as suicidal — it was not judged"
    );
}

/// WHICH VERBS THE SHADOW MODELS, PINNED AS A CENSUS.
///
/// ⛔⛔ **THIS IS THE GUARD THE 2026-09-04 DEFECT SLIPPED PAST, and the reason it
/// is a census rather than a spot check.** `movement_intent` returning `None`
/// does not merely make a verb unjudged — it removes it from `pick_movement`'s
/// FIRST tier, and the second tier (`least_bad`) catches the fall, so the third
/// tier where unmodelled verbs live is almost never reached. Measured: a rollout
/// fighter selected `Dodge` and `Shield` **zero times in 662 decisions**.
/// Declining to model a verb deletes it.
///
/// ⇒ So the modelled/unmodelled split is a BEHAVIOURAL decision, not an
/// implementation detail, and it must not drift silently in either direction:
/// dropping a model deletes a verb, and adding a dishonest one (an air dodge as
/// `Hold`) is the "lying in one direction" this module's own header refuses.
/// Adding a `MovementVerb` without deciding which side it falls on reddens here.
#[test]
fn the_shadow_models_exactly_these_movement_verbs() {
    use ambition_characters::brain::fighter::options::MovementVerb as V;

    // Every variant, listed so a new one is a compile-or-test decision rather
    // than a silent default into "unmodelled".
    let every = [
        V::Approach,
        V::Retreat,
        V::Jump,
        V::Dash,
        V::Dodge,
        V::Shield,
        V::Blink,
        V::Recover,
    ];
    let start = state(100.0, 300.0);

    let modelled: Vec<V> = every
        .into_iter()
        .filter(|verb| super::movement_intent(*verb, &start).is_some())
        .collect();

    assert_eq!(
        modelled,
        vec![V::Approach, V::Retreat, V::Jump, V::Dash, V::Shield, V::Recover],
        "the set of verbs the shadow can simulate changed. That is a behavioural \
         change, not a refactor: an unmodelled verb loses `pick_movement`'s first \
         tier and is reached only through a third tier that `least_bad` almost \
         always pre-empts, so REMOVING a model deletes a verb from the fighter's \
         repertoire. Adding one is only safe if the shadow really simulates what \
         the body does — `Hold` for an air dodge would be a lie in one direction."
    );
}
