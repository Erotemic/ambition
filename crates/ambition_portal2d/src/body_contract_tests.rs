//! What every body that crosses a portal can rely on, whatever stores it.
//!
//! Bodies use the same physical components for each custody state.

use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, AabbExt, BodyKinematics, SweepSample};

use crate::color::{PortalChannel, PortalGunColor};
use crate::placement::{transit_step_with_tuning, TransitStep};
use crate::transit::{PortalTransit};
use crate::tuning::PortalTuning;
use crate::types::{PlacedPortal};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
const HALF: f32 = crate::types::PORTAL_OPENING_HALF;

/// A free body: where it is, how it moves, its last step, and whether
/// something holds it.
fn physical_body(pos: Vec2, vel: Vec2, step: Option<(Vec2, Vec2)>) -> (BodyKinematics, SweepSample) {
    let body = BodyKinematics { pos, vel, canonical_size: Vec2::splat(8.0) * 2.0, ..Default::default() };
    let mut sweep = SweepSample::at_rest(body, Vec2::Y);
    if let Some((from, to)) = step {
        sweep.prev = from;
        sweep.curr = to;
    } else {
        sweep.time = [1.0, 1.0];
    }
    (body, sweep)
}

/// A wall pair: blue at x = 20 facing +x, orange at x = 380 facing -x.
fn wall_pair(app: &mut App) -> [Entity; 2] {
    [
        app.world_mut()
            .spawn(PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF).unwrap())
            .id(),
        app.world_mut()
            .spawn(PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF).unwrap())
            .id(),
    ]
}

fn transit_app() -> App {
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.add_systems(Update, crate::portal_transit);
    app
}

