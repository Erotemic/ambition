//! Unit tests for the FB4b decision rig.
//!
//! The three careful pieces get one test each and the plumbing gets one: what a
//! rig like this gets wrong is never "did it call classify", it is cadence, the
//! APM ceiling, and a noise stream that does not reproduce.

use super::*;
use crate::actor::ActorFaction;
use crate::brain::fighter::options::UtilityWeights;
use crate::brain::fighter::profile::FighterBrainProfile;
use crate::perception::{PerceivedActor, SelfView, StageView, WorldView};
use ambition_engine_core as ae;

fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
    }
}

fn scene(me_x: f32, foe_x: f32) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos: ae::Vec2::new(me_x, 300.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            faction: ActorFaction::Player,
            alive: true,
            on_ground: true,
            ..Default::default()
        },
        stage: stage(),
        actors: vec![PerceivedActor {
            id: "foe".to_string(),
            pos: ae::Vec2::new(foe_x, 300.0),
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// A profile that reacts instantly and never rolls out, so a test measures the
/// rig rather than the delay buffer or L3.
fn immediate_profile() -> FighterBrainProfile {
    FighterBrainProfile {
        level: 5,
        reaction_ms: 0.0,
        apm_cap: 0.0,
        execution_noise: 0.0,
        rollout_depth: 0,
        rollout_k: 0,
        read_weight: 0.5,
        utility_weights: UtilityWeights::default(),
    }
}

fn rig(profile: FighterBrainProfile) -> (FighterCfg, FighterState) {
    let cfg = FighterCfg::new(profile);
    let state = FighterState::new(&cfg, 0x5EED);
    (cfg, state)
}

fn run(cfg: &FighterCfg, state: &mut FighterState, ticks: u32) -> Vec<ActorControlFrame> {
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut frames = Vec::new();
    let mut out = ActorControlFrame::neutral();
    for _ in 0..ticks {
        tick_fighter(cfg, state, &snapshot, Some(&view), &mut out);
        frames.push(out.clone());
    }
    frames
}

/// **The brain emits every tick, not only on decision ticks.**
///
/// The held intent is what a human's hand does between thoughts. A rig that
/// emitted a neutral frame between decisions would produce a fighter that
/// stutters at exactly the decision cadence — visible, and wrong.
#[test]
fn the_intent_is_held_between_decisions() {
    let (cfg, mut state) = rig(immediate_profile());
    let frames = run(&cfg, &mut state, 12);
    let moving: Vec<bool> = frames.iter().map(|f| f.locomotion.x != 0.0).collect();
    assert!(
        moving.iter().filter(|m| **m).count() > 1,
        "the brain only moved on decision ticks, so it stutters at the decision \
         cadence: {moving:?}"
    );
}

/// **A decision happens every `decision_interval_ticks`, not every tick.**
///
/// Measured through the clock the rig actually uses rather than by counting
/// calls: `ticks_until_decision` is reloaded to the interval each time it fires.
#[test]
fn decisions_run_on_the_configured_cadence() {
    let mut cfg = FighterCfg::new(immediate_profile());
    cfg.decision_interval_ticks = 4;
    let mut state = FighterState::new(&cfg, 1);
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    let mut clocks = Vec::new();
    for _ in 0..9 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        clocks.push(state.ticks_until_decision);
    }
    // Reloaded to 4 on the tick it fires, then counts down.
    assert_eq!(clocks, vec![4, 3, 2, 1, 4, 3, 2, 1, 4], "cadence drifted");
}

/// **A zero interval cannot divide by zero.** A config that says "decide every
/// zero ticks" is a mistake, and the rig coerces rather than panicking in a sim
/// step nobody can catch.
#[test]
fn a_zero_decision_interval_is_coerced_to_one() {
    let mut cfg = FighterCfg::new(immediate_profile());
    cfg.decision_interval_ticks = 0;
    let mut state = FighterState::new(&cfg, 1);
    let frames = run(&cfg, &mut state, 3);
    assert_eq!(frames.len(), 3);
}

/// **APM is a ceiling the brain never crosses**, enforced at the one emission
/// point. §3's humanity check in miniature.
#[test]
fn presses_never_exceed_the_profiles_apm_cap() {
    let mut profile = immediate_profile();
    profile.apm_cap = 120.0;
    let mut cfg = FighterCfg::new(profile);
    // Think as fast as possible, so the cap is the ONLY thing limiting presses.
    cfg.decision_interval_ticks = 1;
    let mut state = FighterState::new(&cfg, 7);

    run(&cfg, &mut state, 600); // ten seconds at 60 Hz
    let measured = state.apm.apm(cfg.tick_hz);
    assert!(
        measured <= 120.0,
        "the brain pressed at {measured} APM against a 120 cap — the ledger is \
         not gating the emission point"
    );
}

/// A press with no token is DROPPED and the movement stays. The alternative — a
/// press that waits — would make the cap a delay rather than a ceiling, and the
/// histogram would measure a brain that never misses.
#[test]
fn a_dropped_press_does_not_stop_the_body_moving() {
    let mut profile = immediate_profile();
    profile.apm_cap = 1.0; // effectively no presses at all
    let mut cfg = FighterCfg::new(profile);
    cfg.decision_interval_ticks = 1;
    let mut state = FighterState::new(&cfg, 11);

    let frames = run(&cfg, &mut state, 120);
    assert!(
        frames.iter().any(|f| f.locomotion.x != 0.0),
        "an APM-starved brain stopped moving as well as stopped pressing"
    );
}

/// **The noise stream reproduces**, which is what makes the brain rollback-safe:
/// the same seed and the same inputs produce the same fighter.
#[test]
fn the_same_seed_produces_the_same_fighter() {
    let mut profile = immediate_profile();
    profile.execution_noise = 0.9;
    let cfg = FighterCfg::new(profile);

    let mut a = FighterState::new(&cfg, 0xABCD_EF01);
    let mut b = FighterState::new(&cfg, 0xABCD_EF01);
    let left = run(&cfg, &mut a, 90);
    let right = run(&cfg, &mut b, 90);

    assert_eq!(a.noise, b.noise, "the noise streams diverged");
    assert_eq!(a.apm, b.apm);
    let left_presses: Vec<bool> = left.iter().map(|f| f.melee_pressed).collect();
    let right_presses: Vec<bool> = right.iter().map(|f| f.melee_pressed).collect();
    assert_eq!(
        left_presses, right_presses,
        "two fighters with the same seed pressed on different ticks, so a replay \
         would not reproduce the fight"
    );
}

/// **A tick that consumes no noise leaves the seed alone.** That is the property
/// that makes the stream rewindable — a step-per-tick generator would depend on
/// how many ticks happened rather than on how many samples were taken.
#[test]
fn the_noise_seed_only_moves_when_a_sample_is_taken() {
    let mut profile = immediate_profile();
    profile.execution_noise = 0.0; // never consumes
    let cfg = FighterCfg::new(profile);
    let mut state = FighterState::new(&cfg, 0x1234);
    let before = state.noise;
    run(&cfg, &mut state, 60);
    assert_eq!(
        state.noise, before,
        "the seed advanced on a profile that never asks for a sample"
    );
}

/// **The habit model finally has a writer that is not a test.** (FB5's open loop)
///
/// A foe that keeps closing the gap should be read as an approacher.
#[test]
fn the_decision_tick_feeds_the_habit_model() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();

    // The foe walks in from the right, so its velocity points at me.
    for step in 0..40 {
        let mut view = scene(300.0, 600.0 - step as f32 * 5.0);
        view.actors[0].vel = ae::Vec2::new(-60.0, 0.0);
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    }
    assert!(
        !state.habits.is_empty(),
        "no decision tick ever observed the foe, so FB5's model still has no \
         writer outside its own tests"
    );
}

