use super::*;

use ambition_characters::actor::ActorFaction;
use ambition_characters::perception::{
    Perceived, PerceivedSolid, SelfView, SolidKind, StageView, WorldView,
};

/// The same 800×600 envelope the classifier's tests use, origin at its corner.
fn stage() -> StageView {
    StageView {
        bounds: ae::Aabb::new(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(400.0, 300.0)),
    }
}

/// A body reeling at `pos`, launched at `vel`.
fn reeling(pos: ae::Vec2, vel: ae::Vec2) -> WorldView {
    WorldView {
        self_view: SelfView {
            pos,
            vel,
            gravity_down: ae::Vec2::new(0.0, 1.0),
            faction: ActorFaction::Player,
            alive: true,
            phase: BodyPhase::Hitstun,
            ..Default::default()
        },
        stage: stage(),
        ..Default::default()
    }
}

fn stick(view: &WorldView) -> ae::LocalAxes {
    survival_stick(Perceived::cheating(view))
        .expect("a reeling body on a known stage holds a stick")
}

/// A body that is not being hit holds nothing: its movement belongs to the
/// ordinary decision, and an overlay that fired in neutral would be a brain that
/// walks sideways forever.
#[test]
fn a_body_that_is_not_reeling_asks_for_nothing() {
    let mut view = reeling(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(600.0, 0.0));
    view.self_view.phase = BodyPhase::Neutral;
    assert_eq!(survival_stick(Perceived::cheating(&view)), None);
}

/// Without a stage envelope there is no blastzone to steer away from, and a
/// guessed rotation is as likely to shorten the life as lengthen it.
#[test]
fn an_unknown_stage_produces_no_di() {
    let mut view = reeling(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(600.0, 0.0));
    view.stage = StageView::default();
    assert_eq!(survival_stick(Perceived::cheating(&view)), None);
}

/// THE MECHANIC, AND THE WHOLE POINT: DI is the perpendicular part of the
/// stick, so a body launched flat toward the side blastzone must hold across the
/// launch, not along it. Holding along it does nothing at all.
#[test]
fn a_flat_launch_is_di_d_across_the_launch() {
    let held = stick(&reeling(
        ae::Vec2::new(600.0, 300.0),
        ae::Vec2::new(600.0, 0.0),
    ));
    assert!(
        held.y.abs() > held.x.abs(),
        "a body launched along +x held {held:?}, which has no perpendicular part to influence with"
    );
}

/// And it must pick the side that buys room. Launched right and level with the
/// stage's middle, the survival deflection is the one that turns the flight
/// toward the long axis of the box rather than into the near ceiling or floor.
#[test]
fn di_chooses_the_side_that_keeps_the_body_inside_longer() {
    let view = reeling(ae::Vec2::new(600.0, 100.0), ae::Vec2::new(600.0, 0.0));
    let held = stick(&view);
    let steered = ae::hit_response::di_adjust(
        view.self_view.vel,
        ae::Vec2::new(held.x, held.y),
        view.self_view.gravity_down,
        PROBE_ANGLE,
    );
    let mirrored = ae::hit_response::di_adjust(
        view.self_view.vel,
        ae::Vec2::new(-held.x, -held.y),
        view.self_view.gravity_down,
        PROBE_ANGLE,
    );
    let bounds = view.stage.bounds;
    assert!(
        time_inside(bounds, view.self_view.pos, steered)
            >= time_inside(bounds, view.self_view.pos, mirrored),
        "the brain held the deflection that reaches the blastzone sooner"
    );
}

/// A frozen body still has the hitlag shift, and the only sensible place to
/// spend it is toward the middle of the stage.
#[test]
fn a_frozen_body_holds_toward_the_stage_centre() {
    let held = stick(&reeling(ae::Vec2::new(700.0, 300.0), ae::Vec2::ZERO));
    assert!(
        held.x < 0.0,
        "a body frozen at the right of the stage held {held:?} instead of inward"
    );
}

/// The stick is a full deflection: DI scales with stick magnitude, and a brain
/// that asked for half of it would be throwing away half its influence.
#[test]
fn the_stick_is_held_all_the_way() {
    let held = stick(&reeling(
        ae::Vec2::new(600.0, 300.0),
        ae::Vec2::new(600.0, 0.0),
    ));
    let magnitude = (held.x * held.x + held.y * held.y).sqrt();
    assert!(
        (magnitude - 1.0).abs() < 1e-5,
        "held {magnitude} of the stick"
    );
}

/// Rotated gravity is the same fight. The stick is body-local, so the answer
/// under sideways gravity must be the local mirror of the answer under normal
/// gravity, not a different plan.
#[test]
fn the_answer_is_the_same_under_rotated_gravity() {
    let upright = stick(&reeling(
        ae::Vec2::new(600.0, 100.0),
        ae::Vec2::new(600.0, 0.0),
    ));
    let mut rotated = reeling(ae::Vec2::new(600.0, 100.0), ae::Vec2::new(600.0, 0.0));
    rotated.self_view.gravity_down = ae::Vec2::new(1.0, 0.0);
    let held = stick(&rotated);
    let frame = ae::AccelerationFrame::new(ae::Vec2::new(1.0, 0.0));
    let upright_world = ae::AccelerationFrame::new(ae::Vec2::new(0.0, 1.0))
        .to_world(ae::Vec2::new(upright.x, upright.y));
    let held_world = frame.to_world(ae::Vec2::new(held.x, held.y));
    assert!(
        (held_world - upright_world).length() < 1e-4,
        "the same launch on the same stage produced {held_world:?} under rotated gravity and {upright_world:?} upright"
    );
}