/// A body something holds is not in the world: it does not cross the
/// portal its stale position is in, and a straddle it had is released.
#[test]
fn a_held_free_body_neither_crosses_nor_straddles() {
    let mut app = transit_app();
    wall_pair(&mut app);
    let at = Vec2::new(18.0, 200.0);
    let body = app
        .world_mut()
        .spawn((
            physical_body(at, Vec2::new(-300.0, 0.0), None),
            PortalTransit { straddling: BLUE, crossed: false },
            ae::BodyInStorage,
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos, at, "a held body was moved by the pair");
    assert!(app.world().get::<PortalTransit>(body).is_none(), "a held body still straddles");

    // Control: the same body in the world crosses.
    app.world_mut().entity_mut(body).remove::<ae::BodyInStorage>();
    app.world_mut().entity_mut(body).insert(PortalTransit { straddling: BLUE, crossed: false });
    app.world_mut().entity_mut(body).insert(SweepSample { prev: at + Vec2::X * 12.0, ..physical_body(at, Vec2::new(-300.0, 0.0), Some((at + Vec2::X * 12.0, at))).1 });
    app.update();
    assert!(app.world().get::<BodyKinematics>(body).unwrap().pos.x > 300.0, "control: the body did not cross");
}

/// A crossing that skipped `Begin` (one step from in front of the plane to
/// past the hole) leaves the body straddling the exit, crossed, so it is
/// drawn cut there as every crossing is.
#[test]
fn a_crossing_that_skipped_begin_leaves_the_body_straddling_the_exit() {
    let mut app = transit_app();
    wall_pair(&mut app);
    // Kernel: its step began 30 px in front of blue and ends 6 px behind it.
    let kin = BodyKinematics { material: Default::default(), pos: Vec2::new(14.0, 200.0), vel: Vec2::new(-1800.0, 0.0), canonical_size: Vec2::splat(16.0), facing: 1.0 };
    let record = SweepSample { travel: Default::default(), time: [0.0, 1.0], prev: Vec2::new(44.0, 200.0), curr: kin.pos, vel: kin.vel, half: kin.size() * 0.5, down: Vec2::new(0.0, 1.0) };
    let kernel = app.world_mut().spawn((kin, record)).id();
    let free = app
        .world_mut()
        .spawn(physical_body(Vec2::new(14.0, 230.0), Vec2::new(-1800.0, 0.0), Some((Vec2::new(44.0, 230.0), Vec2::new(14.0, 230.0)))))
        .id();
    app.update();
    for body in [kernel, free] {
        let transit = app.world().get::<PortalTransit>(body).copied();
        assert!(
            transit.is_some_and(|t| t.crossed && t.straddling == ORANGE),
            "{body:?}: crossed without a straddle of the exit: {transit:?}"
        );
    }
}

/// When the last pair closes, a body that straddled it straddles nothing:
/// a kept `PortalTransit` withholds wall verbs and draws the body cut.
#[test]
fn the_last_pair_closing_ends_every_straddle() {
    let mut app = transit_app();
    let [blue, orange] = wall_pair(&mut app);
    let kin = BodyKinematics { material: Default::default(), pos: Vec2::new(22.0, 200.0), vel: Vec2::ZERO, canonical_size: Vec2::splat(16.0), facing: 1.0 };
    let kernel = app.world_mut().spawn((kin, PortalTransit { straddling: BLUE, crossed: false })).id();
    let free = app
        .world_mut()
        .spawn((
            physical_body(Vec2::new(22.0, 230.0), Vec2::ZERO, None),
            PortalTransit { straddling: BLUE, crossed: false },
        ))
        .id();
    app.world_mut().entity_mut(blue).despawn();
    app.world_mut().entity_mut(orange).despawn();
    app.update();
    for body in [kernel, free] {
        assert!(app.world().get::<PortalTransit>(body).is_none(), "{body:?} still straddles a closed pair");
    }
}

/// A fast free body crosses a plane between two samples, past the hole: its
/// last step takes it through, as a kernel body's sweep does.
#[test]
fn a_fast_free_body_crosses_on_its_last_step() {
    let mut app = transit_app();
    wall_pair(&mut app);
    // 16 px walls: 80 px behind blue is past its hole, so only the step can
    // take the body there.
    let mut depths = crate::PortalHostDepthsByRoom::default();
    depths.push(None, BLUE, 16.0);
    depths.push(None, ORANGE, 16.0);
    app.insert_resource(depths);
    let body = app
        .world_mut()
        .spawn(physical_body(Vec2::new(-60.0, 200.0), Vec2::new(-2400.0, 0.0), Some((Vec2::new(40.0, 200.0), Vec2::new(-60.0, 200.0)))))
        .id();
    app.update();
    // 80 px into blue is 80 px out of orange, moving out of it.
    let probe = app.world().get::<BodyKinematics>(body).unwrap();
    assert!((probe.pos - Vec2::new(300.0, 200.0)).length() < 1e-3, "the step through the opening did not cross: {probe:?}");
    assert_eq!(probe.vel, Vec2::new(-2400.0, 0.0), "momentum was not carried: {probe:?}");

    // Control: the same body with no step is past the hole and is not taken.
    let stale = app
        .world_mut()
        .spawn(physical_body(Vec2::new(-60.0, 200.0), Vec2::new(-2400.0, 0.0), None))
        .id();
    app.update();
    assert_eq!(app.world().get::<BodyKinematics>(stale).unwrap().pos.x, -60.0, "control: a body with no step was taken");
}

/// A floor portal at y = 300 closes under a free body whose centre is 4
/// over its plane: it is pushed clear, as a kernel body is.
#[test]
fn a_closing_plane_pushes_a_free_body_clear() {
    let mut app = App::new();
    app.init_resource::<crate::PortalFrameHistory>();
    app.add_systems(
        Update,
        crate::evict_straddlers_on_portal_change,
    );
    let closing = app
        .world_mut()
        .spawn(PlacedPortal::fixed(BLUE, Vec2::new(500.0, 300.0), Vec2::new(0.0, -1.0), HALF).unwrap())
        .id();
    let body = app.world_mut().spawn(physical_body(Vec2::new(500.0, 296.0), Vec2::ZERO, None)).id();
    app.update();
    app.world_mut().entity_mut(closing).despawn();
    app.update();
    let y = app.world().get::<BodyKinematics>(body).unwrap().pos.y;
    assert!(y + 8.0 <= 300.0, "the body still straddles the closed plane: y={y}");
}

/// Two coincident planes facing each other close at once over a body
/// centred on them. Each pushes it to a different side, so the side it ends
/// on is the order they are taken in: the same whatever order the history
/// holds them in.
#[test]
fn planes_closing_together_push_a_body_the_same_whatever_their_order() {
    let ends: Vec<Vec2> = [[BLUE, ORANGE], [ORANGE, BLUE]]
        .into_iter()
        .map(|order| {
            let mut app = App::new();
            app.init_resource::<crate::PortalFrameHistory>();
            app.add_systems(Update, crate::evict_straddlers_on_portal_change);
            let planes: Vec<Entity> = order
                .into_iter()
                .map(|channel| {
                    let normal = if channel == BLUE { Vec2::new(0.0, -1.0) } else { Vec2::new(0.0, 1.0) };
                    app.world_mut().spawn(PlacedPortal::fixed(channel, Vec2::new(500.0, 300.0), normal, HALF).unwrap()).id()
                })
                .collect();
            let kin = BodyKinematics { material: Default::default(), pos: Vec2::new(500.0, 300.0), vel: Vec2::ZERO, canonical_size: Vec2::splat(16.0), facing: 1.0 };
            let body = app.world_mut().spawn(kin).id();
            app.update();
            for plane in planes {
                app.world_mut().entity_mut(plane).despawn();
            }
            app.update();
            app.world().get::<BodyKinematics>(body).unwrap().pos
        })
        .collect();
    assert_ne!(ends[0], Vec2::new(500.0, 300.0), "control: the closing planes pushed nothing");
    assert_eq!(ends[0], ends[1], "the push depends on the order the planes were held in");
}



/// Where a leg enters a paired opening, as a fraction of it; not at all
/// beside the opening or through an unpaired portal.
#[test]
fn a_leg_ends_where_its_centre_enters_a_paired_opening() {
    let blue = PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF).unwrap();
    let orange = PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF).unwrap();
    let size = Vec2::splat(16.0);
    let t = crate::first_portal_crossing(Vec2::new(40.0, 200.0), Vec2::new(0.0, 200.0), size, &[blue.clone(), orange.clone()], None);
    assert!(t.unwrap().is_some_and(|t| (t - 0.5).abs() < 1e-4), "{t:?}");
    let beside = Vec2::new(0.0, 200.0 + HALF + 30.0);
    assert_eq!(crate::first_portal_crossing(beside + Vec2::new(40.0, 0.0), beside, size, &[blue.clone(), orange], None).unwrap(), None);
    assert_eq!(crate::first_portal_crossing(Vec2::new(40.0, 200.0), Vec2::new(0.0, 200.0), size, &[blue], None).unwrap(), None);
}

