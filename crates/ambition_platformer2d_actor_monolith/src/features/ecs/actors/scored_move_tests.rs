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
        flow: None,
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
    use ambition_entity_catalog::smash_capture::{
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

fn jab_uptilt_and_dash() -> MovesetContract {
    let mut set = jab_and_uptilt();
    // The genre's dash attack: `{base}_dash`, exactly the spelling
    // `dash_stance_verb` builds and `move_for_attack` looks for.
    set.verbs
        .insert("attack_dash".to_string(), "dash_attack".to_string());
    set.moves.push(strike("dash_attack", 40.0));
    set
}

/// ⛔⛔ **A RUNNING BODY'S KIT OFFERS THE MOVE ITS PRESS ACTUALLY PRODUCES.**
///
/// The kit is resolved with `move_for_attack(base, dir, grounded, RUNNING)`
/// because that is what `trigger_moveset_moves` calls. For as long as the kit
/// existed it called `move_for_directional_verb` instead — that same function
/// with the running branch skipped — while a comment beside it asserted the two
/// were the same function.
///
/// ⇒ MEASURED 2026-09-10 across the shipped roster: eighteen of eighteen
/// fighters author a dash attack and not one was reachable from the kit. ⚠ And
/// the absence was the SMALLER half. While the body ran, the brain scored
/// `jab`'s frame data, issued the attack press, and the press road performed
/// `dash_attack` — a candidate whose `move_id` and `frames` described a
/// different move than the one it produced. Every scoring term downstream
/// (startup, reach, damage, frame advantage) was reading the wrong move.
///
/// ⭐ **MANDATORY SINCE 2026-09-12.** It spent two days `#[ignore]`d behind
/// `--features truthful_attack_kit` while the question "should this land, given
/// it re-prices CPU matchups" was open. It is not that question: a planner that
/// scores move A while the executor performs move B violates the action model
/// whatever the matchups say, and re-pricing is the EXPECTED consequence of a
/// decision model that starts reading the frame data of the move it actually
/// takes. `Q117`, in `docs/planning/maintainer-decisions.md` — which also
/// records the half it did NOT settle: what utility / run / dash-attack TUNING
/// a brain that reads the right frame data now wants is Jon's, not this arm's.
#[test]
fn a_running_body_is_offered_the_dash_attack_its_press_would_actually_produce() {
    let moveset = ActorMoveset(jab_uptilt_and_dash());
    let brain = fighter_brain();

    let standing = attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, None);
    let standing_ids: Vec<&str> = standing.iter().map(|c| c.move_id.as_str()).collect();
    assert_eq!(
        standing_ids,
        vec!["jab", "uptilt"],
        "a body standing still is offered exactly what it was before — the dash \
         attack is not a new option everywhere, it is the answer to one stance"
    );

    let dashing = attack_kit_of(Some(&moveset), true, true, Some(&brain), None, None, None);
    let dashing_ids: Vec<&str> = dashing.iter().map(|c| c.move_id.as_str()).collect();
    assert!(
        dashing_ids.contains(&"dash_attack"),
        "a running body's attack press produces `dash_attack` and its kit does \
         not offer it, so the brain can never choose the move — and, worse, \
         scores whatever it thinks the press reaches instead: {dashing_ids:?}"
    );
    assert!(
        !dashing_ids.contains(&"jab"),
        "the kit offers `jab` to a RUNNING body, but that press resolves to \
         `dash_attack` — the candidate's id and frame data describe a move the \
         press will not produce, which is the mislabel this guard exists for: \
         {dashing_ids:?}"
    );

    // ⭐ AND THE FRAME DATA TRAVELS WITH THE ID, since the scoring reads that and
    // not the name. A candidate carrying the right id and the wrong frames would
    // satisfy every assertion above and still misprice the choice.
    let dash = dashing
        .iter()
        .find(|c| c.move_id == "dash_attack")
        .expect("checked above");
    assert_eq!(
        dash.frames.max_damage,
        strike("dash_attack", 40.0).frame_data().max_damage,
        "the candidate carries another move's frame data"
    );
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
    let kit = attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, None);

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
    let kit = attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, None);

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
    let kit = attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, None);

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
    let kit = attack_kit_of(Some(&moveset), true, false, Some(&fighter_brain()), None, None, None);
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
            wear: ambition_characters::brain::attack_kit::MoveWear::FRESH,
        },
        AttackCandidate {
            move_id: "unblockable".to_string(),
            frames: unblockable_frames,
            binding: AttackBinding {
                verb: AttackVerb::Grab,
                direction: AttackDir::Neutral,
            },
            legality: ActionLegality::Now,
            wear: ambition_characters::brain::attack_kit::MoveWear::FRESH,
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

/// ⭐⭐ **CAN THE BRAIN SEE A CANCEL THE TRIGGER WOULD ACCEPT?**
///
/// ⛔⛤ **IT COULD NOT, AND `legality_of`'s OWN DOC ASKED FOR THE THING THE CODE
/// DID NOT DO.** It said *"the name list must match `trigger_moveset_moves`
/// exactly: the verb the press resolves through, plus the resolved move id. That
/// is the one cancel namespace, and asking with a different list would make this
/// answer a question nothing enforces."* It then passed `[verb, move_id]` —
/// while the trigger passes `cancel_names_for(base, running)` **plus** the move
/// id, which for an attack is `["attack", "any_attack", <id>]`.
///
/// ⇒ A window authored `into: ["any_attack"]` was INVISIBLE to the brain. That
/// is the CLASS every shipped cancel uses, because an author writes "cancel into
/// an attack" rather than naming twenty-six move ids — the Performer's three
/// tilts author exactly that, `OnHit`, over their recovery. So the window
/// permitted, the trigger would have accepted, and the CPU was told
/// `BlockedByPlayback` and stood through the recovery.
///
/// ⚠ THIS IS A KIT-LEVEL TEST, NOT A MATCH. What it pins is that the two seams
/// answer the SAME question; whether a given match then takes the cancel is the
/// scorer's business and a separate measurement.
#[test]
fn the_brain_can_see_an_any_attack_cancel_the_trigger_would_accept() {
    let mut jab = strike("jab", 16.0);
    // The recovery window, authored the way the shipped tilts author theirs: by
    // CLASS, on a connected hit.
    jab.windows.push(MoveWindow {
        start_s: 0.2,
        end_s: 0.4,
        tag: WindowTag::Cancelable {
            into: vec!["any_attack".to_string()],
            condition: ambition_entity_catalog::CancelCondition::OnHit,
        },
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    });
    let moveset = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            ("attack_up".to_string(), "uptilt".to_string()),
        ]),
        moves: vec![jab.clone(), strike("uptilt", 40.0)],
    });

    // Mid-recovery, with the strike CONNECTED — which is what `OnHit` asks.
    let mut playback = MovePlayback::new_at(jab, 1.0, 0.3);
    playback.connected_hit = true;
    assert!(
        playback
            .spec
            .cancel_permits(playback.t, playback.contact(), &["any_attack"]),
        "the fixture's own window does not permit, so the assertion below would \
         be about the fixture rather than about the brain"
    );

    let kit = attack_kit_of(
        Some(&moveset),
        true,
        false,
        Some(&fighter_brain()),
        Some(&playback),
        None,
        None,
    );
    let blocked: Vec<&str> = kit
        .iter()
        .filter(|c| {
            c.legality == ambition_characters::brain::attack_kit::ActionLegality::BlockedByPlayback
        })
        .map(|c| c.move_id.as_str())
        .collect();
    assert!(
        blocked.is_empty(),
        "the window permits `any_attack` and the trigger would accept it, but the \
         brain was told these are blocked: {blocked:?}"
    );
}

