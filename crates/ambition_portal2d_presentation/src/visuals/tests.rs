use super::*;
use ambition_portal2d::PortalChannelColor;
use crate::PortalWorldFrame;
use bevy::camera::visibility::RenderLayers;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);

/// A thin-wall door pair (the c136/c137 shape): two wall portals 32px
/// apart on opposite faces, apertures aligned.
fn thin_wall_pair() -> (PlacedPortal, PlacedPortal) {
    let left = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(500.0, 300.0),
        Vec2::new(-1.0, 0.0),
        46.0,
    );
    let right = PlacedPortal::fixed(
        ambition_portal2d::PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(532.0, 300.0),
        Vec2::new(1.0, 0.0),
        46.0,
    );
    (left, right)
}

fn test_app() -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Image>::default());
    app.insert_resource(Assets::<TextureAtlasLayout>::default());
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<crate::PortalClipMaterial>::default());
    app
}

/// An app that draws each portal's frame: its line of light and its label.
fn frame_app() -> App {
    let mut app = test_app();
    app.insert_resource(Assets::<crate::PortalGlowMaterial>::default());
    app.init_resource::<Time>();
    app.add_systems(Update, (sync_portal_visuals, crate::sync_portal_glows));
    app
}

/// The name and place of each part of each portal's frame.
fn frame_parts(app: &mut App) -> Vec<(String, Vec3)> {
    app.world_mut()
        .query_filtered::<(&Name, &Transform), Or<(With<PortalVisual>, With<crate::PortalGlow>)>>()
        .iter(app.world())
        .filter(|(n, _)| n.contains("glow") || n.contains("label"))
        .map(|(n, t)| (n.to_string(), t.translation))
        .collect()
}

/// Each portal's frame (its line of light and its label) draws UNDER the
/// exit-side body slice and under the whole window band, and over the world:
/// a body is over the light on each side of a seam, and a window covers the
/// half of its own line that is behind its face. With a viewer or with none:
/// the frame's z does not follow the viewer.
#[test]
fn a_portals_frame_draws_under_the_exit_slice_and_the_window_and_over_the_world() {
    for viewer in [false, true] {
        let mut app = frame_app();
        let (left, right) = thin_wall_pair();
        app.world_mut().spawn(left);
        app.world_mut().spawn(right);
        if viewer {
            app.insert_resource(crate::PortalViewers::one(crate::PortalViewer {
                observer: None,
                present: true,
                room: Some(ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance::ACTIVATION),
                eye: Vec2::new(460.0, 300.0), // left of the left face
                forward: Vec2::ZERO,
                half_size: Vec2::new(12.0, 20.0),
                on_back_of: None,
                crossed_into: None,
                occluders: Vec::new(),
            }));
        }
        app.update();
        app.update();

        let parts = frame_parts(&mut app);
        assert_eq!(parts.len(), 4, "a line and a label for each of two portals, got {parts:?}");
        for (name, at) in &parts {
            assert!(
                at.z < crate::PORTAL_EXIT_COPY_Z && at.z < crate::PORTAL_WINDOW_Z,
                "{name} must draw under the exit slice and the window (viewer: {viewer}), z={}",
                at.z
            );
            assert!(
                at.z > ambition_platformer2d_core::config::WORLD_Z_BLOCK,
                "{name} must draw over the world, z={}",
                at.z
            );
        }

        // And the frame stays on the WORLD layer: portal captures must
        // photograph it, so portals seen through a window still look like
        // portals, and the far portal's line is the other half of a seam.
        let layered = app
            .world_mut()
            .query_filtered::<(), (Or<(With<PortalVisual>, With<crate::PortalGlow>)>, With<RenderLayers>)>()
            .iter(app.world())
            .count();
        assert_eq!(layered, 0, "frame parts live on the default WORLD layer so captures photograph them");
    }
}

/// Two live rooms of different sizes, and the same portal pair in each (view
/// half, V2m). Each portal's visuals are placed by its own room's frame and
/// stamped into its own room, so each view draws only its room's portals.
/// Before, one size was copied from the sole live room, so with two rooms
/// live no frame was synced and every portal was drawn unbanded, by a stale
/// size, in every view.
#[test]
fn each_portal_is_drawn_in_its_own_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = test_app();
    app.add_systems(Update, sync_portal_visuals);
    let small = Vec2::new(400.0, 300.0);
    let first = LiveRoomInstance::ACTIVATION;
    let second = LiveRoomInstance::from_ordinal(1);
    ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(
        app.world_mut(),
        second,
        ae::RoomGeometry(ae::World::new("small portal room", small, small * 0.5, Vec::new())),
    );
    let (left, right) = thin_wall_pair();
    for room in [first, second] {
        app.world_mut().spawn((left.clone(), InRoomInstance(room)));
        app.world_mut().spawn((right.clone(), InRoomInstance(room)));
    }
    app.update();

    let labels: Vec<(Option<LiveRoomInstance>, Vec3)> = app
        .world_mut()
        .query_filtered::<(&Name, &Transform, Option<&InRoomInstance>), With<PortalVisual>>()
        .iter(app.world())
        .filter(|(name, ..)| name.as_str() == "Portal label")
        .map(|(_, transform, stamp)| (stamp.map(|stamp| stamp.0), transform.translation))
        .collect();
    assert_eq!(labels.len(), 4, "two portals in each of two rooms, one label each: {labels:?}");
    for (room, size) in [(first, WORLD), (second, small)] {
        let frame = PortalWorldFrame { size };
        let mut expected: Vec<Vec2> = [&left, &right]
            .iter()
            .map(|portal| frame.to_render(portal.pos + portal.normal.normalize_or_zero() * 24.0, 0.0).truncate())
            .collect();
        let mut drawn: Vec<Vec2> = labels
            .iter()
            .filter(|(stamp, _)| *stamp == Some(room))
            .map(|(_, at)| at.truncate())
            .collect();
        let key = |v: &Vec2| (v.x.round() as i32, v.y.round() as i32);
        expected.sort_by_key(key);
        drawn.sort_by_key(key);
        assert_eq!(drawn, expected, "room {room:?} ({size}): its portals' labels by its own frame");
    }
}

/// A portal's label is kept while the portal stays, and not written; a
/// portal taken away takes it. The labels were despawned and spawned each
/// frame, and a new `Text2d` is laid out again.
#[test]
fn a_portals_label_is_kept_while_the_portal_stays() {
    let mut app = test_app();
    app.add_systems(Update, sync_portal_visuals);
    let (left, _) = thin_wall_pair();
    let portal = app.world_mut().spawn(left).id();
    app.update();
    let labels = |app: &mut App| {
        app.world_mut()
            .query_filtered::<(Entity, Ref<Transform>), With<PortalVisual>>()
            .iter(app.world())
            .map(|(entity, transform)| (entity, transform.last_changed()))
            .collect::<Vec<_>>()
    };
    let first = labels(&mut app);
    assert_eq!(first.len(), 1, "one portal, one label");
    app.update();
    assert_eq!(labels(&mut app), first, "a portal that stays keeps its label and writes nothing");
    app.world_mut().despawn(portal);
    app.update();
    assert!(labels(&mut app).is_empty(), "a portal taken away takes its label");
}
