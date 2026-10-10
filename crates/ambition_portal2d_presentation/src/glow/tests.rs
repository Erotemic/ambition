use super::*;
use ambition_portal2d::PortalChannelColor;
use bevy::camera::visibility::RenderLayers;
use std::time::Duration;

const WORLD: Vec2 = Vec2::new(1000.0, 600.0);
/// One step of these tests: a 60 Hz frame.
const FRAME_S: f32 = 1.0 / 60.0;

fn purple_at(x: f32) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Purple),
        Vec2::new(x, 300.0),
        Vec2::new(-1.0, 0.0),
        Vec2::new(9.0, 46.0),
    )
}

fn yellow_at(x: f32) -> PlacedPortal {
    PlacedPortal::fixed(
        PortalChannel::Authored(PortalChannelColor::Yellow),
        Vec2::new(x, 300.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(9.0, 46.0),
    )
}

fn test_app() -> App {
    let mut app = App::new();
    crate::one_live_room(&mut app, WORLD);
    app.insert_resource(Assets::<Mesh>::default());
    app.insert_resource(Assets::<PortalGlowMaterial>::default());
    app.init_resource::<Time>();
    app.add_systems(Update, sync_portal_glows);
    app
}

/// Step `frames` frames of [`FRAME_S`].
fn step(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_secs_f32(FRAME_S));
        app.update();
    }
}

fn frames_of(seconds: f32) -> usize {
    (seconds / FRAME_S).ceil() as usize + 1
}

fn glows(app: &mut App) -> Vec<PortalGlow> {
    app.world_mut().query::<&PortalGlow>().iter(app.world()).cloned().collect()
}

/// A portal that is in its room when the room comes is level content: it is
/// there, whole, and plays no opening.
#[test]
fn a_portal_that_was_there_with_its_room_does_not_open() {
    let mut app = test_app();
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, 2);
    let all = glows(&mut app);
    assert_eq!(all.len(), 1, "one line of light for one portal: {all:?}");
    assert_eq!(all[0].appear, 1.0, "a portal that was there opened");
    assert_eq!(all[0].dissolve, 0.0);
}

/// A portal that comes after its room has settled was added: it opens from
/// nothing, and is whole after [`APPEAR_S`]. The control is the portal of the
/// same room that was there, which stays whole all the time.
#[test]
fn a_portal_added_to_a_room_opens() {
    let mut app = test_app();
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, frames_of(ROOM_SETTLE_S));
    let added = app.world_mut().spawn(yellow_at(700.0)).id();
    step(&mut app, 2);
    let all = glows(&mut app);
    let (was_there, new): (Vec<_>, Vec<_>) = all.iter().partition(|glow| glow.portal != Some(added));
    assert_eq!((was_there.len(), new.len()), (1, 1), "{all:?}");
    assert_eq!(was_there[0].appear, 1.0);
    assert!(new[0].appear > 0.0 && new[0].appear < 0.2, "an added portal is opening: {}", new[0].appear);

    step(&mut app, frames_of(APPEAR_S));
    let all = glows(&mut app);
    assert!(all.iter().all(|glow| glow.appear == 1.0), "it did not finish opening: {all:?}");
}

/// The first portal of a room that has been live for a time was added to it.
/// A room with no portal is still a room that is seen.
#[test]
fn the_first_portal_of_an_old_room_opens() {
    let mut app = test_app();
    step(&mut app, frames_of(ROOM_SETTLE_S));
    assert!(glows(&mut app).is_empty());
    app.world_mut().spawn(purple_at(500.0));
    step(&mut app, 2);
    let all = glows(&mut app);
    assert_eq!(all.len(), 1);
    assert!(all[0].appear < 0.2, "the first portal of an old room did not open: {}", all[0].appear);
}

