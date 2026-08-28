//! Unit tests for the FB4b decision rig.
//!
//! The three careful pieces get one test each and the plumbing gets one: what a
//! rig like this gets wrong is never "did it call classify", it is cadence, the
//! APM ceiling, and a noise stream that does not reproduce.

use super::*;
use ambition_characters::actor::attack_gesture::AttackDir;
use ambition_characters::actor::ActorFaction;
use ambition_characters::brain::attack_kit::{AttackBinding, AttackVerb};
use ambition_characters::brain::fighter::options::UtilityWeights;
use ambition_characters::brain::fighter::profile::FighterBrainProfile;
use ambition_characters::perception::{PerceivedActor, SelfView, StageView, WorldView};
use ambition_platformer2d_core as ae;

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

fn planted_press(ticks: u32) -> PendingAttack {
    PendingAttack {
        ticks,
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
        hold_ticks: 0,
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

/// THE BRAIN THE SMASH ROSTER ACTUALLY SEATS KNOWS IT IS IN A CAPTURE.
///
/// Three claims, and the third is the one a happy-path test would miss:
///
/// * a captive struggles (some ticks, not all — a machine-rate mash is a
///   different mechanic);
/// * a captive asks for nothing else, ever;
/// * a captor pummels and then throws FORWARD — mirrored by its facing, because
///   `attack_dir_from_axis` reads `axis.x * facing`, so a left-facing fighter
///   that sent a bare `+x` was asking for a BACK throw it cannot perform.
#[test]
fn a_fighter_in_a_capture_struggles_or_spends_the_hold() {
    let (cfg, mut state) = rig(immediate_profile());
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    // 1. HELD.
    let mut snapshot = BrainSnapshot::idle();
    snapshot.captured = true;
    let mut struggles = 0;
    let ticks = 60;
    for tick_index in 0..ticks {
        snapshot.captured_for = tick_index as f32 * snapshot.dt;
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        assert_eq!(
            out.locomotion,
            ae::LocalAxes::ZERO,
            "a held fighter tried to walk"
        );
        assert!(
            !out.jump_pressed && !out.grab_pressed,
            "a held fighter acted"
        );
        if out.melee_pressed {
            struggles += 1;
        }
    }
    assert!(struggles > 0, "a captive never struggled");
    assert!(
        struggles < ticks,
        "it mashed on every tick, which no person can do"
    );

    // 2. HOLDING, facing LEFT — the case the unmirrored stick got wrong.
    for (pummels, expect_forward) in [(0u8, false), (1, true)] {
        let (cfg, mut state) = rig(immediate_profile());
        let mut snapshot = BrainSnapshot::idle();
        snapshot.holding_captive = true;
        snapshot.pummels_landed = pummels;
        snapshot.actor_facing = -1.0;
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        assert!(out.melee_pressed, "a captor did nothing with its captive");
        let asked_forward = ambition_characters::actor::attack_gesture::attack_dir_from_axis(
            out.attack_axis,
            snapshot.actor_facing,
            0.2,
        ) == ambition_characters::actor::attack_gesture::AttackDir::Forward;
        assert_eq!(
            asked_forward, expect_forward,
            "with {pummels} pummel(s) landed and the body facing LEFT, the press \
             resolved to the wrong direction — a throw asked for as a bare `+x` \
             becomes a BACK throw this fighter never authored"
        );
    }
}

/// The brain emits every tick, not only on decision ticks.
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

/// A decision happens every `decision_interval_ticks`, not every tick.
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

/// A zero interval cannot divide by zero. A config that says "decide every
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

/// APM is a ceiling the brain never crosses, enforced at the one emission
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

/// The noise stream reproduces, which is what makes the brain rollback-safe:
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

/// THE SAME SEED SHOWN A DIFFERENT WORLD IS ALLOWED TO DECIDE
/// DIFFERENTLY, and this is the half that keeps Emmy Ethereal's authored mirror
/// symmetry an emergent property rather than a puppet show.
///
/// [`the_same_seed_produces_the_same_fighter`] above is the other half:
/// *identical cognition + symmetric information → symmetric behaviour*, which is
/// exactly what her trait buys by putting two CPU twins on one stream (see
/// `CharacterDefinition::preserves_mirror_symmetry`).  together the pair states
/// the whole invariant: the mirror follows from shared cognition reading a
/// symmetric stage, so it must BREAK as soon as the stage stops being symmetric.
///
/// a forced mirror would pass the first test and fail this one, which is the
/// only reason this test earns its place: it is the falsifier for an
/// implementation that synchronised two fighters' actions instead of their
/// starting streams. Noether's theorem is that claim precisely — the symmetry has
/// to be real for the conservation law to hold.
#[test]
fn the_same_seed_shown_a_different_world_may_decide_differently() {
    let mut profile = immediate_profile();
    profile.execution_noise = 0.9;
    let cfg = FighterCfg::new(profile);
    let snapshot = BrainSnapshot::idle();

    // ONE stream, which is what the authored trait grants a pair of twins.
    let mut near = FighterState::new(&cfg, 0xABCD_EF01);
    let mut far = FighterState::new(&cfg, 0xABCD_EF01);
    assert_eq!(
        near.noise, far.noise,
        "the fixture must start both fighters on ONE stream, or it is not testing \
         the mirror at all"
    );

    // ...and two DIFFERENT worlds. the foe is on the OPPOSITE SIDE, which is
    // the asymmetry this fixture can actually express: it supplies no moveset, so
    // neither fighter ever has an attack option to choose between, and the only
    // decision on the table is which way to go. That makes it the sharpest
    // possible version of the claim — a forced mirror would walk them both the
    // same way.
    let foe_right = scene(300.0, 500.0);
    let foe_left = scene(300.0, 100.0);
    let mut right_out = ActorControlFrame::neutral();
    let mut left_out = ActorControlFrame::neutral();
    let mut diverged = false;
    for _ in 0..180 {
        tick_fighter(&cfg, &mut near, &snapshot, Some(&foe_right), &mut right_out);
        tick_fighter(&cfg, &mut far, &snapshot, Some(&foe_left), &mut left_out);
        if right_out != left_out || near.noise != far.noise {
            diverged = true;
            break;
        }
    }
    assert!(
        diverged,
        "two fighters on one stream never diverged across 180 ticks despite their \
         foes being on opposite sides — that would mean behaviour is synchronised \
         rather than emerging from what each one observes"
    );
    // and name the divergence rather than only counting it: they walk APART.
    assert!(
        right_out.locomotion.x * left_out.locomotion.x < 0.0,
        "the two twins did not head in opposite directions toward opposite foes \
         (right: {:?}, left: {:?})",
        right_out.locomotion,
        left_out.locomotion,
    );
}

/// A tick that consumes no noise leaves the seed alone. That is the property
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

/// The habit model finally has a writer that is not a test. (FB5's open loop)
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
    assert_eq!(out.locomotion, ae::LocalAxes::ZERO);
    assert!(!out.melee_pressed);
}

/// A body standing on a ledge narrower than it can commit to walking across.
///
/// the width is load-bearing and 80 px was NOT narrow enough. The veto asks
/// what a COMMITTED walk does — one decision interval of input, then coasting —
/// and at 160 px/s an interval is only a few px. A ledge the body can step
/// around inside one decision is a ledge no honest veto fires on, which is the
/// veto working. 20 px is inside the commitment window in both directions.
fn on_a_ledge(me_x: f32) -> WorldView {
    let mut view = scene(me_x, me_x + 400.0);
    view.terrain = vec![ambition_characters::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(me_x, 316.0), ae::Vec2::new(10.0, 16.0)),
        kind: ambition_characters::perception::SolidKind::Solid,
    }];
    view
}