/// A brain with no world yet emits a neutral frame rather than panicking. The
/// first tick of any fixture is this case.
#[test]
fn a_brain_that_has_seen_nothing_emits_neutral() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();
    tick_fighter(&cfg, &mut state, &snapshot, None, &mut out);
    assert_eq!(out.locomotion, ae::Vec2::ZERO);
    assert!(!out.melee_pressed);
}

/// A body standing on a ledge narrower than it can commit to walking across.
///
/// ⚠ the width is load-bearing and 80 px was NOT narrow enough. The veto asks
/// what a COMMITTED walk does — one decision interval of input, then coasting —
/// and at 160 px/s an interval is only a few px. A ledge the body can step
/// around inside one decision is a ledge no honest veto fires on, which is the
/// veto working. 20 px is inside the commitment window in both directions.
fn on_a_ledge(me_x: f32) -> WorldView {
    let mut view = scene(me_x, me_x + 400.0);
    view.terrain = vec![crate::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(me_x, 316.0), ae::Vec2::new(10.0, 16.0)),
        kind: crate::perception::SolidKind::Solid,
    }];
    view
}

/// **A chosen verb replaces the held movement; it does not add to it.**
///
/// `frame` arrives holding the last decision's answer, so a verb that only adds
/// inherits the rest. Jump used to do exactly that: veto Retreat, choose Jump,
/// and the body jumps while still walking the direction the veto struck off.
#[test]
fn choosing_a_verb_cancels_the_walk_the_last_decision_left_running() {
    let profile = FighterBrainProfile {
        rollout_depth: 12,
        rollout_k: 4,
        ..immediate_profile()
    };
    let (cfg, mut state) = rig(profile);
    // It is already walking right, off the end of the ledge.
    state.held.locomotion.x = 1.0;

    let snapshot = BrainSnapshot::idle();
    let view = on_a_ledge(300.0);
    let mut out = ActorControlFrame::neutral();
    for _ in 0..cfg.interval() + 1 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    }

    // On a 20 px ledge L2 offers Retreat and Jump; the rollout strikes Retreat
    // (it leaves the ledge inside the commitment window) and keeps Jump, which
    // goes straight up and lands where it started. So the surviving verb is
    // Jump — and the assertion is that choosing it CANCELS the walk rather than
    // jumping on top of it.
    assert_eq!(
        out.locomotion.x, 0.0,
        "the held walk had to be actively cancelled by the chosen verb, not \
         left running underneath it"
    );
    assert!(out.jump_held, "and the verb it chose was Jump");
}