/// ⛔ AND THE CONTROL, because "nothing is blocked" is also what a
/// `legality_of` that always answered `Now` would say. OUTSIDE the cancel
/// window, every candidate is blocked.
#[test]
fn outside_the_cancel_window_the_brain_is_told_the_body_is_busy() {
    let mut jab = strike("jab", 16.0);
    jab.windows.push(MoveWindow {
        start_s: 0.2,
        end_s: 0.4,
        tag: WindowTag::Cancelable {
            into: vec!["any_attack".to_string()],
            condition: ambition_entity_catalog::CancelCondition::OnHit,
        },
        volumes: vec![],
        sustain_effect: None,
        motion_scale: 1.0,
    });
    let moveset = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            ("attack_up".to_string(), "uptilt".to_string()),
        ]),
        moves: vec![jab.clone(), strike("uptilt", 40.0)],
    });

    // t = 0.15: inside the Active window, BEFORE the cancel window opens.
    let mut playback = MovePlayback::new_at(jab, 1.0, 0.15);
    playback.connected_hit = true;

    let kit = attack_kit_of(
        Some(&moveset),
        true,
        false,
        Some(&fighter_brain()),
        Some(&playback),
        None,
        None,
    );
    assert!(!kit.is_empty(), "an empty kit would make this arm vacuous");
    assert!(
        kit.iter().all(|c| c.legality
            == ambition_characters::brain::attack_kit::ActionLegality::BlockedByPlayback),
        "a body mid-swing, before its cancel window opens, was told it could start \
         something"
    );
}

