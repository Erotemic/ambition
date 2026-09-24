//! Sanic stands ON the floor, not in it.
//!
//! Jon, 2026-09-24, with screenshots: *"sanic should be on the top of the blue
//! surface rather than standing on the bottom of it."* The momentum solver
//! rides a circle, and it took that circle's radius from the body's SMALLER
//! half-extent. Sanic is taller than he is wide (33.7x48), so the circle was
//! shorter than his box and the box — with the art drawn on it — hung 7.2
//! units below the surface he rode.

use ambition_platformer2d::engine_core as ae;
use bevy::prelude::*;

#[test]
fn sanic_stands_on_the_floor_he_rides() {
    let mut app = ambition_demo_sanic_app::build_demo_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    for _ in 0..120 {
        app.update();
    }
    let world = app.world_mut();
    let (kin, on_ground) = world
        .query_filtered::<(&ae::BodyKinematics, &ae::BodyGroundState), With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>()
        .iter(world)
        .next()
        .map(|(kin, ground)| (*kin, ground.on_ground))
        .expect("the demo spawned Sanic");
    assert!(on_ground, "the premise: he has settled onto the speedway floor");
    assert!(
        kin.size.y > kin.size.x + 1.0,
        "the premise: a body taller than it is wide, which is the case that sank"
    );
    let feet = kin.pos.y + kin.size.y * 0.5;
    let floor = world
        .query::<&ae::RoomGeometry>()
        .iter(world)
        .next()
        .expect("the room's geometry")
        .0
        .blocks
        .iter()
        .filter(|block| {
            kin.pos.x >= block.aabb.min.x && kin.pos.x <= block.aabb.max.x && block.aabb.min.y >= kin.pos.y
        })
        .map(|block| block.aabb.min.y)
        .fold(f32::INFINITY, f32::min);
    assert!(floor.is_finite(), "the premise: a block under him");
    assert!(
        (feet - floor).abs() < 0.5,
        "his feet are at {feet:.2} and the floor he rides is at {floor:.2}: \
         {:.2} units into it",
        feet - floor
    );
}
