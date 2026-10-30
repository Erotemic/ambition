//! Unit tests for the FB4b decision rig.
//!
//! The three careful pieces get one test each and the plumbing gets one: what a
//! rig like this gets wrong is never "did it call classify", it is cadence, the
//! APM ceiling, and a noise stream that does not reproduce.

use super::*;
use ambition_characters::actor::attack_gesture::AttackDir;
use ambition_characters::actor::ActorFaction;
use ambition_characters::brain::attack_kit::{AttackBinding, AttackVerb};
use ambition_characters::brain::OwnMoveFact;
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
        move_ticks: 0,
    }
}

/// A body that can actually throw something, which nothing else in this module
/// supplies. `run` above hands the brain `BrainSnapshot::idle()` — an EMPTY
/// `attack_kit` — so no fixture here has ever produced a press from the brain's
/// own decision, and the noise stream is never sampled at all.
fn armed_snapshot() -> BrainSnapshot {
    BrainSnapshot {
        attack_kit: vec![ambition_characters::brain::attack_kit::AttackCandidate {
            move_id: "jab".to_string(),
            frames: ambition_entity_catalog::MoveFrameData {
                total_s: 0.35,
                charge_hold_at_s: None,
                startup_s: 0.05,
                active_spans: vec![(0.05, 0.15)],
                recovery_s: 0.2,
                cancel_windows: Vec::new(),
                reach: 60.0,
                ignores_guard: false,
                counter_stance: None,
                hazard: None,
                // `None` keeps every fixture's aim on `startup_s`, which is what the
                // lead read before this field existed. A fixture that means to test the
                // SPLIT states its own time.
                threat_live_at_s: None,
                coverage: Some(ambition_entity_catalog::MoveCoverage {
                    min: (0.0, -12.0),
                    max: (60.0, 12.0),
                }),
                // This fixture authors no windbox.
                push_coverage: None,
                push_dir: None,
                max_damage: 8,
                max_knockback: 40.0,
                launch: ambition_entity_catalog::LaunchEnvelope::default()
                    .with_volume(40.0, Some(1.0)),
                start_impulse: (0.0, 0.0),
                roots_steering: false,
                lift_speed: 0.0,
                lift_at_s: 0.0,
                lift_side: 0.0,
                recovery_route: Default::default(),
            },
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Forward,
            },
            legality: ambition_characters::brain::attack_kit::ActionLegality::Now,
            wear: ambition_characters::brain::attack_kit::MoveWear::FRESH,
        }],
        abilities: Some(ae::AbilitySet::basic()),
        ..BrainSnapshot::idle()
    }
}

