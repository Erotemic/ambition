//! What every body that crosses a portal can rely on, whatever stores it.
//!
//! A free body here is [`Probe`], a component that is its own body, as a
//! thrown item is: the arms run the systems a composition registers for one.

use bevy::prelude::*;

use ambition_platformer2d_core::{AabbExt, BodyKinematics, SweepSample};

use crate::color::{PortalChannel, PortalGunColor};
use crate::placement::{transit_step_with_tuning, TransitStep};
use crate::transit::{FreePortalBody, PortalTransit};
use crate::tuning::PortalTuning;
use crate::types::{PlacedPortal, PortalHostDepths};

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);
const HALF: f32 = crate::types::PORTAL_OPENING_HALF;

/// A free body: where it is, how it moves, its last step, and whether
/// something holds it.
#[derive(Component, Clone, Copy, Debug)]
struct Probe {
    pos: Vec2,
    vel: Vec2,
    step: Option<(Vec2, Vec2)>,
}

#[derive(Component)]
struct Held;

impl FreePortalBody for Probe {
    fn pos(&self) -> Vec2 {
        self.pos
    }
    fn vel(&self) -> Vec2 {
        self.vel
    }
    fn half_extent(&self) -> Vec2 {
        Vec2::splat(8.0)
    }
    fn emerge(&mut self, pos: Vec2, vel: Vec2) {
        self.pos = pos;
        self.vel = vel;
        self.step = None;
    }
    fn last_step(&self) -> Option<(Vec2, Vec2)> {
        self.step
    }
    type Presence = Held;
    fn present(held: Option<&Held>) -> bool {
        held.is_none()
    }
}

/// A wall pair: blue at x = 20 facing +x, orange at x = 380 facing -x.
fn wall_pair(app: &mut App) -> [Entity; 2] {
    [
        app.world_mut()
            .spawn(PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF))
            .id(),
        app.world_mut()
            .spawn(PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF))
            .id(),
    ]
}

fn transit_app() -> App {
    let mut app = App::new();
    app.init_resource::<PortalTuning>();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.add_systems(Update, (crate::portal_transit, crate::portal_transit_free_bodies::<Probe>));
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
            Probe { pos: at, vel: Vec2::new(-300.0, 0.0), step: None },
            PortalTransit { straddling: BLUE, crossed: false },
            Held,
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<Probe>(body).unwrap().pos, at, "a held body was moved by the pair");
    assert!(app.world().get::<PortalTransit>(body).is_none(), "a held body still straddles");

    // Control: the same body in the world crosses.
    app.world_mut().entity_mut(body).remove::<Held>();
    app.world_mut().entity_mut(body).insert(PortalTransit { straddling: BLUE, crossed: false });
    app.update();
    assert!(app.world().get::<Probe>(body).unwrap().pos.x > 300.0, "control: the body did not cross");
}

