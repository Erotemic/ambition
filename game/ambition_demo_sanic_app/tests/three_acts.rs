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
use ambition_platformer2d::portal::{PortalChannel, PortalChannelColor};
use bevy::prelude::*;

#[derive(Resource, Default)]
struct PortalTransits(Vec<PortalChannel>);

fn record_portal_transits(
    mut transits: ResMut<PortalTransits>,
    mut events: MessageReader<ambition_platformer2d::portal::PortalBodyTransited>,
    players: Query<(), With<PrimaryPlayer>>,
) {
    transits.0.extend(events.read().filter(|event| players.contains(event.body))
        .map(|event| event.transfer.entry_channel));
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
    let room = ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).expect("the session's live room");
    // Act 2's painted surfaces, found where they are: the tunnel roof's
    // underside (a right → left run, which faces down, over x = 8400) and the
    // sky bridge (a `Track` floor over x = 6000).
    let world = &room.world;
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
    (room.id.clone(), tunnel, bridge)
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
        ambition_platformer2d::world::rooms::sole_live_room_spec(app.world())
            .expect("the live room")
            .world
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

fn run_into_portal(app: &mut App, start: Vec2, entrance: PortalChannelColor) -> ae::BodyKinematics {
    place(app, start);
    for _ in 0..20 {
        release(app);
        app.update();
    }
    let transits = app.world().resource::<PortalTransits>().0.len();
    for _ in 0..240 {
        hold(app, false);
        app.update();
        if app.world().resource::<PortalTransits>().0[transits..].contains(&PortalChannel::Authored(entrance)) {
            return body(app).0;
        }
    }
    panic!("Sanic did not cross {entrance:?} from {start:?}: {:?}", body(app));
}

/// Exit through the floor of the east road and stand upright without pressing jump.
fn through_the_vault_rift(app: &mut App) {
    let arrived = run_into_portal(app, Vec2::new(11700.0, 1290.0), PortalChannelColor::Teal);
    assert!(arrived.pos.x > 21000.0 && arrived.vel.y < 0.0,
        "the vault rift launches Sanic above the east road: {arrived:?}");
    for _ in 0..120 {
        let (kin, motion) = body(app);
        if matches!(motion, Some(ae::SurfaceMotion::Riding { .. }))
            && kin.pos.x > 22080.0 && kin.pos.y < 1160.0
            && kin.material.render_angle().abs() < 0.01
        {
            return;
        }
        hold(app, false);
        app.update();
    }
    panic!("Sanic did not land upright on the east road: {:?}", body(app));
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
        x > ambition_demo_sanic::speedway_oracle::PIT_LEFT_X - 220.0 && x < ambition_demo_sanic::speedway_oracle::PIT_RIGHT_X
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
        "Act 3 starts at its west end: body={:?}, spawn={:?}",
        body(&mut app),
        ambition_platformer2d::world::rooms::sole_live_room_spec(app.world()).unwrap().world.spawn
    );
    for _ in 0..3 {
        app.update();
    }
    let mut portals = app
        .world_mut()
        .query::<&ambition_platformer2d::portal::PlacedPortal>();
    let placed: Vec<_> = portals.iter(app.world()).collect();
    assert_eq!(placed.len(), 6, "all authored portals spawn");
    for portal in placed {
        assert!(matches!(portal.host, ambition_platformer2d::portal::PortalHost::Face { .. }),
            "portal at {:?} did not attach: {:?}", portal.pos, portal.host);
    }
    ride_high_road(&mut app, 5480.0, 1660.0, 6000.0);
    ride_high_road(&mut app, 21500.0, 1640.0, 21800.0);
    let viaduct_exit = run_into_portal(&mut app, Vec2::new(7660.0, 1740.0), PortalChannelColor::Purple);
    assert!(viaduct_exit.pos.x > 10560.0, "{viaduct_exit:?}");
    through_the_vault_rift(&mut app);
    let shadow_exit = run_into_portal(&mut app, Vec2::new(27580.0, 1712.0), PortalChannelColor::Green);
    assert!(shadow_exit.pos.x > 29000.0 && shadow_exit.vel.y < 0.0, "{shadow_exit:?}");
    let shadow_return = run_into_portal(&mut app, Vec2::new(29330.0, 1400.0), PortalChannelColor::Magenta);
    assert!(shadow_return.pos.x < 27680.0, "{shadow_return:?}");
    place(&mut app, Vec2::new(144.0, 1594.0));
    let transits = app.world().resource::<PortalTransits>().0.len();
    run_to_clear(&mut app, "Act 3", 3600, |_| false);
    assert!(app.world().resource::<PortalTransits>().0[transits..].contains(&PortalChannel::Authored(PortalChannelColor::Teal)),
        "a run along the vault ceiling must use its portal");
    arrive_in(&mut app, SPEEDWAY_ROOM_ID);
    assert!(matches!(act(&mut app).phase, SanicActPhase::Running));
}