/// `run`, but with a foe inside jab range and a body that owns a jab.
fn run_armed(cfg: &FighterCfg, state: &mut FighterState, ticks: u32) -> Vec<bool> {
    let snapshot = armed_snapshot();
    let view = scene(300.0, 340.0);
    let mut out = ActorControlFrame::neutral();
    let mut presses = Vec::new();
    for _ in 0..ticks {
        tick_fighter(cfg, state, &snapshot, Some(&view), &mut out);
        presses.push(out.melee_pressed);
    }
    presses
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

    // 2. HOLDING, facing LEFT and facing RIGHT — the case the unmirrored stick
    // got wrong. The throw is the situational one (`throw_direction`), and the
    // stick the frame carries must resolve to exactly that in the body's own
    // frame whichever way it faces; before the first pummel it asks nothing.
    use ambition_characters::actor::attack_gesture::{attack_dir_from_axis, AttackDir};
    use ambition_characters::perception::Perceived;
    for facing in [-1.0_f32, 1.0] {
        let mut view = scene(300.0, 500.0);
        view.self_view.facing = facing;
        let decided = throw_direction(Perceived::cheating(&view));
        assert_ne!(decided, AttackDir::Neutral, "a hold always decides a throw");
        for (pummels, expect) in [(0u8, AttackDir::Neutral), (1, decided)] {
            let (cfg, mut state) = rig(immediate_profile());
            let mut snapshot = BrainSnapshot::idle();
            snapshot.holding_captive = true;
            snapshot.pummels_landed = pummels;
            snapshot.actor_facing = facing;
            tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
            assert!(out.melee_pressed, "a captor did nothing with its captive");
            let asked = attack_dir_from_axis(out.attack_axis, snapshot.actor_facing, 0.2);
            assert_eq!(
                asked, expect,
                "with {pummels} pummel(s) landed and the body facing {facing:+}, the \
                 press resolved to {asked:?} where the hold decided {expect:?}"
            );
        }
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
///
/// ⛔⛔ THIS TEST WAS VACUOUS ON ITS OWN SUBJECT UNTIL 2026-09-10. It ran on
/// `run`, which hands the brain an EMPTY `attack_kit`, so no press was ever
/// wanted, the noise stream was never sampled, and it compared two all-`false`
/// vectors while asserting `a.noise == b.noise` on two seeds that had not moved.
/// Measured before the change: 0 presses of 90 frames, `a.noise` still the
/// initial seed. ⇒ The guard for the stream's replay determinism never
/// exercised the stream.
#[test]
fn the_same_seed_produces_the_same_fighter() {
    let mut profile = immediate_profile();
    profile.execution_noise = 0.9;
    let cfg = FighterCfg::new(profile);

    let seed = 0xABCD_EF01;
    let mut a = FighterState::new(&cfg, seed);
    let mut b = FighterState::new(&cfg, seed);
    let left_presses = run_armed(&cfg, &mut a, 90);
    let right_presses = run_armed(&cfg, &mut b, 90);

    // ⚠ THE FLOOR IS THE POINT. Two fighters that never press agree trivially,
    // and a fixture that stops offering an attack would turn every assertion
    // below green while proving nothing.
    assert!(
        left_presses.iter().any(|p| *p),
        "the fixture produced no press in 90 ticks, so nothing here samples the \
         noise stream and the comparisons are between two empty vectors"
    );
    assert_ne!(
        a.noise, seed,
        "the noise stream never advanced, so this measures a seed rather than a \
         stream"
    );

    assert_eq!(a.noise, b.noise, "the noise streams diverged");
    assert_eq!(a.apm, b.apm);
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

/// ⛔⛤ **EVERY AUTHORED RUNG CAN MOVE A PRESS, AND UNTIL 2026-09-12 THE TOP ONE
/// COULD NOT.**
///
/// The jitter used to be `(|sample| * noise * interval()).round()`, which
/// returns 0 for every possible sample whenever `noise * interval < 0.5` — and
/// the shipped rung 9 lands at `0.10 * 5 = 0.4999999701976776` in f32, **under
/// the rounding tie by 3e-8**. The hardest CPU therefore pressed exactly on its
/// decision ticks forever, against §1.3 of the fighter-brain design: level 9 is
/// *"small numbers, never zero — a frame-perfect CPU is not a hard opponent, it
/// is a different game."*
///
/// ⭐⭐ **THIS ARM NOW PINS THE PROPERTY RATHER THAN THE GAP, BECAUSE THE FIX IS
/// STRUCTURAL.** `decision.rs` quantizes probabilistically — whole ticks always,
/// the fractional tick with probability equal to the fraction — so
/// `P(jitter >= 1) = span / 2` for ANY span, and a nonzero execution noise can
/// never be a no-op. The old arm asserted rung 9 sat *"within a whisker"* of the
/// boundary and said *"do not fix it by widening the tolerance"*; the tolerance
/// was not widened, the mechanism was replaced, and there is no boundary left to
/// sit near.
///
/// ⚠ **THE INTERVAL SWEEP IS THE LOAD-BEARING HALF.** Raising rung 9's constant
/// until it cleared the tie would have been a number tuned to `interval == 5`,
/// and `decision_interval_ticks` is an AUTHORABLE field — a character choosing 3
/// would have put rungs 7-9 back under the boundary with nothing to say so. So
/// this asks every rung at every interval a character could plausibly author.
#[test]
fn every_authored_rung_can_actually_jitter_a_press_at_every_interval() {
    // ⛔⛤ **THIS SWEEP CALLS THE QUANTIZER. THE VERSION IT REPLACES DID NOT.**
    // The first repair asserted `noise * interval > 0.0` over the authored
    // ladder — arithmetic ABOUT the ladder, not a test OF `jitter_ticks` — so a
    // quantizer rewritten back to `round()` would have left it green while the
    // top rung went frame-perfect again. A guard that names what it protects
    // must call it.
    let mut unreachable = Vec::new();
    for interval in 1..=10u32 {
        for level in 1..=9u8 {
            let noise = crate::brain::fighter::shipped_rung(level).execution_noise;
            if noise <= 0.0 {
                continue;
            }
            let span = noise * interval as f32;
            // Sweep the unit square the two draws live in. `reached` is "some
            // (sample, dither) pair this quantizer can actually be handed
            // produces at least one tick" — which is the whole claim.
            let reached = (0..=20).any(|s| {
                (0..=20).any(|d| {
                    super::jitter_ticks(s as f32 / 20.0, d as f32 / 20.0, span) >= 1
                })
            });
            if !reached {
                unreachable.push((level, interval, noise, span));
            }
        }
    }
    assert!(
        unreachable.is_empty(),
        "a rung's execution noise cannot move a press at some authorable \
         decision interval: {unreachable:?}",
    );

    // ⛔ AND THE QUANTIZER MUST STILL BE ABLE TO SAY ZERO, or "never a no-op"
    // would have been bought by making every press late — which §1.3 would call
    // a different game just as loudly as a frame-perfect one.
    assert_eq!(
        super::jitter_ticks(0.0, 1.0, 0.5),
        0,
        "the quantizer cannot produce an on-time press any more"
    );

    // ⛔ AND THE ORDER IS THE OTHER HALF. "Small numbers, never zero" is two
    // claims; the arm above is the second. Expected jitter is `span / 2`, so a
    // ladder that made a lower rung crisper than a higher one would pass the
    // reachability check and still contradict the design.
    let interval = FighterCfg::new(crate::brain::fighter::shipped_rung(1)).interval() as f32;
    let mut previous = f32::INFINITY;
    for level in 1..=9u8 {
        let expected = crate::brain::fighter::shipped_rung(level).execution_noise * interval / 2.0;
        assert!(
            expected < previous,
            "rung {level}'s expected jitter ({expected} ticks) is not smaller \
             than the rung below it ({previous}) — the ladder no longer gets \
             crisper with level",
        );
        previous = expected;
    }
    assert!(
        previous > 0.0,
        "the top rung's expected jitter is {previous}, i.e. frame-perfect — \
         which §1.3 calls a different game rather than a hard opponent",
    );
}

/// ⛔⛔ THE SEAT-SYMMETRY FIX IS GUARDED BY SEED VALUES AND NOT BY BEHAVIOUR.
/// `brain_builders::cognition_stream_tests` proves two seats get different
/// stream VALUES; nothing anywhere proves a different stream produces a
/// different fighter. It does not at rung 9 — the sole consumer of the stream
/// is the jitter at `decision.rs:471`, and that rounds to zero there — so the
/// reflection those tests exist to prevent is present at the shipped top rung.
///
/// This asks the behaviour instead, and derives its own population so it heals:
/// the rungs it covers are exactly the rungs whose jitter is REACHABLE, computed
/// from the ladder. If rung 9's noise is ever retuned, this covers rung 9 with
/// no edit.
///
/// ⚠ The floor is the load-bearing half. Two fighters that never press agree
/// trivially, so a fixture that stopped pressing would turn this green while
/// proving nothing — which is the failure mode of
/// [`the_same_seed_produces_the_same_fighter`] above, whose press vectors are
/// compared but never counted.
#[test]
fn a_different_stream_makes_a_different_fighter_wherever_the_jitter_is_reachable() {
    const TICKS: u32 = 600;

    let mut covered = Vec::new();
    for level in 1..=9u8 {
        let profile = crate::brain::fighter::shipped_rung(level);
        // ⚠ REACHABILITY IS NOW "the noise is not zero", because the
        // quantization is probabilistic: `P(jitter >= 1) = span / 2`. This
        // condition used to be `(noise * interval).round() == 0`, which excluded
        // rung 9 — the rung the shipped game actually seats.
        if profile.execution_noise <= 0.0 {
            continue;
        }
        covered.push(level);

        let cfg = FighterCfg::new(profile);
        // Two seats of one character, as `fighter_cognition_seed` builds them:
        // the high half differs, the level term below is shared.
        let mut one = FighterState::new(&cfg, 0xd9b0_ccd3_02fd_1ed1);
        let mut two = FighterState::new(&cfg, 0xd8b0_cb3e_02fd_1ed1);
        let left = run_armed(&cfg, &mut one, TICKS);
        let right = run_armed(&cfg, &mut two, TICKS);

        let presses = left.iter().filter(|p| **p).count();
        assert!(
            presses > 0,
            "rung {level}: neither fighter pressed anything in {TICKS} ticks, so              this fixture cannot witness a jitter at all and its agreement is              vacuous"
        );
        assert_ne!(
            left, right,
            "rung {level}: two seats on DIFFERENT streams pressed on identical              ticks ({presses} presses), so the stream reaches no behaviour and              a mirror match is a reflection again"
        );
    }

    assert_eq!(
        covered,
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
        "the set of rungs whose jitter is reachable changed. ⛔ RUNG 9 WAS          ADOPTED 2026-09-12 when the quantization stopped rounding a small          span to nothing; if a rung is LOST, this test just stopped covering          it silently"
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
        open_for_self: false,
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
        open_for_self: false,
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
        open_for_self: false,
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
        move_ticks: 0,
    });
    state.ticks_until_decision = 30;
    tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);

    assert!(out.melee_pressed, "the press never came out");
    assert_eq!(
        out.melee_strength_hint,
        ambition_platformer2d_core::AttackStrengthHint::Smash,
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
        move_ticks: 0,
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
            ambition_platformer2d_core::AttackStrengthHint::Auto,
        );
        // Tick two: the button closes.
        let strong = if matches!(binding.verb, AttackVerb::Smash) {
            ambition_platformer2d_core::AttackStrengthHint::Smash
        } else {
            ambition_platformer2d_core::AttackStrengthHint::Auto
        };
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
        ambition_platformer2d_core::AttackStrengthHint::Auto,
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
        // the hint is `Auto` on purpose: what is being shown is that the
        // deflection ALONE decides, which is exactly why the tilt deflection
        // above has to be below the threshold.
        ambition_platformer2d_core::AttackStrengthHint::Auto,
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
/// `charge_mult` is authored per move and paid out against how long Attack
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
        move_ticks: 0,
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

/// ⭐ D184: A BODY OFF EITHER LEDGE MUST STEER BACK TOWARD THE STAGE.
///
/// `recovery_left` engages in real bouts while `recovery_right` dies at 0%, from
/// placements symmetric in every measured input. The DI overlay and the situation
/// CLASSIFIER were both cleared by test, which leaves the movement the decision
/// actually emits — this.
///
/// `gravity_down = (0,1)` makes `AccelerationFrame` the identity, so +x here is
/// world +x: a body past the RIGHT ledge must ask for NEGATIVE x, and one past the
/// LEFT ledge for positive.
#[test]
fn a_body_off_either_ledge_steers_back_toward_the_stage() {
    let steer = |me_x: f32| -> f32 {
        let mut cfg = FighterCfg::new(immediate_profile());
        cfg.decision_interval_ticks = 1;
        let mut state = FighterState::new(&cfg, 0x5EED);
        let snapshot = BrainSnapshot::idle();
        let mut view = scene(me_x, 400.0);
        // Off the ledge, not standing on it.
        view.self_view.on_ground = false;
        let mut out = ActorControlFrame::neutral();
        // Several ticks: the first decision arms, later ones emit.
        let mut last = 0.0;
        for _ in 0..8 {
            tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
            if out.locomotion.x != 0.0 {
                last = out.locomotion.x;
            }
        }
        last
    };
    let past_right = steer(880.0);
    let past_left = steer(-80.0);
    assert!(
        past_right < 0.0,
        "a body past the RIGHT ledge must steer back LEFT, asked {past_right}"
    );
    assert!(
        past_left > 0.0,
        "a body past the LEFT ledge must steer back RIGHT, asked {past_left}"
    );
}

/// ⭐ D184: A BODY BELOW THE STAGE MUST ASK TO GO UP.
///
/// `recovery_below` is the worst fixture in the ladder — vacuous at TWO rung
/// pairs (3v1 and 6v5) where both fighters die untouched at 0%, reproduced at 45
/// seeds. `recovery_right` fails at one. Left and above never fail.
///
/// Steering LEFT/RIGHT is symmetric by test
/// (`a_body_off_either_ledge_steers_back_toward_the_stage`), which leaves the
/// VERTICAL half: a body under the stage recovers only by jumping, so if the
/// decision never presses jump there is nothing to recover with.
///
/// ⚠ This asks only whether the brain EVER asks to rise while below and
/// airborne. It deliberately does not assert a particular move: which recovery a
/// character owns is a kit question, and pinning one here would be a test of the
/// roster rather than of the decision.
#[test]
fn a_body_below_the_stage_asks_to_rise() {
    let mut cfg = FighterCfg::new(immediate_profile());
    cfg.decision_interval_ticks = 1;
    let mut state = FighterState::new(&cfg, 0x5EED);
    let snapshot = BrainSnapshot::idle();
    // Under the stage, in the air, foe at centre. `stage()` spans 0..800 x 0..600.
    let mut view = scene(400.0, 400.0);
    view.self_view.pos = ae::Vec2::new(400.0, 640.0);
    view.self_view.on_ground = false;
    let mut out = ActorControlFrame::neutral();
    let mut asked_to_rise = false;
    for _ in 0..16 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        // `LocalAxes` +y is TOWARD THE FEET under `gravity_down = (0,1)`, so
        // "up" is NEGATIVE y — and a jump press is the other way to ask.
        if out.jump_pressed || out.jump_held || out.locomotion.y < 0.0 {
            asked_to_rise = true;
            break;
        }
    }
    assert!(
        asked_to_rise,
        "a fighter below the stage never asked to rise in 16 decisions — with no \
         upward request there is nothing for a recovery to execute, which is what \
         `recovery_below` measures when both seats die at 0%"
    );

    // ⭐ THE CONTROL, WITHOUT WHICH THE ARM ABOVE CANNOT FAIL. `jump_held` is a
    // broad signal and a fighter jumps around; if a body standing at centre also
    // "asked to rise", the assertion would pass on any brain at all. Measured: it
    // does not — neither grounded NOR airborne at centre asks to rise in 16
    // decisions, while below-and-airborne presses jump on decision ZERO.
    let mut cfg = FighterCfg::new(immediate_profile());
    cfg.decision_interval_ticks = 1;
    let mut state = FighterState::new(&cfg, 0x5EED);
    let mut grounded = scene(400.0, 400.0);
    grounded.self_view.pos = ae::Vec2::new(400.0, 300.0);
    let mut out = ActorControlFrame::neutral();
    for _ in 0..16 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&grounded), &mut out);
        assert!(
            !(out.jump_pressed || out.jump_held || out.locomotion.y < 0.0),
            "a body standing at centre asked to RISE, so the arm above cannot \
             distinguish a recovering fighter from an ordinary one"
        );
    }
}