/// ⛔⛤ **A RANGED MOVE WAS ADMITTED AT A THOUSAND PIXELS BECAUSE NOBODY HAD
/// JOINED IT TO THE WEAPON IT FIRES.**
///
/// `MoveEventKind::Ranged` pulls whatever `RangedActionSpec` the BODY carries,
/// so `MoveSpec::frame_data()` — which has no body — answered
/// `RANGED_ACTION_REACH`, a constant wider than any stage this game ships. Its
/// own doc named the cost: *"a CPU that fires from further away than its shot
/// can carry"*, and the review of 2026-09-20 named the shape: *"the new
/// `RANGED_ACTION_REACH = 1000` placeholder is evidence of a deeper split in
/// authority."*
///
/// ⭐ THIS IS THE LAYER THAT CAN ANSWER IT, and it is the same layer that
/// already joins a grab to its capture params. The catalog states a REQUEST
/// (`MoveHazard::OwnersRangedAction`) and the kit builder resolves it against
/// the weapon.
///
/// ⛔⛤ **AND A BODY WITH NO WEAPON MAKES NO OFFER — THIS ARM SAID THE
/// OPPOSITE UNTIL 2026-09-20.** It asserted that an unresolvable request is
/// LEFT STANDING, on the reasoning that *"the move fires nothing"* belongs to
/// whoever decides whether to press it. But the request's unjoined `reach()`
/// is `RANGED_ACTION_REACH` — 1000px, wider than any stage this game ships —
/// so *this* layer, having just proven the press fires nothing, was handing
/// the brain a stage-crossing instantaneous threat. Deciding whether to press
/// it IS what the kit is for. An unanswerable request is no hazard.
#[test]
fn a_ranged_move_is_joined_to_the_weapon_the_body_actually_fires() {
    use ambition_characters::brain::action_set::{ProjectileFlight, RangedActionSpec};
    use ambition_entity_catalog::{MoveEvent, MoveEventKind, MoveHazard};

    let mut shot = strike("cannon", 20.0);
    // The body's own trigger, and no Active volume: the projectile IS the
    // damage, which is the shape of every ranged move in the game.
    shot.windows.clear();
    shot.events.push(MoveEvent {
        at_s: 0.18,
        kind: MoveEventKind::Ranged,
    });
    let moveset = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), "cannon".to_string())]),
        moves: vec![shot],
    });
    let brain = fighter_brain();
    let hazard_of = |kit: &[ambition_characters::brain::attack_kit::AttackCandidate]| {
        kit.iter()
            .find(|c| c.move_id == "cannon")
            .expect("the cannon is the body's one move")
            .frames
            .hazard
    };
    let fired = |kit: &[ambition_characters::brain::attack_kit::AttackCandidate]| {
        hazard_of(kit).expect("a move that fires puts a hazard in the world")
    };

    // 540px/s down a 2.4s straight flight — Projectile Polygon's own cannon,
    // cited rather than imported: this crate is below the content that authors
    // her, and what the arm needs is a weapon SHAPED like a shipped one.
    let cannon = RangedActionSpec::bolt(540.0, 4).with_flight(ProjectileFlight::STRAIGHT);
    let joined = fired(&attack_kit_of(
        Some(&moveset),
        true,
        false,
        Some(&brain),
        None,
        Some(&cannon),
        None,
    ));
    // 540px/s is constant, and the shot's own 10px half-extent is ground it
    // does not have to fly: 400px away is `(400 - 10) / 540`.
    assert!(
        joined
            .travel_to(400.0)
            .is_some_and(|t| (t - 390.0 / 540.0).abs() < 1.0e-3),
        "the shot's flight law did not survive the join ({:?}), so a brain \
         leading its aim still has to pretend the bolt arrives where it was \
         thrown",
        joined.travel_to(400.0)
    );
    // `speed × lifetime`, plus the shot's own half-extent: 540 × 2.4 + 10.
    assert!(
        (joined.reach() - 1306.0).abs() < 1.0,
        "her cannon crosses 1306px and the kit says {}px",
        joined.reach()
    );
    assert_ne!(
        joined.reach(),
        ambition_entity_catalog::RANGED_ACTION_REACH,
        "the placeholder survived the join, which is the whole defect"
    );

    // ⭐ A SHORTER WEAPON IS A SHORTER OFFER — without this the arm above
    // passes for a join that reads any weapon at all and answers one number.
    let pistol = RangedActionSpec::bolt(200.0, 1)
        .with_flight(ProjectileFlight::STRAIGHT.with_lifetime(0.5));
    let short = fired(&attack_kit_of(
        Some(&moveset),
        true,
        false,
        Some(&brain),
        None,
        Some(&pistol),
        None,
    ));
    assert!(
        short.reach() < joined.reach(),
        "a 200px/s shot with half a second of life reaches {}px and a 540px/s \
         shot with 2.4s reaches {}px — the join is not reading the weapon",
        short.reach(),
        joined.reach()
    );

    // ⛔⛤ AND A BOOMERANG IS NOT `speed × out_s`, WHICH IS THE ARITHMETIC THE
    // FIRST VERSION OF THIS JOIN USED. `ProjectileFlight::boomerang` is a
    // constant deceleration to a stop, so the shot's displacement at the
    // turnaround is `v0 · out_s / 2` — HALF the straight-line product, and its
    // own doc states the formula. The ponytail's real numbers: 430px/s over
    // 0.34s reaches 73px plus the shot's 10px body, not 146.
    let ponytail =
        RangedActionSpec::bolt(430.0, 7).with_flight(ProjectileFlight::boomerang(0.34));
    let thrown = fired(&attack_kit_of(
        Some(&moveset),
        true,
        false,
        Some(&brain),
        None,
        Some(&ponytail),
        None,
    ));
    assert!(
        (thrown.reach() - 83.1).abs() < 0.5,
        "the ponytail reaches {}px; `430 × 0.34` is 146 and `430 × 0.34 / 2` \
         is 73, so a reading near 156 means the deceleration was dropped",
        thrown.reach()
    );
    // ⛔⛤ **AND ITS ARRIVAL TIME IS SOLVED, NOT AVERAGED — THE SECOND REVIEW
    // FINDING ON THIS JOIN.** The first repair published the average speed
    // over the out-leg, so that `reach / speed` came out right. That is right
    // at the TURNAROUND and nowhere else: the shot is decelerating, so it
    // covers its first pixels fast and its last pixels barely at all. The
    // closed form is `t = out_s - sqrt(out_s^2 - 2 out_s d / v0)` for `d` of
    // CENTRE travel, and at 40px of centre travel (50px of gap, less the
    // shot's 10px body) it is 0.111s where the average said 0.186s — 15px of
    // excess lead at a 200px/s closing speed, which is `ADMISSION_SLACK_PX`.
    assert!(
        thrown
            .travel_to(50.0)
            .is_some_and(|t| (t - 0.1112).abs() < 1.0e-3),
        "the ponytail covers 40px of centre travel in 0.111s and the kit says \
         {:?}; 0.186s is the average-speed model and 0.093s is the launch \
         speed",
        thrown.travel_to(50.0)
    );
    // ⭐ AND THE TURNAROUND IS WHERE THE TWO MODELS AGREE, which is the control:
    // an arm that only checked the far end would pass for the average.
    assert!(
        thrown
            .travel_to(thrown.reach())
            .is_some_and(|t| (t - 0.34).abs() < 1.0e-3),
        "the ponytail reaches its turnaround at 0.34s and the kit says {:?}",
        thrown.travel_to(thrown.reach())
    );
    assert_eq!(
        thrown.travel_to(thrown.reach() + 1.0),
        None,
        "a boomerang answered a distance past its own turnaround"
    );

    // ⛔⛤ AND NO WEAPON MEANS NO HAZARD — not the request left standing, whose
    // `reach()` is the 1000px placeholder.
    let unarmed = attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, None);
    assert_eq!(
        hazard_of(&unarmed),
        None,
        "a body that carries no ranged action kept its move's request, and an \
         unresolved request reads as a {}px instantaneous threat",
        ambition_entity_catalog::RANGED_ACTION_REACH
    );
    // ⚠ THE PREMISE: the move is still in the kit. If the join had dropped the
    // CANDIDATE the assertion above would pass for the wrong reason, and a
    // body would silently lose a move because of what it is not carrying.
    assert!(
        unarmed.iter().any(|c| c.move_id == "cannon"),
        "the unarmed body lost the move itself, not just its offer"
    );
    let _ = MoveHazard::OwnersRangedAction;
}