#[test]
fn a_body_beside_an_opening_does_not_open_it() {
    // A wall portal facing +x whose top end is a ledge's top (y up from the
    // end is past it): a body standing on the ledge, its feet on the end and
    // its box over the face, is beside the opening, not in line with it.
    let p = PlacedPortal::fixed(BLUE, Vec2::new(100.0, 248.0), Vec2::new(1.0, 0.0), 40.0).unwrap();
    let top_end = 248.0 - 40.0;
    let half = Vec2::new(15.0, 24.0);
    let on_the_ledge = ambition_platformer2d_core::Aabb::new(Vec2::new(98.0, top_end - half.y + 0.6), half);
    assert!(!crate::transit::body_opens(&p, on_the_ledge, on_the_ledge.center(), Vec2::new(138.0, 37.5), 60.0));
    // In line with it, coming in from the front, it opens.
    let in_line = ambition_platformer2d_core::Aabb::new(Vec2::new(110.0, 260.0), half);
    assert!(crate::transit::body_opens(&p, in_line, in_line.center(), Vec2::new(-138.0, 0.0), 60.0));
}

#[test]
fn a_body_standing_past_an_openings_end_does_not_open_it() {
    // A portal on a floor line (no host depth) facing up, from x 60 to 140.
    // A body standing on the line past its right end, its box over the end
    // and settling into the line as standing bodies do, is not falling
    // through: the hole stays shut under it (the c130 ramp, Jon 2026-10-16).
    let p = PlacedPortal::fixed(BLUE, Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0), 40.0).unwrap();
    let half = Vec2::new(11.0, 20.0);
    let past_the_end = ambition_platformer2d_core::Aabb::new(Vec2::new(145.0, 300.0 - half.y + 0.5), half);
    assert!(!crate::transit::body_opens(&p, past_the_end, past_the_end.center(), Vec2::new(0.0, 30.0), 0.0));
    // Over the opening, falling, it opens.
    let over = ambition_platformer2d_core::Aabb::new(Vec2::new(100.0, 300.0 - half.y + 0.5), half);
    assert!(crate::transit::body_opens(&p, over, over.center(), Vec2::new(0.0, 30.0), 0.0));
}