/// ⭐⭐ D184: DOES *ROLLOUT* STOP A FIGHTER ASKING TO RISE?
///
/// Measured in the ladder (`--sweep-below`, partner pinned at l5, 45 seeds): the
/// level placed below the stage fails to recover at **l1 and l6** and succeeds at
/// **l3, l5, l9**. l1 is unsurprising — the slowest reaction and the most
/// execution noise. **l6 is a REGRESSION**, and it is exactly the level where
/// the old level curve (`for_level`, deleted 2026-10-16) switched
/// `rollout_depth`/`rollout_k` on (`level >= 6`), while l9
/// carries the same rollout with a better reaction and less noise and recovers.
///
/// So: is the DECISION different at l6, or does l6 still ask to rise and lose the
/// recovery downstream? This asks the brain directly, at each published level.
#[test]
fn every_published_level_below_the_stage_asks_to_rise() {
    for level in [1u8, 3, 5, 6, 9] {
        let mut cfg = FighterCfg::new(crate::brain::fighter::shipped_rung(level));
        cfg.decision_interval_ticks = 1;
        let mut state = FighterState::new(&cfg, 0x5EED);
        let snapshot = BrainSnapshot::idle();
        let mut view = scene(400.0, 400.0);
        view.self_view.pos = ae::Vec2::new(400.0, 640.0);
        view.self_view.on_ground = false;
        let mut out = ActorControlFrame::neutral();
        // ⭐ EVERY decision, not just one. A fighter that asks to rise and then
        // changes its mind falls anyway, so "asked at least once" would not
        // distinguish a committed recovery from dithering. Measured: all five
        // published levels ask on 120 of 120 decisions.
        //
        // ⚠ The view is CONSTANT here — the body does not actually move — so this
        // pins that the decision is stable for a fixed below-stage state, not
        // that it stays stable as the fall develops.
        let mut rises = 0;
        for _ in 0..120 {
            tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
            if out.jump_pressed || out.jump_held || out.locomotion.y < 0.0 {
                rises += 1;
            }
        }
        assert_eq!(
            rises, 120,
            "level {level} asked to rise on only {rises} of 120 decisions while \
             below the stage. The ladder measures l1 and l6 failing to recover \
             from here while l3/l5/l9 succeed; if a level ever stops asking, that \
             failure is a DECISION defect rather than downstream execution"
        );
    }
}

