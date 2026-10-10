//! The capture rigs of the through-portal window, run as a system.

use super::*;
use ambition_platformer2d_shared_tangle::camera_layers::{
    live_room_render_layer, LIVE_ROOM_RENDER_LAYER_BASE, LIVE_ROOM_RENDER_LAYER_LAST,
};
use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

/// A thin-wall door pair: two wall portals 32px apart on opposite faces.
fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
    let left = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(ambition_portal2d::PortalChannelColor::Purple),
        Vec2::new(500.0, 300.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    let right = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(ambition_portal2d::PortalChannelColor::Yellow),
        Vec2::new(532.0, 300.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(9.0, 46.0),
    );
    (left, right)
}

/// The room bands each capture rig renders. The viewer and the portal pair
/// are in room #0 alone, or, with `two_rooms`, in room #1 beside #0.
fn capture_room_bands(two_rooms: bool) -> Vec<Vec<usize>> {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Image>::default())
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<ColorMaterial>::default())
        .insert_resource(crate::PortalEffectSelection {
            active: crate::PortalVisualEffect::ViewCones,
        })
        .init_resource::<PortalViewConeConfig>()
        .init_resource::<PortalCaptureQualityBudget>()
        .init_resource::<Time>();
    let room = if two_rooms {
        let second = LiveRoomInstance::from_ordinal(1);
        ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
            app.world_mut(),
            second,
            ae::RoomGeometry(ae::World::new("second portal room", WORLD, WORLD * 0.5, Vec::new())),
        );
        second
    } else {
        LiveRoomInstance::ACTIVATION
    };
    let (left, right) = thin_wall_pair();
    app.world_mut().spawn((left, InRoomInstance(room)));
    app.world_mut().spawn((right, InRoomInstance(room)));
    app.insert_resource(PortalViewers::one(PortalViewer {
        present: true,
        eye: Vec2::new(300.0, 300.0),
        room: Some(room),
        half_size: Vec2::splat(12.0),
        ..default()
    }));
    app.add_systems(Update, sync_portal_view_cones);
    app.update();
    app.world_mut()
        .query_filtered::<&RenderLayers, With<PortalViewRig>>()
        .iter(app.world())
        .map(|layers| {
            layers
                .iter()
                .filter(|layer| (LIVE_ROOM_RENDER_LAYER_BASE..=LIVE_ROOM_RENDER_LAYER_LAST).contains(layer))
                .collect()
        })
        .collect()
}

/// While two rooms are live, the viewer room's world is on its room band, so
/// each capture renders that band (and not the other room's). The control:
/// with one live room nothing is banded, and a capture renders no band.
#[test]
fn a_capture_renders_the_band_of_the_viewers_room_while_two_rooms_are_live() {
    let one = capture_room_bands(false);
    assert_eq!(one.len(), 2, "one rig per portal of the pair");
    assert!(one.iter().all(Vec::is_empty), "{one:?}");
    let two = capture_room_bands(true);
    assert_eq!(two.len(), 2, "one rig per portal of the pair");
    assert!(
        two.iter().all(|bands| *bands == vec![live_room_render_layer(1)]),
        "each capture renders room #1's band only: {two:?}"
    );
}

/// Two live rooms, a thin-wall pair in each, and an eye in each room of
/// `eyes`. Returns, for each rig: its room, the room bands its capture
/// renders, the per-portal window layers its capture renders, and the room
/// its window mesh is stamped into.
fn windows_of_two_rooms(
    eyes: &[LiveRoomInstance],
) -> Vec<(LiveRoomInstance, Vec<usize>, Vec<usize>, Option<LiveRoomInstance>)> {
    let eyes: Vec<_> = eyes.iter().map(|room| (*room, None)).collect();
    let mut app = two_rooms_with_eyes(&eyes);
    let rigs: Vec<(LiveRoomInstance, Vec<usize>, Vec<usize>, Entity)> = app
        .world_mut()
        .query::<(&PortalViewRig, &RenderLayers)>()
        .iter(app.world())
        .map(|(rig, layers)| {
            (
                rig.room(),
                layers
                    .iter()
                    .filter(|layer| (LIVE_ROOM_RENDER_LAYER_BASE..=LIVE_ROOM_RENDER_LAYER_LAST).contains(layer))
                    .collect(),
                layers.iter().filter(|layer| *layer >= PORTAL_WINDOW_SELF_LAYER_BASE).collect(),
                rig.cone,
            )
        })
        .collect();
    let mut rows: Vec<_> = rigs
        .into_iter()
        .map(|(room, bands, windows, cone)| {
            let stamp = app.world().get::<InRoomInstance>(cone).map(|stamp| stamp.0);
            (room, bands, windows, stamp)
        })
        .collect();
    rows.sort_by_key(|row| row.0);
    rows
}