/// A body whose pose an owner holds (a boss's arm) is posed in its own chart:
/// the pair does not move it, however far into the opening it is posed, and it
/// is in the pair while it reaches into it from the front, so its pieces are
/// at the far end (`docs/planning/engine/portals-a-body-in-two-places.md`).
#[test]
fn a_posed_body_is_in_the_pair_it_reaches_into_and_is_not_moved() {
    let mut app = transit_app();
    wall_pair(&mut app);
    let pose = |app: &mut App, body: Entity, x: f32| {
        app.world_mut().get_mut::<BodyKinematics>(body).unwrap().pos = Vec2::new(x, 200.0);
        app.update();
        let at = app.world().get::<BodyKinematics>(body).unwrap().pos.x;
        (at, app.world().get::<PortalTransit>(body).map(|t| t.straddling))
    };
    let body = app
        .world_mut()
        .spawn((
            BodyKinematics { material: Default::default(), pos: Vec2::new(60.0, 200.0), vel: Vec2::ZERO, canonical_size: Vec2::splat(16.0), facing: 1.0 },
            ambition_platformer2d_core::PoseOwnedExternally::only(ambition_platformer2d_core::PoseHold::Conductor),
        ))
        .id();
    // In front: whole.
    assert_eq!(pose(&mut app, body, 60.0), (60.0, None));
    // Reaching 3 into the blue's opening, then its centre past the plane,
    // then all of it: in the pair, where it was posed.
    assert_eq!(pose(&mut app, body, 25.0), (25.0, Some(BLUE)));
    assert_eq!(pose(&mut app, body, 15.0), (15.0, Some(BLUE)));
    assert_eq!(pose(&mut app, body, 5.0), (5.0, Some(BLUE)));
    // Out again.
    assert_eq!(pose(&mut app, body, 40.0), (40.0, None));
    // Posed behind the opening without coming through it: not in it.
    assert_eq!(pose(&mut app, body, 5.0), (5.0, None));

    // Control: let go, a body past the plane is taken through.
    app.world_mut().entity_mut(body).remove::<ambition_platformer2d_core::PoseOwnedExternally>();
    app.world_mut().entity_mut(body).insert(PortalTransit { straddling: BLUE, crossed: false });
    app.world_mut().get_mut::<BodyKinematics>(body).unwrap().pos = Vec2::new(15.0, 200.0);
    app.world_mut().get_mut::<BodyKinematics>(body).unwrap().vel = Vec2::new(-300.0, 0.0);
    let kin = *app.world().get::<BodyKinematics>(body).unwrap();
    app.world_mut().entity_mut(body).insert(SweepSample {
        prev: Vec2::new(60.0, 200.0), vel: kin.vel, ..SweepSample::at_rest(kin, Vec2::Y)
    });
    app.update();
    assert!(app.world().get::<BodyKinematics>(body).unwrap().pos.x > 300.0, "control: the body was not taken");
}