/// ⭐⭐ THE THREE TIERS `pick_movement` RANKS BY, one arm each.
///
/// ⛔⛔ THE MIDDLE ONE IS THE DEFECT THIS FILE WAS OPENED FOR. An unmodelled
/// verb is absent from `vetoed`, so a `find` over "not vetoed" promoted it above
/// every verb the rollout judged AND suppressed `least_bad_movement`, which was
/// gated on every OFFERED verb being vetoed. Measured on seed 0 of
/// `ladder_rig --sweep-below`, two ticks before level 6 died at 0%:
/// `vetoed=[Approach, Jump] unmodelled=[Dodge] chose=Some(Dodge)
/// least_bad=Some(Approach)`.
mod movement_ranking {
    use ambition_characters::brain::fighter::options::{MoveOption, MovementVerb};

    fn offered(verbs: &[(MovementVerb, f32)]) -> Vec<MoveOption> {
        verbs
            .iter()
            .map(|(verb, score)| MoveOption {
                verb: *verb,
                score: *score,
            })
            .collect()
    }

    /// Tier 1: a judged, unvetoed verb wins outright.
    #[test]
    fn a_line_the_rollout_modelled_and_did_not_veto_wins() {
        let options = offered(&[
            (MovementVerb::Approach, 1.0),
            (MovementVerb::Dodge, 0.9),
            (MovementVerb::Retreat, 0.5),
        ]);
        assert_eq!(
            super::super::pick_movement(
                &options,
                &[MovementVerb::Approach],
                &[MovementVerb::Dodge],
                Some(MovementVerb::Approach),
            ),
            Some(MovementVerb::Retreat),
            "Retreat is modelled and unvetoed; nothing should outrank it"
        );
    }

    /// Tier 2, and the regression: every JUDGED verb is fatal, an unmodelled one
    /// is on offer, and the rollout's own least-bad answer must still win.
    #[test]
    fn the_least_bad_line_outranks_a_verb_nobody_rolled() {
        let options = offered(&[
            (MovementVerb::Approach, 1.0),
            (MovementVerb::Dodge, 0.9),
            (MovementVerb::Jump, 0.5),
        ]);
        assert_eq!(
            super::super::pick_movement(
                &options,
                &[MovementVerb::Approach, MovementVerb::Jump],
                &[MovementVerb::Dodge],
                Some(MovementVerb::Approach),
            ),
            Some(MovementVerb::Approach),
            "the rollout had an answer for `everything I can judge is fatal` and \
             it was discarded for a verb nobody rolled"
        );
    }

    /// Tier 3: nothing was judged at all — no rollout, or every offered verb is
    /// one the shadow cannot simulate. L2's order is the only ranking there is,
    /// and this is also the whole no-rollout path.
    #[test]
    fn with_nothing_judged_l2s_order_comes_straight_through() {
        let options = offered(&[(MovementVerb::Dodge, 1.0), (MovementVerb::Blink, 0.9)]);
        assert_eq!(
            super::super::pick_movement(
                &options,
                &[],
                &[MovementVerb::Dodge, MovementVerb::Blink],
                None,
            ),
            Some(MovementVerb::Dodge),
        );
        // The no-rollout shape: nothing vetoed, nothing unmodelled, no least-bad.
        assert_eq!(
            super::super::pick_movement(&options, &[], &[], None),
            Some(MovementVerb::Dodge),
            "with rollouts off, L2's order must come through untouched"
        );
    }

    /// ⛔ AND AN EMPTY OFFER IS STILL `None`. A tier that invents a verb out of
    /// an empty list would press a direction nobody scored.
    #[test]
    fn nothing_offered_chooses_nothing() {
        assert_eq!(super::super::pick_movement(&[], &[], &[], None), None);
    }
}