/// The fixture of [`windows_of_two_rooms`], after two frames. Each eye is
/// (its room, the centre of its observer's camera). An eye with a camera has
/// an observer, whose sample is 800 x 450 at that centre.
fn two_rooms_with_eyes(eyes: &[(LiveRoomInstance, Option<Vec2>)]) -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Image>::default())
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<ColorMaterial>::default())
        .insert_resource(crate::PortalEffectSelection {
            active: crate::PortalVisualEffect::ViewCones,
        })
        .init_resource::<PortalViewConeConfig>()
        .init_resource::<PortalCaptureQualityBudget>()
        .init_resource::<Time>();
    let second = LiveRoomInstance::from_ordinal(1);
    ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
        app.world_mut(),
        second,
        ae::RoomGeometry(ae::World::new("second portal room", WORLD, WORLD * 0.5, Vec::new())),
    );
    for room in [LiveRoomInstance::ACTIVATION, second] {
        let (left, right) = thin_wall_pair();
        app.world_mut().spawn((left, InRoomInstance(room)));
        app.world_mut().spawn((right, InRoomInstance(room)));
    }
    let mut viewers = PortalViewers::default();
    let mut samples = crate::PortalObserverViews::default();
    for (room, camera) in eyes {
        let observer = camera.map(|centre| {
            let observer = app.world_mut().spawn_empty().id();
            samples.of_mut(observer).capture(centre, centre, centre, Vec2::new(800.0, 450.0), 0, None);
            observer
        });
        viewers.publish(PortalViewer {
            present: true,
            observer,
            eye: Vec2::new(300.0, 300.0),
            room: Some(*room),
            half_size: Vec2::splat(12.0),
            ..default()
        });
    }
    app.insert_resource(viewers);
    app.insert_resource(samples);
    app.add_systems(Update, sync_portal_view_cones);
    // Two frames: the second updates the rigs the first spawned.
    app.update();
    app.update();
    app
}

/// The windows of a room are made for the camera of the view that frames it.
///
/// The parallax anchor of a rig is the centre of the camera, taken through
/// the pair. The two rooms hold the same pair, so the anchors of the two
/// rooms differ by what the two cameras differ by. The control is one camera
/// centre for both observers: the anchors are then the same. With the one
/// camera sample the windows read before, the second room's windows were
/// made for the camera of the first.
#[test]
fn the_windows_of_a_room_are_made_for_the_camera_of_its_own_observer() {
    let first = LiveRoomInstance::ACTIVATION;
    let second = LiveRoomInstance::from_ordinal(1);
    let anchors = |cameras: [Vec2; 2]| -> Vec<(LiveRoomInstance, String, Vec2)> {
        let mut app = two_rooms_with_eyes(&[(first, Some(cameras[0])), (second, Some(cameras[1]))]);
        let mut rows: Vec<_> = app
            .world_mut()
            .query::<&PortalViewRig>()
            .iter(app.world())
            .map(|rig| (rig.room(), rig.channel().name(), rig.parallax_anchor()))
            .collect();
        rows.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
        rows
    };
    let between_the_rooms = |rows: &[(LiveRoomInstance, String, Vec2)]| -> Vec<Vec2> {
        assert_eq!(rows.len(), 4, "two rigs for each room: {rows:?}");
        (0..2).map(|channel| rows[2 + channel].2 - rows[channel].2).collect()
    };
    let here = Vec2::new(300.0, 300.0);
    assert_eq!(between_the_rooms(&anchors([here, here])), vec![Vec2::ZERO; 2], "control: one camera centre");
    assert_eq!(
        between_the_rooms(&anchors([here, here + Vec2::new(340.0, 0.0)])),
        vec![Vec2::new(340.0, 0.0); 2],
        "the windows of the second room are not made for its own camera"
    );
}

/// A3: each live room that has an eye has the windows of its own portals.
///
/// The control is the one eye the seam had: the windows of its room, and no
/// window in the other room. That was the whole seam (`PortalViewer` was one
/// resource), so a pair in the second player's room had no window: "4 portals
/// in 2 rooms, rigs=2".
///
/// The subject is an eye in each room: two rigs for each room. A capture
/// renders the band of its own room only, and its window mesh is stamped
/// into that room, so that the room pass of the renderer keeps it out of a
/// camera that frames the other room.
#[test]
fn each_live_room_that_has_an_eye_has_the_windows_of_its_own_portals() {
    let first = LiveRoomInstance::ACTIVATION;
    let second = LiveRoomInstance::from_ordinal(1);
    let band = |ordinal| vec![live_room_render_layer(ordinal)];

    let one_eye = windows_of_two_rooms(&[first]);
    assert_eq!(
        one_eye.iter().map(|row| row.0).collect::<Vec<_>>(),
        vec![first, first],
        "one eye, in the first room: the windows of that room only"
    );

    let two_eyes = windows_of_two_rooms(&[first, second]);
    assert_eq!(
        two_eyes,
        vec![
            (first, band(0), Vec::new(), Some(first)),
            (first, band(0), Vec::new(), Some(first)),
            (second, band(1), Vec::new(), Some(second)),
            (second, band(1), Vec::new(), Some(second)),
        ],
        "(the room of the rig, the room bands of its capture, the per-portal window layers of its capture, \
         the room its window is stamped into)"
    );

    // An eye in the second room only: the first room has no window.
    let other_eye = windows_of_two_rooms(&[second]);
    assert_eq!(other_eye.iter().map(|row| row.0).collect::<Vec<_>>(), vec![second, second]);
}