/// Projectile contact limits and root transit select the same crossing.
/// A body that extends past an opening end keeps its original room leg.
#[test]
fn a_shots_leg_ends_only_where_the_transit_takes_it() {
    use crate::placement::SweptSample;
    let blue = PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF).unwrap();
    let orange = PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF).unwrap();
    let portals = [blue, orange];
    let size = Vec2::splat(16.0);
    let vel = Vec2::new(-2400.0, 0.0);
    let taken = |from: Vec2, to: Vec2| {
        matches!(
            transit_step_with_tuning(ambition_platformer2d_core::BodyKinematics { pos: to, canonical_size: size, vel: vel, ..Default::default() },
                Some(SweptSample { down: Vec2::ZERO, travel: Default::default(), time: [0.0, 1.0], pos: from, vel }),
                None,
                None,
                &portals,
                Vec2::new(0.0, 1.0),
                &PortalTuning::default(),
            ),
            TransitStep::Transfer { .. }
        )
    };
    for (y, through) in [(200.0, true), (200.0 + HALF - 2.0, false)] {
        let (from, to) = (Vec2::new(40.0, y), Vec2::new(0.0, y));
        let leg = crate::first_portal_crossing(from, to, size, &portals, None);
        assert_eq!(leg.unwrap().is_some(), through, "the leg at y {y} ends at the plane: {leg:?}");
        assert_eq!(taken(from, to), through, "the transit takes the shot at y {y}");
    }
}

/// Each body uses its own material budget when it has one.
#[test]
fn a_body_budget_overrides_the_room_budget() {
    use ambition_platformer2d_core::frame::thread::PieceBudget;
    use ambition_platformer2d_shared_tangle::portal_passages::{PortalPassages, PortalPieceBudget};
    let glued = |geometry: crate::PortalGeometry, own: Option<PortalPieceBudget>| {
        let mut app = App::new();
        app.insert_resource(PortalTuning { geometry, ..PortalTuning::default() });
        app.init_resource::<crate::PortalCarves>();
        app.init_resource::<PortalPassages>();
        app.add_systems(Update, (crate::publish_portal_apertures, crate::publish_portal_carves, crate::publish_portal_passages).chain());
        wall_pair(&mut app);
        let body = app
            .world_mut()
            .spawn((
                BodyKinematics { material: Default::default(), pos: Vec2::new(22.0, 200.0), vel: Vec2::ZERO, canonical_size: Vec2::splat(16.0), facing: -1.0 },
                PortalTransit { straddling: BLUE, crossed: false },
            ))
            .id();
        if let Some(own) = own {
            app.world_mut().entity_mut(body).insert(own);
        }
        app.update();
        let glues = app.world().resource::<crate::PortalCarves>().glues.len();
        let passages = app.world().resource::<PortalPassages>();
        (glues, passages.budget_for(Some(body), None))
    };
    let composed = crate::PortalGeometry::Composed { max_scale_exponent: 12, max_hops: 3, max_pieces: 24 };
    assert_eq!(glued(crate::PortalGeometry::SingleHop, None), (1, PieceBudget::SINGLE_HOP));
    assert_eq!(glued(composed, None), (1, PieceBudget { max_hops: 3, max_pieces: 24, max_scale_exponent: 12 }));
    assert_eq!(glued(composed, Some(PortalPieceBudget(PieceBudget::SINGLE_HOP))), (1, PieceBudget::SINGLE_HOP));
    assert_eq!(glued(crate::PortalGeometry::SingleHop, Some(PortalPieceBudget(PieceBudget::COMPOSED))), (1, PieceBudget::COMPOSED));
}