/// ⭐⭐ **WHEN NOTHING IN THE KIT CAN TOUCH THEM, THE MOVE THAT TRAVELS IS WHAT
/// THE DECISION REACHES FOR — and the arm that does that had no test.**
///
/// `wants_attack` ends in `.or_else(|| options.best_motion())`, which only ever
/// runs on a decision where the attack menu came out EMPTY. Every fixture in
/// this module hands the brain a foe inside jab range, so nothing here could
/// reach the arm, and the whole-grid sweep cannot isolate it either: it shows
/// an outcome, not which branch produced it.
///
/// ⚠ THE DISCRIMINATOR IS THE BUTTON. The jab is `Basic` and the launcher is
/// `Special`, so `special_pressed` without `melee_pressed` says the motion arm
/// answered and the ranking did not.
#[test]
fn a_kit_that_cannot_reach_presses_the_move_that_closes_the_distance() {
    let mut snapshot = armed_snapshot();
    let mut launcher = snapshot.attack_kit[0].frames.clone();
    // Touches nobody and moves its owner — the shape `motion_options` files.
    launcher.coverage = None;
    launcher.push_coverage = None;
    launcher.max_damage = 0;
    launcher.reach = 0.0;
    // Sideways, toward the facing, which is where the foe is.
    launcher.lift_side = 700.0;
    launcher.lift_speed = 0.0;
    snapshot
        .attack_kit
        .push(ambition_characters::brain::attack_kit::AttackCandidate {
            move_id: "dash_in".to_string(),
            frames: launcher,
            binding: AttackBinding {
                verb: AttackVerb::Special,
                direction: AttackDir::Forward,
            },
            legality: ambition_characters::brain::attack_kit::ActionLegality::Now,
            wear: ambition_characters::brain::attack_kit::MoveWear::FRESH,
        });

    let (cfg, mut state) = rig(immediate_profile());
    // Four hundred pixels: far outside the 60px jab, and outside the admission
    // slack that makes a near-miss still worth offering.
    let view = scene(300.0, 700.0);
    let mut out = ActorControlFrame::neutral();
    let mut pressed_special = false;
    let mut pressed_melee = false;
    for _ in 0..120 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        pressed_special |= out.special_pressed;
        pressed_melee |= out.melee_pressed;
    }
    assert!(
        pressed_special,
        "no tick reached for the one move that closes 400px, so the fallback \
         that exists for exactly this decision never runs"
    );
    assert!(
        !pressed_melee,
        "a 60px jab was pressed at a 400px gap, which is the admission defect \
         this fixture is staged past"
    );
}

/// One scene of the reflection test: the decider and its foe, in world x
/// and lateral velocity, with facings and footing.
#[derive(Clone, Copy, Debug)]
struct MirrorCase {
    me_x: f32,
    me_vx: f32,
    me_facing: f32,
    foe_x: f32,
    foe_vx: f32,
    foe_facing: f32,
    grounded: bool,
}

/// The stage of `scene` is centred on this x.
const STAGE_CENTRE_X: f32 = 400.0;

fn mirror_scene(case: MirrorCase, reflected: bool) -> WorldView {
    let x = |x: f32| if reflected { 2.0 * STAGE_CENTRE_X - x } else { x };
    let s = if reflected { -1.0 } else { 1.0 };
    let mut view = scene(x(case.me_x), x(case.foe_x));
    view.self_view.vel = ae::Vec2::new(s * case.me_vx, 0.0);
    view.self_view.facing = s * case.me_facing;
    view.self_view.on_ground = case.grounded;
    let foe = &mut view.actors[0];
    foe.vel = ae::Vec2::new(s * case.foe_vx, 0.0);
    foe.facing = s * case.foe_facing;
    foe.on_ground = case.grounded;
    view
}

/// The first field where `mirror` is not the reflection of `frame`.
fn first_unreflected_field(frame: &ActorControlFrame, mirror: &ActorControlFrame) -> Option<String> {
    let near = |a: f32, b: f32| (a - b).abs() <= 1e-4;
    let lateral = [
        ("locomotion.x", frame.locomotion.x, mirror.locomotion.x),
        ("attack_axis.x", frame.attack_axis.x, mirror.attack_axis.x),
        ("velocity_target.x", frame.velocity_target.0.x, mirror.velocity_target.0.x),
        ("facing", frame.facing, mirror.facing),
    ];
    for (name, a, b) in lateral {
        if !near(a, -b) {
            return Some(format!("{name}: {a} against reflected {b}"));
        }
    }
    let vertical = [
        ("locomotion.y", frame.locomotion.y, mirror.locomotion.y),
        ("attack_axis.y", frame.attack_axis.y, mirror.attack_axis.y),
        ("velocity_target.y", frame.velocity_target.0.y, mirror.velocity_target.0.y),
    ];
    for (name, a, b) in vertical {
        if !near(a, b) {
            return Some(format!("{name}: {a} against {b}"));
        }
    }
    let buttons = [
        ("melee_pressed", frame.melee_pressed, mirror.melee_pressed),
        ("melee_held", frame.melee_held, mirror.melee_held),
        ("melee_released", frame.melee_released, mirror.melee_released),
        ("jump_pressed", frame.jump_pressed, mirror.jump_pressed),
        ("jump_held", frame.jump_held, mirror.jump_held),
        ("burst_pressed", frame.burst_pressed, mirror.burst_pressed),
        ("fire", frame.fire.is_some(), mirror.fire.is_some()),
    ];
    for (name, a, b) in buttons {
        if a != b {
            return Some(format!("{name}: {a} against {b}"));
        }
    }
    None
}