/// A chosen verb replaces the held movement; it does not add to it.
///
/// `frame` arrives holding the last decision's answer, so a verb that only adds inherits the rest.
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
    view.terrain = vec![ambition_characters::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 316.0), ae::Vec2::new(400.0, 16.0)),
        kind: ambition_characters::perception::SolidKind::Solid,
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

/// When every option is fatal, take the one that dies LATEST — never nothing.
///
/// The first cut of this halted instead, on the reading that a body told every direction kills it
/// should stop. Level 9 survival fell 40.2s to 9.2s the moment the model got good enough to condemn
/// the verb.
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

/// The jump button is RELEASED by whichever verb runs next.
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
    view.terrain = vec![ambition_characters::perception::PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, 316.0), ae::Vec2::new(400.0, 16.0)),
        kind: ambition_characters::perception::SolidKind::Solid,
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

/// A queued press is CANCELLED when the body starts recovering.
///
/// A press is armed at one decision and matures several ticks later, and the
/// situation can change in between. On a platform stage it does, in the one
/// direction that matters: the trace of a level-9 self-KO caught an
/// attack armed while airborne OVER the lip — still `Neutral` — maturing two
/// decisions later with the body past the edge and asking to `Recover`. Every
/// attack in this engine lunges, so the fighter's own queued swing carried it
/// further out while its emitted input said "back".
///
/// "L2 already refuses to OFFER attacks in `Recovery`" is no longer true,
/// and the correction is the point rather than a tidy-up: L2 now offers a
/// recovering body its LIFTING moves, because a genre fighter's answer to being
/// offstage IS a move. So this is a DROP, not a ban — the stale press dies and
/// `generate_options` re-arms from the Recovery option set in the same tick.
/// What cannot survive is a press decided under a different situation.
#[test]
fn a_press_in_flight_is_dropped_when_the_body_starts_recovering() {
    let cfg = FighterCfg::new(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();

    // The press is planted rather than earned: `BrainSnapshot::idle()` carries no
    // attack kit, so no scene here can arm one, and what is under test is the
    // CANCELLATION rather than the arming.
    let mut recovering = FighterState::new(&cfg, 1);
    recovering.pending_press = Some(planted_press(6));
    let mut offstage = scene(300.0, 340.0);
    // `stage()`'s envelope is the room; a body outside it is recovering by the
    // oldest of L1's rules.
    offstage.self_view.pos.x = -80.0;
    offstage.self_view.on_ground = false;
    tick_fighter(&cfg, &mut recovering, &snapshot, Some(&offstage), &mut out);
    assert_eq!(
        recovering.pending_press, None,
        "the body is recovering and still holding a swing it decided on while it \
         had a stage under it"
    );

    // NON-VACUITY: the same planted press on a body that is not recovering only
    // AGES. Without this the assertion above would pass against a rig that
    // dropped every press for any reason.
    let mut fighting = FighterState::new(&cfg, 1);
    fighting.pending_press = Some(planted_press(6));
    let on_stage = scene(300.0, 340.0);
    tick_fighter(&cfg, &mut fighting, &snapshot, Some(&on_stage), &mut out);
    assert_eq!(
        fighting.pending_press,
        Some(planted_press(5)),
        "a press on a body with a stage under it must age, not vanish"
    );
}

/// One decision to Blink is ONE press edge.
///
/// `FighterState::held` and clones it every tick between decisions, clearing the
/// edges by hand — melee, jump and dash. `MovementVerb::Blink` sets
/// `blink_pressed`, which was not among them, so a single choice re-emitted a
/// press on every tick until the next decision overwrote it. Cooldowns masked
/// some of the consequences; anything reading `blink_pressed` directly saw
/// several presses for one choice.
#[test]
fn a_blink_choice_is_pressed_once_and_not_carried_forward() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    // The frame a Blink decision leaves behind, and a decision far enough away
    // that the following ticks are pure carry.
    state.held = ActorControlFrame::neutral();
    state.held.blink_pressed = true;
    state.ticks_until_decision = 30;

    let mut pressed_after = 0;
    for _ in 0..5 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        if out.blink_pressed {
            pressed_after += 1;
        }
    }
    assert_eq!(
        pressed_after, 0,
        "the blink press re-fired on {pressed_after} of the 5 ticks after the \
         decision that made it — one choice, several presses"
    );
}