/// A floor whose top surface sits at `top`, wide enough to be under anybody in
/// these fixtures.
fn floor(top: f32) -> PerceivedSolid {
    PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(400.0, top + 50.0), ae::Vec2::new(400.0, 50.0)),
        kind: SolidKind::Solid,
    }
}

/// A body falling out of a launch toward a floor.
fn falling(gap: f32, speed: f32) -> WorldView {
    let floor_top = 500.0;
    let half = ae::Vec2::new(10.0, 20.0);
    let mut view = reeling(
        ae::Vec2::new(400.0, floor_top - gap - half.y),
        ae::Vec2::new(0.0, speed),
    );
    view.self_view.phase = BodyPhase::Neutral;
    view.self_view.tumbling = true;
    view.self_view.half_extent = half;
    view.terrain = vec![floor(floor_top)];
    view
}

/// THE READ: a tumbling body about to touch down presses.
#[test]
fn a_tumbling_body_techs_the_landing_it_can_see_coming() {
    let close = ae::movement::knockdown::TECH_WINDOW * 600.0 * 0.5;
    assert!(tech_press(Perceived::cheating(&falling(close, 600.0))));
}

/// And it does NOT press at the top of the arc. A press that expires without
/// touching anything locks the tech out for twice as long as the window it
/// wasted, so pressing early is worse than not pressing.
#[test]
fn a_body_still_high_above_the_floor_does_not_spend_its_tech() {
    let far = ae::movement::knockdown::TECH_WINDOW * 600.0 * 4.0;
    assert!(!tech_press(Perceived::cheating(&falling(far, 600.0))));
}

/// Rising out of a launch is not a landing, however close the floor is.
#[test]
fn a_body_moving_away_from_the_floor_does_not_tech() {
    let close = ae::movement::knockdown::TECH_WINDOW * 600.0 * 0.5;
    assert!(!tech_press(Perceived::cheating(&falling(close, -600.0))));
}

/// An ordinary fall is not a tumble. Teching a landing the body was never going
/// to be knocked down by would spend the evade for nothing — and it is the same
/// button, so it would come out as an air dodge.
#[test]
fn an_ordinary_fall_is_not_teched() {
    let close = ae::movement::knockdown::TECH_WINDOW * 600.0 * 0.5;
    let mut view = falling(close, 600.0);
    view.self_view.tumbling = false;
    assert!(!tech_press(Perceived::cheating(&view)));
}

/// Nothing below means nothing to tech against. A body tumbling out over the
/// blastzone has a recovery problem, not a landing one.
#[test]
fn a_body_over_the_void_does_not_tech() {
    let close = ae::movement::knockdown::TECH_WINDOW * 600.0 * 0.5;
    let mut view = falling(close, 600.0);
    view.terrain.clear();
    assert!(!tech_press(Perceived::cheating(&view)));
}

/// ⭐ ESCAPE DI: when survival is not at stake, the stick steers AWAY FROM THE
/// FOE instead of picking between two deflections that both survive.
///
/// The genre uses one mechanic for two purposes — survival DI rotates a launch
/// away from the blast zone at kill percent, escape DI rotates it away from the
/// opponent to break a juggle — and only the objective differs, which is why
/// this lives in the brain and not the kernel.
///
/// ⛔ ASSERTED ON THE OUTCOME, NOT ON A SYMMETRY. The first version of this test
/// only checked that moving the foe across the body flipped the stick's sign,
/// and a poison that steered deliberately INTO the foe passed it — flipping the
/// rule flips both answers, so the signs still differed. It could not fail.
/// This runs the chosen stick and its opposite through the kernel's own
/// `di_adjust` and asks which one actually ends further from the foe.
#[test]
fn with_survival_secured_the_stick_steers_away_from_the_foe() {
    let mut view = reeling(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(0.0, -600.0));
    // Long enough that the escape term has real distance to price, short enough
    // that both deflections clear it and the survival term saturates.
    view.self_view.phase_remaining = 0.25;
    let foe = ae::Vec2::new(200.0, 300.0);
    view.actors = vec![ambition_characters::perception::PerceivedActor {
        pos: foe,
        hostile_to_self: true,
        alive: true,
        ..Default::default()
    }];

    let held = stick(&view);
    // Through the KERNEL's rotation, which is the same authority the decision
    // used - re-deriving `di_adjust` here would let the test disagree with the
    // thing it is checking about which way is which.
    let ends_near_foe = |axes: ae::LocalAxes| {
        let steered = ae::hit_response::di_adjust(
            view.self_view.vel,
            ae::Vec2::new(axes.x, axes.y),
            view.self_view.gravity_down,
            super::PROBE_ANGLE,
        );
        (view.self_view.pos + steered * view.self_view.phase_remaining).distance(foe)
    };
    let opposite = ae::LocalAxes {
        x: -held.x,
        y: -held.y,
    };
    assert!(
        ends_near_foe(held) > ends_near_foe(opposite),
        "the stick it chose lands {:.1} from the foe and the other lands {:.1} - \
         it steered toward the opponent, not away",
        ends_near_foe(held),
        ends_near_foe(opposite)
    );
}

