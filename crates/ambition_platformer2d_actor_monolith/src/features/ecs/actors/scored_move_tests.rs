//! A test asserting `melee_pressed` is a test of the disconnected seam itself and passes in both
//! worlds.
//!
//! So this fixture asserts `MovePlayback.spec.id` — the move the body is
//! ACTUALLY playing — and it gets there through the production chain:
//!
//! ```text
//! attack_kit_of          the real kit builder: enumerate presses, ask the
//!                        moveset what each one reaches
//!   → tick_fighter       the real decision: score, refine, commit a binding
//!   → ActorControlFrame  the real emission: verb → button, direction → axis
//!   → resolve_attack_gestures   the real interpreter
//!   → trigger_moveset_moves     the real resolution
//!   → MovePlayback              what the body is swinging
//! ```
//!
//! Only the hop from the brain's output frame onto the body is written by hand;
//! in production that is a component write, and a fixture that stood up the
//! whole actor tick to perform it would be testing the scheduler.

use super::update::attack_kit_of;

use ambition_characters::actor::attack_gesture::{
    AttackGestureState, AttackGestureTuning, ResolvedAttackGesture,
};
use ambition_characters::actor::ActorFaction;
use ambition_characters::brain::fighter::{FighterBrainProfile, FighterCfg, FighterState};
use ambition_characters::brain::{Brain, BrainSnapshot, StateMachineCfg};
use ambition_characters::control::ActorControl;
use ambition_characters::perception::{PerceivedActor, SelfView, StageView, WorldView};
use ambition_combat::moveset::{ActorMoveset, MovePlayback};
use ambition_entity_catalog::{
    ClipBinding, HitVolume, MoveGates, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
    WindowTag,
};
use ambition_platformer2d_core as ae;
use bevy::prelude::*;
use std::collections::BTreeMap;

/// One move whose REACH is the thing under test.
///
/// `frame_data()` derives reach from the farthest body-local `+x` extent of an
/// Active volume, so the offset is how this fixture makes one move the obviously
/// better answer to a gap. Startup and duration are held equal across the two so
/// nothing but reach separates them.
fn strike(id: &str, reach_offset: f32) -> MoveSpec {
    strike_hitting_for(id, reach_offset, 5)
}

/// [`strike`], with the damage stated — for a fixture that needs an attack
/// somebody would actually rather land than a grab.
fn strike_hitting_for(id: &str, reach_offset: f32, damage: i32) -> MoveSpec {
    MoveSpec {
        display_name: None,
        landing_lag_s: None,
        autocancel_after_s: None,
        sprite_spin_hz: None,
        equips: None,
        id: id.to_string(),
        clip: ClipBinding {
            clip: id.to_string(),
            fallbacks: vec![],
        },
        duration_s: 0.4,
        events: vec![],
        windows: vec![MoveWindow {
            start_s: 0.1,
            end_s: 0.2,
            tag: WindowTag::Active,
            volumes: vec![HitVolume {
                // An ordinary hit, not a gust.
                shape: VolumeShape::Rect {
                    offset: (reach_offset, 0.0),
                    half_extents: (6.0, 12.0),
                },
                damage,
                knockback: 0.0,
                knockback_growth: None,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
                reaction: None,
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates::default(),
        start_impulse: None,
        smash_charge_mult: 1.0,
        smash_charge: None,
        charge_gesture: ambition_entity_catalog::ChargeGesture::default(),
        repeat: None,
    }
}

/// A body with a stubby jab and a long up-tilt.
///
/// the two moves must differ in REACH, not in name. A fixture whose moves
/// were interchangeable would pass with the direction still discarded, because
/// whichever move the neutral press resolved to would be as good an answer as
/// the scored one.
fn jab_and_uptilt() -> MovesetContract {
    MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            ("attack_up".to_string(), "uptilt".to_string()),
        ]),
        moves: vec![strike("jab", 10.0), strike("uptilt", 78.0)],
    }
}

fn fighter_brain() -> Brain {
    let cfg = FighterCfg::new(FighterBrainProfile::for_level(8));
    let state = FighterState::new(&cfg, 0x5F37_7A11);
    Brain::StateMachine(StateMachineCfg::Fighter {
        cfg: Box::new(cfg),
        state: Box::new(state),
    })
}

/// A hostile foe at `gap` in front, both grounded on a wide stage.
fn scene(gap: f32) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos: ae::Vec2::new(0.0, 300.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            faction: ActorFaction::Player,
            alive: true,
            on_ground: true,
            ..Default::default()
        },
        stage: StageView {
            bounds: ae::Aabb::new(ae::Vec2::new(0.0, 300.0), ae::Vec2::new(600.0, 300.0)),
        },
        actors: vec![PerceivedActor {
            id: "foe".to_string(),
            pos: ae::Vec2::new(gap, 300.0),
            faction: ActorFaction::Enemy,
            hostile_to_self: true,
            alive: true,
            on_ground: true,
            ..Default::default()
        }],
        ..Default::default()
    }
}