/// Every EDGE the held frame carries is consumed before it can re-emit; every
/// SUSTAIN survives, because a held button is held.
#[test]
fn the_held_frame_carries_sustains_and_never_re_emits_an_edge() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    let mut held = ActorControlFrame::neutral();
    held.blink_pressed = true;
    held.blink_released = true;
    held.projectile_pressed = true;
    held.projectile_released = true;
    held.pogo_pressed = true;
    held.fast_fall_pressed = true;
    held.fly_toggle_pressed = true;
    held.modifier_pressed = true;
    held.special_pressed = true;
    held.interact_pressed = true;
    // …and the sustains that must NOT be touched.
    held.shield_held = true;
    held.projectile_held = true;
    held.blink_held = true;
    held.modifier_held = true;
    held.jump_held = true;
    state.held = held;
    state.ticks_until_decision = 30;

    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);

    assert!(
        !out.blink_pressed
            && !out.blink_released
            && !out.projectile_pressed
            && !out.projectile_released
            && !out.pogo_pressed
            && !out.fast_fall_pressed
            && !out.fly_toggle_pressed
            && !out.modifier_pressed
            && !out.special_pressed
            && !out.interact_pressed,
        "an edge survived the carry and will fire again next tick: {out:?}"
    );
    assert!(
        out.shield_held && out.projectile_held && out.blink_held && out.modifier_held,
        "a SUSTAIN was cleared — a held button that lets go between decisions is \
         a fighter that cannot block: {out:?}"
    );
}

