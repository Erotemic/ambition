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
    app.insert_resource(PortalViewer {
        present: true,
        eye: Vec2::new(300.0, 300.0),
        room: Some(room),
        half_size: Vec2::splat(12.0),
        ..default()
    });
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
