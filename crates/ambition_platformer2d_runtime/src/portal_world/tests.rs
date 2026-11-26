use bevy::prelude::*;

use ambition_platformer2d_core::{self as ae, BodyKinematics, RoomGeometry};
use ambition_portal2d::{
    PlacedPortal, PortalCarves, PortalChannel, PortalChannelColor, PortalHost,
    PortalHostDepthsByRoom,
};

const PURPLE: PortalChannel = PortalChannel::Authored(PortalChannelColor::Purple);
const YELLOW: PortalChannel = PortalChannel::Authored(PortalChannelColor::Yellow);

/// CC6 end-to-end: a portal placed on an identified moving platform attaches
/// to its host face and RIDES it — attach attributes the `GeoFaceRef`, refresh
/// re-derives the aperture pose each frame from the platform's live block,
/// records the aperture's own sweep sample, and derives px/s velocity. An
/// unattributable portal (anon fixture wall) stays byte-static.
#[test]
fn a_portal_on_a_moving_platform_rides_its_host_face() {
    use super::hosts::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    // Authored base: one anon fixture wall (unattributable on purpose).
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new(
            "cc6",
            Vec2::new(2000.0, 1000.0),
            Vec2::ZERO,
            vec![ae::Block::solid(
                "anon-wall",
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 400.0),
            )],
        )),
    );
    // One moving platform sweeping +x at 120 px/s — its collision block
    // carries the placement GeoId + per-tick displacement.
    let mut platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    platform.update(1.0 / 60.0); // publish last_delta (2px)
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.add_systems(
        Update,
        (attach_portal_hosts, refresh_hosted_portal_frames).chain(),
    );

    // A floor portal ON the platform's top face (placed 2px proud, like the
    // gun does), and a second portal on the anon wall.
    let plat_top = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].pos.y
        - ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].size.y * 0.5;
    let plat_x = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world()).expect("the live room has moving platforms").0[0].pos.x;
    let hosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            PURPLE,
            Vec2::new(plat_x, plat_top - 2.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();
    let unhosted = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            YELLOW,
            Vec2::new(22.0, 200.0),
            Vec2::new(1.0, 0.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();

    app.update(); // attach + first refresh

    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    let lift = p.host.face().map(|(_, lift)| lift);
    assert!(lift.is_some(), "portal on identified platform attaches");
    assert!(
        (lift.unwrap() - 2.0).abs() < 1e-3,
        "authored lift preserved, got {lift:?}"
    );
    let u = app.world().get::<PlacedPortal>(unhosted).unwrap().clone();
    assert_eq!(u.host, PortalHost::Static, "anon fixture wall cannot host");
    assert_eq!(u.pos, Vec2::new(22.0, 200.0));
    assert!(!u.moved_this_tick());

    // Advance the platform one frame (+2px at 120 px/s / 60fps) and refresh:
    // the aperture rides, its frame delta and px/s velocity match the host.
    let before = p.pos;
    {
        let mut set = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the live room has moving platforms");
        set.0[0].update(1.0 / 60.0);
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(hosted).unwrap().clone();
    let ridden = p.pos - before;
    assert!(
        (ridden.x - 2.0).abs() < 1e-3 && ridden.y.abs() < 1e-3,
        "aperture rides the host displacement, moved {ridden:?}"
    );
    assert_eq!(p.pos - p.step.origin, ridden, "the aperture's own sweep sample");
    assert_eq!(p.step.displacement,Vec2::new(2.0,0.0));
    assert_eq!(p.step.turn,0.0);
    assert!(
        (p.vel.x - 120.0).abs() < 1e-3,
        "px/s velocity derived from the host block, got {:?}",
        p.vel
    );

    // The host face vanishing closes the portal.
    ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut()).expect("the live room has moving platforms")
        .0
        .clear();
    app.update();
    assert!(
        app.world().get::<PlacedPortal>(hosted).is_none(),
        "a portal cannot outlive its host face"
    );
    assert!(
        app.world().get::<PlacedPortal>(unhosted).is_some(),
        "unhosted portals are unaffected"
    );
}

/// A portal on a plank that spins rides the plank's line: each tick it takes
/// the line's point and normal, and the motion the plank published, its
/// turn per second about its center. That motion is what a body crossing it
/// is carried by.
#[test]
fn a_portal_on_a_spinning_plank_turns_with_it_and_carries_its_spin() {
    use super::hosts::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank, PlatformTurn};

    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("plank", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    let mut plank = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(200.0, 8.0), 0.0, 0.0);
    plank.plank = Some(Plank { turn: PlatformTurn::Spin { deg_per_s: 30.0 }, ..Plank::default() });
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![plank]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.add_systems(Update, (attach_portal_hosts, refresh_hosted_portal_frames).chain());
    let portal = app
        .world_mut()
        .spawn(PlacedPortal::fixed(
            PURPLE,
            Vec2::new(440.0, 500.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap())
        .id();
    app.update();
    let p = app.world().get::<PlacedPortal>(portal).unwrap().clone();
    assert!(matches!(p.host, PortalHost::Line { .. }), "rides the plank's line: {:?}", p.host);

    {
        let mut set = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(app.world_mut())
            .expect("the live room has moving platforms");
        set.0[0].update(1.0 / 60.0);
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(portal).unwrap().clone();
    let spin = 30f32.to_radians();
    assert!((p.spin - spin).abs() < 1e-3, "the plank's turn per second: {}", p.spin);
    assert!(p.pivot.distance(Vec2::new(400.0, 500.0)) < 1e-3, "about its center: {:?}", p.pivot);
    let turned = p.step.normal.angle_to(p.normal);
    assert!((turned - spin / 60.0).abs() < 1e-4, "turned with it this tick: {turned}");
    let line=ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component::<MovingPlatformSet>(app.world())
        .unwrap().0[0].as_collision_line().unwrap();
    assert_eq!(p.step.turn,line.turn);
    assert_eq!(p.step.displacement,line.velocity);
    let half_step=ae::SurfaceChain {velocity:line.velocity*0.5,turn:line.turn*0.5,
        pivot:line.pivot-line.velocity*0.5,..line};
    let middle=p.step.sample(p.aperture(),0.5).unwrap();
    assert!(middle.frame.origin.distance(half_step.carried(p.step.origin))<0.001);
    // Still 40 px along the line from its center.
    assert!((p.pos.distance(p.pivot) - 40.0).abs() < 0.5, "{:?}", p.pos);
}

/// A portal on a line whose shape changes (SUP1's `shape_before`) keeps its
/// material place. The line stretches from x 0..100 to x 0..200; the portal
/// three quarters along goes from x 75 to x 150, and its step is that point's
/// travel. A distance along the line stayed at x 75.
#[test]
fn a_portal_on_a_stretching_line_keeps_its_material_place() {
    use super::hosts::{attach_portal_hosts, refresh_hosted_portal_frames};

    let mut world = ae::World::new("stretch", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![]);
    world.chains.push(ae::SurfaceChain::open("stretch", vec![Vec2::new(0.0, 300.0), Vec2::new(100.0, 300.0)]));
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), RoomGeometry(world));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.add_systems(Update, (attach_portal_hosts, refresh_hosted_portal_frames).chain());
    let portal = app
        .world_mut()
        .spawn(PlacedPortal::fixed(PURPLE, Vec2::new(75.0, 300.0), Vec2::new(0.0, -1.0), 10.0).unwrap())
        .id();
    app.update();
    let host = app.world().get::<PlacedPortal>(portal).unwrap().host.clone();
    assert!(matches!(host, PortalHost::Line { segment: 0, fraction, .. } if (fraction - 0.75).abs() < 1e-6), "{host:?}");

    {
        let mut geometry = ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<RoomGeometry>(app.world_mut())
            .expect("the live room has geometry");
        let chain = &mut geometry.0.chains.to_mut()[0];
        chain.shape_before = chain.points.clone();
        chain.points[1].x = 200.0;
    }
    app.update();
    let p = app.world().get::<PlacedPortal>(portal).unwrap().clone();
    assert!(p.pos.distance(Vec2::new(150.0, 300.0)) < 1e-4, "at its material place: {:?}", p.pos);
    assert!(p.step.displacement.distance(Vec2::new(75.0, 0.0)) < 1e-4, "its own point's travel: {:?}", p.step.displacement);
    assert!(p.step.origin.distance(Vec2::new(75.0, 300.0)) < 1e-4, "from where it was: {:?}", p.step.origin);
    assert!(p.spin.abs() < 1e-6 && p.step.turn.abs() < 1e-6, "a stretch does not turn it");
}

#[test]
fn a_wrapping_host_closes_its_old_plane_without_sweeping_the_wrap() {
    use super::hosts::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank, PlatformTurn};

    for spinning in [false, true] {
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            RoomGeometry(ae::World::new("wrap", Vec2::splat(1000.0), Vec2::ZERO, vec![])),
        );
        let mut host = MovingPlatformState::from_vertical_loop(
            "wrap", "wrap", Vec2::new(400.0, 499.0), Vec2::new(200.0, 20.0),
            100.0, 500.0, 120.0, true,
        );
        if spinning {
            host.plank = Some(Plank {
                turn: PlatformTurn::Spin { deg_per_s: 30.0 },
                ..Plank::default()
            });
        }
        let start = host.pos + Vec2::new(40.0, if spinning { 0.0 } else { -10.0 });
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(), MovingPlatformSet(vec![host]),
        );
        app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
        app.init_resource::<ambition_portal2d::PortalFrameHistory>();
        app.add_systems(Update, (
            attach_portal_hosts,
            refresh_hosted_portal_frames,
            ambition_portal2d::evict_straddlers_on_portal_change,
        ).chain());
        let portal = app.world_mut().spawn(PlacedPortal::fixed(
            PURPLE, start, Vec2::NEG_Y, ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap()).id();
        let body_start = start - Vec2::Y * 2.0;
        let body = app.world_mut().spawn(BodyKinematics {
            pos: body_start, canonical_size: Vec2::new(24.0, 40.0),
            vel: Vec2::ZERO, facing: 1.0, material: Default::default(),
        }).id();
        app.update();
        assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos, body_start);

        ambition_platformer2d_shared_tangle::lifecycle::sole_live_room_component_mut::<MovingPlatformSet>(
            app.world_mut(),
        ).unwrap().0[0].update(1.0 / 60.0);
        app.update();

        let p = app.world().get::<PlacedPortal>(portal).unwrap();
        assert!(p.host.is_hosted());
        assert_eq!(p.step.displacement, Vec2::new(0.0, 2.0));
        let continuous_start = p.step.sample(p.aperture(), 0.0).unwrap();
        assert!(continuous_start.frame.origin.distance(start - Vec2::Y * 400.0) < 0.001);
        assert!(continuous_start.frame.normal.distance(Vec2::NEG_Y) < 0.00001);
        assert!(p.step.sample(p.aperture(), 0.5).unwrap().frame.origin.y < 110.0);
        let evicted = app.world().get::<BodyKinematics>(body).unwrap();
        assert!((evicted.pos.y - (start.y - 21.0)).abs() < 0.001);
    }
}