/// ⛔⛤ **A RANGED MOVE WAS SCORED AT ZERO POWER, AND THAT COST THE ROSTER'S
/// PROJECTILE FIGHTER HER BEST SHOT.**
///
/// `MoveFrameData::max_damage` folds ACTIVE VOLUMES, and a launcher authors
/// none — the projectile IS the damage. So the option scorer's
/// `expected_payoff`, which is a move's power over the kit's strongest, read
/// `0` for every ranged move in the game. Nothing was wrong with the
/// arithmetic; the number it spent was about a volume that does not exist.
///
/// ⭐ It is the SAME JOIN as the reach above, one field further: the numbers
/// are on the `RangedActionSpec` the kit builder already had in its hand.
///
/// ⚠ **THE ORDERING IS THE POINT, NOT THE MAGNITUDE.** Projectile Polygon's
/// ponytail deals `7` and her cannon `4`. With both reading `0` the only
/// things separating them were reach and frame advantage — and once the
/// boomerang's true 83px replaced the 1000px placeholder she stopped throwing
/// it at range and fell through to the weaker shot, which is the loss the
/// grid sweep recorded (`144% / 228%`, ponytail ×25 → charge shot ×35) and
/// the loss this closes.
#[test]
fn the_shot_a_ranged_move_fires_is_what_that_move_is_worth() {
    use ambition_characters::brain::action_set::{ProjectileFlight, RangedActionSpec};
    use ambition_entity_catalog::{MoveEvent, MoveEventKind};

    // ⚠ THE VOLUME IS KEPT, and it is the control for the `max` below: a
    // launcher that also swings must report the LARGER of the two, never the
    // sum and never whichever road was consulted last. `2` is under both
    // shipped shots, so a reading of 2 means the hazard was ignored.
    let mut shot = strike_hitting_for("cannon", 20.0, 2);
    shot.events.push(MoveEvent {
        at_s: 0.18,
        kind: MoveEventKind::Ranged,
    });
    let moveset = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), "cannon".to_string())]),
        moves: vec![shot],
    });
    let brain = fighter_brain();
    let frames_with = |ranged: Option<&RangedActionSpec>| {
        attack_kit_of(Some(&moveset), true, false, Some(&brain), None, ranged, None)
            .into_iter()
            .find(|c| c.move_id == "cannon")
            .expect("the cannon is the body's one move")
            .frames
    };

    // Her cannon and her ponytail, cited rather than imported for the reason
    // the reach arm above states: this crate sits below the content that
    // authors her.
    let cannon = RangedActionSpec::bolt(540.0, 4).with_flight(ProjectileFlight::STRAIGHT);
    let ponytail = RangedActionSpec::bolt(430.0, 7).with_flight(ProjectileFlight::boomerang(0.34));

    let fired = frames_with(Some(&cannon));
    assert_eq!(
        fired.hazard.map(ambition_entity_catalog::MoveHazard::damage),
        Some(4),
        "the shot's damage did not survive the join, so the one layer holding \
         both halves threw the number away"
    );
    assert_eq!(
        fired.max_damage, 2,
        "the join wrote over the move's own volume damage; `max_damage` is a \
         fact about what the BODY swings and the rollout reads it as one"
    );
    assert_eq!(
        fired.strongest_hit(),
        4,
        "a move that swings for 2 and fires for 4 is worth 4"
    );

    let thrown = frames_with(Some(&ponytail));
    assert_eq!(thrown.strongest_hit(), 7);
    assert!(
        thrown.strongest_hit() > fired.strongest_hit(),
        "the ponytail (7) did not outrank the cannon (4); with both at zero \
         the scorer could only separate them on reach, and the boomerang \
         reaches less"
    );

    // ⭐ THE CONTROL, and it is the arm that fails if the join simply pours
    // the weapon's damage in wherever it finds one: a body carrying NO ranged
    // action has no shot to be worth anything, so the move is worth what it
    // swings for.
    let unarmed = frames_with(None);
    assert_eq!(unarmed.hazard, None);
    assert_eq!(
        unarmed.strongest_hit(),
        2,
        "an unarmed body's move reported a shot's damage"
    );

    // ⚠ AND A MOVE THAT SWINGS HARDER THAN IT SHOOTS KEEPS ITS SWING —
    // without this the `max` passes for a plain overwrite in the other
    // direction.
    let mut heavy = strike_hitting_for("cannon", 20.0, 9);
    heavy.events.push(MoveEvent {
        at_s: 0.18,
        kind: MoveEventKind::Ranged,
    });
    let heavy_set = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([("attack".to_string(), "cannon".to_string())]),
        moves: vec![heavy],
    });
    let both = attack_kit_of(
        Some(&heavy_set),
        true,
        false,
        Some(&brain),
        None,
        Some(&cannon),
        None,
    )
    .into_iter()
    .find(|c| c.move_id == "cannon")
    .expect("the cannon is the body's one move")
    .frames;
    assert_eq!(
        both.strongest_hit(),
        9,
        "a 9-damage swing with a 4-damage shot reported {}; 13 is a sum and 4 \
         is an overwrite",
        both.strongest_hit()
    );
}