/// ⭐ Q49: THE DECISION OF A REFLECTED SCENE IS THE REFLECTED DECISION.
///
/// One brain, one profile, one seed. Each case is stepped twice: as written,
/// and reflected about the stage centre (positions, lateral velocities and
/// facings). Each emitted frame must be the reflection of the other: lateral
/// fields negated, vertical fields and buttons equal. A side chosen by a
/// constant (`signum(0.0) = +1`, a left-first search) gives the same side in
/// both scenes and fails here.
///
/// The stacked cases put both bodies at one x, where "toward the foe" is not
/// a side: the facing is the only fact that can choose one.
#[test]
fn the_decision_of_a_reflected_scene_is_the_reflected_decision() {
    let case = |me_x, me_vx, me_facing, foe_x, foe_vx, foe_facing, grounded| MirrorCase {
        me_x,
        me_vx,
        me_facing,
        foe_x,
        foe_vx,
        foe_facing,
        grounded,
    };
    let cases = [
        case(300.0, 0.0, 1.0, 500.0, 0.0, -1.0, true),
        case(300.0, 0.0, 1.0, 340.0, 0.0, -1.0, true),
        case(300.0, 120.0, 1.0, 360.0, -80.0, -1.0, true),
        case(250.0, -60.0, -1.0, 330.0, 0.0, 1.0, false),
        case(-80.0, 0.0, 1.0, 400.0, 0.0, -1.0, false),
        // Stacked: one x for both bodies.
        case(400.0, 0.0, 1.0, 400.0, 0.0, -1.0, true),
        case(300.0, 0.0, -1.0, 300.0, 0.0, 1.0, true),
        case(300.0, 0.0, 1.0, 300.0, 0.0, 1.0, false),
    ];
    let mut profiles = vec![("immediate", immediate_profile())];
    for level in [3, 6, 9] {
        let mut profile = crate::brain::fighter::shipped_rung(level);
        // The execution stream is a per-seat fact. Q49 allows it only as a
        // modelled asymmetry, so this test removes it.
        profile.execution_noise = 0.0;
        profiles.push(("rung", profile));
    }
    let mut failures = Vec::new();
    let mut lateral_seen = false;
    for (label, profile) in &profiles {
        for (index, case) in cases.iter().enumerate() {
            let snapshot = armed_snapshot();
            let (cfg, mut state) = rig(profile.clone());
            let (_, mut mirror_state) = rig(profile.clone());
            let view = mirror_scene(*case, false);
            let mirror_view = mirror_scene(*case, true);
            let mut out = ActorControlFrame::neutral();
            let mut mirror_out = ActorControlFrame::neutral();
            for tick in 0..60 {
                tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
                tick_fighter(&cfg, &mut mirror_state, &snapshot, Some(&mirror_view), &mut mirror_out);
                lateral_seen |= out.locomotion.x != 0.0;
                if let Some(field) = first_unreflected_field(&out, &mirror_out) {
                    failures.push(format!(
                        "{label} level {} case {index} {case:?}, tick {tick}: {field}",
                        profile.level
                    ));
                    break;
                }
            }
        }
    }
    assert!(
        lateral_seen,
        "premise — no case emitted a lateral, so negation was never tested"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A recovering body below the top of the stage does not steer in under it.
/// It steered to the centre of the room, which is over the floor: from below
/// the top that is under the floor, and the floor stopped its jumps until it
/// fell out of the room (the George mirror bout, seat 0, x = 571, y = 353).
///
/// The stage is the smash stage: a floor from x = 80 to 560, top at y = 300.
/// Each case is also asked mirrored about x = 320, so the rule cannot hold on
/// one side only.
#[test]
fn a_recovering_body_below_the_top_does_not_steer_under_the_floor() {
    let steer_at = |x: f32, y: f32| {
        let mut view = scene(x, 320.0);
        view.self_view.pos = ae::Vec2::new(x, y);
        view.self_view.on_ground = false;
        view.self_view.half_extent = ae::Vec2::new(12.0, 20.0);
        view.terrain = vec![ambition_characters::perception::PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(320.0, 316.0), ae::Vec2::new(240.0, 16.0)),
            kind: ambition_characters::perception::SolidKind::Solid,
            open_for_self: false,
        }];
        let mut out = ActorControlFrame::neutral();
        apply_movement(
            MovementVerb::Recover,
            ambition_characters::perception::Perceived::cheating(&view),
            &mut out,
        );
        assert!(out.jump_held, "a recovery jumps");
        (out.locomotion.x, out.facing)
    };
    for side in [1.0_f32, -1.0] {
        let x = |right_x: f32| 320.0 + side * (right_x - 320.0);
        // Beside the floor and below its top: hold the side, face the floor.
        assert_eq!(steer_at(x(600.0), 360.0), (0.0, -side), "beside and below, side {side}");
        // Under the floor: go out from under it.
        assert_eq!(steer_at(x(550.0), 360.0), (side, -side), "under the floor, side {side}");
        // Control: above the top, go in.
        assert_eq!(steer_at(x(600.0), 250.0), (-side, -side), "above the top, side {side}");
    }
}

/// Twin floors seen from the centre line tie on side distance and on height.
/// The floor the body faces wins, so two mirrored bodies steer mirrored, and
/// the list order of the terrain decides nothing (Q49).
#[test]
fn twin_floors_are_told_apart_by_the_facing_and_not_by_the_list() {
    let ledge_for = |facing: f32, reversed: bool| {
        let mut view = scene(320.0, 500.0);
        view.self_view.pos = ae::Vec2::new(320.0, 360.0);
        view.self_view.facing = facing;
        view.self_view.half_extent = ae::Vec2::new(12.0, 20.0);
        let shelf = |cx: f32| ambition_characters::perception::PerceivedSolid {
            aabb: ae::Aabb::new(ae::Vec2::new(cx, 316.0), ae::Vec2::new(48.0, 16.0)),
            kind: ambition_characters::perception::SolidKind::Solid,
            open_for_self: false,
        };
        view.terrain = vec![shelf(220.0), shelf(420.0)];
        if reversed {
            view.terrain.reverse();
        }
        view.recovery_ledge().expect("a floor").toward
    };
    for reversed in [false, true] {
        assert_eq!(ledge_for(1.0, reversed), 100.0, "facing right, list reversed {reversed}");
        assert_eq!(ledge_for(-1.0, reversed), -100.0, "facing left, list reversed {reversed}");
    }
}

/// How the body under a test brain answers its presses: each melee press
/// starts a fresh use of the move on the next tick, which plays `move_ticks`
/// and touches a body when `lands` says so.
#[derive(Clone, Copy)]
struct Body {
    /// A press starts a move at all.
    plays: bool,
    /// The use of the move touches a body on its fourth tick.
    lands: bool,
}

