//! A feature visual is placed by the geometry of its own live room (view half,
//! cut V2b). Two live rooms of different sizes; a visual stamped into each, at
//! one simulation position. Each is placed by its own room's flip into Bevy
//! space. A sole-room read did not run while two rooms were live, so no visual
//! moved.

use super::*;

const AT: ae::Vec2 = ae::Vec2::new(100.0, 200.0);

fn view() -> ambition_sim_view::FeatureView {
    let mut view = super::depth_plane_tests::view_at(AT, ae::DepthPlane::PLAYABLE);
    view.kind = FeatureVisualKind::Actor;
    view
}

fn room(size: ae::Vec2) -> ae::RoomGeometry {
    ae::RoomGeometry(ae::World::new("room placement", size, ae::Vec2::new(40.0, 40.0), Vec::new()))
}

#[test]
fn each_feature_visual_is_placed_in_its_own_live_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::{InRoomInstance, LiveRoomInstance};
    let mut app = App::new();
    let big = ae::Vec2::new(800.0, 600.0);
    let small = ae::Vec2::new(400.0, 300.0);
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(app.world_mut(), room(big));
    let second = LiveRoomInstance::ACTIVATION.next();
    ambition_platformer2d_shared_tangle::lifecycle::spawn_live_room(app.world_mut(), second, room(small));
    app.insert_resource(FeatureViewIndex::from_rows([
        ("in_big".to_string(), view()),
        ("in_small".to_string(), view()),
    ]));
    app.init_resource::<ambition_sim_view::PresentedFeaturePoses>();
    app.add_systems(Update, sync_visuals);
    let spawn = |app: &mut App, id: &str, room: LiveRoomInstance| {
        app.world_mut()
            .spawn((
                FeatureVisual { id: id.to_string() },
                InRoomInstance(room),
                Transform::default(),
                Sprite::default(),
                Visibility::Visible,
            ))
            .id()
    };
    let in_big = spawn(&mut app, "in_big", LiveRoomInstance::ACTIVATION);
    let in_small = spawn(&mut app, "in_small", second);
    app.update();
    let placed = |e: Entity| app.world().get::<Transform>(e).expect("a drawn feature").translation.truncate();
    assert_eq!(
        [placed(in_big), placed(in_small)],
        [
            BVec2::new(AT.x - big.x * 0.5, big.y * 0.5 - AT.y),
            BVec2::new(AT.x - small.x * 0.5, small.y * 0.5 - AT.y),
        ],
        "a feature visual was not placed by its own live room's geometry"
    );
}
