//! The trump's ARBITRATION, on a hand-built app.
//!
//! The hang itself is the kernel's and is guarded in
//! `ambition_platformer2d_core::ledge_grab`. What is left to prove here is the
//! part only a multi-body pass can decide: that one edge ends the tick with one
//! body on it, that the survivor is the LATER arrival, and that a body on its
//! own edge is left alone.

use super::*;
use ae::ledge_grab::{LedgeContact, LedgeGrabState};

fn hanging_at(app: &mut App, id: &str, anchor: ae::Vec2, elapsed: f32) -> Entity {
    let mut model =
        crate::features::MotionModel::axis_swept(ae::DEFAULT_TUNING.axis_swept_params());
    let ae::MotionModel::AxisSwept(axis) = &mut model else {
        unreachable!("axis_swept built an axis model")
    };
    axis.state.ledge_grab = Some(LedgeGrabState {
        elapsed,
        ..LedgeGrabState::hanging(LedgeContact {
            wall_normal_x: 1.0,
            anchor,
            climb_target: anchor,
        })
    });
    // The window a grab arms, so losing it is observable.
    axis.state.dodge_roll_timer = ae::LEDGE_GRAB_INVULN_TIME;
    app.world_mut()
        .spawn((SimId::placement(id), model, ae::BodyLedgeState::default()))
        .id()
}

fn still_hanging(app: &App, entity: Entity) -> bool {
    match app
        .world()
        .get::<crate::features::MotionModel>(entity)
        .expect("the body kept its motion model")
    {
        ae::MotionModel::AxisSwept(axis) => axis.state.ledge_grab.is_some(),
        _ => false,
    }
}

fn invuln(app: &App, entity: Entity) -> f32 {
    match app
        .world()
        .get::<crate::features::MotionModel>(entity)
        .expect("the body kept its motion model")
    {
        ae::MotionModel::AxisSwept(axis) => axis.state.dodge_roll_timer,
        _ => 0.0,
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_systems(Update, resolve_ledge_trumps);
    app
}

/// THE LATER ARRIVAL KEEPS THE EDGE, AND THE EARLIER ONE KEEPS NOTHING.
///
/// all three halves: the trumper stays on, the trumped comes off, and the
/// window it bought with airtime it no longer has comes off with it. A body
/// dropped while still intangible would be the safest thing on the stage.
#[test]
fn the_body_that_caught_the_edge_last_keeps_it() {
    let mut app = app();
    let anchor = ae::Vec2::new(100.0, 100.0);
    let camper = hanging_at(&mut app, "camper", anchor, 1.4);
    let arriving = hanging_at(&mut app, "arriving", anchor, 0.02);

    app.update();

    assert!(
        still_hanging(&app, arriving),
        "the later arrival lost the edge"
    );
    assert!(!still_hanging(&app, camper), "two bodies shared one edge");
    assert_eq!(
        invuln(&app, camper),
        0.0,
        "a trumped body fell away still intangible"
    );
    assert!(
        app.world()
            .get::<ae::BodyLedgeState>(camper)
            .expect("the trumped body kept its ledge cluster")
            .release_cooldown
            > 0.0,
        "nothing stopped the trumped body re-latching on the next tick"
    );
}

/// A BODY ON ITS OWN EDGE IS LEFT ALONE.
///
/// the floor, and without it the test above would also pass on a system that
/// simply knocked every hanging body off.
#[test]
fn two_bodies_on_two_edges_both_keep_them() {
    let mut app = app();
    let left = hanging_at(&mut app, "left", ae::Vec2::new(0.0, 100.0), 1.4);
    let right = hanging_at(&mut app, "right", ae::Vec2::new(400.0, 100.0), 0.02);

    app.update();

    assert!(still_hanging(&app, left));
    assert!(still_hanging(&app, right));
    assert_eq!(invuln(&app, left), ae::LEDGE_GRAB_INVULN_TIME);
}

/// THREE ON ONE EDGE LEAVES ONE, AND A TIE IS BROKEN BY IDENTITY.
///
/// two fighters that grabbed on the same tick have the same `elapsed` to the
/// float. Resolving that by query order would be stable within a run and NOT
/// stable across a rollback resimulation, which is the definition of a desync.
#[test]
fn a_tie_is_broken_the_same_way_every_time() {
    let mut app = app();
    let anchor = ae::Vec2::new(100.0, 100.0);
    let old = hanging_at(&mut app, "c_old", anchor, 1.4);
    let a = hanging_at(&mut app, "a_tied", anchor, 0.02);
    let b = hanging_at(&mut app, "b_tied", anchor, 0.02);

    app.update();

    assert!(still_hanging(&app, a), "the tie went out of SimId order");
    assert!(!still_hanging(&app, b));
    assert!(!still_hanging(&app, old));
}

/// A BODY ALREADY PULLING ITSELF UP IS NOT CONTESTING THE EDGE.
///
/// trumping it would cancel a getup that has already left the hang, which is
/// a different mechanic — and a worse one, because the getup is the beat a
/// fighter is committed to and cannot answer.
#[test]
fn a_body_mid_getup_is_neither_trumper_nor_trumped() {
    let mut app = app();
    let anchor = ae::Vec2::new(100.0, 100.0);
    let climbing = hanging_at(&mut app, "climbing", anchor, 0.02);
    let hanging = hanging_at(&mut app, "hanging", anchor, 1.4);
    let mut model = app
        .world_mut()
        .get_mut::<crate::features::MotionModel>(climbing)
        .expect("the body kept its motion model");
    let ae::MotionModel::AxisSwept(axis) = &mut *model else {
        unreachable!()
    };
    axis.state.ledge_grab.as_mut().expect("hanging").climbing = true;
    drop(model);

    app.update();

    assert!(
        still_hanging(&app, hanging),
        "a body mid-getup trumped somebody it had already stopped contesting"
    );
}