/// Drive a brain for `ticks` against `view`, with `body` answering its
/// presses. Returns each press (tick, binding) and the brain's last state.
fn drive(
    snapshot: &BrainSnapshot,
    view: &WorldView,
    body: Body,
    ticks: u32,
) -> (Vec<(u32, AttackBinding)>, FighterState) {
    let (cfg, mut state) = rig(immediate_profile());
    let mut snapshot = snapshot.clone();
    let mut out = ActorControlFrame::neutral();
    let mut presses = Vec::new();
    let move_ticks = (0.35f32 * 60.0).ceil() as u32;
    let mut instance = 7u32;
    let mut started: Option<u32> = None;
    for tick in 0..ticks {
        snapshot.own_move = started.filter(|at| tick - at < move_ticks).map(|at| OwnMoveFact {
            instance,
            contact: ambition_entity_catalog::MoveContact {
                overlapped: body.lands && tick - at >= 3,
                connected: body.lands && tick - at >= 3,
                blocked: false,
            },
        });
        let pending = state.pending_press;
        tick_fighter(&cfg, &mut state, &snapshot, Some(view), &mut out);
        if out.melee_pressed {
            presses.push((tick, pending.expect("a press comes from an armed attack").binding));
            if body.plays {
                instance += 1;
                started = Some(tick + 1);
            }
        }
    }
    (presses, state)
}

/// The armed kit with a second, weaker move bound on another direction.
fn jab_and_poke() -> (BrainSnapshot, AttackBinding, AttackBinding) {
    let mut snapshot = armed_snapshot();
    let mut poke = snapshot.attack_kit[0].clone();
    poke.move_id = "poke".to_string();
    poke.frames.max_damage = 2;
    poke.binding = AttackBinding {
        verb: AttackVerb::Basic,
        direction: AttackDir::Down,
    };
    snapshot.attack_kit.push(poke);
    let jab = snapshot.attack_kit[0].binding;
    let poke = snapshot.attack_kit[1].binding;
    (snapshot, jab, poke)
}

/// BRAIN road 1: a brain that whiffed declines that binding for as long as the
/// move plays, and throws the next attack of its ranking instead. With no
/// memory of its own move it threw the same move until the world changed (the
/// medic: 177 presses of one move, none landed).
///
/// ⛔ THE OUTCOME IS ITS OWN MOVE'S RECORD, NOT THE FOE REELING. Here the foe
/// is in hitstun the whole time (another body's hit), and this brain's jab
/// touches nothing: it whiffed, and it is declined. The brain read the foe's
/// hitstun as its own hit before (review 2026-10-12).
#[test]
fn a_brain_that_whiffed_declines_that_move_for_as_long_as_it_plays() {
    let (snapshot, jab, poke) = jab_and_poke();
    let mut view = scene(300.0, 340.0);
    view.actors[0].phase = ambition_characters::perception::BodyPhase::Hitstun;
    let (whiffed, _) = drive(&snapshot, &view, Body { plays: true, lands: false }, 120);
    assert_eq!(whiffed.first().map(|(_, binding)| *binding), Some(jab), "premise: the jab ranks first: {whiffed:?}");
    // The jab plays 0.35 s, 21 ticks at 60 Hz.
    let move_ticks = (0.35f32 * 60.0).ceil() as u32;
    let first = whiffed[0].0;
    assert!(
        whiffed.iter().all(|(tick, binding)| *tick > first + move_ticks || *binding == jab),
        "nothing is declined before the jab's time is over: {whiffed:?}"
    );
    assert!(
        whiffed.iter().any(|(tick, binding)| *binding == poke && *tick > first + move_ticks && *tick <= first + 2 * move_ticks + 2),
        "a jab that touched nothing is declined, though the foe reels from another hit: {whiffed:?}"
    );
}

/// The control: the same brain's jab touches the foe, and the foe is not seen
/// reeling (a hit the brain has not seen yet, or a block). It landed, so
/// nothing is declined and the best move stays the only one thrown.
#[test]
fn a_brain_whose_own_move_touched_the_foe_declines_nothing() {
    let (snapshot, jab, _) = jab_and_poke();
    let view = scene(300.0, 340.0);
    let (landed, state) = drive(&snapshot, &view, Body { plays: true, lands: true }, 120);
    assert!(
        !landed.is_empty() && landed.iter().all(|(_, binding)| *binding == jab),
        "a jab that landed is not declined: {landed:?}"
    );
    assert_eq!(state.refused, None);
}

/// A press the body does not play (the engine refused it) is no whiff: the
/// move was not thrown, so nothing is declined.
#[test]
fn a_press_that_starts_no_move_declines_nothing() {
    let (snapshot, jab, _) = jab_and_poke();
    let view = scene(300.0, 340.0);
    let (presses, state) = drive(&snapshot, &view, Body { plays: false, lands: false }, 120);
    assert!(
        presses.len() > 1 && presses.iter().all(|(_, binding)| *binding == jab),
        "an unplayed jab is pressed again, not declined: {presses:?}"
    );
    assert_eq!(state.refused, None);
}

/// The use of a move that plays when the press is made is not the press's
/// move. A press over a jab that touched the foe starts a fresh jab; that one
/// touches nothing, and it is declined.
#[test]
fn the_move_playing_at_the_press_is_not_the_pressed_move() {
    let (snapshot, jab, _) = jab_and_poke();
    let (cfg, mut state) = rig(immediate_profile());
    let touched = ambition_entity_catalog::MoveContact { overlapped: true, connected: true, blocked: false };
    let playing = |instance: u32, contact| Some(OwnMoveFact { instance, contact });
    state.own_attack = Some(OwnAttack {
        binding: jab,
        watch_ticks: 40,
        refuse_ticks: 21,
        before: Some(3),
        instance: None,
        landed: false,
    });
    let mut snapshot = snapshot.clone();
    // The old jab still plays, and it touched the foe: not this press's move.
    snapshot.own_move = playing(3, touched);
    watch_the_own_attack(&mut state, &snapshot);
    assert_eq!(state.own_attack.map(|own| (own.instance, own.landed)), Some((None, false)));
    // The fresh jab plays and touches nothing, then it is over.
    snapshot.own_move = playing(4, Default::default());
    watch_the_own_attack(&mut state, &snapshot);
    snapshot.own_move = None;
    watch_the_own_attack(&mut state, &snapshot);
    assert_eq!(state.own_attack, None, "the move is over when the body plays none");
    assert_eq!(state.refused.map(|refused| refused.binding), Some(jab), "the fresh jab whiffed");
    let _ = cfg;
}

