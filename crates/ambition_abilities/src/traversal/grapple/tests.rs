use super::*;

fn world_with_right_wall() -> ambition_platformer2d_core::RoomGeometry {
    // A solid wall at x[380,400], y[0,400]; open space to its left.
    let blocks = vec![ae::Block::solid(
        "wall",
        ae::Vec2::new(380.0, 0.0),
        ae::Vec2::new(20.0, 400.0),
    )];
    ambition_platformer2d_core::RoomGeometry(ae::World::new(
        "grapple_test",
        ae::Vec2::new(400.0, 400.0),
        ae::Vec2::new(100.0, 200.0),
        blocks,
    ))
}

fn test_app(world: Option<ambition_platformer2d_core::RoomGeometry>) -> App {
    let mut app = App::new();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    if let Some(w) = world {
        ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
            app.world_mut(),
            w,
        );
    }
    app.add_systems(Update, grapple_system);
    app
}

fn spawn_player_holding(app: &mut App, id: &str, pos: ae::Vec2, facing: f32) -> Entity {
    crate::test_support::spawn_primary_player_holding_at(app, id, pos, facing)
}

fn player_vel(app: &App, player: Entity) -> ae::Vec2 {
    app.world().get::<BodyKinematics>(player).unwrap().vel
}

#[test]
fn grapple_yanks_the_player_toward_a_grappled_wall() {
    let mut app = test_app(Some(world_with_right_wall()));
    // Player to the left of the wall, facing/aiming right.
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    let vel = player_vel(&app, player);
    assert!(
        vel.x > 0.0,
        "the yank velocity points toward the wall (right)"
    );
    assert!(
        vel.x.abs() > vel.y.abs(),
        "a horizontal grapple yanks mostly horizontally ({vel:?})"
    );
    assert!(
        (vel.length() - GRAPPLE_PULL_SPEED).abs() < 1.0,
        "the yank is at the pull speed",
    );
}

#[test]
fn grapple_into_empty_space_does_not_move_the_player() {
    // No world (or no wall in range) → fizzle, velocity untouched.
    let mut app = test_app(None);
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.world_mut()
        .get_mut::<ActorControl>(player)
        .unwrap()
        .0
        .melee_pressed = true;
    app.update();
    assert_eq!(
        player_vel(&app, player),
        ae::Vec2::ZERO,
        "a dry grapple yanks nothing"
    );
}

#[test]
fn no_grapple_without_attack_or_with_a_different_item() {
    // Holding grapple but not attacking → no pull.
    let mut app = test_app(Some(world_with_right_wall()));
    let player = spawn_player_holding(&mut app, GRAPPLE_ID, ae::Vec2::new(100.0, 200.0), 1.0);
    app.update();
    assert_eq!(player_vel(&app, player), ae::Vec2::ZERO);
    // Holding the bomb + attacking → grapple_system ignores it.
    let mut app2 = test_app(Some(world_with_right_wall()));
    let player2 = spawn_player_holding(&mut app2, "bomb", ae::Vec2::new(100.0, 200.0), 1.0);
    app2.world_mut()
        .get_mut::<ActorControl>(player2)
        .unwrap()
        .0
        .melee_pressed = true;
    app2.update();
    assert_eq!(player_vel(&app2, player2), ae::Vec2::ZERO);
}

/// ⭐⭐ A SECOND DRIVEN BODY GRAPPLES TOO — same singular-`ControlledSubject`
/// defect as the blink.
#[test]
fn two_driven_bodies_each_grapple_the_wall_they_face() {
    use crate::test_support::spawn_seated_body_holding;
    let mut app = test_app(Some(world_with_right_wall()));
    app.insert_resource(ambition_platformer2d_shared_tangle::markers::ControlledSubject(None));
    // Both to the left of the same wall, at different heights.
    let a = spawn_seated_body_holding(
        &mut app,
        GRAPPLE_ID,
        0,
        "seat_a",
        ae::Vec2::new(100.0, 150.0),
    );
    let b = spawn_seated_body_holding(
        &mut app,
        GRAPPLE_ID,
        1,
        "seat_b",
        ae::Vec2::new(100.0, 250.0),
    );
    for body in [a, b] {
        app.world_mut()
            .get_mut::<ActorControl>(body)
            .unwrap()
            .0
            .melee_pressed = true;
    }
    app.update();
    assert!(
        player_vel(&app, a).x > 0.0,
        "seat a was not yanked toward the wall: {:?}",
        player_vel(&app, a)
    );
    assert!(
        player_vel(&app, b).x > 0.0,
        "seat b was not yanked toward the wall: {:?}",
        player_vel(&app, b)
    );
}