/// A crossing that skipped `Begin` (one step from in front of the plane to
/// past the hole) leaves the body straddling the exit, crossed, so it is
/// drawn cut there as every crossing is.
#[test]
fn a_crossing_that_skipped_begin_leaves_the_body_straddling_the_exit() {
    let mut app = transit_app();
    wall_pair(&mut app);
    // Kernel: its step began 30 px in front of blue and ends 6 px behind it.
    let kin = BodyKinematics { pos: Vec2::new(14.0, 200.0), vel: Vec2::new(-1800.0, 0.0), size: Vec2::splat(16.0), facing: 1.0 };
    let record = SweepSample { prev: Vec2::new(44.0, 200.0), curr: kin.pos, vel: kin.vel, half: kin.size * 0.5, down: Vec2::new(0.0, 1.0) };
    let kernel = app.world_mut().spawn((kin, record)).id();
    let free = app
        .world_mut()
        .spawn(Probe {
            pos: Vec2::new(14.0, 230.0),
            vel: Vec2::new(-1800.0, 0.0),
            step: Some((Vec2::new(44.0, 230.0), Vec2::new(14.0, 230.0))),
        })
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
    let kin = BodyKinematics { pos: Vec2::new(22.0, 200.0), vel: Vec2::ZERO, size: Vec2::splat(16.0), facing: 1.0 };
    let kernel = app.world_mut().spawn((kin, PortalTransit { straddling: BLUE, crossed: false })).id();
    let free = app
        .world_mut()
        .spawn((
            Probe { pos: Vec2::new(22.0, 230.0), vel: Vec2::ZERO, step: None },
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
        .spawn(Probe {
            pos: Vec2::new(-60.0, 200.0),
            vel: Vec2::new(-2400.0, 0.0),
            step: Some((Vec2::new(40.0, 200.0), Vec2::new(-60.0, 200.0))),
        })
        .id();
    app.update();
    // 80 px into blue is 80 px out of orange, moving out of it.
    let probe = app.world().get::<Probe>(body).unwrap();
    assert!((probe.pos - Vec2::new(300.0, 200.0)).length() < 1e-3, "the step through the opening did not cross: {probe:?}");
    assert_eq!(probe.vel, Vec2::new(-2400.0, 0.0), "momentum was not carried: {probe:?}");

    // Control: the same body with no step is past the hole and is not taken.
    let stale = app
        .world_mut()
        .spawn(Probe { pos: Vec2::new(-60.0, 200.0), vel: Vec2::new(-2400.0, 0.0), step: None })
        .id();
    app.update();
    assert_eq!(app.world().get::<Probe>(stale).unwrap().pos.x, -60.0, "control: a body with no step was taken");
}

/// A floor portal at y = 300 closes under a free body whose centre is 4
/// over its plane: it is pushed clear, as a kernel body is.
#[test]
fn a_closing_plane_pushes_a_free_body_clear() {
    let mut app = App::new();
    app.init_resource::<crate::PortalFrameHistory>();
    app.add_systems(
        Update,
        (crate::evict_free_straddlers_on_portal_change::<Probe>, crate::evict_straddlers_on_portal_change).chain(),
    );
    let closing = app
        .world_mut()
        .spawn(PlacedPortal::fixed(BLUE, Vec2::new(500.0, 300.0), Vec2::new(0.0, -1.0), HALF))
        .id();
    let body = app.world_mut().spawn(Probe { pos: Vec2::new(500.0, 296.0), vel: Vec2::ZERO, step: None }).id();
    app.update();
    app.world_mut().entity_mut(closing).despawn();
    app.update();
    let y = app.world().get::<Probe>(body).unwrap().pos.y;
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
                    app.world_mut().spawn(PlacedPortal::fixed(channel, Vec2::new(500.0, 300.0), normal, HALF)).id()
                })
                .collect();
            let kin = BodyKinematics { pos: Vec2::new(500.0, 300.0), vel: Vec2::ZERO, size: Vec2::splat(16.0), facing: 1.0 };
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

/// A body under a portal on a line (a plank with open air below it) moving
/// away from the portal on top is not taken through it. With a 60 px host
/// behind the plane, the same body is in the hole and is taken: the
/// measured depth is what decides.
#[test]
fn a_body_under_a_portal_on_a_bare_line_is_not_taken() {
    let floor = PlacedPortal::fixed(BLUE, Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0), HALF);
    let exit = PlacedPortal::fixed(ORANGE, Vec2::new(400.0, 100.0), Vec2::new(0.0, -1.0), HALF);
    let portals = [floor, exit];
    let step = |depth: f32| {
        transit_step_with_tuning(
            // Under the plank, its top 4 px below the plane, falling away.
            Vec2::new(100.0, 324.0),
            Vec2::splat(40.0),
            Vec2::new(0.0, 200.0),
            None,
            None,
            None,
            &portals,
            Vec2::new(0.0, 1.0),
            &PortalHostDepths(vec![(BLUE, depth), (ORANGE, depth)]),
            &PortalTuning::default(),
        )
    };
    assert!(matches!(step(60.0), TransitStep::Transfer { .. }), "control: a deep host's hole takes the body");
    assert!(matches!(step(0.0), TransitStep::Idle), "a body under a bare line was taken: {:?}", step(0.0));
}