/// The chosen attack is PRESSED as itself.
///
/// `RefinedChoice::move_id` named one — and the emission set `melee_pressed` with
/// a neutral axis, so the moveset resolved whatever the default gesture maps to.
/// The binding now rides the pending press through the execution jitter, and
/// this is the frame that comes out the other side.
///
/// the fixture plants the STICK as well as the press, because
/// that is what an arming decision produces. The direction stopped riding the
/// `PendingAttack` struct and now rides the held frame, the way a hand holds a
/// stick while the button is still on its way down — so a fixture that planted
/// only the press was modelling a construction production no longer has. What is
/// asserted below is unchanged and is the thing that matters: the direction
/// the brain chose is on the frame the body receives. That it is the RIGHT
/// direction at either facing is
/// [`the_aimed_stick_round_trips_through_the_bodys_own_resolver`].
#[test]
fn a_pending_attack_matures_into_the_press_that_reaches_its_move() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    // An up-smash, decided two ticks ago and maturing now — press and stick
    // together, exactly as `decide` emits them.
    let binding = AttackBinding {
        verb: AttackVerb::Smash,
        direction: AttackDir::Up,
    };
    super::aim_the_stick(binding, view.self_view.facing, &mut state.held);
    state.pending_press = Some(PendingAttack {
        ticks: 0,
        binding,
        hold_ticks: 0,
    });
    state.ticks_until_decision = 30;
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);

    assert!(out.melee_pressed, "the press never came out");
    assert!(
        out.melee_strong_hint,
        "the SMASH verb arrived as a plain attack, so `move_for_directional_verb` \
         resolves the jab the brain scored against"
    );
    assert!(
        out.attack_axis.y < -0.5,
        "the UP direction did not reach the axis ({:?}), so the up-tilt the brain \
         chose comes out as whatever neutral maps to",
        out.attack_axis
    );

    // …and a Special is a different button, not a melee edge with a hint.
    let mut state = FighterState::new(&cfg, 7);
    state.pending_press = Some(PendingAttack {
        ticks: 0,
        binding: AttackBinding {
            verb: AttackVerb::Special,
            direction: AttackDir::Neutral,
        },
        hold_ticks: 0,
    });
    state.ticks_until_decision = 30;
    let mut out = ActorControlFrame::neutral();
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    assert!(out.special_pressed && !out.melee_pressed);
}