/// ALL FOUR THROWS, each from the situation that is its own, and mirrored.
///
/// The forward throw was the only one ever thrown (2026-10-29 census). The
/// stage is `scene`'s 800 px room: a captor at `x = 700` facing right has the
/// edge AHEAD; facing left it is BEHIND; the mirror at `x = 100` reflects both.
#[test]
fn a_hold_is_thrown_by_percent_and_by_where_the_edge_is() {
    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::perception::Perceived;
    let throw = |me_x: f32, facing: f32, damage: i32| {
        let mut view = scene(me_x, me_x + 20.0 * facing);
        view.self_view.facing = facing;
        view.actors[0].damage_taken = damage;
        view.actors[0].health_max = 100;
        throw_direction(Perceived::cheating(&view))
    };
    // Kill percent: toward the nearer blast line, whichever way I face.
    assert_eq!(throw(700.0, 1.0, 70), AttackDir::Forward);
    assert_eq!(throw(700.0, -1.0, 70), AttackDir::Back);
    assert_eq!(throw(100.0, -1.0, 70), AttackDir::Forward, "the mirror");
    assert_eq!(throw(100.0, 1.0, 70), AttackDir::Back, "the mirror");
    // Set-up percent: off the stage behind me, or up into a juggle.
    assert_eq!(throw(700.0, -1.0, 40), AttackDir::Back);
    assert_eq!(throw(700.0, 1.0, 40), AttackDir::Up);
    // Low percent: a tech chase needs room on both sides; at an edge, up.
    assert_eq!(throw(400.0, 1.0, 10), AttackDir::Down);
    assert_eq!(throw(700.0, 1.0, 10), AttackDir::Up);
}

/// THE MIX draws among the options within the margin of the best, and only
/// those: a verb a tier below never comes out, and a zero margin is the old
/// argmax.
#[test]
fn the_mix_picks_among_near_equals_and_never_below_the_margin() {
    use ambition_characters::brain::fighter::options::MoveOption;
    let options = [
        MoveOption { verb: MovementVerb::Shield, score: 0.9 },
        MoveOption { verb: MovementVerb::Dodge, score: 0.85 },
        MoveOption { verb: MovementVerb::Approach, score: 0.5 },
    ];
    let pick = |mix: f32, margin: f32| pick_movement_mixed(&options, &[], &[], None, mix, margin);
    assert_eq!(pick(0.1, 0.08), Some(MovementVerb::Shield));
    assert_eq!(pick(0.9, 0.08), Some(MovementVerb::Dodge));
    assert_eq!(pick(0.99, 0.08), Some(MovementVerb::Dodge), "approach is a tier below");
    assert_eq!(pick(0.99, 0.0), Some(MovementVerb::Shield), "no margin: the argmax");
    assert_eq!(
        pick_movement_mixed(&options, &[MovementVerb::Shield], &[], None, 0.1, 0.08),
        Some(MovementVerb::Dodge),
        "a vetoed best is not in the draw"
    );
}

/// ⛔⛔ A PRESS THAT MATURES IN HITSTUN IS BUFFERED, NOT SPENT.
///
/// Under `hitstun_locks_actions` the gate eats every attack edge for the whole
/// stun, and the emission is open-loop: it charged an APM token and armed the
/// own-attack memory for a move that never started. Measured 2026-10-29 in the
/// George mirror: a seat 364 ticks offstage pressed its authored recovery NEVER.
///
/// Three claims: nothing comes out while the body reels; the token and the
/// memory are untouched; and the SAME press comes out on the first tick the body
/// can act, which is also the non-vacuity arm.
#[test]
fn a_press_that_matures_in_hitstun_waits_for_the_first_actionable_tick() {
    let (cfg, mut state) = rig(immediate_profile());
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();

    let mut reeling = scene(300.0, 340.0);
    reeling.self_view.phase = ambition_characters::perception::BodyPhase::Hitstun;
    reeling.self_view.phase_remaining = 0.5;

    state.pending_press = Some(planted_press(0));
    // No decision runs in this test: what is under test is the EMISSION.
    state.ticks_until_decision = 1000;
    for tick in 0..6 {
        tick_fighter(&cfg, &mut state, &snapshot, Some(&reeling), &mut out);
        assert!(
            !out.melee_pressed,
            "tick {tick}: a reeling body pressed into the gate"
        );
        assert_eq!(
            state.pending_press,
            Some(planted_press(0)),
            "tick {tick}: the matured press was spent or dropped instead of held"
        );
    }
    assert_eq!(
        state.apm.presses, 0,
        "a press the body could not act on spent a token"
    );
    assert!(
        state.own_attack.is_none(),
        "the brain remembers a move it never started"
    );

    let free = scene(300.0, 340.0);
    tick_fighter(&cfg, &mut state, &snapshot, Some(&free), &mut out);
    assert!(
        out.melee_pressed,
        "the buffered press never came out when the stun ended"
    );
    assert_eq!(state.pending_press, None);
    assert_eq!(state.apm.presses, 1);
}

/// ⭐ THE BUFFER LEADS THE VIEW: the press lands on the first actionable tick,
/// not `reaction` ticks after it.
///
/// A brain with a 3-tick reaction sees its own stun end three ticks late. The
/// stun it SAW reported how much it had left, and the view says how old it is,
/// so the end is known the tick it happens. Ten ticks of stun, a press buffered
/// from tick 0: it must come out at tick 10 — before tick 13, when the view
/// first shows `Neutral`.
#[test]
fn the_buffered_press_lands_the_tick_the_stun_runs_down_not_when_the_view_says_so() {
    let (cfg, mut state) = rig(FighterBrainProfile {
        reaction_ms: 50.0,
        ..immediate_profile()
    });
    assert_eq!(
        state.perception.delay_ticks(),
        3,
        "fixture: the delay this test is built on"
    );
    let snapshot = BrainSnapshot::idle();
    let mut out = ActorControlFrame::neutral();
    // A binary-exact clock, so the comparison at the boundary is the rule and
    // not float noise.
    const DT: f32 = 1.0 / 64.0;
    const STUN_TICKS: usize = 10;

    state.pending_press = Some(planted_press(0));
    state.ticks_until_decision = 1000;
    let mut first_press = None;
    for tick in 0..(STUN_TICKS + 10) {
        let mut view = scene(300.0, 340.0);
        view.sim_time = tick as f32 * DT;
        if tick < STUN_TICKS {
            view.self_view.phase = ambition_characters::perception::BodyPhase::Hitstun;
            view.self_view.phase_remaining = (STUN_TICKS - tick) as f32 * DT;
        }
        tick_fighter(&cfg, &mut state, &snapshot, Some(&view), &mut out);
        if out.melee_pressed && first_press.is_none() {
            first_press = Some(tick);
        }
    }
    assert_eq!(
        first_press,
        Some(STUN_TICKS),
        "the press must land the tick the stun runs down (10), not when the delayed \
         view first shows it over (13), and never inside the stun"
    );
}