/// A body with a stubby jab and an AUTHORED GRAB — a real one, built through the
/// same helpers a character's own file uses, so the kit reads the same shape a
/// fighter really publishes.
///
/// the jab out-damages the throw (14 against 11), which is the genre's own arrangement and the
/// only one that asks a real question.
fn jab_and_grab() -> MovesetContract {
    use ambition_characters::smash_capture::{
        author_standing_grab, author_throw, capture_beat, grab_shell, CaptureAttemptParams,
        CaptureThrowParams,
    };
    let grab = author_standing_grab(
        // George's real numbers. A grab is SLOW and its whiff is long; a
        // fixture that gave it a jab's startup would be asking whether the
        // scorer prefers a strictly better move, which is not a question.
        grab_shell("grab", "grab", 0.16, 0.06, 0.30),
        CaptureAttemptParams {
            offset: (20.0, 0.0),
            half_extents: (24.0, 13.0),
            hold_offset: (20.0, -2.0),
        },
    );
    let throw = author_throw(
        capture_beat("fthrow", "throw", 0.30),
        0.18,
        CaptureThrowParams {
            damage: 11,
            knockback: 130.0,
            knockback_growth: 2.0,
            launch_dir: (1.0, -0.35),
        },
    );
    MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            (
                ambition_entity_catalog::GRAB_VERB.to_string(),
                "grab".to_string(),
            ),
            (
                ambition_entity_catalog::CAPTURE_THROW_FORWARD_VERB.to_string(),
                "fthrow".to_string(),
            ),
        ]),
        moves: vec![strike_hitting_for("jab", 22.0, 14), grab, throw],
    }
}

/// [`scene`], with the foe's guard up or down.
fn scene_guarding(gap: f32, guarding: bool) -> WorldView {
    let mut view = scene(gap);
    view.actors[0].shield_raised = guarding;
    view
}

/// Run the real decision until it emits an attack edge, and return that frame.
///
/// The brain deliberately does not press on the tick it decides — a committed
/// press matures over a few ticks of execution jitter, which is the behaviour
/// `PendingAttack` exists to carry. So the fixture steps until the edge appears
/// rather than assuming tick one.
fn frame_when_the_fighter_attacks(
    brain: &mut Brain,
    kit: Vec<ambition_characters::brain::attack_kit::AttackCandidate>,
    view: &WorldView,
) -> ambition_characters::actor::control::ActorControlFrame {
    let Brain::StateMachine(StateMachineCfg::Fighter { cfg, state }) = brain else {
        panic!("fixture built a fighter");
    };
    let mut snapshot = BrainSnapshot::idle();
    snapshot.attack_kit = kit;
    for _ in 0..240 {
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        ambition_combat::brain::fighter::tick_fighter(
            cfg,
            state,
            &snapshot,
            Some(view),
            &mut frame,
        );
        if frame.melee_pressed || frame.special_pressed || frame.grab_pressed {
            return frame;
        }
    }
    panic!("the fighter never acted on a hostile foe inside its reach in 240 ticks");
}

/// Put the emitted frame on a real body and let the production systems resolve
/// it, returning the move the body ends up playing.
fn move_played_for(frame: ambition_characters::actor::control::ActorControlFrame) -> String {
    move_played_for_moveset(frame, jab_and_uptilt())
}

/// [`move_played_for`], for a body carrying `moveset`.
fn move_played_for_moveset(
    frame: ambition_characters::actor::control::ActorControlFrame,
    moveset: MovesetContract,
) -> String {
    let mut app = App::new();
    app.add_systems(
        Update,
        (
            ambition_combat::moveset::resolve_attack_gestures,
            ambition_combat::moveset::trigger_moveset_moves,
        )
            .chain(),
    );
    let body = app
        .world_mut()
        .spawn((
            ActorControl(frame),
            ActorMoveset(moveset),
            AttackGestureState::default(),
            AttackGestureTuning::default(),
            ResolvedAttackGesture::default(),
            ae::BodyKinematics {
                pos: ae::Vec2::ZERO,
                vel: ae::Vec2::ZERO,
                size: ae::Vec2::new(24.0, 32.0),
                facing: 1.0,
            },
            ae::BodyGroundState {
                head_contact: false,
                on_ground: true,
                ..Default::default()
            },
        ))
        .id();
    app.update();
    app.world()
        .get::<MovePlayback>(body)
        .map(|playback| playback.spec.id.clone())
        .expect("an attack or grab edge on a body with a moveset starts a move")
}