/// THE DIRECTION THE BRAIN CHOSE IS THE DIRECTION THE BODY RESOLVES — AT
/// EITHER FACING, AND AT THE STRENGTH THAT WAS ASKED FOR.
///
/// * the mirror. `attack_axis` is documented *"in the controlled actor's
///   local frame"* — the gravity-local frame `locomotion` is in — and
///   [`attack_dir_from_axis`] recovers *forward* by multiplying `axis.x` by the
///   body's `facing`. The brain wrote a FACING-relative `+x` for `Forward`, so
///   facing was applied twice and every forward/back attack chosen while the
///   body faced left came out reversed. Measured consequence: George Booul's
///   `special_forward` was selected 19–24 times per match and performed zero
///   times, while the body's ledger recorded `bivalence` presses the decision log
///   never chose — `Forward` mirrored to `Back`, no `special_back` exists, and
///   the chain fell back to the neutral special.
/// * the accidental smash. A full deflection is a FLICK, and a press inside
///   the flick window is a smash whatever the strength hint says. So a brain
///   that shoved the stick to 1.0 could not ask for a tilt at all.
///
/// nothing here is a restatement of the brain's own arithmetic: the axis
/// goes through `resolve_attack_gesture` — the production function
/// `resolve_attack_gestures` calls for every body, with the production tuning —
/// and the assertion is on what came out the far side.
#[test]
fn the_aimed_stick_round_trips_through_the_bodys_own_resolver() {
    use ambition_characters::actor::attack_gesture::{
        resolve_attack_gesture, AttackGestureState, AttackGestureTuning, AttackStrength,
    };

    let tuning = AttackGestureTuning::default();
    // What the body makes of a stick the brain aimed, pressed on the NEXT tick —
    // the real ordering, since `aim_the_stick` runs in the decision and the
    // press matures after it.
    let resolved = |binding: AttackBinding, facing: f32| {
        let mut frame = ActorControlFrame::neutral();
        super::aim_the_stick(binding, facing, &mut frame);
        let mut state = AttackGestureState::default();
        // Tick one: the stick moves, no button.
        resolve_attack_gesture(
            &mut state,
            tuning,
            frame.attack_axis,
            facing,
            true,
            false,
            false,
            false,
            false,
        );
        // Tick two: the button closes.
        let strong = matches!(binding.verb, AttackVerb::Smash);
        resolve_attack_gesture(
            &mut state,
            tuning,
            frame.attack_axis,
            facing,
            true,
            true,
            false,
            false,
            strong,
        )
        .pressed
        .expect("a press was requested")
    };

    for facing in [1.0_f32, -1.0] {
        for direction in [
            AttackDir::Forward,
            AttackDir::Back,
            AttackDir::Up,
            AttackDir::Down,
        ] {
            let out = resolved(
                AttackBinding {
                    verb: AttackVerb::Basic,
                    direction,
                },
                facing,
            );
            assert_eq!(
                out.direction, direction,
                "facing {facing}: the brain chose {direction:?} and the body \
                 resolved {:?} — a fighter whose forward attack points backwards \
                 half the time",
                out.direction
            );
            // and the STRENGTH, in the same breath: a Basic binding is a tilt.
            assert_eq!(
                out.strength,
                AttackStrength::Tilt,
                "facing {facing}, {direction:?}: the brain asked for a plain \
                 attack and the body resolved a SMASH, so the tilt half of every \
                 authored kit is unreachable"
            );
        }
    }

    // THE POISON, and it is the same resolver. A full deflection is still
    // a flick and still a smash — so the assertion above is measuring the
    // brain's chosen deflection rather than a resolver that answers `Tilt` to
    // everything.
    let mut frame = ActorControlFrame::neutral();
    super::aim_the_stick(
        AttackBinding {
            verb: AttackVerb::Smash,
            direction: AttackDir::Up,
        },
        1.0,
        &mut frame,
    );
    assert!(
        frame.attack_axis.length() >= tuning.flick_threshold,
        "a SMASH binding must shove the stick past the flick threshold, or the \
         brain has no way to ask for one"
    );
    let mut state = AttackGestureState::default();
    resolve_attack_gesture(
        &mut state,
        tuning,
        frame.attack_axis,
        1.0,
        true,
        false,
        false,
        false,
        false,
    );
    let smashed = resolve_attack_gesture(
        &mut state,
        tuning,
        frame.attack_axis,
        1.0,
        true,
        true,
        false,
        false,
        // the hint is FALSE on purpose: what is being shown is that the
        // deflection ALONE decides, which is exactly why the tilt deflection
        // above has to be below the threshold.
        false,
    )
    .pressed
    .expect("a press was requested");
    assert_eq!(
        smashed.strength,
        AttackStrength::Smash,
        "a full-deflection flick stopped meaning a smash, so the tilt assertion \
         above proves nothing about the brain's deflection"
    );
}