/// ⛔ AND SURVIVAL STILL WINS WHERE IT IS ACTUALLY AT STAKE. Near the side wall
/// and launched into it, the two deflections do NOT both clear the hitstun, so
/// the survival term does not saturate and decides outright — whatever the foe
/// is doing. A body that DI'd away from its opponent into the blast zone would
/// have traded a juggle for a stock.
#[test]
fn survival_outranks_escape_when_the_blastzone_is_close() {
    let mut view = reeling(ae::Vec2::new(780.0, 300.0), ae::Vec2::new(1200.0, 0.0));
    view.self_view.phase_remaining = 0.25;
    // The foe is placed so that escaping it means steering FURTHER toward the
    // wall this body is about to cross.
    view.actors = vec![ambition_characters::perception::PerceivedActor {
        pos: ae::Vec2::new(400.0, 300.0),
        hostile_to_self: true,
        alive: true,
        ..Default::default()
    }];
    let held = stick(&view);
    let survival_only = {
        let mut bare = view.clone();
        bare.actors.clear();
        stick(&bare)
    };
    assert_eq!(
        (held.x, held.y),
        (survival_only.x, survival_only.y),
        "with a blastzone in reach the foe changed the answer, so escape overrode survival"
    );
}

/// ⛔⛔ THE CAP IS WHAT MAKES ESCAPE REACHABLE AT ALL, and without this fixture
/// it could be deleted with every other test here still passing.
///
/// Uncapped, `time_inside` is a continuous number that essentially never ties,
/// so survival would decide every launch and the escape term would be dead code
/// reached only on an exact float tie. Capping it at the hitstun this body still
/// owes is what turns "both of these are safe enough" into a genuine tie and
/// hands the decision to the foe.
///
/// ⚠ THE PREMISE IS ASSERTED, because a fixture where the two objectives happen
/// to AGREE would pass whatever the rule is. Measured for this geometry: the two
/// deflections survive 0.53s and 0.77s, so both clear a 0.25s hitstun and the
/// cap ties them — while uncapped survival would prefer the 0.77s one outright.
#[test]
fn the_survival_cap_is_what_lets_the_foe_decide_a_safe_launch() {
    let mut view = reeling(ae::Vec2::new(400.0, 300.0), ae::Vec2::new(300.0, -500.0));
    view.self_view.phase_remaining = 0.25;
    let foe = ae::Vec2::new(700.0, 200.0);
    view.actors = vec![ambition_characters::perception::PerceivedActor {
        pos: foe,
        hostile_to_self: true,
        alive: true,
        ..Default::default()
    }];

    let me = view.self_view.clone();
    let frame = ae::AccelerationFrame::new(me.gravity_down);
    let dir = me.vel / me.vel.length();
    let perpendicular = ae::Vec2::new(-dir.y, dir.x);
    let steer = |c: ae::Vec2| {
        let local = super::unit_local(frame, c).unwrap();
        (
            local,
            ae::hit_response::di_adjust(
                me.vel,
                ae::Vec2::new(local.x, local.y),
                me.gravity_down,
                super::PROBE_ANGLE,
            ),
        )
    };
    let (local_a, steered_a) = steer(-perpendicular);
    let (local_b, steered_b) = steer(perpendicular);
    let inside = |v: ae::Vec2| super::time_inside(view.stage.bounds, me.pos, v);
    let away = |v: ae::Vec2| (me.pos + v * me.phase_remaining).distance(foe);

    // THE PREMISE, and the test is worthless without it: the two objectives must
    // DISAGREE here, and both must be safe for the whole hitstun.
    assert!(
        inside(steered_a).min(inside(steered_b)) > me.phase_remaining,
        "this fixture is meant to be safe either way: {:.3} / {:.3} against {:.3} of hitstun",
        inside(steered_a),
        inside(steered_b),
        me.phase_remaining
    );
    let survival_prefers_a = inside(steered_a) > inside(steered_b);
    let escape_prefers_a = away(steered_a) > away(steered_b);
    assert_ne!(
        survival_prefers_a, escape_prefers_a,
        "the two objectives agree in this fixture, so it cannot tell which one decided"
    );

    let held = stick(&view);
    let chose_a = (held.x, held.y) == (local_a.x, local_a.y);
    assert_eq!(
        chose_a,
        escape_prefers_a,
        "the foe did not decide a launch that was safe either way - uncapped survival did, \
         which means the cap is doing nothing (survival {:.3}/{:.3}, distance {:.1}/{:.1})",
        inside(steered_a),
        inside(steered_b),
        away(steered_a),
        away(steered_b)
    );
    let _ = local_b;
}
