//! OW1: a portal pair is two portals of ONE live room. Each witness puts two
//! live rooms side by side with portals at the same coordinates, and checks
//! that a body, a carve, an eviction and a link group see only their own room.

use super::*;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, RoomInstanceRoot};

use crate::color::{PortalChannel, PortalChannelColor, PortalGunColor};
use crate::link::{link_hash, resolve_portal_links, PortalLink};
use crate::types::portal_half_extent;

const BLUE: PortalChannel = PortalChannel::Gun(PortalGunColor::BLUE);
const ORANGE: PortalChannel = PortalChannel::Gun(PortalGunColor::ORANGE);

/// An app with two live room roots. The two live rooms are returned.
fn two_rooms() -> (App, [LiveRoomInstance; 2]) {
    let mut app = App::new();
    let live = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
    for room in live {
        app.world_mut().spawn((RoomInstanceRoot, room));
    }
    (app, live)
}

/// The wall pair of the transit witnesses: blue at x=20 facing +x, orange at
/// x=380 facing -x.
fn wall(channel: PortalChannel) -> PlacedPortal {
    let (x, normal) = if channel == BLUE {
        (20.0, Vec2::new(1.0, 0.0))
    } else {
        (380.0, Vec2::new(-1.0, 0.0))
    };
    PlacedPortal::fixed(channel, Vec2::new(x, 200.0), normal, portal_half_extent(Vec2::new(1.0, 0.0)))
}

/// A body at the blue portal, moving into it.
fn into_blue() -> BodyKinematics {
    BodyKinematics {
        pos: Vec2::new(20.0, 200.0),
        vel: Vec2::new(-100.0, 0.0),
        size: Vec2::new(24.0, 40.0),
        facing: -1.0,
    }
}

fn transit_app() -> (App, [LiveRoomInstance; 2]) {
    let (mut app, live) = two_rooms();
    app.add_message::<crate::PortalBodyEntered>();
    app.add_message::<crate::PortalBodyTransited>();
    app.init_resource::<crate::PortalTuning>();
    app.add_systems(Update, crate::portal_transit);
    (app, live)
}

fn x_of(app: &App, body: Entity) -> f32 {
    app.world().get::<BodyKinematics>(body).unwrap().pos.x
}

/// A body crosses only a pair of its own live room. The pair is in #1; a
/// body in #1 and a body in #0 stand at the blue portal. The body in #1 comes
/// out at the orange; the body in #0 meets no portal and stays. Before, both
/// crossed: the core paired and transited with no room filter.
#[test]
fn a_body_crosses_only_a_pair_of_its_own_live_room() {
    let (mut app, live) = transit_app();
    for channel in [BLUE, ORANGE] {
        app.world_mut().spawn((wall(channel), InRoomInstance(live[1])));
    }
    let own = app.world_mut().spawn((into_blue(), InRoomInstance(live[1]))).id();
    let other = app.world_mut().spawn((into_blue(), InRoomInstance(live[0]))).id();
    // Frame 1 begins (the leading edge is in the opening), frame 2 transfers.
    app.update();
    app.update();
    assert!(x_of(&app, own) > 250.0, "the body in #1 crosses the pair of #1: x={}", x_of(&app, own));
    assert!(x_of(&app, other) < 100.0, "the body in #0 crosses a pair of #1: x={}", x_of(&app, other));
}

/// A blue in #0 and an orange in #1 are not a pair: a body at the blue in #0
/// does not come out in #1. Before, a channel's partner was found in every
/// room.
#[test]
fn a_blue_and_an_orange_in_two_rooms_are_not_a_pair() {
    let (mut app, live) = transit_app();
    app.world_mut().spawn((wall(BLUE), InRoomInstance(live[0])));
    app.world_mut().spawn((wall(ORANGE), InRoomInstance(live[1])));
    let body = app.world_mut().spawn((into_blue(), InRoomInstance(live[0]))).id();
    app.update();
    app.update();
    assert!(x_of(&app, body) < 100.0, "a split pair carried the body: x={}", x_of(&app, body));
}