#[test]
fn passage_publication_keeps_the_recorded_motion_in_both_directions() {
    use ambition_platformer2d_shared_tangle::portal_passages::PortalPassages;
    let mut app=App::new();
    app.init_resource::<PortalPassages>();
    app.add_systems(Update,crate::publish_portal_apertures);
    let [a,b]=wall_pair(&mut app);
    {
        let mut portal=app.world_mut().get_mut::<PlacedPortal>(a).unwrap();
        portal.step.origin=portal.pos-Vec2::new(20.0,0.0);
        portal.step.displacement=Vec2::new(20.0,0.0);
        portal.vel=Vec2::new(1200.0,0.0);
    }
    {
        let mut portal=app.world_mut().get_mut::<PlacedPortal>(b).unwrap();
        portal.step.turn=std::f32::consts::TAU*2.0;
        portal.pivot=portal.pos;
        portal.spin=portal.step.turn*60.0;
    }
    app.update();
    let passages=app.world().resource::<PortalPassages>();
    let links=passages.links_in(None);
    assert_eq!(links.len(),2);
    assert_eq!(links[0].enter_step,links[1].exit_step);
    assert_eq!(links[0].exit_step,links[1].enter_step);
    assert_eq!(links[0].enter_step.displacement,Vec2::new(20.0,0.0));
    assert_eq!(links[0].exit_step.turn,std::f32::consts::TAU*2.0);
    let sample=links[0].snapshot_at(0.125).unwrap();
    assert!(sample.enter.frame.origin.distance(links[0].enter_step.origin+Vec2::new(2.5,0.0))<1.0e-5);
    assert!(sample.exit.frame.normal.distance(links[0].exit.frame.tangent())<1.0e-5);
}


/// A body wider than the opening retains its host support.
#[test]
fn a_body_wider_than_the_opening_does_not_open_the_host() {
    let p = PlacedPortal::fixed(BLUE, Vec2::new(500.0, 300.0), -Vec2::Y, HALF).unwrap();
    let half = Vec2::new(46.5, 30.0);
    let body = ae::Aabb::new(Vec2::new(500.0, 270.5), half);
    assert!(!crate::transit::body_opens(&p, body, body.center(), Vec2::Y * 30.0, 60.0));
    assert!(!crate::portal_fits(half * 2.0, &p));
    let hole = crate::pieces::carve_face_with_depth(&p.aperture(), 60.0).bounds();
    assert_eq!(hole.max.x - hole.min.x, HALF * 2.0);
}

#[test]
fn a_carried_vector_is_mapped_atomically_for_both_body_storage_types() {
    use ambition_platformer2d_shared_tangle::frame_env::CarriedAcceleration;
    for refused in [false, true] {
        let mut app = transit_app();
        let [enter, exit] = wall_pair(&mut app);
        let mut portal = app.world_mut().get_mut::<PlacedPortal>(exit).unwrap();
        let pos = portal.pos;
        portal.place_at(pos, Vec2::ONE.normalize()).unwrap();
        drop(portal);
        let kin = BodyKinematics {
            pos: Vec2::new(19.0, 200.0), vel: -Vec2::X * 400.0,
            canonical_size: Vec2::splat(8.0), ..Default::default()
        };
        let force = if refused { Vec2::splat(f32::MAX) } else { Vec2::new(-100.0, 20.0) };
        let record = SweepSample { prev: kin.pos + Vec2::X * 10.0, vel: kin.vel, ..SweepSample::at_rest(kin, Vec2::Y) };
        let kernel = app.world_mut().spawn((kin, record, CarriedAcceleration(force))).id();
        let free = app.world_mut().spawn((
            physical_body(kin.pos, kin.vel, Some((record.prev, kin.pos))), CarriedAcceleration(force),
        )).id();
        let map = ae::frame::Sim2::through(
            &app.world().get::<PlacedPortal>(enter).unwrap().aperture(),
            &app.world().get::<PlacedPortal>(exit).unwrap().aperture(),
            app.world().resource::<PortalTuning>().convention.map_convention()).unwrap();
        app.update();
        for entity in [kernel, free] {
            let pos = if entity == kernel { app.world().get::<BodyKinematics>(entity).unwrap().pos }
                else { app.world().get::<BodyKinematics>(entity).unwrap().pos };
            let actual = app.world().get::<CarriedAcceleration>(entity).unwrap().0;
            if refused {
                assert_eq!(pos, kin.pos, "a refused payload moved its body");
                assert_eq!(actual, force);
                assert!(app.world().get::<crate::PortalTransitCooldown>(entity).is_none());
            } else {
                assert!((pos - map.point(kin.pos).unwrap()).length() < 1.0e-4);
                assert_eq!(actual, map.vector(force).unwrap());
                assert_ne!(actual, force);
            }
        }
        let events = app.world_mut().resource_mut::<Messages<crate::PortalBodyTransited>>().drain().collect::<Vec<_>>();
        assert_eq!(events.len(), if refused { 0 } else { 2 });
    }
}