/// The kit is what the body can press, and every entry can be pressed.
///
/// The guard on the row above it: if the kit ever went back to listing
/// `moveset.moves`, a move no input reaches could be scored and the acceptance
/// test below would fail somewhere much less legible.
#[test]
fn every_candidate_in_the_kit_carries_the_press_that_invokes_it() {
    let moveset = ActorMoveset(jab_and_uptilt());
    let brain = fighter_brain();
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain), None);

    let ids: Vec<&str> = kit.iter().map(|c| c.move_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["jab", "uptilt"],
        "both authored moves are reachable by some press, each listed once"
    );

    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::attack_kit::AttackVerb;
    let uptilt = kit
        .iter()
        .find(|c| c.move_id == "uptilt")
        .expect("the up press reaches the up-tilt");
    assert_eq!(uptilt.binding.verb, AttackVerb::Basic);
    assert_eq!(
        uptilt.binding.direction,
        AttackDir::Up,
        "the binding is the press that reached this move, not a default"
    );
    assert!(
        uptilt.frames.reach > kit[0].frames.reach,
        "the fixture's premise: the up-tilt out-reaches the jab ({} vs {})",
        uptilt.frames.reach,
        kit[0].frames.reach
    );
}

/// The move the body plays is the move the brain scored.
///
/// this asserts `MovePlayback.spec.id` and NOT `melee_pressed`.
///
/// PROBED: with `press_the_chosen_attack`'s axis forced back to `Vec2::ZERO` —
/// the pre-fix emission, a neutral press — this reports
/// `assertion failed: "jab" == "uptilt"`. The fixture fails for the original
/// reason, not merely for a reason.
#[test]
fn the_fighter_plays_the_move_it_scored_not_the_neutral_one() {
    let moveset = ActorMoveset(jab_and_uptilt());
    let mut brain = fighter_brain();
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain), None);

    // A gap only the up-tilt's reach fits: the jab (reach 16) falls far short,
    // so the scoring has one clear answer and the test is not measuring a tie.
    let view = scene(70.0);
    let frame = frame_when_the_fighter_attacks(&mut brain, kit, &view);

    assert_eq!(
        move_played_for(frame),
        "uptilt",
        "the reach/frame-advantage work chose the up-tilt; the body must swing it"
    );
}

/// The other half of the same claim, and the one that keeps the test above
/// honest: when the SHORT move is the right answer, the body plays that one.
///
/// Without this, an emission that hard-coded `AttackDir::Up` would pass.
#[test]
fn a_close_foe_gets_the_jab_the_scoring_actually_picked() {
    let moveset = ActorMoveset(jab_and_uptilt());
    let mut brain = fighter_brain();
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain), None);

    let view = scene(16.0);
    let frame = frame_when_the_fighter_attacks(&mut brain, kit, &view);

    assert_eq!(
        move_played_for(frame),
        "jab",
        "at jab range the scored move is the jab, and the direction must not be invented"
    );
}

/// THE GRAB ENTERS THE KIT FROM ITS OWN AUTHORED DATA.
///
/// A CPU could not choose a grab at all: the kit enumerated the three attack
/// buttons, and a grab answers its own. Everything the scorer needs about one
/// has to come from the authored capture itself, because `frame_data` derives
/// reach, coverage and power from HIT VOLUMES and a grab lands none.
///
/// no character id, no role, no hand-written distance. A CPU that knew
/// "George grabs at 44px" would stop working the day George is retuned, and a
/// second fighter would need a second constant.
#[test]
fn the_kit_prices_a_grab_from_the_capture_its_own_move_authors() {
    let moveset = ActorMoveset(jab_and_grab());
    let kit = attack_kit_of(Some(&moveset), true, Some(&fighter_brain()), None);
    let grab = kit
        .iter()
        .find(|candidate| candidate.move_id == "grab")
        .expect("the kit offers no grab, so no CPU could ever choose one");
    let coverage = grab
        .frames
        .coverage
        .expect("a grab with no coverage cannot be scored against a distance");
    assert_eq!(
        (coverage.min, coverage.max),
        ((-4.0, -13.0), (44.0, 13.0)),
        "the grab's reach is not the rect its own capture attempt sustains"
    );
    assert_eq!(
        grab.frames.max_damage, 0,
        "a grab was priced as if it DEALT damage. It deals none; what it is \
         worth is that the opponent is held, and the generic scorer has no term \
         for that — see the note on `capture_candidate`, and the match this cost"
    );
    assert!(
        grab.frames.ignores_guard,
        "the planner still thinks a shield stops a grab"
    );
    assert_eq!(
        grab.binding.verb,
        ambition_characters::brain::attack_kit::AttackVerb::Grab,
        "the grab is bound to some other button"
    );
}

