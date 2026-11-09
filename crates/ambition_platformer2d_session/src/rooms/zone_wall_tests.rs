//! An edge exit is a wall for a body that cannot travel through it.

use super::systems::contribute_loading_zone_walls;
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::feature_overlay::FeatureEcsWorldOverlay;
use ambition_platformer2d_shared_tangle::lifecycle::*;
use ambition_platformer2d_world::rooms::*;
use bevy::prelude::*;

fn body(app: &mut App, x: f32) -> Entity {
    app.world_mut()
        .spawn((
            InRoomInstance(LiveRoomInstance::ACTIVATION),
            ae::BodyKinematics {
                material: Default::default(),
                pos: ae::Vec2::new(x, 100.0),
                vel: ae::Vec2::ZERO,
                canonical_size: ae::Vec2::new(24.0, 16.0),
                facing: 1.0,
            },
        ))
        .id()
}

/// Who the room's exit and door are walls for, after one tick.
fn walled(app: &mut App) -> Vec<(String, Vec<Entity>)> {
    app.update();
    let mut overlays = app.world_mut().query::<&FeatureEcsWorldOverlay>();
    overlays
        .single(app.world())
        .expect("one live room")
        .body_barriers
        .iter()
        .map(|barrier| (barrier.block.name.clone(), barrier.bodies.clone()))
        .collect()
}

/// A crawler went out of a room through an edge exit and crawled round the
/// outside of the level. An edge exit is a wall for each body near it that
/// does not travel; a driven body passes, a body far from it is not
/// listed, and a door (pressed, not walked through) is no wall.
#[test]
fn an_edge_exit_is_a_wall_for_each_body_near_it_that_does_not_travel() {
    let zone = |id: &str, x, activation| LoadingZone {
        id: id.into(),
        name: id.into(),
        activation,
        aabb: ae::Aabb::new(ae::Vec2::new(x, 100.0), ae::Vec2::splat(24.0)),
    };
    let mut room = RoomSpec::new("room", ae::World::new("room", ae::Vec2::new(640.0, 480.0), ae::Vec2::new(96.0, 96.0), Vec::new()));
    room.loading_zones = vec![zone("exit", 616.0, LoadingZoneActivation::EdgeExit), zone("door", 300.0, LoadingZoneActivation::Door)];
    let mut app = App::new();
    insert_room_set(app.world_mut(), RoomSet::from_parts_or_panic("room", vec![room], Vec::new()));
    insert_live_room_component(app.world_mut(), LiveRoomInstance::ACTIVATION);
    insert_live_room_component(app.world_mut(), FeatureEcsWorldOverlay::default());
    app.add_systems(Update, contribute_loading_zone_walls);

    let crawler = body(&mut app, 580.0);
    let far = body(&mut app, 200.0);
    let driven = body(&mut app, 585.0);
    app.world_mut().entity_mut(driven).insert(DrivingParticipant(PlayerSlot(0)));
    let beside_the_door = body(&mut app, 300.0);

    let walls = walled(&mut app);
    assert_eq!(walls, vec![("zone-wall:exit".to_string(), vec![crawler])], "far {far:?}, beside the door {beside_the_door:?}");
}