/// OW1: a portal finds its host face in its OWN live room. A moving platform
/// is in #1 only; a portal stamped #1 on its top attaches to it, and a portal
/// stamped #0 at the same place finds no face and is `Static`. Before, the
/// adapter read the sole live room, so while two rooms were live it did not
/// look at all, and both portals stayed unattributed.
#[test]
fn a_portal_finds_its_host_face_in_its_own_live_room() {
    use super::hosts::attach_portal_hosts;
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    let platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    let top = platform.pos.y - platform.size.y * 0.5;
    let x = platform.pos.x;
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in live {
        let geometry = RoomGeometry(ae::World::new("ow1", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![]));
        let mut root = app.world_mut().spawn((RoomInstanceRoot, room, geometry));
        if room == live[1] {
            root.insert(MovingPlatformSet(vec![platform.clone()]));
        }
    }
    app.add_systems(Update, attach_portal_hosts);
    let portal = |room| {
        (
            PlacedPortal::fixed(
                YELLOW,
                Vec2::new(x, top - 2.0),
                Vec2::new(0.0, -1.0),
                ambition_portal2d::PORTAL_OPENING_HALF,
            ).unwrap(),
            InRoomInstance(room),
        )
    };
    let on_platform = app.world_mut().spawn(portal(live[1])).id();
    let elsewhere = app.world_mut().spawn(portal(live[0])).id();
    app.update();
    let host = |portal| app.world().get::<PlacedPortal>(portal).unwrap().host.clone();
    assert!(host(on_platform).face().is_some(), "the portal in #1 did not attach to #1's platform: {:?}", host(on_platform));
    assert_eq!(host(elsewhere), PortalHost::Static, "the portal in #0 is not static");
}