/// A GUARD-IGNORING MOVE BEATS A BLOCKABLE ONE AGAINST A RAISED SHIELD.
///
/// The generic half of the triangle, at the layer that owns it: the ROLLOUT is
/// where a shield zeroes a swing, so it is where `ignores_guard` has to be
/// visible. Two candidates identical in every other respect, and the fighter's
/// READ of its opponent as the only thing that changes.
///
/// What is missing is not a number: it is that "how valuable is holding somebody" is
/// platform-fighter policy, and this scorer is shared by every actor in every game the engine runs.
#[test]
fn a_guard_ignoring_move_is_what_answers_a_raised_shield() {
    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::attack_kit::{
        ActionLegality, AttackBinding, AttackCandidate, AttackVerb,
    };
    use ambition_characters::brain::fighter::data::ShadowTuning;
    use ambition_characters::brain::fighter::habit::{Choice, HabitModel};
    use ambition_characters::brain::fighter::options::generate_options;
    use ambition_characters::brain::fighter::situation::classify;
    use ambition_characters::brain::fighter::FighterBrainProfile;
    use ambition_combat::brain::fighter::rollout::refine_by_rollout;

    // Close enough that both reach. At range the shadow's whiff decides instead,
    // which is a different question with the same answer shape — measured, and
    // the reason this gap is stated rather than picked.
    const GAP: f32 = 20.0;
    let blockable = strike_hitting_for("blockable", 22.0, 10);
    let mut unblockable_frames = strike_hitting_for("unblockable", 22.0, 10).frame_data();
    unblockable_frames.ignores_guard = true;
    let kit = vec![
        AttackCandidate {
            move_id: "blockable".to_string(),
            frames: blockable.frame_data(),
            binding: AttackBinding {
                verb: AttackVerb::Basic,
                direction: AttackDir::Neutral,
            },
            legality: ActionLegality::Now,
        },
        AttackCandidate {
            move_id: "unblockable".to_string(),
            frames: unblockable_frames,
            binding: AttackBinding {
                verb: AttackVerb::Grab,
                direction: AttackDir::Neutral,
            },
            legality: ActionLegality::Now,
        },
    ];
    let profile = FighterBrainProfile::for_level(8);

    for (read, expected) in [
        (Choice::Shield, "unblockable"),
        (Choice::Attack, "blockable"),
    ] {
        let mut delayed = ambition_characters::perception::DelayedPerception::default();
        delayed.observe(scene_guarding(GAP, read == Choice::Shield));
        let perceived = delayed.perceive().expect("the fixture published a view");
        let situation = classify(perceived);
        let options = generate_options(perceived, situation, &kit, &profile.utility_weights);
        // What this fighter has learned its opponent does here. The shadow's foe
        // acts on it, which is the only way a raised guard exists to answer.
        let mut habits = HabitModel::default();
        for _ in 0..20 {
            habits.observe(situation, read);
        }
        let refined = refine_by_rollout(
            perceived,
            situation,
            &options,
            &habits,
            &profile,
            &ShadowTuning::default(),
            60.0,
            6,
            None,
        )
        .expect("a level-8 fighter rolls out");
        assert_eq!(
            refined.move_id.as_deref(),
            Some(expected),
            "reading {read:?} from its opponent, the fighter chose {:?}",
            refined.move_id
        );
    }
}

#[test]
fn a_grab_edge_plays_the_authored_grab() {
    let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
    frame.grab_pressed = true;
    assert_eq!(
        move_played_for_moveset(frame, jab_and_grab()),
        "grab",
        "a Grab press reached a body that authors one and it swung something else"
    );
}
