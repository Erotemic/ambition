//! The three acts are one course. A headless run crosses each goal.
//!
//! Act 2 exercises what Act 1 cannot: ground with height, loops authored as
//! LDtk data (`SurfaceLoop` + `attach_to`), a speed-gated split where a fast
//! runner clears the chasm onto the sky bridge, and an upside-down tunnel
//! where a `GravityZone` points gravity up and the momentum solver rides the
//! ceiling. Its other road and its secrets are `act_two_routes`.

use ambition_demo_sanic::{
    SanicActPhase, SanicActState, DARKNESS_ROOM_ID, HIGHWAY_ROOM_ID, SPEEDWAY_ROOM_ID,
};
use ambition_demo_sanic_app::build_demo_app;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

#[derive(Resource, Default)]
struct PortalTransits(usize);

fn record_portal_transits(
    mut transits: ResMut<PortalTransits>,
    mut events: MessageReader<ambition_platformer2d::portal::PortalBodyTransited>,
) {
    transits.0 += events.read().count();
}

fn body(app: &mut App) -> (ae::BodyKinematics, Option<ae::SurfaceMotion>) {
    let mut q = app
        .world_mut()
        .query_filtered::<(&ae::BodyKinematics, &ae::MotionModel), With<PrimaryPlayer>>();
    q.iter(app.world())
        .next()
        .map(|(kin, model)| {
            let motion = match model {
                ae::MotionModel::SurfaceMomentum(momentum) => Some(momentum.state),
                _ => None,
            };
            (*kin, motion)
        })
        .expect("the demo spawned Sanic")
}

fn act(app: &mut App) -> SanicActState {
    let mut q = app.world_mut().query::<&SanicActState>();
    q.iter(app.world())
        .next()
        .copied()
        .expect("the act owner exists")
}

fn room(app: &mut App) -> (String, Option<usize>, Option<usize>) {
    let mut q = app
        .world_mut()
        .query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let set = q.iter(app.world()).next().expect("the session's room set");
    // Act 2's painted surfaces, found where they are: the tunnel roof's
    // underside (a right → left run, which faces down, over x = 8400) and the
    // sky bridge (a `Track` floor over x = 6000).
    let world = set.active_world();
    let spans = |chain: &ae::SurfaceChain, x: f32, leftward: bool| {
        chain.points.windows(2).any(|p| {
            let (a, b) = if leftward { (p[1], p[0]) } else { (p[0], p[1]) };
            a.x < x && x < b.x
        })
    };
    let tunnel = world.chains.iter().position(|c| {
        c.name.starts_with("terrain:") && spans(c, 8400.0, true) && c.points.iter().all(|p| p.y < 1100.0)
    });
    let bridge = world
        .chains
        .iter()
        .position(|c| c.name.starts_with("track:") && spans(c, 6000.0, false));
    (set.active_spec().id.clone(), tunnel, bridge)
}

fn hold(app: &mut App, jump: bool) {
    let mut frame = ControlFrame::default();
    frame.axis_x = 1.0;
    frame.right_pressed = true;
    frame.jump_pressed = jump;
    frame.jump_held = jump;
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
}

fn release(app: &mut App) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = ControlFrame::default();
}

fn place(app: &mut App, at: Vec2) {
    let mut q = app.world_mut().query_filtered::<(
        ae::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut clusters, mut model) = q.iter_mut(world).next().expect("Sanic exists");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(
        &mut model,
        &mut clusters,
        at,
        ae::movement::TransitVelocity::Zero,
    );
}

/// Spring from `spring_x` and land on the painted high road: a `Track`
/// chain (named `track:…` by the loader) whose floor passes over `road_x`.
fn ride_high_road(app: &mut App, spring_x: f32, spring_y: f32, road_x: f32) {
    let road = format!("the painted high road over x={road_x:.0}");
    let chain = {
        let mut q = app
            .world_mut()
            .query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let set = q.iter(app.world()).next().expect("room set");
        set.active_world()
            .chains
            .iter()
            .position(|chain| {
                chain.name.starts_with("track:")
                    && chain.points.windows(2).any(|p| p[0].x < road_x && road_x < p[1].x)
            })
            .unwrap_or_else(|| panic!("no painted high road over x={road_x}"))
    };
    place(app, Vec2::new(spring_x + 24.0, spring_y - 130.0));
    let mut launched = false;
    for _ in 0..120 {
        release(app);
        app.update();
        if body(app).0.vel.y < -500.0 {
            launched = true;
            break;
        }
    }
    assert!(launched, "spring at {spring_x:.0} must launch Sanic");
    let mut highest = f32::MAX;
    let mut furthest = f32::MIN;
    for _ in 0..480 {
        let (kin, motion) = body(app);
        highest = highest.min(kin.pos.y);
        furthest = furthest.max(kin.pos.x);
        if matches!(motion, Some(ae::SurfaceMotion::Riding { on: ae::SurfaceRef::Chain(on), .. }) if on == chain)
        {
            return;
        }
        hold(app, false);
        app.update();
    }
    panic!("spring at {spring_x:.0} did not reach {road}; highest y {highest:.0}, furthest x {furthest:.0}");
}

