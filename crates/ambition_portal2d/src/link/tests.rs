use super::*;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance, RoomInstanceRoot};

fn floor(pos: Vec2) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Indexed(0)),
        pos,
        Vec2::new(0.0, -1.0),
        crate::PORTAL_OPENING_HALF,
    )
}

fn app() -> App {
    let mut app = App::new();
    app.add_systems(
        Update,
        (resolve_portal_links, equalize_pair_apertures).chain(),
    );
    app
}

#[test]
fn same_link_pairs_by_position_distinct_partners() {
    let mut app = app();
    let a = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("door")),
            floor(Vec2::new(100.0, 300.0)),
        ))
        .id();
    let b = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("door")),
            floor(Vec2::new(500.0, 300.0)),
        ))
        .id();
    app.update();
    let ca = app.world().get::<PlacedPortal>(a).unwrap().channel;
    let cb = app.world().get::<PlacedPortal>(b).unwrap().channel;
    assert_ne!(ca, cb, "the two ends get distinct channels");
    assert_eq!(ca.partner(), cb, "and they are each other's partners");
    assert_eq!(cb.partner(), ca);
}

#[test]
fn wrong_arity_link_is_closed() {
    let mut app = app();
    // THREE portals share a link → all closed (no member has a partner).
    let es: Vec<_> = [100.0, 300.0, 500.0]
        .iter()
        .map(|x| {
            app.world_mut()
                .spawn((PortalLink(link_hash("trio")), floor(Vec2::new(*x, 300.0))))
                .id()
        })
        .collect();
    // A lone portal on another link → also closed.
    let lone = app
        .world_mut()
        .spawn((
            PortalLink(link_hash("solo")),
            floor(Vec2::new(700.0, 300.0)),
        ))
        .id();
    app.update();
    let all: Vec<PlacedPortal> = {
        let mut q = app.world_mut().query::<&PlacedPortal>();
        q.iter(app.world()).cloned().collect()
    };
    for e in es.iter().chain(std::iter::once(&lone)) {
        let c = app.world().get::<PlacedPortal>(*e).unwrap().channel;
        assert!(
            !all.iter().any(|p| p.channel == c.partner()),
            "closed: no partner exists for {c:?}"
        );
    }
}

/// The opening of `entity` this frame (its half-length along the surface).
fn opening(app: &App, entity: Entity) -> f32 {
    let p = app.world().get::<PlacedPortal>(entity).unwrap();
    p.opening_half_length()
}

fn authored(app: &App, entity: Entity) -> f32 {
    let p = app.world().get::<PlacedPortal>(entity).unwrap();
    p.half_length
}

fn set_authored(app: &mut App, entity: Entity, half: f32) {
    let mut p = app.world_mut().get_mut::<PlacedPortal>(entity).unwrap();
    p.half_length = half;
}

/// ⭐ THE EFFECTIVE OPENING IS DERIVED; THE AUTHORED ONE IS KEPT. An 80 end
/// beside a 30 end opens 30, and opens 80 again when its partner grows to
/// 80. The minimum was written back into the extent, so the 80 was lost and
/// the pair stayed 30. Both spawn orders.
#[test]
fn a_pair_opens_its_smaller_end_and_opens_wide_again_when_it_grows() {
    for wide_first in [true, false] {
        let mut app = app();
        let mut big = floor(Vec2::new(100.0, 300.0));
        big.half_length = 80.0;
        let mut small = floor(Vec2::new(500.0, 300.0));
        small.half_length = 30.0;
        let (a, b) = if wide_first {
            let a = app.world_mut().spawn((PortalLink(link_hash("d")), big)).id();
            (a, app.world_mut().spawn((PortalLink(link_hash("d")), small)).id())
        } else {
            let b = app.world_mut().spawn((PortalLink(link_hash("d")), small)).id();
            (app.world_mut().spawn((PortalLink(link_hash("d")), big)).id(), b)
        };
        app.update();
        assert!((opening(&app, a) - 30.0).abs() < 1e-3, "wide_first={wide_first}: {}", opening(&app, a));
        assert!((opening(&app, b) - 30.0).abs() < 1e-3);
        assert_eq!(app.world().get::<PlacedPortal>(a).unwrap().pos, Vec2::new(100.0, 300.0), "centered");
        assert!((authored(&app, a) - 80.0).abs() < 1e-3, "the authored 80 is kept: {}", authored(&app, a));

        set_authored(&mut app, b, 80.0);
        app.update();
        assert!((opening(&app, a) - 80.0).abs() < 1e-3, "wide_first={wide_first}: the pair opens wide again: {}", opening(&app, a));
        assert!((opening(&app, b) - 80.0).abs() < 1e-3);

        // A lone end opens what it authors.
        app.world_mut().entity_mut(b).despawn();
        set_authored(&mut app, a, 60.0);
        app.update();
        assert!((opening(&app, a) - 60.0).abs() < 1e-3, "{}", opening(&app, a));
    }
}

