use super::*;

use crate::actor::ActorFaction;
use crate::perception::{Perceived, SelfView, StageView, WorldView};

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
