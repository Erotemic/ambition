use super::*;
use ambition_characters::actor::{BodyHealth, Health};

fn health(max: i32) -> BodyHealth {
    BodyHealth::new(Health::new(max))
}

fn reservation(rider: Entity) -> MountReservedFor {
    MountReservedFor { rider, lease_seconds: 5.0, board_within: 200.0, expires_in: 1.0 }
}

/// A rider reads the mount it waits for, the mount it rides and whether that
/// mount lives; a mount reads who is in its saddle and who is coming.
#[test]
fn a_rider_and_its_mount_read_the_ride() {
    let mut world = World::new();
    let rider = world.spawn_empty().id();
    let mount = world.spawn((health(40), MountSlot { rider: None })).id();
    assert_eq!(ride_of(&world, rider), Some(Ride::None));
    assert_eq!(saddle_of(&world, mount), Some(Saddle::default()));

    world.entity_mut(mount).insert(reservation(rider));
    assert_eq!(ride_of(&world, rider), Some(Ride::Awaiting));
    let stranger = world.spawn_empty().id();
    assert_eq!(ride_of(&world, stranger), Some(Ride::None), "a reservation for another body was read as mine");
    assert_eq!(saddle_of(&world, mount), Some(Saddle { ridden: false, awaiting: true }));

    world.entity_mut(mount).remove::<MountReservedFor>().insert(MountSlot { rider: Some(rider) });
    world.entity_mut(rider).insert(RidingOn { mount });
    assert_eq!(ride_of(&world, rider), Some(Ride::Aboard { mount_alive: true }));
    assert_eq!(saddle_of(&world, mount), Some(Saddle { ridden: true, awaiting: false }));

    world.get_mut::<BodyHealth>(mount).unwrap().damage(40);
    assert_eq!(ride_of(&world, rider), Some(Ride::Aboard { mount_alive: false }), "a dead mount still lives");
    world.despawn(mount);
    assert_eq!(ride_of(&world, rider), Some(Ride::Aboard { mount_alive: false }), "a gone mount still lives");
}

/// A body reads its presses' edges, whether it tumbles, and its room's size.
#[test]
fn a_body_reads_its_presses_its_tumble_and_its_room() {
    use ambition_platformer2d_core as ae;
    let mut world = World::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
        &mut world,
        ae::RoomGeometry(ae::World::new("stage", ae::Vec2::new(640.0, 480.0), ae::Vec2::ZERO, Vec::new())),
    );
    let mut control = ActorControl::default();
    control.0.burst_pressed = true;
    let facts = ae::BodyMotionFacts { tumbling: true, ..Default::default() };
    let body = world.spawn((control, facts)).id();
    assert_eq!(presses_of(&world, body), Some(BodyPresses { jump: false, burst: true }));
    assert_eq!(tumbling_of(&world, body), Some(true));
    assert_eq!(room_of(&world, body), Some(BodyRoom { size: Some([640.0, 480.0]) }));
    let mut jumped = ActorControl::default();
    jumped.0.jump_pressed = true;
    let jumper = world.spawn(jumped).id();
    assert_eq!(presses_of(&world, jumper), Some(BodyPresses { jump: true, burst: false }));
    let nothing = world.spawn_empty().id();
    assert_eq!((presses_of(&world, nothing), tumbling_of(&world, nothing)), (None, None));
}

/// A held course is the body's intent, whatever its brain asked.
#[test]
fn a_held_course_is_the_bodys_intent() {
    use bevy::ecs::system::RunSystemOnce as _;
    let mut world = World::new();
    let mut control = ActorControl::default();
    control.0.velocity_target = ambition_platformer2d_core::WorldVec2(Vec2::new(0.0, -300.0));
    let held = world.spawn((control, HeldCourse(Vec2::new(-1400.0, 0.0)))).id();
    let free = world.spawn(control).id();
    world.run_system_once(steer_held_courses).unwrap();
    assert_eq!(world.get::<ActorControl>(held).unwrap().0.velocity_target.0, Vec2::new(-1400.0, 0.0));
    assert_eq!(world.get::<ActorControl>(free).unwrap().0.velocity_target.0, Vec2::new(0.0, -300.0));
}