/// A portal is looked at once. The answer is part of the portal, so a
/// `Static` portal is not attached later, even on a face that could host it.
/// A later scan would see moving platforms in other places than the confirmed
/// timeline did. The unattributed twin at the same place is the control: the
/// face under both of them can host.
#[test]
fn a_portal_the_adapter_has_decided_is_not_scanned_again() {
    use super::hosts::attach_portal_hosts;
    use ambition_platformer2d_world::collision::MovingPlatformSet;
    use ambition_platformer2d_world::platforms::MovingPlatformState;

    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("cc6", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    let platform = MovingPlatformState::from_authored(
        Vec2::new(400.0, 500.0),
        Vec2::new(120.0, 20.0),
        400.0,
        120.0,
    );
    let top = platform.pos.y - platform.size.y * 0.5;
    let x = platform.pos.x;
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.add_systems(Update, attach_portal_hosts);

    let place = |channel| {
        PlacedPortal::fixed(
            channel,
            Vec2::new(x, top - 2.0),
            Vec2::new(0.0, -1.0),
            ambition_portal2d::PORTAL_OPENING_HALF,
        ).unwrap()
    };
    let decided = app
        .world_mut()
        .spawn(PlacedPortal { host: PortalHost::Static, ..place(PURPLE) })
        .id();
    let fresh = app.world_mut().spawn(place(YELLOW)).id();
    app.update();

    assert!(
        app.world().get::<PlacedPortal>(fresh).unwrap().host.face().is_some(),
        "control: the face under both portals can host"
    );
    assert_eq!(
        app.world().get::<PlacedPortal>(decided).unwrap().host,
        PortalHost::Static,
        "a decided portal was scanned again"
    );
}

/// Two live room roots, each with its own overlay, and the carves published
/// this frame, one for each entry of `holes`: the index of the live room it
/// was cut in, or `None` for no live room. The carves each overlay holds
/// after [`super::carves::bridge_portal_carves`] runs, by room.
fn carves_by_room(holes: &[Option<usize>]) -> [usize; 2] {
    use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let mut app = App::new();
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    let roots = live.map(|room| {
        app.world_mut()
            .spawn((RoomInstanceRoot, room, FeatureEcsWorldOverlay::default()))
            .id()
    });
    let body = app.world_mut().spawn_empty().id();
    app.insert_resource(PortalCarves {
        holes: holes
            .iter()
            .map(|room| ambition_portal2d::PortalHole {
                room: room.map(|room| live[room]),
                face: ae::frame::FaceRect {
                    center: Vec2::new(200.0, 390.0),
                    normal: Vec2::new(0.0, -1.0),
                    half_along: 24.0,
                    half_out: 10.0,
                },
                open_for: vec![body],
            })
            .collect(),
        glues: Vec::new(),
    });
    app.add_systems(Update, super::carves::bridge_portal_carves);
    app.update();
    roots.map(|root| {
        app.world()
            .get::<FeatureEcsWorldOverlay>(root)
            .expect("the root keeps its overlay")
            .portal_holes
            .len()
    })
}

/// OW1 cut 7l: with two live rooms, each carve goes to the overlay of the
/// room it was cut in, and a room with no carve stays sealed. When the bridge
/// wrote the sole live room's overlay, no room was carved while two were
/// live; when it wrote the one room all portals were in, a pair in each room
/// carved neither.
#[test]
fn each_carve_goes_to_the_live_room_it_was_cut_in() {
    assert_eq!(carves_by_room(&[Some(1)]), [0, 1], "a carve in #1 goes to #1 only");
    assert_eq!(carves_by_room(&[Some(0)]), [1, 0], "a carve in #0 goes to #0 only");
    assert_eq!(carves_by_room(&[Some(0), Some(1), Some(1)]), [1, 2], "each room keeps its own carves");
    assert_eq!(carves_by_room(&[None]), [0, 0], "a carve in no live room is not written");
}

/// The bridge runs while a carve stands or a room still holds a hole, so the
/// hole of a portal that went is cleared and the wall seals again.
#[test]
fn a_hole_left_after_the_last_carve_is_sealed() {
    use ambition_platformer2d_shared_tangle::feature_overlay::{FeatureEcsWorldOverlay, PortalHolePass};
    use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, RoomInstanceRoot};
    let mut app = App::new();
    let mut overlay = FeatureEcsWorldOverlay::default();
    overlay.portal_holes.push(PortalHolePass {
        face: ae::frame::FaceRect {
            center: Vec2::new(200.0, 390.0),
            normal: Vec2::new(0.0, -1.0),
            half_along: 24.0,
            half_out: 10.0,
        },
        bodies: Vec::new(),
    });
    let root = app.world_mut().spawn((RoomInstanceRoot, LiveRoomInstance::ACTIVATION, overlay)).id();
    app.insert_resource(PortalCarves::default());
    app.add_systems(
        Update,
        super::carves::bridge_portal_carves.run_if(super::carves::carves_or_holes_stand),
    );
    app.update();
    assert!(
        app.world().get::<FeatureEcsWorldOverlay>(root).unwrap().portal_holes.is_empty(),
        "the hole of a portal that went stayed open"
    );
}