/// The other half, and the one that says the veto is not just paralysis: given
/// room, the same brain still moves.
#[test]
fn the_same_brain_on_solid_ground_still_walks() {
    let profile = FighterBrainProfile {
        rollout_depth: 12,
        rollout_k: 4,
        ..immediate_profile()
    };
    let (cfg, mut state) = rig(profile);
    let mut view = scene(300.0, 500.0);
    view.terrain = vec![crate::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 316.0), ae::Vec2::new(400.0, 16.0)),
        kind: crate::perception::SolidKind::Solid,
    }];

    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();
    for _ in 0..cfg.interval() + 1 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    }

    assert_ne!(
        out.locomotion.x, 0.0,
        "on a stage 800px wide with the foe 200px away, refusing to move is not \
         caution — it is a veto whose horizon outgrew the stage"
    );
}

/// **When every option is fatal, take the one that dies LATEST — never nothing.**
///
/// The first cut of this halted instead, on the reading that a body told every
/// direction kills it should stop. That is right on the ground and wrong in the
/// air, and the difference cost a measurable regression: once `Recover` became
/// modellable the rollout could condemn it, `Recover` is the ONLY verb offered
/// in `Situation::Recovery`, so an airborne body's list emptied and the halt
/// replaced a doomed recovery with a certain one. Level 9 survival fell 40.2s to
/// 9.2s the moment the model got good enough to condemn the verb.
///
/// The scenario is a body already in the air with no floor beneath it, which is
/// where the shadow has no `ground_level` at all.
#[test]
fn a_body_whose_every_option_is_fatal_takes_the_longest_lived_one() {
    let profile = FighterBrainProfile {
        rollout_depth: 12,
        rollout_k: 4,
        ..immediate_profile()
    };
    let (cfg, mut state) = rig(profile);
    state.held.locomotion.x = 1.0;

    // Airborne, over nothing, and to the RIGHT of the stage centre — so the
    // recovery it should reach for goes left, against the held walk.
    let mut view = scene(700.0, 780.0);
    view.self_view.on_ground = false;
    view.self_view.air_jumps_left = 1;
    view.self_view.vel = ae::Vec2::new(160.0, 40.0);

    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();
    for _ in 0..cfg.interval() + 1 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    }

    assert_ne!(
        out.locomotion.x, 1.0,
        "the held walk must not survive the decision, whatever the decision was"
    );
    assert!(
        out.jump_held || out.locomotion.x != 0.0,
        "a falling body that freezes has thrown away its last option: {out:?}"
    );
}

/// **The jump button is RELEASED by whichever verb runs next.**
///
/// `jump_held` was written `true` at two verbs and `false` nowhere, and `frame`
/// starts each tick as `state.held`, so one jump pinned the button down for the
/// rest of the match. That is a held input the brain never chose to keep — the
/// same leak as `locomotion.x`, one field over.
///
/// Asserts the RELEASE, not the press: a test that only checked the press was
/// green throughout the entire time this was broken.
#[test]
fn the_jump_button_does_not_stay_held_after_the_jump() {
    let (cfg, mut state) = rig(immediate_profile());
    // Enter the decision already holding jump, as a body that jumped last time
    // would be.
    state.held.jump_held = true;

    let snapshot = BrainSnapshot::idle();
    // Solid ground and a foe in reach: L2 answers with an approach, not a jump.
    let mut view = scene(300.0, 340.0);
    view.terrain = vec![crate::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 316.0), ae::Vec2::new(400.0, 16.0)),
        kind: crate::perception::SolidKind::Solid,
    }];

    let mut out = ActorControlFrame::neutral();
    for _ in 0..cfg.interval() + 1 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    }

    assert!(
        !out.jump_held,
        "the fighter is still holding jump a decision after it stopped choosing \
         to. One jump used to hold the button for the whole match: {out:?}"
    );
}