/// ⛔⛤ **THE HIT RESOLVER HAS STALED EVERY LANDING SINCE THE MECHANIC EXISTED
/// AND THE BRAIN THAT CHOOSES THE MOVES COULD NOT SEE IT.**
///
/// `apply_hitbox_damage` resolves a landing as `damage × stale_scale(n)` and
/// its launch's percent term as
/// `victim_percent_knockback_scale × knockback_stale_scale(..)`. The brain's
/// `LaunchLaw` carried only the first factor of that product — while
/// `LaunchConditions::growth_scale`'s own doc had said all along that it is
/// *"its `victim_percent_knockback_scale` folded with THIS MOVE'S STALING
/// INFLUENCE"*. A specification on the type and one term in the value.
///
/// ⚠ **THE NUMBERS ARE THE SHIPPED STAGE'S**, cited rather than imported for
/// the reason the ranged arms above state: this crate sits below the content
/// that declares them. `ambition_demo_smash` authors `stale_step: 0.05`,
/// `stale_floor: 0.55`, `stale_knockback_influence: Some(0.30)`.
#[test]
fn a_move_this_body_keeps_landing_reaches_the_kit_already_worn() {
    use ambition_characters::brain::attack_kit::MoveWear;
    use ambition_combat::stale::{stale_move_hash, BodyStaleMoves};

    let jab = strike_hitting_for("jab", 20.0, 10);
    let smash = strike_hitting_for("smash_forward", 40.0, 16);
    let moveset = ActorMoveset(MovesetContract {
        verbs: BTreeMap::from([
            ("attack".to_string(), "jab".to_string()),
            ("smash_forward".to_string(), "smash_forward".to_string()),
        ]),
        moves: vec![jab, smash],
    });
    let brain = fighter_brain();

    let smash_rules = ambition_combat::rules::ResolvedCombatTuning {
        stale_step: 0.05,
        stale_floor: 0.55,
        stale_knockback_influence: 0.30,
        ..Default::default()
    };
    // Nine recent landings of the jab and nothing else — the ring's whole
    // length, which is the genre's own queue and the floor's reference point.
    let mut ring = BodyStaleMoves::default();
    for _ in 0..9 {
        ring.record(stale_move_hash("jab"));
    }

    let wear_of = |worn: Option<super::update::WornMoves>, id: &str| {
        attack_kit_of(Some(&moveset), true, false, Some(&brain), None, None, worn)
            .into_iter()
            .find(|c| c.move_id == id)
            .unwrap_or_else(|| panic!("the kit offers no `{id}`"))
            .wear
    };
    let worn = Some(super::update::WornMoves {
        recent: ring,
        rules: smash_rules,
    });

    let jab_wear = wear_of(worn, "jab");
    // `1 - 0.05 * 9 = 0.55`, which is exactly the declared floor.
    assert!(
        (jab_wear.damage - 0.55).abs() < 1.0e-6,
        "nine landings of the jab left it at {} of its damage; 1.0 means the \
         ring never reached the kit",
        jab_wear.damage
    );
    // `1 - 0.30 * (1 - 0.55) = 0.865` — the damage falls to 55% while the
    // percent term keeps 86.5%, which is the split the hit resolver makes.
    assert!(
        (jab_wear.launch_growth - 0.865).abs() < 1.0e-6,
        "the worn jab's percent term reads {}; 0.55 is the damage answer \
         spent twice and 1.0 is the influence never applied",
        jab_wear.launch_growth
    );

    // ⭐ THE CONTROL, and without it the arm passes for a join that stales
    // everything in the kit: the smash is in the same ring and has landed
    // nothing.
    assert_eq!(
        wear_of(worn, "smash_forward"),
        MoveWear::FRESH,
        "a move this body has never landed came back worn"
    );

    // ⭐ AND THE SECOND CONTROL IS THE UNDECLARED WORLD — every Ambition room.
    // `stale_step: 0.0` is no staling, so the same ring must change nothing.
    let ambition = Some(super::update::WornMoves {
        recent: ring,
        rules: ambition_combat::rules::ResolvedCombatTuning::default(),
    });
    assert_eq!(
        wear_of(ambition, "jab"),
        MoveWear::FRESH,
        "a world that declares no staling staled a move anyway"
    );
    // ⭐ AND NO AUTHORITY AT ALL, which is a composition with no stale ring.
    assert_eq!(wear_of(None, "jab"), MoveWear::FRESH);
}