/// A portal that is removed dissolves where it was, and its line is gone
/// after [`DISSOLVE_S`]. A portal that stays does not dissolve.
#[test]
fn a_portal_that_is_removed_dissolves_where_it_was() {
    let mut app = test_app();
    let leaves = app.world_mut().spawn(purple_at(500.0)).id();
    app.world_mut().spawn(yellow_at(700.0));
    step(&mut app, 3);
    app.world_mut().despawn(leaves);
    step(&mut app, 3);
    let all = glows(&mut app);
    let (gone, stays): (Vec<_>, Vec<_>) = all.iter().partition(|glow| glow.portal.is_none());
    assert_eq!((gone.len(), stays.len()), (1, 1), "{all:?}");
    assert!(gone[0].dissolve > 0.0 && gone[0].dissolve < 0.3, "{}", gone[0].dissolve);
    assert_eq!(gone[0].pos, Vec2::new(500.0, 300.0), "it dissolves where it was");
    assert_eq!(stays[0].dissolve, 0.0);

    step(&mut app, frames_of(DISSOLVE_S));
    let all = glows(&mut app);
    assert_eq!(all.len(), 1, "the dissolved line stayed: {all:?}");
    assert!(all[0].portal.is_some());
}

/// A rollback gives a restored portal a new entity. The same channel at the
/// same place in the same frame is the same aperture: no dissolve, no opening.
/// The control moves the new portal, which is then a removal and an addition.
#[test]
fn a_portal_that_comes_back_as_a_new_entity_at_its_place_is_the_same_aperture() {
    for (moved_to, effects) in [(500.0, false), (640.0, true)] {
        let mut app = test_app();
        let old = app.world_mut().spawn(purple_at(500.0)).id();
        step(&mut app, frames_of(ROOM_SETTLE_S));
        app.world_mut().despawn(old);
        let new = app.world_mut().spawn(purple_at(moved_to)).id();
        step(&mut app, 2);
        let all = glows(&mut app);
        let of_new: Vec<_> = all.iter().filter(|glow| glow.portal == Some(new)).collect();
        assert_eq!(of_new.len(), 1, "moved to {moved_to}: {all:?}");
        if effects {
            assert_eq!(all.len(), 2, "the old line dissolves and a new one opens: {all:?}");
            assert!(of_new[0].appear < 0.2, "a portal at a new place did not open");
        } else {
            assert_eq!(all.len(), 1, "one aperture, one line: {all:?}");
            assert_eq!(of_new[0].appear, 1.0, "the same aperture opened again");
        }
    }
}

/// The line follows a portal that moves (a portal on a moving face), and is on
/// the world layer and in its room, so a portal capture photographs it.
#[test]
fn the_line_follows_its_portal_and_is_drawn_in_its_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = test_app();
    let portal = app.world_mut().spawn(purple_at(500.0)).id();
    step(&mut app, 2);
    app.world_mut().get_mut::<PlacedPortal>(portal).unwrap().pos.y = 340.0;
    step(&mut app, 1);
    let frame = crate::PortalWorldFrame { size: WORLD };
    let (transform, stamp, layers) = app
        .world_mut()
        .query_filtered::<(&Transform, Option<&InRoomInstance>, Option<&RenderLayers>), With<PortalGlow>>()
        .single(app.world())
        .expect("one line");
    let at = frame.to_render(Vec2::new(500.0, 340.0), crate::PORTAL_RIM_OVERLAY_Z);
    assert_eq!(transform.translation, at, "the line did not follow its portal");
    assert_eq!(stamp.map(|stamp| stamp.0), Some(LiveRoomInstance::ACTIVATION));
    assert!(layers.is_none(), "the line is on the world layer, so captures photograph it");
    // A wall portal's opening is upright: the quad's long axis is the y axis.
    let long = transform.rotation * Vec3::X;
    assert!(long.x.abs() < 1e-4 && long.y.abs() > 0.999, "{long:?}");
    // Its local +y, the room side, is the portal's normal in render space.
    let room_side = transform.rotation * Vec3::Y;
    assert!((room_side.x - -1.0).abs() < 1e-4, "{room_side:?}");
}