#[test]
fn an_approaching_body_prepares_both_charts_before_begin() {
    let mut app = App::new();
    app.init_resource::<crate::PortalCarves>();
    app.add_systems(Update, crate::publish_portal_carves);
    wall_pair(&mut app);
    let kernel = app.world_mut().spawn(BodyKinematics {
        pos: Vec2::new(70.0, 200.0), vel: Vec2::new(-4000.0, 0.0),
        canonical_size: Vec2::splat(16.0), ..Default::default()
    }).id();
    let free = app.world_mut().spawn(physical_body(Vec2::new(70.0, 230.0), Vec2::new(-4000.0, 0.0), None)).id();
    let bystander = app.world_mut().spawn(BodyKinematics {
        pos: Vec2::new(600.0, 200.0), ..Default::default()
    }).id();
    app.update();
    let holes = &app.world().resource::<crate::PortalCarves>().holes;
    assert_eq!(holes.len(), 2);
    for hole in holes {
        assert!(hole.open_for.contains(&kernel));
        assert!(hole.open_for.contains(&free));
        assert!(!hole.open_for.contains(&bystander));
    }
    for body in [kernel, free] {
        assert!(app.world().get::<PortalTransit>(body).is_none());
    }
}

#[test]
fn authored_directions_define_valid_stationary_frames_at_every_sample() {
    use ae::frame::{MapConvention, Sim2};
    for normal in [Vec2::new(0.174, 0.985), Vec2::splat(f32::MAX), Vec2::new(f32::from_bits(1), 0.0)] {
        let portal = PlacedPortal::fixed(BLUE, Vec2::new(400.0, 1123.2), normal, HALF).unwrap();
        let other = PlacedPortal::fixed(ORANGE, Vec2::ZERO, Vec2::X, HALF).unwrap();
        for time in [0.0, 0.5, 1.0] {
            let sampled = portal.step.sample(portal.aperture(), time).unwrap();
            let map = Sim2::through(&sampled, &other.aperture(), MapConvention::Reflection).unwrap();
            assert!(map.point(portal.pos).unwrap().distance(other.pos) < 0.001);
            assert!(sampled.frame.normal.distance(portal.normal) < 0.00001);
        }
    }
}

#[test]
fn invalid_placement_preserves_the_live_pose_and_motion() {
    use ae::frame::NumericFault;
    let mut portal = PlacedPortal::fixed(BLUE, Vec2::new(3.0, 5.0), Vec2::X, HALF).unwrap();
    portal.vel = Vec2::new(10.0, 20.0);
    portal.spin = 2.0;
    portal.step.turn = 0.5;
    portal.step.displacement = Vec2::Y;
    let before = portal.clone();
    for (pos, normal) in [(Vec2::ZERO, Vec2::ZERO), (Vec2::splat(f32::NAN), Vec2::Y), (Vec2::ZERO, Vec2::splat(f32::INFINITY))] {
        assert!(portal.place_at(pos, normal).is_err());
        assert_eq!(portal.pos, before.pos);
        assert_eq!(portal.normal, before.normal);
        assert_eq!(portal.vel, before.vel);
        assert_eq!(portal.spin, before.spin);
        assert_eq!(portal.step, before.step);
        assert_eq!(portal.pivot, before.pivot);
    }
    for length in [0.0, -1.0, f32::INFINITY, f32::NAN] {
        assert!(PlacedPortal::fixed(BLUE, Vec2::ZERO, Vec2::X, length).is_err());
    }
    assert_eq!(PlacedPortal::fixed(BLUE, Vec2::ZERO, Vec2::ZERO, HALF).unwrap_err(), NumericFault::InvalidFrame);
}