/// The host a portal at `at` (facing up) attaches to on `platform`, and the
/// depth the adapter measures behind it.
fn host_and_depth(
    platform: ambition_platformer2d_world::platforms::MovingPlatformState,
    at: Vec2,
) -> (PortalHost, f32) {
    use super::carves::sync_portal_host_depths;
    use super::hosts::{attach_portal_hosts, refresh_hosted_portal_frames};
    use ambition_platformer2d_world::collision::MovingPlatformSet;

    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        app.world_mut(),
        RoomGeometry(ae::World::new("hosts", Vec2::new(2000.0, 1000.0), Vec2::ZERO, vec![])),
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), MovingPlatformSet(vec![platform]));
    app.insert_resource(ambition_time::WorldTime::new(0.0, 1.0 / 60.0));
    app.init_resource::<PortalHostDepthsByRoom>();
    app.add_systems(
        Update,
        (attach_portal_hosts, refresh_hosted_portal_frames, sync_portal_host_depths).chain(),
    );
    let portal = app.world_mut().spawn(PlacedPortal::fixed(PURPLE, at, Vec2::new(0.0, -1.0), 30.0).unwrap()).id();
    app.update();
    let host = app.world().get::<PlacedPortal>(portal).expect("the portal stays on its host").host.clone();
    let depth = app.world().resource::<PortalHostDepthsByRoom>().0.iter().map(|(_, d)| d.depth(PURPLE)).find(|d| d.is_finite());
    (host, depth.expect("the portal is measured"))
}

