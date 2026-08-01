//! Unit tests for the FB4b decision rig.
//!
//! The three careful pieces get one test each and the plumbing gets one: what a
//! rig like this gets wrong is never "did it call classify", it is cadence, the
//! APM ceiling, and a noise stream that does not reproduce.

use super::*;
use crate::actor::ActorFaction;
use crate::actor::attack_gesture::AttackDir;
use crate::brain::fighter::options::UtilityWeights;
use crate::brain::fighter::options::{AttackBinding, AttackVerb};
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

/// A press planted mid-flight: the ordinary forward jab, at `ticks` from
/// maturing. The BINDING is what the press will be — since 2026-07-31 the
/// pending action carries it, so a planted press has to name one too.
fn planted_press(ticks: u32) -> PendingAttack {
    PendingAttack {
        ticks,
        binding: AttackBinding {
            verb: AttackVerb::Basic,
            direction: AttackDir::Forward,
        },
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

/// **A queued press is CANCELLED when the body starts recovering.**
///
/// A press is armed at one decision and matures several ticks later, and the
/// situation can change in between. On a platform stage it does, in the one
/// direction that matters: the trace of a level-9 self-KO (2026-07-31) caught an
/// attack armed while airborne OVER the lip — still `Neutral` — maturing two
/// decisions later with the body past the edge and asking to `Recover`. Every
/// attack in this engine lunges, so the fighter's own queued swing carried it
/// further out while its emitted input said "back".
///
/// L2 already refuses to OFFER attacks in `Recovery`; this is that rule applied
/// to the press already in flight.
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

/// **One decision to Blink is ONE press edge.**
///
/// GPT 5.6, 2026-07-31 (finding 3). The brain stores its last emitted frame in
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
    held.drop_through = true;
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
            && !out.drop_through
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

/// **The chosen attack is PRESSED as itself.**
///
/// GPT 5.6, 2026-07-31 (finding 2): L2 scored every move, L3 refined the choice,
/// `RefinedChoice::move_id` named one — and the emission set `melee_pressed` with
/// a neutral axis, so the moveset resolved whatever the default gesture maps to.
/// The binding now rides the pending press through the execution jitter, and
/// this is the frame that comes out the other side.
///
/// ⚠ **this is the emission half only.** The review's acceptance condition is a
/// production `MovePlayback.spec.id`, and a test that observes `melee_pressed` is
/// the disconnected seam itself — see the S26 row in the 72h queue for the
/// end-to-end fixture this does not replace.
#[test]
fn a_pending_attack_matures_into_the_press_that_reaches_its_move() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let view = scene(300.0, 500.0);
    let mut out = ActorControlFrame::neutral();

    // An up-smash, decided two ticks ago and maturing now.
    state.pending_press = Some(PendingAttack {
        ticks: 0,
        binding: AttackBinding {
            verb: AttackVerb::Smash,
            direction: AttackDir::Up,
        },
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
    });
    state.ticks_until_decision = 30;
    let mut out = ActorControlFrame::neutral();
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
    assert!(out.special_pressed && !out.melee_pressed);
}

/// **Why did this fighter choose this action?** — asked headlessly, of the real
/// brain, without reading a line of engine implementation.
///
/// This is the program's first required inspector question and it is answered
/// against the SAME `tick_fighter` the game runs. What it replaces is
/// `AMBITION_FIGHTER_TRACE=1`: one `eprintln!` per decision, unqueryable,
/// uncorrelatable, and — by its own docstring — unable to tell an original tick
/// from a resimulated one.
///
/// ⚠ the assertion is on FIELDS, never on the summary sentence. A test that
/// greps prose breaks when somebody improves the wording, which teaches the next
/// person that improving the wording is dangerous.
#[cfg(feature = "causal")]
#[test]
fn the_inspector_answers_why_this_fighter_chose_this_action() {
    use ambition_causal::{CausalLog, FactValue, RecordingPolicy, SubjectKey, domains, with_sink};

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

    // The fact is ABOUT this fighter. It used to be about nobody — the brain has
    // no sim id of its own, so the fact was a world fact that `explain` returned
    // for any subject you asked about. The id arrives through the snapshot's
    // world-in port, filled by the integration layer that assigns it.
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

    // 2. And WHY it was available: what was offered, and what the rollout struck
    //    off. This is the pair the old text line existed to expose and the pair
    //    no test could previously assert on.
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
    use ambition_causal::{CausalLog, RecordingPolicy, with_sink};

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

/// **Two fighters, two explanations** — the reason the subject had to arrive.
///
/// An unattributed decision fact is returned by `explain` for whatever subject
/// you ask about, so on a stage with two fighters the inspector answered "why
/// did THIS one walk off the edge" with both of their reasoning interleaved and
/// no way to separate it. For a fighting game that is every interesting tick.
///
/// PROBED: with `snapshot.subject` left `None` on both — the state before this
/// row — asking about `fighter_left` returns **4** decisions belonging to the
/// other fighter. Not two: an unattributed fact is returned for EVERY subject,
/// so both fighters' whole streams merge into each query.
#[cfg(feature = "causal")]
#[test]
fn two_fighters_facts_do_not_merge_into_one_explanation() {
    use ambition_causal::{CausalLog, RecordingPolicy, SubjectKey, domains, with_sink};

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