/// Where a leg enters a paired opening, as a fraction of it; not at all
/// beside the opening or through an unpaired portal.
#[test]
fn a_leg_ends_where_its_centre_enters_a_paired_opening() {
    let blue = PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF);
    let orange = PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF);
    let size = Vec2::splat(16.0);
    let t = crate::first_portal_crossing(Vec2::new(40.0, 200.0), Vec2::new(0.0, 200.0), size, &[blue.clone(), orange.clone()], None);
    assert!(t.is_some_and(|t| (t - 0.5).abs() < 1e-4), "{t:?}");
    let beside = Vec2::new(0.0, 200.0 + HALF + 30.0);
    assert_eq!(crate::first_portal_crossing(beside + Vec2::new(40.0, 0.0), beside, size, &[blue.clone(), orange], None), None);
    assert_eq!(crate::first_portal_crossing(Vec2::new(40.0, 200.0), Vec2::new(0.0, 200.0), size, &[blue], None), None);
}

#[test]
fn a_body_beside_an_opening_does_not_open_it() {
    // A wall portal facing +x whose top end is a ledge's top (y up from the
    // end is past it): a body standing on the ledge, its feet on the end and
    // its box over the face, is beside the opening, not in line with it.
    let p = PlacedPortal::fixed(BLUE, Vec2::new(100.0, 248.0), Vec2::new(1.0, 0.0), 40.0);
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
    let p = PlacedPortal::fixed(BLUE, Vec2::new(100.0, 300.0), Vec2::new(0.0, -1.0), 40.0);
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
            BodyKinematics { pos: Vec2::new(60.0, 200.0), vel: Vec2::ZERO, size: Vec2::splat(16.0), facing: 1.0 },
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
    app.update();
    assert!(app.world().get::<BodyKinematics>(body).unwrap().pos.x > 300.0, "control: the body was not taken");
}

/// A shot's contacts stop where the transit takes it, and nowhere else (review
/// of 15d0cf99c, finding B): one crossing ([`crate::placement::swept_crossing`])
/// decides both. A shot whose centre crosses inside the opening but whose box
/// still cuts the plane past an end is not taken, so its leg is not cut short
/// (it would skip the wall beside the opening and stay in its room); the same
/// leg through the middle is cut where the transit takes it.
#[test]
fn a_shots_leg_ends_only_where_the_transit_takes_it() {
    use crate::placement::SweptSample;
    let blue = PlacedPortal::fixed(BLUE, Vec2::new(20.0, 200.0), Vec2::new(1.0, 0.0), HALF);
    let orange = PlacedPortal::fixed(ORANGE, Vec2::new(380.0, 200.0), Vec2::new(-1.0, 0.0), HALF);
    let portals = [blue, orange];
    let size = Vec2::splat(16.0);
    let vel = Vec2::new(-2400.0, 0.0);
    let taken = |from: Vec2, to: Vec2| {
        matches!(
            transit_step_with_tuning(
                to,
                size,
                vel,
                Some(SweptSample { pos: from, vel }),
                None,
                None,
                &portals,
                Vec2::new(0.0, 1.0),
                &PortalHostDepths::default(),
                &PortalTuning::default(),
            ),
            TransitStep::Transfer { .. }
        )
    };
    for (y, through) in [(200.0, true), (200.0 + HALF - 2.0, false)] {
        let (from, to) = (Vec2::new(40.0, y), Vec2::new(0.0, y));
        let leg = crate::first_portal_crossing(from, to, size, &portals, None);
        assert_eq!(leg.is_some(), through, "the leg at y {y} ends at the plane: {leg:?}");
        assert_eq!(taken(from, to), through, "the transit takes the shot at y {y}");
    }
}