/// A per-portal window layer is the layer of a channel, so a capture may see
/// the other windows only while one room is live. The control is one live
/// room: the capture of one end sees the window of the other end.
#[test]
fn a_capture_sees_no_other_window_while_two_rooms_are_live() {
    let (left, right) = thin_wall_pair();
    let all = [left.clone(), right.clone()];
    assert_eq!(
        windows_a_capture_may_see(&all, left.channel, None),
        vec![portal_window_self_layer(right.channel)],
        "one live room: the other window of the room"
    );
    assert!(windows_a_capture_may_see(&all, left.channel, Some(live_room_render_layer(1))).is_empty());
}

/// A live room has one eye: the first the host publishes. A second eye in
/// that room is refused, and an eye in another room is not.
#[test]
fn a_live_room_has_the_eye_of_the_first_observer_the_host_publishes() {
    let room = LiveRoomInstance::ACTIVATION;
    let other = LiveRoomInstance::from_ordinal(1);
    let eye_at = |x: f32, room| PortalViewer { present: true, eye: Vec2::new(x, 0.0), room: Some(room), ..default() };
    let mut viewers = PortalViewers::default();
    assert!(viewers.publish(eye_at(1.0, room)));
    assert!(!viewers.publish(eye_at(2.0, room)), "a second eye in one room");
    assert!(viewers.publish(eye_at(3.0, other)));
    assert_eq!(
        (viewers.in_room(Some(room)).map(|v| v.eye.x), viewers.in_room(Some(other)).map(|v| v.eye.x)),
        (Some(1.0), Some(3.0))
    );
    assert!(viewers.in_room(None).is_none(), "a room that cannot be told has no eye");
    viewers.clear();
    assert!(viewers.first().is_none());
}

/// A window that closes is still the near one or the far one. The viewer
/// stands at the left face of a door through a thin wall, and then at the
/// right face: the left window closes (the wall hides its face), and it is the
/// far one now, so the left portal's frame is drawn under the right window's
/// glass. A closed window kept its old answer, and its frame was drawn over
/// its partner's takeover. The control is the first stand, where the answers
/// are the other way round.
#[test]
fn a_window_that_closed_when_the_viewer_crossed_is_the_far_pane() {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Image>::default())
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<ColorMaterial>::default())
        .insert_resource(crate::PortalEffectSelection {
            active: crate::PortalVisualEffect::ViewCones,
        })
        .init_resource::<PortalViewConeConfig>()
        .init_resource::<PortalCaptureQualityBudget>()
        .init_resource::<Time>();
    let (left, right) = thin_wall_pair();
    let (left_channel, right_channel) = (left.channel, right.channel);
    app.world_mut().spawn(left);
    app.world_mut().spawn(right);
    let wall = ae::Aabb::new(Vec2::new(516.0, 300.0), Vec2::new(16.0, 300.0));
    app.add_systems(Update, sync_portal_view_cones);
    let near_panes = |app: &mut App, eye_x: f32| {
        app.insert_resource(PortalViewers::one(PortalViewer {
            present: true,
            eye: Vec2::new(eye_x, 300.0),
            room: Some(LiveRoomInstance::ACTIVATION),
            half_size: Vec2::splat(12.0),
            occluders: vec![wall],
            ..default()
        }));
        app.update();
        app.update();
        let room = LiveRoomInstance::ACTIVATION;
        let mut near_of = |channel| {
            app.world_mut()
                .query::<&PortalViewRig>()
                .iter(app.world())
                .find(|rig| rig.serves(room, channel))
                .expect("a rig for each portal")
                .pane_dominant()
        };
        (near_of(left_channel), near_of(right_channel))
    };
    assert_eq!(near_panes(&mut app, 470.0), (true, false), "at the left face, the left pane is the near one");
    assert_eq!(
        near_panes(&mut app, 562.0),
        (false, true),
        "at the right face the left window is closed, and it is the far pane"
    );
}