/// Why did this fighter choose this action? — asked headlessly, of the real
/// brain, without reading a line of engine implementation.
///
/// This is the program's first required inspector question and it is answered against the SAME
/// `tick_fighter` the game runs.
///
/// the assertion is on FIELDS, never on the summary sentence. A test that
/// greps prose breaks when somebody improves the wording, which teaches the next
/// person that improving the wording is dangerous.
#[cfg(feature = "causal")]
#[test]
fn the_inspector_answers_why_this_fighter_chose_this_action() {
    use ambition_causal::{domains, with_sink, CausalLog, FactValue, RecordingPolicy, SubjectKey};

    let (cfg, mut state) = rig(immediate_profile());
    let mut snapshot = BrainSnapshot::idle();
    // WHICH body, as the integration layer names it. See the sibling test below
    // for why an unattributed fact was not good enough.
    snapshot.subject = Some("fighter_1".to_string());
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::only([domains::BRAIN]));
    // The scope owner stamps the world's clock; the brain never guesses one.
    log.set_tick(41);

    let (log, ()) = with_sink(log, || {
        for _ in 0..6 {
            tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        }
    });

    // The fact is ABOUT this fighter. The id arrives through the snapshot's world-in port, filled
    // by the integration layer that assigns it.
    let explanation = log.explain(41, &SubjectKey::Sim("fighter_1".into()));
    let decision = explanation
        .first("fighter_decision")
        .expect("the brain decided at least once in six ticks");

    // 1. WHICH verb, as a field.
    let chose = decision
        .get("chose")
        .expect("every decision records its verb");
    assert!(
        !matches!(chose, FactValue::Text(text) if text == "None"),
        "a foe 200px away and open floor: the brain chose something — {chose}"
    );

    assert!(decision.get("offered").is_some());
    assert_eq!(
        decision.get("vetoed_count"),
        Some(&FactValue::Int(0)),
        "at depth 0 the rollout does not run, so nothing is vetoed"
    );

    // 3. The situation the choice was made in — enough to reconstruct the call.
    assert_eq!(decision.get("on_ground"), Some(&FactValue::Bool(true)));
    assert_eq!(decision.get("pos_x"), Some(&FactValue::Float(300.0)));

    // 4. And what actually reached the body, so the chain from decision to
    //    emitted input is closed without inferring it.
    assert!(matches!(
        decision.get("emit_locomotion_x"),
        Some(FactValue::Float(_))
    ));

    // 5. WHICH QUESTION the choice answered, and which ACTION it selected.
    //    A verb without its situation cannot be grouped, and the one histogram a
    //    platform-fighter brain is judged on is `situation → action`. This scene
    //    is a body on solid ground, 200px from a foe that is doing nothing and
    //    300px from every edge, which is the definition of neutral.
    assert_eq!(
        decision.get("situation"),
        Some(&FactValue::Text("Neutral".into())),
        "on flat ground with an idle foe in the middle distance the tick is \
         neutral; a different answer means L1 and this instrument disagree"
    );
    //    `BrainSnapshot::idle()` carries no attack kit, so there is no move to
    //    name — and "none" is the honest answer rather than an absent field.
    assert_eq!(
        decision.get("attack"),
        Some(&FactValue::Text("none".into())),
        "a body with an empty kit selected a move"
    );
    //    The recovery search runs in `Situation::Recovery` and nowhere else, so
    //    a `true` here would mean the brain is paying for kernel probes every
    //    neutral tick.
    assert_eq!(
        decision.get("recovery_searched"),
        Some(&FactValue::Bool(false))
    );

    // The tick is the one the OWNER stamped, not a brain-local counter.
    assert!(explanation.facts().iter().all(|fact| fact.tick == 41));
    println!("{}", explanation.render());
}