/// A pair is one size whatever its ends face. A portal on a slope and one on
/// a wall, each placed with the standard opening, open to the same length:
/// the slope's opening was read from its axis-aligned box, which is longer
/// and thicker than the opening for any normal that is not a quarter turn.
#[test]
fn a_slope_portal_and_a_wall_portal_open_to_one_length() {
    let mut app = app();
    let slope = Vec2::new(-(30f32.to_radians().sin()), -(30f32.to_radians().cos()));
    let wall = Vec2::new(-1.0, 0.0);
    let channel = PortalChannel::Authored(PortalChannelColor::Indexed(0));
    let a = app
        .world_mut()
        .spawn((PortalLink(link_hash("e")), PlacedPortal::fixed(channel, Vec2::new(100.0, 300.0), slope, crate::PORTAL_OPENING_HALF)))
        .id();
    let b = app
        .world_mut()
        .spawn((PortalLink(link_hash("e")), PlacedPortal::fixed(channel, Vec2::new(500.0, 300.0), wall, crate::PORTAL_OPENING_HALF)))
        .id();
    app.update();
    let (oa, ob) = (opening(&app, a), opening(&app, b));
    assert!((oa - crate::types::PORTAL_OPENING_HALF).abs() < 1e-3, "slope opens the standard length: {oa}");
    assert!((oa - ob).abs() < 1e-3, "one size: slope {oa}, wall {ob}");
    let thick = app.world().get::<PlacedPortal>(a).unwrap().opening_half_thickness();
    assert!((thick - crate::types::PORTAL_THICKNESS_HALF).abs() < 1e-3, "slope keeps the standard thickness: {thick}");
}

/// A link name whose hash sorts below `than`'s: a group that came later and
/// sorts first took the first channel pair from the groups already open.
fn a_name_that_sorts_below(than: &str) -> &'static str {
    ["a", "b", "c", "d", "e", "f", "g", "h", "door", "gate"]
        .into_iter()
        .find(|name| link_hash(name) < link_hash(than))
        .expect("one of the names hashes below")
}

/// The frame of the eviction road: links, openings, then the push of a body
/// whose portal closed or moved.
fn eviction_app() -> App {
    let mut app = App::new();
    app.init_resource::<crate::PortalFrameHistory>();
    app.add_systems(
        Update,
        (resolve_portal_links, equalize_pair_apertures, crate::evict_straddlers_on_portal_change).chain(),
    );
    app
}

/// A body in the plane of the floor portal at x=100.
fn straddler() -> BodyKinematics {
    BodyKinematics {
        pos: Vec2::new(100.0, 300.0),
        vel: Vec2::ZERO,
        size: Vec2::new(24.0, 40.0),
        facing: 1.0,
    }
}

/// ⭐ WITNESS 1: A PAIR KEEPS ITS CHANNELS WHEN ANOTHER PAIR OPENS, in its own
/// room or in another live room. The channel was the link's rank among all
/// the link hashes of every live room, so a new name that sorts first moved
/// the open pair to the next channels: its portal "closed" under a body in
/// its plane, and the body was pushed out. Then the new pair goes again.
#[test]
fn a_pair_keeps_its_channels_when_another_pair_opens_and_goes() {
    let late = a_name_that_sorts_below("z");
    for elsewhere in [false, true] {
        let mut app = eviction_app();
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::ACTIVATION.next()];
        for room in rooms {
            app.world_mut().spawn((RoomInstanceRoot, room));
        }
        let z: Vec<Entity> = [100.0, 500.0]
            .map(|x| {
                app.world_mut()
                    .spawn((PortalLink(link_hash("z")), floor(Vec2::new(x, 300.0)), InRoomInstance(rooms[0])))
                    .id()
            })
            .to_vec();
        let body = app.world_mut().spawn((straddler(), InRoomInstance(rooms[0]))).id();
        app.update();
        let channels = |app: &App| z.iter().map(|e| app.world().get::<PlacedPortal>(*e).unwrap().channel).collect::<Vec<_>>();
        let before = channels(&app);
        assert_eq!(before[0].partner(), before[1], "the premise: z is a pair");
        assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos, straddler().pos, "the premise: nothing pushed the body yet");

        let room = rooms[usize::from(elsewhere)];
        let a: Vec<Entity> = [200.0, 700.0]
            .map(|x| {
                app.world_mut()
                    .spawn((PortalLink(link_hash(late)), floor(Vec2::new(x, 600.0)), InRoomInstance(room)))
                    .id()
            })
            .to_vec();
        app.update();
        assert_eq!(channels(&app), before, "elsewhere={elsewhere}: z keeps its channels when {late:?} opens");
        assert_eq!(
            app.world().get::<BodyKinematics>(body).unwrap().pos,
            straddler().pos,
            "elsewhere={elsewhere}: the body in z's plane is not pushed: its portal did not close"
        );
        let ca = app.world().get::<PlacedPortal>(a[0]).unwrap().channel;
        assert_eq!(ca.partner(), app.world().get::<PlacedPortal>(a[1]).unwrap().channel, "the new pair works");

        for e in a {
            app.world_mut().entity_mut(e).despawn();
        }
        app.update();
        assert_eq!(channels(&app), before, "elsewhere={elsewhere}: and when it goes again");
        assert_eq!(app.world().get::<BodyKinematics>(body).unwrap().pos, straddler().pos);
    }
}

