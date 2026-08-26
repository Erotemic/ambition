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
    axis.state.ledge_invuln_timer = ae::LEDGE_GRAB_INVULN_TIME;
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
        ae::MotionModel::AxisSwept(axis) => axis.state.ledge_invuln_timer,
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

/// ⭐⭐ A TRUMPED BODY IS THROWN OFF THE EDGE — WHEN THE MATCH ASKS FOR IT.
///
/// The parity inventory's *"Ledge-trump outward pop/commitment"*. ⛔ AND IT IS A
/// DECLARED RULE, not the law: trumping exists in every platform fighter and
/// being popped outward does not, so a world that declares nothing keeps
/// today's behaviour — the loser simply drops.
mod outward_pop {
    use super::*;

    /// A hanging body that also has a velocity to be thrown with.
    fn hanging_body_at(app: &mut App, id: &str, anchor: ae::Vec2, elapsed: f32) -> Entity {
        let entity = hanging_at(app, id, anchor, elapsed);
        app.world_mut()
            .entity_mut(entity)
            .insert(ae::BodyKinematics::default());
        entity
    }

    fn velocity_after_trump(pop: Option<f32>) -> f32 {
        let mut app = app();
        if let Some(pop) = pop {
            app.world_mut()
                .insert_resource(ambition_combat::rules::ResolvedCombatTuning {
                    ledge_trump_pop: pop,
                    ..Default::default()
                });
        }
        let edge = ae::Vec2::new(100.0, 100.0);
        // The EARLIER arrival is the one that loses the edge.
        let loser = hanging_body_at(&mut app, "loser", edge, 0.5);
        let _winner = hanging_body_at(&mut app, "winner", edge, 0.1);
        app.update();
        assert!(
            !still_hanging(&app, loser),
            "the fixture never trumped anybody, so there is no pop to observe"
        );
        app.world()
            .get::<ae::BodyKinematics>(loser)
            .expect("the loser kept its kinematics")
            .vel
            .x
    }

    /// ⛔ THE BASELINE IS UNCHANGED. A world that declared no rule drops the
    /// loser where it hung, which is what every trump did before this existed.
    #[test]
    fn a_world_that_declares_no_pop_drops_the_loser_in_place() {
        assert_eq!(velocity_after_trump(None), 0.0);
        assert_eq!(velocity_after_trump(Some(0.0)), 0.0);
    }

    /// ⭐ AND THE POP GOES OUTWARD — away from the wall, at the declared speed.
    ///
    /// ⛔ THE DIRECTION IS THE HANG'S `wall_normal_x`, read before the knock-off
    /// clears it. A reading off the body's facing would be backwards for a body
    /// hanging facing out, and there would be nothing left to read afterwards.
    #[test]
    fn a_declared_pop_throws_the_loser_away_from_the_wall() {
        // The fixture hangs with the wall pushing toward +x.
        assert_eq!(velocity_after_trump(Some(420.0)), 420.0);
    }

    /// ⛔⛔ AND OUTWARD IS THE BODY'S OWN SIDE, NOT WORLD X.
    ///
    /// `wall_normal_x` is a body-LOCAL side sign despite its name — its producer
    /// computes `world_normal.dot(frame.side).signum()`, and
    /// `probe_ledge_grab_in_frame` says so in as many words. The pop wrote
    /// `kin.vel.x`, so under sideways gravity it threw the loser along the axis
    /// it FALLS on and left its outward drift untouched.
    ///
    /// ⚠ A 2026-08-25 review reported exactly this and it was REFUSED, on the
    /// reading that the input was world-X too — taken from the NAME. The name
    /// was the stale thing.
    #[test]
    fn the_pop_leaves_along_the_bodys_own_side_under_rotated_gravity() {
        let mut app = app();
        app.world_mut()
            .insert_resource(ambition_combat::rules::ResolvedCombatTuning {
                ledge_trump_pop: 420.0,
                ..Default::default()
            });
        let edge = ae::Vec2::new(100.0, 100.0);
        let loser = hanging_body_at(&mut app, "loser", edge, 0.5);
        let _winner = hanging_body_at(&mut app, "winner", edge, 0.1);

        // GRAVITY PULLS ALONG +X, so the body's side axis is world Y.
        let mut resolved =
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default();
        resolved.publish_resolved_frame(ae::MotionFrame::from_direction(
            ae::Vec2::new(1.0, 0.0),
            900.0,
        ));
        app.world_mut().entity_mut(loser).insert(resolved);

        app.update();
        assert!(
            !still_hanging(&app, loser),
            "the fixture never trumped anybody, so there is no pop to observe"
        );
        let vel = app
            .world()
            .get::<ae::BodyKinematics>(loser)
            .expect("the loser kept its kinematics")
            .vel;
        assert!(
            vel.y.abs() > 400.0,
            "the pop left {vel:?} — outward under this gravity is world Y, and \
             nothing went that way"
        );
        assert!(
            vel.x.abs() < 1.0,
            "the pop pushed {vel:?} along the axis this body FALLS on — it is \
             writing world X and calling it outward"
        );
    }
}

/// UNDER THE HOG RULE THE SAME CONTEST RESOLVES THE OTHER WAY.
///
/// ⭐ THE PAIR IS THE POINT, and it is the same fixture twice: identical
/// camper and arriving bodies, identical edge, one declared rule apart. That is
/// what makes this a POLICY rather than two mechanics — and it is why the arm
/// asserting Trump is here beside it rather than trusted from the neighbouring
/// test, which declares no rules at all.
#[test]
fn the_ledge_policy_decides_which_holder_survives() {
    let contest = |occupancy: Option<ambition_combat::rules::LedgeOccupancy>| -> (bool, bool) {
        let mut app = app();
        if let Some(occupancy) = occupancy {
            app.insert_resource(ambition_combat::rules::ResolvedCombatTuning {
                ledge_occupancy: occupancy,
                ..Default::default()
            });
        }
        let anchor = ae::Vec2::new(100.0, 100.0);
        let camper = hanging_at(&mut app, "camper", anchor, 1.4);
        let arriving = hanging_at(&mut app, "arriving", anchor, 0.02);
        app.update();
        (still_hanging(&app, camper), still_hanging(&app, arriving))
    };

    // TRUMP — declared explicitly, so this arm is about the RULE rather than
    // about a world that happens to declare nothing.
    assert_eq!(
        contest(Some(ambition_combat::rules::LedgeOccupancy::Trump)),
        (false, true),
        "under Trump the newcomer must take the edge and the camper must fall"
    );

    // HOG — the same contest, the other survivor.
    assert_eq!(
        contest(Some(ambition_combat::rules::LedgeOccupancy::Hog)),
        (true, false),
        "under Hog the body that got there first must keep the edge"
    );

    // AND A WORLD THAT DECLARES NOTHING STILL TRUMPS, which is every ledge in
    // this engine before the knob existed.
    assert_eq!(
        contest(None),
        (false, true),
        "an undeclared world stopped trumping"
    );
}