/// Ride the vault ceiling right into its rift and come out of the red one on
/// the east high road, standing on it.
///
/// A portal keeps a body's offset along its plane, so a pair centred at
/// different heights off their surfaces sets a runner down inside one. The red
/// rift put him 12 px into the road; the next frame the solver popped him out,
/// and the camera jolted with him (Jon: "the character embeds into the floor
/// before settling back on the surface").
fn through_the_vault_rift(app: &mut App) {
    place(app, Vec2::new(11700.0, 1290.0));
    for _ in 0..20 {
        release(app);
        app.update();
    }
    let transits = app.world().resource::<PortalTransits>().0;
    let mut arrived = None;
    for _ in 0..240 {
        hold(app, false);
        app.update();
        if app.world().resource::<PortalTransits>().0 > transits {
            arrived = Some(body(app).0.pos);
            break;
        }
    }
    let arrived = arrived.expect("riding the vault ceiling right must cross its rift");
    assert!(arrived.x > 21000.0, "the vault rift leads to the east road: {arrived:?}");
    for _ in 0..10 {
        hold(app, false);
        app.update();
    }
    let (kin, motion) = body(app);
    assert!(
        matches!(motion, Some(ae::SurfaceMotion::Riding { .. })),
        "he runs on along the east road: {motion:?}"
    );
    assert!(
        (kin.pos.y - arrived.y).abs() < 0.5,
        "he came out of the red rift at y {:.1} and stands at {:.1}: set down in the road, then popped out",
        arrived.y,
        kin.pos.y
    );
}

/// Run until the act clears, or panic naming how far he got.
fn run_to_clear(
    app: &mut App,
    act_name: &str,
    frames: usize,
    jump_at: impl Fn(f32) -> bool,
) -> usize {
    let mut furthest = f32::MIN;
    for frame in 0..frames {
        let x = body(app).0.pos.x;
        furthest = furthest.max(x);
        hold(app, jump_at(x));
        app.update();
        if matches!(act(app).phase, SanicActPhase::Cleared { .. }) {
            return frame;
        }
    }
    panic!(
        "{act_name}: held Right for {frames} frames and never cleared; furthest x {furthest:.0}"
    );
}

/// Release the stick and wait for the named room to become active.
fn arrive_in(app: &mut App, room_id: &str) {
    for _ in 0..900 {
        release(app);
        app.update();
        if room(app).0 == room_id {
            return;
        }
    }
    panic!(
        "the cleared act never arrived in `{room_id}` (still in `{}`)",
        room(app).0
    );
}

#[test]
fn the_three_acts_connect_and_clear() {
    let mut app = build_demo_app();
    app.init_resource::<PortalTransits>();
    app.add_systems(Last, record_portal_transits);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        room(&mut app).0,
        SPEEDWAY_ROOM_ID,
        "the session enters at Act 1"
    );

    // Act 1. It asks for a jump at its pit, as its own completion proof does.
    run_to_clear(&mut app, "Act 1", 2400, |x| {
        x > ambition_demo_sanic::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::PIT_RIGHT_X
    });
    arrive_in(&mut app, HIGHWAY_ROOM_ID);
    let arrived = act(&mut app);
    assert!(
        matches!(arrived.phase, SanicActPhase::Running) && arrived.elapsed < 0.5,
        "arriving in Act 2 starts a fresh act, not Act 1's results: {arrived:?}"
    );
    assert!(body(&mut app).0.pos.x < 400.0, "and at Act 2's start line");

    // Act 2, Right only: a full-speed runner clears the chasm off the kicker
    // lip onto the sky bridge (the fast road), and rides the tunnel ceiling.
    let (_, tunnel, bridge) = room(&mut app);
    let tunnel = tunnel.expect("the highway authors its tunnel ceiling");
    let bridge = bridge.expect("the highway authors its sky bridge");
    let mut rode_the_ceiling = false;
    let mut rode_the_bridge = false;
    let mut furthest = f32::MIN;
    let mut cleared = false;
    for _ in 0..3000 {
        let (kin, motion) = body(&mut app);
        furthest = furthest.max(kin.pos.x);
        if matches!(motion, Some(ae::SurfaceMotion::Riding { on: ae::SurfaceRef::Chain(chain), .. }) if chain == tunnel)
        {
            rode_the_ceiling = true;
        }
        if matches!(motion, Some(ae::SurfaceMotion::Riding { on: ae::SurfaceRef::Chain(chain), .. }) if chain == bridge)
        {
            rode_the_bridge = true;
        }
        hold(&mut app, false);
        app.update();
        if matches!(act(&mut app).phase, SanicActPhase::Cleared { .. }) {
            cleared = true;
            break;
        }
    }
    assert!(
        cleared,
        "Act 2: held Right and never cleared; furthest x {furthest:.0}"
    );
    assert!(
        rode_the_bridge,
        "at full speed the kicker lip throws him across the chasm onto the sky bridge"
    );
    assert!(
        rode_the_ceiling,
        "inside the tunnel the GravityZone points gravity up, so he must ride its ceiling chain"
    );

    arrive_in(&mut app, DARKNESS_ROOM_ID);
    assert!(
        matches!(act(&mut app).phase, SanicActPhase::Running),
        "the highway's goal leads to a fresh Act 3"
    );
    assert!(
        body(&mut app).0.pos.x < 400.0,
        "Act 3 starts at its west end"
    );
    let mut portals = app
        .world_mut()
        .query::<&ambition_platformer2d::portal::PlacedPortal>();
    assert_eq!(
        portals.iter(app.world()).count(),
        6,
        "all authored portals spawn"
    );
    ride_high_road(&mut app, 5480.0, 1660.0, 6000.0);
    ride_high_road(&mut app, 21500.0, 1640.0, 22000.0);
    through_the_vault_rift(&mut app);
    place(&mut app, Vec2::new(144.0, 1594.0));
    run_to_clear(&mut app, "Act 3", 7200, |_| false);
    assert!(
        app.world().resource::<PortalTransits>().0 > 0,
        "a run through Act 3 must cross a portal, not only pass their art"
    );
    arrive_in(&mut app, SPEEDWAY_ROOM_ID);
    assert!(matches!(act(&mut app).phase, SanicActPhase::Running));
}
