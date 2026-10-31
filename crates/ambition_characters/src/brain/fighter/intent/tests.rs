use super::*;
use crate::brain::attack_kit::{AttackBinding, AttackVerb};
use crate::brain::fighter::options::tests::{candidate, frames, view_with};
use crate::brain::fighter::options::{AttackOption, MoveOption};
use crate::perception::{Perceived, WorldView};

const RUN: f32 = 300.0;

fn option(id: &str, startup_s: f32, reach: f32) -> AttackOption {
    AttackOption {
        move_id: id.to_string(),
        frames: frames(startup_s, reach, 0.2),
        binding: AttackBinding { verb: AttackVerb::Basic, direction: crate::actor::attack_gesture::AttackDir::Forward },
        score: 1.0,
        features: Default::default(),
    }
}

fn movement(verbs: &[MovementVerb]) -> Vec<MoveOption> {
    verbs.iter().map(|&verb| MoveOption { verb, score: 1.0 }).collect()
}

/// The foe 70 px of gap away (bodies 24 px wide), threatening 40 px.
fn spaced(gap: f32) -> WorldView {
    let mut view = view_with(300.0, 300.0 + gap + 24.0);
    view.self_view.half_extent = ae::Vec2::new(12.0, 24.0);
    view.actors[0].half_extent = ae::Vec2::new(12.0, 24.0);
    view.actors[0].threat_reach = Some(40.0);
    view
}

fn space_intent() -> Intent {
    Intent { kind: IntentKind::Space, target: "foe".to_string(), ticks_left: 0 }
}

use ambition_platformer2d_core as ae;

/// Inside the band (threat 40 + 10 .. + 30) a spacing body holds: it neither
/// closes nor backs off, and it swings only at what is already in reach.
#[test]
fn a_spacing_body_holds_its_band() {
    let view = spaced(60.0);
    let mut options = OptionSet {
        movement: movement(&[MovementVerb::Approach, MovementVerb::Retreat, MovementVerb::Jump]),
        attacks: vec![option("poke", 0.1, 30.0), option("long", 0.2, 70.0)],
        ..Default::default()
    };
    constrain(&space_intent(), Perceived::cheating(&view), &mut options, RUN);
    let verbs: Vec<_> = options.movement.iter().map(|m| m.verb).collect();
    assert_eq!(verbs, vec![MovementVerb::Jump]);
    let attacks: Vec<_> = options.attacks.iter().map(|a| a.move_id.as_str()).collect();
    assert_eq!(attacks, vec!["long"], "only a swing that reaches where the foe stands");
}

/// Inside the foe's threat it backs out; beyond the band it closes.
#[test]
fn a_spacing_body_backs_out_of_the_threat_and_closes_from_beyond_the_band() {
    let all = || movement(&[MovementVerb::Approach, MovementVerb::Retreat]);
    let mut close = OptionSet { movement: all(), ..Default::default() };
    constrain(&space_intent(), Perceived::cheating(&spaced(20.0)), &mut close, RUN);
    assert_eq!(close.movement.iter().map(|m| m.verb).collect::<Vec<_>>(), vec![MovementVerb::Retreat]);
    let mut far = OptionSet { movement: all(), ..Default::default() };
    constrain(&space_intent(), Perceived::cheating(&spaced(150.0)), &mut far, RUN);
    assert_eq!(far.movement.iter().map(|m| m.verb).collect::<Vec<_>>(), vec![MovementVerb::Approach]);
}

/// A whiffed swing with 0.5 s of recovery left, 60 px away: an attack that
/// can land in time arms a punish, and the punish closes and keeps only
/// the swings that land inside the window.
#[test]
fn a_whiff_with_time_left_arms_a_punish_that_closes_with_what_lands() {
    let mut view = spaced(60.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.5;
    let mut options = OptionSet {
        movement: movement(&[MovementVerb::Retreat, MovementVerb::Shield, MovementVerb::Jump]),
        attacks: vec![option("fast", 0.1, 30.0), option("slow", 0.6, 30.0)],
        ..Default::default()
    };
    let kit = [candidate("fast", 0.1, 30.0), candidate("slow", 0.6, 30.0)];
    let intent = select(None, Perceived::cheating(&view), Situation::Neutral, &kit, RUN, 60.0, 5)
        .expect("a window this wide arms something");
    assert_eq!(intent.kind, IntentKind::Punish);
    constrain(&intent, Perceived::cheating(&view), &mut options, RUN);
    assert_eq!(options.attacks.iter().map(|a| a.move_id.as_str()).collect::<Vec<_>>(), vec!["fast"]);
    assert_eq!(options.movement.iter().map(|m| m.verb).collect::<Vec<_>>(), vec![MovementVerb::Approach]);
}

/// From the band no swing reaches, so the ranking admits none; the punish
/// is armed from the kit all the same, and closes.
#[test]
fn a_punish_arms_from_the_kit_when_nothing_is_admitted_yet() {
    let mut view = spaced(60.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.5;
    let kit = [candidate("fast", 0.1, 30.0)];
    let intent = select(None, Perceived::cheating(&view), Situation::Neutral, &kit, RUN, 60.0, 5);
    assert_eq!(intent.map(|i| i.kind), Some(IntentKind::Punish));
}

/// A window no swing can make is no punish: the body keeps its spacing.
#[test]
fn a_window_too_short_to_land_in_is_not_a_punish() {
    let mut view = spaced(200.0);
    view.actors[0].phase = BodyPhase::AttackRecovery;
    view.actors[0].phase_remaining = 0.2;
    let kit = [candidate("fast", 0.1, 30.0)];
    let intent = select(None, Perceived::cheating(&view), Situation::Neutral, &kit, RUN, 60.0, 5);
    assert_eq!(intent.map(|i| i.kind), Some(IntentKind::Space));
}

/// Recovery is the reactive brain's alone.
#[test]
fn recovery_holds_no_intent() {
    let view = spaced(60.0);
    let intent = select(None, Perceived::cheating(&view), Situation::Recovery, &[], RUN, 60.0, 5);
    assert_eq!(intent, None);
}