/// The instrument is OFF unless something opens a scope, and the brain behaves
/// identically either way.
///
/// The second half is the one that matters: an observer that changes what it
/// observes is not an observer. Same seed, same views, same frames.
#[cfg(feature = "causal")]
#[test]
fn recording_changes_nothing_about_what_the_brain_does() {
    use ambition_causal::{with_sink, CausalLog, RecordingPolicy};

    let (cfg, mut unobserved) = rig(immediate_profile());
    let quiet = run(&cfg, &mut unobserved, 20);

    let (cfg, mut observed) = rig(immediate_profile());
    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::All);
    let (log, loud) = with_sink(log, || run(&cfg, &mut observed, 20));

    assert!(!log.is_empty(), "the scope collected something to compare");
    assert_eq!(
        quiet, loud,
        "the brain emitted a different frame while being watched — an observer that changes \
         what it observes is not an observer, and under a rollback host it would be a desync"
    );
}

/// Two fighters, two explanations — the reason the subject had to arrive.
///
/// An unattributed decision fact is returned by `explain` for whatever subject
/// you ask about, so on a stage with two fighters the inspector answered "why
/// did THIS one walk off the edge" with both of their reasoning interleaved and
/// no way to separate it. For a fighting game that is every interesting tick.
///
/// Not two: an unattributed fact is returned for EVERY subject, so both fighters' whole streams
/// merge into each query.
#[cfg(feature = "causal")]
#[test]
fn two_fighters_facts_do_not_merge_into_one_explanation() {
    use ambition_causal::{domains, with_sink, CausalLog, RecordingPolicy, SubjectKey};

    let (cfg, mut left_state) = rig(immediate_profile());
    let (_, mut right_state) = rig(immediate_profile());
    let mut left = BrainSnapshot::idle();
    left.subject = Some("fighter_left".to_string());
    let mut right = BrainSnapshot::idle();
    right.subject = Some("fighter_right".to_string());

    // Deliberately different scenes, so the two streams are genuinely distinct
    // and a test that merged them could not accidentally pass.
    let left_view = scene(300.0, 500.0);
    let right_view = scene(500.0, 300.0);
    let mut out = ActorControlFrame::neutral();

    let mut log = CausalLog::default();
    log.set_policy(RecordingPolicy::only([domains::BRAIN]));
    log.set_tick(7);
    let (log, ()) = with_sink(log, || {
        for _ in 0..6 {
            tick_fighter(&cfg, &mut left_state, &left, Some(&left_view), &mut out);
            tick_fighter(&cfg, &mut right_state, &right, Some(&right_view), &mut out);
        }
    });

    for id in ["fighter_left", "fighter_right"] {
        let explanation = log.explain(7, &SubjectKey::Sim(id.into()));
        let decisions = explanation.all("fighter_decision").count();
        assert!(
            decisions > 0,
            "{id} decided at least once in six ticks and the fact is about it"
        );
        let other = explanation
            .all("fighter_decision")
            .filter(|fact| fact.subject != Some(SubjectKey::Sim(id.into())))
            .count();
        assert_eq!(
            other, 0,
            "asking about {id} returned {other} decision(s) belonging to the other fighter"
        );
    }
}