/// The rescue reaches as deep as the host is: a portal on a moving block is
/// measured through that block where it is, and a portal on a plank (a line
/// with open air under it) has nothing behind it. Before, both read the
/// authored room only, found nothing, and took the 60 px probe, so a body
/// under the plank moving away from its portal was taken through it.
#[test]
fn a_portal_is_as_deep_as_the_host_it_rides() {
    use ambition_platformer2d_world::platforms::{MovingPlatformState, Plank};
    // A block 20 tall with its top at y = 490; the portal 2 px proud of it.
    let block = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(120.0, 20.0), 0.0, 0.0);
    let (host, depth) = host_and_depth(block, Vec2::new(400.0, 488.0));
    assert!(matches!(host, PortalHost::Face { .. }), "{host:?}");
    assert!((depth - 22.0).abs() < 1.0, "through the block from 2 px proud: {depth}");

    let mut plank = MovingPlatformState::from_authored(Vec2::new(400.0, 500.0), Vec2::new(200.0, 8.0), 0.0, 0.0);
    plank.plank = Some(Plank::default());
    let (host, depth) = host_and_depth(plank, Vec2::new(440.0, 500.0));
    assert!(matches!(host, PortalHost::Line { .. }), "{host:?}");
    assert_eq!(depth, 0.0, "nothing is behind a plank");
}
