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
    let mut model = ambition_platformer2d_core::movement::MotionModel::axis_swept(
        ae::DEFAULT_TUNING.axis_swept_params(),
    );
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
        .get::<ambition_platformer2d_core::movement::MotionModel>(entity)
        .expect("the body kept its motion model")
    {
        ae::MotionModel::AxisSwept(axis) => axis.state.ledge_grab.is_some(),
        _ => false,
    }
}

fn invuln(app: &App, entity: Entity) -> f32 {
    match app
        .world()
        .get::<ambition_platformer2d_core::movement::MotionModel>(entity)
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
        .get_mut::<ambition_platformer2d_core::movement::MotionModel>(climbing)
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
                .insert_resource(crate::rules::ResolvedCombatTuning {
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
            .insert_resource(crate::rules::ResolvedCombatTuning {
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
    let contest = |occupancy: Option<crate::rules::LedgeOccupancy>| -> (bool, bool) {
        let mut app = app();
        if let Some(occupancy) = occupancy {
            app.insert_resource(crate::rules::ResolvedCombatTuning {
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
        contest(Some(crate::rules::LedgeOccupancy::Trump)),
        (false, true),
        "under Trump the newcomer must take the edge and the camper must fall"
    );

    // HOG — the same contest, the other survivor.
    assert_eq!(
        contest(Some(crate::rules::LedgeOccupancy::Hog)),
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

/// ONE EDGE IS ONE EDGE IN ONE LIVE ROOM (OW1). Two live rooms share one local
/// frame, so the same anchor in two rooms is two edges, and each body keeps
/// its own. Control: the same two bodies in one room, where the later arrival
/// keeps the edge.
#[test]
fn the_same_anchor_in_two_live_rooms_is_two_edges() {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
    };
    let first = LiveRoomInstance::ACTIVATION;
    let hanging = |camper_room: LiveRoomInstance| {
        let mut app = app();
        app.world_mut().spawn((RoomInstanceRoot, first));
        app.world_mut().spawn((RoomInstanceRoot, first.next()));
        let anchor = ae::Vec2::new(100.0, 100.0);
        let camper = hanging_at(&mut app, "camper", anchor, 1.4);
        let arriving = hanging_at(&mut app, "arriving", anchor, 0.02);
        app.world_mut().entity_mut(camper).insert(InRoomInstance(camper_room));
        app.world_mut().entity_mut(arriving).insert(InRoomInstance(first));
        app.update();
        (still_hanging(&app, camper), still_hanging(&app, arriving))
    };
    assert_eq!(
        (hanging(first), hanging(first.next())),
        ((false, true), (true, true)),
        "(camper and arrival in one room, camper in the other live room): (camper hangs, arrival hangs)"
    );
}

/// A block whose top-left corner is (100, 100) and top-right corner (300, 100).
fn one_block_world() -> ae::World {
    ae::World::new(
        "ledge",
        ae::Vec2::new(800.0, 600.0),
        ae::Vec2::ZERO,
        vec![ae::world::Block::solid(
            "stage",
            ae::Vec2::new(100.0, 100.0),
            ae::Vec2::new(200.0, 200.0),
        )],
    )
}

/// The contact the kernel's probe gives a body of `size` clung to one face of
/// [`one_block_world`]'s block, its head 13 px above the lip. `face` is the
/// wall normal: -1 for the left face, +1 for the right.
fn probed_contact(size: ae::Vec2, face: f32) -> LedgeContact {
    let wall_x = if face < 0.0 { 100.0 } else { 300.0 };
    let pos = ae::Vec2::new(wall_x + face * size.x * 0.5, 87.0 + size.y * 0.5);
    ae::ledge_grab::probe_ledge_grab_in_frame(pos, size, face, &one_block_world(), ae::Vec2::new(0.0, 1.0))
        .unwrap_or_else(|| panic!("premise: a {size:?} body on face {face} catches its ledge"))
}

fn hanging_on(app: &mut App, id: &str, contact: LedgeContact, elapsed: f32) -> Entity {
    let entity = hanging_at(app, id, contact.anchor, elapsed);
    let mut model = app
        .world_mut()
        .get_mut::<ambition_platformer2d_core::movement::MotionModel>(entity)
        .expect("the body has a motion model");
    if let ae::MotionModel::AxisSwept(axis) = &mut *model {
        axis.state.ledge_grab = Some(LedgeGrabState {
            elapsed,
            ..LedgeGrabState::hanging(contact)
        });
    }
    entity
}

/// ⭐ ONE CORNER IS ONE EDGE, WHATEVER THE BODIES' SIZES.
///
/// The contacts come from the kernel's probe, not from a shared anchor built
/// by hand: a small and a large fighter on one corner hang at centres more
/// than 1 px apart, and the rule compared those centres. The later arrival
/// keeps the edge. The control is the same pair on the two faces of the
/// block: two edges, both kept.
#[test]
fn two_fighters_of_different_sizes_on_one_corner_are_one_edge() {
    let small_size = ae::Vec2::new(28.0, 46.0);
    let large_size = ae::Vec2::new(44.0, 76.0);
    let small = probed_contact(small_size, -1.0);
    let large = probed_contact(large_size, -1.0);
    assert!(
        small.anchor.distance(large.anchor) > SAME_EDGE_EPSILON,
        "premise: the two hang centres differ, or this case is the old one"
    );

    let mut app = app();
    let camper = hanging_on(&mut app, "small", small, 1.4);
    let arriving = hanging_on(&mut app, "large", large, 0.02);
    app.update();
    assert!(still_hanging(&app, arriving), "the later arrival lost the edge");
    assert!(!still_hanging(&app, camper), "two fighters of different sizes shared one corner");

    // Control: one on each face of the block is two edges.
    let mut app = app_two_faces(small_size, large_size);
    app.update();
    let bodies: Vec<Entity> = {
        let world = app.world_mut();
        let mut query = world.query::<(Entity, &SimId)>();
        query.iter(world).map(|(entity, _)| entity).collect()
    };
    let hanging: Vec<bool> = bodies.into_iter().map(|entity| still_hanging(&app, entity)).collect();
    assert_eq!(hanging, vec![true, true], "control: a body on each face keeps its own edge");
}

fn app_two_faces(left: ae::Vec2, right: ae::Vec2) -> App {
    let mut app = app();
    hanging_on(&mut app, "left", probed_contact(left, -1.0), 1.4);
    hanging_on(&mut app, "right", probed_contact(right, 1.0), 0.02);
    app
}

/// THE TRUMPED BODY'S LOCKOUT (LEDGE-OCCUPANCY): a declared
/// `ledge_trump_lockout` holds the body that lost the edge for that long, as
/// a hard control lock. The controls: the winner gets no lock, and a world
/// that declares no lockout leaves the loser in control.
#[test]
fn a_declared_lockout_holds_the_trumped_body_and_not_the_one_that_trumped() {
    let locks_after_trump = |lockout: Option<f32>| -> (f32, f32) {
        let mut app = app();
        if let Some(lockout) = lockout {
            app.world_mut().insert_resource(crate::rules::ResolvedCombatTuning {
                ledge_trump_lockout: lockout,
                ..Default::default()
            });
        }
        let edge = ae::Vec2::new(100.0, 100.0);
        let loser = hanging_at(&mut app, "loser", edge, 0.5);
        let winner = hanging_at(&mut app, "winner", edge, 0.1);
        for body in [loser, winner] {
            app.world_mut()
                .entity_mut(body)
                .insert(ambition_characters::actor::BodyCombat::default());
        }
        app.update();
        assert!(!still_hanging(&app, loser), "precondition: the fixture trumped nobody");
        let lock = |body| {
            app.world()
                .get::<ambition_characters::actor::BodyCombat>(body)
                .expect("the body kept its combat state")
                .hard_lock_timer()
        };
        (lock(loser), lock(winner))
    };
    assert_eq!(
        (locks_after_trump(Some(0.5)), locks_after_trump(None)),
        ((0.5, 0.0), (0.0, 0.0)),
        "((loser, winner) with a 0.5 s lockout, the same with none): the hard lock"
    );
}