/// The kernel states the contract at `LocalAxes`: *"controlled-body-local axes:
/// `+x` local side/right … produced by resolving raw `ScreenAxes` against the
/// body's current `AccelerationFrame`"*. `apply_movement` wrote
/// `(foe.pos.x - self.pos.x).signum()` — a WORLD sign — into it, and the
/// conversion downstream (`LocalAxes::from_vec`) copies the components and
/// renames the type, so nothing performed the transform the type asserts.
///
/// the two conventions agree under screen-down gravity, which is why this never showed:
/// `side` is world `+x` there and `to_local` is the identity.
///
/// With gravity pointing world-LEFT the body's local side axis is world `+y`, so a foe displaced
/// along world `+x` sits on this body's GRAVITY axis — directly above or below it in its own frame.
/// There is no sideways throttle that closes on it, and every lateral this brain emits derives from
/// that same resolved sign.
#[test]
fn every_lateral_this_brain_emits_is_in_the_bodys_own_frame() {
    let lateral = |gravity_down: ae::Vec2| -> Vec<f32> {
        let (cfg, mut state) = rig(immediate_profile());
        let snapshot = BrainSnapshot::idle();
        let mut view = scene(300.0, 500.0);
        view.self_view.gravity_down = gravity_down;
        let mut out = ActorControlFrame::neutral();
        let mut seen = Vec::new();
        for _ in 0..40 {
            tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
            seen.push(out.locomotion.x);
        }
        seen
    };

    // Screen-down: unchanged behaviour. The body walks at the foe, so SOME tick
    // must carry a lateral — otherwise the rotated case below proves nothing,
    // because "no lateral" would be this brain's answer everywhere.
    let upright = lateral(ae::Vec2::new(0.0, 1.0));
    assert!(
        upright.iter().any(|x| x.abs() > 0.0),
        "the fighter emitted no lateral at all under normal gravity, so the \
         rotated assertion below would hold for the wrong reason: {upright:?}"
    );

    // Rotated a quarter turn: the foe is on the gravity axis, so no lateral
    // closes on it and every verb's resolved side sign is zero.
    let sideways = lateral(ae::Vec2::new(-1.0, 0.0));
    assert!(
        sideways.iter().all(|x| *x == 0.0),
        "with gravity pointing world-left the foe sits on this body's gravity \
         axis, so a sideways throttle cannot close on it — a non-zero here is a \
         WORLD x sign stamped into a body-local field: {sideways:?}"
    );
}

/// THE SMASH IS HELD, AND THAT IS WHAT MAKES IT A SMASH.
///
/// `smash_charge_mult` is authored per move and paid out against how long Attack
/// stays down, so a brain that only ever tapped was silently taking every
/// fighter's strongest option at its floor. The hold rides the pending press so
/// the situation that read the opening is the one that pays for it.
#[test]
fn a_smash_keeps_the_button_down_after_the_press() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    let binding = AttackBinding {
        verb: AttackVerb::Smash,
        direction: AttackDir::Forward,
    };
    super::aim_the_stick(binding, view.self_view.facing, &mut state.held);
    state.pending_press = Some(PendingAttack {
        ticks: 0,
        binding,
        hold_ticks: 4,
    });
    state.ticks_until_decision = 30;

    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    assert!(out.melee_pressed, "the press never came out");
    assert!(
        out.melee_held,
        "the charge was armed and the button was already up on the press frame — \
         the move freezes at its hold point and asks what Attack is doing"
    );

    // Three more ticks of hold, and then it lets go on its own. A charge that
    // depended on another decision to release it would hold through a rewind.
    for tick in 0..3 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        assert!(out.melee_held, "the charge let go on tick {tick} of 3");
    }
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    assert!(!out.melee_held, "the charge never released");
}

/// A tap is still a tap. Every non-smash press arms no hold, so nothing about
/// the ordinary jab changed.
#[test]
fn an_ordinary_press_holds_nothing() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    state.pending_press = Some(planted_press(0));
    state.ticks_until_decision = 30;
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);

    assert!(out.melee_pressed, "the press never came out");
    assert!(!out.melee_held, "a jab held the button down");
}