/// ⭐ WITNESS 2: THE ENDS OF A PAIR KEEP THEIR ROLES WHEN THEY PASS EACH OTHER
/// AND WHEN THEY MEET. The end was the order of the positions, so two ends
/// that passed changed channels, and two at one place both found the first
/// end: no partner.
#[test]
fn the_ends_of_a_pair_keep_their_roles_when_they_pass_and_meet() {
    let mut app = app();
    let a = app.world_mut().spawn((PortalLink(link_hash("door")), floor(Vec2::new(100.0, 300.0)))).id();
    let b = app.world_mut().spawn((PortalLink(link_hash("door")), floor(Vec2::new(500.0, 300.0)))).id();
    app.update();
    let channel = |app: &App, e: Entity| app.world().get::<PlacedPortal>(e).unwrap().channel;
    let (ca, cb) = (channel(&app, a), channel(&app, b));
    assert_eq!(ca.partner(), cb, "the premise: a pair");
    for (xa, xb) in [(600.0, 50.0), (300.0, 300.0), (100.0, 500.0)] {
        app.world_mut().get_mut::<PlacedPortal>(a).unwrap().pos.x = xa;
        app.world_mut().get_mut::<PlacedPortal>(b).unwrap().pos.x = xb;
        app.update();
        assert_eq!((channel(&app, a), channel(&app, b)), (ca, cb), "the ends at x={xa} and x={xb} keep their roles");
    }
}

/// WITNESS 6: EACH LIVE ROOM HAS ITS OWN PAIRS. 64 rooms with one distinct
/// link each all work; the cap was 63 hashes over every live room, so the
/// 64th room's pair was refused.
#[test]
fn sixty_four_rooms_with_a_link_each_all_work() {
    let mut app = app();
    let mut ends: Vec<[Entity; 2]> = Vec::new();
    let mut room = LiveRoomInstance::ACTIVATION;
    for index in 0..64 {
        app.world_mut().spawn((RoomInstanceRoot, room));
        let name = format!("door_{index}");
        ends.push([100.0, 500.0].map(|x| {
            app.world_mut()
                .spawn((PortalLink(link_hash(&name)), floor(Vec2::new(x, 300.0)), InRoomInstance(room)))
                .id()
        }));
        room = room.next();
    }
    app.update();
    for (index, [a, b]) in ends.into_iter().enumerate() {
        let ca = app.world().get::<PlacedPortal>(a).unwrap().channel;
        let cb = app.world().get::<PlacedPortal>(b).unwrap().channel;
        assert_eq!(ca.partner(), cb, "room {index}: its pair works");
    }
}

/// The cap of one room: the 64th group of a room is refused, and the groups
/// that have a pair keep it.
#[test]
fn the_sixty_fourth_group_of_one_room_is_refused_and_the_others_keep_their_pairs() {
    let mut app = app();
    let mut ends: Vec<[Entity; 2]> = Vec::new();
    let spawn_group = |app: &mut App, index: usize| {
        let name = format!("door_{index}");
        [100.0, 500.0].map(|x| {
            app.world_mut()
                .spawn((PortalLink(link_hash(&name)), floor(Vec2::new(x + index as f32, 300.0))))
                .id()
        })
    };
    for index in 0..63 {
        ends.push(spawn_group(&mut app, index));
    }
    app.update();
    let channels = |app: &App, ends: &[[Entity; 2]]| {
        ends.iter()
            .map(|[a, b]| (app.world().get::<PlacedPortal>(*a).unwrap().channel, app.world().get::<PlacedPortal>(*b).unwrap().channel))
            .collect::<Vec<_>>()
    };
    let before = channels(&app, &ends);
    assert!(before.iter().all(|(a, b)| a.partner() == *b), "63 groups fit in one room");
    let [x, y] = spawn_group(&mut app, 63);
    app.update();
    assert_eq!(channels(&app, &ends), before, "the open pairs keep their channels");
    let cx = app.world().get::<PlacedPortal>(x).unwrap().channel;
    assert_ne!(cx.partner(), app.world().get::<PlacedPortal>(y).unwrap().channel, "the 64th is refused");
}
