//! **The acceptance test for "the fighter presses the move it scored"** — GPT
//! 5.6's 2026-07-31 review, finding 2.
//!
//! The emission half landed first and its tests observe the CONTROL FRAME. That
//! is not enough, and the review said why: the defect was never in the frame, it
//! was that nothing downstream read the direction, so a brain that scored an
//! up-tilt still produced the plain jab. A test asserting `melee_pressed` is a
//! test of the disconnected seam itself and passes in both worlds.
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

use ambition_characters::actor::ActorFaction;
use ambition_characters::actor::attack_gesture::{
    AttackGestureState, AttackGestureTuning, ResolvedAttackGesture,
};
use ambition_characters::brain::fighter::{FighterBrainProfile, FighterCfg, FighterState};
use ambition_characters::brain::{ActorControl, Brain, BrainSnapshot, StateMachineCfg};
use ambition_characters::perception::{PerceivedActor, SelfView, StageView, WorldView};
use ambition_combat::moveset::{ActorMoveset, MovePlayback};
use ambition_engine_core as ae;
use ambition_entity_catalog::{
    ClipBinding, HitVolume, MoveGates, MoveSpec, MoveWindow, MovesetContract, VolumeShape,
    WindowTag,
};
use bevy::prelude::*;
use std::collections::BTreeMap;

/// One move whose REACH is the thing under test.
///
/// `frame_data()` derives reach from the farthest body-local `+x` extent of an
/// Active volume, so the offset is how this fixture makes one move the obviously
/// better answer to a gap. Startup and duration are held equal across the two so
/// nothing but reach separates them.
fn strike(id: &str, reach_offset: f32) -> MoveSpec {
    MoveSpec {
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
                shape: VolumeShape::Rect {
                    offset: (reach_offset, 0.0),
                    half_extents: (6.0, 12.0),
                },
                damage: 5,
                knockback: 0.0,
                kb_growth: 0.0,
                launch_dir: None,
                on_hit: None,
                vfx: None,
                hit_sfx: None,
            }],
            sustain_effect: None,
            motion_scale: 1.0,
        }],
        gates: MoveGates { grounded: None },
        start_impulse: None,
        smash_charge_mult: 1.0,
    }
}

/// A body with a stubby jab and a long up-tilt.
///
/// ⚠ **the two moves must differ in REACH, not in name.** A fixture whose moves
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

/// Run the real decision until it emits an attack edge, and return that frame.
///
/// The brain deliberately does not press on the tick it decides — a committed
/// press matures over a few ticks of execution jitter, which is the behaviour
/// `PendingAttack` exists to carry. So the fixture steps until the edge appears
/// rather than assuming tick one.
fn frame_when_the_fighter_attacks(
    brain: &mut Brain,
    kit: Vec<ambition_characters::brain::fighter::options::AttackCandidate>,
    view: &WorldView,
) -> ambition_characters::actor::control::ActorControlFrame {
    let Brain::StateMachine(StateMachineCfg::Fighter { cfg, state }) = brain else {
        panic!("fixture built a fighter");
    };
    let mut snapshot = BrainSnapshot::idle();
    snapshot.attack_kit = kit;
    for _ in 0..240 {
        let mut frame = ambition_characters::actor::control::ActorControlFrame::neutral();
        ambition_characters::brain::fighter::tick_fighter(
            cfg,
            state,
            &snapshot,
            Some(view),
            &mut frame,
        );
        if frame.melee_pressed || frame.special_pressed {
            return frame;
        }
    }
    panic!("the fighter never attacked a hostile foe inside its reach in 240 ticks");
}

/// Put the emitted frame on a real body and let the production systems resolve
/// it, returning the move the body ends up playing.
fn move_played_for(frame: ambition_characters::actor::control::ActorControlFrame) -> String {
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
            ActorMoveset(jab_and_uptilt()),
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
                on_ground: true,
                ..Default::default()
            },
        ))
        .id();
    app.update();
    app.world()
        .get::<MovePlayback>(body)
        .map(|playback| playback.spec.id.clone())
        .expect("a melee edge on a body with a moveset starts a move")
}

/// **The kit is what the body can press, and every entry can be pressed.**
///
/// The guard on the row above it: if the kit ever went back to listing
/// `moveset.moves`, a move no input reaches could be scored and the acceptance
/// test below would fail somewhere much less legible.
#[test]
fn every_candidate_in_the_kit_carries_the_press_that_invokes_it() {
    let moveset = ActorMoveset(jab_and_uptilt());
    let brain = fighter_brain();
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain));

    let ids: Vec<&str> = kit.iter().map(|c| c.move_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["jab", "uptilt"],
        "both authored moves are reachable by some press, each listed once"
    );

    use ambition_characters::actor::attack_gesture::AttackDir;
    use ambition_characters::brain::fighter::options::AttackVerb;
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

/// **The move the body plays is the move the brain scored.**
///
/// ⚠ this asserts `MovePlayback.spec.id` and NOT `melee_pressed`. The latter was
/// true in the broken world too — it is the seam that was disconnected, so a
/// test that observes it cannot see the bug it exists to catch.
///
/// PROBED: with `press_the_chosen_attack`'s axis forced back to `Vec2::ZERO` —
/// the pre-fix emission, a neutral press — this reports
/// `assertion failed: "jab" == "uptilt"`. The fixture fails for the original
/// reason, not merely for a reason.
#[test]
fn the_fighter_plays_the_move_it_scored_not_the_neutral_one() {
    let moveset = ActorMoveset(jab_and_uptilt());
    let mut brain = fighter_brain();
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain));

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
    let kit = attack_kit_of(Some(&moveset), true, Some(&brain));

    let view = scene(16.0);
    let frame = frame_when_the_fighter_attacks(&mut brain, kit, &view);

    assert_eq!(
        move_played_for(frame),
        "jab",
        "at jab range the scored move is the jab, and the direction must not be invented"
    );
}
