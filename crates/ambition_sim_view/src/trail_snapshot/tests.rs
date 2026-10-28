use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::{
    InRoomInstance, LiveRoomInstance, RoomInstanceRoot,
};
use bevy::ecs::system::RunSystemOnce;

fn room_with_trail(world: &mut World, ordinal: u32, size: Vec2) -> Entity {
    let room = LiveRoomInstance::from_ordinal(ordinal);
    let root = world
        .spawn((
            RoomInstanceRoot,
            room,
            ae::RoomGeometry(ae::World::new("room", size, Vec2::ZERO, Vec::new())),
        ))
        .id();
    let mut trail = PlayerTrail::emitting_from(Vec2::splat(10.0));
    trail.emit_to(Vec2::new(60.0, 10.0));
    world.spawn((PlayerEntity, InRoomInstance(room), trail));
    root
}

fn snapshot(world: &mut World) -> TrailSnapshot {
    world
        .run_system_once(snapshot_emitted_trails)
        .expect("trail sources are available")
}

#[test]
fn each_trail_is_placed_by_its_own_live_room() {
    let mut one = World::new();
    room_with_trail(&mut one, 0, Vec2::new(400.0, 300.0));
    assert_eq!(snapshot(&mut one).0.len(), 1);

    let mut two = World::new();
    let sizes = [Vec2::new(400.0, 300.0), Vec2::new(800.0, 900.0)];
    for (ordinal, size) in sizes.into_iter().enumerate() {
        room_with_trail(&mut two, ordinal as u32, size);
    }
    let rows = snapshot(&mut two);
    let mut starts: Vec<_> = rows.0.iter().map(|strip| strip.points[0]).collect();
    let mut expected: Vec<_> = sizes
        .into_iter()
        .map(|size| {
            ae::config::world_size_to_bevy(
                size,
                Vec2::splat(10.0),
                ae::config::WORLD_Z_PLAYER - 0.1,
            )
            .truncate()
        })
        .collect();
    let key = |p: &Vec2| (p.x.to_bits(), p.y.to_bits());
    starts.sort_by_key(key);
    expected.sort_by_key(key);
    assert_eq!(starts, expected);
    assert!(
        rows.0
            .iter()
            .all(|strip| strip.state == TrailStripState::Emitting)
    );
}

#[test]
fn room_bands_use_live_positions_instead_of_instance_ordinals() {
    let mut one = World::new();
    room_with_trail(&mut one, 12, Vec2::new(400.0, 300.0));
    assert_eq!(snapshot(&mut one).0[0].band, None);

    let mut two = World::new();
    room_with_trail(&mut two, 7, Vec2::new(400.0, 300.0));
    room_with_trail(&mut two, 12, Vec2::new(800.0, 900.0));
    let mut bands: Vec<_> = snapshot(&mut two)
        .0
        .into_iter()
        .map(|strip| strip.band)
        .collect();
    bands.sort();
    assert_eq!(bands, vec![Some(0), Some(1)]);
}

#[test]
fn a_retired_room_leaves_no_trail_geometry() {
    let mut world = World::new();
    let root = room_with_trail(&mut world, 7, Vec2::splat(400.0));
    assert_eq!(snapshot(&mut world).0.len(), 1);
    world.despawn(root);
    assert!(snapshot(&mut world).0.is_empty());
}