/// The rooms of the carves cut while a body in `body_room` stands in the
/// blue portal of a pair in #0.
fn carve_rooms(body_room: usize) -> Vec<PortalRoom> {
    let (mut app, live) = two_rooms();
    app.init_resource::<crate::PortalCarves>();
    app.add_systems(Update, crate::publish_portal_carves);
    for channel in [BLUE, ORANGE] {
        app.world_mut().spawn((wall(channel), InRoomInstance(live[0])));
    }
    app.world_mut().spawn((into_blue(), InRoomInstance(live[body_room])));
    app.update();
    app.world().resource::<crate::PortalCarves>().holes.iter().map(|(room, _)| *room).collect()
}

/// A carve is cut for a body of the pair's own room, and names that room. A
/// body in #1 at the same coordinates cuts nothing. Before, it opened the
/// wall of the pair in #0.
#[test]
fn a_carve_is_cut_only_for_a_body_of_the_pairs_room() {
    let (_, live) = two_rooms();
    assert_eq!(carve_rooms(0), vec![Some(live[0])], "control: the body in #0 opens its pair's wall");
    assert_eq!(carve_rooms(1), Vec::<PortalRoom>::new(), "a body in #1 opened the wall of #0");
}

/// A closing portal evicts only the bodies of its own room. The blue of #1
/// vanishes under two straddlers, one in each room. The one in #1 is pushed
/// clear; the one in #0 straddles no portal of its room and stays. Before,
/// both were pushed.
#[test]
fn a_closing_portal_evicts_only_the_bodies_of_its_room() {
    let (mut app, live) = two_rooms();
    app.init_resource::<crate::PortalFrameHistory>();
    app.add_systems(Update, crate::evict_straddlers_on_portal_change);
    let floor = |pos| PlacedPortal::fixed(BLUE, pos, Vec2::new(0.0, -1.0), portal_half_extent(Vec2::new(0.0, -1.0)));
    let closing = app.world_mut().spawn((floor(Vec2::new(500.0, 300.0)), InRoomInstance(live[1]))).id();
    // #0 has its own blue elsewhere, which does not move.
    app.world_mut().spawn((floor(Vec2::new(100.0, 300.0)), InRoomInstance(live[0])));
    let straddler = |room| {
        let body = BodyKinematics {
            pos: Vec2::new(500.0, 290.0),
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        };
        (body, InRoomInstance(room))
    };
    let own = app.world_mut().spawn(straddler(live[1])).id();
    let other = app.world_mut().spawn(straddler(live[0])).id();
    app.update();
    app.world_mut().entity_mut(closing).despawn();
    app.update();
    let pos = |body| app.world().get::<BodyKinematics>(body).unwrap().pos;
    assert!(pos(own).y + 20.0 <= 300.0 + 1e-3, "the straddler in #1 is pushed clear: {:?}", pos(own));
    assert_eq!(pos(other), Vec2::new(500.0, 290.0), "the body in #0 was pushed by a portal of #1");
}

/// One link name in two rooms is two pairs, one in each room. Before, the
/// four ends were one group of four, and a group that is not two ends is
/// closed: no door opened in either room.
#[test]
fn one_link_in_two_rooms_is_a_pair_in_each() {
    let (mut app, live) = two_rooms();
    app.add_systems(Update, resolve_portal_links);
    let floor = |x| {
        PlacedPortal::fixed(
            PortalChannel::Authored(PortalChannelColor::Indexed(0)),
            Vec2::new(x, 300.0),
            Vec2::new(0.0, -1.0),
            portal_half_extent(Vec2::new(0.0, -1.0)),
        )
    };
    let ends = live.map(|room| {
        [100.0, 500.0].map(|x| {
            app.world_mut()
                .spawn((PortalLink(link_hash("door")), floor(x), InRoomInstance(room)))
                .id()
        })
    });
    app.update();
    for (index, [a, b]) in ends.into_iter().enumerate() {
        let channel = |end| app.world().get::<PlacedPortal>(end).unwrap().channel;
        assert_eq!(channel(a).partner(), channel(b), "the two ends in #{index} are not partners");
    }
}
